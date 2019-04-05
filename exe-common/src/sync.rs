use cardano::block::{Block, BlockDate, BlockHeader, ChainState, EpochId, HeaderHash, RawBlock};
use cardano::config::GenesisData;
use cardano::util::hex;
use cardano_storage::{
    blob, chain_state,
    epoch::{self, epoch_exists},
    pack, tag, types, Error, Storage,
};
use config::net;
use network::{api::Api, api::BlockRef, Peer, Result};
use std::mem;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use storage_units::packfile;

fn duration_print(d: Duration) -> String {
    format!("{}.{:03} seconds", d.as_secs(), d.subsec_millis())
}

struct EpochWriterState {
    epoch_id: EpochId,
    writer: packfile::Writer,
    write_start_time: SystemTime,
    blobs_to_delete: Vec<HeaderHash>,
}

fn net_sync_to<A: Api>(
    net: &mut A,
    net_cfg: &net::Config,
    genesis_data: &GenesisData,
    storage: Arc<RwLock<Storage>>,
    tip_header: &BlockHeader,
) -> Result<()> {
    let tip = BlockRef {
        hash: tip_header.compute_hash(),
        parent: tip_header.get_previous_header(),
        date: tip_header.get_blockdate(),
    };
    let storage_config = storage.read().unwrap().config.clone();

    debug!("Configured genesis   : {}", net_cfg.genesis);
    debug!("Configured genesis-1 : {}", net_cfg.genesis_prev);
    info!(
        "Network TIP is       : {} ({}) <- {}",
        tip.hash, tip.date, tip.parent
    );

    // Start fetching at the current HEAD tag, or the genesis block if
    // it doesn't exist.
    let (our_tip, our_tip_is_genesis) = match storage.read().unwrap().get_block_from_tag(&tag::HEAD)
    {
        Err(Error::NoSuchTag) => (
            BlockRef {
                hash: net_cfg.genesis.clone(),
                parent: net_cfg.genesis_prev.clone(),
                date: BlockDate::Boundary(net_cfg.epoch_start),
            },
            true,
        ),
        Err(err) => panic!(err),
        Ok(block) => {
            let header = block.header();
            (
                BlockRef {
                    hash: header.compute_hash(),
                    parent: header.previous_header(),
                    date: header.blockdate(),
                },
                false,
            )
        }
    };

    // TODO: we need to handle the case where our_tip is not an
    // ancestor of tip. In that case we should start from the last
    // stable epoch before our_tip.

    info!("Fetching from        : {} ({})", our_tip.hash, our_tip.date);

    // Determine whether the previous epoch is stable yet. Note: This
    // assumes that k is smaller than the number of blocks in an
    // epoch.
    let first_unstable_epoch = tip.date.get_epochid()
        - match tip.date {
            BlockDate::Boundary(_) => 1,
            BlockDate::Normal(d) => {
                if d.slotid as usize <= net_cfg.epoch_stability_depth {
                    1
                } else {
                    0
                }
            }
        };
    info!("First unstable epoch : {}", first_unstable_epoch);

    let mut epoch_writer_state: Option<EpochWriterState> = None;

    // If our tip is in an epoch that has become stable, we now need
    // to pack it. So read the previously fetched blocks in this epoch
    // and prepend them to the incoming blocks.
    if our_tip.date.get_epochid() < first_unstable_epoch
        && !our_tip_is_genesis
        && !epoch_exists(&storage_config, our_tip.date.get_epochid()).unwrap()
    {
        let epoch_id = our_tip.date.get_epochid();

        // Read the blocks in the current epoch.
        let mut blobs_to_delete = vec![];
        let (last_block_in_prev_epoch, blocks) = get_unpacked_blocks_in_epoch(
            &storage.read().unwrap(),
            &our_tip.hash,
            epoch_id,
            &mut blobs_to_delete,
        );

        // If tip.slotid < w, the previous epoch won't have been
        // created yet either, so do that now.
        if epoch_id > net_cfg.epoch_start {
            maybe_create_epoch(
                &mut storage.write().unwrap(),
                genesis_data,
                epoch_id - 1,
                &last_block_in_prev_epoch,
            )?;
        }

        // Initialize the epoch writer and add the blocks in the current epoch.
        epoch_writer_state = Some(EpochWriterState {
            epoch_id,
            writer: pack::packwriter_init(&storage_config).unwrap(),
            write_start_time: SystemTime::now(),
            blobs_to_delete,
        });

        let mut chain_state = chain_state::restore_chain_state(
            &storage.read().unwrap(),
            genesis_data,
            &last_block_in_prev_epoch,
        )?;

        append_blocks_to_epoch_reverse(
            epoch_writer_state.as_mut().unwrap(),
            &mut chain_state,
            blocks,
        )?;
    }
    // If the previous epoch has become stable, then we may need to
    // pack it.
    else if our_tip.date.get_epochid() == first_unstable_epoch
        && first_unstable_epoch > net_cfg.epoch_start
        && !epoch_exists(&storage_config, first_unstable_epoch - 1).unwrap()
    {
        // Iterate to the last block in the previous epoch.
        let mut cur_hash = our_tip.hash.clone();
        loop {
            let block_raw = storage
                .read()
                .unwrap()
                .read_block(&cur_hash.into())
                .unwrap();
            let block = block_raw.decode().unwrap();
            let hdr = block.header();
            let blockdate = hdr.blockdate();
            assert!(blockdate.get_epochid() == first_unstable_epoch);
            cur_hash = hdr.previous_header();
            if blockdate.is_boundary() {
                break;
            }
        }

        maybe_create_epoch(
            &mut storage.write().unwrap(),
            genesis_data,
            first_unstable_epoch - 1,
            &cur_hash,
        )?;
    }

    let mut chain_state = chain_state::restore_chain_state(
        &storage.read().unwrap(),
        genesis_data,
        if our_tip_is_genesis {
            &our_tip.parent
        } else {
            &our_tip.hash
        },
    )?;

    net.get_blocks(
        &our_tip,
        our_tip_is_genesis,
        &tip,
        &mut |block_hash, block, block_raw| {
            let date = block.header().blockdate();

            // Flush the previous epoch (if any). FIXME: shouldn't rely on
            // 'date' here since the block hasn't been verified yet.
            if date.is_boundary() {
                let mut writer_state = None;
                mem::swap(&mut writer_state, &mut epoch_writer_state);

                if let Some(epoch_writer_state) = writer_state {
                    finish_epoch(
                        &mut storage.write().unwrap(),
                        genesis_data,
                        epoch_writer_state,
                        &chain_state,
                    )
                    .unwrap();

                    // Checkpoint the tip so we don't have to refetch
                    // everything if we get interrupted.
                    tag::write(
                        &storage.read().unwrap(),
                        &tag::HEAD,
                        &chain_state.last_block.as_ref(),
                    );
                }
            }

            // FIXME: propagate errors
            chain_state
                .verify_block(block_hash, block)
                .expect(&format!("Block {} ({}) failed to verify", block_hash, date));

            if date.get_epochid() >= first_unstable_epoch {
                // This block is not part of a stable epoch yet and could
                // be rolled back. Therefore we can't pack this epoch
                // yet. Instead we write this block to disk separately.
                let block_hash = types::header_to_blockhash(&block_hash);
                blob::write(&storage.read().unwrap(), &block_hash, block_raw.as_ref()).unwrap();
            } else {
                // If this is the epoch genesis block, start writing a new epoch pack.
                if date.is_boundary() {
                    epoch_writer_state = Some(EpochWriterState {
                        epoch_id: date.get_epochid(),
                        writer: pack::packwriter_init(&storage_config).unwrap(),
                        write_start_time: SystemTime::now(),
                        blobs_to_delete: vec![],
                    });
                }

                // And append the block to the epoch pack.
                if let Some(epoch_writer_state) = epoch_writer_state.as_mut() {
                    epoch_writer_state
                        .writer
                        .append(&types::header_to_blockhash(&block_hash), block_raw.as_ref())
                        .unwrap();
                } else {
                    unreachable!();
                }
            }
        },
    )?;

    // Update the tip tag to point to the most recent block.
    tag::write(
        &storage.read().unwrap(),
        &tag::HEAD,
        chain_state.last_block.as_ref(),
    );

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
    genesis_data: &GenesisData,
    storage: Arc<RwLock<Storage>>,
    sync_once: bool,
) -> Result<()> {
    // recover and print the TIP of the network
    let mut tip_header = net.get_tip()?;

    loop {
        net_sync_to(net, net_cfg, genesis_data, storage.clone(), &tip_header)?;

        if sync_once {
            break;
        }

        tip_header = net.wait_for_new_tip(&tip_header.compute_hash())?;
    }

    Ok(())
}

