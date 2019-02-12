use chain_core::property::{Block, HasHeader, Header, TransactionId};

use network_core::client::{self as core_client, block::BlockService, block::HeaderService};
pub use protocol::protocol::ProtocolMagic;
use protocol::{
    network_transport::LightWeightConnectionId,
    protocol::{CloseLightConnection, GetBlockHeaders, GetBlocks, NewLightConnection},
    Inbound, InboundError, InboundStream, Message, OutboundError, OutboundSink, ProtocolBlock,
    ProtocolBlockId, ProtocolHeader, ProtocolTransactionId, Response,
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

use tokio::net::TcpStream;
use tokio::prelude::*;

/// A handle that can be used in order for communication
/// with the client thread.
pub struct ClientHandle<T: Block + HasHeader, Tx> {
    channel: mpsc::UnboundedSender<Request<T>>,
    phantom: PhantomData<Tx>,
}

/// Connect to the remote client. Returns future that can
/// be run on any executor.
pub fn connect<B, H, I, D, Tx>(
    sockaddr: SocketAddr,
    magic: ProtocolMagic,
) -> impl Future<Item = (Connection<TcpStream, B, Tx>, ClientHandle<B, Tx>), Error = core_client::Error>
where
    Tx: ProtocolTransactionId,
    B: NttBlock<D, I, H>,
    H: NttHeader<D, I>,
    I: NttId,
    D: NttDate,
{
    TcpStream::connect(&sockaddr)
        .map_err(move |err| core_client::Error::new(core_client::ErrorKind::Rpc, err))
        .and_then(move |stream| {
            protocol::Connection::connect(stream, magic)
                .map_err(move |err| core_client::Error::new(core_client::ErrorKind::Rpc, err))
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
pub struct RequestFuture<T>(oneshot::Receiver<Result<T, core_client::Error>>);

impl<T> Future for RequestFuture<T> {
    type Item = T;
    type Error = core_client::Error;

    fn poll(&mut self) -> Poll<T, Self::Error> {
        match self.0.poll() {
            Ok(Async::Ready(Ok(x))) => Ok(Async::Ready(x)),
            Ok(Async::Ready(Err(x))) => Err(x),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(core_client::Error::new(core_client::ErrorKind::Rpc, e)),
        }
    }
}

pub struct RequestStream<T>(oneshot::Receiver<T>);

impl<T> Stream for RequestStream<T> {
    type Item = T;
    type Error = core_client::Error;

    fn poll(&mut self) -> Poll<Option<T>, Self::Error> {
        match self.0.poll() {
            _ => unimplemented!(),
        }
    }
}

pub struct PullBlocksToTip<T: Block + HasHeader> {
    chan: TipFuture<T::Header>,
    from: T::Id,
    request: mpsc::UnboundedSender<Request<T>>,
}

impl<T: Block + HasHeader> Future for PullBlocksToTip<T>
where
    T::Header: Header<Id = <T as Block>::Id, Date = <T as Block>::Date>,
{
    type Item = PullBlocksToTipStream<T>;
    type Error = core_client::Error;

    fn poll(&mut self) -> Poll<PullBlocksToTipStream<T>, Self::Error> {
        match self.chan.poll() {
            Ok(Async::Ready((tip, _date))) => {
                let (sender, receiver) = mpsc::unbounded();
                self.request
                    .unbounded_send(Request::Block(sender, self.from.clone(), tip))
                    .unwrap();
                let stream = PullBlocksToTipStream { channel: receiver };
                Ok(Async::Ready(stream))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(core_client::Error::new(core_client::ErrorKind::Rpc, e)),
        }
    }
}

pub struct PullBlocksToTipStream<T> {
    channel: mpsc::UnboundedReceiver<Result<T, core_client::Error>>,
}

impl<T: Block> Stream for PullBlocksToTipStream<T> {
    type Item = T;
    type Error = core_client::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.channel.poll() {
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(Ok(block)))) => Ok(Async::Ready(Some(block))),
            Ok(Async::Ready(Some(Err(err)))) => Err(err),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(_) => Err(core_client::Error::new(
                core_client::ErrorKind::Rpc,
                "error reading from unbounded channel",
            )),
        }
    }
}

pub struct TipFuture<T>(RequestFuture<T>);

impl<T: Header> Future for TipFuture<T> {
    type Item = (T::Id, T::Date);
    type Error = core_client::Error;

    fn poll(&mut self) -> Poll<(T::Id, T::Date), Self::Error> {
        match self.0.poll() {
            Ok(Async::Ready(hdr)) => {
                let id = hdr.id();
                let date = hdr.date();
                Ok(Async::Ready((id, date)))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(core_client::Error::new(core_client::ErrorKind::Rpc, e)),
        }
    }
}

impl<T: Block + HasHeader, Tx> BlockService<T> for ClientHandle<T, Tx>
where
    T::Header: Header<Id = <T as Block>::Id, Date = <T as Block>::Date>,
{
    type TipFuture = TipFuture<T::Header>;
    type PullBlocksToTipStream = PullBlocksToTipStream<T>;
    type PullBlocksToTipFuture = PullBlocksToTip<T>;
    type GetBlocksStream = RequestStream<T>;
    type GetBlocksFuture = RequestFuture<RequestStream<T>>;

    fn tip(&mut self) -> Self::TipFuture {
        let (source, sink) = oneshot::channel();
        self.channel.unbounded_send(Request::Tip(source)).unwrap();
        TipFuture(RequestFuture(sink))
    }

    fn pull_blocks_to_tip(&mut self, from: &[T::Id]) -> Self::PullBlocksToTipFuture {
        let (source, sink) = oneshot::channel();
        self.channel.unbounded_send(Request::Tip(source)).unwrap();
        PullBlocksToTip {
            chan: TipFuture(RequestFuture(sink)),
            from: from[0].clone(),
            request: self.channel.clone(),
        }
    }
}

impl<T: Block, Tx> HeaderService<T> for ClientHandle<T, Tx>
where
    T: HasHeader,
{
    //type GetHeadersStream = Self::GetHeadersStream<T::Header>;
    //type GetHeadersFuture = Self::GetHeaders<T::Header>;
    type GetTipFuture = RequestFuture<T::Header>;

    fn tip_header(&mut self) -> Self::GetTipFuture {
        let (source, sink) = oneshot::channel();
        self.channel.unbounded_send(Request::Tip(source)).unwrap();
        RequestFuture(sink)
    }
}

enum Request<T: Block + HasHeader> {
    Tip(oneshot::Sender<Result<T::Header, core_client::Error>>),
    Block(
        mpsc::UnboundedSender<Result<T, core_client::Error>>,
        T::Id,
        T::Id,
    ),
}

pub trait NttBlock<D, I, H>:
    Block<Id = I, Date = D>
    + core::fmt::Debug
    + HasHeader<Header = H>
    + cbor_event::Deserialize
    + cbor_event::Serialize
where
    D: NttDate,
    I: NttId,
    H: NttHeader<D, I> + Clone + core::fmt::Debug,
{
}

impl<D, I, H, T> NttBlock<D, I, H> for T
where
    T: Block<Id = I, Date = D>
        + core::fmt::Debug
        + HasHeader<Header = H>
        + cbor_event::Deserialize
        + cbor_event::Serialize,
    D: NttDate,
    I: NttId,
    H: NttHeader<D, I> + Clone + core::fmt::Debug,
{
}

pub trait NttHeader<D, I>:
    Header<Id = I, Date = D>
    + cbor_event::Deserialize
    + cbor_event::Serialize
    + core::fmt::Debug
    + Clone
where
    D: chain_core::property::BlockDate + core::fmt::Debug,
    I: cbor_event::Deserialize
        + cbor_event::Serialize
        + chain_core::property::BlockId
        + core::fmt::Debug,
{
}

impl<D, I, T> NttHeader<D, I> for T
where
    T: Header<Id = I, Date = D>
        + cbor_event::Deserialize
        + cbor_event::Serialize
        + core::fmt::Debug
        + Clone,
    D: chain_core::property::BlockDate + core::fmt::Debug,
    I: cbor_event::Deserialize
        + cbor_event::Serialize
        + chain_core::property::BlockId
        + core::fmt::Debug,
{
}

pub trait NttDate: chain_core::property::BlockDate + core::fmt::Debug {}

impl<T> NttDate for T where T: chain_core::property::BlockDate + core::fmt::Debug {}

pub trait NttId:
    cbor_event::Deserialize + cbor_event::Serialize + chain_core::property::BlockId + core::fmt::Debug
{
}

impl<T> NttId for T where
    T: cbor_event::Deserialize
        + cbor_event::Serialize
        + chain_core::property::BlockId
        + core::fmt::Debug
{
}

#[derive(Debug)]
pub enum Error {
    Inbound(InboundError),
    Outbound(OutboundError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Inbound(e) => write!(f, "network input error"),
            Error::Outbound(e) => write!(f, "network output error"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
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
    commands: mpsc::UnboundedReceiver<Request<B>>,
    requests: HashMap<LightWeightConnectionId, Request<B>>,
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
        commands: mpsc::UnboundedReceiver<Request<B>>,
    ) -> Self {
        let (sink, stream) = connection.split();
        Connection {
            inbound: Some(stream),
            out_state: OutboundState::Ready(sink),
            commands,
            requests: HashMap::new(),
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
        if let Some(ref mut inbound) = self.inbound {
            loop {
                let mut events_processed = false;
                match inbound.poll() {
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
                            Ok(Async::Ready(Some(req))) => {
                                self.process_request(req);
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
        }

        // We only get here if the inbound stream or the request queue
        // is closed.
        // Make sure the inbound half is dropped.
        self.inbound.take();

        // Manage shutdown of the outbound half,
        // returning result as the result of the connection poll.
        try_ready!(self.out_state.close());
        Ok(Async::Ready(()))
    }
}

fn unexpected_response_error() -> core_client::Error {
    core_client::Error::new(core_client::ErrorKind::Rpc, "unexpected response".into())
}

fn convert_response<P, Q, F>(
    response: Response<P, String>,
    conversion: F,
) -> Result<Q, core_client::Error>
where
    F: FnOnce(P) -> Q,
{
    match response {
        Response::Ok(x) => Ok(conversion(x)),
        Response::Err(err) => Err(core_client::Error::new(core_client::ErrorKind::Rpc, err)),
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
                let request = self.requests.remove(&lwcid);
                match request {
                    None => {
                        // TODO: log the bogus response
                    }
                    Some(Request::Tip(chan)) => {
                        let res = convert_response(response, |p| p.0[0]);
                        chan.send(res).unwrap();
                    }
                    Some(Request::Block(chan, ..)) => {
                        chan.unbounded_send(Err(unexpected_response_error()))
                            .unwrap();
                    }
                }
            }
            Inbound::Block(lwcid, response) => {
                use hash_map::Entry::*;

                match self.requests.entry(lwcid) {
                    Vacant(_) => {
                        // TODO: log the bogus response
                    }
                    Occupied(entry) => match entry.get() {
                        Request::Block(chan, ..) => {
                            let res = convert_response(response, |p| p);
                            chan.unbounded_send(res).unwrap();
                        }
                        Request::Tip(chan) => {
                            chan.send(Err(unexpected_response_error())).unwrap();
                        }
                    },
                }
            }
            Inbound::TransactionReceived(_lwcid, _response) => {
                // TODO: to be implemented
            }
            Inbound::CloseConnection(lwcid) => {
                match self.requests.remove(&lwcid) {
                    None => {
                        // TODO: log the bogus close message
                    }
                    Some(Request::Tip(chan)) => {
                        chan.send(Err(core_client::Error::new(
                            core_client::ErrorKind::Rpc,
                            "unexpected close",
                        )))
                        .unwrap();
                    }
                    Some(Request::Block(mut chan, ..)) => {
                        chan.close().unwrap();
                    }
                    _ => (),
                };
            }
            _ => {}
        }
    }

    fn process_request(&mut self, req: Request<B>) {
        let (new_out_state, lwcid) = self.out_state.start_send(&req);
        self.out_state = new_out_state;
        self.requests.insert(lwcid, req);
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
    PendingMessage(NewLightConnection<T, B, Tx>, Option<Message<B, Tx>>),
    Sending(sink::Send<OutboundSink<T, B, Tx>>, LightWeightConnectionId),
    ClosingLightConnection(CloseLightConnection<T, B, Tx>),
    Closing(OutboundSink<T, B, Tx>),
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
                Closing(_) => panic!("outbound connection polled after closing"),
                Closed => panic!("outbound connection polled after closing"),
            };
            *self = new_state;
        }
    }

    fn start_send(self, req: &Request<B>) -> (Self, LightWeightConnectionId) {
        use OutboundState::*;

        match self {
            Ready(sink) => {
                let future = sink.new_light_connection();
                let lwcid = future.connection_id();
                let msg = match req {
                    Request::Tip(t) => Message::GetBlockHeaders(
                        lwcid,
                        GetBlockHeaders {
                            from: vec![],
                            to: None,
                        },
                    ),
                    Request::Block(t, from, to) => Message::GetBlocks(
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
        }
    }

    fn close(&mut self) -> Poll<(), OutboundError> {
        use OutboundState::*;

        let sink = match self {
            Closed => return Ok(Async::Ready(())),
            Closing(sink) => sink,
            Ready(sink) => sink,
            PendingMessage(future, _) => future.get_mut(),
            Sending(future, _) => future.get_mut(),
            ClosingLightConnection(future) => future.get_mut(),
        };

        sink.close()
    }
}
