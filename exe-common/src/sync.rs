use cardano::block;
use config::net;
use network::{api, Peer, api::Api};
use storage;
use utils::*;

pub fn net_sync(net: &mut Api, net_cfg: &net::Config, mut storage: storage::Storage) {
    //let mut our_tip = tag::read_hash(&storage, &"TIP".to_string()).unwrap_or(genesis.clone());

    // recover and print the TIP of the network
    let mbh = net.get_tip().unwrap();
    let network_tip = mbh.compute_hash();
    let network_slotid = mbh.get_blockdate();

    info!("Configured genesis   : {}", net_cfg.genesis);
    info!("Configured genesis-1 : {}", net_cfg.genesis_prev);
    info!("Network TIP is       : {}", network_tip);
    info!("Network TIP slotid   : {}", network_slotid);

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

    let mut download_epoch_id = latest_known_epoch_id;
    let mut download_prev_hash = prev_hash.clone();
    let mut download_start_hash = mstart_hash.or(Some(prev_hash)).unwrap();

    while download_epoch_id < network_slotid.get_epochid() {
        info!(
            "downloading epoch {} {}",
            download_epoch_id, download_start_hash
        );
        let fep = api::FetchEpochParams {
            epoch_id: download_epoch_id,
            start_header_hash: download_start_hash,
            previous_header_hash: download_prev_hash,
            upper_bound_hash: network_tip.clone(),
        };
        let result = net.fetch_epoch(&net_cfg, &mut storage, fep).unwrap();

        storage::tag::write(&storage, &storage::tag::get_epoch_tag(download_epoch_id), &result.packhash[..]);

        storage::epoch::epoch_create(&storage.config, &result.packhash, download_epoch_id);

        storage::refpack_epoch_pack(&storage, &format!("EPOCH_{}", download_epoch_id)).unwrap();

        download_prev_hash = result.last_header_hash.clone();
        download_start_hash = result.next_epoch_hash.unwrap_or(result.last_header_hash);
        download_epoch_id += 1;
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