// Create an epoch from a complete set of previously fetched blocks on
// disk.
fn maybe_create_epoch(
    storage: &mut Storage,
    genesis_data: &GenesisData,
    epoch_id: EpochId,
    last_block: &HeaderHash,
) -> Result<()> {
    if epoch_exists(&storage.config, epoch_id).unwrap() {
        return Ok(());
    }

    info!("Packing epoch {}", epoch_id);

    let mut epoch_writer_state = EpochWriterState {
        epoch_id,
        writer: pack::packwriter_init(&storage.config).unwrap(),
        write_start_time: SystemTime::now(),
        blobs_to_delete: vec![],
    };

    let (end_of_prev_epoch, blocks) = get_unpacked_blocks_in_epoch(
        storage,
        last_block,
        epoch_writer_state.epoch_id,
        &mut epoch_writer_state.blobs_to_delete,
    );

    let mut chain_state =
        chain_state::restore_chain_state(storage, genesis_data, &end_of_prev_epoch)?;

    append_blocks_to_epoch_reverse(&mut epoch_writer_state, &mut chain_state, blocks)?;

    finish_epoch(storage, genesis_data, epoch_writer_state, &chain_state)?;

    Ok(())
}

fn append_blocks_to_epoch_reverse(
    epoch_writer_state: &mut EpochWriterState,
    chain_state: &mut ChainState,
    mut blocks: Vec<(HeaderHash, RawBlock, Block)>,
) -> Result<()> {
    while let Some((hash, block_raw, block)) = blocks.pop() {
        chain_state.verify_block(&hash, &block)?;
        epoch_writer_state
            .writer
            .append(&types::header_to_blockhash(&hash), block_raw.as_ref())
            .unwrap();
    }

    Ok(())
}

