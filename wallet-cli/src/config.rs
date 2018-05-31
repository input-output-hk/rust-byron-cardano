use std::path::{PathBuf};
use std::env::{home_dir};
use storage::{self, config::StorageConfig};

#[derive(Debug)]
pub struct Config {
    pub network: String,
    pub root_dir: PathBuf,
}
impl Default for Config {
    fn default() -> Self {
        let mut storage_dir = home_dir().unwrap();
        storage_dir.push(".ariadne/");
        Config::new(storage_dir, "mainnet".to_owned())
    }
}

impl Config {
    pub fn new(root_dir: PathBuf, network: String) -> Self {
        Config {
            network: network,
            root_dir: root_dir,
        }
    }

    pub fn get_network_dir(&self) -> PathBuf {
        // TODO: check if `network`  starts with a `/`. if that is the case
        // it is an absolute path and it means the user wanted to use this
        // directly instead of our standard profile.
        let mut blk_dir_default = self.root_dir.clone();
        blk_dir_default.push("networks");
        blk_dir_default.push(self.network.as_str());
        blk_dir_default
    }

    pub fn get_storage_config(&self) -> StorageConfig {
        StorageConfig::new(&self.get_network_dir())
    }
    pub fn get_storage(&self) -> storage::Result<storage::Storage> {
        storage::Storage::init(&self.get_storage_config())
    }
}
