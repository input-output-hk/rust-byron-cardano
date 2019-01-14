use refpack;
use std::collections::VecDeque;
use storage_units::reffile;
use types::BlockHash;

use super::super::{Error, Result, Storage};
use super::reverse_iter;

pub struct Range(VecDeque<BlockHash>);

pub fn iter(storage: &Storage, from: BlockHash, to: BlockHash) -> Result<Range> {
    let ri = reverse_iter(storage, to.into())?;
    let mut rp = VecDeque::new();
    let mut finished = false;

    for block in ri {
        let hash = block.header().compute_hash().into();
        rp.push_front(hash);
        if hash == from {
            finished = true;
            break;
        }
    }

    if !finished {
        Err(Error::BlockNotFound(to.into()))
    } else {
        Ok(Range(rp))
    }
}

impl Range {
    #[deprecated(note = "use Storage::range")]
    pub fn new(storage: &Storage, from: BlockHash, to: BlockHash) -> Result<Self> {
        iter(storage, from, to)
    }

    pub fn refpack(self) -> reffile::Lookup {
        let v: Vec<BlockHash> = self.0.into();
        v.into()
    }

    pub fn iter<'a>(&'a self) -> refpack::Iter<'a, BlockHash> {
        self.0.iter()
    }
}
impl Iterator for Range {
    type Item = BlockHash;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }
}
