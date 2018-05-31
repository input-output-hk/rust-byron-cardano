use types::{BlockHash};
use std::{result, io};
use wallet_crypto::{hash, cbor};

#[derive(Debug)]
pub enum Error {
    NoTagHead,
    IoError(io::Error),
    BlockEncodingError(cbor::Value, cbor::Error),
    InvalidHeaderHash(hash::Error),
    HashNotFound(BlockHash)
}
impl From<hash::Error> for Error {
    fn from(e: hash::Error) -> Self { Error::InvalidHeaderHash(e) }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl From<(cbor::Value, cbor::Error)> for Error {
    fn from(e: (cbor::Value, cbor::Error)) -> Self { Error::BlockEncodingError(e.0, e.1) }
}

pub type Result<T> = result::Result<T, Error>;

