use super::gossip::NodeId;

use chain_core::property::{Block, HasHeader, Header};

use network_core::{
    client::{block::BlockService, p2p::P2pService},
    error as core_error,
    subscription::BlockEvent,
};
pub use protocol::protocol::ProtocolMagic;
use protocol::{
    network_transport::LightWeightConnectionId,
    protocol::{CloseLightConnection, GetBlockHeaders, GetBlocks, NewLightConnection},
    ConnectingError, Inbound, InboundError, InboundStream, Message, OutboundError, OutboundSink,
    ProtocolBlock, ProtocolBlockId, ProtocolHeader, ProtocolTransactionId, Response,
};

use futures::{
    sink,
    sync::{mpsc, oneshot},
};

use std::{
    collections::{hash_map, HashMap},
    error, fmt,
    marker::PhantomData,
    mem,
    net::SocketAddr,
};

use tokio::prelude::*;
use tokio::{io, net::TcpStream};

/// A handle that can be used in order for communication
/// with the client thread.
pub struct ClientHandle<B: Block + HasHeader, Tx> {
    channel: mpsc::UnboundedSender<Command<B>>,
    phantom: PhantomData<Tx>,
}

/// Connect to the remote client. Returns future that can
/// be run on any executor.
pub fn connect<B, Tx>(
    sockaddr: SocketAddr,
    magic: ProtocolMagic,
) -> impl Future<Item = (Connection<TcpStream, B, Tx>, ClientHandle<B, Tx>), Error = Error>
where
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as Block>::Id: ProtocolBlockId,
    <B as HasHeader>::Header: ProtocolHeader,
{
    TcpStream::connect(&sockaddr)
        .map_err(Error::Connect)
        .and_then(move |stream| {
            protocol::Connection::connect(stream, magic)
                .map_err(Error::Handshake)
                .and_then(move |connection| {
                    let (cmd_sink, cmd_source) = mpsc::unbounded();
                    let handle = ClientHandle {
                        channel: cmd_sink,
                        phantom: PhantomData,
                    };
                    future::ok((Connection::new(connection, cmd_source), handle))
                })
        })
}

/// Internal message that is used to load reply from the client.
pub struct RequestFuture<T>(oneshot::Receiver<Result<T, core_error::Error>>);

impl<T> Future for RequestFuture<T> {
    type Item = T;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<T, Self::Error> {
        match self.0.poll() {
            Ok(Async::Ready(Ok(x))) => Ok(Async::Ready(x)),
            Ok(Async::Ready(Err(x))) => Err(x),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(core_error::Error::new(core_error::Code::Internal, e)),
        }
    }
}

pub struct PullBlocksToTip<T: Block + HasHeader> {
    tip_future: TipFuture<T::Header>,
    from: T::Id,
    command_channel: mpsc::UnboundedSender<Command<T>>,
}

