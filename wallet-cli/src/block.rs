use wallet_crypto::util::{hex};
use wallet_crypto::{cbor};
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use config::{Config};
use storage::{pack_blobs, block_location, block_read_location, tag, pack, PackParameters};
use storage::types::PackHash;
use storage;
use blockchain;
use ansi_term::Colour::*;
use exe_common::{config::{net}};

use std::io::{Write, stdout};

pub struct Block;

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
    match blk {
        &blockchain::Block::GenesisBlock(ref mblock) => {
            println!("genesis block display unimplemented");
            println!("{:?}", mblock)
        },
        &blockchain::Block::MainBlock(ref mblock) => {
            let hdr = &mblock.header;
            let body = &mblock.body;
            println!("### Header");
            println!("{} : {}"  , Green.paint("protocol magic"), hdr.protocol_magic);
            println!("{} : {}"  , Green.paint("previous hash "), hex::encode(hdr.previous_header.as_ref()));
            println!("{}"       , Green.paint("body proof    "));
            println!("  - {}"   , Cyan.paint("tx proof    "));
            println!("       - {}: {}", Yellow.paint("number      "), hdr.body_proof.tx.number);
            println!("       - {}: {}", Yellow.paint("root        "), hdr.body_proof.tx.root);
            println!("       - {}: {}", Yellow.paint("witness hash"), hdr.body_proof.tx.witnesses_hash);
            println!("  - {} : {:?}", Cyan.paint("mpc         "), hdr.body_proof.mpc);
            println!("  - {} : {:?}", Cyan.paint("proxy sk    "), hdr.body_proof.proxy_sk);
            println!("  - {} : {:?}", Cyan.paint("update      "), hdr.body_proof.update);
            println!("{}"           , Green.paint("consensus     "));
            println!("  - {} : {:?}", Cyan.paint("slot id         "), hdr.consensus.slot_id);
            println!("  - {} : {}"  , Cyan.paint("leader key      "), hex::encode(hdr.consensus.leader_key.as_ref()));
            println!("  - {} : {}"  , Cyan.paint("chain difficulty"), hdr.consensus.chain_difficulty);
            println!("  - {} : {:?}", Cyan.paint("block signature "), hdr.consensus.block_signature);
            println!("{} : {:?}", Green.paint("extra-data    "), hdr.extra_data);
            println!("### Body");
            println!("{}", Green.paint("tx-payload"));
            for e in body.tx.iter() {
                println!("  {}", e);
            }
            println!("{} : {:?}", Green.paint("scc           "), body.scc);
            println!("{} : {:?}", Green.paint("delegation    "), body.delegation);
            println!("{} : {:?}", Green.paint("update        "), body.update);
            println!("### Extra");
            println!("{} : {:?}", Green.paint("extra         "), mblock.extra);
            //println!("{}: {}", Red.paint("protocol magic:"), mblock.protocol.magic);
        },
    }
}

impl HasCommand for Block {
    type Output = ();
    type Config = Config;

