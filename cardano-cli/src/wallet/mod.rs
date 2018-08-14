mod config;
pub mod commands;
mod error;
mod result;
pub mod state;

pub use self::error::{Error};
pub use self::result::{Result};
pub use self::config::{HDWalletModel, Config};

use self::config::{decrypt_primary_key};

use std::{path::PathBuf, fs, io::{Read, Write}};
use cardano::{wallet};
use storage::{tmpfile::{TmpFile}};
use serde_yaml;

use utils::password_encrypted::{Password};

static WALLET_CONFIG_FILE : &'static str = "config.yml";
static WALLET_PRIMARY_KEY : &'static str = "wallet.key";

/// convenient Wallet object
///
/// simple object to provide small atomic actions that can be composed
/// in the commands.
pub struct Wallet {
    /// by design we do not want to decryp the wallet everytime
    ///
    /// We will need to ask the user for the password in order to retrieve
    /// the root private key.
    ///
    /// Then we will need to use the selected `HDWalletModel` to retrieve
    /// what kind of wallet we are dealing with.
    pub encrypted_key: Vec<u8>,

    pub root_dir: PathBuf,
    // conveniently keep the name given by the user to this wallet.
    pub name: String,

    pub config: Config
}
impl Wallet {

    /// create a new wallet, we expect the key to have been properly encrypted
    pub fn new(root_dir: PathBuf, name: String, config: Config, encrypted_key: Vec<u8>) -> Self {
        Wallet {
            encrypted_key: encrypted_key,
            root_dir: root_dir,
            name: name,
            config: config
        }
    }
    pub fn save(&self) {
        let dir = config::directory(self.root_dir.clone(), &self.name);
        fs::DirBuilder::new().recursive(true).create(dir.clone())
            .unwrap();

        // 1. save the configuration file
        let mut tmpfile = TmpFile::create(dir.clone())
            .unwrap();
        serde_yaml::to_writer(&mut tmpfile, &self.config)
            .unwrap();
        tmpfile.render_permanent(&dir.join(WALLET_CONFIG_FILE))
            .unwrap();

        // 2. save the encrypted key
        let mut tmpfile = TmpFile::create(dir.clone())
            .unwrap();
        tmpfile.write(&self.encrypted_key).unwrap();
        tmpfile.render_permanent(&dir.join(WALLET_PRIMARY_KEY))
            .unwrap();
    }

    pub fn load(root_dir: PathBuf, name: String) -> Self {
        let dir = config::directory(root_dir.clone(), &name);

        let mut file = fs::File::open(&dir.join(WALLET_CONFIG_FILE))
            .unwrap();
        let cfg = serde_yaml::from_reader(&mut file).unwrap();

        let mut file = fs::File::open(&dir.join(WALLET_PRIMARY_KEY))
            .unwrap();
        let mut key = Vec::with_capacity(150);
        file.read_to_end(&mut key).unwrap();

        Self::new(root_dir, name, cfg, key)
    }

    /// convenient function to reconstruct a BIP44 wallet from the encrypted key and password
    ///
    /// # Error
    ///
    /// This function may fail if:
    ///
    /// * the password in invalid;
    /// * the encrypted value did not represent a HDWallet XPrv
    ///
    pub fn get_wallet_bip44(&self, password: &Password) -> Result<wallet::bip44::Wallet> {
        let xprv = decrypt_primary_key(password, &self.encrypted_key)?;
        Ok(wallet::bip44::Wallet::from_root_key(
            xprv,
            self.config.derivation_scheme
        ))
    }

    /// convenient function to reconstruct a 2 level of random indices wallet from the encrypted key and password
    ///
    /// # Error
    ///
    /// This function may fail if:
    ///
    /// * the password in invalid;
    /// * the encrypted value did not represent a HDWallet XPrv
    ///
    pub fn get_wallet_rindex(&self, password: &Password) -> Result<wallet::rindex::Wallet> {
        let xprv = decrypt_primary_key(password, &self.encrypted_key)?;
        let root_key = wallet::rindex::RootKey::new(xprv, self.config.derivation_scheme);
        Ok(wallet::rindex::Wallet::from_root_key(
            self.config.derivation_scheme,
            root_key
        ))
    }
}
