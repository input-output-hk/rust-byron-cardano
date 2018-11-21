use std::{error, fmt};

use bytes::{Buf, Bytes, IntoBuf};
use futures::{Async, Future, Poll};
use tokio_codec::Framed;
use tokio_io::{AsyncRead, AsyncWrite};

use super::{event, Connection, ResponseCode};

/// the accepting states
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum AcceptingState {
    AwaitHandshake,
    SendingResponse(bool),
}

/// Future object to establish a connection to a remote client.
/// Only establishing the low level network transport protocol.
///
/// Once this future has successfuly returned the expected value it
/// be discarded as any other attempt to poll value from it will
/// result to an error.
///
/// # Errors
///
/// `Future::poll` may failed if the response to the initial handshake
/// handshake did not succeed: the remote client did sent an invalid
/// or incompatible handshake.
///
pub struct Accepting<T> {
    inner: Option<T>,
    state: AcceptingState,
    handshake: [u8; 16],
    handshake_read: usize,
    response: ::std::io::Cursor<Bytes>,
}
impl<T> Accepting<T> {
    pub fn new(inner: T) -> Self {
        Accepting {
            inner: Some(inner),
            state: AcceptingState::AwaitHandshake,
            handshake: [0u8; 16],
            handshake_read: 0,
            response: Bytes::new().into_buf(),
        }
    }
}

impl<T: AsyncRead + AsyncWrite> Future for Accepting<T> {
    type Item = Connection<T>;
    type Error = AcceptingError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.state {
                AcceptingState::AwaitHandshake => {
                    if let Some(ref mut inner) = &mut self.inner {
                        let from = self.handshake_read;
                        let to = 16 - self.handshake_read;
                        debug!("handshake query ({} out of {} bytes)", from, 16);
                        let read = try_ready!(inner.poll_read(&mut self.handshake[from..to]));
                        self.handshake_read += read;

                        if self.handshake_read == 16 {
                            let mut bytes = Bytes::from(self.handshake.as_ref()).into_buf();
                            let version = bytes.get_u32_be();
                            let stuff1 = bytes.get_u32_be();
                            let stuff2 = bytes.get_u32_be();
                            let stuff3 = bytes.get_u32_be();
                            debug!("handshake version 0x{:08X}", version);
                            debug!("handshake field1  0x{:08X}", stuff1);
                            debug!("handshake field2  0x{:08X}", stuff2);
                            debug!("handshake field3  0x{:08X}", stuff3);
                            if version == 0x00000000 {
                                self.response = Bytes::from([0; 4].as_ref()).into_buf();
                                self.state = AcceptingState::SendingResponse(true);
                            } else {
                                self.response = Bytes::from([0xff; 4].as_ref()).into_buf();
                                self.state = AcceptingState::SendingResponse(false);
                            }
                        }
                    } else {
                        return Err(AcceptingError::AlreadyConnected);
                    };
                }
                AcceptingState::SendingResponse(succeed) => {
                    let done = if let Some(ref mut inner) = &mut self.inner {
                        debug!(
                            "sending response ({} out of {} bytes)",
                            self.response.position(),
                            self.response.get_ref().len()
                        );
                        try_ready!(inner.write_buf(&mut self.response));

                        if self.response.get_ref().len() == (self.response.position() as usize) {
                            debug!("response sent");
                            true
                        } else {
                            false
                        }
                    } else {
                        return Err(AcceptingError::AlreadyConnected);
                    };
                    if done {
                        if succeed {
                            if let Some(inner) = ::std::mem::replace(&mut self.inner, None) {
                                info!("connection initialized");
                                return Ok(Async::Ready(Connection(Framed::new(
                                    inner,
                                    event::EventCodec,
                                ))));
                            } else {
                                unreachable!() /* `self.inner` is already guaranteed to be `Some(inner)` here */
                            }
                        } else {
                            return Err(AcceptingError::ConnectionFailed(
                                ResponseCode::UnsupportedVersion,
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Error that may happen while establishing the connection to a
/// remove NT
#[derive(Debug)]
pub enum AcceptingError {
    /// this is in case the underlying operation reported an error
    /// (it is required by the AsyncRead/AsyncWrite dependency).
    IoError(::std::io::Error),

    /// the connection failed
    ConnectionFailed(ResponseCode),

    /// the connecting opbject should have not been reused because
    /// the connection has already been established
    AlreadyConnected,
}
impl From<::std::io::Error> for AcceptingError {
    fn from(e: ::std::io::Error) -> Self {
        AcceptingError::IoError(e)
    }
}
impl fmt::Display for AcceptingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AcceptingError::IoError(_) => write!(f, "I/O Error"),
            AcceptingError::ConnectionFailed(_) => write!(f, "Cannot establish connection"),
            AcceptingError::AlreadyConnected => write!(
                f,
                "The connecting object was already connected and should have not been reused"
            ),
        }
    }
}
impl error::Error for AcceptingError {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            AcceptingError::IoError(ref err) => Some(err),
            AcceptingError::ConnectionFailed(ref err) => Some(err),
            AcceptingError::AlreadyConnected => None,
        }
    }
}
