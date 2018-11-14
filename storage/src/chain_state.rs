use super::{Result, Storage};
use cardano::block::{BlockDate, EpochId, EpochSlotId, HeaderHash, Utxos, ChainState};
use cardano::config::{GenesisData};
use cardano::tx::TxoPointer;
use cbor_event::{de, se, Len};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use storage_units::utils::magic;

const FILE_TYPE: magic::FileType = 0x5554584f; // = UTXO
const VERSION: magic::Version = 2;

/// Write the chain state at the end of the specified epoch to
/// disk. To reduce storage requirements (in particular of the utxo
/// state), we actually write a delta between some "parent" epoch and
/// the specified epoch, such that the full utxo state for an epoch
/// can be reconstructed by reading O(lg epoch) files. The parent of
/// an epoch is that epoch with the least significant bit cleared. For
/// example, for epoch 37, the patch sequence is 0 -> 32 -> 36 -> 37.
pub fn write_chain_state(
    storage: &Storage,
    chain_state: &ChainState,
) -> Result<()> {
    let last_date = chain_state.last_date.unwrap();
    assert!(last_date.is_boundary());
    let epoch = last_date.get_epochid();

    let mut tmpfile = super::tmpfile_create_type(storage, super::StorageFileType::Epoch);

    write_chain_state_delta(
        storage,
        chain_state,
        parent_for_epoch(epoch),
        &mut tmpfile,
    )?;

    tmpfile.render_permanent(&storage.config.get_epoch_utxos_filepath(epoch))?;

    //assert_eq!(&get_utxos_for_epoch(storage, epoch)?.utxos, utxos);

    Ok(())
}

/// Write the chain state delta between two arbitrary epochs, or write
/// a full utxo dump if parent_epoch is None.
fn write_chain_state_delta<W: Write>(
    storage: &Storage,
    chain_state: &ChainState,
    parent_epoch: Option<EpochId>,
    writer: &mut W,
) -> Result<()> {
    let last_date = chain_state.last_date.unwrap();

    magic::write_header(writer, FILE_TYPE, VERSION)?;

    let parent_utxos = match parent_epoch {
        None => BTreeMap::new(),
        Some(parent_epoch) => {
            let mut dummy_chain_state = ChainState {
                protocol_magic: chain_state.protocol_magic,
                fee_policy: chain_state.fee_policy,
                last_block: chain_state.last_block.clone(),
                last_date: None,
                slot_leaders: vec![],
                utxos: BTreeMap::new(),
                chain_length: 0,
                nr_transactions: 0,
                spend_txos: 0,
            };
            do_get_chain_state(storage, parent_epoch, &mut dummy_chain_state)?;
            dummy_chain_state.utxos
        }
    };

    let (removed_utxos, added_utxos) = diff_maps(&parent_utxos, &chain_state.utxos);

    debug!(
        "writing chain state delta {:?} -> {}, total {} utxos, added {} utxos, removed {} utxos",
        parent_epoch,
        last_date.get_epochid(),
        chain_state.utxos.len(),
        added_utxos.len(),
        removed_utxos.len()
    );

    let serializer = se::Serializer::new(writer)
        .write_array(Len::Len(7))?
        .serialize(&parent_epoch)?
        .serialize(&chain_state.last_block)?
        .serialize(&last_date.get_epochid())?
        .serialize(&match last_date {
            BlockDate::Boundary(_) => 0u16,
            BlockDate::Normal(s) => s.slotid + 1,
        })?
        .serialize(&chain_state.chain_length)?;
    let serializer = se::serialize_fixed_array(removed_utxos.iter(), serializer)?;
    se::serialize_fixed_map(added_utxos.iter(), serializer)?;

    Ok(())
}

pub struct UtxoState {
    pub last_block: HeaderHash,
    pub last_date: BlockDate,
    pub utxos: Utxos,
}

/// Reconstruct the full utxo state at the end of the specified epoch
/// by reading and applying the epoch's ancestor delta chain.
pub fn read_chain_state(storage: &Storage, genesis_data: &GenesisData, epoch: EpochId) -> Result<ChainState> {
    let mut chain_state = ChainState::new(genesis_data);
    do_get_chain_state(storage, epoch, &mut chain_state)?;
    Ok(chain_state)
}

fn do_get_chain_state(
    storage: &Storage,
    epoch: EpochId,
    chain_state: &mut ChainState,
) -> Result<()> {
    let filename = storage.config.get_epoch_utxos_filepath(epoch);

    let file = decode_chain_state_file(&mut fs::File::open(&filename)?)?;

    assert_eq!(file.last_date.get_epochid(), epoch);

    if let Some(parent) = file.parent {
        do_get_chain_state(storage, parent, chain_state)?;
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

    assert_eq!(chain_state.last_date.unwrap(), BlockDate::Boundary(epoch));

    Ok(())
}

#[derive(Debug)]
struct ChainStateFile {
    pub parent: Option<EpochId>,
    pub last_block: HeaderHash,
    pub last_date: BlockDate,
    pub chain_length: u64,
    pub removed_utxos: Vec<TxoPointer>,
    pub added_utxos: Utxos,
}

fn decode_chain_state_file<R: Read>(file: &mut R) -> Result<ChainStateFile> {
    magic::check_header(file, FILE_TYPE, VERSION, VERSION)?;

    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let mut raw = de::RawCbor::from(&data);

    raw.tuple(7, "chain state delta file")?;
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
    let removed_utxos = raw.deserialize()?;
    let added_utxos = raw.deserialize()?;

    Ok(ChainStateFile {
        parent,
        last_block,
        last_date,
        chain_length,
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
