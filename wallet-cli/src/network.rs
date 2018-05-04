use wallet_crypto::util::{hex};
use wallet_crypto::{cbor};
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use config::{Config};
use storage;
use storage::{blob, tag};
use storage::tag::{OLDEST_BLOCK, HEAD};
use rand;
use std::net::TcpStream;
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
    let mbh = GetBlockHeader::first().execute(&mut net.0).expect("to get one header at least");
    tag::write(&storage, &HEAD.to_string(), mbh.get_previous_header().as_ref());
    mbh
}

fn net_sync(config: Config) {
    let storage = config.get_storage().unwrap();

    let mut net = Network::new(&config);

    // read from the tags (try OLDEST_BLOCK, then HEAD) is they exist
    let read_tag = tag::read(&storage, &OLDEST_BLOCK.to_string()).or_else(|| { tag::read(&storage, &HEAD.to_string()) });
    let oldest_ref = match read_tag {
        None => {
            let mbh = network_get_head_header(&storage, &mut net);
            mbh.get_previous_header()
        },
        Some(oldest_ref) => {
            let hh = blockchain::HeaderHash::from_slice(&oldest_ref).expect("blockid invalid");
            hh
        },
    };

    println!("last known start block is {}", oldest_ref);

    let mut to_get = oldest_ref.clone();
    loop {
        let mut b = GetBlock::only(to_get.clone()).execute(&mut net.0)
            .expect("to get one block at least");
        blob::write(&storage, to_get.bytes(), &b[2..]);
        let blk : blockchain::Block = cbor::decode_from_cbor(&b[2..]).unwrap();
        match blk {
            blockchain::Block::GenesisBlock(blk) => {
                let tag_name = tag::get_epoch_tag(blk.header.consensus.epoch);
                println!("Genesis block {} epoch {} difficulty {}", to_get, blk.header.consensus.epoch, blk.header.consensus.chain_difficulty);
                tag::write(&storage, &OLDEST_BLOCK.to_string(), blk.header.previous_header.as_ref());
                tag::write(&storage, &tag_name, to_get.as_ref());
                to_get = blk.header.previous_header.clone()
            }
            blockchain::Block::MainBlock(blk) => {
                println!("block {} epoch {} slotid {}", to_get, blk.header.consensus.slot_id.epoch, blk.header.consensus.slot_id.slotid);
                tag::write(&storage, &OLDEST_BLOCK.to_string(), blk.header.previous_header.as_ref());
                to_get = blk.header.previous_header.clone()
            }
        }
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
                let mut b = GetBlock::only(hh.clone()).execute(&mut net.0)
                    .expect("to get one block at least");

                let storage = config.get_storage().unwrap();
                blob::write(&storage, hh.bytes(), &b[2..]);
            },
            ("sync", _) => {
                net_sync(config);
            },
            _ => {
                println!("{}", args.usage());
                ::std::process::exit(1);
            },
        }
    }
}

