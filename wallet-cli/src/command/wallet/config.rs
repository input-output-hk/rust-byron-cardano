//! wallet configuration
//!
//! defines everything that correspond to a wallet per se in the prometheus
//! and more specifically in the ariadne environment.
//!

#![allow(dead_code)]

use cardano::{
    self,
    hdwallet::{XPrv, DerivationScheme},
    fee::{SelectionPolicy},
    wallet::{self, Wallet, Account},
    bip::bip44
};
use exe_common::config::{net};
use std::{io, slice::{Iter}, result, path::{PathBuf, Path}, env::{VarError, self, home_dir}, fs};
use std::{num::{ParseIntError}, collections::{BTreeMap}};
use storage::{self, tmpfile::{TmpFile}};
use serde_yaml;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    VarError(VarError),
    WalletError(wallet::Error),
    Bip44Error(bip44::Error),
    YamlError(serde_yaml::Error),
    ParseIntError(ParseIntError),
    AccountIndexNotFound(bip44::Account),
    StorageError(storage::Error),
    AccountAliasNotFound(String),
    BlockchainConfigError(&'static str)
}
impl From<VarError> for Error {
    fn from(e: VarError) -> Error { Error::VarError(e) }
}
impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Error { Error::ParseIntError(e) }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error { Error::IoError(e) }
}
impl From<wallet::Error> for Error {
    fn from(e: wallet::Error) -> Error { Error::WalletError(e) }
}
impl From<bip44::Error> for Error {
    fn from(e: bip44::Error) -> Error { Error::Bip44Error(e) }
}
impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Error { Error::YamlError(e) }
}
impl From<storage::Error> for Error {
    fn from(e: storage::Error) -> Error { Error::StorageError(e) }
}

pub type Result<T> = result::Result<T, Error>;

static FILENAME : &'static str = "config.yml";

/// config of a given Wallet
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// the name of the local associated network
    ///
    /// it can be a string, a relative path or an absolute path.
    pub blockchain: PathBuf,

    /// useful for spending, so far only
    pub selection_fee_policy: SelectionPolicy,

    /// TODO, this needs to be encrypted in the very near future
    pub cached_root_key: XPrv,

    /// epoch when the wallet was created. this affect recovery
    /// and the safe default is 0, where we don't skip any epoch
    pub epoch_start: u32,
}
impl Config {
    /// construct a wallet configuration from the given wallet and blockchain name
    ///
    pub fn from_wallet<P: Into<PathBuf>>(wallet: Wallet, blockchain: P, epoch_start: Option<u32>) -> Self {
        Config {
            blockchain: blockchain.into(),
            selection_fee_policy: wallet.selection_policy,
            cached_root_key: wallet.cached_root_key,
            epoch_start: epoch_start.unwrap_or(0),
        }
    }

    /// retrieve the blockchain configuration associated to the wallet
    pub fn blockchain_config(&self) -> Result<net::Config> {
        let path = ariadne_path()?.join("networks").join(&self.blockchain).join("config.yml");
        match net::Config::from_file(path.clone()) {
            None => {
                error!("error with blockchain config file {:?}", path);
                Err(Error::BlockchainConfigError("unable to parse wallet config file"))
            },
            Some(cfg) => Ok(cfg)
        }
    }

    pub fn blockchain_storage_config(&self) -> Result<storage::StorageConfig> {
        let path = ariadne_path()?.join("networks").join(&self.blockchain);

        Ok(storage::StorageConfig::new(&path))
    }

    pub fn blockchain_storage(&self) -> Result<storage::Storage> {
        Ok(storage::Storage::init(&self.blockchain_storage_config()?)?)
    }

    /// construct the wallet object from the wallet configuration
    pub fn wallet(&self) -> Result<Wallet> {
        let blockchain_config = self.blockchain_config()?;
        let wallet_cfg = cardano::config::Config::new(blockchain_config.protocol_magic);
        Ok(Wallet::new(self.cached_root_key.clone(), wallet_cfg, self.selection_fee_policy))
    }

    pub fn to_file<P: AsRef<Path>>(&self, name: &P) -> Result<()> {
        let path = wallet_path(name)?;
        fs::DirBuilder::new().recursive(true).create(path.clone())?;
        let mut tmpfile = TmpFile::create(path.clone())?;
        serde_yaml::to_writer(&mut tmpfile, self)?;
        tmpfile.render_permanent(&path.join(FILENAME))?;
        Ok(())
    }

    pub fn from_file<P: AsRef<Path>>(name: &P) -> Result<Self> {
        let path = wallet_path(name)?.join(FILENAME);
        let mut file = fs::File::open(path)?;
        serde_yaml::from_reader(&mut file).map_err(Error::YamlError)
    }
}

#[derive(Debug)]
pub struct Accounts(Vec<account::Config>);
impl Accounts {
    pub fn new() -> Self { Accounts(Vec::new()) }

