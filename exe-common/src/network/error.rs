use std::{io};
use protocol::{self, ntt};

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    NttError(ntt::Error),
    ProtocolError(protocol::Error),
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
