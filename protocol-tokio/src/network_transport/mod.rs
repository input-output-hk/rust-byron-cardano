mod accepting;
mod closing;
mod connecting;
mod event;
mod response_code;

use futures::{Poll, Sink, StartSend, Stream};
use std::io;
use tokio_codec::Framed;
use tokio_io::{AsyncRead, AsyncWrite};

pub use self::accepting::{Accepting, AcceptingError};
pub use self::closing::{Closing, ClosingError};
pub use self::connecting::{Connecting, ConnectingError};
pub use self::event::{ControlHeader, DecodeEventError, Event, LightWeightConnectionId};
pub use self::response_code::ResponseCode;

/// Network Transport connection where we can accept Event
/// or send events too
///
#[derive(Debug)]
pub struct Connection<T>(Framed<T, event::EventCodec>);
impl<T: AsyncRead + AsyncWrite> Connection<T> {
    /// take ownsership of the given `T` and start to establish a connection
    pub fn connect(inner: T) -> Connecting<T> {
        Connecting::new(inner)
    }

    /// from a server side point of view: accept an inbound connection
    pub fn accept(inner: T) -> Accepting<T> {
        Accepting::new(inner)
    }

    pub fn close(self) -> Closing<T> {
        Closing::new(self.0.into_inner())
    }
}

impl<T: AsyncRead> Stream for Connection<T> {
    type Item = Event;
    type Error = DecodeEventError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.0.poll()
    }
}
impl<T: AsyncWrite> Sink for Connection<T> {
    type SinkItem = Event;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.0.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.0.poll_complete()
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.0.close()
    }
}
