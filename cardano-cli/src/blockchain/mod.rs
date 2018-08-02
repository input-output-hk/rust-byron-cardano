pub mod config;
pub mod commands;
mod peer;

use std::path::PathBuf;

use exe_common::network::api::BlockRef;
pub use exe_common::{config::net::{self, Config, Peer, Peers}, network};
use storage::{tag, Storage, config::{StorageConfig}};
use cardano::block;

const LOCAL_BLOCKCHAIN_TIP_TAG : &'static str = "tip";

/// handy structure to use to manage and orginise a blockchain
///
pub struct Blockchain {
    pub name: String,
    pub dir: PathBuf,
    pub storage_config: StorageConfig,
    pub storage: Storage,
    pub config: Config,
}
impl Blockchain {
    /// create the new blockhain with the given setting
    pub fn new(root_dir: PathBuf, name: String, config: Config) -> Self {
        let dir = config::directory(root_dir, &name);
        let storage_config = StorageConfig::new(&dir);

        let storage = Storage::init(&storage_config).unwrap();
        let file = storage_config.get_config_file();
        config.to_file(file);

        // by default, the config file comes with pre-set remote peers,
        // check that, for every peer, we add them to the fold
        for peer in config.peers.iter() {
            let tag = format!("remote/{}", peer.name());
            tag::write_hash(&storage, &tag, &config.genesis)
        }

        let blockchain = Blockchain {
            name,
            dir,
            storage_config,
            storage,
            config,
        };

        blockchain.save_tip(&blockchain.config.genesis);

        blockchain
    }

    /// load the blockchain
    pub fn load(root_dir: PathBuf, name: String) -> Self {
        let dir = config::directory(root_dir, &name);
        let storage_config = StorageConfig::new(&dir);
        let storage = Storage::init(&storage_config).unwrap();

        let file = storage_config.get_config_file();
        let config = Config::from_file(file).unwrap();

        Blockchain {
            name,
            dir,
            storage_config,
            storage,
            config
        }
    }

    /// save the blockchain settings
    pub fn save(&self) {
        self.config.to_file(self.storage_config.get_config_file());
    }

    /// add a peer to the blockchain
    pub fn add_peer(&mut self, remote_alias: String, remote_endpoint: String) {
        let tag = self.mk_remote_tag(&remote_alias);

        let peer = Peer::new(remote_endpoint);
        self.config.peers.push(remote_alias, peer);

        tag::write_hash(&self.storage, &tag, &self.config.genesis)
    }

    pub fn mk_remote_tag(&self, remote: &str) -> String {
        format!("remote/{}", remote)
    }

    /// remove a peer from the blockchain
    pub fn remove_peer(&mut self, remote_alias: String) {
        self.config.peers = self.config.peers.iter().filter(|np| np.name() != remote_alias).cloned().collect();
        let tag = self.mk_remote_tag(&remote_alias);
        tag::remove_tag(&self.storage, &tag);
    }

    pub fn peers<'a>(&'a self)
        -> impl Iterator<Item = &'a net::NamedPeer>
    {
        self.config.peers.iter()
    }

    pub fn set_wallet_tag(&self, wallet_name: &str, hh: &block::HeaderHash) {
        let tag = format!("wallet/{}", wallet_name);
        tag::write_hash(&self.storage, &tag, hh)
    }
    pub fn remove_wallet_tag(&self, wallet_name: &str) {
        let tag = format!("wallet/{}", wallet_name);
        tag::remove_tag(&self.storage, &tag);
    }

    pub fn load_tip(&self) -> (BlockRef, bool) {
        let genesis_ref = (BlockRef {
            hash: self.config.genesis.clone(),
            parent: self.config.genesis_prev.clone(),
            date: block::BlockDate::Genesis(self.config.epoch_start)
        }, true);
        match self.storage.get_block_from_tag(LOCAL_BLOCKCHAIN_TIP_TAG) {
            Err(::storage::Error::NoSuchTag) => genesis_ref,
            Err(err) => panic!(err),
            Ok(block) => {
                let header = block.get_header();
                let hash = header.compute_hash();
                let is_genesis = hash == genesis_ref.0.hash;
                (BlockRef {
                    hash: hash,
                    parent: header.get_previous_header(),
                    date: header.get_blockdate()
                }, is_genesis)
            }
        }
    }
    pub fn save_tip(&self, hh: &block::HeaderHash) {
        tag::write_hash(&self.storage, &LOCAL_BLOCKCHAIN_TIP_TAG, hh);
    }
}
