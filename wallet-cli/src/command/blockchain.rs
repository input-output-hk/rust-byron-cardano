use wallet_crypto::{cbor, util::{hex}};
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use storage;
use storage::{blob, tag, Storage};
use storage::types::{PackHash};
use storage::{pack_blobs, block_location, block_read_location, pack, PackParameters};
//use storage::tag::{HEAD};
use std::time::{SystemTime, Duration};
use blockchain;
use blockchain::{BlockDate, SlotId};
use config::{Config};
use std::io::{Write, stdout};

use protocol::command::*;
use exe_common::{config::{net}, network::{Network}};

use command::pretty;

pub fn new_network(cfg: &net::Config) -> Network {
    Network::new(cfg.protocol_magic, &cfg.domain.clone())
}

// TODO return BlockHeader not MainBlockHeader
fn network_get_head_header(_storage: &Storage, net: &mut Network) -> blockchain::BlockHeader {
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

    let debug = false;

    loop {
        println!("  ### slotid={} from={}", expected_slotid, start_hash);
        let metrics = net.read_start();
        let block_headers_raw = network_get_blocks_headers(&mut net, &start_hash, tip_hash);
        let hdr_metrics = net.read_elapsed(&metrics);
        let block_headers = block_headers_raw.decode().unwrap();
        println!("  got {} headers  ( {} )", block_headers.len(), hdr_metrics);

        let mut start = 0;
        let mut end = block_headers.len() - 1;

        if debug {
            println!("  asked {} to {}", start_hash, tip_hash);
            println!("  start {} {} <- {}", block_headers[start].compute_hash(),  block_headers[start].get_blockdate(), block_headers[start].get_previous_header());
            println!("  end   {} {} <- {}", block_headers[end].compute_hash(), block_headers[end].get_blockdate(), block_headers[end].get_previous_header());
        }

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

        if debug {
            println!("  hdr latest {} {}", latest_block.compute_hash(), latest_block.get_blockdate());
            println!("  hdr first  {} {}", first_block.compute_hash(), first_block.get_blockdate());
        }

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

        if debug {
            let first_block = blocks_raw[0].decode().unwrap();
            let first_block_hdr = first_block.get_header();
            println!("first block {} {} prev {}", first_block_hdr.compute_hash(), first_block_hdr.get_blockdate(), first_block_hdr.get_previous_header());
        }

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

fn blockchain_name_arg<'a, 'b>(index: u64) -> Arg<'a,'b> {
    Arg::with_name("name")
        .help("the blockchain name")
        .index(index)
        .required(true)
}

fn resolv_network_by_name<'a>(opts: &ArgMatches<'a>) -> Config {
    let name = value_t!(opts.value_of("name"), String).unwrap();
    let mut config = Config::default();
    config.network = name;
    config
}

// TODO: rename Network to Blockchain?
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
                .arg(blockchain_name_arg(1))
            )
            .subcommand(SubCommand::with_name("get-block-header")
                .arg(blockchain_name_arg(1))
                .about("get a given block header. (deprecated will be replaced soon).")
            )
            .subcommand(SubCommand::with_name("get-block")
                .about("get a given block (deprecated will be replaced soon).")
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("blockid").help("hexadecimal encoded block id").index(2).required(true))
            )
            .subcommand(SubCommand::with_name("sync")
                .about("get the next block repeatedly (deprecated will be replaced soon).")
                .arg(blockchain_name_arg(1))
            )
            .subcommand(SubCommand::with_name("cat")
                .about("show content of a block")
                .arg(Arg::with_name("noparse").long("raw").help("cat the binary encoded block, no pretty print"))
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("blockid").help("hexadecimal encoded block id").index(2).required(true))
            )
            .subcommand(SubCommand::with_name("debug-index")
                .about("internal debug command")
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("packhash").help("pack to query").index(2))
            )
            .subcommand(SubCommand::with_name("debug-pack")
                .about("internal debug command")
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("packhash").help("pack to query").index(2))
            )
            .subcommand(SubCommand::with_name("re-index")
                .about("internal re-index command")
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("packhash").help("pack to re-index").index(2).required(true))
            )
            .subcommand(SubCommand::with_name("pack")
                .about("internal pack command")
                .arg(Arg::with_name("preserve-blobs").long("keep").help("keep what is being packed in its original state"))
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("range").help("<tag|ref>..<tag|ref>").index(2).required(false))
            )
            .subcommand(SubCommand::with_name("epoch-refpack")
                .about("generate the refpack of a given epoch")
                .arg(Arg::with_name("epoch").help("The epoch to generate the refpack").index(1).required(true))
            )
            .subcommand(SubCommand::with_name("unpack")
                .about("internal unpack command")
                .arg(Arg::with_name("preserve-packs").long("keep").help("keep what is being unpacked in its original state"))
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("packhash").help("pack to query").index(2))
            )
            .subcommand(SubCommand::with_name("integrity-check")
                .about("check the integrity of the blockchain")
            )
            .subcommand(SubCommand::with_name("is-pack-epoch")
                .about("internal check to see if a pack is a valid epoch-pack")
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("packhash").help("pack to query").index(2))
                .arg(Arg::with_name("previoushash").help("pack to query").index(3))
                .arg(Arg::with_name("epoch-id").help("pack to query").index(4))
            )
            .subcommand(SubCommand::with_name("tag")
                .about("show content of a tag or set a tag")
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("tag-name").help("name of the tag").index(2).required(true))
                .arg(Arg::with_name("tag-value").help("value to set to the given tag").index(3).required(false))
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
                let config = resolv_network_by_name(&opts);
                let storage_config = config.get_storage_config();
                let _ = Storage::init(&storage_config).unwrap();

                let network_file = storage_config.get_config_file();
                net_cfg.to_file(&network_file)
            },
            ("get-block-header", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let netcfg_file = config.get_storage_config().get_config_file();
                let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
                let mut net = new_network(&net_cfg);
                let storage = config.get_storage().unwrap();
                let mbh = network_get_head_header(&storage, &mut net);
                println!("prv block header: {}", mbh.get_previous_header());
            },
            ("get-block", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
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
                let config = resolv_network_by_name(&opts);
                net_sync_fast(config.get_storage().unwrap())
            },
            ("debug-index", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let store_config = config.get_storage_config();
                match opts.value_of("packhash") {
                    None    => {
                        let vs = store_config.list_indexes();
                        for &v in vs.iter() {
                            println!("{}", hex::encode(&v));
                        }
                    },
                    Some(s) => {
                        let mut packref = [0u8;32];
                        packref.clone_from_slice(&hex::decode(&s).unwrap()[..]);
                        let (_, refs) = pack::dump_index(&store_config, &packref).unwrap();
                        for r in refs.iter() {
                            println!("{}", hex::encode(r));
                        }
                    }
                }
            },
            ("debug-pack", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let packrefhex = opts.value_of("packhash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                pack_debug(&config, &packref_fromhex(&packrefhex));
            },
            ("unpack", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let packrefhex = opts.value_of("packhash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                block_unpack(&config, &packref_fromhex(&packrefhex), opts.is_present("preserve-pack"));
            },
            ("re-index", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let packrefhex = opts.value_of("packhash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                pack_reindex(&config, &packref_fromhex(&packrefhex))
            },
            ("is-pack-epoch", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let packrefhex = opts.value_of("packhash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                let previoushashhex = opts.value_of("previoushash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                //let epoch_id = values_t!(opts.value_of("epoch-id"), blockchain::EpochId).unwrap_or_else(|_| 0);
                let previoushash = blockchain::HeaderHash::from_slice(&hex::decode(&previoushashhex).unwrap()[..]).unwrap();
                let (result, lasthash) = pack_is_epoch(&config,
                                                       &packref_fromhex(&packrefhex),
                                                       &previoushash);
                match result {
                    true => {
                        println!("Pack is valid");
                        println!("last hash {}", lasthash);
                    },
                    false => {
                        println!("Pack is invalid");
                        println!("last hash {}", lasthash);
                    }
                }
            }
            ("pack", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let mut storage = config.get_storage().unwrap();
                let mut pack_params = PackParameters::default();
                pack_params.delete_blobs_after_pack = ! opts.is_present("preserve-blobs");
                if opts.is_present("range") {
                    let range = value_t!(opts.value_of("range"), internal::RangeOption).unwrap();
                    let from = match tag::read(&storage, &range.from) {
                        None => hex::decode(&range.from).unwrap(),
                        Some(t) => t
                    };
                    let to = if let Some(to_str) = range.to {
                        match tag::read(&storage, &to_str) {
                            None => hex::decode(&to_str).unwrap(),
                            Some(t) => t
                        }
                    } else {
                        panic!("We do not support packing without a terminal block");
                    };
                    let mut from_bytes = [0;32]; from_bytes[0..32].clone_from_slice(from.as_slice());
                    let mut to_bytes = [0;32];   to_bytes[0..32].clone_from_slice(to.as_slice());
                    pack_params.range = Some((from_bytes, to_bytes));
                }
                let packhash = pack_blobs(&mut storage, &pack_params);
                println!("pack created: {}", hex::encode(&packhash));
            },
            ("integrity-check", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let storage = config.get_storage().unwrap();
                let netcfg_file = config.get_storage_config().get_config_file();
                let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
                storage::integrity_check(&storage, net_cfg.genesis_prev, 20);
                println!("integrity check succeed");
            },
            ("epoch-refpack", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let storage = config.get_storage().unwrap();
                let epoch = value_t!(opts.value_of("epoch"), String).unwrap();
                storage::refpack_epoch_pack(&storage, &epoch).unwrap();
                println!("refpack successfuly created");
            },
            ("tag", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let mut storage = config.get_storage().unwrap();

                let tag = value_t!(opts.value_of("tag-name"), String).unwrap();

                match opts.value_of("tag-value") {
                    None => {
                        let value = hex::encode(&tag::read(&storage, &tag).unwrap());
                        println!("{}", value);
                    },
                    Some(value) => {
                        tag::write(&storage, &tag, &hex::decode(value).unwrap());
                    }
                }
            },
            ("cat", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let storage = config.get_storage().unwrap();
                let hh_hex = value_t!(opts.value_of("blockid"), String).unwrap();
                let hh_bytes = match tag::read(&storage, &hh_hex) {
                    None => hex::decode(&hh_hex).unwrap(),
                    Some(t) => t
                };
                let hh = blockchain::HeaderHash::from_slice(&hh_bytes).expect("blockid invalid");

                match block_location(&storage, hh.bytes()) {
                    None => {
                        println!("Error: block `{}' does not exist", hh);
                        ::std::process::exit(1);
                    },
                    Some(loc) => {
                        match block_read_location(&storage, &loc, hh.bytes()) {
                            None        => println!("error while reading"),
                            Some(bytes) => {
                                if opts.is_present("noparse") {
                                    stdout().write(&bytes).unwrap();
                                    stdout().flush().unwrap();
                                } else {
                                    let blk : blockchain::Block = cbor::decode_from_cbor(&bytes).unwrap();
                                    let hdr = blk.get_header();
                                    let hash = hdr.compute_hash();
                                    println!("blk location: {:?}", loc);
                                    println!("hash computed: {} expected: {}", hash, hh);
                                    display_block(&blk)
                                }
                            }
                        }
                    }
                }


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

fn block_unpack(config: &Config, packref: &PackHash, _preserve_pack: bool) {
    let storage_config = config.get_storage_config();
    let storage = config.get_storage().unwrap();

    let mut reader = storage::pack::PackReader::init(&storage_config, packref);
    loop {
        match reader.get_next() {
            None => { break; },
            Some(blk_raw) => {
                let blk : blockchain::Block = cbor::decode_from_cbor(&blk_raw[..]).unwrap();
                let hdr = blk.get_header();
                let hash = hdr.compute_hash();
                println!("unpacking {}", hash);
                let mut hash_repack = [0u8;32];
                hash_repack.clone_from_slice(hash.as_ref());
                storage::blob::write(&storage, &hash_repack, &blk_raw[..]).unwrap()
            }
        }
    }
}

fn pack_reindex(config: &Config, packref: &PackHash) {
    let storage_config = config.get_storage_config();
    let storage = config.get_storage().unwrap();
    let mut reader = storage::pack::PackReader::init(&storage_config, packref);
    let mut index = storage::pack::Index::new();
    loop {
        let ofs = reader.pos;
        println!("offset {}", ofs);
        match reader.get_next() {
            None    => { break; },
            Some(b) => {
                let blk : blockchain::Block = cbor::decode_from_cbor(&b[..]).unwrap();
                let hdr = blk.get_header();
                let hash = hdr.compute_hash();
                let mut packref = [0u8;32];
                packref.clone_from_slice(hash.as_ref());
                println!("packing hash {} slotid {}", hash, hdr.get_slotid());
                index.append(&packref, ofs);
            },
        }
    }

    let (_, tmpfile) = storage::pack::create_index(&storage, &index);
    tmpfile.render_permanent(&storage.config.get_index_filepath(&packref)).unwrap();
}

fn pack_debug(config: &Config,
              packref: &PackHash) {
    let storage_config = config.get_storage_config();
    let mut reader = storage::pack::PackReader::init(&storage_config, packref);
    while let Some(blk_raw) = reader.get_next() {
        let blk : blockchain::Block = cbor::decode_from_cbor(&blk_raw[..]).unwrap();
        let hdr = blk.get_header();
        let hash = hdr.compute_hash();
        let prev_hdr = hdr.get_previous_header();
        println!("slotid={} hash={} prev={}", hdr.get_slotid(), hash, prev_hdr);
    }
}

fn pack_is_epoch(config: &Config,
                 packref: &PackHash,
                 start_previous_header: &blockchain::HeaderHash)
             -> (bool, blockchain::HeaderHash) {
    let storage_config = config.get_storage_config();
    let mut reader = storage::pack::PackReader::init(&storage_config, packref);
    let mut known_prev_header = start_previous_header.clone();
    loop {
        match reader.get_next() {
            None      => { return (true, known_prev_header.clone()); },
            Some(blk_raw) => {
                let blk : blockchain::Block = cbor::decode_from_cbor(&blk_raw[..]).unwrap();
                let hdr = blk.get_header();
                let hash = hdr.compute_hash();
                let prev_hdr = hdr.get_previous_header();
                debug!("slotid={} hash={} prev={}", hdr.get_slotid(), hash, prev_hdr);
                if &prev_hdr != &known_prev_header {
                    return (false, hash)
                } else {
                    known_prev_header = hash.clone();
                }
            }
        }
    }
}

fn packref_fromhex(s: &String) -> PackHash {
    let mut packref = [0u8;32];
    packref.clone_from_slice(&hex::decode(&s).unwrap()[..]);
    packref
}

fn display_block(blk: &blockchain::Block) {
    println!("{}", pretty::format(blk, 4));
}

mod internal {
    use std::str::FromStr;

    #[derive(Debug)]
    pub struct RangeOption {
        pub from: String,
        pub to: Option<String>
    }

    #[derive(Debug)]
    pub enum Error {
        Empty,
        InvalidRange
    }

    impl FromStr for RangeOption {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.is_empty() { return Err(Error::Empty); }

            let mut v : Vec<&str> = s.split("..").collect();
            if v.is_empty() || v.len() > 2 { return Err(Error::InvalidRange); }

            let h1 = v.pop().unwrap().to_string(); // we expect at least one
            if let Some(h2) = v.pop().map(|v| v.to_string()) {
                Ok(RangeOption { from: h2, to: Some(h1)})
            } else {
                Ok(RangeOption { from: h1, to: None})
            }
        }
    }
}
