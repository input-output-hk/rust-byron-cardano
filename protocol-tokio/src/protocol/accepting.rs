use bytes::{Buf, IntoBuf};
use futures::{
    sink::SendAll,
    stream::{self, IterOk, StreamFuture},
    Async, Future, Poll, Sink, Stream,
};
use tokio_io::{AsyncRead, AsyncWrite};

use cbor_event::de::Deserializer;
use std::{self, io::Cursor, vec};

use super::{nt, Connection, Handshake, Message, NodeId};

enum AcceptingState<T, Header, BlockId, Block, TransactionId> {
    NtAccepting(nt::Accepting<T>),
    ExpectNewLightWeightId(StreamFuture<Connection<T, Header, BlockId, Block, TransactionId>>),
    ExpectHandshake(StreamFuture<Connection<T, Header, BlockId, Block, TransactionId>>),
    ExpectNodeId(StreamFuture<Connection<T, Header, BlockId, Block, TransactionId>>),
    SendHandshake(
        SendAll<
            Connection<T, Header, BlockId, Block, TransactionId>,
            IterOk<vec::IntoIter<nt::Event>, std::io::Error>,
        >,
    ),
    Consumed,
}

enum Transition<T, Header, BlockId, Block, TransactionId> {
    Connected(Connection<T, Header, BlockId, Block, TransactionId>),
    ReceivedNewLightWeightId(Connection<T, Header, BlockId, Block, TransactionId>),
    ReceivedHandshake(Connection<T, Header, BlockId, Block, TransactionId>),
    ReceivedNodeId(Connection<T, Header, BlockId, Block, TransactionId>),
    HandshakeSent(Connection<T, Header, BlockId, Block, TransactionId>),
}

pub struct Accepting<T, Header, BlockId, Block, TransactionId> {
    state: AcceptingState<T, Header, BlockId, Block, TransactionId>,
}

impl<T: AsyncRead + AsyncWrite, Header, BlockId, Block, TransactionId>
    Accepting<T, Header, BlockId, Block, TransactionId>
{
    pub fn new(inner: T) -> Self {
        Accepting {
            state: AcceptingState::NtAccepting(nt::Connection::accept(inner)),
        }
    }
}

impl<T: AsyncRead + AsyncWrite, Header, BlockId, Block, TransactionId> Future
    for Accepting<T, Header, BlockId, Block, TransactionId>
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
    type Error = AcceptingError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let connection = match &mut self.state {
                AcceptingState::Consumed => {
                    return Err(AcceptingError::AlreadyConnected);
                }
                AcceptingState::NtAccepting(ref mut nt) => {
                    let nt = try_ready!(nt.poll());
                    Transition::Connected(Connection::new(nt))
                }
                AcceptingState::ExpectNewLightWeightId(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let lwcid = match e {
                        None => return Err(AcceptingError::ConnectionClosed),
                        Some(e) => {
                            if let Ok((nt::ControlHeader::CreateNewConnection, lwcid)) =
                                e.expect_control()
                            {
                                lwcid
                            } else {
                                return Err(AcceptingError::ExpectedNewLightWeightConnectionId);
                            }
                        }
                    };
                    debug!("peer created LightWeightConnectionId {:?}", lwcid);
                    Transition::ReceivedNewLightWeightId(connection)
                }
                AcceptingState::ExpectHandshake(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let (lwcid, peer_handshake) = match e {
                        None => return Err(AcceptingError::ConnectionClosed),
                        Some(e) => {
                            if let Ok((lwcid, bytes)) = e.expect_data() {
                                let bytes: Vec<_> = bytes.into_iter().collect();
                                let mut de = Deserializer::from(Cursor::new(&bytes));
                                let peer_handshake: Handshake =
                                    de.deserialize().map_err(AcceptingError::InvalidHandshake)?;
                                (lwcid, peer_handshake)
                            } else {
                                return Err(AcceptingError::ExpectedHandshake);
                            }
                        }
                    };
                    debug!("peer sent handshake {:?} {:#?}", lwcid, peer_handshake);
                    Transition::ReceivedHandshake(connection)
                }
                AcceptingState::ExpectNodeId(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let (lwcid, ack, node_id) = match e {
                        None => return Err(AcceptingError::ConnectionClosed),
                        Some(e) => {
                            if let Ok((lwcid, bytes)) = e.expect_data() {
                                let mut bytes = bytes.into_buf();
                                let ack = bytes.get_u8();
                                let node_id: NodeId = bytes.get_u64_be().into();
                                (lwcid, ack, node_id)
                            } else {
                                return Err(AcceptingError::ExpectedNodeId);
                            }
                        }
                    };
                    debug!("peer sent new node {:?} 0x{:x} {:?}", lwcid, ack, node_id);
                    Transition::ReceivedNodeId(connection)
                }
                AcceptingState::SendHandshake(ref mut send_all) => {
                    let (connection, _) = try_ready!(send_all.poll());
                    debug!("Handshake sent");
                    Transition::HandshakeSent(connection)
                }
            };

            match connection {
                Transition::Connected(connection) => {
                    self.state = AcceptingState::ExpectNewLightWeightId(connection.into_future());
                }
                Transition::ReceivedNewLightWeightId(connection) => {
                    self.state = AcceptingState::ExpectHandshake(connection.into_future());
                }
                Transition::ReceivedHandshake(connection) => {
                    self.state = AcceptingState::ExpectNodeId(connection.into_future());
                }
                Transition::ReceivedNodeId(mut connection) => {
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
                    let commands = stream::iter_ok::<_, std::io::Error>(vec![
                        msg1.to_nt_event(),
                        msg2.to_nt_event(),
                        msg3.to_nt_event(),

                    ]);
                    let send_all = connection.send_all(commands);
                    self.state = AcceptingState::SendHandshake(send_all);
                }
                Transition::HandshakeSent(connection) => {
                    self.state = AcceptingState::Consumed;
                    return Ok(Async::Ready(connection));
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum AcceptingError {
    NtError(nt::AcceptingError),
    IoError(std::io::Error),
    EventDecodeError(nt::DecodeEventError),
    ConnectionClosed,
    ExpectedNewLightWeightConnectionId,
    ExpectedHandshake,
    InvalidHandshake(cbor_event::Error),
    ExpectedNodeId,
    AlreadyConnected,
}
impl From<std::io::Error> for AcceptingError {
    fn from(e: std::io::Error) -> Self {
        AcceptingError::IoError(e)
    }
}
impl From<nt::AcceptingError> for AcceptingError {
    fn from(e: nt::AcceptingError) -> Self {
        AcceptingError::NtError(e)
    }
}
impl From<nt::DecodeEventError> for AcceptingError {
    fn from(e: nt::DecodeEventError) -> Self {
        AcceptingError::EventDecodeError(e)
    }
}
