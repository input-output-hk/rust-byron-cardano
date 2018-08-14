use config::net;
use network::{Peer, api::Api, api::BlockRef, Result};
use storage::{self, tag, Error, block_read};
use cardano::block::{BlockDate, EpochId, HeaderHash, BlockHeader};
use cardano::util::{hex};
use std::time::{SystemTime, Duration};

fn duration_print(d: Duration) -> String {
    format!("{}.{:03} seconds", d.as_secs(), d.subsec_millis())
}

struct EpochWriterState {
    epoch_id: EpochId,
    writer: storage::pack::PackWriter,
    write_start_time: SystemTime,
    blobs_to_delete: Vec<HeaderHash>,
}

fn net_sync_to<A: Api>(
    net: &mut A,
    net_cfg: &net::Config,
    storage: &storage::Storage,
    tip_header: &BlockHeader)
    -> Result<()>
{
    let tip = BlockRef {
        hash: tip_header.compute_hash(),
        parent: tip_header.get_previous_header(),
        date: tip_header.get_blockdate()
    };

    debug!("Configured genesis   : {}", net_cfg.genesis);
    debug!("Configured genesis-1 : {}", net_cfg.genesis_prev);
    info!( "Network TIP is       : {} ({}) <- {}", tip.hash, tip.date, tip_header.get_previous_header());

    // Start fetching at the current HEAD tag, or the genesis block if
    // it doesn't exist.
    let genesis_ref = (BlockRef {
        hash: net_cfg.genesis.clone(),
        parent: net_cfg.genesis_prev.clone(),
        date: BlockDate::Genesis(net_cfg.epoch_start)
    }, true);

    let our_tip = match storage.get_block_from_tag(&tag::HEAD) {
        Err(Error::NoSuchTag) => genesis_ref.clone(),
        Err(err) => panic!(err),
        Ok(block) => {
            let header = block.get_header();
            (BlockRef {
                hash: header.compute_hash().clone(),
                parent: header.get_previous_header(),
                date: header.get_blockdate()
            }, false)
        }
    };

    // TODO: we need to handle the case where our_tip is not an
    // ancestor of tip. In that case we should start from the last
    // stable epoch before our_tip.

    info!("Fetching from        : {} ({})", our_tip.0.hash, our_tip.0.date);

    // Determine whether the previous epoch is stable yet. Note: This
    // assumes that k is smaller than the number of blocks in an
    // epoch.
    let first_unstable_epoch = tip.date.get_epochid() -
        match tip.date {
            BlockDate::Genesis(_) => 1,
            BlockDate::Normal(d) => if d.slotid as usize <= net_cfg.epoch_stability_depth { 1 } else { 0 }
        };
    info!("First unstable epoch : {}", first_unstable_epoch);

    let mut epoch_writer_state : Option<EpochWriterState> = None;

    let mut last_block : Option<HeaderHash> = None;

    // If our tip is in an epoch that has become stable, we now need
    // to pack it. So read the previously fetched blocks in this epoch
    // and prepend them to the incoming blocks.
    if our_tip.0.date.get_epochid() < first_unstable_epoch && our_tip != genesis_ref
        && !epoch_exists(storage, our_tip.0.date.get_epochid())
    {
        let epoch_id = our_tip.0.date.get_epochid();

        epoch_writer_state = Some(EpochWriterState {
            epoch_id,
            writer: storage::pack::PackWriter::init(&storage.config),
            write_start_time: SystemTime::now(),
            blobs_to_delete: vec![]
        });
        last_block = Some(our_tip.0.hash.clone());

        let prev_block = append_blocks_to_epoch_reverse(
            storage, epoch_writer_state.as_mut().unwrap(), &our_tip.0.hash);

        // If tip.slotid < w, the previous epoch won't have been
        // created yet either, so do that now.
        if epoch_id > net_cfg.epoch_start {
            maybe_create_epoch(storage, epoch_id - 1, &prev_block);
        }
    }

    // If the previous epoch has become stable, then we may need to
    // pack it.
    else if our_tip.0.date.get_epochid() == first_unstable_epoch
        && first_unstable_epoch > net_cfg.epoch_start
        && !epoch_exists(storage, first_unstable_epoch - 1)
    {
        // Iterate to the last block in the previous epoch.
        let mut cur_hash = our_tip.0.hash.clone();
        loop {
            let block_raw = block_read(&storage, cur_hash.bytes()).unwrap();
            let block = block_raw.decode().unwrap();
            let hdr = block.get_header();
            assert!(hdr.get_blockdate().get_epochid() == first_unstable_epoch);
            cur_hash = hdr.get_previous_header();
            if hdr.get_blockdate().is_genesis() { break }
        }

        maybe_create_epoch(storage, first_unstable_epoch - 1, &cur_hash);
    }

    net.get_blocks(&our_tip.0, our_tip.1, &tip, &mut |block_hash, block, block_raw| {
        let date = block.get_header().get_blockdate();

        // Flush the previous epoch (if any).
        if date.is_genesis() {
            if let Some(epoch_writer_state) = epoch_writer_state.as_mut() {
                finish_epoch(storage, epoch_writer_state);

                // Checkpoint the tip so we don't have to refetch
                // everything if we get interrupted.
                storage::tag::write(storage, &tag::HEAD, &last_block.as_ref().unwrap().bytes()[..]);
            }
        }

        if date.get_epochid() >= first_unstable_epoch {
            // This block is not part of a stable epoch yet and could
            // be rolled back. Therefore we can't pack this epoch
            // yet. Instead we write this block to disk separately.
            let block_hash = storage::types::header_to_blockhash(&block_hash);
            storage::blob::write(storage, &block_hash, block_raw.as_ref()).unwrap();
        } else {

            // If this is the epoch genesis block, start writing a new epoch pack.
            if date.is_genesis() {
                epoch_writer_state = Some(EpochWriterState {
                    epoch_id: date.get_epochid(),
                    writer: storage::pack::PackWriter::init(&storage.config),
                    write_start_time: SystemTime::now(),
                    blobs_to_delete: vec![]
                });
            }

            // And append the block to the epoch pack.
            if let Some(epoch_writer_state) = epoch_writer_state.as_mut() {
                epoch_writer_state.writer.append(
                    &storage::types::header_to_blockhash(&block_hash), block_raw.as_ref());
            }
        }

        last_block = Some(block_hash.clone());
    })?;

    // Update the tip tag to point to the most recent block.
    if let Some(block_hash) = last_block {
        storage::tag::write(&storage, &tag::HEAD,
                            &storage::types::header_to_blockhash(&block_hash));
    }

    Ok(())
}

