use wallet_crypto::config::{ProtocolMagic};
use rand;
use std::{net::{SocketAddr, ToSocketAddrs}, ops::{Deref, DerefMut}};
use blockchain::{BlockHeader, Block, HeaderHash};
use storage::{Storage, types::{PackHash}};

use hyper::Client;
use tokio_core::reactor::Core;
use config::net;

use network::{Error, Result};
use network::api::{Api, FetchEpochParams, FetchEpochResult};

/// hermes end point
pub struct HermesEndPoint {
    pub url: String,
    pub core: Core,
}

impl HermesEndPoint {
    pub fn new(url: &String) -> Self {
        HermesEndPoint { url: url, core = Core::new()?; }
    }
}

impl Api for HermesEndPoint {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        unimplemented!()
    }

    fn get_block(&mut self, hash: HeaderHash) -> Result<Block> {
        unimplemented!()
    }

    fn fetch_epoch(&mut self, config: &net::Config, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult> {
        let uri_str = format!("{}/epoch/{}", self.url, fep.epoch_id);
        let uri = uri_str.parse()?;
        let client = Client::new(&self.core.handle());
        let work = client.get(uri);
        work.and_then(|res| {
            println!("Response: {}", res.status());

            //res.body()
        });
    }
}
