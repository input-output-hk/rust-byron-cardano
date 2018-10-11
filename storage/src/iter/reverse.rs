//! objects to iterate through the blocks depending on the backend used
//!

use super::super::{Storage, block_location, block_read_location};
use cardano::block::{HeaderHash, Block};

use std::{iter};

use super::super::{Error, Result};

/// reverse iterator over the block chain
pub struct ReverseIter<'a> {
    storage: &'a Storage,
    current_block: Option<HeaderHash>
}
impl<'a> ReverseIter<'a> {
    pub fn from(storage: &'a Storage, hh: HeaderHash) -> Result<Self> {
        let hash = hh.clone().into();
        if let None = block_location(storage, &hash) {
            return Err(Error::HashNotFound(hh));
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

        let hash = hh.clone().into();
        let loc = block_location(&self.storage, &hash).expect("block location");
        match block_read_location(&self.storage, &loc, &hash) {
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

