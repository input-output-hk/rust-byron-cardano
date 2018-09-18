use refpack;
use super::super::{Storage};
use types::{BlockHash};

use super::error::{Error, Result};
use super::iter::{ReverseIter};
use storage_units::reffile;
use std::collections::VecDeque;

pub struct Range(VecDeque<BlockHash>);
impl Range {
    pub fn new(storage: &Storage, from: BlockHash, to: BlockHash) -> Result<Self> {
        let ri = ReverseIter::from(storage, to.into())?;
        let mut rp = VecDeque::new();
        let mut finished = false;

        for block in ri {
            let hash = block.get_header().compute_hash().into();
            rp.push_front(hash);
            if hash == from { finished = true; break; }
        }

        if ! finished {
            Err(Error::HashNotFound(to))
        } else {
            Ok(Range(rp))
        }
    }

    pub fn refpack(self) -> reffile::Lookup {
        let v : Vec<BlockHash> = self.0.into();
        v.into()
    }

    pub fn iter<'a>(&'a self) -> refpack::Iter<'a, BlockHash> { self.0.iter() }
}
impl Iterator for Range {
    type Item = BlockHash;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }
}
