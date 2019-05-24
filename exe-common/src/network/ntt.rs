use futures::Future;
use tokio::runtime::Runtime;

use network::api::{Api, BlockReceivingFlag, BlockRef};
use network::{Error, Result};

//to_socket_addr
use network_core::client::block::BlockService;
use network_ntt::client as ntt;
use std::net::SocketAddr;
use std::ops::Deref;

use cardano::{
    block::{Block, BlockHeader, HeaderHash, RawBlock},
    config::ProtocolMagic,
    tx::{TxAux, TxId},
};

pub struct NetworkCore {
    handle: ntt::ClientHandle<Block, TxId>,
    pub rt: Runtime,
}

impl NetworkCore {
    pub fn new(sockaddr: SocketAddr, magic: ProtocolMagic) -> Result<Self> {
        trace!("New network core: {}", sockaddr);
        let connecting = ntt::connect(sockaddr, ntt::ProtocolMagic::from(*magic.deref()));
        match connecting.wait() {
            Ok((connection, handle)) => {
                // FIXME: use default executor, or take
                // executor argument before merge.
                let mut rt = Runtime::new().unwrap();
                rt.spawn(
                    connection
                        .map(|_| {
                            debug!("Exited");
                        })
                        .map_err(|e| {
                            error!("NTT connection error: {:?}", e);
                        }),
                );
                Ok(NetworkCore { handle, rt })
            }
            Err(_err) => unimplemented!(),
        }
    }
}

impl Api for NetworkCore {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        self.handle
            .tip()
            .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))
            .wait()
    }

    fn wait_for_new_tip(&mut self, _prev_tip: &HeaderHash) -> Result<BlockHeader> {
        unimplemented!("not yet ready")
    }

    fn get_block(&mut self, _hash: &HeaderHash) -> Result<RawBlock> {
        unimplemented!("not yet ready")
    }

    fn get_blocks<F>(
        &mut self,
        _from: &BlockRef,
        _incluside: bool,
        _to: &BlockRef,
        _got_block: &mut F,
    ) -> Result<()>
    where
        F: FnMut(&HeaderHash, &Block, &RawBlock) -> BlockReceivingFlag,
    {
        unimplemented!("not yet ready")
    }

    fn send_transaction(&mut self, _txaux: TxAux) -> Result<bool> {
        unimplemented!("not yet ready")
    }
}
