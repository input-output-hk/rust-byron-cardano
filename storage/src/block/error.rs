use types::{BlockHash};
use std::{result};

#[derive(Debug)]
pub enum Error {
    NoTagHead,
    InvalidHeaderHash(Vec<u8>),
    HashNotFound(BlockHash)
}

pub type Result<T> = result::Result<T, Error>;

