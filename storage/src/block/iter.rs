//! objects to iterate through the blocks depending on the backend used
//!

use super::super::{Storage, StorageConfig, block_location, block_read, block_read_location, header_to_blockhash, packreader_init};
use super::super::{blob};
use super::super::epoch::{epoch_read_pack, epoch_open_packref};
use super::super::containers::packfile;
use super::super::types::{BlockHash, PackHash};
use cardano::block::{HeaderHash, Block, RawBlock, BlockDate};

use std::{iter, fs, mem};
use std::cmp::Ordering;

use super::error::{Error, Result};

#[derive(Clone)]
pub struct IterParams {
    storage: StorageConfig,
    start: StartIter,
    end: EndIter,
    storage_tip: HeaderHash,
}

pub struct Iter {
    config: IterParams,
    storage: Storage,
    start_date: BlockDate,
    end_date: BlockDate,
    epoch_packrefs: Vec<PackHash>,
    blocks: Vec<Block>,
    packreader: Option<packfile::Reader<fs::File>>,
}

#[derive(Clone)]
pub enum StartIter {
    Date(BlockDate),
}

#[derive(Clone)]
pub enum EndIter {
    Date(BlockDate),
    Tip,
}

impl IterParams {
    pub fn new(storage: StorageConfig, storage_tip: &HeaderHash, start: StartIter, end: EndIter) -> IterParams {
        IterParams {
            storage: storage,
            start: start,
            end: end,
            storage_tip: storage_tip.clone(),
        }
    }
}

// TODO proper error handling
fn previous_block(storage: &Storage, block: &Block) -> Block {
    let prev_hash = block.get_header().get_previous_header();
    let blk = blob::read(&storage, &header_to_blockhash(&prev_hash)).unwrap().decode().unwrap();
    blk
}

fn next_until_range(packreader: &mut packfile::Reader<fs::File>, start_date: &BlockDate, end_date: &BlockDate) -> Result<Option<Block>> {
    loop {
        match packreader.get_next() {
            None => { return Ok(None) },
            Some(b) => {
                let mut blk = RawBlock(b).decode().unwrap();
                let blk_date = blk.get_header().get_blockdate();
                if &blk_date > end_date {
                    return Ok(None)
                };
                if &blk_date >= start_date {
                    return Ok(Some(blk))
                }
            },
        }
    }
}

pub enum ReverseSearch {
    Continue,
    Found,
    Abort,
}

fn block_reverse_search_from_tip<F>(storage: &Storage, first_block: &Block, find: F) -> Result<Option<Block>>
    where F: Fn(&Block) -> Result<ReverseSearch>
{
    let mut current_blk = first_block.clone();
    loop {
        match find(&current_blk)? {
            ReverseSearch::Continue => {
                let blk = previous_block(&storage, &current_blk);
                current_blk = blk;
            },
            ReverseSearch::Found => { return Ok(Some(current_blk)) },
            ReverseSearch::Abort => { return Ok(None) },
        };
    }
}

pub fn resolve_date_to_blockhash(storage: &Storage, tip: &BlockHash, date: &BlockDate) -> Result<Option<BlockHash>> {
    let epoch = date.get_epochid();
    match epoch_open_packref(&storage.config, epoch) {
        Ok(mut handle) => {
            let slotid = match date {
                BlockDate::Genesis(_) => 0,
                BlockDate::Normal(sid) => sid.slotid,
            };
            let r = handle.getref_at_index(slotid)?;
            Ok(r)
        },
        Err(_) => {
            let tip_rblk = block_read(&storage, tip);
            match tip_rblk {
                None => return Ok(None),
                Some(rblk) => {
                    let blk = rblk.decode()?;
                    let found = block_reverse_search_from_tip(storage, &blk, |x|
                        match x.get_header().get_blockdate().cmp(date) {
                            Ordering::Equal => Ok(ReverseSearch::Found),
                            Ordering::Greater => Ok(ReverseSearch::Continue),
                            Ordering::Less => Ok(ReverseSearch::Abort),
                        })?;
                    Ok(found.map(|x| header_to_blockhash(&x.get_header().compute_hash())))
                }
            }
        },
    }
}

