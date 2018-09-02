use super::config::{encrypt_primary_key, Config, HDWalletModel};
use super::{Wallet};
use super::state::{log, state, lookup, ptr, iter::TransactionIterator, utxo::UTxO};
use super::error::{Error};

use std::{path::PathBuf, io::Write};
use cardano::{hdwallet::{self, DerivationScheme}, address::ExtendedAddr, wallet, bip::bip39, block::{BlockDate}};
use rand::random;

use utils::term::{Term, style::{Style}};

use blockchain::{self, Blockchain};
use serde;

pub fn list( mut term: Term
           , root_dir: PathBuf
           , detailed: bool
           )
{
    let wallets_dir = super::config::wallet_directory(&root_dir);
    for entry in ::std::fs::read_dir(wallets_dir).unwrap() {
        let entry = entry.unwrap();
        if ! entry.file_type().unwrap().is_dir() {
            term.warn(&format!("unexpected file in wallet directory: {:?}", entry.path())).unwrap();
            continue;
        }
        let name = entry.file_name().into_string().unwrap_or_else(|err| {
            panic!("invalid utf8... {:?}", err)
        });

        // load the wallet
        let wallet = Wallet::load(root_dir.clone(), name);

        let detail = if detailed {
            if let Some(blk_name) = &wallet.config.attached_blockchain {
                // 1. get the wallet's blockchain
                let blockchain = load_attached_blockchain(&mut term, root_dir.clone(), wallet.config.attached_blockchain.clone());


                // 2. prepare the wallet state
                let initial_ptr = ptr::StatePtr::new_before_genesis(blockchain.config.genesis.clone());
                let mut state = state::State::new(initial_ptr, lookup::accum::Accum::default());

                update_wallet_state_with_logs(&wallet, &mut state);

                let total = state.total().unwrap();

                format!("\t{}\t{}@{}",
                    style!(total).green().bold(),
                    style!(blk_name.as_str()).underlined().white(),
                    style!(state.ptr.latest_block_date())
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        writeln!(term, "{}{}", style!(wallet.name).cyan().italic(), detail).unwrap();
    }
}

/// function to create a new wallet
///
pub fn new<D>( mut term: Term
             , root_dir: PathBuf
             , name: String
             , wallet_scheme: HDWalletModel
             , derivation_scheme: DerivationScheme
             , mnemonic_size: bip39::Type
             , languages: Vec<D>
             )
    where D: bip39::dictionary::Language
{
    let config = Config {
        attached_blockchain: None,
        derivation_scheme: derivation_scheme,
        hdwallet_model: wallet_scheme
    };

    // 1. generate the mnemonics

    let entropy = bip39::Entropy::generate(mnemonic_size, random);
    // 2. perform the seed generation from the entropy

    term.info("You can add a recovery wallet password. You can set no password, however you won't benefit from plausible deniability\n").unwrap();
    let recovery_password = term.new_password("recovery password", "confirm password", "password mismatch ").unwrap();
    let mut seed = [0;hdwallet::XPRV_SIZE];
    wallet::keygen::generate_seed(&entropy, recovery_password.as_bytes(), &mut seed);

    term.info("Please, note carefully the following mnemonic words. They will be needed to recover your wallet.\n").unwrap();
    for lang in languages {
        term.warn(&format!("{}: ", lang.name())).unwrap();
        let mnemonic_phrase = entropy.to_mnemonics().to_string(&lang);
        term.simply(&format!("{}\n", mnemonic_phrase)).unwrap();
    }

    // 3. normalize the seed to make it a valid private key

    let xprv = hdwallet::XPrv::normalize_bytes(seed);

    // create the root public key
    let public_key = match wallet_scheme {
        HDWalletModel::BIP44 => None,
        HDWalletModel::RandomIndex2Levels => Some(xprv.public()),
    };

    // 4. encrypt the private key
    term.info("Set a wallet password. This is for local usage only, allows you to protect your cached private key and prevent from creating non desired transactions.\n").unwrap();
    let password = term.new_password("spending password", "confirm spending password", "password mismatch").unwrap();
    let encrypted_xprv = encrypt_primary_key(password.as_bytes(), &xprv);

    // 5. create the wallet
    let wallet = Wallet::new(root_dir, name, config, encrypted_xprv, public_key);

    // 6. save the wallet
    wallet.save();

    term.success(&format!("wallet `{}' successfully created.\n", &wallet.name)).unwrap();
}

pub fn recover<D>( mut term: Term
                 , root_dir: PathBuf
                 , name: String
                 , wallet_scheme: HDWalletModel
                 , derivation_scheme: DerivationScheme
                 , mnemonic_size: bip39::Type
                 , interactive: bool
                 , daedalus_seed: bool
                 , language: D
                 )
    where D: bip39::dictionary::Language
{
    let config = Config {
        attached_blockchain: None,
        derivation_scheme: derivation_scheme,
        hdwallet_model: wallet_scheme
    };

    // 1. generate the mnemonics
    term.info("enter your mnemonics\n").unwrap();

    let (string, _, entropy) = if interactive {
        super::utils::prompt::mnemonics::interactive_input_words(&mut term, &language, mnemonic_size)
    } else {
        super::utils::prompt::mnemonics::input_mnemonic_phrase(&mut term, &language, mnemonic_size)

    };

    // 3. perform the seed generation from the entropy
    let xprv = if daedalus_seed {
        match wallet::rindex::RootKey::from_daedalus_mnemonics(derivation_scheme, &language, string.to_string()) {
            Err(err) => {
                term.error(&format!("Invalid mnemonics: {:#?}\n", err)).unwrap();
                ::std::process::exit(1)
            },
            Ok(root_key) => { (*root_key).clone() }
        }
    } else {
        term.info("Enter the wallet recovery password (if the password is wrong, you won't know).\n").unwrap();
        let recovery_password = term.password("recovery password: ").unwrap();

        let mut seed = [0;hdwallet::XPRV_SIZE];
        wallet::keygen::generate_seed(&entropy, recovery_password.as_bytes(), &mut seed);

        // normalize the seed to make it a valid private key
        hdwallet::XPrv::normalize_bytes(seed)
    };

    // create the root public key
    let public_key = match wallet_scheme {
        HDWalletModel::BIP44 => None,
        HDWalletModel::RandomIndex2Levels => Some(xprv.public()),
    };

    // 4. encrypt the private key
    term.info("Set a wallet password. This is for local usage only, allows you to protect your cached private key and prevent from creating non desired transactions.\n").unwrap();
    let password = term.new_password("spending password", "confirm spending password", "password mismatch").unwrap();
    let encrypted_xprv = encrypt_primary_key(password.as_bytes(), &xprv);

    // 5. create the wallet
    let wallet = Wallet::new(root_dir, name, config, encrypted_xprv, public_key);

    // 6. save the wallet
    wallet.save();

    term.success(&format!("wallet `{}' successfully recovered.\n", &wallet.name)).unwrap();
}

pub fn destroy( mut term: Term
              , root_dir: PathBuf
              , name: String
              )
{
    // load the wallet
    let wallet = Wallet::load(root_dir.clone(), name);

    writeln!(term, "You are about to destroy your wallet {}.
This means that all the data associated to this wallet will be deleted on this device.
The only way you will be able to reuse the wallet, recover the funds and create
new transactions will be by recovering the wallet with the mnemonic words.",
        ::console::style(&wallet.name).bold().red(),
    ).unwrap();

    let confirmation = ::dialoguer::Confirmation::new("Are you sure?")
        .use_line_input(true)
        .clear(false)
        .default(false)
        .interact().unwrap();
    if ! confirmation { ::std::process::exit(0); }

    unsafe { wallet.destroy() }.unwrap();

    term.success("Wallet successfully destroyed.\n").unwrap()
}


pub fn attach( mut term: Term
             , root_dir: PathBuf
             , name: String
             , blockchain_name: String
             )
{
    // load the wallet
    let mut wallet = Wallet::load(root_dir.clone(), name);

    // 1. is the wallet already attached
    if let Some(ref bn) = wallet.config.attached_blockchain {
        term.error(&format!("Wallet already attached to blockchain `{}'\n", bn)).unwrap();
        ::std::process::exit(1);
    }

    // 2. check the blockchain exists
    let blockchain_dir = blockchain::config::directory(root_dir.clone(), &blockchain_name);
    if let Err(err) = ::std::fs::read_dir(blockchain_dir) {
        term.error(&format!("Blockchain `{}' does not exists or you do not have user permissions\n", blockchain_name)).unwrap();
        term.error(&format!("   |-> {}\n", err)).unwrap();
        ::std::process::exit(2);
    }
    let _ = Blockchain::load(root_dir, blockchain_name.clone());

    // 3. save the attached wallet
    wallet.config.attached_blockchain = Some(blockchain_name);
    wallet.save();

    term.success("Wallet successfully attached to blockchain.\n").unwrap()
}

pub fn detach( mut term: Term
             , root_dir: PathBuf
             , name: String
             )
{
    // load the wallet
    let mut wallet = Wallet::load(root_dir.clone(), name);

    // 1. get the wallet's blockchain
    let _ = load_attached_blockchain(
        &mut term,
        root_dir,
        ::std::mem::replace(&mut wallet.config.attached_blockchain, None)
    );

    // 2. delete the wallet log
    wallet.delete_log().unwrap();

    wallet.save();

    term.success("Wallet successfully attached to blockchain.\n").unwrap()
}

pub fn status( mut term: Term
             , root_dir: PathBuf
             , name: String
             )
{
    // load the wallet
    let wallet = Wallet::load(root_dir.clone(), name);

    if let Some(ref blk_name) = &wallet.config.attached_blockchain {
        term.simply("Wallet ").unwrap();
        term.warn(&wallet.name).unwrap();
        term.simply(" on blockchain ").unwrap();
        term.info(blk_name).unwrap();
        term.simply("\n").unwrap();
    } else {
        term.info(&format!("Wallet {} status\n", &wallet.name)).unwrap();
        term.warn("wallet not attached to a blockchain").unwrap();
        return;
    }

    term.simply(" * wallet model ").unwrap();
    term.warn(&format!("{:?}", &wallet.config.hdwallet_model)).unwrap();
    term.simply("\n").unwrap();
    term.simply(" * derivation scheme ").unwrap();
    term.warn(&format!("{:?}", &wallet.config.derivation_scheme)).unwrap();
    term.simply("\n").unwrap();

    // 1. get the wallet's blockchain
    let blockchain = load_attached_blockchain(&mut term, root_dir, wallet.config.attached_blockchain.clone());


    // 2. prepare the wallet state
    let initial_ptr = ptr::StatePtr::new_before_genesis(blockchain.config.genesis.clone());
    let mut state = state::State::new(initial_ptr, lookup::accum::Accum::default());

    update_wallet_state_with_logs(&wallet, &mut state);

    let total = state.total().unwrap();

    term.simply(" * balance ").unwrap();
    term.success(&format!(" {}", total)).unwrap();
    term.simply("\n").unwrap();
    term.simply(" * synced to block ").unwrap();
    term.warn(&format!(" {} ({})", state.ptr.latest_known_hash, state.ptr.latest_addr.unwrap())).unwrap();
    term.simply("\n").unwrap();
}

pub fn log( mut term: Term
          , root_dir: PathBuf
          , name: String
          , pretty: bool
          )
{
    // load the wallet
    let wallet = Wallet::load(root_dir.clone(), name);

    // 1. get the wallet's blockchain
    let blockchain = load_attached_blockchain(&mut term, root_dir, wallet.config.attached_blockchain.clone());

    // 2. prepare the wallet state
    let initial_ptr = ptr::StatePtr::new_before_genesis(blockchain.config.genesis.clone());
    let mut state = state::State::new(initial_ptr, lookup::accum::Accum::default());

    display_wallet_state_logs(&mut term, &wallet, &mut state, pretty);
}

pub fn sync( mut term: Term
           , root_dir: PathBuf
           , name: String
           )

{
    // 0. load the wallet
    let wallet = Wallet::load(root_dir.clone(), name);

    // 1. get the wallet's blockchain
    let blockchain = load_attached_blockchain(&mut term, root_dir, wallet.config.attached_blockchain.clone());

    // 2. prepare the wallet state
    let initial_ptr = ptr::StatePtr::new_before_genesis(blockchain.config.genesis.clone());
    match wallet.config.hdwallet_model {
        HDWalletModel::BIP44 => {
            let mut state = {
                let mut lookup_struct = load_bip44_lookup_structure(&mut term, &wallet);
                lookup_struct.prepare_next_account().unwrap();
                state::State::new(initial_ptr, lookup_struct)
            };

            update_wallet_state_with_logs(&wallet, &mut state);

            update_wallet_state_with_utxos(&mut term, &wallet, &blockchain, &mut state);
        },
        HDWalletModel::RandomIndex2Levels => {
            let mut state = {
                let lookup_struct = load_randomindex_lookup_structure(&mut term, &wallet);
                state::State::new(initial_ptr, lookup_struct)
            };

            update_wallet_state_with_logs(&wallet, &mut state);

            update_wallet_state_with_utxos(&mut term, &wallet, &blockchain, &mut state);
        },
    };
}

fn update_wallet_state_with_utxos<LS>(term: &mut Term, wallet: &Wallet, blockchain: &Blockchain, state: &mut state::State<LS>)
    where LS: lookup::AddressLookup<AddressInput = ExtendedAddr>
        , for<'de> LS::AddressOutput : serde::Deserialize<'de> + serde::Serialize + Clone + ::std::fmt::Debug
{
    let blockchain_tip = blockchain.load_tip().0;

    let from_ptr = state.ptr().clone();
    let from = from_ptr.latest_known_hash;
    let from_date = from_ptr.latest_addr.unwrap_or(BlockDate::Genesis(0));
    let num_blocks = blockchain_tip.date - from_date;

    term.info(&format!("syncing wallet from {} to {}\n", from_date, blockchain_tip.date)).unwrap();

    let progress = term.progress_bar(num_blocks as u64);
    progress.set_message("loading transactions... ");

    let mut last_block_date = from_date;
    for res in TransactionIterator::new(progress, blockchain.iter_to_tip(from).unwrap() /* BAD */) {
        let (ptr, txaux) = res.unwrap(); // BAD

        if let Some(addr) = ptr.latest_addr {
            if last_block_date.get_epochid() != addr.get_epochid() {

                let log_lock = lock_wallet_log(&wallet);
                let mut writer = log::LogWriter::open(log_lock).unwrap();
                let log : log::Log<ExtendedAddr> = log::Log::Checkpoint(ptr.clone());
                writer.append(&log).unwrap();
            }

            last_block_date = addr.clone();
        }

        {
            let logs = state.forward_with_txins(txaux.tx.inputs.iter()).unwrap();
            let log_lock = lock_wallet_log(&wallet);
            let mut writer = log::LogWriter::open(log_lock).unwrap();
            for log in logs { writer.append(&log).unwrap(); }
        }

        {
            let txid = txaux.tx.id();
            let logs = state.forward_with_utxos(
                txaux.tx.outputs.into_iter().enumerate().map(|(idx, txout)| {
                    UTxO {
                        transaction_id: txid.clone(),
                        index_in_transaction: idx as u32,
                        blockchain_ptr: ptr.clone(),
                        credited_address: txout.address,
                        credited_value: txout.value
                    }
                })
            ).unwrap();

            let log_lock = lock_wallet_log(&wallet);
            let mut writer = log::LogWriter::open(log_lock).unwrap();
            for log in logs { writer.append(&log).unwrap(); }
        }
    }
}

fn display_wallet_state_logs<LS>( term: &mut Term
                                , wallet: &Wallet
                                , _state: &mut state::State<LS>
                                , pretty: bool
                                )
    where LS: lookup::AddressLookup
        , for<'de> LS::AddressOutput : serde::Deserialize<'de>
{
    let log_lock = lock_wallet_log(&wallet);
    let reader = log::LogReader::open(log_lock).unwrap();
    let reader : log::LogIterator<LS::AddressOutput> = reader.into_iter();
    let reader = reader.filter_map(|r| {
        match r {
            Err(err) => {
                panic!("{:?}", err)
            },
            Ok(v) => Some(v)
        }
    });

    for log in reader {
        match log {
            log::Log::Checkpoint(ptr) => {
                if ! pretty {
                    writeln!(term, "{} {} ({})",
                        style!("checkpoint").cyan(),
                        style!(ptr.latest_block_date()),
                        style!(ptr.latest_known_hash)
                    ).unwrap();
                    writeln!(term, "").unwrap();
                }
            },
            log::Log::ReceivedFund(utxo) => {
                if pretty {
                    display_utxo(term, utxo, false);
                } else {
                    dump_utxo(term, utxo, false);
                }
            },
            log::Log::SpentFund(utxo) => {
                if pretty {
                    display_utxo(term, utxo, true);
                } else {
                    dump_utxo(term, utxo, true);
                }
            }
        }
    }
}

fn display_utxo<L>(term: &mut Term, utxo: UTxO<L>, debit: bool) {
    let ptr = format!("{:9}", format!("{}", style!(utxo.blockchain_ptr.latest_block_date())));
    let tid = format!("{}", style!(utxo.transaction_id));
    let tii = format!("{:03}", utxo.index_in_transaction);
    const WIDTH : usize = 14;
    let credit = if debit {
        format!("{:>width$}", " ", width = WIDTH)
    } else {
        format!("{:>width$}", format!("{}", utxo.credited_value), width = WIDTH)
    };
    let debit = if debit {
        format!("{:>width$}", format!("{}", utxo.credited_value), width = WIDTH)
    } else {
        format!("{:>width$}", " ", width = WIDTH)
    };

    writeln!(term, "{}|{}.{}|{}|{}",
        ptr,
        tid,
        style!(tii).yellow(),
        style!(credit).green(),
        style!(debit).red()
    ).unwrap()
}

fn dump_utxo<L>(term: &mut Term, utxo: UTxO<L>, debit: bool) {
    let title = if debit {
        style!("debit").red()
    } else {
        style!("credit").green()
    };
    let amount = if debit {
        style!(format!("{}", utxo.credited_value)).red()
    } else {
        style!(format!("{}", utxo.credited_value)).green()
    };

    writeln!(term, "{} {}.{}",
        title,
        style!(utxo.transaction_id),
        style!(utxo.index_in_transaction).yellow(),
    ).unwrap();
    writeln!(term, "Date {}", style!(utxo.blockchain_ptr.latest_block_date())).unwrap();
    writeln!(term, "Block {}", style!(utxo.blockchain_ptr.latest_known_hash)).unwrap();
    writeln!(term, "Value {}", amount).unwrap();
    writeln!(term, "").unwrap()
}


fn update_wallet_state_with_logs<LS>(wallet: &Wallet, state: &mut state::State<LS>)
    where LS: lookup::AddressLookup
        , for<'de> LS::AddressOutput : serde::Deserialize<'de>
{
    let log_lock = lock_wallet_log(wallet);
    state.update_with_logs(
        log::LogReader::open(log_lock).unwrap() // BAD
            .into_iter().filter_map(|r| {
                match r {
                    Err(err) => {
                        panic!("{:?}", err)
                    },
                    Ok(v) => Some(v)
                }
            })
    ).unwrap(); // BAD
}

fn load_bip44_lookup_structure(term: &mut Term, wallet: &Wallet) -> lookup::sequentialindex::SequentialBip44Lookup {
    // TODO: to prevent from the need of the password, we can ask the user to create accounts ahead.
    //       if we store the wallet's account public keys in the config file we may not need for the
    //       password (and for the private key).
    term.info("Enter the wallet password.\n").unwrap();
    let password = term.password("wallet password: ").unwrap();

    let wallet = match wallet.get_wallet_bip44(password.as_bytes()) {
        Err(Error::CannotRetrievePrivateKeyInvalidPassword) => {
            term.error("Invalid wallet spending password").unwrap();
            ::std::process::exit(1);
        },
        Err(Error::CannotRetrievePrivateKey(err)) => {
            term.error(&format!("Cannot retrieve the private key of the wallet: {}", err)).unwrap();
            term.info("The encrypted wallet password is in an invalid format. You might need to delete this wallet and recover it.").unwrap();
            ::std::process::exit(1);
        },
        Err(err) => {
            term.error(IMPOSSIBLE_HAPPENED).unwrap();
            panic!("failing with an unexpected error {:#?}", err);
        },
        Ok(wallet) => { wallet }
    };
    lookup::sequentialindex::SequentialBip44Lookup::new(wallet)
}
fn load_randomindex_lookup_structure(term: &mut Term, wallet: &Wallet) -> lookup::randomindex::RandomIndexLookup {
    // in the case of the random index, we may not need the password if we have the public key
    term.info("Enter the wallet password.\n").unwrap();
    let password = term.password("wallet password: ").unwrap();

    let wallet = match wallet.get_wallet_rindex(password.as_bytes()) {
        Err(Error::CannotRetrievePrivateKeyInvalidPassword) => {
            term.error("Invalid wallet spending password").unwrap();
            ::std::process::exit(1);
        },
        Err(Error::CannotRetrievePrivateKey(err)) => {
            term.error(&format!("Cannot retrieve the private key of the wallet: {}", err)).unwrap();
            term.info("The encrypted wallet password is in an invalid format. You might need to delete this wallet and recover it.").unwrap();
            ::std::process::exit(1);
        },
        Err(err) => {
            term.error(IMPOSSIBLE_HAPPENED).unwrap();
            panic!("failing with an unexpected error {:#?}", err);
        },
        Ok(wallet) => { wallet }
    };
    lookup::randomindex::RandomIndexLookup::from(wallet)
}

fn lock_wallet_log(wallet: &Wallet) -> log::LogLock {
    match wallet.log() {
        Err(Error::WalletLogAlreadyLocked(pid)) => {
            error!("Wallet's LOG already locked by another process or thread ({})\n", pid);
            ::std::process::exit(1);
        },
        Err(err) => {
            error!("{}", IMPOSSIBLE_HAPPENED);
            panic!("`lock_wallet_log' has failed with an unexpected error {:#?}", err);
        },
        Ok(lock) => { lock }
    }
}

fn load_attached_blockchain(term: &mut Term, root_dir: PathBuf, name: Option<String>) -> Blockchain {
    match name {
        None => {
            term.error("Wallet is not attached to any blockchain\n").unwrap();
            ::std::process::exit(1);
        },
        Some(blockchain) => {
            Blockchain::load(root_dir, blockchain)
        }
    }
}

const IMPOSSIBLE_HAPPENED : &'static str = "The impossible happened
The process will panic with an error message, this is because something
unexpected happened. Please report the error message with the panic
error message to: https://github.com/input-output-hk/rust-cardano/issues
";
