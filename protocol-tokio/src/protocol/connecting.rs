use std::{io::Cursor, vec};

use bytes::{Buf, IntoBuf};
use futures::{
    sink::SendAll,
    stream::{self, IterOk, StreamFuture},
    Async, Future, Poll, Sink, Stream,
};
use tokio_io::{AsyncRead, AsyncWrite};

use cbor_event::{self, de::Deserializer};

use super::{nt, Connection, Handshake, Message, NodeId};

enum ConnectingState<T, Header, BlockId, Block, TransactionId> {
    NtConnecting(nt::Connecting<T>),
    SendHandshake(
        SendAll<
            Connection<T, Header, BlockId, Block, TransactionId>,
            IterOk<vec::IntoIter<nt::Event>, ::std::io::Error>,
        >,
    ),
    ExpectNewLightWeightId(StreamFuture<Connection<T, Header, BlockId, Block, TransactionId>>),
    ExpectHandshake(StreamFuture<Connection<T, Header, BlockId, Block, TransactionId>>),
    ExpectNodeId(StreamFuture<Connection<T, Header, BlockId, Block, TransactionId>>),
    Consumed,
}

enum Transition<T, Header, BlockId, Block, TransactionId> {
    Connected(Connection<T, Header, BlockId, Block, TransactionId>),
    HandshakeSent(Connection<T, Header, BlockId, Block, TransactionId>),
    ReceivedNewLightWeightId(Connection<T, Header, BlockId, Block, TransactionId>),
    ReceivedHandshake(Connection<T, Header, BlockId, Block, TransactionId>),
    ReceivedNodeId(Connection<T, Header, BlockId, Block, TransactionId>),
}

pub struct Connecting<T, Header, BlockId, Block, TransactionId> {
    state: ConnectingState<T, Header, BlockId, Block, TransactionId>,
}

impl<T: AsyncRead + AsyncWrite, Header, BlockId, Block, TransactionId>
    Connecting<T, Header, BlockId, Block, TransactionId>
{
    pub fn new(inner: T) -> Self {
        Connecting {
            state: ConnectingState::NtConnecting(nt::Connection::connect(inner)),
        }
    }
}

impl<T: AsyncRead + AsyncWrite, Header, BlockId, Block, TransactionId> Future
    for Connecting<T, Header, BlockId, Block, TransactionId>
