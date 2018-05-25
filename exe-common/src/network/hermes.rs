use blockchain::{BlockHeader, Block, HeaderHash};
use storage::{self, Storage, types::{PackHash}};

use config::net;

use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;

use network::{Result};
use network::api::{Api, FetchEpochParams, FetchEpochResult};


/// hermes end point
pub struct HermesEndPoint {
    pub url: String,
    pub blockchain: String,
    core: Core
}

impl HermesEndPoint {
    pub fn new(url: String, blockchain: String) -> Self {
        HermesEndPoint { url, blockchain, core: Core::new().unwrap() }
    }

    pub fn uri(& mut self, path: &str) -> String {
        format!("{}/{}/{}", self.url, self.blockchain, path)
    }
}

impl Api for HermesEndPoint {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        unimplemented!()
    }

    fn get_block(&mut self, _hash: HeaderHash) -> Result<Block> {
        unimplemented!()
    }

    fn fetch_epoch(&mut self, _config: &net::Config, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult> {
        let path = format!("epoch/{}", fep.epoch_id);

        let mut writer = storage::pack::RawBufPackWriter::init(&storage.config);
        {
            let uri = self.uri(&path).as_str().parse().unwrap();
            let client = Client::new(&self.core.handle());
            let work = client.get(uri).and_then(|res| {
                debug!("Response: {}", res.status());

                res.body().for_each(|chunk| {
                    info!("received: {} bytes", chunk.len());
                    writer.append(&chunk);
                    Ok(())
                })
            });
            self.core.run(work)?;

        }
        let (packhash, index) = writer.finalize();

        let (_, tmpfile) = storage::pack::create_index(storage, &index);
        tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash)).unwrap();
        storage::epoch::epoch_create(&storage.config, &packhash, fep.epoch_id);

        let last = match writer.last() {
            None => { panic!("no last block found, error.") },
            Some(blk) => blk
        };
        let last_hdr = last.get_header();

        Ok(FetchEpochResult {
            previous_last_header_hash: last_hdr.get_previous_header(),
            last_header_hash: last_hdr.compute_hash(),
            packhash: packhash
        })
    }
}
