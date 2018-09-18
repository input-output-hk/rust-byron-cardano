use cardano::{block::{block, Block, BlockHeader, BlockDate, RawBlock, HeaderHash}, tx::{TxAux}};
use cardano::hash::{HASH_SIZE_256};
use storage;
use std::io::Write;
use std::time::{SystemTime, Duration};
use std::thread;

use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;

use network::{Result, Error};
use network::api::{Api, BlockRef};

// Time between get_tip calls. FIXME: make configurable?
static NETWORK_REFRESH_FREQUENCY: Duration = Duration::from_secs(60 * 10);

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

    fn wait_for_new_tip(&mut self, prev_tip: &HeaderHash) -> Result<BlockHeader> {
        loop {
            let new_tip = self.get_tip()?;
            if new_tip.compute_hash() != *prev_tip { return Ok(new_tip) }

            info!("Sleeping for {:?}", NETWORK_REFRESH_FREQUENCY);
            thread::sleep(NETWORK_REFRESH_FREQUENCY);
        }
    }

    fn get_block(&mut self, hash: &HeaderHash) -> Result<RawBlock> {
        let uri = self.uri(&format!("block/{}", hash));
        info!("querying uri: {}", uri);
        let client = Client::new(&self.core.handle());
        let mut block_raw = vec!();
        let mut err = None;
        {
            let work = client.get(uri.parse().unwrap()).and_then(|res| {
                if !res.status().is_success() {
                    err = Some(Error::HttpError(uri, res.status().clone()));
                };
                res.body().for_each(|chunk| {
                    block_raw.append(&mut chunk.to_vec());
                    Ok(())
                })
            });
            let now = SystemTime::now();
            self.core.run(work)?;
            let time_elapsed = now.elapsed().unwrap();
            info!("Downloaded block in {}sec", time_elapsed.as_secs());
        }
        if let Some(err) = err { return Err(err) };
        Ok(RawBlock::from_dat(block_raw))
    }

    fn get_blocks<F>( &mut self
                    , from: &BlockRef
                    , inclusive: bool
                    , to: &BlockRef
                    , got_block: &mut F
                    ) -> Result<()>
        where F: FnMut(&HeaderHash, &Block, &RawBlock) -> ()
    {
        let mut inclusive = inclusive;
        let mut from = from.clone();

        loop {

            // FIXME: hack
            if let BlockDate::Normal(d) = from.date {
                if d.slotid == 21599 && !inclusive {
                    from = BlockRef {
                        hash: HeaderHash::from([0;HASH_SIZE_256]), // FIXME: use None?
                        parent: from.hash.clone(),
                        date: BlockDate::Genesis(d.epoch + 1)
                    };
                    inclusive = true;
                };
            };

            let epoch = from.date.get_epochid();

            if !inclusive && to.hash == from.hash { break }

            if inclusive && from.date.is_genesis() && epoch < to.date.get_epochid() {

                // Fetch a complete epoch.

                let mut tmppack = vec!();
                let mut err = None;

                {
                    let uri = self.uri(&format!("epoch/{}", epoch));
                    info!("querying uri: {}", uri);
                    let client = Client::new(&self.core.handle());
                    let work = client.get(uri.parse().unwrap()).and_then(|res| {
                        if !res.status().is_success() {
                            err = Some(Error::HttpError(uri, res.status().clone()));
                        };
                        res.body().for_each(|chunk| {
                            tmppack.append(&mut chunk.to_vec());
                            Ok(())
                        })
                    });
                    let now = SystemTime::now();
                    self.core.run(work)?;
                    let time_elapsed = now.elapsed().unwrap();
                    info!("Downloaded EPOCH in {}sec", time_elapsed.as_secs());
                }

                if let Some(err) = err { return Err(err) };

                let mut packfile = storage::containers::packfile::Reader::from(&tmppack[..]);

                while let Some(data) = packfile.get_next() {
                    let block_raw = block::RawBlock(data);
                    let block = block_raw.decode()?;
                    let hdr = block.get_header();

                    assert!(hdr.get_blockdate().get_epochid() == epoch);
                    //assert!(from.date != hdr.get_blockdate() || from.hash == hdr.compute_hash());

                    if from.date <= hdr.get_blockdate() {
                        got_block(&hdr.compute_hash(), &block, &block_raw);
                    }

                    from = BlockRef {
                        hash: hdr.compute_hash(),
                        parent: hdr.get_previous_header(),
                        date: hdr.get_blockdate()
                    };
                    inclusive = false;
                }
            }

            else {

                //assert!(from.date.get_epochid() == to.date.get_epochid());

                let mut blocks = vec![];
                let mut to = to.hash.clone();

                loop {
                    let block_raw = self.get_block(&to)?;
                    let block = block_raw.decode()?;
                    let hdr = block.get_header();
                    assert!(hdr.get_blockdate() >= from.date);
                    let prev = hdr.get_previous_header();
                    blocks.push((hdr.compute_hash(), block, block_raw));
                    if (inclusive && prev == from.parent)
                        || (!inclusive && prev == from.hash)
                    {
                        break
                    }
                    to = prev;
                }

                while let Some((hash, block, block_raw)) = blocks.pop() {
                    got_block(&hash, &block, &block_raw);
                }

                break;
            }
        }

        Ok(())
    }

    fn send_transaction( &mut self, _txaux: TxAux) -> Result<bool> {
        Ok(false)
    }
}
