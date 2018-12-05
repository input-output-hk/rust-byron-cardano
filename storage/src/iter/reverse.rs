//! objects to iterate through the blocks depending on the backend used
//!

use super::super::{block_location, block_read_location, Storage};
use cardano::block::{Block, HeaderHash};

use std::iter;

use super::super::{Result};

/// reverse iterator over the block chain
pub struct ReverseIter<'a> {
    storage: &'a Storage,
    current_block: Option<HeaderHash>,
}
impl<'a> ReverseIter<'a> {
    pub fn from(storage: &'a Storage, hh: HeaderHash) -> Result<Self> {
        let hash = hh.clone().into();
        block_location(storage, &hash)?;
        let ri = ReverseIter {
            storage: storage,
            current_block: Some(hh),
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
        let blk = block_read_location(&self.storage, &loc, &hash).unwrap();
        let block = blk.decode().unwrap();
        let hdr = block.get_header();
        self.current_block = Some(hdr.get_previous_header());
        Some(block)
    }
}
