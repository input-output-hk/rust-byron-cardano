use std::{error, fmt};

use bytes::{Buf, Bytes, IntoBuf};
use futures::{Async, Future, Poll};
use tokio_codec::Framed;
use tokio_io::{AsyncRead, AsyncWrite};

use super::{event, Connection, ResponseCode};

/// the connecting states
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ConnectingState {
    /// here we are awaiting to finalize the full handshake
    ToSendHandshake,
    /// here we are awaiting the response to validate
    /// the established transaction
    AwaitResponse,
}

/// Future object to establish a connection to a remote node.
/// Only establishing the low level network transport protocol.
///
/// Once this future has successfully returned the expected value it
/// be discarded as any other attempt to poll value from it will
/// result to an error.
///
/// # Errors
///
/// `Future::poll` may failed if the response to the initial handshake
/// handshake did not succeed: the remote server did not accept or
/// recognize the query.
///
pub struct Connecting<T> {
    inner: Option<T>,
    state: ConnectingState,
    handshake: ::std::io::Cursor<Bytes>,
    response: [u8; 4],
    response_read: usize,
}
impl<T> Connecting<T> {
    pub fn new(inner: T) -> Self {
        const HANDSHAKE: [u8; 16] = [0; 16];
        Connecting {
            inner: Some(inner),
            state: ConnectingState::ToSendHandshake,
            handshake: Bytes::from(HANDSHAKE.as_ref()).into_buf(),
            response: [0; 4],
            response_read: 0,
        }
    }
}

impl<T: AsyncRead + AsyncWrite> Future for Connecting<T> {
    type Item = Connection<T>;
    type Error = ConnectingError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.state {
                ConnectingState::ToSendHandshake => {
                    if let Some(ref mut inner) = &mut self.inner {
                        debug!(
                            "sending handshake ({} out of {} bytes)",
                            self.handshake.position(),
                            16
                        );
                        try_ready!(inner.write_buf(&mut self.handshake));

                        if self.handshake.get_ref().len() == (self.handshake.position() as usize) {
                            debug!("handshake sent, awaiting response");
                            self.state = ConnectingState::AwaitResponse;
                        }
                    } else {
                        return Err(ConnectingError::AlreadyConnected);
                    }
                }
                ConnectingState::AwaitResponse => {
                    let done = if let Some(ref mut inner) = &mut self.inner {
                        let from = self.response_read;
                        let to = 4 - self.response_read;
                        debug!("handshake response ({} out of {} bytes)", from, 4);
                        let read = try_ready!(inner.poll_read(&mut self.response[from..to]));
                        self.response_read += read;

                        if self.response_read == 4 {
                            let mut bytes = Bytes::from(self.response.as_ref()).into_buf();
                            let response = bytes.get_u32_be();
                            debug!("handshake response 0x{:08X}", response);
                            match response.into() {
                                ResponseCode::Success => true,
                                c => return Err(ConnectingError::ConnectionFailed(c)),
                            }
                        } else {
                            false
                        }
                    } else {
                        return Err(ConnectingError::AlreadyConnected);
                    };

                    if done {
                        if let Some(inner) = ::std::mem::replace(&mut self.inner, None) {
                            info!("connection initialized");
                            return Ok(Async::Ready(Connection(Framed::new(
                                inner,
                                event::EventCodec,
                            ))));
                        } else {
                            unreachable!() /* `self.inner` is already guaranteed to be `Some(inner)` here */
                        }
                    }
                }
            }
        }
    }
}

/// Error that may happen while establishing the connection to a
/// remote NT
#[derive(Debug)]
pub enum ConnectingError {
    /// this is in case the underlying operation reported an error
    /// (it is required by the AsyncRead/AsyncWrite dependency).
    IoError(::std::io::Error),

    /// the connection failed
    ConnectionFailed(ResponseCode),

    /// the connecting opbject should have not been reused because
    /// the connection has already been established
    AlreadyConnected,
}
impl From<::std::io::Error> for ConnectingError {
    fn from(e: ::std::io::Error) -> Self {
        ConnectingError::IoError(e)
    }
}
impl fmt::Display for ConnectingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnectingError::IoError(_) => write!(f, "I/O Error"),
            ConnectingError::ConnectionFailed(_) => write!(f, "Cannot establish connection"),
            ConnectingError::AlreadyConnected => write!(
                f,
                "The connecting object was already connected and should have not been reused"
            ),
        }
    }
}
impl error::Error for ConnectingError {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            ConnectingError::IoError(ref err) => Some(err),
            ConnectingError::ConnectionFailed(ref err) => Some(err),
            ConnectingError::AlreadyConnected => None,
        }
    }
}
