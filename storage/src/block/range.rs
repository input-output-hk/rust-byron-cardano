use refpack;
use super::super::{Storage};
use types::{BlockHash};

use super::iter::{ReverseIter, Error, Result};

pub struct Range(refpack::RefPack);
impl Range {
    pub fn new(storage: &Storage, from: BlockHash, to: BlockHash) -> Result<Self> {
        let ri = ReverseIter::from(storage, &to[..])?;
        let mut rp = refpack::RefPack::new();
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

    pub fn refpack(self) -> refpack::RefPack { self.0 }

    pub fn iter<'a>(&'a self) -> refpack::Iter<'a, BlockHash> { self.0.iter() }
}

