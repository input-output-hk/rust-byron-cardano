use std::{io};
use protocol::{self, ntt};
use wallet_crypto::{cbor};

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    NttError(ntt::Error),
    ProtocolError(protocol::Error),
    CborError(cbor::Value, cbor::Error),
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
impl From<(cbor::Value, cbor::Error)> for Error {
    fn from((v, e): (cbor::Value, cbor::Error)) -> Self { Error::CborError(v, e) }
}
