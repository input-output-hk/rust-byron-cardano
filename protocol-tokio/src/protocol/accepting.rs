use futures::{Poll, Future, Async, Sink, Stream, sink::{SendAll}, stream::{self, IterOk, StreamFuture}};
use tokio_io::{AsyncRead, AsyncWrite};
use bytes::{IntoBuf, Buf};

use std::{self, vec};
use cbor_event::{self, de::{RawCbor}};

use super::{nt, Connection, Message, Handshake, NodeId};

enum AcceptingState<T> {
    NtAccepting(nt::Accepting<T>),
    ExpectNewLightWeightId(StreamFuture<Connection<T>>),
    ExpectHandshake(StreamFuture<Connection<T>>),
    ExpectNodeId(StreamFuture<Connection<T>>),
    SendHandshake(SendAll<Connection<T>, IterOk<vec::IntoIter<nt::Event>, std::io::Error>>),
    Consumed,
}

enum Transition<T> {
    Connected(Connection<T>),
    ReceivedNewLightWeightId(Connection<T>),
    ReceivedHandshake(Connection<T>),
    ReceivedNodeId(Connection<T>),
    HandshakeSent(Connection<T>),
}

pub struct Accepting<T> {
    state: AcceptingState<T>
}

impl<T: AsyncRead+AsyncWrite> Accepting<T> {
    pub fn new(inner: T) -> Self {
        Accepting {
            state: AcceptingState::NtAccepting(nt::Connection::accept(inner)),
        }
    }
}

impl<T: AsyncRead+AsyncWrite> Future for Accepting<T> {
    type Item = Connection<T>;
    type Error = AcceptingError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let connection = match &mut self.state {
                AcceptingState::Consumed => {
                    return Err(AcceptingError::AlreadyConnected);
                },
                AcceptingState::NtAccepting(ref mut nt) => {
                    let nt = try_ready!(nt.poll());
                    Transition::Connected(Connection::new(nt))
                },
                AcceptingState::ExpectNewLightWeightId(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let lwcid = match e {
                        None => { return Err(AcceptingError::ConnectionClosed) },
                        Some(e) => {
                            if let Ok((nt::ControlHeader::CreateNewConnection, lwcid)) = e.expect_control() {
                                lwcid
                            } else { return Err(AcceptingError::ExpectedNewLightWeightConnectionId) }
                        }
                    };
                    debug!("peer created LightWeightConnectionId {:?}", lwcid);
                    Transition::ReceivedNewLightWeightId(connection)
                },
                AcceptingState::ExpectHandshake(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let (lwcid, peer_handshake) = match e {
                        None => { return Err(AcceptingError::ConnectionClosed) },
                        Some(e) => {
                            if let Ok((lwcid, bytes)) = e.expect_data() {
                                let bytes : Vec<_> = bytes.into_iter().collect();
                                let peer_handshake : Handshake = RawCbor::from(&bytes)
                                    .deserialize().map_err(AcceptingError::InvalidHandshake)?;
                                (lwcid, peer_handshake)
                            } else { return Err(AcceptingError::ExpectedHandshake) }
                        }
                    };
                    debug!("peer sent handshake {:?} {:#?}", lwcid, peer_handshake);
                    Transition::ReceivedHandshake(connection)
                },
                AcceptingState::ExpectNodeId(ref mut connection) => {
                    let (e, connection) = try_ready!(connection.poll().map_err(|(e, _)| e));
                    let (lwcid, ack, node_id) = match e {
                        None => { return Err(AcceptingError::ConnectionClosed) },
                        Some(e) => {
                            if let Ok((lwcid, bytes)) = e.expect_data() {
                                let mut bytes = bytes.into_buf();
                                let ack = bytes.get_u8();
                                let node_id : NodeId = bytes.get_u64_be().into();
                                (lwcid, ack, node_id)
                            } else { return Err(AcceptingError::ExpectedNodeId) }
                        }
                    };
                    debug!("peer sent new node {:?} 0x{:x} {:?}", lwcid, ack, node_id);
                    Transition::ReceivedNodeId(connection)
                },
                AcceptingState::SendHandshake(ref mut send_all) => {
                    let (connection, _) = try_ready!(send_all.poll());
                    debug!("Handshake sent");
                    Transition::HandshakeSent(connection)
                }
            };

            match connection {
                Transition::Connected(connection) => {
                    self.state = AcceptingState::ExpectNewLightWeightId(connection.into_future());
                },
                Transition::ReceivedNewLightWeightId(connection) => {
                    self.state = AcceptingState::ExpectHandshake(connection.into_future());
                },
                Transition::ReceivedHandshake(connection) => {
                    self.state = AcceptingState::ExpectNodeId(connection.into_future());
                },
                Transition::ReceivedNodeId(mut connection) => {
                    let lid = connection.get_next_light_id();
                    let nid = connection.get_next_node_id();
                    let commands = stream::iter_ok::<_, std::io::Error>(vec![
                        Message::CreateLightWeightConnectionId(lid).to_nt_event(),
                        Message::Bytes(lid, cbor!(Handshake::default()).unwrap().into()).to_nt_event(),
                        Message::CreateNodeId(lid, nid).to_nt_event(),
                    ]);
                    let send_all = connection.send_all(commands);
                    self.state = AcceptingState::SendHandshake(send_all);
                },
                Transition::HandshakeSent(connection) => {
                    self.state = AcceptingState::Consumed;
                    return Ok(Async::Ready(connection))
                },
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
    fn from(e: std::io::Error) -> Self { AcceptingError::IoError(e) }
}
impl From<nt::AcceptingError> for AcceptingError {
    fn from(e: nt::AcceptingError) -> Self { AcceptingError::NtError(e) }
}
impl From<nt::DecodeEventError> for AcceptingError {
    fn from(e: nt::DecodeEventError) -> Self { AcceptingError::EventDecodeError(e) }
}
