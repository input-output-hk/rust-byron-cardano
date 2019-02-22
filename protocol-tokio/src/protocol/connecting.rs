use std::{io::Cursor, vec};

use chain_core::property;

use bytes::{Buf, IntoBuf};
use futures::{
    sink::SendAll,
    stream::{self, IterOk, StreamFuture},
    Async, Future, Poll, Sink, Stream,
};
use tokio_io::{AsyncRead, AsyncWrite};

use cbor_event::{self, de::Deserializer};

use super::{
    chain_bounds::{ProtocolBlock, ProtocolBlockId, ProtocolHeader, ProtocolTransactionId},
    nt, Connection, Handshake, Message, NodeId, ProtocolMagic,
};

use std::fmt;

enum ConnectingState<T, B: property::Block, Tx: property::TransactionId> {
    NtConnecting(nt::Connecting<T>, ProtocolMagic),
    SendHandshake(
        SendAll<Connection<T, B, Tx>, IterOk<vec::IntoIter<nt::Event>, ::std::io::Error>>,
    ),
    ExpectNewLightWeightId(StreamFuture<Connection<T, B, Tx>>),
    ExpectHandshake(StreamFuture<Connection<T, B, Tx>>),
    ExpectNodeId(StreamFuture<Connection<T, B, Tx>>),
    Consumed,
}

enum Transition<T, B: property::Block, Tx: property::TransactionId> {
    Connected(Connection<T, B, Tx>, ProtocolMagic),
    HandshakeSent(Connection<T, B, Tx>),
    ReceivedNewLightWeightId(Connection<T, B, Tx>),
    ReceivedHandshake(Connection<T, B, Tx>),
    ReceivedNodeId(Connection<T, B, Tx>),
}

pub struct Connecting<T, B: property::Block, Tx: property::TransactionId> {
    state: ConnectingState<T, B, Tx>,
}

impl<T: AsyncRead + AsyncWrite, B: property::Block, Tx: property::TransactionId>
    Connecting<T, B, Tx>
{
    pub fn new(inner: T, magic: ProtocolMagic) -> Self {
        Connecting {
            state: ConnectingState::NtConnecting(nt::Connection::connect(inner), magic),
        }
    }
}

impl<T, B, Tx> Future for Connecting<T, B, Tx>
where
    T: AsyncRead + AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as property::Block>::Id: ProtocolBlockId,
    <B as property::HasHeader>::Header: ProtocolHeader,
{
    type Item = Connection<T, B, Tx>;
    type Error = ConnectingError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let connection = match &mut self.state {
                ConnectingState::Consumed => {
                    return Err(ConnectingError::AlreadyConnected);
                }
                ConnectingState::NtConnecting(ref mut nt, magic) => {
                    let nt = try_ready!(nt.poll());
                    Transition::Connected(Connection::new(nt), *magic)
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
                Transition::Connected(mut connection, magic) => {
                    let lid = connection.get_next_light_id();
                    let nid = connection.get_next_node_id();
                    let msg1: Message<B, Tx> = Message::CreateLightWeightConnectionId(lid);
                    let msg2: Message<B, Tx> =
                        Message::Bytes(lid, cbor!(Handshake::default_with(magic)).unwrap().into());
                    let msg3: Message<B, Tx> = Message::CreateNodeId(lid, nid);
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
impl std::error::Error for ConnectingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConnectingError::NtError(e) => Some(e),
            ConnectingError::IoError(e) => Some(e),
            ConnectingError::EventDecodeError(e) => Some(e),
            ConnectingError::InvalidHandshake(e) => Some(e),
            _ => None,
        }
    }
}
impl fmt::Display for ConnectingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnectingError::NtError(_) => write!(f, "network transport error"),
            ConnectingError::IoError(_) => write!(f, "I/O error"),
            ConnectingError::EventDecodeError(_) => write!(f, "event decode error"),
            ConnectingError::ConnectionClosed => write!(f, "connection closed"),
            ConnectingError::ExpectedNewLightWeightConnectionId => {
                write!(f, "expected new lightweight connection id")
            }
            ConnectingError::ExpectedHandshake => write!(f, "expected handshake"),
            ConnectingError::InvalidHandshake(_) => write!(f, "invalid handshake"),
            ConnectingError::ExpectedNodeId => write!(f, "expected node id"),
            ConnectingError::AlreadyConnected => write!(f, "already connected"),
        }
    }
}
