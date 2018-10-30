use std::{error, fmt, io, mem};

use futures::{Async, Future, Poll};
use tokio_io::{AsyncRead, AsyncWrite};

/// Future object to terminate a connection with a peer.
///
/// Once this future has successfully returned the expected value it
/// be discarded as any other attempt to poll value from it will
/// result to an error.
///
pub struct Closing<T> {
    inner: Option<T>,
}
impl<T> Closing<T> {
    pub fn new(inner: T) -> Self {
        Closing { inner: Some(inner) }
    }
}

impl<T: AsyncRead + AsyncWrite> Future for Closing<T> {
    type Item = T;
    type Error = ClosingError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some(inner) = mem::replace(&mut self.inner, None) {
            Ok(Async::Ready(inner))
        } else {
            Err(ClosingError::AlreadyClosed)
        }
    }
}

/// Error that may happen while closing the connection with a
/// remove NT
#[derive(Debug)]
pub enum ClosingError {
    /// this is in case the underlying operation reported an error
    /// (it is required by the AsyncRead/AsyncWrite dependency).
    IoError(io::Error),

    AlreadyClosed,
}
impl From<io::Error> for ClosingError {
    fn from(e: io::Error) -> Self {
        ClosingError::IoError(e)
    }
}
impl fmt::Display for ClosingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClosingError::IoError(_) => write!(f, "I/O Error"),
            ClosingError::AlreadyClosed => write!(
                f,
                "The connecting object was already closed and should have not been reused"
            ),
        }
    }
}
impl error::Error for ClosingError {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            ClosingError::IoError(ref err) => Some(err),
            ClosingError::AlreadyClosed => None,
        }
    }
}
