use cardano::block::HeaderHash;
use cardano_storage as storage;
use cbor_event;
use hyper;
use protocol::{self, ntt};
use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    NttError(ntt::Error),
    ProtocolError(protocol::Error),
    CborError(cbor_event::Error),
    HyperError(hyper::Error),
    ConnectionTimedOut,
    HttpError(String, hyper::StatusCode),
    NoSuchBlock(HeaderHash),
    StorageError(storage::Error),
    BlockError(cardano::block::Error),
    InvalidPeerAddress(String),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}
impl From<protocol::Error> for Error {
    fn from(e: protocol::Error) -> Self {
        Error::ProtocolError(e)
    }
}
impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self {
        Error::HyperError(e)
    }
}
impl From<ntt::Error> for Error {
    fn from(e: ntt::Error) -> Self {
        Error::NttError(e)
    }
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self {
        Error::CborError(e)
    }
}
impl From<storage::Error> for Error {
    fn from(e: storage::Error) -> Self {
        Error::StorageError(e)
    }
}
impl From<cardano::block::Error> for Error {
    fn from(e: cardano::block::Error) -> Self {
        Error::BlockError(e)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(_) => write!(f, "I/O Error"),
            Error::NttError(_) => write!(f, "Low level protocol error"),
            Error::ProtocolError(_) => write!(f, "Blockchain protocol error"),
            Error::CborError(_) => write!(f, "Data encoding error"),
            Error::HyperError(_) => write!(f, "Error in HTTP engine"),
            Error::ConnectionTimedOut => write!(f, "connection time out"),
            Error::HttpError(err, code) => write!(f, "HTTP error {}: {}", code, err),
            Error::NoSuchBlock(hash) => write!(f, "Requested block {} does not exist", hash),
            Error::StorageError(_) => write!(f, "Storage error"),
            Error::BlockError(_) => write!(f, "Block error"),
            Error::InvalidPeerAddress(addr) => write!(f, "Invalid peer address {}", addr),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::IoError(ref err) => Some(err),
            Error::NttError(ref err) => Some(err),
            Error::ProtocolError(ref err) => Some(err),
            Error::CborError(ref err) => Some(err),
            Error::HyperError(ref err) => Some(err),
            Error::ConnectionTimedOut => None,
            Error::HttpError(_, _) => None,
            Error::NoSuchBlock(_) => None,
            Error::StorageError(ref err) => Some(err),
            Error::BlockError(ref err) => Some(err),
            Error::InvalidPeerAddress(_) => None,
        }
    }
}