    const COMMAND : &'static str = "block";


    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("block/blobs operations")
            .subcommand(SubCommand::with_name("cat")
                .about("show content of a block")
                .arg(Arg::with_name("noparse").long("raw").help("cat the binary encoded block, no pretty print"))
                .arg(Arg::with_name("blockid").help("hexadecimal encoded block id").index(1).required(true))
            )
            .subcommand(SubCommand::with_name("debug-index")
                .about("internal debug command")
                .arg(Arg::with_name("packhash").help("pack to query").index(1))
            )
            .subcommand(SubCommand::with_name("re-index")
                .about("internal re-index command")
                .arg(Arg::with_name("packhash").help("pack to re-index").index(1).required(true))
            )
            .subcommand(SubCommand::with_name("pack")
                .about("internal pack command")
                .arg(Arg::with_name("preserve-blobs").long("keep").help("keep what is being packed in its original state"))
                .arg(Arg::with_name("range").help("<tag|ref>..<tag|ref>").index(1).required(false))
            )
            .subcommand(SubCommand::with_name("epoch-refpack")
                .about("generate the refpack of a given epoch")
                .arg(Arg::with_name("epoch").help("The epoch to generate the refpack").index(1).required(true))
            )
            .subcommand(SubCommand::with_name("unpack")
                .about("internal unpack command")
                .arg(Arg::with_name("preserve-packs").long("keep").help("keep what is being unpacked in its original state"))
                .arg(Arg::with_name("packhash").help("pack to query").index(1))
            )
            .subcommand(SubCommand::with_name("integrity-check")
                .about("check the integrity of the blockchain")
            )
            .subcommand(SubCommand::with_name("is-pack-epoch")
                .about("internal check to see if a pack is a valid epoch-pack")
                .arg(Arg::with_name("packhash").help("pack to query").index(1))
                .arg(Arg::with_name("previoushash").help("pack to query").index(2))
                .arg(Arg::with_name("epoch-id").help("pack to query").index(3))
            )
            .subcommand(SubCommand::with_name("tag")
                .about("show content of a tag or set a tag")
                .arg(Arg::with_name("tag-name").help("name of the tag").index(1).required(true))
                .arg(Arg::with_name("tag-value").help("value to set to the given tag").index(2).required(false))
            )
    }
    fn run(config: Config, args: &ArgMatches) -> Self::Output {
        match args.subcommand() {
            ("debug-index", opts) => {
                let store_config = config.get_storage_config();
                match opts {
                    None    => {
                        let vs = store_config.list_indexes();
                        for &v in vs.iter() {
                            println!("{}", hex::encode(&v));
                        }
                    },
                    Some(opts) => {
                        let packrefhex = opts.value_of("packhash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                        let mut packref = [0u8;32];
                        packref.clone_from_slice(&hex::decode(&packrefhex).unwrap()[..]);
                        let (_, refs) = pack::dump_index(&store_config, &packref).unwrap();
                        for r in refs.iter() {
                            println!("{}", hex::encode(r));
                        }
                    }
                }
            },
            ("unpack", Some(opts)) => {
                let packrefhex = opts.value_of("packhash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                block_unpack(&config, &packref_fromhex(&packrefhex), opts.is_present("preserve-pack"));
            },
            ("re-index", Some(opts)) => {
                let packrefhex = opts.value_of("packhash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                pack_reindex(&config, &packref_fromhex(&packrefhex))
            },
            ("is-pack-epoch", Some(opts)) => {
                let packrefhex = opts.value_of("packhash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                let previoushashhex = opts.value_of("previoushash")
                            .and_then(|s| Some(s.to_string()))
                            .unwrap();
                //let epoch_id = values_t!(opts.value_of("epoch-id"), blockchain::EpochId).unwrap_or_else(|_| 0);
                let epoch_id = 0;
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
                let mut storage = config.get_storage().unwrap();
                let mut pack_params = PackParameters::default();
                pack_params.delete_blobs_after_pack = ! opts.is_present("preserve-blobs");
                if opts.is_present("range") {
                    let range = value_t!(opts.value_of("range"), internal::RangeOption).unwrap();
                    let from = match tag::read(&storage, &range.from) {
                        None => hex::decode(&range.from).unwrap(),
                        Some(t) => t
                    };
                    let to = if let &Some(ref to_str) = &range.to {
                        match tag::read(&storage, to_str) {
                            None => hex::decode(to_str).unwrap(),
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
            ("integrity-check", _) => {
                let storage = config.get_storage().unwrap();
                let netcfg_file = config.get_storage_config().get_config_file();
                let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
                storage::integrity_check(&storage, net_cfg.genesis, 20);
                println!("integrity check succeed");
            },
            ("epoch-refpack", Some(opts)) => {
                let storage = config.get_storage().unwrap();
                let epoch = value_t!(opts.value_of("epoch"), String).unwrap();
                storage::refpack_epoch_pack(&storage, &epoch).unwrap();
                println!("refpack successfuly created");
            },
            ("tag", Some(opt)) => {
                let mut storage = config.get_storage().unwrap();

                let tag = value_t!(opt.value_of("tag-name"), String).unwrap();

                match opt.value_of("tag-value") {
                    None => {
                        let value = hex::encode(&tag::read(&storage, &tag).unwrap());
                        println!("{}", value);
                    },
                    Some(value) => {
                        tag::write(&storage, &tag, &hex::decode(value).unwrap());
                    }
                }
            },
            ("cat", Some(opt)) => {
                let storage = config.get_storage().unwrap();
                let hh_hex = value_t!(opt.value_of("blockid"), String).unwrap();
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
                                if opt.is_present("noparse") {
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