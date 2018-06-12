use std::{io};
use protocol::{self, ntt};
use hyper;
use raw_cbor;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    NttError(ntt::Error),
    ProtocolError(protocol::Error),
    CborError(raw_cbor::Error),
    HyperError(hyper::Error),
    ConnectionTimedOut,
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl From<protocol::Error> for Error {
    fn from(e: protocol::Error) -> Self { Error::ProtocolError(e) }
}
impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self { Error::HyperError(e) }
}
impl From<ntt::Error> for Error {
    fn from(e: ntt::Error) -> Self { Error::NttError(e) }
}
impl From<raw_cbor::Error> for Error {
    fn from(e: raw_cbor::Error) -> Self { Error::CborError(e) }
}
