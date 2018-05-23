use wallet_crypto::{cbor, util::{hex}};
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use storage;
use storage::{blob, tag, Storage};
use storage::types::{PackHash};
//use storage::tag::{HEAD};
use std::time::{SystemTime, Duration};
use blockchain;
use blockchain::{BlockDate, SlotId};
use config::{Config};

use protocol::command::*;
use exe_common::{config::{net}, network::{Network}};

pub fn new_network(cfg: &net::Config) -> Network {
    Network::new(cfg.protocol_magic, &cfg.domain.clone())
}

// TODO return BlockHeader not MainBlockHeader
fn network_get_head_header(storage: &Storage, net: &mut Network) -> blockchain::BlockHeader {
    let block_headers_raw = GetBlockHeader::tip().execute(&mut net.0).expect("to get one header at least");

    let block_headers = block_headers_raw.decode().unwrap();

    if block_headers.len() != 1 {
        panic!("get head header return more than 1 header")
    }
    let mbh = block_headers[0].clone();
    //tag::write(&storage, &HEAD.to_string(), mbh.get_previous_header().as_ref());
    mbh
}

// Return the chain of block headers starting at from's next block
// and terminating at to, unless this range represent a number
// of blocks greater than the limit imposed by the node we're talking to.
fn network_get_blocks_headers(net: &mut Network, from: &blockchain::HeaderHash, to: &blockchain::HeaderHash) -> blockchain::RawBlockHeaderMultiple {
    let mbh = GetBlockHeader::range(&vec![from.clone()], to.clone()).execute(&mut net.0).expect("to get one header at least");
    mbh
}

fn duration_print(d: Duration) -> String {
    format!("{}.{:03} seconds", d.as_secs(), d.subsec_millis())
}

fn find_earliest_epoch(storage: &storage::Storage, minimum_epochid: blockchain::EpochId, start_epochid: blockchain::EpochId)
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

        if epoch_id > minimum_epochid {
            epoch_id -= 1
        } else {
            return None
        }
    }
}

