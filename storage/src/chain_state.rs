use super::{Result, Storage, block_read};
use cardano::block::{BlockDate, EpochId, EpochSlotId, HeaderHash, Utxos, ChainState, Block};
use cardano::config::{GenesisData};
use cardano::tx::TxoPointer;
use cbor_event::{de, se, Len};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use storage_units::utils::magic;
use epoch;

const FILE_TYPE: magic::FileType = 0x5554584f; // = UTXO
const VERSION: magic::Version = 2;

/// Write the chain state to disk. To reduce storage requirements (in
/// particular of the utxo state), we actually write a delta between
/// some "parent" epoch and the specified epoch, such that the full
/// utxo state for an epoch can be reconstructed by reading O(lg
/// epoch) files. The parent of an epoch is that epoch with the least
/// significant bit cleared. For example, for epoch 37, the patch
/// sequence is 0 -> 32 -> 36 -> 37.
pub fn write_chain_state(
    storage: &Storage,
    genesis_data: &GenesisData,
    chain_state: &ChainState,
) -> Result<()> {
    let last_date = chain_state.last_date.unwrap();
    assert!(last_date.is_boundary());
    let epoch = last_date.get_epochid();

    let mut tmpfile = super::tmpfile_create_type(storage, super::StorageFileType::Epoch);

    let parent_hash = match parent_for_epoch(epoch) {
        None => genesis_data.genesis_prev.clone(),
        Some(parent_epoch) => get_first_block_of_epoch(storage, parent_epoch)?
    };

    write_chain_state_delta(
        storage,
        genesis_data,
        chain_state,
        &parent_hash,
        &mut tmpfile,
    )?;

    tmpfile.render_permanent(&storage.config.get_chain_state_filepath(&chain_state.last_block))?;

    // Check that we can reconstruct the state from disk.
    debug_assert!(&read_chain_state(storage, genesis_data, &chain_state.last_block)? == chain_state);

    Ok(())
}

/// Write the chain state delta between two arbitrary epochs, or write
/// a full utxo dump if parent_epoch is None.
fn write_chain_state_delta<W: Write>(
    storage: &Storage,
    genesis_data: &GenesisData,
    chain_state: &ChainState,
    parent_block: &HeaderHash,
    writer: &mut W,
) -> Result<()> {
    let last_date = chain_state.last_date.unwrap();

    magic::write_header(writer, FILE_TYPE, VERSION)?;

    let parent_utxos = read_chain_state(storage, genesis_data, parent_block)?.utxos;

    let (removed_utxos, added_utxos) = diff_maps(&parent_utxos, &chain_state.utxos);

    debug!(
        "writing chain state delta {} -> {} ({:?}), total {} utxos, added {} utxos, removed {} utxos\n",
        parent_block,
        chain_state.last_block,
        chain_state.last_date,
        chain_state.utxos.len(),
        added_utxos.len(),
        removed_utxos.len()
    );

    let serializer = se::Serializer::new(writer)
        .write_array(Len::Len(9))?
        .serialize(&parent_block)?
        .serialize(&chain_state.last_block)?
        .serialize(&last_date.get_epochid())?
        .serialize(&match last_date {
            BlockDate::Boundary(_) => 0u16,
            BlockDate::Normal(s) => s.slotid + 1,
        })?
        .serialize(&chain_state.chain_length)?
        .serialize(&chain_state.nr_transactions)?
        .serialize(&chain_state.spent_txos)?;
    let serializer = se::serialize_fixed_array(removed_utxos.iter(), serializer)?;
    se::serialize_fixed_map(added_utxos.iter(), serializer)?;

    Ok(())
}

pub struct UtxoState {
    pub last_block: HeaderHash,
    pub last_date: BlockDate,
    pub utxos: Utxos,
}

/// Reconstruct the full utxo state as of the specified block by
/// reading and applying the blocks's ancestor delta chain.
pub fn read_chain_state(storage: &Storage, genesis_data: &GenesisData, block_hash: &HeaderHash) -> Result<ChainState> {
    let mut chain_state = ChainState::new(genesis_data);

    if block_hash != &genesis_data.genesis_prev {
        do_get_chain_state(storage, genesis_data, block_hash, &mut chain_state)?;

        // We don't store the slot leaders because we can easily get
        // them from the boundary block.
        chain_state.slot_leaders = match block_read(storage, block_hash).unwrap().decode()? {
            Block::BoundaryBlock(blk) => blk.body.slot_leaders.clone(),
            _ => panic!("unexpected non-boundary block")
        }
    }

    Ok(chain_state)
}

