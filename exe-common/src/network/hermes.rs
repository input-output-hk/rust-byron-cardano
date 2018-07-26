use cardano::block::{block, Block, BlockHeader, BlockDate, RawBlock, HeaderHash};
use storage;
use std::io::Write;
use std::time::{SystemTime};

use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;

use network::{Result, Error};
use network::api::{Api, BlockRef};


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

    fn get_block(&mut self, _hash: HeaderHash) -> Result<RawBlock> {
        unimplemented!()
    }

    fn get_blocks(&mut self, from: &BlockRef, inclusive: bool, to: &BlockRef,
                   got_block: &mut FnMut(&HeaderHash, &Block, &RawBlock) -> ())
    {
        let mut inclusive = inclusive;
        let mut from = from.clone();

        // FIXME: hack
        if let BlockDate::Normal(d) = from.date {
            if d.slotid == 21599 && !inclusive {
                from.date = BlockDate::Genesis(d.epoch + 1);
                inclusive = true;
            };
        };

        assert!(inclusive); // FIXME

        loop {

            /* Fetch a complete epoch at once? */
            let epoch = from.date.get_epochid();

            if from.date.is_genesis() && epoch < to.date.get_epochid() {

                let mut tmppack = vec!();
                {
                    let uri = self.uri(&format!("epoch/{}", epoch)).as_str().parse().unwrap();
                    info!("querying uri: {}", uri);
                    let client = Client::new(&self.core.handle());
                    let work = client.get(uri).and_then(|res| {
                        res.body().for_each(|chunk| {
                            tmppack.append(&mut chunk.to_vec());
                            Ok(())
                        })
                    });
                    let now = SystemTime::now();
                    self.core.run(work).unwrap();
                    let time_elapsed = now.elapsed().unwrap();
                    info!("Downloaded EPOCH in {}sec", time_elapsed.as_secs());
                }

                let mut packfile = storage::pack::PackReader::from(&tmppack[..]);

                while let Some(block_raw) = packfile.get_next() {
                    let block = block_raw.decode().unwrap();
                    let hdr = block.get_header();

                    assert!(hdr.get_blockdate().get_epochid() == epoch);
                    //assert!(from.date != hdr.get_blockdate() || from.hash == hdr.compute_hash());

                    if from.date <= hdr.get_blockdate() {
                        got_block(&hdr.compute_hash(), &block, &block_raw);
                    }

                    from.hash = hdr.compute_hash();
                    from.date = hdr.get_blockdate();
                    //inclusive = false;
                }

                from.date = BlockDate::Genesis(epoch + 1);
                //inclusive = true;
            }

            else {
                // FIXME: fetch the remaining blocks
                break;
            }
        }
    }
}
