mod config;
pub mod commands;
mod error;
mod result;
pub mod state;
pub mod utils;

pub use self::error::{Error};
pub use self::result::{Result};
pub use self::config::{HDWalletModel, Config};

use self::config::{decrypt_primary_key};

use self::state::log::{LogLock, LogWriter};

use std::{fmt, path::PathBuf, fs, io::{Read, Write}, collections::{BTreeMap}};
use cardano::{wallet, hdwallet::{XPub, XPUB_SIZE}};
use storage::utils::{tmpfile::{TmpFile}};
use serde_yaml;

use utils::password_encrypted::{Password};

static WALLET_CONFIG_FILE : &'static str = "config.yml";
static WALLET_PRIMARY_KEY : &'static str = "wallet.key";
static WALLET_PUBLIC_KEY  : &'static str = "wallet.pub";

/// User friendly name associated with a Wallet.
///
/// A valid wallet name need to be unicode compliant, and
/// not contains dot, nor slash. TODO: validate the rules
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WalletName(String);

impl WalletName {
    pub fn new(v: String) -> Option<Self> {
        if v.find(|c: char| (c == '/') && (c == '.')).is_some() {
            None
        } else {
            Some(WalletName(v))
        }
    }
    pub fn as_dirname(&self) -> String {
        self.0.clone()
    }
}
impl ::std::str::FromStr for WalletName {
    type Err = &'static str;
    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        match Self::new(s.to_owned()) {
            Some(wn) => Ok(wn),
            None => Err("Invalid Wallet Name")
        }
    }
}
impl fmt::Display for WalletName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

    /// in some cases, we might want to store the public key in the wallet
    /// this is optional and we might be able to let the user decide if they
    /// are happy to keep the public key un-protected in the hard drive disk
    /// (needing to remind that it is not possible to spend funds with the public
    /// key, only with the private key. Leaking the public key will have _only_
    /// for consequence to lose privacy of the wallet).
    pub public_key: Option<XPub>,

    pub root_dir: PathBuf,
    // conveniently keep the name given by the user to this wallet.
    pub name: WalletName,

    pub config: Config
}
impl Wallet {

    /// create a new wallet, we expect the key to have been properly encrypted
    pub fn new(root_dir: PathBuf, name: WalletName, config: Config, encrypted_key: Vec<u8>, xpub: Option<XPub>) -> Self {
        Wallet {
            encrypted_key: encrypted_key,
            public_key: xpub,
            root_dir: root_dir,
            name: name,
            config: config
        }
    }

    pub unsafe fn destroy(self) -> ::std::io::Result<()> {
        let dir = config::directory(self.root_dir.clone(), &self.name.0);
        ::std::fs::remove_dir_all(dir)
    }

    pub fn save(&self) {
        let dir = config::directory(self.root_dir.clone(), &self.name.0);
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

        // 3. save the public key
        if let Some(ref xpub) = self.public_key {
            let mut tmpfile = TmpFile::create(dir.clone())
                .unwrap();
            tmpfile.write(xpub.as_ref()).unwrap();
            tmpfile.render_permanent(&dir.join(WALLET_PUBLIC_KEY))
                .unwrap();
        };
    }

    pub fn load(root_dir: PathBuf, name: WalletName) -> Self {
        let dir = config::directory(root_dir.clone(), &name.as_dirname());

        let mut file = fs::File::open(&dir.join(WALLET_CONFIG_FILE))
            .unwrap();
        let cfg = serde_yaml::from_reader(&mut file).unwrap();

        let mut file = fs::File::open(&dir.join(WALLET_PRIMARY_KEY))
            .unwrap();
        let mut key = Vec::with_capacity(150);
        file.read_to_end(&mut key).unwrap();

        let xpub = match fs::File::open(&dir.join(WALLET_PUBLIC_KEY)) {
            Err(_err) => None, // TODO, check for file does not exists
            Ok(mut file) => {
                let mut key = [0;XPUB_SIZE];
                file.read_exact(&mut key).unwrap();
                Some(XPub::from_bytes(key))
            }
        };

        Self::new(root_dir, name, cfg, key, xpub)
    }

    /// lock the LOG file of the wallet for Read and/or Write operations
    pub fn log(&self) -> Result<LogLock> {
        let dir = config::directory(self.root_dir.clone(), &self.name.as_dirname());
        let lock = LogLock::acquire_wallet_log_lock(dir)?;

        let writer = LogWriter::open(lock)?;
        Ok(writer.release_lock())
    }

    pub fn delete_log(&self) -> ::std::io::Result<()> {
        let dir = config::directory(self.root_dir.clone(), &self.name.as_dirname());
        let lock = LogLock::acquire_wallet_log_lock(dir.clone()).unwrap();
        lock.delete_wallet_log_lock(dir)
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

pub struct Wallets(BTreeMap<WalletName, Wallet>);
impl Wallets {
    pub fn new() -> Self { Wallets(BTreeMap::new())}

    pub fn load(root_dir: PathBuf) -> Result<Self> {
        let mut wallets = Wallets::new();

        let wallets_dir = config::wallet_directory(&root_dir);
        for entry in ::std::fs::read_dir(wallets_dir).unwrap() {
            let entry = entry.unwrap();
            if ! entry.file_type().unwrap().is_dir() {
                warn!("unexpected file in wallet directory: {:?}", entry.path());
                continue;
            }
            let s = entry.file_name().into_string().unwrap_or_else(|err| {
                panic!("invalid utf8... {:?}", err)
            });

            if let Some(name) = WalletName::new(s) {
                // load the wallet
                let wallet = Wallet::load(root_dir.clone(), name);
                wallets.insert(wallet.name.clone(), wallet);
            } else {
                warn!("unexpected file in wallet directory: {:?}", entry.path());
                continue
            }
        }
        Ok(wallets)
    }
}
impl ::std::ops::Deref for Wallets {
    type Target = BTreeMap<WalletName, Wallet>;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl ::std::ops::DerefMut for Wallets {
    fn deref_mut(& mut self) -> &mut Self::Target { &mut self.0 }
}
impl IntoIterator for Wallets {
    type Item     = <BTreeMap<WalletName, Wallet> as IntoIterator>::Item;
    type IntoIter = <BTreeMap<WalletName, Wallet> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}
