use std::{io};
use protocol::{self, ntt};
use wallet_crypto::{cbor};
use curl;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    NttError(ntt::Error),
    ProtocolError(protocol::Error),
    CborError(cbor::Value, cbor::Error),
    CurlError(curl::Error),
    ConnectionTimedOut,
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl From<protocol::Error> for Error {
    fn from(e: protocol::Error) -> Self { Error::ProtocolError(e) }
}
impl From<ntt::Error> for Error {
    fn from(e: ntt::Error) -> Self { Error::NttError(e) }
}
impl From<curl::Error> for Error {
    fn from(e: curl::Error) -> Self { Error::CurlError(e) }
}
impl From<(cbor::Value, cbor::Error)> for Error {
    fn from((v, e): (cbor::Value, cbor::Error)) -> Self { Error::CborError(v, e) }
}
