use chain_core::property::{Block, HasHeader, Header, TransactionId};
use future::Either;
use futures::{sync::mpsc, sync::oneshot};
use network_core::client::{self as core_client, block::BlockService};
use protocol::{
    network_transport::LightWeightConnectionId, protocol::BlockHeaders, protocol::GetBlockHeaders,
    protocol::GetBlocks, Inbound, Message, Response,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::prelude::*;

/// A handle that can be used in order for communication
/// with the client thread.
pub struct ClientHandle<T: Block, Tx> {
    channel: mpsc::UnboundedSender<Request<T>>,
    phantom: PhantomData<Tx>,
}

/// Connect to the remote client. Returns future that can
/// be run on any executor.
pub fn connect<B, H, I, D, Tx>(
    sockaddr: SocketAddr,
) -> impl Future<
    Item = (impl Future<Item = (), Error = ()>, ClientHandle<B, Tx>),
    Error = core_client::Error,
>
where
    Tx: TransactionId + cbor_event::Serialize + cbor_event::Deserialize,
    B: NttBlock<D, I, H>,
    H: NttHeader<D, I>,
    I: NttId,
    D: NttDate,
{
    TcpStream::connect(&sockaddr)
        .map_err(move |err| core_client::Error::new(core_client::ErrorKind::Rpc, err))
        .and_then(move |stream| {
            protocol::Connection::connect(stream)
                .map_err(move |err| core_client::Error::new(core_client::ErrorKind::Rpc, err))
                .and_then(move |connection: protocol_tokio::Connection<_, B, Tx>| {
                    let (stream, source) = mpsc::unbounded();
                    let handle = ClientHandle {
                        channel: stream,
                        phantom: PhantomData,
                    };
                    future::ok((run_connection(connection, source), handle))
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

pub struct PullBlocksToTip<T: Block> {
    chan: RequestFuture<(T::Id, T::Date)>,
    from: T::Id,
    request: mpsc::UnboundedSender<Request<T>>,
}

impl<T: Block> Future for PullBlocksToTip<T> {
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

impl<T: Block, Tx> BlockService<T> for ClientHandle<T, Tx> {
    type TipFuture = RequestFuture<(T::Id, T::Date)>;

    type PullBlocksToTipStream = PullBlocksToTipStream<T>;
    type PullBlocksToTipFuture = PullBlocksToTip<T>;
    type GetBlocksStream = RequestStream<T>;
    type GetBlocksFuture = RequestFuture<RequestStream<T>>;

    fn tip(&mut self) -> Self::TipFuture {
        let (source, sink) = oneshot::channel();
        self.channel.unbounded_send(Request::Tip(source)).unwrap();
        RequestFuture(sink)
    }

    fn pull_blocks_to_tip(&mut self, from: &[T::Id]) -> Self::PullBlocksToTipFuture {
        let (source, sink) = oneshot::channel();
        self.channel.unbounded_send(Request::Tip(source)).unwrap();
        PullBlocksToTip {
            chan: RequestFuture(sink),
            from: from[0].clone(),
            request: self.channel.clone(),
        }
    }
}

enum Request<T: Block> {
    Tip(oneshot::Sender<Result<(T::Id, T::Date), core_client::Error>>),
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
    H: NttHeader<D, I>,
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
    H: NttHeader<D, I>,
{
}

pub trait NttHeader<D, I>:
    Header<Id = I, Date = D> + cbor_event::Deserialize + cbor_event::Serialize
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
    T: Header<Id = I, Date = D> + cbor_event::Deserialize + cbor_event::Serialize,
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

struct ConnectionState<B: Block + HasHeader> {
    requests: HashMap<LightWeightConnectionId, Request<B>>,
}

impl<B: Block + HasHeader> ConnectionState<B> {
    pub fn new() -> Self {
        ConnectionState {
            requests: HashMap::new(),
        }
    }
}

enum Command<B: Block + HasHeader, Tx: TransactionId> {
    Message(Message<B, Tx>),
    BlockHeaders(
        LightWeightConnectionId,
        Response<BlockHeaders<B::Header>, String>,
    ),
    Blocks(LightWeightConnectionId, Response<B, String>),
    Transaction(LightWeightConnectionId, Response<bool, String>),
    Request(Request<B>),
    CloseConnection(LightWeightConnectionId),
}

enum V<A1, A2, A3, A4, A5, A6, A7> {
    A1(A1),
    A2(A2),
    A3(A3),
    A4(A4),
    A5(A5),
    A6(A6),
    A7(A7),
}

impl<A1, A2, A3, A4, A5, A6, A7> Future for V<A1, A2, A3, A4, A5, A6, A7>
where
    A1: Future,
    A2: Future<Item = A1::Item, Error = A1::Error>,
    A3: Future<Item = A1::Item, Error = A1::Error>,
    A4: Future<Item = A1::Item, Error = A1::Error>,
    A5: Future<Item = A1::Item, Error = A1::Error>,
    A6: Future<Item = A1::Item, Error = A1::Error>,
    A7: Future<Item = A1::Item, Error = A1::Error>,
{
    type Item = A1::Item;
    type Error = A1::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match *self {
            V::A1(ref mut x) => x.poll(),
            V::A2(ref mut x) => x.poll(),
            V::A3(ref mut x) => x.poll(),
            V::A4(ref mut x) => x.poll(),
            V::A5(ref mut x) => x.poll(),
            V::A6(ref mut x) => x.poll(),
            V::A7(ref mut x) => x.poll(),
        }
    }
}

fn run_connection<T, B: NttBlock<D, I, H>, H: NttHeader<D, I>, I: NttId, D: NttDate, Tx>(
    connection: protocol::Connection<T, B, Tx>,
    input: mpsc::UnboundedReceiver<Request<B>>,
) -> impl future::Future<Item = (), Error = ()>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite,
    Tx: TransactionId + cbor_event::Serialize + cbor_event::Deserialize,
{
    let (sink, stream) = connection.split();
    let (sink_tx, sink_rx) = mpsc::unbounded();
    let sink2_tx = sink_tx.clone();
    // process messages comming from the network.
    let stream = stream
        .for_each(move |inbound| match inbound {
            Inbound::NothingExciting => future::ok(()),
            Inbound::BlockHeaders(lwcid, response) => {
                sink_tx
                    .unbounded_send(Command::BlockHeaders(lwcid, response))
                    .unwrap();
                future::ok(())
            }
            Inbound::Block(lwcid, response) => {
                sink_tx
                    .unbounded_send(Command::Blocks(lwcid, response))
                    .unwrap();
                future::ok(())
            }
            Inbound::TransactionReceived(_lwcid, _response) => future::ok(()),
            Inbound::CloseConnection(lwcid) => {
                sink_tx
                    .unbounded_send(Command::CloseConnection(lwcid))
                    .unwrap();
                future::ok(())
            }
            _ => future::ok(()),
        })
        .map_err(|_| ())
        .map(|_| ());
    // Accept all commands from the program and send that
    // further in the ppeline.
    let commands = input
        .for_each(move |request| {
            sink2_tx.unbounded_send(Command::Request(request)).unwrap();
            future::ok(())
        })
        .map_err(|_err| ())
        .map(|_| ());

    // Receive commands.
    let sink = sink
        .subscribe(false)
        .map_err(|_err| ())
        .and_then(move |(_lwcid, sink)| {
            let cc: ConnectionState<B> = ConnectionState::new();
            sink_rx
                .fold((sink, cc), move |(sink, mut cc), outbound| match outbound {
                    Command::Message(Message::AckNodeId(_lwcid, node_id)) => V::A1(
                        sink.ack_node_id(node_id)
                            .map_err(|_err| ())
                            .map(|x| (x, cc)),
                    ),
                    Command::Message(message) => {
                        V::A2(sink.send(message).map_err(|_err| ()).map(|x| (x, cc)))
                    }
                    Command::BlockHeaders(lwid, resp) => {
                        let request = cc.requests.remove(&lwid);
                        V::A3(match request {
                            Some(Request::Tip(chan)) => match resp {
                                Response::Ok(x) => {
                                    let id = x.0[0].id();
                                    let date = x.0[0].date();
                                    chan.send(Ok((id, date))).unwrap();
                                    Either::A(future::ok((sink, cc)))
                                }
                                Response::Err(x) => {
                                    chan.send(Err(core_client::Error::new(
                                        core_client::ErrorKind::Rpc,
                                        x,
                                    )))
                                    .unwrap();
                                    Either::A(future::ok((sink, cc)))
                                }
                            },
                            Some(Request::Block(chan, _, _)) => Either::B(
                                chan.send(Err(core_client::Error::new(
                                    core_client::ErrorKind::Rpc,
                                    "unexpected reply".to_string(),
                                )))
                                .map_err(|_| ())
                                .and_then(|_| future::ok((sink, cc))),
                            ),
                            None => Either::A(future::ok((sink, cc))),
                        })
                    }
                    Command::Blocks(lwid, resp) => {
                        let val = cc.requests.remove(&lwid);
                        V::A4(
                            match val {
                                Some(Request::Block(chan, a, b)) => Either::B(
                                    match resp {
                                        Response::Ok(x) => {
                                            cc.requests
                                                .insert(lwid, Request::Block(chan.clone(), a, b));
                                            chan.send(Ok(x))
                                        }
                                        Response::Err(x) => chan.send(Err(
                                            core_client::Error::new(core_client::ErrorKind::Rpc, x),
                                        )),
                                    }
                                    .map_err(|_| ())
                                    .map(|_| ()),
                                ),
                                Some(Request::Tip(chan)) => {
                                    chan.send(Err(core_client::Error::new(
                                        core_client::ErrorKind::Rpc,
                                        "unexpected response".to_string(),
                                    )))
                                    .unwrap();
                                    Either::A(future::ok(()))
                                }
                                None => Either::A(future::ok(())),
                            }
                            .and_then(move |_| {
                                sink.close_light_connection(lwid)
                                    .and_then(|x| future::ok((x, cc)))
                                    .map_err(|_| ())
                            }),
                        )
                    }
                    Command::Transaction(_, _) => V::A5(future::ok((sink, cc))),
                    Command::Request(request) => V::A6(
                        sink.new_light_connection()
                            .and_then(move |(lwcid, sink)| match request {
                                Request::Tip(t) => {
                                    cc.requests.insert(lwcid, Request::Tip(t));
                                    future::Either::A({
                                        sink.send(Message::GetBlockHeaders(
                                            lwcid,
                                            GetBlockHeaders {
                                                from: vec![],
                                                to: None,
                                            },
                                        ))
                                        .and_then(|sink| future::ok((sink, cc)))
                                    })
                                }
                                Request::Block(t, from, to) => {
                                    let from1 = from.clone();
                                    let to1 = to.clone();
                                    cc.requests.insert(lwcid, Request::Block(t, from1, to1));
                                    future::Either::B({
                                        sink.send(Message::GetBlocks(lwcid, GetBlocks { from, to }))
                                            .and_then(|sink| future::ok((sink, cc)))
                                    })
                                }
                            })
                            .map_err(|_| ()),
                    ),
                    Command::CloseConnection(lwcid) => V::A7({
                        match cc.requests.remove(&lwcid) {
                            Some(Request::Tip(chan)) => {
                                chan.send(Err(core_client::Error::new(
                                    core_client::ErrorKind::Rpc,
                                    "unexpected close",
                                )))
                                .unwrap();
                            }
                            Some(Request::Block(mut chan, _, _)) => {
                                chan.close().unwrap();
                            }
                            _ => (),
                        };
                        future::ok((sink, cc))
                    }),
                })
                .map_err(|_| ())
                .map(|_| ())
        });
    let cmds = commands.select(sink).map_err(|_err| ()).map(|_| ());

    stream.select(cmds).then(|_| Ok(()))
}
