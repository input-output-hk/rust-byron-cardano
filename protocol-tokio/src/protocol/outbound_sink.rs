use super::{
    chain_bounds::{ProtocolBlock, ProtocolBlockId, ProtocolHeader, ProtocolTransactionId},
    nt, ConnectionState, KeepAlive, Message, NodeId,
};

use chain_core::property;

use futures::prelude::*;
use futures::{sink, stream::SplitSink};
use tokio_io::AsyncWrite;

use std::{
    error, fmt, io,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

pub type Outbound<B, Tx> = Message<B, Tx>;

#[derive(Debug)]
pub enum OutboundError {
    IoError(io::Error),
    Unknown,
}
impl From<()> for OutboundError {
    fn from(_: ()) -> Self {
        OutboundError::Unknown
    }
}
impl From<io::Error> for OutboundError {
    fn from(e: io::Error) -> Self {
        OutboundError::IoError(e)
    }
}

impl fmt::Display for OutboundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use OutboundError::*;

        match self {
            IoError(_) => write!(f, "I/O error"),
            Unknown => write!(f, "unknown error"),
        }
    }
}

impl error::Error for OutboundError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use OutboundError::*;

        match self {
            IoError(e) => Some(e),
            Unknown => None,
        }
    }
}

pub struct OutboundSink<T, B, Tx> {
    sink: SplitSink<nt::Connection<T>>,
    state: Arc<Mutex<ConnectionState>>,
    phantoms: PhantomData<(B, Tx)>,
}

impl<T, B, Tx> OutboundSink<T, B, Tx> {
    fn get_next_light_id(&mut self) -> nt::LightWeightConnectionId {
        self.state.lock().unwrap().get_next_light_id()
    }

    fn get_next_node_id(&mut self) -> NodeId {
        self.state.lock().unwrap().get_next_node_id()
    }
}

impl<T, B, Tx> OutboundSink<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    pub fn new(sink: SplitSink<nt::Connection<T>>, state: Arc<Mutex<ConnectionState>>) -> Self {
        OutboundSink {
            sink,
            state,
            phantoms: PhantomData,
        }
    }

    /// create a new light weight connection with the remote peer
    ///
    pub fn new_light_connection(mut self) -> NewLightConnection<T, B, Tx> {
        let lwcid = self.get_next_light_id();
        let node_id = self.get_next_node_id();
        NewLightConnection::new(self, lwcid, node_id)
    }

    /// initialize a subscription from the given outbound halve.
    pub fn subscribe(
        self,
        keep_alive: KeepAlive,
    ) -> impl Future<Item = (nt::LightWeightConnectionId, Self), Error = OutboundError> {
        self.new_light_connection()
            .and_then(move |(lwcid, connection)| {
                connection
                    .send(Message::Subscribe(lwcid, keep_alive))
                    .map(move |connection| (lwcid, connection))
            })
    }

    /// close a light connection that has been created with
    /// `new_light_connection`.
    ///
    pub fn close_light_connection(
        self,
        lwcid: nt::LightWeightConnectionId,
    ) -> CloseLightConnection<T, B, Tx> {
        CloseLightConnection::new(self, lwcid)
    }

    /// this function it to acknowledge the creation of the NodeId on the remote
    /// client side
    pub fn ack_node_id(
        mut self,
        node_id: NodeId,
    ) -> impl Future<Item = Self, Error = OutboundError> {
        let our_lwcid = self.get_next_light_id();

        self.send(Message::CreateLightWeightConnectionId(our_lwcid))
            .and_then(move |connection| connection.send(Message::AckNodeId(our_lwcid, node_id)))
            .map(move |connection| {
                // here we need to wire the acknowledged NodeId to our new created client LWCID
                connection
                    .state
                    .lock()
                    .unwrap()
                    .map_to_client
                    .insert(node_id, our_lwcid);
                connection
            })
    }
}

impl<T, B, Tx> Sink for OutboundSink<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    type SinkItem = Outbound<B, Tx>;
    type SinkError = OutboundError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.sink
            .start_send(item.to_nt_event())
            .map_err(OutboundError::IoError)
            .map(|async| async.map(Message::from_nt_event))
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.poll_complete().map_err(OutboundError::IoError)
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.close().map_err(OutboundError::IoError)
    }
}

pub struct NewLightConnection<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    lwcid: nt::LightWeightConnectionId,
    node_id: NodeId,
    state: CreationState<T, B, Tx>,
}

impl<T, B, Tx> NewLightConnection<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    fn new(
        sink: OutboundSink<T, B, Tx>,
        lwcid: nt::LightWeightConnectionId,
        node_id: NodeId,
    ) -> Self {
        let send = sink.send(Message::CreateLightWeightConnectionId(lwcid));
        let state = CreationState::CreatingConnectionId(send);
        NewLightConnection {
            lwcid,
            node_id,
            state,
        }
    }

    pub fn connection_id(&self) -> nt::LightWeightConnectionId {
        self.lwcid
    }

    pub fn get_mut(&mut self) -> &mut OutboundSink<T, B, Tx> {
        use self::CreationState::*;
        match &mut self.state {
            CreatingConnectionId(send) => send.get_mut(),
            CreatingNodeId(send) => send.get_mut(),
        }
    }
}

enum CreationState<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    CreatingConnectionId(sink::Send<OutboundSink<T, B, Tx>>),
    CreatingNodeId(sink::Send<OutboundSink<T, B, Tx>>),
}

impl<T, B, Tx> Future for NewLightConnection<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    type Item = (nt::LightWeightConnectionId, OutboundSink<T, B, Tx>);
    type Error = OutboundError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let new_state = match self.state {
                CreationState::CreatingConnectionId(ref mut send) => {
                    let sink = try_ready!(send.poll());
                    let send = sink.send(Message::CreateNodeId(self.lwcid, self.node_id));
                    CreationState::CreatingNodeId(send)
                }
                CreationState::CreatingNodeId(ref mut send) => {
                    let sink = try_ready!(send.poll());
                    // Here we need to wire the acknowledged NodeId to our new created client LWCID
                    sink.state
                        .lock()
                        .unwrap()
                        .map_to_client
                        .insert(self.node_id, self.lwcid);
                    return Ok(Async::Ready((self.lwcid, sink)));
                }
            };
            self.state = new_state;
        }
    }
}

pub struct CloseLightConnection<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    lwcid: nt::LightWeightConnectionId,
    send: sink::Send<OutboundSink<T, B, Tx>>,
}

impl<T, B, Tx> CloseLightConnection<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    fn new(sink: OutboundSink<T, B, Tx>, lwcid: nt::LightWeightConnectionId) -> Self {
        let send = sink.send(Message::CloseConnection(lwcid));
        CloseLightConnection { lwcid, send }
    }

    pub fn get_mut(&mut self) -> &mut OutboundSink<T, B, Tx> {
        self.send.get_mut()
    }
}

impl<T, B, Tx> Future for CloseLightConnection<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    type Item = OutboundSink<T, B, Tx>;
    type Error = OutboundError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let sink = try_ready!(self.send.poll());
            // Here we need to wire the acknowledged NodeId to our new created client LWCID
            sink.state
                .lock()
                .unwrap()
                .client_handles
                .remove(&self.lwcid);
            return Ok(Async::Ready(sink));
        }
    }
}
