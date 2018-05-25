use config;
use network::{native, Error, Result};
use wallet_crypto::config::{ProtocolMagic};

/// network object to handle a peer connection and redirect to constructing
/// the appropriate network protocol object (native, http...)
pub enum Peer {
    Native(native::Peer),
}
impl Peer {
    pub fn new(name: String, cfg: config::net::Peer, protocol_magic: ProtocolMagic) -> Result<Self> {
        match cfg {
            config::net::Peer::Native(addr) => {
                Ok(Peer::Native(native::Peer::new(name, addr, protocol_magic)?))
            },
            config::net::Peer::Http(addr) => {
                unimplemented!("connot to connect to peer (`{}') address `{}'", name, addr);
            }
        }
    }
}
