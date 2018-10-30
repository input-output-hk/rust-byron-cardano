use super::{Result, Storage};
use cardano::block::{BlockDate, EpochId, EpochSlotId, HeaderHash, Utxos};
use cardano::tx::TxoPointer;
use cbor_event::{de, se, Len};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use storage_units::utils::magic;

const FILE_TYPE: magic::FileType = 0x5554584f; // = UTXO
const VERSION: magic::Version = 1;

/// Write the utxo state at the end of the specified epoch to disk. To
/// reduce storage requirements, we actually write a delta between
/// some "parent" epoch and the specified epoch, such that the full
/// utxo state for an epoch can be reconstructed by reading O(lg
/// epoch) files. The parent of an epoch is that epoch with the least
/// significant bit cleared. For example, for epoch 37, the patch
/// sequence is 0 -> 32 -> 36 -> 37.
pub fn write_utxos(
    storage: &Storage,
    last_block: &HeaderHash,
    last_date: &BlockDate,
    utxos: &Utxos,
) -> Result<()> {
    let epoch = last_date.get_epochid();

    let mut tmpfile = super::tmpfile_create_type(storage, super::StorageFileType::Epoch);

    write_utxos_delta(
        storage,
        last_block,
        last_date,
        utxos,
        parent_for_epoch(epoch),
        &mut tmpfile,
    )?;

    tmpfile.render_permanent(&storage.config.get_epoch_utxos_filepath(epoch))?;

    //assert_eq!(&get_utxos_for_epoch(storage, epoch)?.utxos, utxos);

    Ok(())
}

/// Write the utxo delta between two arbitrary epochs, or write a full
/// utxo dump if parent_epoch is None.
pub fn write_utxos_delta<W: Write>(
    storage: &Storage,
    last_block: &HeaderHash,
    last_date: &BlockDate,
    utxos: &Utxos,
    parent_epoch: Option<EpochId>,
    writer: &mut W,
) -> Result<()> {
    magic::write_header(writer, FILE_TYPE, VERSION)?;

    let parent_utxos = match parent_epoch {
        None => BTreeMap::new(),
        Some(parent_epoch) => get_utxos_for_epoch(storage, parent_epoch)?.utxos,
    };

    let (removed, added) = diff_maps(&parent_utxos, &utxos);

    debug!(
        "writing utxo delta {:?} -> {}, total {}, added {}, removed {}",
        parent_epoch,
        last_date.get_epochid(),
        utxos.len(),
        added.len(),
        removed.len()
    );

    let serializer = se::Serializer::new(writer)
        .write_array(Len::Len(5))?
        .serialize(&parent_epoch)?
        .serialize(&last_block)?
        .serialize(&last_date.get_epochid())?
        .serialize(&match last_date {
            BlockDate::Boundary(_) => 0u16,
            BlockDate::Normal(s) => s.slotid + 1,
        })?;
    let serializer = se::serialize_fixed_array(removed.iter(), serializer)?;
    se::serialize_fixed_map(added.iter(), serializer)?;

    Ok(())
}

pub struct UtxoState {
    pub last_block: HeaderHash,
    pub last_date: BlockDate,
    pub utxos: Utxos,
}

/// Reconstruct the full utxo state at the end of the specified epoch
/// by reading and applying the epoch's ancestor delta chain.
pub fn get_utxos_for_epoch(storage: &Storage, epoch: EpochId) -> Result<UtxoState> {
    let mut utxos = Utxos::new();
    let (last_block, last_date) = do_get_utxos(storage, epoch, &mut utxos)?;
    Ok(UtxoState {
        last_block,
        last_date,
        utxos,
    })
}

fn do_get_utxos(
    storage: &Storage,
    epoch: EpochId,
    utxos: &mut Utxos,
) -> Result<(HeaderHash, BlockDate)> {
    let filename = storage.config.get_epoch_utxos_filepath(epoch);

    let file = decode_utxo_file(&mut fs::File::open(&filename)?)?;

    assert_eq!(file.last_date.get_epochid(), epoch);

    if let Some(parent) = file.parent {
        do_get_utxos(storage, parent, utxos)?;
    }

    for txo_ptr in &file.removed {
        if utxos.remove(txo_ptr).is_none() {
            panic!("utxo delta removes non-existent utxo {}", txo_ptr);
        }
    }

    for (txo_ptr, txo) in file.added {
        if utxos.insert(txo_ptr, txo).is_some() {
            panic!("utxo delta inserts duplicate utxo");
        }
    }

    Ok((file.last_block, file.last_date))
}

#[derive(Debug)]
pub struct UtxoFile {
    pub parent: Option<EpochId>,
    pub last_block: HeaderHash,
    pub last_date: BlockDate,
    pub removed: Vec<TxoPointer>,
    pub added: Utxos,
}

pub fn decode_utxo_file<R: Read>(file: &mut R) -> Result<UtxoFile> {
    magic::check_header(file, FILE_TYPE, VERSION, VERSION)?;

    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let mut raw = de::RawCbor::from(&data);

    raw.tuple(5, "utxo delta file")?;
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
    let removed = raw.deserialize()?;
    let added = raw.deserialize()?;

    Ok(UtxoFile {
        parent,
        last_block,
        last_date,
        removed,
        added,
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
