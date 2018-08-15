use super::config::{encrypt_primary_key, Config, HDWalletModel};
use super::{Wallet};

use std::{path::PathBuf};
use cardano::{hdwallet::{self, DerivationScheme}, wallet, bip::bip39};
use rand::random;

use utils::term::Term;

use blockchain::{self, Blockchain};

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
    let recovery_password              = term.password("recovery password: ").unwrap();
    let recovery_password_confirmation = term.password("confirm password: ").unwrap();
    if recovery_password != recovery_password_confirmation {
        term.error("Not the same password.").unwrap();
        ::std::process::exit(1);
    }

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

    // 4. encrypt the private key
    term.info("Set a wallet password. This is for local usage only, allows you to protect your cached private key and prevent from creating non desired transactions.\n").unwrap();
    let password              = term.password("spending password: ").unwrap();
    let password_confirmation = term.password("confirm password: ").unwrap();
    if password != password_confirmation {
        term.error("Not the same password.").unwrap();
        ::std::process::exit(1);
    }
    let encrypted_xprv = encrypt_primary_key(password.as_bytes(), &xprv);

    // create the root public key (if not unsafe wallet)
    let public_key = match wallet_scheme {
        HDWalletModel::BIP44 => Some(xprv.public()),
        HDWalletModel::RandomIndex2Levels => None
    };

    // 5. create the wallet
    let wallet = Wallet::new(root_dir, name, config, encrypted_xprv, public_key);

    // 6. save the wallet
    wallet.save();

    term.success(&format!("wallet `{}' successfully created.\n", &wallet.name)).unwrap();
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
        term.error(&format!("Blockchain `{}' does not exists or you do not have user permissions", blockchain_name)).unwrap();
        term.error(&format!("   |-> {}", err)).unwrap();
        ::std::process::exit(2);
    }
    let blockchain = Blockchain::load(root_dir, blockchain_name.clone());

    // 3. save the attached wallet
    wallet.config.attached_blockchain = Some(blockchain_name);
    wallet.save();

    // 4. set the wallet state tag to the genesis of the blockchain
    blockchain.set_wallet_tag(&wallet.name, &blockchain.config.genesis);

    term.success("Wallet successfully attached to blockchain.").unwrap()
}

pub fn detach( mut term: Term
             , root_dir: PathBuf
             , name: String
             )
{
    // load the wallet
    let mut wallet = Wallet::load(root_dir.clone(), name);

    // 1. get the wallet's blockchain
    let blockchain = match wallet.config.attached_blockchain {
        None => {
            term.error("Wallet is not attached to any blockchain\n").unwrap();
            ::std::process::exit(1);
        },
        Some(blockchain) => {
            Blockchain::load(root_dir, blockchain)
        }
    };

    // 2. remove the wallet tag
    blockchain.remove_wallet_tag(&wallet.name);

    // 3. remove the blockchain name from the wallet config
    wallet.config.attached_blockchain = None;

    // TODO: clear the wallet log too, we are not linked to any blockchain
    //       we cannot keep UTxO or other logs associated to a blockchain
    //       as it may not be compatible with the next attached blockchain

    wallet.save();

    term.success("Wallet successfully attached to blockchain.").unwrap()
}