/// Synchronize the local blockchain stored in `storage` with the
/// network `net`. That is, fetch all blocks between the most recent
/// block we received (as denoted by the `HEAD` tag) and the network's
/// current tip. Blocks will be packed into epochs on disk as soon
/// they're stable.
///
/// If `sync_once` is set to `true`, then this function will
/// synchronize once and then return. If it's set to `false`, then
/// this function will run forever, continuously synchronizing to the
/// network's latest tip. (In the case of the Hermes backend, it will
/// sleep for some time between polling for new tips; with the native
/// protocol backend, it will block waiting for the server to send us
/// new tip announcements.)
pub fn net_sync<A: Api>(
    net: &mut A,
    net_cfg: &net::Config,
    storage: &storage::Storage,
    sync_once: bool)
    -> Result<()>
{
    // recover and print the TIP of the network
    let mut tip_header = net.get_tip()?;

    loop {

        net_sync_to(net, net_cfg, storage, &tip_header)?;

        if sync_once { break }

        tip_header = net.wait_for_new_tip(&tip_header.compute_hash())?;
    }

    Ok(())
}

// Create an epoch from a complete set of previously fetched blocks on
// disk.
fn maybe_create_epoch(storage: &storage::Storage, epoch_id: EpochId, last_block: &HeaderHash)
{
    if epoch_exists(&storage, epoch_id) { return }

    info!("Packing epoch {}", epoch_id);

    let mut epoch_writer_state = EpochWriterState {
        epoch_id,
        writer: storage::pack::PackWriter::init(&storage.config),
        write_start_time: SystemTime::now(),
        blobs_to_delete: vec![]
    };

    append_blocks_to_epoch_reverse(&storage, &mut epoch_writer_state, last_block);

    finish_epoch(storage, &mut epoch_writer_state);
}

// Check whether an epoch pack exists on disk.
fn epoch_exists(storage: &storage::Storage, epoch_id: EpochId) -> bool
{
    // FIXME: epoch_read() is a bit inefficient here; we really only
    // want to know if it exists.
    storage::epoch::epoch_read(&storage.config, epoch_id).is_ok()
}

fn append_blocks_to_epoch_reverse(
    storage: &storage::Storage,
    epoch_writer_state: &mut EpochWriterState,
    last_block: &HeaderHash)
    -> HeaderHash
{
    let mut cur_hash = last_block.clone();
    let mut blocks = vec![];
    loop {
        let block_raw = block_read(&storage, cur_hash.bytes()).unwrap();
        epoch_writer_state.blobs_to_delete.push(cur_hash.clone());
        let block = block_raw.decode().unwrap();
        let hdr = block.get_header();
        assert!(hdr.get_blockdate().get_epochid() == epoch_writer_state.epoch_id);
        blocks.push((storage::types::header_to_blockhash(&cur_hash), block_raw));
        cur_hash = hdr.get_previous_header();
        if hdr.get_blockdate().is_genesis() { break }
    }

    while let Some((hash, block_raw)) = blocks.pop() {
        epoch_writer_state.writer.append(&hash, block_raw.as_ref());
    }

    cur_hash
}

fn finish_epoch(storage: &storage::Storage, epoch_writer_state: &mut EpochWriterState) {
    let epoch_id = epoch_writer_state.epoch_id;
    let (packhash, index) = epoch_writer_state.writer.finalize();
    let (_, tmpfile) = storage::pack::create_index(&storage, &index);
    tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash)).unwrap();
    let epoch_time_elapsed = epoch_writer_state.write_start_time.elapsed().unwrap();

    if epoch_id > 0 {
        assert!(
            epoch_exists(storage, epoch_id - 1),
            "Attempted finish_epoch() with non-existent previous epoch (ID {}, previous' ID {})",
            epoch_id,
            epoch_id - 1
        );
    }

    storage::epoch::epoch_create(&storage.config, &packhash, epoch_id);

    info!("=> pack {} written for epoch {} in {}", hex::encode(&packhash[..]),
          epoch_id, duration_print(epoch_time_elapsed));

    for hash in &epoch_writer_state.blobs_to_delete {
        debug!("removing blob {}", hash);
        storage::blob::remove(&storage, hash.bytes());
    }
}

pub fn get_peer(blockchain: &str, cfg: &net::Config, native: bool) -> Peer {
    for peer in cfg.peers.iter() {
        if (native && peer.is_native()) || (!native && peer.is_http()) {
            return Peer::new(
                String::from(blockchain),
                peer.name().to_owned(),
                peer.peer().clone(),
                cfg.protocol_magic,
            ).unwrap();
        }
    }

    panic!("no peer to connect to")
}
