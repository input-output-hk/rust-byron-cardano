mod connecting;
mod accepting;
mod codec;

use super::network_transport as nt;

use tokio::prelude::{*};
use futures::{StartSend, Poll};

pub use self::connecting::{Connecting, ConnectingError};
pub use self::accepting::{Accepting, AcceptingError};
pub use self::codec::{*};

pub struct Connection<T> {
    connection: nt::Connection<T>,

    next_lightweight_connection_id: nt::LightWeightConnectionId,

    next_node_id: NodeId,
}

impl<T: AsyncRead+AsyncWrite> Connection<T> {
    fn new(connection: nt::Connection<T>) -> Self {
        Connection {
            connection: connection,

            next_lightweight_connection_id: nt::LightWeightConnectionId::first_non_reserved(),
            next_node_id: NodeId::default(),

        }
    }

    pub fn connect(inner: T) -> Connecting<T> { Connecting::new(inner) }

    pub fn accept(inner: T) -> Accepting<T> { Accepting::new(inner) }
}

impl<T: AsyncRead> Stream for Connection<T> {
    type Item = nt::Event;
    type Error = nt::DecodeEventError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.connection.poll()
    }
}
impl<T: AsyncWrite> Sink for Connection<T> {
    type SinkItem = nt::Event;
    type SinkError = tokio::io::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError>
    {
        self.connection.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.connection.poll_complete()
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.connection.close()
    }
}