// download a complete epoch and create a new pack with all the blocks
//
// x_start_hash should reference an epoch genesis block, and tip_hash
// should gives the latest known hash of the chain.
fn download_epoch(storage: &storage::Storage, mut net: &mut Network,
                  epoch_id: blockchain::EpochId,
                  x_start_hash: &blockchain::HeaderHash,
                  x_previous_headerhash: &blockchain::HeaderHash,
                  tip_hash: &blockchain::HeaderHash) -> (blockchain::HeaderHash, blockchain::HeaderHash) {
    let mut start_hash = x_start_hash.clone();
    let mut found_epoch_boundary = None;
    let mut writer = storage::pack::PackWriter::init(&storage.config);
    let mut previous_headerhash = x_previous_headerhash.clone();
    let epoch_time_start = SystemTime::now();
    let mut expected_slotid = blockchain::BlockDate::Genesis(epoch_id);

    loop {
        println!("  ### slotid={} from={}", expected_slotid, start_hash);
        let metrics = net.read_start();
        let block_headers_raw = network_get_blocks_headers(&mut net, &start_hash, tip_hash);
        let hdr_metrics = net.read_elapsed(&metrics);
        let block_headers = block_headers_raw.decode().unwrap();
        println!("  got {} headers  ( {} )", block_headers.len(), hdr_metrics);

        let mut start = 0;
        let mut end = block_headers.len() - 1;

        // if the earliest block headers we receive has an epoch
        // less than the expected epoch, we just fast skip
        // this set of headers and restart the loop with the
        // latest known hash
        if block_headers[start].get_blockdate().get_epochid() < epoch_id {
            start_hash = block_headers[start].compute_hash();
            println!("headers are of previous epochs, fast skip to {}", start_hash);
            continue;
        }

        while end >= start && block_headers[start].get_blockdate().get_epochid() > epoch_id {
            start += 1
        }
        while end > start && block_headers[end].get_blockdate().get_epochid() < epoch_id {
            end -= 1
        }

        if start > 0 {
            println!("  found next epoch");
            found_epoch_boundary = Some(block_headers[start-1].compute_hash());
        }
        let latest_block = &block_headers[start];
        let first_block = &block_headers[end];

        let download_start_hash = if first_block.get_blockdate() == expected_slotid {
            first_block.compute_hash()
        } else if first_block.get_blockdate() == expected_slotid.next() {
            first_block.get_previous_header()
        } else {
            panic!("not matching. gap")
        };

        let metrics = net.read_start();
        let blocks_raw = GetBlock::from(&download_start_hash, &latest_block.compute_hash())
                                .execute(&mut net.0)
                                .expect("to get one block at least");
        let blocks_metrics = net.read_elapsed(&metrics);
        println!("  got {} blocks  ( {} )", blocks_raw.len(), blocks_metrics);

        for block_raw in blocks_raw.iter() {
            let block = block_raw.decode().unwrap();
            let hdr = block.get_header();
            let date = hdr.get_blockdate();
            let blockhash = hdr.compute_hash();
            let block_previous_header = hdr.get_previous_header();

            if date.get_epochid() != epoch_id {
                panic!("trying to append a block of different epoch id {}", date.get_epochid())
            }

            if previous_headerhash != block_previous_header {
                panic!("previous header doesn't match: hash {} date {} got {} expected {}",
                       blockhash, date, block_previous_header, previous_headerhash)
            }

            if &date != &expected_slotid {
                println!("  WARNING: not contiguous. addr {} found, expected {} {}", date, expected_slotid, block_previous_header);
            }

            match date {
                BlockDate::Genesis(epoch) => {
                    expected_slotid = BlockDate::Normal(SlotId { epoch: epoch, slotid: 0 });
                },
                BlockDate::Normal(slotid) => {
                    expected_slotid = BlockDate::Normal(slotid.next());
                },
            }

            writer.append(&storage::types::header_to_blockhash(&blockhash), block_raw.as_ref());
            previous_headerhash = blockhash.clone();
        }
        // println!("packing {}", slot);
        start_hash = previous_headerhash.clone();

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
                return (previous_headerhash, b)
            },
        }
    }
}

fn net_sync_fast(storage: Storage) {
    let netcfg_file = storage.config.get_config_file();
    let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
    let mut net = new_network(&net_cfg);

    //let mut our_tip = tag::read_hash(&storage, &"TIP".to_string()).unwrap_or(genesis.clone());

    // recover and print the TIP of the network
    let mbh = network_get_head_header(&storage, &mut net);
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
    let (latest_known_epoch_id, mstart_hash, prev_hash) = match find_earliest_epoch(&storage, net_cfg.epoch_start, network_slotid.get_epochid()) {
        None => { (net_cfg.epoch_start, Some(net_cfg.genesis), net_cfg.genesis_prev) },
        Some((found_epoch_id, packhash)) => { (found_epoch_id + 1, None, get_last_blockid(&storage.config, &packhash).unwrap()) }
    };
    println!("latest known epoch {} hash={:?}", latest_known_epoch_id, mstart_hash);

    let mut download_epoch_id = latest_known_epoch_id;
    let mut download_prev_hash = prev_hash.clone();
    let mut download_start_hash = mstart_hash.or(Some(prev_hash)).unwrap();

    while download_epoch_id < network_slotid.get_epochid() {
        println!("downloading epoch {} {}", download_epoch_id, download_start_hash);
        let result = download_epoch(&storage, &mut net, download_epoch_id, &download_start_hash, &download_prev_hash, &network_tip);
        download_prev_hash = result.0;
        download_start_hash = result.1;
        download_epoch_id += 1;
    }
}

