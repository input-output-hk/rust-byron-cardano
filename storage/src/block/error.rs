use types::{BlockHash};
use std::{result, io};
use cardano::{hash};
use raw_cbor;

#[derive(Debug)]
pub enum Error {
    NoTagHead,
    IoError(io::Error),
    BlockEncodingError(raw_cbor::Error),
    InvalidHeaderHash(hash::Error),
    HashNotFound(BlockHash)
}
impl From<hash::Error> for Error {
    fn from(e: hash::Error) -> Self { Error::InvalidHeaderHash(e) }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl From<raw_cbor::Error> for Error {
    fn from(e: raw_cbor::Error) -> Self { Error::BlockEncodingError(e) }
}

pub type Result<T> = result::Result<T, Error>;

