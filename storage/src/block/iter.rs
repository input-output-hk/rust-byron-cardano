//! objects to iterate through the blocks depending on the backend used
//! 

use super::super::{Storage, block_location, block_read_location};
use super::super::tag;
use blockchain::{HeaderHash, Block};
use wallet_crypto::{cbor};
use types::{BlockHash};
use refpack::{RefPack};
use refpack;

use std::{result, iter};

#[derive(Debug)]
pub enum Error {
    NoTagHead,
    InvalidHeaderHash(Vec<u8>),
    HashNotFound(BlockHash)
}

pub type Result<T> = result::Result<T, Error>;

/// reverse iterator over the block chain
pub struct ReverseIter<'a> {
    storage: &'a Storage,
    current_block: Option<HeaderHash>
}
impl<'a> ReverseIter<'a> {
    pub fn from(storage: &'a Storage, bh: &[u8]) -> Result<Self> {
        let hh = match HeaderHash::from_slice(&bh) {
            None => return Err(Error::InvalidHeaderHash(bh.iter().cloned().collect())),
            Some(hh) => hh
        };
        // TODO: check location of the hash actually exists
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

pub struct Range(RefPack);
impl Range {
    pub fn new(storage: &Storage, from: BlockHash, to: BlockHash) -> Result<Self> {
        let ri = ReverseIter::from(storage, &to[..])?;
        let mut rp = RefPack::new();
        let mut finished = false;

        for block in ri {
            let hash = block.get_header().compute_hash().into_bytes();
            rp.push_front(hash);
            if hash == from { finished = true; break; }
        }

        if ! finished {
            Err(Error::HashNotFound(to))
        } else {
            Ok(Range(rp))
        }
    }

    pub fn refpack(self) -> RefPack { self.0 }

    pub fn iter<'a>(&'a self) -> refpack::Iter<'a, BlockHash> { self.0.iter() }
}