    pub fn new_account(&mut self, wallet: &Wallet, alias: Option<String>) -> Result<Account> {
        let account_index = self.0.len() as u32;
        let account = wallet.account(account_index)?;
        let account_cfg = account::Config::from_account(account.clone(), alias);
        self.0.push(account_cfg);
        Ok(account)
    }

    pub fn iter(&self) -> Iter<account::Config> { self.0.iter() }

    pub fn get_account_index(&self, account_index: u32) -> Result<Account> {
        let account = bip44::Account::new(account_index)?;

        match self.0.get(account_index as usize) {
            None => Err(Error::AccountIndexNotFound(account)),
            Some(cfg) => Ok(Account::new(account, cfg.cached_root_key.clone(), DerivationScheme::V2)),
        }
    }

    pub fn get_account_alias(&self, alias: &str) -> Result<Account> {
        let alias_ = Some(alias.to_owned());
        match self.iter().position(|cfg| cfg.alias == alias_) {
            None => Err(Error::AccountAliasNotFound(alias.to_owned())),
            Some(idx) => self.get_account_index(idx as u32)
        }
    }

    pub fn to_files<P: AsRef<Path>>(&self, name: P) -> Result<()> {
        let dir = wallet_path(name)?;
        fs::DirBuilder::new().recursive(true).create(dir.clone())?;
        for index in 0..self.0.len() {
            let account_cfg = &self.0[index];
            let account = bip44::Account::new(index as u32)?;

            let mut tmpfile = TmpFile::create(dir.clone())?;
            serde_yaml::to_writer(&mut tmpfile, account_cfg)?;
            tmpfile.render_permanent(&dir.join(format!("{}{}.yml", account::PREFIX, account)))?;
        }
        Ok(())
    }

    pub fn from_files<P: AsRef<Path>>(name: &P) -> Result<Self> {
        let dir = wallet_path(name)?;
        let mut accounts = Self::new();

        let mut to_read = BTreeMap::new();
        let mut indices = 0;

        for entry in fs::read_dir(dir.clone())? {
            let entry = entry?;
            if entry.file_type()?.is_dir() { continue; }
            let name = entry.file_name();
            if let Some(name) = name.to_str() {
                if name.starts_with(account::PREFIX) && name.ends_with(".yml") {
                    let index = name.trim_left_matches(account::PREFIX).trim_right_matches(".yml").parse::<u32>()?;
                    to_read.insert(index, name.to_owned());
                    indices += 1;
                }
            }
        }

        let mut expected_index = 0;
        for (index, filename) in to_read {
            assert!(index == expected_index);
            let path = dir.join(filename);
            let mut file = fs::File::open(path)?;
            accounts.0.push(serde_yaml::from_reader(file)?);
            expected_index = index + 1;
        }
        assert_eq!(indices, expected_index);

        Ok(accounts)
    }
}

pub mod account {
    use cardano::{bip::bip44, coin::Coin, wallet::{Account}, hdwallet::{XPub}};

    pub static PREFIX : &'static str = "account-";

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Config {
        pub alias: Option<String>,
        pub addresses: Vec<bip44::Addressing>,
        pub balance: Coin,
        pub cached_root_key: XPub
    }
    impl Config {
        pub fn from_account(account: Account, alias: Option<String>) -> Self {
            Config {
                alias: alias,
                addresses: Vec::new(),
                balance: Coin::zero(),
                cached_root_key: account.cached_account_key
            }
        }
    }
}

// ***************************************** TO MOVE IN ANOTHER MODULE ************************************************** //
//                                                                                                                        //
// The following should be accessible in every other module of this binary.                                               //
// Move this to another file.                                                                                             //
//                                                                                                                        //
// ********************************************************************************************************************** //

/// the environment variable to define where the Ariadne files are stores
///
/// this will include all the cardano network you will connect to (mainnet, testnet, ...),
/// the different wallets you will create and all metadata.
pub static ARIADNE_PATH_ENV : &'static str = "ARIADNE_PATH";

/// the home directory hidden directory where to find Ariadne files.
///
/// # TODO
///
/// This is not standard on windows, set the appropriate setting here
///
pub static ARIADNE_HOME_PATH : &'static str = ".ariadne";

/// get the root directory of all the ariadne path
///
/// it is either environment variable `ARIADNE_PATH` or the `${HOME}/.ariadne`
pub fn ariadne_path() -> Result<PathBuf> {
    match env::var(ARIADNE_PATH_ENV) {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(VarError::NotPresent) => match home_dir() {
            None => Err(Error::BlockchainConfigError("no home directory to base ariadne root dir. Set ARIADNE_PATH` variable environment to fix the problem.")),
            Some(path) => Ok(path.join(ARIADNE_HOME_PATH))
        },
        Err(err) => Err(Error::VarError(err))
    }
}

/// retrieve the root path of the given wallet by its name
pub fn wallet_path<P: AsRef<Path>>(wallet_name: P) -> Result<PathBuf> {
    ariadne_path().map(|p| p.join("wallets").join(wallet_name))
}
