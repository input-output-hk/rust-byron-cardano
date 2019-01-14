mod accepting;
mod codec;
mod connecting;
mod inbound_stream;
mod outbound_sink;

use super::network_transport as nt;

use futures::{Poll, Sink, StartSend, Stream};
use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
};
use tokio_io::{AsyncRead, AsyncWrite};

pub use self::accepting::{Accepting, AcceptingError};
pub use self::codec::{
    BlockHeaders, GetBlockHeaders, GetBlocks, HandlerSpec, HandlerSpecs, Handshake,
    KeepAlive, Message, MessageType, NodeId, Response,
};
pub use self::connecting::{Connecting, ConnectingError};
pub use self::inbound_stream::{Inbound, InboundError, InboundStream};
pub use self::outbound_sink::{Outbound, OutboundError, OutboundSink};
use std::marker::PhantomData;

/// the connection state, shared between the `ConnectionStream` and the `ConnectionSink`.
///
pub struct ConnectionState {
    /// this is the global state of the Light Connection Identifier
    ///
    /// It always point to the _next_ available identifier.
    next_lightweight_connection_id: nt::LightWeightConnectionId,

    /// this is the next available NodeId. A NodeId is a value created
    /// by a client `Handle`.
    next_node_id: NodeId,

    /// these are the Connection Id created by the remote
    server_handles: BTreeMap<nt::LightWeightConnectionId, LightWeightConnectionState>,
    /// these are the connection Id created by this connection
    client_handles: BTreeMap<nt::LightWeightConnectionId, LightWeightConnectionState>,
    /// this is a map between our NodeId and our Light Connection Id
    map_to_client: BTreeMap<NodeId, nt::LightWeightConnectionId>,
}
impl ConnectionState {
    fn new() -> Self {
        ConnectionState {
            next_lightweight_connection_id: nt::LightWeightConnectionId::first_non_reserved(),
            next_node_id: NodeId::default(),
            server_handles: BTreeMap::new(),
            client_handles: BTreeMap::new(),
            map_to_client: BTreeMap::new(),
        }
    }

    fn get_next_light_id(&mut self) -> nt::LightWeightConnectionId {
        self.next_lightweight_connection_id.next()
    }
    fn get_next_node_id(&mut self) -> NodeId {
        self.next_node_id.next()
    }
}

/// this is the connection to establish or listen from
///
/// Once established call `split` to get the inbound stream
/// and the outbound sink and starts processing queries
pub struct Connection<T, Header, BlockId, Block, TransactionId> {
    connection: nt::Connection<T>,
    state: Arc<Mutex<ConnectionState>>,
    phantoms: PhantomData<(Header, BlockId, Block, TransactionId)>,
}

impl<T: AsyncRead + AsyncWrite, Header, BlockId, Block, TransactionId>
    Connection<T, Header, BlockId, Block, TransactionId>
{
    fn new(connection: nt::Connection<T>) -> Self {
        Connection {
            connection: connection,
            state: Arc::new(Mutex::new(ConnectionState::new())),
            phantoms: PhantomData,
        }
    }

    fn get_next_light_id(&mut self) -> nt::LightWeightConnectionId {
        self.state.lock().unwrap().get_next_light_id()
    }

    fn get_next_node_id(&mut self) -> NodeId {
        self.state.lock().unwrap().get_next_node_id()
    }

    /// this function is to use when establishing a connection with
    /// with a remote.
    pub fn connect(inner: T) -> Connecting<T, Header, BlockId, Block, TransactionId> {
        Connecting::new(inner)
    }

    /// this function is to use when receiving inbound connection
    pub fn accept(inner: T) -> Accepting<T, Header, BlockId, Block, TransactionId> {
        Accepting::new(inner)
    }

    pub fn split(
        self,
    ) -> (
        OutboundSink<T, Header, BlockId, Block, TransactionId>,
        InboundStream<T, Header, BlockId, Block, TransactionId>,
    )
    where
        Header: cbor_event::Serialize,
        Header: cbor_event::Deserialize,
        BlockId: cbor_event::Serialize,
        BlockId: cbor_event::Deserialize,
        Block: cbor_event::Serialize,
        Block: cbor_event::Deserialize,
        TransactionId: cbor_event::Serialize,
        TransactionId: cbor_event::Deserialize,
    {
        let state = self.state;
        let (sink, stream) = self.connection.split();

        (
            OutboundSink::new(sink, state.clone()),
            InboundStream::new(stream, state),
        )
    }
}

impl<T: AsyncRead, Header, BlockId, Block, TransactionId> Stream
    for Connection<T, Header, BlockId, Block, TransactionId>
{
    type Item = nt::Event;
    type Error = nt::DecodeEventError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.connection.poll()
    }
}
impl<T: AsyncWrite, Header, BlockId, Block, TransactionId> Sink
    for Connection<T, Header, BlockId, Block, TransactionId>
{
    type SinkItem = nt::Event;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.connection.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.connection.poll_complete()
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.connection.close()
    }
}

#[derive(Clone, Copy, Debug)]
struct LightWeightConnectionState {
    id: nt::LightWeightConnectionId,
    node: Option<NodeId>,
    remote_initiated: bool,
    remote_close: bool,
}
impl LightWeightConnectionState {
    fn new(id: nt::LightWeightConnectionId) -> Self {
        LightWeightConnectionState {
            id: id,
            node: None,
            remote_initiated: false,
            remote_close: false,
        }
    }

    fn with_node_id(mut self, node_id: NodeId) -> Self {
        self.node = Some(node_id);
        self
    }

    fn remote_initiated(mut self, remote_initiated: bool) -> Self {
        self.remote_initiated = remote_initiated;
        self
    }
}
