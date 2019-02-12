use bytes::{Buf, IntoBuf};
use futures::{
    sink::SendAll,
    stream::{self, IterOk, StreamFuture},
    Async, Future, Poll, Sink, Stream,
};
use tokio_io::{AsyncRead, AsyncWrite};

use cbor_event::de::Deserializer;
use chain_core::property;
use std::{self, fmt, io::Cursor, vec};

use super::{
    chain_bounds::{ProtocolBlock, ProtocolBlockId, ProtocolHeader, ProtocolTransactionId},
    nt, Connection, Handshake, Message, NodeId,
};

enum AcceptingState<T, B: property::Block, Tx: property::TransactionId> {
    NtAccepting(nt::Accepting<T>),
    ExpectNewLightWeightId(StreamFuture<Connection<T, B, Tx>>),
    ExpectHandshake(StreamFuture<Connection<T, B, Tx>>),
    ExpectNodeId(StreamFuture<Connection<T, B, Tx>>),
    SendHandshake(SendAll<Connection<T, B, Tx>, IterOk<vec::IntoIter<nt::Event>, std::io::Error>>),
    Consumed,
}

enum Transition<T, B: property::Block, Tx: property::TransactionId> {
    Connected(Connection<T, B, Tx>),
    ReceivedNewLightWeightId(Connection<T, B, Tx>),
    ReceivedHandshake(Connection<T, B, Tx>),
    ReceivedNodeId(Connection<T, B, Tx>),
    HandshakeSent(Connection<T, B, Tx>),
}

pub struct Accepting<T, B: property::Block, Tx: property::TransactionId> {
    state: AcceptingState<T, B, Tx>,
}

impl<T: AsyncRead + AsyncWrite, B: property::Block, Tx: property::TransactionId>
    Accepting<T, B, Tx>
{
    pub fn new(inner: T) -> Self {
        Accepting {
            state: AcceptingState::NtAccepting(nt::Connection::accept(inner)),
        }
    }
}

impl<T, B, Tx> Future for Accepting<T, B, Tx>
where
    T: AsyncRead + AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    type Item = Connection<T, B, Tx>;
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
                    let msg1: Message<B, Tx> = Message::CreateLightWeightConnectionId(lid);
                    let msg2: Message<B, Tx> =
                        Message::Bytes(lid, cbor!(Handshake::default()).unwrap().into());
                    let msg3: Message<B, Tx> = Message::CreateNodeId(lid, nid);
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
impl std::error::Error for AcceptingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AcceptingError::IoError(e) => Some(e),
            AcceptingError::NtError(e) => Some(e),
            AcceptingError::EventDecodeError(e) => Some(e),
            AcceptingError::InvalidHandshake(e) => Some(e),
            _ => None,
        }
    }
}
impl fmt::Display for AcceptingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AcceptingError::NtError(_) => write!(f, "network transport error"),
            AcceptingError::IoError(_) => write!(f, "I/O error"),
            AcceptingError::EventDecodeError(_) => write!(f, "failed to decode event message"),
            AcceptingError::ConnectionClosed => write!(f, "connection closed"),
            AcceptingError::ExpectedNewLightWeightConnectionId => {
                write!(f, "expected new lightweight connection id")
            }
            AcceptingError::ExpectedHandshake => write!(f, "expected handshake"),
            AcceptingError::InvalidHandshake(_) => write!(f, "invalid handshake"),
            AcceptingError::ExpectedNodeId => write!(f, "expected node id"),
            AcceptingError::AlreadyConnected => write!(f, "already connected"),
        }
    }
}