where
    Block: cbor_event::Deserialize,
    Block: cbor_event::Serialize,
    BlockId: cbor_event::Deserialize,
    BlockId: cbor_event::Serialize,
    Header: cbor_event::Deserialize,
    Header: cbor_event::Serialize,
    TransactionId: cbor_event::Deserialize,
    TransactionId: cbor_event::Serialize,
{
    type Item = Connection<T, Header, BlockId, Block, TransactionId>;
    type Error = ConnectingError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let connection = match &mut self.state {
                ConnectingState::Consumed => {
                    return Err(ConnectingError::AlreadyConnected);
                }
                ConnectingState::NtConnecting(ref mut nt) => {
                    let nt = try_ready!(nt.poll());
                    Transition::Connected(Connection::new(nt))
                }
                ConnectingState::SendHandshake(ref mut send_all) => {
                    let (connection, _) = try_ready!(send_all.poll());
                    debug!("Handshake sent");
                    Transition::HandshakeSent(connection)
                }
                ConnectingState::ExpectNewLightWeightId(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let lwcid = match e {
                        None => return Err(ConnectingError::ConnectionClosed),
                        Some(e) => {
                            if let Ok((nt::ControlHeader::CreateNewConnection, lwcid)) =
                                e.expect_control()
                            {
                                lwcid
                            } else {
                                return Err(ConnectingError::ExpectedNewLightWeightConnectionId);
                            }
                        }
                    };
                    debug!("peer created LightWeightConnectionId {:?}", lwcid);
                    Transition::ReceivedNewLightWeightId(connection)
                }
                ConnectingState::ExpectHandshake(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let (lwcid, peer_handshake) = match e {
                        None => return Err(ConnectingError::ConnectionClosed),
                        Some(e) => {
                            if let Ok((lwcid, bytes)) = e.expect_data() {
                                let bytes: Vec<_> = bytes.into_iter().collect();
                                let mut de = Deserializer::from(Cursor::new(&bytes));
                                let peer_handshake: Handshake = de
                                    .deserialize()
                                    .map_err(ConnectingError::InvalidHandshake)?;
                                (lwcid, peer_handshake)
                            } else {
                                return Err(ConnectingError::ExpectedHandshake);
                            }
                        }
                    };
                    debug!("peer sent handshake {:?} {:#?}", lwcid, peer_handshake);
                    Transition::ReceivedHandshake(connection)
                }
                ConnectingState::ExpectNodeId(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let (lwcid, ack, node_id) = match e {
                        None => return Err(ConnectingError::ConnectionClosed),
                        Some(e) => {
                            if let Ok((lwcid, bytes)) = e.expect_data() {
                                let mut bytes = bytes.into_buf();
                                let ack = bytes.get_u8();
                                let node_id: NodeId = bytes.get_u64_be().into();
                                (lwcid, ack, node_id)
                            } else {
                                return Err(ConnectingError::ExpectedNodeId);
                            }
                        }
                    };
                    debug!("peer sent new node {:?} 0x{:x} {:?}", lwcid, ack, node_id);
                    Transition::ReceivedNodeId(connection)
                }
            };

            match connection {
                Transition::Connected(mut connection) => {
                    let lid = connection.get_next_light_id();
                    let nid = connection.get_next_node_id();
                    let msg1 : Message<Header,BlockId,Block,TransactionId> =
                        Message::CreateLightWeightConnectionId(lid);
                    let msg2 : Message<Header,BlockId,Block,TransactionId> =
                        Message::Bytes(
                            lid,
                            cbor!(Handshake::default()).unwrap().into(),
                        );
                    let msg3 : Message<Header,BlockId,Block,TransactionId> =
                        Message::CreateNodeId(lid, nid);
                    let commands = stream::iter_ok::<_, ::std::io::Error>(vec![
                        msg1.to_nt_event(),
                        msg2.to_nt_event(),
                        msg3.to_nt_event(),
                    ]);
                    let send_all = connection.send_all(commands);
                    self.state = ConnectingState::SendHandshake(send_all);
                }
                Transition::HandshakeSent(nt) => {
                    self.state = ConnectingState::ExpectNewLightWeightId(nt.into_future());
                }
                Transition::ReceivedNewLightWeightId(connection) => {
                    self.state = ConnectingState::ExpectHandshake(connection.into_future());
                }
                Transition::ReceivedHandshake(connection) => {
                    self.state = ConnectingState::ExpectNodeId(connection.into_future());
                }
                Transition::ReceivedNodeId(connection) => {
                    self.state = ConnectingState::Consumed;
                    return Ok(Async::Ready(connection));
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ConnectingError {
    NtError(nt::ConnectingError),
    IoError(::std::io::Error),
    EventDecodeError(nt::DecodeEventError),
    ConnectionClosed,
    ExpectedNewLightWeightConnectionId,
    ExpectedHandshake,
    InvalidHandshake(cbor_event::Error),
    ExpectedNodeId,
    AlreadyConnected,
}
impl From<::std::io::Error> for ConnectingError {
    fn from(e: ::std::io::Error) -> Self {
        ConnectingError::IoError(e)
    }
}
impl From<nt::ConnectingError> for ConnectingError {
    fn from(e: nt::ConnectingError) -> Self {
        ConnectingError::NtError(e)
    }
}
impl From<nt::DecodeEventError> for ConnectingError {
    fn from(e: nt::DecodeEventError) -> Self {
        ConnectingError::EventDecodeError(e)
    }
}
