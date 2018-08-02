use protocol;
use mstream::{MStream, MetricStart, MetricStats};
use cardano::{config::{ProtocolMagic}};
use rand;
use std::{net::{SocketAddr, ToSocketAddrs}, ops::{Deref, DerefMut}};
use cardano::block::{Block, BlockHeader, RawBlock, HeaderHash};
use protocol::command::*;

use network::{Error, Result};
use network::api::{Api, BlockRef};

/// native peer
pub struct PeerPool {
    pub name: String,

    /// the domain or the IP address of the peer
    pub address: String,

    /// there is at least one connection
    ///
    /// multiple connections if the IP addresses are different
    /// when contacting the DNS resolver
    pub connections: Vec<Connection>
}
impl PeerPool {
    pub fn new(name: String, address: String, protocol_magic: ProtocolMagic) -> Result<Self> {
        let mut connections = Vec::new();
        for sockaddr in address.to_socket_addrs()? {
            match Connection::new(sockaddr, protocol_magic) {
                Ok(connection) => connections.push(connection),
                Err(Error::ConnectionTimedOut) => {
                    warn!("connection peer `{}' address {} timed out, ignoring for now.", name, sockaddr)
                },
                Err(err) => {
                    error!("connection peer `{}' address {} failed: {:?}", name, sockaddr, err);
                    return Err(err)
                },
            }
        }
        Ok(PeerPool { name, address, connections })
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
            Some(conn) => conn.get_tip()
        }
    }

    fn get_block(&mut self, hash: &HeaderHash) -> Result<RawBlock> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_block(hash)
        }
    }

    fn get_blocks<F>( &mut self
                    , from: &BlockRef
                    , inclusive: bool
                    , to: &BlockRef
                    , got_block: &mut F
                    )
        where F: FnMut(&HeaderHash, &Block, &RawBlock) -> ()
    {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_blocks(from, inclusive, to, got_block)
        }
    }
}

pub struct Connection(pub SocketAddr, pub OpenPeer);
impl Connection {
    pub fn new(sockaddr: SocketAddr, protocol_magic: ProtocolMagic) -> Result<Self> {
        let network = OpenPeer::new(protocol_magic, &sockaddr)?;
        Ok(Connection (sockaddr, network))
    }
}
impl Deref for Connection {
    type Target = OpenPeer;
    fn deref(&self) -> &Self::Target { &self.1 }
}
impl DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target { & mut self.1 }
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
        let block_headers_raw = GetBlockHeader::tip().execute(&mut self.0).expect("to get one header at least");

        let block_headers = block_headers_raw.decode()?;

        if block_headers.len() != 1 {
            panic!("get head header return more than 1 header")
        }
        Ok(block_headers[0].clone())
    }

    fn get_block(&mut self, hash: &HeaderHash) -> Result<RawBlock> {
        let b = GetBlock::only(&hash).execute(&mut self.0)
            .expect("to get one block at least");

        Ok(RawBlock::from_dat(b[0].as_ref().to_vec()))
    }

    fn get_blocks<F>( &mut self
                    , from: &BlockRef
                    , inclusive: bool
                    , to: &BlockRef
                    , got_block: &mut F
                    )
        where F: FnMut(&HeaderHash, &Block, &RawBlock) -> ()
    {
        let mut inclusive = inclusive;
        let mut from = from.clone();

        loop {
            // FIXME: Work around a GetBlockHeader bug: it fails on
            // the interval (x.parent, x].
            if (inclusive && from.hash == to.hash) || (!inclusive && from.hash == to.parent) {
                let block_raw = self.get_block(&to.hash).unwrap();
                got_block(&to.hash, &block_raw.decode().unwrap(), &block_raw);
                return;
            }

            if inclusive {
                if from.date > to.date { break }
                info!("  ### get headers [{}..{}]", from.hash, to.hash);
            } else {
                if from.date >= to.date { break }
                info!("  ### get headers ({}..{}]", from.hash, to.hash);
            }
            let metrics = self.read_start();
            let block_headers_raw = GetBlockHeader::range(
                &vec![from.hash.clone()], to.hash.clone())
                .execute(&mut self.0).expect("to get one header at least");
            let hdr_metrics = self.read_elapsed(&metrics);
            let block_headers = block_headers_raw.decode().unwrap();
            info!("  got {} headers  ( {} )", block_headers.len(), hdr_metrics);

            assert!(!block_headers.is_empty());

            let start = 0;
            let end = block_headers.len() - 1;

            info!("  start {} {} <- {}", block_headers[start].compute_hash(), block_headers[start].get_blockdate(), block_headers[start].get_previous_header());
            info!("  end   {} {} <- {}", block_headers[end].compute_hash(), block_headers[end].get_blockdate(), block_headers[end].get_previous_header());

            // The server will return the oldest ~2000 blocks starting at
            // 'from'. However, they're in reverse order. Thus the last
            // element of 'block_headers' should have 'from' as its
            // parent.
            assert!(block_headers[end].get_previous_header() == from.hash);

            let start_hash = if inclusive { block_headers[end].get_previous_header() } else { block_headers[end].compute_hash() };
            let end_hash = block_headers[start].compute_hash();

            info!("  get blocks [{}..{}]", start_hash, end_hash);

            let metrics = self.read_start();
            let blocks_raw = GetBlock::from(&start_hash, &end_hash)
                .execute(&mut self.0)
                .expect("to get one block at least");
            let blocks_metrics = self.read_elapsed(&metrics);
            info!("  got {} blocks  ( {} )", blocks_raw.len(), blocks_metrics);

            assert!(!blocks_raw.is_empty());

            for block_raw in blocks_raw.iter() {
                let block = block_raw.decode().unwrap();
                let hdr = block.get_header();
                let date = hdr.get_blockdate();
                let blockhash = hdr.compute_hash();

                //info!("  got block {} {} prev {}", blockhash, date, hdr.get_previous_header());

                if !inclusive && hdr.get_previous_header() != from.hash {
                    panic!("previous header doesn't match: hash {} date {} got {} expected {}",
                           blockhash, date, hdr.get_previous_header(), from.hash)
                }

                got_block(&hdr.compute_hash(), &block, &block_raw);

                from = BlockRef {
                    hash: blockhash,
                    parent: hdr.get_previous_header(),
                    date: date
                };
                inclusive = false;
            }
        }
    }
}
