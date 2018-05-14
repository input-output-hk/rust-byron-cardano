use wallet_crypto::{bip44, bip39, cbor, address};
use wallet_crypto::util::base58;
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use config::{Config};
use account::{Account};
use storage::{tag, pack};
use blockchain::{Block};

mod definition;
mod new;
mod util;

pub use self::definition::{Wallet};
use self::util::{*};

impl HasCommand for Wallet {
    type Output = Option<Config>;
    type Config = Config;

    const COMMAND : &'static str = "wallet";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("wallet management")
            .subcommand(new::CommandNewWallet::mk_command())
            .subcommand(SubCommand::with_name("recover")
                .about("recover a wallet from bip39 mnemonics")
                .arg(Arg::with_name("LANGUAGE")
                    .long("language")
                    .takes_value(true)
                    .value_name("LANGUAGE")
                    .possible_values(&["english"])
                    .help("use the given language for the mnemonic")
                    .required(false)
                    .default_value(r"english")
                )
                .arg(Arg::with_name("FROM PAPER WALLET")
                    .long("from-paper-wallet")
                    .takes_value(false)
                    .help("if this option is set, we will try to recover the wallet from the paper wallet instead.")
                    .required(false)
                )
                .arg(Arg::with_name("PASSWORD")
                    .long("--password")
                    .takes_value(true)
                    .value_name("PASSWORD")
                    .help("set the password from the CLI instead of prompting for it. It is quite unsafe as the password can be visible from your shell history.")
                    .required(false)
                )
            )
            .subcommand(SubCommand::with_name("address")
                .about("create an address with the given options")
                .arg(Arg::with_name("is_internal").long("internal").help("to generate an internal address (see BIP44)"))
                .arg(Arg::with_name("account").help("account to generate an address in").index(1).required(true))
                .arg(Arg::with_name("indices")
                    .help("list of indices for the addresses to create")
                    .multiple(true)
                )
            )
            .subcommand(SubCommand::with_name("find-addresses")
                .about("retrieve addresses in what have been synced from the network")
                .arg(Arg::with_name("addresses")
                    .help("list of addresses to retrieve")
                    .multiple(true)
                    .required(true)
                )
            )
    }
    fn run(mut cfg: Config, args: &ArgMatches) -> Self::Output {
        match args.subcommand() {
            (new::CommandNewWallet::COMMAND, Some(opts)) => {
                new::CommandNewWallet::run(cfg, opts)
            },
            ("recover", Some(opts)) => {
                // expect no existing wallet
                assert!(cfg.wallet.is_none());
                let language    = value_t!(opts.value_of("LANGUAGE"), String).unwrap(); // we have a default value
                let password    = value_t!(opts.value_of("PASSWORD"), String).ok();
                let from_paper_wallet = opts.is_present("FROM PAPER WALLET");
                let seed = if from_paper_wallet {
                    recover_paperwallet(language, password)
                } else {
                    recover_entropy(language, password)
                };
                cfg.wallet = Some(Wallet::generate(seed));
                let _storage = cfg.get_storage().unwrap();
                Some(cfg) // we need to update the config's wallet
            },
            ("address", Some(opts)) => {
                // expect existing wallet
                assert!(cfg.wallet.is_some());
                match &cfg.wallet {
                    &None => panic!("No wallet created, see `wallet generate` command"),
                    &Some(ref wallet) => {
                        let addr_type = if opts.is_present("is_internal") {
                            bip44::AddrType::Internal
                        } else {
                            bip44::AddrType::External
                        };
                        let account_name = opts.value_of("account")
                            .and_then(|s| Some(Account::new(s.to_string())))
                            .unwrap();
                        let account = match cfg.find_account(&account_name) {
                            None => panic!("no account {:?}", account_name),
                            Some(r) => r,
                        };
                        let indices = values_t!(opts.values_of("indices"), u32).unwrap_or_else(|_| vec![0]);

                        let addresses = wallet.0.gen_addresses(account, addr_type, indices).unwrap();
                        for addr in addresses {
                            println!("{}", base58::encode(&addr.to_bytes()));
                        };
                        None // we don't need to update the wallet
                    }
                }
            },
            ("find-addresses", Some(opts)) => {
                let storage = cfg.get_storage().unwrap();
                let addresses_bytes : Vec<_> = values_t!(opts.values_of("addresses"), String)
                    .unwrap().iter().map(|s| base58::decode(s).unwrap()).collect();
                let mut addresses : Vec<address::ExtendedAddr> = vec![];
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