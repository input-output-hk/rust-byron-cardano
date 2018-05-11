use wallet_crypto::util::{hex};
use wallet_crypto::{cbor};
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use config::{Config};
use storage;
use storage::{blob, tag};
use storage::types::{PackHash};
use storage::tag::{HEAD};
use rand;
use std::net::TcpStream;
use std::time::{SystemTime, Duration};
use blockchain;

use protocol;
use protocol::command::*;

pub struct Network(protocol::Connection<TcpStream>);
impl Network {
    fn new(cfg: &Config) -> Self {
        let drg_seed = rand::random();
        let mut hs = protocol::packet::Handshake::default();
        hs.protocol_magic = cfg.protocol_magic;

        let stream = TcpStream::connect(cfg.network_domain.clone()).unwrap();
        stream.set_nodelay(true).unwrap();

        let conn = protocol::ntt::Connection::handshake(drg_seed, stream).unwrap();
        let mut conne = protocol::Connection::new(conn);
        conne.handshake(&hs).unwrap();
        Network(conne)
    }
}

// TODO return BlockHeader not MainBlockHeader
fn network_get_head_header(storage: &storage::Storage, net: &mut Network) -> blockchain::BlockHeader {
    let block_headers = GetBlockHeader::tip().execute(&mut net.0).expect("to get one header at least");
    if block_headers.len() != 1 {
        panic!("get head header return more than 1 header")
    }
    let mbh = block_headers[0].clone();
    tag::write(&storage, &HEAD.to_string(), mbh.get_previous_header().as_ref());
    mbh
}

fn network_get_blocks_headers(net: &mut Network, from: &blockchain::HeaderHash, to: &blockchain::HeaderHash) -> Vec<blockchain::BlockHeader> {
    let mbh = GetBlockHeader::range(&vec![from.clone()], to.clone()).execute(&mut net.0).expect("to get one header at least");
    mbh
}

fn duration_print(d: Duration) -> String {
    format!("{}.{:03} seconds", d.as_secs(), d.subsec_millis())
}

fn decode_hash_hex(s: &String) -> blockchain::HeaderHash {
    blockchain::HeaderHash::from_slice(&hex::decode(&s).unwrap()).expect("blockid invalid")
}

fn find_earliest_epoch(storage: &storage::Storage, start_epochid: blockchain::EpochId)
        -> Option<(blockchain::EpochId, PackHash)> {
    let mut epoch_id = start_epochid;
    loop {
        match tag::read_hash(storage, &tag::get_epoch_tag(epoch_id)) {
            None => {},
            Some(h) => {
                println!("latest known epoch found is {}", epoch_id);
                return Some((epoch_id, h.into_bytes()))
            },
        }

        if epoch_id > 0 {
            epoch_id -= 1
        } else {
            return None
        }
    }
}

// download a complete epoch and create a new pack with all the blocks
//
// x_start_hash should reference an epoch genesis block, and latest_hash
// should gives the latest known hash of the chain.
fn download_epoch(storage: &storage::Storage, mut net: &mut Network,
                  epoch_id: blockchain::EpochId,
                  x_start_hash: &blockchain::HeaderHash,
                  latest_hash: &blockchain::HeaderHash) -> blockchain::HeaderHash {
    let mut start_hash = x_start_hash.clone();
    let mut found_epoch_boundary = None;
    let mut writer = storage::pack::PackWriter::init(&storage.config);
    let mut last_packed = None;
    let epoch_time_start = SystemTime::now();
    let mut expected_slotid = 0;
    loop {
        println!("  ### slotid={} from={}", expected_slotid, start_hash);
        let hdr_time_start = SystemTime::now();
        let block_headers = network_get_blocks_headers(&mut net, &start_hash, latest_hash);
        let hdr_time_elapsed = hdr_time_start.elapsed().unwrap();
        println!("  got {} headers in {}", block_headers.len(), duration_print(hdr_time_elapsed));

        let mut start = 0;
        let mut end = block_headers.len() - 1;
        while end > start && block_headers[start].get_slotid().epoch > epoch_id {
            start += 1
        }
        while end > start && block_headers[end].get_slotid().epoch < epoch_id {
            end -= 1
        }

        if start > 0 {
            println!("  found next epoch");
            found_epoch_boundary = Some(block_headers[start-1].compute_hash());
        }
        let latest_block = &block_headers[start];
        let first_block = &block_headers[end];

        let blk_time_start = SystemTime::now();
        let blocks_raw = GetBlock::from(&first_block.compute_hash(), &latest_block.compute_hash())
                                .execute(&mut net.0)
                                .expect("to get one block at least");
        let blk_time_elapsed = blk_time_start.elapsed().unwrap();
        println!("  got {} blocks in {}", blocks_raw.len(), duration_print(blk_time_elapsed));

        for block_raw in blocks_raw.iter() {
            let block : blockchain::Block = cbor::decode_from_cbor(&block_raw).unwrap();
            let hdr = block.get_header();
            let slot = hdr.get_slotid();
            let blockhash = hdr.compute_hash();
            if slot.epoch != epoch_id {
                panic!("trying to append a block of different epoch id {}", slot.epoch)
            }
            match last_packed {
                None    => {},
                Some(ref p) => { if p == &blockhash { continue; } else {} },
            }
            if slot.slotid == expected_slotid {
                expected_slotid += 1
            } else {
                println!("  WARNING: not contiguous. slot id {} found, expected {}", slot.slotid, expected_slotid);
                expected_slotid = slot.slotid + 1
            }

            writer.append(&storage::types::header_to_blockhash(&blockhash), block_raw);
            last_packed = Some(blockhash);
        }
        // println!("packing {}", slot);
        match last_packed {
            None    => {},
            Some(ref p) => { start_hash = p.clone() },
        }

        match found_epoch_boundary {
            None    => {},
            Some(b) => {
                println!("=> packing finished {} slotids", expected_slotid);
                // write packfile
                let (packhash, index) = writer.finalize();
                let (_, tmpfile) = storage::pack::create_index(storage, &index);
                tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash)).unwrap();
                let epoch_time_elapsed = epoch_time_start.elapsed().unwrap();
                println!("=> pack {} written for epoch {} in {}", hex::encode(&packhash[..]), epoch_id, duration_print(epoch_time_elapsed));
                tag::write(storage, &tag::get_epoch_tag(epoch_id), &packhash[..]);
                return b
            },
        }
    }
}

