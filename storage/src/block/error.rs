use types::{BlockHash};
use std::{result};
use wallet_crypto::{hash};

#[derive(Debug)]
pub enum Error {
    NoTagHead,
    InvalidHeaderHash(hash::Error),
    HashNotFound(BlockHash)
}
impl From<hash::Error> for Error {
    fn from(e: hash::Error) -> Self { Error::InvalidHeaderHash(e) }
}

pub type Result<T> = result::Result<T, Error>;