impl<T: Block + HasHeader> Future for PullBlocksToTip<T>
where
    T::Header: Header<Id = <T as Block>::Id, Date = <T as Block>::Date>,
{
    type Item = RequestStream<T>;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<RequestStream<T>, Self::Error> {
        use StreamRequest::Blocks;

        match self.tip_future.poll() {
            Ok(Async::Ready((tip, _date))) => {
                let (sender, receiver) = mpsc::unbounded();
                self.command_channel
                    .unbounded_send(Command::Stream(Blocks(sender, self.from.clone(), tip)))
                    .unwrap();
                let stream = RequestStream { channel: receiver };
                Ok(Async::Ready(stream))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

pub struct RequestStream<T> {
    channel: mpsc::UnboundedReceiver<Result<T, core_error::Error>>,
}

impl<T> Stream for RequestStream<T> {
    type Item = T;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.channel.poll() {
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(Ok(block)))) => Ok(Async::Ready(Some(block))),
            Ok(Async::Ready(Some(Err(err)))) => Err(err),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(_) => Err(core_error::Error::new(
                core_error::Code::Internal,
                "error reading from unbounded channel",
            )),
        }
    }
}

pub struct TipFuture<T>(RequestFuture<T>);

impl<T: Header> Future for TipFuture<T> {
    type Item = (T::Id, T::Date);
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<(T::Id, T::Date), Self::Error> {
        match self.0.poll() {
            Ok(Async::Ready(hdr)) => {
                let id = hdr.id();
                let date = hdr.date();
                Ok(Async::Ready((id, date)))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

impl<T, Tx> P2pService for ClientHandle<T, Tx>
where
    T: Block + HasHeader,
{
    type NodeId = NodeId;
}

impl<T: Block + HasHeader, Tx> BlockService for ClientHandle<T, Tx>
where
    T::Header: Header<Id = <T as Block>::Id, Date = <T as Block>::Date>,
{
    type Block = T;
    type TipFuture = RequestFuture<T::Header>;
    type PullBlocksStream = RequestStream<T>;
    type PullBlocksToTipFuture = PullBlocksToTip<T>;
    type PullHeadersStream = RequestStream<T::Header>;
    type PullHeadersFuture = RequestFuture<RequestStream<T::Header>>;
    type GetBlocksStream = RequestStream<T>;
    type GetBlocksFuture = RequestFuture<RequestStream<T>>;
    type UploadBlocksFuture = RequestFuture<()>;
    type BlockSubscription = RequestStream<BlockEvent<T>>;
    type BlockSubscriptionFuture = RequestFuture<(Self::BlockSubscription, NodeId)>;

    fn tip(&mut self) -> Self::TipFuture {
        use UnaryRequest::Tip;

        let (source, sink) = oneshot::channel();
        self.channel
            .unbounded_send(Command::Unary(Tip(source)))
            .unwrap();
        RequestFuture(sink)
    }

    fn pull_blocks_to_tip(&mut self, from: &[T::Id]) -> Self::PullBlocksToTipFuture {
        use UnaryRequest::Tip;

        let (source, sink) = oneshot::channel();
        self.channel
            .unbounded_send(Command::Unary(Tip(source)))
            .unwrap();
        PullBlocksToTip {
            tip_future: TipFuture(RequestFuture(sink)),
            from: from[0].clone(),
            command_channel: self.channel.clone(),
        }
    }

    fn pull_headers(
        &mut self,
        _from: &[<Self::Block as Block>::Id],
        _to: &<Self::Block as Block>::Id,
    ) -> Self::PullHeadersFuture {
        unimplemented!()
    }

    fn get_blocks(&mut self, _ids: &[<Self::Block as Block>::Id]) -> Self::GetBlocksFuture {
        unimplemented!()
    }

    fn upload_blocks<S>(&mut self, _blocks: S) -> Self::UploadBlocksFuture
    where
        S: Stream<Item = Self::Block> + Send + 'static,
    {
        unimplemented!()
    }

    fn block_subscription<Out>(&mut self, _outbound: Out) -> Self::BlockSubscriptionFuture
    where
        Out: Stream<Item = T::Header>,
    {
        unimplemented!()
    }
}

enum Command<B: Block + HasHeader> {
    Unary(UnaryRequest<B>),
    Stream(StreamRequest<B>),
}

enum UnaryRequest<B: Block + HasHeader> {
    Tip(oneshot::Sender<Result<B::Header, core_error::Error>>),
}

enum StreamRequest<B: Block + HasHeader> {
    Blocks(
        mpsc::UnboundedSender<Result<B, core_error::Error>>,
        B::Id,
        B::Id,
    ),
}

#[derive(Debug)]
pub enum Error {
    Connect(io::Error),
    Handshake(ConnectingError),
    Inbound(InboundError),
    Outbound(OutboundError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Connect(_) => write!(f, "connection error"),
            Error::Handshake(_) => write!(f, "failed to set up the protocol connection"),
            Error::Inbound(_) => write!(f, "network input error"),
            Error::Outbound(_) => write!(f, "network output error"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Connect(e) => Some(e),
            Error::Handshake(e) => Some(e),
            Error::Inbound(e) => Some(e),
            Error::Outbound(e) => Some(e),
        }
    }
}

impl From<InboundError> for Error {
    fn from(err: InboundError) -> Self {
        Error::Inbound(err)
    }
}

impl From<OutboundError> for Error {
    fn from(err: OutboundError) -> Self {
        Error::Outbound(err)
    }
}

pub struct Connection<T, B, Tx>
where
    T: AsyncRead + AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as Block>::Id: ProtocolBlockId,
    <B as HasHeader>::Header: ProtocolHeader,
{
    inbound: Option<InboundStream<T, B, Tx>>,
    out_state: OutboundState<T, B, Tx>,
    commands: mpsc::UnboundedReceiver<Command<B>>,
    unary_requests: HashMap<LightWeightConnectionId, UnaryRequest<B>>,
    stream_requests: HashMap<LightWeightConnectionId, StreamRequest<B>>,
}

impl<T, B, Tx> Connection<T, B, Tx>
where
    T: AsyncRead + AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as Block>::Id: ProtocolBlockId,
    <B as HasHeader>::Header: ProtocolHeader,
{
    fn new(
        connection: protocol::Connection<T, B, Tx>,
        commands: mpsc::UnboundedReceiver<Command<B>>,
    ) -> Self {
        let (sink, stream) = connection.split();
        Connection {
            inbound: Some(stream),
            out_state: OutboundState::Ready(sink),
            commands,
            unary_requests: HashMap::new(),
            stream_requests: HashMap::new(),
        }
    }
}

impl<T, B, Tx> Future for Connection<T, B, Tx>
where
    T: AsyncRead + AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as Block>::Id: ProtocolBlockId,
    <B as HasHeader>::Header: ProtocolHeader,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<(), Self::Error> {
        if self.inbound.is_some() {
            loop {
                let mut events_processed = false;
                match self.inbound.as_mut().unwrap().poll() {
                    Ok(Async::NotReady) => {}
                    Ok(Async::Ready(None)) => {
                        break;
                    }
                    Ok(Async::Ready(Some(msg))) => {
                        self.process_inbound(msg);
                        events_processed = true;
                    }
                    Err(err) => return Err(err.into()),
                }
                match self.out_state.poll_ready() {
                    Ok(Async::NotReady) => {
                        // Some work on the output is pending,
                        // not processing commands this time.
                    }
                    Ok(Async::Ready(())) => {
                        // The output state machine is ready
                        // for sending messages.
                        match self.commands.poll() {
                            Ok(Async::NotReady) => {}
                            Ok(Async::Ready(Some(cmd))) => {
                                self.process_command(cmd);
                                events_processed = true;
                            }
                            Ok(Async::Ready(None)) => {
                                // The request queue has been closed,
                                // proceed to shutdown.
                                break;
                            }
                            Err(err) => panic!("unexpected error in command queue: {:?}", err),
                        }
                    }
                    Err(err) => return Err(err.into()),
                }
                if !events_processed {
                    return Ok(Async::NotReady);
                }
            }

            // We only get here if the inbound stream or the request queue
            // is closed.
            // Make sure the inbound half is dropped.
            self.inbound = None;
        }

        // Manage shutdown of the outbound half,
        // returning result as the result of the connection poll.
        try_ready!(self.out_state.close());
        Ok(Async::Ready(()))
    }
}

// To be used when UnaryRequest and StreamRequest are extended with more variants.
#[allow(dead_code)]
fn unexpected_response_error() -> core_error::Error {
    core_error::Error::new(core_error::Code::Unimplemented, "unexpected response")
}

fn convert_response<P, Q, F>(
    response: Response<P, String>,
    conversion: F,
) -> Result<Q, core_error::Error>
where
    F: FnOnce(P) -> Q,
{
    match response {
        Response::Ok(x) => Ok(conversion(x)),
        Response::Err(err) => Err(core_error::Error::new(core_error::Code::Unknown, err)),
    }
}

impl<T, B, Tx> Connection<T, B, Tx>
where
    T: AsyncRead + AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as Block>::Id: ProtocolBlockId,
    <B as HasHeader>::Header: ProtocolHeader,
{
    fn process_inbound(&mut self, inbound: Inbound<B, Tx>) {
        match inbound {
            Inbound::NothingExciting => {}
            Inbound::BlockHeaders(lwcid, response) => {
                let request = self.unary_requests.remove(&lwcid);
                #[allow(unreachable_patterns)]
                match request {
                    None => {
                        // TODO: log the bogus response
                    }
                    Some(UnaryRequest::Tip(chan)) => {
                        let res = convert_response(response, |headers| {
                            headers.0.into_iter().next().unwrap()
                        });
                        chan.send(res).unwrap();
                    }
                    Some(_ /* UnaryRequest::Blah(chan) */) => {
                        // chan.unbounded_send(Err(unexpected_response_error()))
                        //     .unwrap();
                    }
                }
            }
            Inbound::Block(lwcid, response) => {
                use hash_map::Entry::*;

                #[allow(unreachable_patterns)]
                match self.stream_requests.entry(lwcid) {
                    Vacant(_) => {
                        // TODO: log the bogus response
                    }
                    Occupied(entry) => match entry.get() {
                        StreamRequest::Blocks(chan, ..) => {
                            let res = convert_response(response, |p| p);
                            chan.unbounded_send(res).unwrap();
                        }
                        _ /* StreamRequest::Blah(chan) */ => {
                            // chan.send(Err(unexpected_response_error())).unwrap();
                        }
                    },
                }
            }
            Inbound::TransactionReceived(_lwcid, _response) => {
                // TODO: to be implemented
            }
            Inbound::CloseConnection(lwcid) => {
                match self.stream_requests.remove(&lwcid) {
                    None => {
                        // TODO: log the bogus close message
                    }
                    Some(StreamRequest::Blocks(mut chan, ..)) => {
                        chan.close().unwrap();
                    }
                };
            }
            _ => {}
        }
    }

    fn process_command(&mut self, cmd: Command<B>) {
        let lwcid = self.out_state.start_send(&cmd);
        match cmd {
            Command::Unary(req) => {
                self.unary_requests.insert(lwcid, req);
            }
            Command::Stream(req) => {
                self.stream_requests.insert(lwcid, req);
            }
        }
    }
}

enum OutboundState<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as Block>::Id: ProtocolBlockId,
    <B as HasHeader>::Header: ProtocolHeader,
{
    Ready(OutboundSink<T, B, Tx>),
    Intermediate,
    PendingMessage(NewLightConnection<T, B, Tx>, Option<Message<B, Tx>>),
    Sending(sink::Send<OutboundSink<T, B, Tx>>, LightWeightConnectionId),
    ClosingLightConnection(CloseLightConnection<T, B, Tx>),
    Closed,
}

impl<T, B, Tx> OutboundState<T, B, Tx>
where
    T: AsyncWrite,
    B: ProtocolBlock,
    Tx: ProtocolTransactionId,
    <B as Block>::Id: ProtocolBlockId,
    <B as HasHeader>::Header: ProtocolHeader,
{
    fn poll_ready(&mut self) -> Poll<(), OutboundError> {
        use OutboundState::*;

        loop {
            let new_state = match self {
                Ready(_) => return Ok(Async::Ready(())),
                PendingMessage(future, msg) => {
                    let (lwcid, sink) = try_ready!(future.poll());
                    let msg = mem::replace(msg, None).unwrap();
                    Sending(sink.send(msg), lwcid)
                }
                Sending(future, lwcid) => {
                    let sink = try_ready!(future.poll());
                    ClosingLightConnection(sink.close_light_connection(lwcid.clone()))
                }
                ClosingLightConnection(future) => {
                    let sink = try_ready!(future.poll());
                    Ready(sink)
                }
                Closed => panic!("outbound connection polled after closing"),
                Intermediate => unreachable!(),
            };
            *self = new_state;
        }
    }

    fn start_send(&mut self, cmd: &Command<B>) -> LightWeightConnectionId {
        use OutboundState::*;

        let (new_state, lwcid) = match mem::replace(self, Intermediate) {
            Ready(sink) => {
                let future = sink.new_light_connection();
                let lwcid = future.connection_id();
                let msg = match cmd {
                    Command::Unary(UnaryRequest::Tip(_)) => Message::GetBlockHeaders(
                        lwcid,
                        GetBlockHeaders {
                            from: vec![],
                            to: None,
                        },
                    ),
                    Command::Stream(StreamRequest::Blocks(_, from, to)) => Message::GetBlocks(
                        lwcid,
                        GetBlocks {
                            from: from.clone(),
                            to: to.clone(),
                        },
                    ),
                };
                (PendingMessage(future, Some(msg)), lwcid)
            }
            _ => unreachable!(),
        };
        *self = new_state;
        lwcid
    }

    fn close(&mut self) -> Poll<(), OutboundError> {
        use OutboundState::*;

        // If closed already, return Ready,
        // otherwise, get the sink reference out of the current state,
        // so that we can call its close method.
        // Note that if close is called repeatedly due to returning
        // Async::NotReady, we simply delegate the closing work
        // to the inner sink regardless of the operation that was in
        // flight when close was first called.
        let sink = match self {
            Closed => return Ok(Async::Ready(())),
            Ready(sink) => sink,
            PendingMessage(future, _) => future.get_mut(),
            Sending(future, _) => future.get_mut(),
            ClosingLightConnection(future) => future.get_mut(),
            Intermediate => unreachable!(),
        };

        try_ready!(sink.close());

        // Closing is done. Finalize the state and report completion.
        *self = Closed;
        Ok(Async::Ready(()))
    }
}
