use protocol;
use mstream::{MStream, MetricStart, MetricStats};
use wallet_crypto::config::{ProtocolMagic};
use rand;
use std::{net::{SocketAddr, ToSocketAddrs}, ops::{Deref, DerefMut}};
use blockchain::{BlockHeader, Block, HeaderHash};
use storage::{Storage, types::{PackHash}};
use protocol::command::*;

use network::{Error, Result};
use network::api::{Api, FetchEpochParams, FetchEpochResult};

/// native peer
pub struct Peer {
    pub name: String,

    /// the domain or the IP address of the peer
    pub address: String,

    /// there is at least one connection
    ///
    /// multiple connections if the IP addresses are different
    /// when contacting the DNS resolver
    pub connections: Vec<Connection>
}
impl Peer {
    pub fn new(name: String, address: String, protocol_magic: ProtocolMagic) -> Result<Self> {
        let mut connections = Vec::new();
        for sockaddr in address.to_socket_addrs()? {
            match Connection::new(sockaddr, protocol_magic) {
                Ok(connection) => connections.push(connection),
                Err(Error::ConnectionTimedOut) => {
                    warn!("connection peer `{}' address {} timedout, ignoring for now.", name, sockaddr)
                },
                Err(err) => {
                    error!("connection peer `{}' address {} failed: {:?}", name, sockaddr, err);
                    return Err(err)
                },
            }
        }
        Ok(Peer { name, address, connections })
    }
}

// TODO: this is not necessarily what we want to do here,
//
// in the case we have multiple connection on a peer, we might want to operate
// paralellisation of the effort
impl Api for Peer {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_tip()
        }
    }

    fn get_block(&mut self, hash: HeaderHash) -> Result<Block> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_block(hash)
        }
    }

    fn fetch_epoch(&mut self, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.fetch_epoch(storage, fep)
        }
    }
}

pub struct Connection(pub SocketAddr, pub Network);
impl Connection {
    pub fn new(sockaddr: SocketAddr, protocol_magic: ProtocolMagic) -> Result<Self> {
        let network = Network::new(protocol_magic, &sockaddr)?;
        Ok(Connection (sockaddr, network))
    }
}
impl Deref for Connection {
    type Target = Network;
    fn deref(&self) -> &Self::Target { &self.1 }
}
impl DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target { & mut self.1 }
}

pub struct Network(pub protocol::Connection<MStream>);

impl Network {
    pub fn new(protocol_magic: ProtocolMagic, host: &SocketAddr) -> Result<Self> {
        let drg_seed = rand::random();
        let mut hs = protocol::packet::Handshake::default();
        hs.protocol_magic = protocol_magic;

        let stream = MStream::init(host)?;

        let conn = protocol::ntt::Connection::handshake(drg_seed, stream)?;
        let mut conne = protocol::Connection::new(conn);
        conne.handshake(&hs)?;
        Ok(Network(conne))
    }

    pub fn read_start(&self) -> MetricStart {
        MetricStart::new(self.0.get_backend().get_read_sz())
    }

    pub fn read_elapsed(&self, start: &MetricStart) -> MetricStats {
        start.diff(self.0.get_backend().get_read_sz())
    }
}
impl Api for Network {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        let block_headers_raw = GetBlockHeader::tip().execute(&mut self.0).expect("to get one header at least");

        let block_headers = block_headers_raw.decode()?;

        if block_headers.len() != 1 {
            panic!("get head header return more than 1 header")
        }
        Ok(block_headers[0].clone())
    }

    fn get_block(&mut self, hash: HeaderHash) -> Result<Block> {
        unimplemented!()
    }

    fn fetch_epoch(&mut self, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult> {
        unimplemented!()
    }
}
