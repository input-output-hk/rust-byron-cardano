use std::{vec};

use tokio::prelude::{*};
use futures::{Poll, sink::{SendAll}, stream::{self, IterOk, StreamFuture}};
use bytes::{IntoBuf, Buf};

use cbor_event::{self, de::RawCbor};

use super::{nt, Connection, Message, Handshake, NodeId};

enum ConnectingState<T> {
    NtConnecting(nt::Connecting<T>),
    SendHandshake(SendAll<Connection<T>, IterOk<vec::IntoIter<nt::Event>, ::std::io::Error>>),
    ExpectNewLightWeightId(StreamFuture<Connection<T>>),
    ExpectHandshake(StreamFuture<Connection<T>>),
    ExpectNodeId(StreamFuture<Connection<T>>),
    Consumed,
}

enum Transition<T> {
    Connected(Connection<T>),
    HandshakeSent(Connection<T>),
    ReceivedNewLightWeightId(Connection<T>),
    ReceivedHandshake(Connection<T>),
    ReceivedNodeId(Connection<T>),
}

pub struct Connecting<T> {
    state: ConnectingState<T>
}

impl<T: AsyncRead+AsyncWrite> Connecting<T> {
    pub fn new(inner: T) -> Self {
        Connecting {
            state: ConnectingState::NtConnecting(nt::Connection::connect(inner)),
        }
    }
}

impl<T: AsyncRead+AsyncWrite> Future for Connecting<T> {
    type Item = Connection<T>;
    type Error = ConnectingError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let connection = match &mut self.state {
                ConnectingState::Consumed => {
                    return Err(ConnectingError::AlreadyConnected);
                },
                ConnectingState::NtConnecting(ref mut nt) => {
                    let nt = try_ready!(nt.poll());
                    Transition::Connected(Connection::new(nt))
                },
                ConnectingState::SendHandshake(ref mut send_all) => {
                    let (connection, _) = try_ready!(send_all.poll());
                    debug!("Handshake sent");
                    Transition::HandshakeSent(connection)
                }
                ConnectingState::ExpectNewLightWeightId(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let lwcid = match e {
                        None => { return Err(ConnectingError::ConnectionClosed) },
                        Some(e) => {
                            if let Ok((nt::ControlHeader::CreateNewConnection, lwcid)) = e.expect_control() {
                                lwcid
                            } else { return Err(ConnectingError::ExpectedNewLightWeightConnectionId) }
                        }
                    };
                    debug!("peer created LightWeightConnectionId {:?}", lwcid);
                    Transition::ReceivedNewLightWeightId(connection)
                },
                ConnectingState::ExpectHandshake(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let (lwcid, peer_handshake) = match e {
                        None => { return Err(ConnectingError::ConnectionClosed) },
                        Some(e) => {
                            if let Ok((lwcid, bytes)) = e.expect_data() {
                                let bytes : Vec<_> = bytes.into_iter().collect();
                                let peer_handshake : Handshake = RawCbor::from(&bytes)
                                    .deserialize().map_err(ConnectingError::InvalidHandshake)?;
                                (lwcid, peer_handshake)
                            } else { return Err(ConnectingError::ExpectedHandshake) }
                        }
                    };
                    debug!("peer sent handshake {:?} {:#?}", lwcid, peer_handshake);
                    Transition::ReceivedHandshake(connection)
                },
                ConnectingState::ExpectNodeId(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let (lwcid, ack, node_id) = match e {
                        None => { return Err(ConnectingError::ConnectionClosed) },
                        Some(e) => {
                            if let Ok((lwcid, bytes)) = e.expect_data() {
                                let mut bytes = bytes.into_buf();
                                let ack = bytes.get_u8();
                                let node_id : NodeId = bytes.get_u64_be().into();
                                (lwcid, ack, node_id)
                            } else { return Err(ConnectingError::ExpectedNodeId) }
                        }
                    };
                    debug!("peer sent new node {:?} 0x{:x} {:?}", lwcid, ack, node_id);
                    Transition::ReceivedNodeId(connection)
                },
            };

            match connection {
                Transition::Connected(mut connection) => {
                    let lid = connection.next_lightweight_connection_id.next();
                    let nid = connection.next_node_id.next();
                    let commands = stream::iter_ok::<_, ::std::io::Error>(vec![
                        Message::CreateLightWeightConnectionId(lid).to_nt_event(),
                        Message::Handshake(lid, Handshake::default()).to_nt_event(),
                        Message::CreateNodeId(lid, nid).to_nt_event(),
                    ]);
                    let send_all = connection.send_all(commands);
                    self.state = ConnectingState::SendHandshake(send_all);
                },
                Transition::HandshakeSent(nt) => {
                    self.state = ConnectingState::ExpectNewLightWeightId(nt.into_future());
                },
                Transition::ReceivedNewLightWeightId(connection) => {
                    self.state = ConnectingState::ExpectHandshake(connection.into_future());
                },
                Transition::ReceivedHandshake(connection) => {
                    self.state = ConnectingState::ExpectNodeId(connection.into_future());
                },
                Transition::ReceivedNodeId(connection) => {
                    self.state = ConnectingState::Consumed;
                    return Ok(Async::Ready(connection));
                },
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
    fn from(e: ::std::io::Error) -> Self { ConnectingError::IoError(e) }
}
impl From<nt::ConnectingError> for ConnectingError {
    fn from(e: nt::ConnectingError) -> Self { ConnectingError::NtError(e) }
}
impl From<nt::DecodeEventError> for ConnectingError {
    fn from(e: nt::DecodeEventError) -> Self { ConnectingError::EventDecodeError(e) }
}