fn get_unpacked_blocks_in_epoch(
    storage: &Storage,
    last_block: &HeaderHash,
    epoch_id: EpochId,
    blobs_to_delete: &mut Vec<HeaderHash>,
) -> (HeaderHash, Vec<(HeaderHash, RawBlock, Block)>) {
    let mut cur_hash = last_block.clone();
    let mut blocks = vec![];
    loop {
        let block_raw = storage.read_block(&cur_hash.clone().into()).unwrap();
        blobs_to_delete.push(cur_hash.clone());
        let block = block_raw.decode().unwrap();
        let (blockdate, prev_hash) = {
            let hdr = block.header();
            (hdr.blockdate(), hdr.previous_header())
        };
        assert!(blockdate.get_epochid() == epoch_id);
        blocks.push((cur_hash, block_raw, block));
        cur_hash = prev_hash;
        if blockdate.is_boundary() {
            break;
        }
    }
    (cur_hash, blocks)
}

fn finish_epoch(
    storage: &mut Storage,
    genesis_data: &GenesisData,
    epoch_writer_state: EpochWriterState,
    chain_state: &ChainState,
) -> Result<()> {
    let epoch_id = epoch_writer_state.epoch_id;
    let (packhash, index) = pack::packwriter_finalize(&storage.config, epoch_writer_state.writer);
    let (lookup, tmpfile) = pack::create_index(&storage, &index);
    tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash))?;
    storage.add_lookup(packhash, lookup);
    let epoch_time_elapsed = epoch_writer_state.write_start_time.elapsed().unwrap();

    if epoch_id > 0 {
        assert!(
            epoch_exists(&storage.config, epoch_id - 1)?,
            "Attempted finish_epoch() with non-existent previous epoch (ID {}, previous' ID {})",
            epoch_id,
            epoch_id - 1
        );
    }

    assert_eq!(chain_state.last_date.unwrap().get_epochid(), epoch_id);

    epoch::epoch_create(
        &storage,
        &packhash,
        epoch_id,
        Some((chain_state, genesis_data)),
    );

    info!(
        "=> pack {} written for epoch {} in {}",
        hex::encode(&packhash[..]),
        epoch_id,
        duration_print(epoch_time_elapsed)
    );

    for hash in &epoch_writer_state.blobs_to_delete {
        debug!("removing blob {}", hash);
        blob::remove(&storage, &hash.clone().into());
    }

    Ok(())
}

pub fn get_peer(blockchain: &str, cfg: &net::Config, native: bool) -> Peer {
    for peer in cfg.peers.get_peers().iter() {
        if (native && peer.peer().is_native()) || (!native && peer.peer().is_http()) {
            return Peer::new(
                String::from(blockchain),
                peer.name().to_owned(),
                peer.peer().clone(),
                cfg.protocol_magic,
            )
            .unwrap();
        }
    }

    panic!("no peer to connect to")
}

pub fn get_chain_state_at_end_of(
    storage: &Storage,
    epoch_id: EpochId,
    genesis_data: &GenesisData,
) -> Result<ChainState> {
    Ok(chain_state::read_chain_state(
        storage,
        genesis_data,
        &chain_state::get_last_block_of_epoch(storage, epoch_id)?,
    )?)
}
