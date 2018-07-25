use config::net;
use network::{Peer, api::Api, api::BlockRef};
use storage;
use cardano::block::{BlockDate, EpochId};
use cardano::util::{hex};
use std::time::{SystemTime, Duration};

fn duration_print(d: Duration) -> String {
    format!("{}.{:03} seconds", d.as_secs(), d.subsec_millis())
}

pub fn net_sync(net: &mut Api, net_cfg: &net::Config, storage: storage::Storage) {
    // recover and print the TIP of the network
    let tip_header = net.get_tip().unwrap();
    let tip = BlockRef { hash: tip_header.compute_hash(), date: tip_header.get_blockdate() };

    info!("Configured genesis   : {}", net_cfg.genesis);
    info!("Configured genesis-1 : {}", net_cfg.genesis_prev);
    info!("Network TIP is       : {} <- {}", tip.hash, tip_header.get_previous_header());
    info!("Network TIP slotid   : {}", tip.date);

    let our_tip = BlockRef { hash: net_cfg.genesis.clone(), date: BlockDate::Genesis(net_cfg.epoch_start) };

    /*
    // find the earliest epoch we know about starting from network_slotid
    let (latest_known_epoch_id, mstart_hash, prev_hash) =
        match find_earliest_epoch(&storage, net_cfg.epoch_start, network_slotid.get_epochid()) {
            None => (
                net_cfg.epoch_start,
                Some(net_cfg.genesis.clone()),
                net_cfg.genesis_prev.clone(),
            ),
            Some((found_epoch_id, packhash)) => (
                found_epoch_id + 1,
                None,
                get_last_blockid(&storage.config, &packhash).unwrap(),
            ),
        };
    info!(
        "latest known epoch {} hash={:?}",
        latest_known_epoch_id, mstart_hash
    );
    */

    // Determine whether the previous epoch is stable yet. Note: This
    // assumes that k is smaller than the number of blocks in an
    // epoch.
    let first_unstable_epoch = tip.date.get_epochid() -
        match tip.date {
            BlockDate::Genesis(_) => 1,
            BlockDate::Normal(d) => if d.slotid <= net_cfg.k { 1 } else { 0 }
        };
    info!("First unstable epoch : {}", first_unstable_epoch);

    let mut cur_epoch_state : Option<(EpochId, storage::pack::PackWriter, SystemTime)> = None;

    let mut last_block = None;

    net.get_blocks(&our_tip, true, &tip, &mut |block_hash, block, block_raw| {
        let date = block.get_header().get_blockdate();

        // Flush the previous epoch (if any).
        if date.is_genesis() {
            if let Some((epoch_id, writer, epoch_time_start)) = cur_epoch_state.as_mut() {
                let (packhash, index) = writer.finalize();
                let (_, tmpfile) = storage::pack::create_index(&storage, &index);
                tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash)).unwrap();
                let epoch_time_elapsed = epoch_time_start.elapsed().unwrap();

                storage::tag::write(&storage, &storage::tag::get_epoch_tag(*epoch_id), &packhash[..]);

                storage::epoch::epoch_create(&storage.config, &packhash, *epoch_id);

                storage::refpack_epoch_pack(&storage, &format!("EPOCH_{}", epoch_id)).unwrap();

                // Checkpoint the tip so we don't have to refetch
                // everything if we get interrupted.
                storage::tag::write(&storage, &"tip", &packhash[..]);

                info!("=> pack {} written for epoch {} in {}", hex::encode(&packhash[..]),
                      epoch_id, duration_print(epoch_time_elapsed));
            }
        }

        if date.get_epochid() >= first_unstable_epoch {
            // This block is not part of a stable epoch yet and could
            // be rolled back. Therefore we can't pack this epoch
            // yet. Instead we write this block to disk separately.
            let block_hash = storage::types::header_to_blockhash(&block_hash);
            storage::blob::write(&storage, &block_hash, block_raw.as_ref()).unwrap();
        } else {

            // If this is the epoch genesis block, start writing a new epoch pack.
            if date.is_genesis() {
                cur_epoch_state = Some((date.get_epochid(), storage::pack::PackWriter::init(&storage.config), SystemTime::now()));
            }

            // And append the block to the epoch pack.
            let (_, writer, _) = &mut cur_epoch_state.as_mut().unwrap();
            writer.append(&storage::types::header_to_blockhash(&block_hash), block_raw.as_ref());
        }

        last_block = Some(block_hash.clone());
    });

    // Update the tip tag to point to the most recent block.
    if let Some(block_hash) = last_block {
        storage::tag::write(&storage, &"TIP",
                            &storage::types::header_to_blockhash(&block_hash));
    }
}

pub fn net_sync_http(network: String, storage: storage::Storage) {
    let netcfg_file = storage.config.get_config_file();
    let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
    let mut net = get_http_peer(network, &net_cfg);
    net_sync(&mut net, &net_cfg, storage)
}

pub fn net_sync_native(network: String, storage: storage::Storage) {
    let netcfg_file = storage.config.get_config_file();
    let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
    let mut net = get_native_peer(network, &net_cfg);
    net_sync(&mut net, &net_cfg, storage)
}

pub fn get_http_peer(blockchain: String, cfg: &net::Config) -> Peer {
    for peer in cfg.peers.iter() {
        if peer.is_http() {
            return Peer::new(
                blockchain,
                peer.name().to_owned(),
                peer.peer().clone(),
                cfg.protocol_magic,
            ).unwrap();
        }
    }

    panic!("no http peer to connect to")
}

pub fn get_native_peer(blockchain: String, cfg: &net::Config) -> Peer {
    for peer in cfg.peers.iter() {
        if peer.is_native() {
            return Peer::new(
                blockchain,
                peer.name().to_owned(),
                peer.peer().clone(),
                cfg.protocol_magic,
            ).unwrap();
        }
    }

    panic!("no native peer to connect to")
}
