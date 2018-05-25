use protocol;
use mstream::{MStream, MetricStart, MetricStats};
use wallet_crypto::config::{ProtocolMagic};
use rand;
use std::{net::{SocketAddr, ToSocketAddrs}};

use network::{Error, Result};

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

pub struct Connection(pub SocketAddr, pub Network);
impl Connection {
    pub fn new(sockaddr: SocketAddr, protocol_magic: ProtocolMagic) -> Result<Self> {
        let network = Network::new(protocol_magic, &sockaddr)?;
        Ok(Connection (sockaddr, network))
    }
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