impl Iter {
    pub fn start(params: &IterParams) -> Result<Self> {

        let mut epoch_packrefs = Vec::new();

        let start = match params.start {
            StartIter::Date(date) => date,
        };
        let end = match params.end {
            EndIter::Date(date) => date,
            EndIter::Tip => unimplemented!(), // TODO use the code to read tip block's date that is written down there
        };

        if end <= start {
            // error
        }

        let mut iter_epoch = start.get_epochid();
        while iter_epoch <= end.get_epochid() {
            match epoch_read_pack(&params.storage, iter_epoch) {
                Ok(packref) => {
                    epoch_packrefs.push(packref);
                    iter_epoch += 1;
                },
                Err(_) => {
                    break;
                },
            };
        }

        let mut loose_blocks = Vec::new();

        let storage = Storage::init(&params.storage).unwrap(); // TODO merge errors from block and general storage

        // check if we have everything through epoch pack, no block needed in this case. if not we reverse iter the blocks
        if iter_epoch <= end.get_epochid() {
            // earliest missing block date
            let earliest = BlockDate::Genesis(iter_epoch);

            // move reading of the tip at the beginning to be able to early
            // bail if we don't have the blocks asked for.
            // also, it will need to use generic block storage, instead of
            // using blob storage.
            let tip_blk = blob::read(&storage, &header_to_blockhash(&params.storage_tip)).unwrap().decode().unwrap();
            let tip_date = tip_blk.get_header().get_blockdate();
            if tip_date < end {
                return Err(Error::DateNotAvailable(end))
            }

            // rewind until we reach the end boundary
            let mut current_date = tip_date;
            let mut current_blk = tip_blk;
            loop {
                if current_date == end {
                    break;
                }
                let blk = previous_block(&storage, &current_blk);
                current_date = blk.get_header().get_blockdate();
                current_blk = blk;
            }

            // append to loose_blocks from end (included), to earliest (included)
            while current_date >= earliest {
                loose_blocks.push(current_blk.clone());
                current_blk = previous_block(&storage, &current_blk);
                current_date = current_blk.get_header().get_blockdate();
            }

            // reverse blocks
            loose_blocks.reverse();
        }

        Ok(Iter {
            config: params.clone(),
            storage: storage,
            start_date: start,
            end_date: end,
            epoch_packrefs: epoch_packrefs,
            blocks: loose_blocks,
            packreader: None,
        })
    }

    /// get the next raw block, don't attempt to decode the raw block
    pub fn next(&mut self) -> Result<Option<Block>> {
        let mut packreader = None;
        mem::swap(&mut self.packreader, &mut packreader);
        match packreader {
            Some(mut pr) => {
                match next_until_range(&mut pr, &self.start_date, &self.end_date)? {
                    None      => self.next(),
                    Some(blk) => {
                        let mut v = Some(pr);
                        mem::swap(&mut self.packreader, &mut v);
                        Ok(Some(blk))
                    },
                }
            },
            None => {
                match self.epoch_packrefs.pop() {
                    None => {
                        match self.blocks.pop() {
                            None => Ok(None),
                            Some(blk) => Ok(Some(blk)),
                        }
                    },
                    Some(pref) => {
                        let packreader = packreader_init(&self.config.storage, &pref);
                        self.packreader = Some(packreader);
                        self.next()
                    }
                }
            },
        }
    }
}

impl<'a> iter::Iterator for Iter {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        self.next().unwrap()
    }
}

/// reverse iterator over the block chain
pub struct ReverseIter<'a> {
    storage: &'a Storage,
    current_block: Option<HeaderHash>
}
impl<'a> ReverseIter<'a> {
    pub fn from(storage: &'a Storage, hh: HeaderHash) -> Result<Self> {
        if let None = block_location(storage, hh.bytes()) {
            return Err(Error::HashNotFound(hh.into_bytes()));
        }
        let ri = ReverseIter {
            storage: storage,
            current_block: Some(hh)
        };
        Ok(ri)
    }
}
impl<'a> iter::Iterator for ReverseIter<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let hh = match &self.current_block {
            &None => return None,
            &Some(ref hh) => hh.clone(),
        };

        let loc = block_location(&self.storage, hh.bytes()).expect("block location");
        match block_read_location(&self.storage, &loc, hh.bytes()) {
            None        => panic!("error while reading block {}", hh),
            Some(blk) => {
                let block = blk.decode().unwrap();
                let hdr = block.get_header();
                self.current_block = Some(hdr.get_previous_header());
                Some(block)
            }
        }
    }
}