fn net_sync_fast(config: Config) {
    let storage = config.get_storage().unwrap();
    let mut net = Network::new(&config);

    let genesis = decode_hash_hex(&config.network_genesis);

    //let mut our_tip = tag::read_hash(&storage, &"TIP".to_string()).unwrap_or(genesis.clone());

    // recover and print the TIP of the network
    let mbh = network_get_head_header(&storage, &mut net);
    let network_tip = mbh.compute_hash();
    let network_slotid = mbh.get_slotid();

    println!("Configured genesis : {}", genesis);
    println!("Network TIP is     : {}", network_tip);
    println!("Network TIP slotid : {}", network_slotid);

    // start from our tip towards network tip
    /*
    if &network_tip == &our_tip {
        println!("Qapla ! already synchronised");
        return ();
    }
    */

    // find the earliest epoch we know about starting from network_slotid
    let (latest_known_epoch_id, start_hash) = match find_earliest_epoch(&storage, network_slotid.epoch) {
        None => { (0, genesis) },
        Some(r) => { get_last_blockid(&storage.config, &r.1).unwrap() }
    };
    println!("latest known epoch {} hash={}", latest_known_epoch_id, start_hash);

    let mut download_epoch_id = latest_known_epoch_id;
    let mut download_start_hash = start_hash;
    while download_epoch_id < network_slotid.epoch {
        println!("downloading epoch {}", download_epoch_id);
        download_start_hash = download_epoch(&storage, &mut net, download_epoch_id, &download_start_hash, &network_tip);
        download_epoch_id += 1;
    }

}

impl HasCommand for Network {
    type Output = ();

    fn clap_options<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name("network")
            .about("blockchain network operation")
            .subcommand(SubCommand::with_name("get-block-header")
                .about("get a given block header")
            )
            .subcommand(SubCommand::with_name("get-block")
                .about("get a given block")
                .arg(Arg::with_name("blockid").help("hexadecimal encoded block id").index(1).required(true))
            )
            .subcommand(SubCommand::with_name("sync")
                .about("get the next block repeatedly")
            )
    }

    fn run(config: Config, args: &ArgMatches) -> Self::Output {
        match args.subcommand() {
            ("get-block-header", _) => {
                let mut net = Network::new(&config);
                let storage = config.get_storage().unwrap();
                let mbh = network_get_head_header(&storage, &mut net);
                println!("prv block header: {}", mbh.get_previous_header());
            },
            ("get-block", Some(opt)) => {
                let hh_hex = value_t!(opt.value_of("blockid"), String).unwrap();
                let hh_bytes = hex::decode(&hh_hex).unwrap();
                let hh = blockchain::HeaderHash::from_slice(&hh_bytes).expect("blockid invalid");
                let mut net = Network::new(&config);
                let mut b = GetBlock::only(&hh).execute(&mut net.0)
                    .expect("to get one block at least");

                let storage = config.get_storage().unwrap();
                blob::write(&storage, hh.bytes(), &b[0][..]).unwrap();
            },
            ("sync", _) => net_sync_fast(config),
            _ => {
                println!("{}", args.usage());
                ::std::process::exit(1);
            },
        }
    }
}


fn get_last_blockid(storage_config: &storage::config::StorageConfig, packref: &PackHash) -> Option<(blockchain::EpochId, blockchain::HeaderHash)> {
    let mut reader = storage::pack::PackReader::init(&storage_config, packref);
    let mut last_blk_raw = None;

    while let Some(blk_raw) = reader.get_next() {
        last_blk_raw = Some(blk_raw);
    }
    if let Some(blk_raw) = last_blk_raw {
        let blk : blockchain::Block = cbor::decode_from_cbor(&blk_raw[..]).unwrap();
        let hdr = blk.get_header();
        Some((hdr.get_slotid().epoch + 1, hdr.compute_hash()))
    } else {
        None
    }
}