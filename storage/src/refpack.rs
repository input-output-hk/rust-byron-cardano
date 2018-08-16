//! pack of references, in a certain order

use types::{BlockHash, HASH_SIZE};
use std::collections::vec_deque::{VecDeque};
use std::{io, fs, result, fmt};
use config::{StorageConfig};

pub use std::collections::vec_deque::{Iter};

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
    pub fn push_back_missing(&mut self) { self.push_back([0u8; HASH_SIZE]) }
    pub fn push_front_missing(&mut self) { self.push_front([0u8; HASH_SIZE]) }

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
impl ::std::ops::Deref for RefPack {
    type Target = VecDeque<BlockHash>;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl ::std::ops::DerefMut for RefPack {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

pub fn read_refpack<P: AsRef<str>>(storage_config: &StorageConfig, name: P) -> Result<RefPack> {
    let mut file = fs::File::open(storage_config.get_refpack_filepath(name))?;
    RefPack::read(&mut file)
}

pub fn write_refpack<P: AsRef<str>>(storage_config: &StorageConfig, name: P, rf: &RefPack) -> Result<()> {
    let path = storage_config.get_refpack_filepath(name);
    let mut file = fs::File::create(path).unwrap();
    rf.write(&mut file)
}
