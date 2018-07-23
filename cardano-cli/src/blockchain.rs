use std::path::PathBuf;

pub use exe_common::config::net::{Config, Peer, Peers};
use storage::{tag, Storage, config::{StorageConfig}};

use utils::term::Term;

fn blockchain_directory( root_dir: PathBuf
                       , name: &str
                       ) -> PathBuf
{
    root_dir.join("blockchains").join(name)
}

pub fn command_new( mut term: Term
                  , root_dir: PathBuf
                  , name: String
                  , config: Config
                  )
{
    let blockchain = Blockchain::new(root_dir, name.clone(), config);
    blockchain.save();

    term.success(&format!("local blockchain `{}' created.\n", &name)).unwrap();
}

pub fn command_remote_add( mut term: Term
                         , root_dir: PathBuf
                         , name: String
                         , remote_alias: String
                         , remote_endpoint: String
                         )
{
    let mut blockchain = Blockchain::load(root_dir, name);
    blockchain.add_peer(remote_alias.clone(), remote_endpoint);
    blockchain.save();

    term.success(&format!("remote `{}' node added to blockchain `{}'\n", remote_alias, blockchain.name)).unwrap();
}

pub fn command_remote_rm( mut term: Term
                        , root_dir: PathBuf
                        , name: String
                        , remote_alias: String
                        )
{
    let mut blockchain = Blockchain::load(root_dir, name);
    blockchain.remove_peer(remote_alias.clone());
    blockchain.save();

    term.success(&format!("remote `{}' node removed from blockchain `{}'\n", remote_alias, blockchain.name)).unwrap();
}

struct Blockchain {
    name: String,
    storage_config: StorageConfig,
    storage: Storage,
    config: Config,
}
impl Blockchain {
    fn new(root_dir: PathBuf, name: String, config: Config) -> Self {
        let dir = blockchain_directory(root_dir, &name);
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

        Blockchain {
            name,
            storage_config,
            storage,
            config,
        }
    }
    fn load(root_dir: PathBuf, name: String) -> Self {
        let dir = blockchain_directory(root_dir, &name);
        let storage_config = StorageConfig::new(&dir);
        let storage = Storage::init(&storage_config).unwrap();

        let file = storage_config.get_config_file();
        let config = Config::from_file(file).unwrap();

        Blockchain {
            name,
            storage_config,
            storage,
            config
        }
    }
    fn save(&self) {
        self.config.to_file(self.storage_config.get_config_file());
    }
    fn add_peer(&mut self, remote_alias: String, remote_endpoint: String) {
        let peer = Peer::new(remote_endpoint);
        self.config.peers.push(remote_alias.clone(), peer);

        let tag = format!("remote/{}", remote_alias);
        tag::write_hash(&self.storage, &tag, &self.config.genesis)
    }
    fn remove_peer(&mut self, remote_alias: String) {
        self.config.peers = self.config.peers.iter().filter(|np| np.name() != remote_alias).cloned().collect();
        let tag = format!("remote/{}", remote_alias);
        tag::remove_tag(&self.storage, &tag);
    }
}
