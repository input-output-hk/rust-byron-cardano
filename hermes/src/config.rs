use serde_yaml;

use storage::{self, tmpfile::{TmpFile}};
use storage::config::StorageConfig;
use exe_common::config::{net};
use std::{io, result, path::{PathBuf, Path}, env::{VarError, self, home_dir}, fs};
use std::{num::{ParseIntError}, collections::{BTreeMap}, sync::{Arc}};

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    VarError(VarError),
    YamlError(serde_yaml::Error),
    ParseIntError(ParseIntError),
    StorageError(storage::Error),
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
impl From<storage::Error> for Error {
    fn from(e: storage::Error) -> Error { Error::StorageError(e) }
}
impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Error { Error::YamlError(e) }
}

type Result<T> = result::Result<T, Error>;

/// Configuration file for the Wallet CLI
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub root_dir: PathBuf,
    pub port: u16
}

impl Default for Config {
    fn default() -> Self {
        let storage_dir = hermes_path().unwrap().join("networks");
        Config::new(storage_dir, 80)
    }
}

impl Config {
    pub fn new(root_dir: PathBuf, port: u16) -> Self {
        Config {
            root_dir: root_dir,
            port: port
        }
    }

    pub fn open() -> Result<Self> {
        let p = hermes_path()?.join("config.yml");
        Self::from_file(p)
    }

    pub fn save(&self) -> Result<PathBuf> {
        let p = hermes_path()?.join("config.yml");
        self.to_file(p)
    }

    pub fn get_networks_dir(&self) -> PathBuf { self.root_dir.clone() }

    pub fn get_networks(&self) -> Result<Networks> {
        let dir = self.get_networks_dir();
        let mut networks = Networks::new();

        for entry in fs::read_dir(dir.clone())? {
            let entry = entry?;
            if ! entry.file_type()?.is_dir() { continue; }
            let name = entry.file_name();
            if let Some(name) = name.to_str() {
                let network = Network {
                    path: entry.path().to_path_buf(),
                    config: self.get_network_config(name)?,
                    storage: Arc::new(self.get_storage(name)?)
                };
                networks.insert(name.to_owned(), network);
            }
        }

        Ok(networks)
    }

    pub fn get_network_config<P: AsRef<Path>>(&self, name: P) -> Result<net::Config> {
        let path = self.get_networks_dir().join(name).join("config.yml");
        match net::Config::from_file(&path) {
            None => {
                error!("error while parsing config file: {:?}", path);
                Err(Error::BlockchainConfigError("error while parsing network config file"))
            },
            Some(cfg) => Ok(cfg)
        }
    }

    pub fn get_storage_config<P: AsRef<Path>>(&self, name: P) -> StorageConfig {
        StorageConfig::new(&self.get_networks_dir().join(name))
    }
    pub fn get_storage<P: AsRef<Path>>(&self, name: P) -> Result<storage::Storage> {
        let cfg = storage::Storage::init(&self.get_storage_config(name))?;
        Ok(cfg)
    }

    /// read the file associated to the given filepath, if the file does not exists
    /// this function creates the default `Config`;
    ///
    pub fn from_file<P: AsRef<Path>>(p: P) -> Result<Self> {
        use std::fs::{File};

        let path = p.as_ref();
        let mut file = File::open(path)?;
        Ok(serde_yaml::from_reader(&mut file)?)
    }

    /// write the config in the given file
    ///
    /// if the file already exists it will erase the original data.
    pub fn to_file<P: AsRef<Path>>(&self, p: P) -> Result<P> {
        let dir = p.as_ref().parent().unwrap().to_path_buf();
        fs::DirBuilder::new().recursive(true).create(dir.clone())?;
        let mut file = TmpFile::create(dir)?;
        serde_yaml::to_writer(&mut file, &self)?;
        file.render_permanent(&p.as_ref().to_path_buf())?;
        Ok(p)
    }
}

pub struct Network {
    pub path: PathBuf,
    pub config: net::Config,
    pub storage: Arc<storage::Storage>,
}
pub type Networks = BTreeMap<String, Network>;

/// the environment variable to define where the Hermes files are stores
///
/// this will include all the cardano network you will connect to (mainnet, testnet, ...),
/// the different wallets you will create and all metadata.
pub static HERMES_PATH_ENV : &'static str = "HERMES_PATH";

/// the home directory hidden directory where to find Hermes files.
///
/// # TODO
///
/// This is not standard on windows, set the appropriate setting here
///
pub static HERMES_HOME_PATH : &'static str = ".hermes";

/// get the root directory of all the hermes path
///
/// it is either environment variable `HERMES_PATH` or the `${HOME}/.hermes`
pub fn hermes_path() -> Result<PathBuf> {
    match env::var(HERMES_PATH_ENV) {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(VarError::NotPresent) => match home_dir() {
            None => Err(Error::BlockchainConfigError("no home directory to base hermes root dir. Set `HERMES_PATH' variable environment to fix the problem.")),
            Some(path) => Ok(path.join(HERMES_HOME_PATH))
        },
        Err(err) => Err(Error::VarError(err))
    }
}
