use cardano::config::ProtocolMagic;
use cardano::{
    block::{Block, BlockHeader, HeaderHash, RawBlock},
    tx::TxAux,
};
use config;
use network::api::{BlockRef, *};
use network::{hermes, native, ntt, Error, Result};
use std::net::ToSocketAddrs;

/// network object to handle a peer connection and redirect to constructing
/// the appropriate network protocol object (native, http...)
pub enum Peer {
    Native(native::PeerPool),
    Http(hermes::HermesEndPoint),
    Ntt(ntt::NetworkCore),
}
impl Peer {
    pub fn new(
        network: String,
        name: String,
        cfg: config::net::Peer,
        protocol_magic: ProtocolMagic,
    ) -> Result<Self> {
        match cfg {
            config::net::Peer::Native(addr) => Ok(Peer::Native(native::PeerPool::new(
                name,
                addr,
                protocol_magic,
            )?)),
            config::net::Peer::Http(addr) => {
                Ok(Peer::Http(hermes::HermesEndPoint::new(addr, network)))
            }
            config::net::Peer::Ntt(addr) => {
                let mut addrs_iter = addr
                    .to_socket_addrs()
                    .or_else(|_| Err(Error::InvalidPeerAddress(addr.to_string())))?;
                match addrs_iter.next() {
                    Some(addr) => ntt::NetworkCore::new(addr, protocol_magic).map(Peer::Ntt),
                    None => Err(Error::InvalidPeerAddress(addr.to_string())),
                }
            }
        }
    }
}
impl Api for Peer {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        match self {
            Peer::Native(peer) => peer.get_tip(),
            Peer::Http(endpoint) => endpoint.get_tip(),
            Peer::Ntt(endpoint) => endpoint.get_tip(),
        }
    }

    fn wait_for_new_tip(&mut self, prev_tip: &HeaderHash) -> Result<BlockHeader> {
        match self {
            Peer::Native(peer) => peer.wait_for_new_tip(prev_tip),
            Peer::Http(endpoint) => endpoint.wait_for_new_tip(prev_tip),
            Peer::Ntt(endpoint) => endpoint.wait_for_new_tip(prev_tip),
        }
    }

    fn get_block(&mut self, hash: &HeaderHash) -> Result<RawBlock> {
        match self {
            Peer::Native(peer) => peer.get_block(hash),
            Peer::Http(endpoint) => endpoint.get_block(hash),
            Peer::Ntt(endpoint) => endpoint.get_block(hash),
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
        match self {
            Peer::Native(peer) => peer.get_blocks(from, inclusive, to, got_block),
            Peer::Http(endpoint) => endpoint.get_blocks(from, inclusive, to, got_block),
            Peer::Ntt(endpoint) => endpoint.get_blocks(from, inclusive, to, got_block),
        }
    }

    fn send_transaction(&mut self, txaux: TxAux) -> Result<bool> {
        match self {
            Peer::Native(peer) => peer.send_transaction(txaux),
            Peer::Http(endpoint) => endpoint.send_transaction(txaux),
            Peer::Ntt(endpoint) => endpoint.send_transaction(txaux),
        }
    }
}
