//! objects to iterate through the blocks depending on the backend used
//! 

use super::super::{Storage, block_location, block_read_location};
use super::super::tag;
use blockchain::{HeaderHash, Block};
use wallet_crypto::{cbor};

use std::{iter};

use super::error::{Error, Result};

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
            Some(bytes) => {
                let blk : Block = cbor::decode_from_cbor(&bytes).unwrap();
                // TODO, we might have a special case for when we see the first GenesisBlock
                let hdr = blk.get_header();
                self.current_block = Some(hdr.get_previous_header());
                Some(blk)
            }
        }
    }
}