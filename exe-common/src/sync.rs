use config::net;
use network::{Peer, api::Api, api::BlockRef};
use storage::{self, tag, Error, block_read};
use cardano::block::{BlockDate, EpochId, HeaderHash};
use cardano::util::{hex};
use std::time::{SystemTime, Duration};

fn duration_print(d: Duration) -> String {
    format!("{}.{:03} seconds", d.as_secs(), d.subsec_millis())
}

pub fn net_sync(net: &mut Api, net_cfg: &net::Config, storage: storage::Storage) {
    // recover and print the TIP of the network
    let tip_header = net.get_tip().unwrap();
    let tip = BlockRef {
        hash: tip_header.compute_hash(),
        parent: tip_header.get_previous_header(),
        date: tip_header.get_blockdate()
    };

    info!("Configured genesis   : {}", net_cfg.genesis);
    info!("Configured genesis-1 : {}", net_cfg.genesis_prev);
    info!("Network TIP is       : {} <- {}", tip.hash, tip_header.get_previous_header());
    info!("Network TIP slotid   : {}", tip.date);

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
            BlockDate::Normal(d) => if d.slotid <= net_cfg.k { 1 } else { 0 }
        };
    info!("First unstable epoch : {}", first_unstable_epoch);

    let mut cur_epoch_state : Option<(EpochId, storage::pack::PackWriter, SystemTime)> = None;

    let mut last_block : Option<HeaderHash> = None;

    // If our tip is in an epoch that has become stable, we now need
    // to pack it. So read the previously fetched blocks in this epoch
    // and prepend them to the incoming blocks.
    if our_tip.0.date.get_epochid() < first_unstable_epoch {

        let epoch_id = our_tip.0.date.get_epochid();
        let mut writer = storage::pack::PackWriter::init(&storage.config);
        let epoch_time_start = SystemTime::now();

        let mut cur_hash = our_tip.0.hash.clone();
        let mut blocks = vec![];
        loop {
            let block_raw = block_read(&storage, cur_hash.bytes()).unwrap();
            let block = block_raw.decode().unwrap();
            let hdr = block.get_header();
            assert!(hdr.get_blockdate().get_epochid() == epoch_id);
            blocks.push((storage::types::header_to_blockhash(&cur_hash), block_raw));
            if hdr.get_blockdate().is_genesis() { break }
            cur_hash = hdr.get_previous_header();
        }

        while let Some((hash, block_raw)) = blocks.pop() {
            writer.append(&hash, block_raw.as_ref());
        }

        cur_epoch_state = Some((epoch_id, writer, epoch_time_start));

        // FIXME: in fact we may need to pack the previous epoch as
        // well. (If tip.slotid < w, it won't have been packed yet).
    }

    net.get_blocks(&our_tip.0, our_tip.1, &tip, &mut |block_hash, block, block_raw| {
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

                // Checkpoint the tip so we don't have to refetch
                // everything if we get interrupted.
                storage::tag::write(&storage, &tag::HEAD, &last_block.as_ref().unwrap().bytes()[..]);

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
        storage::tag::write(&storage, &tag::HEAD,
                            &storage::types::header_to_blockhash(&block_hash));
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
