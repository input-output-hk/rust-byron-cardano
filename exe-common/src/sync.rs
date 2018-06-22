use blockchain;
use config::net;
use network::{api, Peer, api::Api};
use storage;
use storage::types::PackHash;

pub fn net_sync_fast(network: String, mut storage: storage::Storage) {
    let netcfg_file = storage.config.get_config_file();
    let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
    let mut net = get_native_peer(network, &net_cfg);

    //let mut our_tip = tag::read_hash(&storage, &"TIP".to_string()).unwrap_or(genesis.clone());

    // recover and print the TIP of the network
    let mbh = net.get_tip().unwrap();
    let network_tip = mbh.compute_hash();
    let network_slotid = mbh.get_blockdate();

    println!("Configured genesis   : {}", net_cfg.genesis);
    println!("Configured genesis-1 : {}", net_cfg.genesis_prev);
    println!("Network TIP is       : {}", network_tip);
    println!("Network TIP slotid   : {}", network_slotid);

    // start from our tip towards network tip
    /*
    if &network_tip == &our_tip {
        println!("Qapla ! already synchronised");
        return ();
    }
    */

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
    println!(
        "latest known epoch {} hash={:?}",
        latest_known_epoch_id, mstart_hash
    );

    let mut download_epoch_id = latest_known_epoch_id;
    let mut download_prev_hash = prev_hash.clone();
    let mut download_start_hash = mstart_hash.or(Some(prev_hash)).unwrap();

    while download_epoch_id < network_slotid.get_epochid() {
        println!(
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
        download_prev_hash = result.last_header_hash.clone();
        download_start_hash = result.next_epoch_hash.unwrap_or(result.last_header_hash);
        download_epoch_id += 1;
    }
}

pub fn net_sync_faster(network: String, mut storage: storage::Storage) {
    let netcfg_file = storage.config.get_config_file();
    let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
    let mut net = get_http_peer(network, &net_cfg);

    //let mut our_tip = tag::read_hash(&storage, &"TIP".to_string()).unwrap_or(genesis.clone());

    println!("Configured genesis   : {}", net_cfg.genesis);
    println!("Configured genesis-1 : {}", net_cfg.genesis_prev);

    // find the earliest epoch we know about starting from network_slotid
    let (latest_known_epoch_id, mstart_hash, prev_hash) =
        match find_earliest_epoch(&storage, net_cfg.epoch_start, 100) {
            // TODO
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
    println!(
        "latest known epoch {} hash={:?}",
        latest_known_epoch_id, mstart_hash
    );

    let mut download_epoch_id = latest_known_epoch_id;
    let mut download_prev_hash = prev_hash.clone();
    let mut download_start_hash = mstart_hash.or(Some(prev_hash)).unwrap();

    while download_epoch_id < 46 {
        println!(
            "downloading epoch {} {}",
            download_epoch_id, download_start_hash
        );
        let fep = api::FetchEpochParams {
            epoch_id: download_epoch_id,
            start_header_hash: download_start_hash,
            previous_header_hash: download_prev_hash,
            upper_bound_hash: net_cfg.genesis_prev.clone(),
        };
        let result = net.fetch_epoch(&net_cfg, &mut storage, fep).unwrap();
        download_prev_hash = result.last_header_hash.clone();
        download_start_hash = result.next_epoch_hash.unwrap_or(result.last_header_hash);
        download_epoch_id += 1;
    }
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

// Return the chain of block headers starting at from's next block
// and terminating at to, unless this range represent a number
// of blocks greater than the limit imposed by the node we're talking to.
fn find_earliest_epoch(
    storage: &storage::Storage,
    minimum_epochid: cardano::block::EpochId,
    start_epochid: cardano::block::EpochId,
) -> Option<(cardano::block::EpochId, PackHash)> {
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
                println!("latest known epoch found is {}", epoch_id);
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

fn get_last_blockid(
    storage_config: &storage::config::StorageConfig,
    packref: &PackHash,
) -> Option<cardano::block::HeaderHash> {
    let mut reader = storage::pack::PackReader::init(&storage_config, packref);
    let mut last_blk_raw = None;

    while let Some(blk_raw) = reader.get_next() {
        last_blk_raw = Some(blk_raw);
    }
    if let Some(blk_raw) = last_blk_raw {
        let blk = blk_raw.decode().unwrap();
        let hdr = blk.get_header();
        println!("last_blockid: {} {}", hdr.compute_hash(), hdr.get_slotid());
        Some(hdr.compute_hash())
    } else {
        None
    }
}
