use types::{BlockHash};
use std::{result, io};
use cardano::{hash};
use cardano::block::BlockDate;
use cbor_event;

#[derive(Debug)]
pub enum Error {
    NoTagHead,
    IoError(io::Error),
    BlockEncodingError(cbor_event::Error),
    InvalidHeaderHash(hash::Error),
    HashNotFound(BlockHash),
    DateNotAvailable(BlockDate),
}
impl From<hash::Error> for Error {
    fn from(e: hash::Error) -> Self { Error::InvalidHeaderHash(e) }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self { Error::BlockEncodingError(e) }
}

pub type Result<T> = result::Result<T, Error>;
