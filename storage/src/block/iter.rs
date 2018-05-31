//! objects to iterate through the blocks depending on the backend used
//!

use super::super::{Storage, StorageConfig, block_location, block_read_location};
use super::super::tag;
use super::super::epoch::epoch_read_pack;
use super::super::pack::{PackReader};
use blockchain::{HeaderHash, Block, RawBlock, EpochId};

use std::{iter, fs};

use super::error::{Error, Result};

pub struct Iter<'a> {
    storage: &'a StorageConfig,
    from:    EpochId,
    current: PackReader<fs::File>,
    end:     bool
}

impl<'a> Iter<'a> {
    /// create a block iterator, going forward, moving from epoch to epoch
    /// starting from the given epoch.
    pub fn new(storage: &'a StorageConfig, from: EpochId) -> Result<Self> {
        let current = {
            let epochref = epoch_read_pack(storage, from)?;
            PackReader::init(&storage, &epochref)
        };
        Ok(Iter { storage, from, current, end: false })
    }

    /// get the next raw block, don't attempt to decode the raw block
    pub fn next_raw(&mut self) -> Result<Option<RawBlock>> {
        match self.current.get_next() {
            Some(expr) => Ok(Some(expr)),
            None => {
                if self.end { return Ok(None); }
                self.end = true;
                let next_epoch = self.from + 1;
                self.current = {
                    let epochref = match epoch_read_pack(self.storage, next_epoch) {
                        Err(err) => {
                            if err.kind() == ::std::io::ErrorKind::NotFound {
                                return Ok(None);
                            } else {
                                return Err(Error::IoError(err));
                            }
                        },
                        Ok(c) => c
                    };
                    PackReader::init(self.storage, &epochref)
                };
                self.from = next_epoch;
                self.next_raw()
            },
        }
    }

    /// just like `next_raw` but perform the cbor decoding into block
    pub fn next_block(&mut self) -> Result<Option<Block>> {
        match self.next_raw()? {
            None => Ok(None),
            Some(raw) => Ok(Some(raw.decode()?))
        }
    }
}
impl<'a> iter::Iterator for Iter<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_block().unwrap()
    }
}

/// reverse iterator over the block chain
pub struct ReverseIter<'a> {
    storage: &'a Storage,
    current_block: Option<HeaderHash>
}
impl<'a> ReverseIter<'a> {
    pub fn from(storage: &'a Storage, bh: &[u8]) -> Result<Self> {
        let hh = HeaderHash::from_slice(&bh)?;
        if let None = block_location(storage, hh.bytes()) {
            return Err(Error::HashNotFound(hh.into_bytes()));
        }
        let ri = ReverseIter {
            storage: storage,
            current_block: Some(hh)
        };
        Ok(ri)
    }

    pub fn new(storage: &'a Storage) -> Result<Self> {
        let hh_bytes = match tag::read(&storage, &tag::HEAD) {
            None => return Err(Error::NoTagHead),
            Some(t) => t
        };
        Self::from(storage, &hh_bytes)
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
