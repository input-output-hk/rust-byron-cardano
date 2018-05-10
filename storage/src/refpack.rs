//! pack of references, in a certain order

use types::{BlockHash, HASH_SIZE};
use std::collections::vec_deque::{VecDeque, Iter};
use std::{io, result, fmt};

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::IoError(ref err) => write!(f, "IO Error: {}", err)
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

/// a ref pack internal structure is a VecQueue as it will
/// allow us to insert element front of the pack. This will
/// be useful when (if) exploring the blocks from the current
/// head position and iterating through the blocks backward.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct RefPack(VecDeque<BlockHash>);
impl RefPack {
    pub fn new() -> Self { RefPack(VecDeque::new()) }
    pub fn push_back(&mut self, bh: BlockHash) { self.0.push_back(bh) }
    pub fn push_front(&mut self, bh: BlockHash) { self.0.push_front(bh) }
    pub fn iter<'a>(&'a self) -> Iter<'a, BlockHash> { self.0.iter() }

    pub fn read<R: io::Read>(reader: &mut R) -> Result<Self> {
        let mut rf = Self::new();
        let mut bh = [0;HASH_SIZE];
        while let HASH_SIZE = reader.read(&mut bh[..])? {
            rf.push_back(bh);
        }
        Ok(rf)
    }

    pub fn write<W: io::Write>(&self, writer: &mut W) -> Result<()> {
        for bh in self.iter() { writer.write_all(&bh[..])?; }
        Ok(())
    }
}