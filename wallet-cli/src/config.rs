use std::path::{Path, PathBuf};
use std::env::{home_dir};
use std::io;
use clap::{ArgMatches, Arg, SubCommand, App};
use serde_yaml;

use account::{Account};
use wallet::{Wallet};
use command::{HasCommand};

use storage;
use storage::config::StorageConfig;

use wallet_crypto::config::{ProtocolMagic};

/// Configuration file for the Wallet CLI
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub accounts: Vec<Account>,
    pub wallet: Option<Wallet>,
    pub network_type: String,
    pub network_domain: String,
    pub network_genesis: String,
    pub protocol_magic: ProtocolMagic,
    pub root_dir: PathBuf,
    pub block_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        let mut storage_dir = home_dir().unwrap();
        storage_dir.push(".ariadne/");
        Config::new_mainnet(storage_dir)
    }
}

impl Config {
    pub fn new_mainnet(root_dir: PathBuf) -> Self {
        Config {
            accounts: vec![Account::default()],
            wallet: None,
            network_type: "mainnet".to_string(),
            network_domain: "relays.cardano-mainnet.iohk.io:3000".to_string(),
            network_genesis: "89D9B5A5B8DDC8D7E5A6795E9774D97FAF1EFEA59B2CAF7EAF9F8C5B32059DF4".to_string(),
            protocol_magic: ProtocolMagic::default(),
            root_dir: root_dir,
            block_dir: None,
        }
    }

    pub fn new_testnet(root_dir: PathBuf) -> Self {
        Config {
            accounts: vec![Account::default()],
            wallet: None,
            network_type: "testnet".to_string(),
            network_domain: "relays.awstest.iohkdev.io:3000".to_string(),
            network_genesis: "".to_string(),
            protocol_magic: ProtocolMagic::new(633343913),
            root_dir: root_dir,
            block_dir: None,
        }
    }

    pub fn get_block_dir(&self) -> PathBuf {
        match self.block_dir {
            None    => {
                let mut blk_dir_default = self.root_dir.clone();
                blk_dir_default.push("blocks");
                blk_dir_default
            },
            Some(ref v) => v.clone(),
        }
        //match self.clone().block_dir.unwrap_or(blk_dir_default)
    }

    pub fn get_storage_config(&self) -> StorageConfig {
        StorageConfig::new(&self.get_block_dir(), &self.network_type)
    }
    pub fn get_storage(&self) -> io::Result<storage::Storage> {
        storage::Storage::init(&self.get_storage_config())
    }

    /// read the file associated to the given filepath, if the file does not exists
    /// this function creates the default `Config`;
    ///
    pub fn from_file<P: AsRef<Path>>(p: P) -> Self {
        use std::fs::{File};

        let path = p.as_ref();
        if ! path.is_file() {
            return Self::default();
        }

        let mut file = File::open(path).unwrap();
        serde_yaml::from_reader(&mut file).unwrap()
    }

    pub fn find_account(&self, account: &Account) -> Option<u32> {
        self.accounts.iter().position(|acc| acc == account)
            .map(|v| v as u32)
            .map(|v| v | 0x80000000)
    }

    /// write the config in the given file
    ///
    /// if the file already exists it will erase the original data.
    pub fn to_file<P: AsRef<Path>>(&self, p: P) {
        use std::fs::{File};

        let mut file = File::create(p.as_ref()).unwrap();
        serde_yaml::to_writer(&mut file, &self).unwrap();
    }

    pub fn to_yaml(&self) -> serde_yaml::Value {
        serde_yaml::to_value(self).unwrap()
    }
    pub fn from_yaml(value: serde_yaml::Value) -> Self {
        serde_yaml::from_value(value).unwrap()
    }

    fn get(&self, path: &[serde_yaml::Value]) -> serde_yaml::Value {
        let mut obj = self.to_yaml();

        for e in path {
            obj = if obj.is_sequence() {
                obj.as_sequence().unwrap().get(e.as_u64().unwrap() as usize).unwrap().clone()
            } else {
                obj.get(e).unwrap().clone()
            }
        }

        obj
    }

    fn set(&mut self, path: &[serde_yaml::Value], value: serde_yaml::Value) {
        let mut obj = self.to_yaml();

        {
            let mut objr = &mut obj;

            for e in path {
                let mut objr_c = objr;
                objr = if objr_c.is_sequence() {
                    objr_c.as_sequence_mut().unwrap().get_mut(e.as_u64().unwrap() as usize).unwrap()
                } else if objr_c.is_mapping() {
                    objr_c.as_mapping_mut().unwrap().get_mut(e).unwrap()
                } else {
                    panic!("not a value")
                };
            }

            *objr = value;
        }

        *self = Self::from_yaml(obj)
    }
}

impl HasCommand for Config {
    type Output = Option<Config>;

    fn clap_options<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name("config")
            .about("get or set info from the config")
            .arg(Arg::with_name("path").help("path to a given option in the config").index(1).required(true))
            .arg(Arg::with_name("value").help("value to set to the given option").index(2).required(false))
    }

    fn run(cfg: Config, args: &ArgMatches) -> Self::Output {
        let path   : Vec<serde_yaml::Value> = args.value_of("path").unwrap().split('.').map(|s| serde_yaml::from_str(s).unwrap()).collect();

        match args.value_of("value") {
            None => {
                let r = cfg.get(&path);
                match r {
                    serde_yaml::Value::Null => println!(""),
                    serde_yaml::Value::Bool(b) => println!("{}", b),
                    serde_yaml::Value::Number(n) => println!("{}", n),
                    serde_yaml::Value::String(n) => println!("{}", n),
                    serde_yaml::Value::Sequence(n) => {
                        for e in n {
                            println!("{:?}", e);
                        }
                    },
                    serde_yaml::Value::Mapping(n) => {
                        for e in n {
                            println!("{:?}", e);
                        }
                    }
                };
                None
            },
            Some(val) => {
                let value : serde_yaml::Value = serde_yaml::from_str(val).unwrap();
                let mut cpy = cfg;
                cpy.set(&path, value);
                Some(cpy)
             },
        }
    }
}
