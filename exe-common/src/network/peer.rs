use config;
use network::{native, Result, hermes};
use network::api::{*};
use wallet_crypto::config::{ProtocolMagic};
use blockchain::{BlockHeader, Block, HeaderHash};
use storage::{Storage};

/// network object to handle a peer connection and redirect to constructing
/// the appropriate network protocol object (native, http...)
pub enum Peer {
    Native(native::PeerPool),
    Http(hermes::HermesEndPoint)
}
impl Peer {
    pub fn new(network: String, name: String, cfg: config::net::Peer, protocol_magic: ProtocolMagic) -> Result<Self> {
        match cfg {
            config::net::Peer::Native(addr) => {
                Ok(Peer::Native(native::PeerPool::new(name, addr, protocol_magic)?))
            },
            config::net::Peer::Http(addr) => {
                Ok(Peer::Http(hermes::HermesEndPoint::new(addr, network)))
            }
        }
    }
}
impl Api for Peer {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        match self {
            Peer::Native(peer)   => peer.get_tip(),
            Peer::Http(endpoint) => endpoint.get_tip(),
        }
    }

    fn get_block(&mut self, hash: HeaderHash) -> Result<Block> {
        match self {
            Peer::Native(peer)   => peer.get_block(hash),
            Peer::Http(endpoint) => endpoint.get_block(hash),
        }
    }

    fn fetch_epoch(&mut self, config: &config::net::Config, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult> {
        match self {
            Peer::Native(peer)   => peer.fetch_epoch(config, storage, fep),
            Peer::Http(endpoint) => endpoint.fetch_epoch(config, storage, fep),
        }
    }
}
