use wallet_crypto::{cbor, address::{ExtendedAddr}};
use wallet_crypto::util::base58;
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use config::{Config};
use storage::{tag, pack};
use blockchain::{Block};

mod definition;
mod new;
mod recover;
mod address;
mod util;

pub use self::definition::{Wallet};

impl HasCommand for Wallet {
    type Output = Option<Config>;
    type Config = Config;

    const COMMAND : &'static str = "wallet";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("wallet management")
            .subcommand(new::CommandNewWallet::mk_command())
            .subcommand(recover::Recover::mk_command())
            .subcommand(address::Generate::mk_command())
            // TODO: move this command to the blockchain
            .subcommand(SubCommand::with_name("find-addresses")
                .about("retrieve addresses in what have been synced from the network")
                .arg(Arg::with_name("addresses")
                    .help("list of addresses to retrieve")
                    .multiple(true)
                    .required(true)
                )
            )
    }
    fn run(cfg: Config, args: &ArgMatches) -> Self::Output {
        match args.subcommand() {
            (new::CommandNewWallet::COMMAND, Some(opts)) => {
                new::CommandNewWallet::run(cfg, opts)
            },
            (recover::Recover::COMMAND, Some(opts)) => {
                recover::Recover::run(cfg, opts)
            },
            (address::Generate::COMMAND, Some(opts)) => {
                address::Generate::run(cfg, opts)
            },
            ("find-addresses", Some(opts)) => {
                let storage = cfg.get_storage().unwrap();
                let addresses_bytes : Vec<_> = values_t!(opts.values_of("addresses"), String)
                    .unwrap().iter().map(|s| base58::decode(s).unwrap()).collect();
                let mut addresses : Vec<ExtendedAddr> = vec![];
                for address in addresses_bytes {
                    addresses.push(cbor::decode_from_cbor(&address).unwrap());
                }
                let mut epoch_id = 0;
                while let Some(h) = tag::read_hash(&storage, &tag::get_epoch_tag(epoch_id)) {
                    info!("looking in epoch {}", epoch_id);
                    let mut reader = pack::PackReader::init(&storage.config, &h.into_bytes());
                    while let Some(blk_bytes) = reader.get_next() {
                        let blk : Block = cbor::decode_from_cbor(&blk_bytes).unwrap();
                        let hdr = blk.get_header();
                        let blk_hash = hdr.compute_hash();
                        debug!("  looking at slot {}", hdr.get_slotid().slotid);
                        match blk {
                            Block::GenesisBlock(_) => {
                                debug!("    ignoring genesis block")
                            },
                            Block::MainBlock(mblk) => {
                                for txaux in mblk.body.tx.iter() {
                                    for txout in &txaux.tx.outputs {
                                        if let Some(_) = addresses.iter().find(|a| *a == &txout.address) {
                                            println!("found address: {} in block {} at Epoch {} SlotId {}",
                                                base58::encode(&cbor::encode_to_cbor(&txout.address).unwrap()),
                                                blk_hash,
                                                hdr.get_slotid().epoch,
                                                hdr.get_slotid().slotid,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    epoch_id += 1;
                }
                None
            },
            _ => {
                println!("{}", args.usage());
                ::std::process::exit(1);
            },
        }
    }
}