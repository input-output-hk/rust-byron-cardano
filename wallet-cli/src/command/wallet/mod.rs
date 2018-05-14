use wallet_crypto::{wallet, bip44, bip39, paperwallet, cbor, address};
use wallet_crypto::util::base58;
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use config::{Config};
use account::{Account};
use storage::{tag, pack};
use blockchain::{Block};
use rand;

use termion::{style, color, clear, cursor};
use termion::input::TermRead;
use std::io::{Write, stdout, stdin};

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet(wallet::Wallet);
impl Wallet {
    fn generate(seed: bip39::Seed) -> Self {
        Wallet(wallet::Wallet::new_from_bip39(&seed))
    }
}

impl HasCommand for Wallet {
    type Output = Option<Config>;
    type Config = Config;

    const COMMAND : &'static str = "wallet";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("wallet management")
            .subcommand(SubCommand::with_name("generate")
                .about("generate a new wallet")
                .arg(Arg::with_name("LANGUAGE")
                    .long("language")
                    .takes_value(true)
                    .value_name("LANGUAGE")
                    .possible_values(&["english"])
                    .help("use the given language for the mnemonic")
                    .required(false)
                    .default_value(r"english")
                )
                .arg(Arg::with_name("NO PAPER WALLET")
                    .long("no-paper-wallet")
                    .takes_value(false)
                    .help("if this option is set, the interactive mode won't ask you about generating a paperwallet")
                    .required(false)
                )
                .arg(Arg::with_name("MNEMONIC SIZE")
                    .long("number-of-mnemonic-words")
                    .takes_value(true)
                    .value_name("MNEMONIC_SIZE")
                    .possible_values(&["12", "15", "18", "21", "24"])
                    .help("set the number of the mnemonic words")
                    .required(false)
                    .default_value(r"15")
                )
                .arg(Arg::with_name("PASSWORD")
                    .long("--password")
                    .takes_value(true)
                    .value_name("PASSWORD")
                    .help("set the password from the CLI instead of prompting for it. It is quite unsafe as the password can be visible from your shell history.")
                    .required(false)
                )
            )
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
    fn run(config: Config, args: &ArgMatches) -> Self::Output {
        let mut cfg = config;
        match args.subcommand() {
            ("generate", Some(opts)) => {
                // expect no existing wallet
                assert!(cfg.wallet.is_none());
                let language    = value_t!(opts.value_of("LANGUAGE"), String).unwrap(); // we have a default value
                let mnemonic_sz = value_t!(opts.value_of("MNEMONIC SIZE"), bip39::Type).unwrap();
                let password    = value_t!(opts.value_of("PASSWORD"), String).ok();
                let without_paper_wallet = opts.is_present("NO PAPER WALLET");
                let seed = generate_entropy(language, password, mnemonic_sz, without_paper_wallet);
                cfg.wallet = Some(Wallet::generate(seed));
                let _storage = cfg.get_storage().unwrap();
                Some(cfg) // we need to update the config's wallet
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

fn get_password() -> String {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(b"password: ").unwrap();
    stdout.flush().unwrap();

    let pwd = stdin.read_passwd(&mut stdout).unwrap().unwrap_or("".to_string());
    stdout.write_all(b"\n").unwrap();
    stdout.flush().unwrap();
    pwd
}

fn new_password() -> String {
    {
        let stdout = stdout();
        let mut stdout = stdout.lock();

        write!(  stdout, "{}", style::Italic).unwrap();
        writeln!(stdout, "Enter you wallet password. It will be needed to recover").unwrap();
        writeln!(stdout, "your wallet later with the mnemonic phrase.").unwrap();
        write!(stdout, "{}", style::NoItalic).unwrap();
        stdout.flush().unwrap();
    }
    let pwd1 = get_password();
    {
        let stdout = stdout();
        let mut stdout = stdout.lock();

        write!(  stdout, "{}", style::Italic).unwrap();
        writeln!(stdout, "Type your password again.").unwrap();
        write!(stdout, "{}", style::NoItalic).unwrap();
        stdout.flush().unwrap();
    }
    let pwd2 = get_password();

    if pwd1 != pwd2 {
        eprintln!("{}not the same password{}", color::Fg(color::Red), color::Fg(color::Reset));
        panic!("try again");
    }

    pwd1
}

fn get_mnemonic_word<D>(index: usize, dic: &D) -> Option<bip39::Mnemonic>
    where D: bip39::dictionary::Language
{
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    let mut mmne = None;

    for _ in 0..3 {
        write!(stdout, "mnemonic {}: ", index).unwrap();
        stdout.flush().unwrap();
        let midx = stdin.read_passwd(&mut stdout).unwrap();
        write!(stdout, "{}{}", clear::CurrentLine, cursor::Left(14)).unwrap();
        stdout.flush().unwrap();
        match midx.and_then(|s| if s == "" { None } else { Some(s)}) {
            None => {
                write!(stdout, "{}No mnemonic entered.{} Are you done? (No|yes): ", color::Fg(color::Red), color::Fg(color::Reset)).unwrap();
                stdout.flush().unwrap();
                let mchoice = stdin.read_line().unwrap();
                match mchoice {
                    None => {},
                    Some(choice) => {
                        if choice.to_uppercase() == "YES" { break; }
                    }
                };
            },
            Some(word) => {
                match bip39::Mnemonic::from_word(dic, word.as_str()) {
                    Ok(mne) => { mmne = Some(mne); break; },
                    Err(err) => {
                        writeln!(stdout, "{}Invalid mnemonic{}: {}", color::Fg(color::Red), color::Fg(color::Reset), err).unwrap();
                        stdout.flush().unwrap();
                    }
                }
            }
        }
    }

    mmne
}

fn display_mnemonic_phrase(mnemonic: &bip39::MnemonicString) {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    write!(  stdout, "{}", style::Italic).unwrap();
    writeln!(stdout, "Note the following words carrefully as you will need it to recover your wallet.").unwrap();
    writeln!(stdout, "Press `Enter' when you are sure you have saved them.").unwrap();
    writeln!(stdout, "{}", style::NoItalic).unwrap();
    write!(stdout, "mnemonic: {}{}{}", color::Fg(color::Green), mnemonic, color::Fg(color::Reset)).unwrap();
    stdout.flush().unwrap();
    let _ = stdin.read_passwd(&mut stdout).unwrap().unwrap();
    write!(stdout, "{}{}", clear::CurrentLine, cursor::Left(128)).unwrap();
    stdout.flush().unwrap();
}

fn get_mnemonic_words<D>(dic: &D) -> bip39::Mnemonics
    where D: bip39::dictionary::Language
{
    let mut vec = vec![];

    print!("{}", style::Italic);
    println!("Enter the mnemonic word one by one as prompted.");
    print!("{}", style::NoItalic);

    for index in 1..25 {
        match get_mnemonic_word(index, dic) {
            None => break,
            Some(idx) => vec.push(idx)
        }
    }

    match bip39::Mnemonics::from_mnemonics(vec) {
        Err(err) => { panic!("Invalid mnemonic phrase: {}", err); },
        Ok(mn) => mn
    }
}

fn recover_paperwallet(language: String, opt_pwd: Option<String>) -> bip39::Seed {
    assert!(language == "english");
    let dic = &bip39::dictionary::ENGLISH;

    println!("{}", style::Italic);
    println!("We are about to recover from a paperwallet. It is the mnemonic words");
    println!("and the password you might have set after generating a new wallet.");
    println!("{}", style::NoItalic);

    // 1. get the mnemonic words of the paperwallet
    let shielded_mnemonics = get_mnemonic_words(dic);

    // 2. get the password of the paperwallet
    let pwd = match opt_pwd {
        Some(pwd) => pwd,
        None => get_password()
    };

    // 3. retrieve the shielded entropy
    let shielded_entropy = bip39::Entropy::from_mnemonics(&shielded_mnemonics).unwrap();

    // 4. unscramble the shielded entropy
    let entropy_bytes = paperwallet::unscramble(pwd.as_bytes(), shielded_entropy.as_ref());

    // 5. reconstruct the entropy
    let entropy = bip39::Entropy::from_slice(&entropy_bytes).unwrap();

    // 6. retrieve the mnemonic string
    let mnemonics_str = entropy.to_mnemonics().to_string(dic);

    // 7. rebuild the seed
    bip39::Seed::from_mnemonic_string(&mnemonics_str, pwd.as_bytes())
}

fn generate_paper_wallet<D>(dic: &D, entropy: &bip39::Entropy)
    where D: bip39::dictionary::Language
{
    // 1. gen an IV
    let mut iv = [0u8; paperwallet::IV_SIZE];
    for byte in iv.iter_mut() { *byte = rand::random(); }
    println!("{}", style::Italic);
    println!("We are about to generate a paperwallet. It mainly is a longer mnemonic phrase");
    println!("protected with a password (or not, but un-advised) that you can print and store");
    println!("securely in order to recover your wallet and your funds.");
    println!("{}", style::NoItalic);
    // 2. get a password
    let pwd = new_password();
    // 3. generate the scrambled entropy
    let shielded_entropy_bytes = paperwallet::scramble(&iv[..], pwd.as_bytes(), entropy.as_ref());
    // 4. create an antropy from the given bytes
    let shielded_entropy = bip39::Entropy::from_slice(&shielded_entropy_bytes).unwrap();

    println!("shielded entropy: {}{}{}{}{}",
        color::Fg(color::Cyan),
        style::Bold,
        shielded_entropy.to_mnemonics().to_string(dic),
        style::NoBold,
        color::Fg(color::Reset),
    );
}

fn generate_entropy(language: String, opt_pwd: Option<String>, t: bip39::Type, no_paper_wallet: bool) -> bip39::Seed {
    assert!(language == "english");
    let dic = &bip39::dictionary::ENGLISH;

    let pwd = match opt_pwd {
        Some(pwd) => pwd,
        None => new_password()
    };

    let entropy = bip39::Entropy::generate(t, rand::random);

    let mnemonic = entropy.to_mnemonics().to_string(dic);
    display_mnemonic_phrase(&mnemonic);

    if ! no_paper_wallet {
        generate_paper_wallet(dic, &entropy);
    }

    bip39::Seed::from_mnemonic_string(&mnemonic, pwd.as_bytes())
}

fn recover_entropy(language: String, opt_pwd: Option<String>) -> bip39::Seed {
    assert!(language == "english");
    let dic = &bip39::dictionary::ENGLISH;

    let mnemonics = get_mnemonic_words(dic);

    let pwd = match opt_pwd {
        Some(pwd) => pwd,
        None => get_password()
    };

    let mnemonics_str = mnemonics.to_string(dic);

    bip39::Seed::from_mnemonic_string(&mnemonics_str, pwd.as_bytes())
}