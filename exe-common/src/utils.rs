use cardano::block;
use storage;
use storage::types::PackHash;

// Return the chain of block headers starting at from's next block
// and terminating at to, unless this range represent a number
// of blocks greater than the limit imposed by the node we're talking to.
pub fn find_earliest_epoch(
    storage: &storage::Storage,
    minimum_epochid: block::EpochId,
    start_epochid: block::EpochId,
) -> Option<(block::EpochId, PackHash)> {
    let mut epoch_id = start_epochid;
    loop {
        match storage::tag::read_hash(storage, &storage::tag::get_epoch_tag(epoch_id)) {
            None => match storage::epoch::epoch_read_pack(&storage.config, epoch_id).ok() {
                None => {}
                Some(h) => {
                    return Some((epoch_id, h));
                }
            },
            Some(h) => {
                info!("latest known epoch found is {}", epoch_id);
                return Some((epoch_id, h.into_bytes()));
            }
        }

        if epoch_id > minimum_epochid {
            epoch_id -= 1
        } else {
            return None;
        }
    }
}

pub fn get_last_blockid(
    storage_config: &storage::config::StorageConfig,
    packref: &PackHash,
) -> Option<block::HeaderHash> {
    let mut reader = storage::pack::PackReader::init(&storage_config, packref);
    let mut last_blk_raw = None;

    while let Some(blk_raw) = reader.get_next() {
        last_blk_raw = Some(blk_raw);
    }
    if let Some(blk_raw) = last_blk_raw {
        let blk = blk_raw.decode().unwrap();
        let hdr = blk.get_header();
        info!("last_blockid: {} {}", hdr.compute_hash(), hdr.get_slotid());
        Some(hdr.compute_hash())
    } else {
        None
    }
}