impl HasCommand for Network {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "blockchain";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("blockchain operations")
            .subcommand(SubCommand::with_name("new")
                .about("create a new blockchain, blockchain that can be shared between wallets and work independently from the wallet.")
                .arg(Arg::with_name("template")
                        .long("template").help("the template for the new blockchain").required(false)
                        .possible_values(&["mainnet", "testnet"]).default_value("mainnet"))
                .arg(Arg::with_name("name").help("the blockchain name").index(1).required(true))
            )
            .subcommand(SubCommand::with_name("get-block-header")
                .arg(Arg::with_name("name").help("the network name").index(1).required(true))
                .about("get a given block header. (deprecated will be replaced soon).")
            )
            .subcommand(SubCommand::with_name("get-block")
                .about("get a given block (deprecated will be replaced soon).")
                .arg(Arg::with_name("name").help("the network name").index(1).required(true))
                .arg(Arg::with_name("blockid").help("hexadecimal encoded block id").index(2).required(true))
            )
            .subcommand(SubCommand::with_name("sync")
                .about("get the next block repeatedly (deprecated will be replaced soon).")
                .arg(Arg::with_name("name").help("the network name").index(1).required(true))
            )
    }

    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        match args.subcommand() {
            ("new", Some(opts)) => {
                let net_cfg = match value_t!(opts.value_of("template"), String).unwrap().as_str() {
                    "mainnet" => { net::Config::mainnet() },
                    "testnet" => { net::Config::testnet() },
                    _         => {
                        // we do not support custom template yet.
                        // in the mean while the error is handled by clap
                        // (possible_values)
                        panic!("invalid template option")
                    }
                };
                let name = value_t!(opts.value_of("name"), String).unwrap();

                let mut config = Config::default();
                config.network = name;

                let storage_config = config.get_storage_config();
                let _ = Storage::init(&storage_config).unwrap();

                let network_file = storage_config.get_config_file();
                net_cfg.to_file(&network_file)
            },
            ("get-block-header", Some(opts)) => {
                let name = value_t!(opts.value_of("name"), String).unwrap();
                let mut config = Config::default();
                config.network = name;
                let netcfg_file = config.get_storage_config().get_config_file();
                let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
                let mut net = new_network(&net_cfg);
                let storage = config.get_storage().unwrap();
                let mbh = network_get_head_header(&storage, &mut net);
                println!("prv block header: {}", mbh.get_previous_header());
            },
            ("get-block", Some(opts)) => {
                let name = value_t!(opts.value_of("name"), String).unwrap();
                let mut config = Config::default();
                config.network = name;
                let hh_hex = value_t!(opts.value_of("blockid"), String).unwrap();
                let hh_bytes = hex::decode(&hh_hex).unwrap();
                let hh = blockchain::HeaderHash::from_slice(&hh_bytes).expect("blockid invalid");
                let netcfg_file = config.get_storage_config().get_config_file();
                let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
                let mut net = new_network(&net_cfg);
                let mut b = GetBlock::only(&hh).execute(&mut net.0)
                    .expect("to get one block at least");

                let storage = config.get_storage().unwrap();
                blob::write(&storage, hh.bytes(), b[0].as_ref()).unwrap();
            },
            ("sync", Some(opts)) => {
                let name = value_t!(opts.value_of("name"), String).unwrap();
                let mut config = Config::default();
                config.network = name;
                net_sync_fast(config.get_storage().unwrap())
            },
            _ => {
                println!("{}", args.usage());
                ::std::process::exit(1);
            },
        }
    }
}


fn get_last_blockid(storage_config: &storage::config::StorageConfig, packref: &PackHash) -> Option<blockchain::HeaderHash> {
    let mut reader = storage::pack::PackReader::init(&storage_config, packref);
    let mut last_blk_raw = None;

    while let Some(blk_raw) = reader.get_next() {
        last_blk_raw = Some(blk_raw);
    }
    if let Some(blk_raw) = last_blk_raw {
        let blk : blockchain::Block = cbor::decode_from_cbor(&blk_raw[..]).unwrap();
        let hdr = blk.get_header();
        println!("last_blockid: {} {}", hdr.compute_hash(), hdr.get_slotid());
        Some(hdr.compute_hash())
    } else {
        None
    }
}
