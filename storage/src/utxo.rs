use cardano::block::{EpochId, Utxos, HeaderHash, BlockDate, EpochSlotId};
use cardano::tx::{TxoPointer};
use cbor_event::{se, de, Len};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use storage_units::utils::magic;
use super::{Result, Storage};

const FILE_TYPE: magic::FileType = 0x5554584f; // = UTXO
const VERSION: magic::Version = 1;

/// Write the utxo state at the end of the specified epoch to disk. To
/// reduce storage requirements, we actually write a delta between
/// some "parent" epoch and the specified epoch, such that the full
/// utxo state for an epoch can be reconstructed by reading O(lg
/// epoch) files. The parent of an epoch is that epoch with the least
/// significant bit cleared. For example, for epoch 37, the patch
/// sequence is 0 -> 32 -> 36 -> 37.
pub fn write_utxos(storage: &Storage, last_block: &HeaderHash, last_date: &BlockDate, utxos: &Utxos) -> Result<()> {

    let epoch = last_date.get_epochid();

    let mut tmpfile = super::tmpfile_create_type(storage, super::StorageFileType::Epoch);

    magic::write_header(&mut tmpfile, FILE_TYPE, VERSION)?;

    let parent = parent_for_epoch(epoch);
    let parent_utxos = if epoch == 0 { BTreeMap::new() } else { get_utxos_for_epoch(storage, parent)?.2 };
    let (removed, added) = diff_maps(&parent_utxos, &utxos);

    println!("writing utxo delta {} -> {}, total {}, added {}, removed {}", parent, epoch,
           utxos.len(), added.len(), removed.len());

    {
        let serializer = se::Serializer::new(&mut tmpfile)
            .write_array(Len::Len(5))?
            .serialize(&parent)?
            .serialize(&last_block)?
            .serialize(&match last_date {
                BlockDate::Genesis(_) => 0u16,
                BlockDate::Normal(s) => s.slotid + 1,
            })?;
        let serializer = se::serialize_fixed_array(removed.iter(), serializer)?;
        se::serialize_fixed_map(added.iter(), serializer)?;
    }

    tmpfile.render_permanent(&storage.config.get_epoch_utxos_filepath(epoch))?;

    //assert_eq!(&get_utxos_for_epoch(storage, epoch)?, utxos);

    Ok(())
}

/// Reconstruct the full utxo state at the end of the specified epoch
/// by reading and applying the epoch's ancestor delta chain.
pub fn get_utxos_for_epoch(storage: &Storage, epoch: EpochId)
                           -> Result<(HeaderHash, BlockDate, Utxos)>
{
    let mut utxos = Utxos::new();
    let (last_block, last_date) = do_get_utxos(storage, epoch, &mut utxos)?;
    Ok((last_block, last_date, utxos))
}

fn do_get_utxos(storage: &Storage, epoch: EpochId, utxos: &mut Utxos) -> Result<(HeaderHash, BlockDate)> {

    let parent = parent_for_epoch(epoch);

    if epoch > 0 {
        do_get_utxos(storage, parent_for_epoch(epoch), utxos)?;
    }

    let filename = storage.config.get_epoch_utxos_filepath(epoch);

    let mut file = fs::File::open(&filename)?;

    magic::check_header(&mut file, FILE_TYPE, VERSION, VERSION)?;

    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let mut raw = de::RawCbor::from(&data);

    raw.tuple(5, "utxo delta file")?;
    let actual_parent = raw.deserialize::<EpochId>()?;
    assert_eq!(actual_parent, parent, "utxo delta file parent mismatch");

    let last_block = raw.deserialize()?;

    let last_date = match raw.deserialize()? {
        0 => BlockDate::Genesis(epoch),
        n => BlockDate::Normal(EpochSlotId { epoch, slotid: n - 1 }),
    };

    let removed = raw.deserialize::<Vec<TxoPointer>>()?;

    for txo_ptr in &removed {
        if utxos.remove(txo_ptr).is_none() {
            panic!("utxo delta removes non-existent utxo {}", txo_ptr);
        }
    }

    let added = raw.deserialize::<Utxos>()?;

    for (txo_ptr, txo) in added {
        if utxos.insert(txo_ptr, txo).is_some() {
            panic!("utxo delta inserts duplicate utxo");
        }
    }

    Ok((last_block, last_date))
}

/// Compute the parent of this epoch in the patch chain by clearing
/// the least-significant bit.
fn parent_for_epoch(epoch: EpochId) -> EpochId {
    if epoch == 0 { return 0; }
    for n in 0..63 {
        if epoch & (1 << n) != 0 {
            return epoch & !(1 << n);
        }
    }
    unreachable!();
}

/// Compute the diff from BTreeMap 'm1' to BTreeMap 'm2', returning
/// the set of keys in 'm1' that are not in 'm2', and the map of
/// keys/values that are in 'm2' but not in 'm1'.
fn diff_maps<'a, K, V>(m1: &'a BTreeMap<K, V>, m2: &'a BTreeMap<K, V>) -> (BTreeSet<&'a K>, BTreeMap<&'a K, &'a V>)
    where K: Ord
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
                None => { break }
                Some((n2, v2)) => {
                    added.insert(n2, v2);
                    e2 = i2.next();
                }
            },
            Some((n1, _)) => match e2 {
                None => {
                    removed.insert(n1);
                    e1 = i1.next();
                },
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
            }
        };
    };

    (removed, added)
}
