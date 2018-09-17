use types::{BlockHash};
use std::{result, io, fmt, error};
use cardano::{hash, util::hex};
use cardano::block::BlockDate;
use cbor_event;

#[derive(Debug)]
pub enum Error {
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
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(_) => write!(f, "I/O error"),
            Error::BlockEncodingError(_) => write!(f, "Block is encoded in an unknown format or is corrupted"),
            Error::InvalidHeaderHash(_) => write!(f, "Invalid Block Hash"),
            Error::HashNotFound(bh) => write!(f, "Block hash not found `{}`", &hex::encode(bh)),
            Error::DateNotAvailable(bd) => write!(f, "Block date not available `{}`", bd),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::IoError(ref err)            => Some(err),
            Error::BlockEncodingError(ref err) => Some(err),
            Error::InvalidHeaderHash(ref err)  => Some(err),
            Error::HashNotFound(_)             => None,
            Error::DateNotAvailable(_)         => None,
        }
    }
}

pub type Result<T> = result::Result<T, Error>;
