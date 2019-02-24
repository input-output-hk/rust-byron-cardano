use cardano::config::ProtocolMagic;
use cardano::{
    block::{Block, BlockHeader, HeaderHash, RawBlock},
    tx::TxAux,
};
use mstream::{MStream, MetricStart, MetricStats};
use protocol;
use protocol::command::*;
use rand;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    ops::{Deref, DerefMut},
};

use network::api::{Api, BlockReceivingFlag, BlockRef};
use network::{Error, Result};

/// native peer
pub struct PeerPool {
    pub name: String,

    /// the domain or the IP address of the peer
    pub address: String,

    /// there is at least one connection
    ///
    /// multiple connections if the IP addresses are different
    /// when contacting the DNS resolver
    pub connections: Vec<Connection>,
}
impl PeerPool {
    pub fn new(name: String, address: String, protocol_magic: ProtocolMagic) -> Result<Self> {
        let mut connections = Vec::new();
        for sockaddr in address.to_socket_addrs()? {
            match Connection::new(sockaddr, protocol_magic) {
                Ok(connection) => {
                    connections.push(connection);
                    break;
                }
                Err(Error::ConnectionTimedOut) => warn!(
                    "connection peer `{}' address {} timed out, ignoring for now.",
                    name, sockaddr
                ),
                Err(err) => {
                    error!(
                        "connection peer `{}' address {} failed: {:?}",
                        name, sockaddr, err
                    );
                    return Err(err);
                }
            }
        }
        Ok(PeerPool {
            name,
            address,
            connections,
        })
    }
}

// TODO: this is not necessarily what we want to do here,
//
// in the case we have multiple connection on a peer, we might want to operate
// paralellisation of the effort
impl Api for PeerPool {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_tip(),
        }
    }

    fn wait_for_new_tip(&mut self, prev_tip: &HeaderHash) -> Result<BlockHeader> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.wait_for_new_tip(prev_tip),
        }
    }

    fn get_block(&mut self, hash: &HeaderHash) -> Result<RawBlock> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_block(hash),
        }
    }

    fn get_blocks<F>(
        &mut self,
        from: &BlockRef,
        inclusive: bool,
        to: &BlockRef,
        got_block: &mut F,
    ) -> Result<()>
    where
        F: FnMut(&HeaderHash, &Block, &RawBlock) -> BlockReceivingFlag,
    {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_blocks(from, inclusive, to, got_block),
        }
    }

    fn send_transaction(&mut self, txaux: TxAux) -> Result<bool> {
        let mut sent = false;
        for connection in self.connections.iter_mut() {
            sent |= connection.send_transaction(txaux.clone())?;
        }
        Ok(sent)
    }
}

pub struct Connection(pub SocketAddr, pub OpenPeer);
impl Connection {
    pub fn new(sockaddr: SocketAddr, protocol_magic: ProtocolMagic) -> Result<Self> {
        let network = OpenPeer::new(protocol_magic, &sockaddr)?;
        Ok(Connection(sockaddr, network))
    }
}
impl Deref for Connection {
    type Target = OpenPeer;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}
impl DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}

pub struct OpenPeer(pub protocol::Connection<MStream>);

impl OpenPeer {
    pub fn new(protocol_magic: ProtocolMagic, host: &SocketAddr) -> Result<Self> {
        let drg_seed = rand::random();
        let mut hs = protocol::packet::Handshake::default();
        hs.protocol_magic = protocol_magic;

        let stream = MStream::init(host)?;

        let conn = protocol::ntt::Connection::handshake(drg_seed, stream)?;
        let mut conne = protocol::Connection::new(conn);
        conne.handshake(&hs)?;

        // FIXME: make it configurable whether we want to subscribe to
        // receive tip updates.
        conne.subscribe()?;

        Ok(OpenPeer(conne))
    }

    pub fn read_start(&self) -> MetricStart {
        MetricStart::new(self.0.get_backend().get_read_sz())
    }

    pub fn read_elapsed(&self, start: &MetricStart) -> MetricStats {
        start.diff(self.0.get_backend().get_read_sz())
    }
}
impl Api for OpenPeer {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        if let Some(prev_tip) = self.0.get_latest_tip() {
            return Ok(prev_tip);
        }

        let block_headers_raw = GetBlockHeader::tip()
            .execute(&mut self.0)
            .expect("to get one header at least");

        let block_headers = block_headers_raw.decode()?;

        if block_headers.len() != 1 {
            panic!("get head header return more than 1 header")
        }
        Ok(block_headers[0].clone())
    }

    fn wait_for_new_tip(&mut self, prev_tip: &HeaderHash) -> Result<BlockHeader> {
        loop {
            self.0.process_message()?;
            let new_tip = self.0.get_latest_tip();
            if new_tip.is_some() && (new_tip.as_ref().unwrap().compute_hash() != *prev_tip) {
                return Ok(new_tip.unwrap());
            }
        }
    }

    fn get_block(&mut self, hash: &HeaderHash) -> Result<RawBlock> {
        let b = GetBlock::only(&hash)
            .execute(&mut self.0)
            .expect("to get one block at least");

        match b.first() {
            Some(b) => Ok(RawBlock::from_dat(b.as_ref().to_vec())),
            None => Err(Error::NoSuchBlock(hash.clone())),
        }
    }

    fn get_blocks<F>(
        &mut self,
        from: &BlockRef,
        inclusive: bool,
        to: &BlockRef,
        got_block: &mut F,
    ) -> Result<()>
    where
        F: FnMut(&HeaderHash, &Block, &RawBlock) -> BlockReceivingFlag,
    {
        if inclusive {
            let rblk = self.get_block(&from.hash)?;
            if got_block(&from.hash, &rblk.decode()?, &rblk) == BlockReceivingFlag::Stop {
                return Ok(());
            }
        }

        stream_blocks(
            &mut self.0,
            &vec![from.hash.clone()],
            to.hash.clone(),
            &mut |rblk| {
                let blk = rblk.decode()?;
                match got_block(&blk.header().compute_hash(), &blk, &rblk) {
                    BlockReceivingFlag::Continue => Ok(BlockStreamingFlag::Continue),
                    BlockReceivingFlag::Stop => Ok(BlockStreamingFlag::Stop),
                }
            },
        )?;

        Ok(())
    }

    fn send_transaction(&mut self, txaux: TxAux) -> Result<bool> {
        Ok(SendTx::new(txaux).execute(&mut self.0).map(|_| true)?)
    }
}
