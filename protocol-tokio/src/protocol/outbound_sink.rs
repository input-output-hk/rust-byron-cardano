use super::{*};
use super::codec::{Message};

pub type Outbound = Message;

#[derive(Debug)]
pub enum OutboundError {
    IoError(io::Error),
    Unknown,
}
impl From<()> for OutboundError {
    fn from(e: ()) -> Self { OutboundError::Unknown }
}
impl From<io::Error> for OutboundError {
    fn from(e: io::Error) -> Self { OutboundError::IoError(e) }
}

pub struct OutboundSink<T> {
    sink:  SplitSink<nt::Connection<T>>,
    state: Arc<Mutex<ConnectionState>>,
}
impl<T> OutboundSink<T> {
    fn get_next_light_id(&mut self) -> nt::LightWeightConnectionId {
        self.state.lock().unwrap().get_next_light_id()
    }

    fn get_next_node_id(&mut self) -> NodeId {
        self.state.lock().unwrap().get_next_node_id()
    }
}

impl<T: AsyncWrite> OutboundSink<T> {
    pub fn new(sink: SplitSink<nt::Connection<T>>, state: Arc<Mutex<ConnectionState>>) -> Self {
        OutboundSink {
            sink,
            state,
        }
    }

    /// create a new light weight connection with the remote peer
    ///
    pub fn new_light_connection(mut self) -> impl Future<Item = (nt::LightWeightConnectionId, Self), Error = OutboundError>
    {
        let lwcid = self.get_next_light_id();
        let node_id = self.get_next_node_id();

        self.send(Message::CreateLightWeightConnectionId(lwcid))
            .and_then(move |connection| {
                connection.send(Message::CreateNodeId(lwcid, node_id))
            })
            .and_then(move |connection| {
                let light_weight_connection_state =
                    LightWeightConnectionState::new(lwcid)
                        .remote_initiated(false)
                        .with_node_id(node_id);

                connection.state.lock().unwrap().client_handles.insert(lwcid, light_weight_connection_state);

                future::ok((lwcid, connection))
            })
    }

    /// close a light connection that has been created with
    /// `new_light_connection`.
    ///
    pub fn close_light_connection(self, lwcid: nt::LightWeightConnectionId) -> impl Future<Item = Self, Error = OutboundError>
    {
        self.send(Message::CloseConnection(lwcid))
            .and_then(move |connection| {
                connection.state.lock().unwrap().client_handles.remove(&lwcid);
                future::ok(connection)
            })
    }

    /// this function it to acknowledge the creation of the NodeId on the remote
    /// client side
    pub fn ack_node_id(mut self, node_id: NodeId) -> impl Future<Item = Self, Error = OutboundError>
    {
        let tmp_lwcid = self.get_next_light_id();

        self.send(Message::CreateLightWeightConnectionId(tmp_lwcid))
            .and_then(move |connection| {
                connection.send(Message::AckNodeId(tmp_lwcid, node_id))
            })
            .and_then(move |connection| {
                connection.send(Message::CloseConnection(tmp_lwcid))
            })
    }
}

impl<T: AsyncWrite> Sink for OutboundSink<T> {
    type SinkItem  = Outbound;
    type SinkError = OutboundError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError>
    {
        self.sink.start_send(item.to_nt_event())
            .map_err(OutboundError::IoError)
            .map(|async| async.map(Message::from_nt_event))
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.poll_complete()
            .map_err(OutboundError::IoError)
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.close()
            .map_err(OutboundError::IoError)
    }
}
