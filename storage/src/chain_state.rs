use super::{Result, Error, Storage};
use cardano::block::{BlockDate, EpochId, EpochSlotId, HeaderHash, Utxos, ChainState, Block};
use cardano::config::{GenesisData};
use cardano::tx::TxoPointer;
use cbor_event::{de, se, Len};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use storage_units::utils::{magic, error::StorageError};
use epoch;

const FILE_TYPE: magic::FileType = 0x5554584f; // = UTXO
const VERSION: magic::Version = 3;

/// Write the chain state to disk. To reduce storage requirements (in
/// particular of the utxo state), we actually write a delta between
/// some "parent" epoch and the specified epoch, such that the full
/// utxo state for an epoch can be reconstructed by reading O(lg
/// epoch) files. The parent of an epoch is that epoch with the least
/// significant bit cleared. For example, for epoch 37, the patch
/// sequence is 0 -> 32 -> 36 -> 37.
///
/// Note: we currently assume that chain_state.last_block is the last
/// block of an epoch.
pub fn write_chain_state(
    storage: &Storage,
    genesis_data: &GenesisData,
    chain_state: &ChainState,
) -> Result<()> {
    let last_date = chain_state.last_date.unwrap();
    let epoch = last_date.get_epochid();

    let mut tmpfile = super::tmpfile_create_type(storage, super::StorageFileType::Epoch);

    let parent_hash = match parent_for_epoch(epoch) {
        None => genesis_data.genesis_prev.clone(),
        Some(parent_epoch) => get_last_block_of_epoch(storage, parent_epoch)?
    };

    write_chain_state_delta(
        storage,
        genesis_data,
        chain_state,
        &parent_hash,
        &mut tmpfile,
    )?;

    let path = storage.config.get_chain_state_filepath(
        chain_state.last_block.as_hash_bytes()
    );
    tmpfile.render_permanent(&path)?;

    // Check that we can reconstruct the state from disk.
    debug_assert!(&read_chain_state(storage, genesis_data, &chain_state.last_block)? == chain_state);

    Ok(())
}

const NR_FIELDS: u64 = 10;

/// Write the chain state delta between chain_state and the state at
/// 'parent_block'.
pub fn write_chain_state_delta<W: Write>(
    storage: &Storage,
    genesis_data: &GenesisData,
    chain_state: &ChainState,
    parent_block: &HeaderHash,
    writer: &mut W,
) -> Result<()> {
    let last_date = chain_state.last_date.unwrap();

    magic::write_header(writer, FILE_TYPE, VERSION)?;

    let parent_chain_state = read_chain_state(storage, genesis_data, parent_block)?;
    assert_eq!(&parent_chain_state.last_block, parent_block);

    let (removed_utxos, added_utxos) = diff_maps(&parent_chain_state.utxos, &chain_state.utxos);

    debug!(
        "writing chain state delta {} ({:?}) -> {} ({:?}), total {} utxos, added {} utxos, removed {} utxos\n",
        parent_chain_state.last_block,
        parent_chain_state.last_date,
        chain_state.last_block,
        chain_state.last_date,
        chain_state.utxos.len(),
        added_utxos.len(),
        removed_utxos.len()
    );

    let serializer = se::Serializer::new(writer)
        .write_array(Len::Len(NR_FIELDS))?
        .serialize(&parent_block)?
        .serialize(&chain_state.last_block)?
        .serialize(&last_date.get_epochid())?
        .serialize(&match last_date {
            BlockDate::Boundary(_) => 0u16,
            BlockDate::Normal(s) => s.slotid + 1,
        })?
        .serialize(&chain_state.last_boundary_block.as_ref().unwrap())?
        .serialize(&chain_state.chain_length)?
        .serialize(&chain_state.nr_transactions)?
        .serialize(&chain_state.spent_txos)?;
    let serializer = se::serialize_fixed_array(removed_utxos.iter(), serializer)?;
    se::serialize_fixed_map(added_utxos.iter(), serializer)?;

    Ok(())
}

/// Reconstruct the full utxo state as of the specified block by
/// reading and applying the blocks's ancestor delta chain.
pub fn read_chain_state(storage: &Storage, genesis_data: &GenesisData, block_hash: &HeaderHash) -> Result<ChainState> {
    if block_hash == &genesis_data.genesis_prev {
        return Ok(ChainState::new(genesis_data));
    }

    let mut chain_state = do_get_chain_state(storage, genesis_data, block_hash)?;

    // We don't store the slot leaders because we can easily get them
    // from the boundary block.
    if let Some(last_boundary_block) = &chain_state.last_boundary_block {
        let hash = last_boundary_block.as_hash_bytes();
        chain_state.slot_leaders = match storage.read_block(hash).unwrap().decode()? {
            Block::BoundaryBlock(blk) => {
                assert_eq!(blk.header.consensus.epoch, chain_state.last_date.unwrap().get_epochid());
                blk.body.slot_leaders.clone()
            },
            _ => panic!("unexpected non-boundary block")
        };
    }

    Ok(chain_state)
}

