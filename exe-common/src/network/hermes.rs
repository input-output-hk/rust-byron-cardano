use wallet_crypto::config::{ProtocolMagic};
use rand;
use std::{net::{SocketAddr, ToSocketAddrs}, ops::{Deref, DerefMut}};
use blockchain::{BlockHeader, Block, HeaderHash};
use storage::{self, Storage, types::{PackHash}};

use config::net;

use network::{Error, Result};
use network::api::{Api, FetchEpochParams, FetchEpochResult};

use std::io::{Write};
use curl::easy::Easy;


/// hermes end point
pub struct HermesEndPoint {
    pub url: String,
    pub blockchain: String,
}

impl HermesEndPoint {
    pub fn new(url: String, blockchain: String) -> Self {
        HermesEndPoint { url, blockchain }
    }

    pub fn handle(&mut self, path: &str) -> Result<Easy> {
        let mut handle = Easy::new();

        handle.url(&format!("{}/{}/{}", self.url, self.blockchain, path))?;
        Ok(handle)
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
        let path = format!("epoch/{}", fep.epoch_id);

        let mut handle = self.handle(&path)?;
        let mut writer = storage::pack::RawBufPackWriter::init(&storage.config);
        handle.write_function(|data| {
            writer.append(&data);
            Ok(data.len())
        });
        handle.perform()?;
        let (packhash, index) = writer.finalize();

        let (_, tmpfile) = storage::pack::create_index(storage, &index);
        tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash)).unwrap();

        Ok(FetchEpochResult {
            previous_last_header_hash: result.0, // TODO
            last_header_hash: result.1, // TODO
            packhash: packhash
        })
    }
}
