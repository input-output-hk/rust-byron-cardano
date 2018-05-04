use wallet_crypto::util::{hex};
use wallet_crypto::{cbor};
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use config::{Config};
use storage::{pack_blobs, block_location, block_read_location, tag, pack, PackParameters};
use blockchain;
use ansi_term::Colour::*;

pub struct Block;

impl HasCommand for Block {
    type Output = ();

    fn clap_options<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name("block")
            .about("block/blobs operations")
            .subcommand(SubCommand::with_name("cat")
                .about("show content of a block")
                .arg(Arg::with_name("blockid").help("hexadecimal encoded block id").index(1).required(true))
            )
            .subcommand(SubCommand::with_name("debug-index")
                .about("internal debug command")
                .arg(Arg::with_name("packhash").help("pack to query").index(1))
            )
            .subcommand(SubCommand::with_name("pack")
                .about("internal pack command")
                .arg(Arg::with_name("preserve-blobs").long("keep").help("keep what is being packed in its original state"))
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
            ("pack", Some(opts)) => {
                let mut storage = config.get_storage().unwrap();
                let pack_params = PackParameters {
                    limit_nb_blobs: None,
                    limit_size: None,
                    delete_blobs_after_pack: ! opts.is_present("preserve-blobs"),
                };
                let packhash = pack_blobs(&mut storage, &pack_params);
                println!("pack created: {}", hex::encode(&packhash));
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
                                let blk : blockchain::Block = cbor::decode_from_cbor(&bytes).unwrap();
                                println!("blk location: {:?}", loc);
                                match blk {
                                    blockchain::Block::GenesisBlock(mblock) => {
                                        println!("genesis block display unimplemented");
                                        println!("{:?}", mblock)
                                    },
                                    blockchain::Block::MainBlock(mblock) => {
                                        let hdr = mblock.header;
                                        let body = mblock.body;
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
                                //println!("[header]");
                                //println!("");
                                //println!("{}: {}", Red.paint("hash"));
                                //println!("{}", blk);
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


