use wallet_crypto::{cbor, util::{hex}};
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use storage;
use storage::{blob, tag, Storage};
use storage::types::{PackHash};
use storage::{pack_blobs, block_location, block_read_location, pack, PackParameters};
//use storage::tag::{HEAD};
use blockchain;
use config::{Config};
use std::io::{Write, stdout};

use exe_common::{config::{net}, network::{Peer, api::{*}}};

use command::pretty::Pretty;

pub fn get_native_peer(blockchain: String, cfg: &net::Config) -> Peer {
    for peer in cfg.peers.iter() {
        if peer.is_native() {
            return Peer::new(blockchain, peer.name().to_owned(), peer.peer().clone(), cfg.protocol_magic).unwrap()
        }
    }

    panic!("no native peer to connect to")
}

pub fn get_http_peer(blockchain: String, cfg: &net::Config) -> Peer {
    for peer in cfg.peers.iter() {
        if peer.is_http() {
            return Peer::new(blockchain, peer.name().to_owned(), peer.peer().clone(), cfg.protocol_magic).unwrap()
        }
    }

    panic!("no native peer to connect to")
}

// Return the chain of block headers starting at from's next block
// and terminating at to, unless this range represent a number
// of blocks greater than the limit imposed by the node we're talking to.
fn find_earliest_epoch(storage: &storage::Storage, minimum_epochid: blockchain::EpochId, start_epochid: blockchain::EpochId)
        -> Option<(blockchain::EpochId, PackHash)> {
    let mut epoch_id = start_epochid;
    loop {
        match tag::read_hash(storage, &tag::get_epoch_tag(epoch_id)) {
            None => {
                match storage::epoch::epoch_read_pack(&storage.config, epoch_id).ok() {
                    None => {}
                    Some(h) => {
                        return Some((epoch_id, h));
                    }
                }
            },
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

fn net_sync_fast(network: String, mut storage: Storage) {
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
    let (latest_known_epoch_id, mstart_hash, prev_hash) = match find_earliest_epoch(&storage, net_cfg.epoch_start, network_slotid.get_epochid()) {
        None => { (net_cfg.epoch_start, Some(net_cfg.genesis.clone()), net_cfg.genesis_prev.clone()) },
        Some((found_epoch_id, packhash)) => { (found_epoch_id + 1, None, get_last_blockid(&storage.config, &packhash).unwrap()) }
    };
    println!("latest known epoch {} hash={:?}", latest_known_epoch_id, mstart_hash);

    let mut download_epoch_id = latest_known_epoch_id;
    let mut download_prev_hash = prev_hash.clone();
    let mut download_start_hash = mstart_hash.or(Some(prev_hash)).unwrap();

    while download_epoch_id < network_slotid.get_epochid() {
        println!("downloading epoch {} {}", download_epoch_id, download_start_hash);
        let fep = FetchEpochParams {
            epoch_id: download_epoch_id,
            start_header_hash: download_start_hash,
            previous_header_hash: download_prev_hash,
            upper_bound_hash: network_tip.clone()
        };
        let result = net.fetch_epoch(&net_cfg, &mut storage, fep).unwrap();
        download_prev_hash = result.last_header_hash.clone();
        download_start_hash = result.next_epoch_hash.unwrap_or(result.last_header_hash);
        download_epoch_id += 1;
    }
}

fn net_sync_faster(network: String, mut storage: Storage) {
    let netcfg_file = storage.config.get_config_file();
    let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
    let mut net = get_http_peer(network, &net_cfg);

    //let mut our_tip = tag::read_hash(&storage, &"TIP".to_string()).unwrap_or(genesis.clone());

    println!("Configured genesis   : {}", net_cfg.genesis);
    println!("Configured genesis-1 : {}", net_cfg.genesis_prev);

    // find the earliest epoch we know about starting from network_slotid
    let (latest_known_epoch_id, mstart_hash, prev_hash) = match find_earliest_epoch(&storage, net_cfg.epoch_start, 100) { // TODO
        None => { (net_cfg.epoch_start, Some(net_cfg.genesis.clone()), net_cfg.genesis_prev.clone()) },
        Some((found_epoch_id, packhash)) => { (found_epoch_id + 1, None, get_last_blockid(&storage.config, &packhash).unwrap()) }
    };
    println!("latest known epoch {} hash={:?}", latest_known_epoch_id, mstart_hash);

    let mut download_epoch_id = latest_known_epoch_id;
    let mut download_prev_hash = prev_hash.clone();
    let mut download_start_hash = mstart_hash.or(Some(prev_hash)).unwrap();

    while download_epoch_id < 40 {
        println!("downloading epoch {} {}", download_epoch_id, download_start_hash);
        let fep = FetchEpochParams {
            epoch_id: download_epoch_id,
            start_header_hash: download_start_hash,
            previous_header_hash: download_prev_hash,
            upper_bound_hash: net_cfg.genesis_prev.clone()
        };
        let result = net.fetch_epoch(&net_cfg, &mut storage, fep).unwrap();
        download_prev_hash = result.last_header_hash.clone();
        download_start_hash = result.next_epoch_hash.unwrap_or(result.last_header_hash);
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

pub struct Blockchain;

// TODO: rename Network to Blockchain?
impl HasCommand for Blockchain {
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
            .subcommand(SubCommand::with_name("epoch-from-pack")
                .about("enshrine a pack as a special epoch pack")
                .arg(blockchain_name_arg(1))
                .arg(Arg::with_name("epoch").help("The epoch to generate the refpack").index(2).required(true))
                .arg(Arg::with_name("packhash").help("pack to query").index(3).required(true))
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
                let mut net = get_native_peer(config.network, &net_cfg);
                let mbh = net.get_tip().unwrap();
                println!("prv block header: {}", mbh.get_previous_header());
            },
            ("get-block", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let hh_hex = value_t!(opts.value_of("blockid"), String).unwrap();
                let hh_bytes = hex::decode(&hh_hex).unwrap();
                let hh = blockchain::HeaderHash::from_slice(&hh_bytes).expect("blockid invalid");
                let netcfg_file = config.get_storage_config().get_config_file();
                let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
                let mut net = get_native_peer(config.network.clone(), &net_cfg);
                let b = net.get_block(hh.clone()).unwrap();
                let storage = config.get_storage().unwrap();
                blob::write(&storage, hh.bytes(), &cbor::encode_to_cbor(&b).unwrap()).unwrap();
            },
            ("sync", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                net_sync_faster(config.network.clone(), config.get_storage().unwrap())
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
            ("epoch-from-pack", Some(opts)) => {
                let config = resolv_network_by_name(&opts);
                let storage = config.get_storage_config();
                let epoch = value_t!(opts.value_of("epoch"), u32).unwrap();
                let packrefhex = opts.value_of("packhash").and_then(|s| Some(s.to_string())).unwrap();
                storage::epoch::epoch_create(&storage, &packref_fromhex(&packrefhex), epoch);
                println!("epoch {} successfuly created", epoch);
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
                            Some(rblk) => {
                                if opts.is_present("noparse") {
                                    stdout().write(rblk.as_ref()).unwrap();
                                    stdout().flush().unwrap();
                                } else {
                                    let blk = rblk.decode().unwrap();
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
        let blk = blk_raw.decode().unwrap();
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
                let blk = blk_raw.decode().unwrap();
                let hdr = blk.get_header();
                let hash = hdr.compute_hash();
                println!("unpacking {}", hash);
                let mut hash_repack = [0u8;32];
                hash_repack.clone_from_slice(hash.as_ref());
                storage::blob::write(&storage, &hash_repack, blk_raw.as_ref()).unwrap()
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
                let blk = b.decode().unwrap();
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
        let blk = blk_raw.decode().unwrap();
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
                let blk : blockchain::Block = cbor::decode_from_cbor(blk_raw.as_ref()).unwrap();
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
    println!("{}", blk.to_pretty());
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