fn do_get_chain_state(
    storage: &Storage,
    genesis_data: &GenesisData,
    block_hash: &HeaderHash,
    chain_state: &mut ChainState,
) -> Result<()> {
    let filename = storage.config.get_chain_state_filepath(block_hash);

    let file = decode_chain_state_file(&mut fs::File::open(&filename)?)?;

    assert!(file.last_date.is_boundary());

    if file.parent != genesis_data.genesis_prev {
        do_get_chain_state(storage, genesis_data, &file.parent, chain_state)?;
    }

    for txo_ptr in &file.removed_utxos {
        if chain_state.utxos.remove(txo_ptr).is_none() {
            panic!("utxo delta removes non-existent utxo {}", txo_ptr);
        }
    }

    for (txo_ptr, txo) in file.added_utxos {
        if chain_state.utxos.insert(txo_ptr, txo).is_some() {
            panic!("utxo delta inserts duplicate utxo");
        }
    }

    chain_state.last_block = file.last_block;
    chain_state.last_date = Some(file.last_date);
    chain_state.chain_length = file.chain_length;
    chain_state.nr_transactions = file.nr_transactions;
    chain_state.spent_txos = file.spent_txos;

    Ok(())
}

#[derive(Debug)]
struct ChainStateFile {
    pub parent: HeaderHash,
    pub last_block: HeaderHash,
    pub last_date: BlockDate,
    pub chain_length: u64,
    pub nr_transactions: u64,
    pub spent_txos: u64,
    pub removed_utxos: Vec<TxoPointer>,
    pub added_utxos: Utxos,
}

fn decode_chain_state_file<R: Read>(file: &mut R) -> Result<ChainStateFile> {
    magic::check_header(file, FILE_TYPE, VERSION, VERSION)?;

    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let mut raw = de::RawCbor::from(&data);

    raw.tuple(9, "chain state delta file")?;
    let parent = raw.deserialize()?;
    let last_block = raw.deserialize()?;
    let epoch = raw.deserialize()?;
    let last_date = match raw.deserialize()? {
        0 => BlockDate::Boundary(epoch),
        n => BlockDate::Normal(EpochSlotId {
            epoch,
            slotid: n - 1,
        }),
    };
    let chain_length = raw.deserialize()?;
    let nr_transactions = raw.deserialize()?;
    let spent_txos = raw.deserialize()?;
    let removed_utxos = raw.deserialize()?;
    let added_utxos = raw.deserialize()?;

    Ok(ChainStateFile {
        parent,
        last_block,
        last_date,
        chain_length,
        nr_transactions,
        spent_txos,
        removed_utxos,
        added_utxos,
    })
}

/// Compute the parent of this epoch in the patch chain by clearing
/// the least-significant bit.
fn parent_for_epoch(epoch: EpochId) -> Option<EpochId> {
    if epoch == 0 {
        return None;
    }
    for n in 0..63 {
        if epoch & (1 << n) != 0 {
            return Some(epoch & !(1 << n));
        }
    }
    unreachable!();
}

/// Compute the diff from BTreeMap 'm1' to BTreeMap 'm2', returning
/// the set of keys in 'm1' that are not in 'm2', and the map of
/// keys/values that are in 'm2' but not in 'm1'.
fn diff_maps<'a, K, V>(
    m1: &'a BTreeMap<K, V>,
    m2: &'a BTreeMap<K, V>,
) -> (BTreeSet<&'a K>, BTreeMap<&'a K, &'a V>)
where
    K: Ord,
{
    let mut removed = BTreeSet::new();
    let mut added = BTreeMap::new();

    let mut i1 = m1.iter();
    let mut i2 = m2.iter();

    let mut e1 = i1.next();
    let mut e2 = i2.next();

    loop {
        match e1 {
            None => match e2 {
                None => break,
                Some((n2, v2)) => {
                    added.insert(n2, v2);
                    e2 = i2.next();
                }
            },
            Some((n1, _)) => match e2 {
                None => {
                    removed.insert(n1);
                    e1 = i1.next();
                }
                Some((n2, v2)) => {
                    if n1 < n2 {
                        removed.insert(n1);
                        e1 = i1.next();
                    } else if n1 > n2 {
                        added.insert(n2, v2);
                        e2 = i2.next();
                    } else {
                        e1 = i1.next();
                        e2 = i2.next();
                    }
                }
            },
        };
    }

    (removed, added)
}

pub fn get_first_block_of_epoch(storage: &Storage, epoch: EpochId) -> Result<HeaderHash> {
    // FIXME: don't rely on epoch refpacks since they may not be stable.
    Ok(epoch::epoch_open_packref(&storage.config, epoch)?.next()?.unwrap().into())
}