fn do_get_chain_state(
    storage: &Storage,
    genesis_data: &GenesisData,
    block_hash: &HeaderHash,
) -> Result<ChainState> {
    let filename = storage.config.get_chain_state_filepath(block_hash.as_hash_bytes());

    let file = decode_chain_state_file(&mut fs::File::open(&filename)?)?;

    let mut chain_state = if file.parent != genesis_data.genesis_prev {
        do_get_chain_state(storage, genesis_data, &file.parent)?
    } else {
        ChainState::new(genesis_data)
    };

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
    chain_state.last_boundary_block = Some(file.last_boundary_block);
    chain_state.chain_length = file.chain_length;
    chain_state.nr_transactions = file.nr_transactions;
    chain_state.spent_txos = file.spent_txos;

    Ok(chain_state)
}

#[derive(Debug)]
pub struct ChainStateFile {
    pub parent: HeaderHash,
    pub last_block: HeaderHash,
    pub last_date: BlockDate,
    pub last_boundary_block: HeaderHash,
    pub chain_length: u64,
    pub nr_transactions: u64,
    pub spent_txos: u64,
    pub removed_utxos: Vec<TxoPointer>,
    pub added_utxos: Utxos,
}

pub fn decode_chain_state_file<R: Read>(file: &mut R) -> Result<ChainStateFile> {
    magic::check_header(file, FILE_TYPE, VERSION, VERSION)?;

    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let mut raw = de::RawCbor::from(&data);

    raw.tuple(NR_FIELDS, "chain state delta file")?;
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
    let last_boundary_block = raw.deserialize()?;
    let chain_length = raw.deserialize()?;
    let nr_transactions = raw.deserialize()?;
    let spent_txos = raw.deserialize()?;
    let removed_utxos = raw.deserialize()?;
    let added_utxos = raw.deserialize()?;

    Ok(ChainStateFile {
        parent,
        last_block,
        last_date,
        last_boundary_block,
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

pub fn get_last_block_of_epoch(storage: &Storage, epoch: EpochId) -> Result<HeaderHash> {
    // FIXME: don't rely on epoch refpacks since they may not be stable.
    let mut it = epoch::epoch_open_packref(&storage.config, epoch)?;
    let mut last_block = None;
    while let Some(x) = it.next()? {
        last_block = Some(x);
    }
    Ok(last_block.unwrap().into())
}

/// Return the chain state at block 'block_hash'. This seeks backwards
/// in the chain, starting at 'block_hash' until it reaches a block
/// that has a chain state on disk. It then iterates forwards to
/// 'block_hash', verifying blocks and updating the chain state.
pub fn restore_chain_state(storage: &Storage, genesis_data: &GenesisData, block_hash: &HeaderHash) -> Result<ChainState> {

    debug!("restoring chain state at block {}", block_hash);

    let mut cur = block_hash.clone();
    let mut blocks_to_apply = vec![];

    loop {

        let mut chain_state = match read_chain_state(storage, genesis_data, &cur) {
            Ok(chain) => chain,
            Err(Error::StorageError(StorageError::IoError(ref err)))
                if err.kind() == ::std::io::ErrorKind::NotFound =>
            {
                let rblk = storage.read_block(cur.as_hash_bytes())
                    .expect(&format!("reading block {}", cur));
                let blk = rblk.decode().unwrap();
                // FIXME: store 'blk' in blocks_to_apply? Would
                // prevent having to read the block again below, but
                // require more memory.
                blocks_to_apply.push(cur);
                cur = blk.get_header().get_previous_header();
                continue;
            },
            Err(err) => return Err(err),
        };

        debug!("loaded chain state at block {}, have to apply {} blocks",
               cur, blocks_to_apply.len());

        assert_eq!(chain_state.last_block, cur);

        for hash in blocks_to_apply.iter().rev() {
            let rblk = storage.read_block(hash.as_hash_bytes())
                .expect(&format!("reading block {}", hash));
            let blk = rblk.decode().unwrap();
            chain_state.verify_block(hash, &blk)?;
        }

        assert_eq!(&chain_state.last_block, block_hash);

        return Ok(chain_state);
    };
}
