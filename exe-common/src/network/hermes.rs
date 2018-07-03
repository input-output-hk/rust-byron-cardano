use cardano::block::{block, BlockHeader, Block, HeaderHash};
use storage::{self, Storage, tmpfile::{TmpFile}};
use std::io::{Write, Seek, SeekFrom};
use std::time::{SystemTime};

use config::net;

use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;

use network::{Result, Error};
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
        format!("{}/{}", self.url, path)
    }
}

impl Api for HermesEndPoint {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        let uri = self.uri("tip");
        info!("querying uri: {}", uri);

        let mut err = None;

        let mut bh_bytes = Vec::with_capacity(4096);
        {
            let client = Client::new(&self.core.handle());
            let work = client.get(uri.parse().unwrap()).from_err::<Error>()
                .and_then(|res| {
                if !res.status().is_success() {
                    err = Some(Error::HttpError(uri, res.status().clone()));
                };
                res.body().from_err::<Error>().for_each(|chunk| {
                    bh_bytes.write_all(&chunk).map_err(From::from)
                })
            });
            let now = SystemTime::now();
            self.core.run(work)?;
            let time_elapsed = now.elapsed().unwrap();
            info!("Downloaded TIP in {}sec", time_elapsed.as_secs());
        }

        if let Some(err) = err { return Err(err) };

        let bh_raw = block::RawBlockHeader::from_dat(bh_bytes);
        Ok(bh_raw.decode()?)
    }

    fn get_block(&mut self, _hash: HeaderHash) -> Result<Block> {
        unimplemented!()
    }

    fn fetch_epoch(&mut self, _config: &net::Config, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult> {
        let path = format!("epoch/{}", fep.epoch_id);

        let mut tmppack = TmpFile::create(storage.config.get_filetype_dir(storage::types::StorageFileType::Pack))?;
        {
            let uri = self.uri(&path).as_str().parse().unwrap();
            info!("querying uri: {}", uri);
            let client = Client::new(&self.core.handle());
            let work = client.get(uri).and_then(|res| {
                res.body().for_each(|chunk| {
                    tmppack.write_all(&chunk).map_err(From::from)
                })
            });
            let now = SystemTime::now();
            self.core.run(work)?;
            let time_elapsed = now.elapsed().unwrap();
            info!("Downloaded EPOCH in {}sec", time_elapsed.as_secs());

        }
        let now = SystemTime::now();
        tmppack.seek(SeekFrom::Start(0))?;
        let mut packfile = storage::pack::PackReader::from(tmppack);
        let mut packwriter = storage::pack::PackWriter::init(&storage.config);
        let mut last = None;
        while let Some(rblock) = packfile.get_next() {
            let rhdr = rblock.to_header();
            // TODO: do some checks: let block = rblock.decode()?;
            last = Some(rhdr.decode()?);
            packwriter.append(rhdr.compute_hash().bytes(), rblock.as_ref());
        }

        let (packhash, index) = packwriter.finalize();
        let (_, tmpfile) = storage::pack::create_index(storage, &index);
        tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash))?;
        storage::epoch::epoch_create(&storage.config, &packhash, fep.epoch_id);

        let last_hdr = match last {
            None => { panic!("no last block found, error.") },
            Some(blk) => blk
        };
        let time_elapsed = now.elapsed().unwrap();
        info!("Processing EPOCH in {}sec", time_elapsed.as_secs());

        Ok(FetchEpochResult {
            last_header_hash: last_hdr.compute_hash(),
            next_epoch_hash: None,
            packhash: packhash
        })
    }
}
