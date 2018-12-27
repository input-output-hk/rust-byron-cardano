use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::{error, fmt, io, result};

use ntt::{self, LightweightConnectionId};
use packet;
use packet::{Handshake, Message};

use cardano;

use cbor_event::{self, de::RawCbor, se, Deserialize};

#[derive(Debug)]
pub enum Error {
    NttError(ntt::Error),
    IOError(io::Error),
    ByteEncodingError(cbor_event::Error),
    ServerCreatedLightIdTwice(LightId),
    UnsupportedControl(ntt::protocol::ControlHeader),
    NodeIdNotFound(ntt::protocol::NodeId),
    ClientIdNotFoundFromNodeId(ntt::protocol::NodeId, LightId),
    UnexpectedResponse,
    NoResponse,
    ServerError(String),
    TransactionRejected,
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self {
        Error::ByteEncodingError(e)
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}
impl From<ntt::Error> for Error {
    fn from(e: ntt::Error) -> Self {
        Error::NttError(e)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NttError(_) => write!(f, "Protocol error"),
            Error::IOError(_) => write!(f, "I/O error"),
            Error::ByteEncodingError(_) => write!(f, "Bytes encoded in an unknown format"),
            Error::ServerCreatedLightIdTwice(lid) => {
                write!(f, "Same LightId created twice by peer {}", lid)
            }
            Error::UnsupportedControl(ctr) => write!(f, "Control unsupported here: `{:?}`", ctr),
            Error::NodeIdNotFound(nid) => write!(f, "NodeId `{}` not found", nid),
            Error::ClientIdNotFoundFromNodeId(nid, lid) => {
                write!(f, "ClientId `{}` not found in Node `{}`", lid, nid)
            }
            Error::UnexpectedResponse => write!(f, "Unexpected response from peer"),
            Error::NoResponse => write!(f, "No response from peer"),
            Error::ServerError(err) => write!(f, "Error from server: {}", err),
            Error::TransactionRejected => write!(f, "The transaction has been rejected by peer"),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::NttError(ref err) => Some(err),
            Error::IOError(ref err) => Some(err),
            Error::ByteEncodingError(ref err) => Some(err),
            Error::ServerCreatedLightIdTwice(_) => None,
            Error::UnsupportedControl(_) => None,
            Error::NodeIdNotFound(_) => None,
            Error::ClientIdNotFoundFromNodeId(_, _) => None,
            Error::UnexpectedResponse => None,
            Error::NoResponse => None,
            Error::ServerError(_) => None,
            Error::TransactionRejected => None,
        }
    }
}
pub type Result<T> = result::Result<T, Error>;

/// Light ID create by the server or by the client
pub type LightId = LightweightConnectionId;

/// A client light connection will hold pending message to send or
/// awaiting to be read data
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct LightConnection {
    id: LightId,
    node_id: Option<ntt::protocol::NodeId>,
    received: Vec<Vec<u8>>,
    eos: bool,
}
impl LightConnection {
    pub fn new(id: LightId) -> Self {
        LightConnection {
            id: id,
            node_id: None,
            received: Vec::new(),
            eos: false,
        }
    }

    pub fn new_with_nodeid(id: LightId, nonce: u64) -> Self {
        LightConnection {
            id: id,
            node_id: Some(ntt::protocol::NodeId::make_syn(nonce)),
            received: Vec::new(),
            eos: false,
        }
    }

    pub fn new_expecting_nodeid(id: LightId, node: ntt::protocol::NodeId) -> Self {
        LightConnection {
            id: id,
            node_id: Some(node),
            received: Vec::new(),
            eos: false,
        }
    }

    pub fn get_id(&self) -> LightId {
        self.id
    }

    /// tell if the `LightConnection` has some pending message to read
    pub fn pending_received(&self) -> bool {
        self.received.len() > 0
    }

    pub fn is_eos(&self) -> bool {
        self.eos
    }

    /// consume the eventual data to read
    ///
    /// to call only if you are ready to process the data
    pub fn pop_received(&mut self) -> Option<Vec<u8>> {
        if self.received.len() > 0 {
            Some(self.received.remove(0))
        } else {
            None
        }
    }

    /// add data to the received bucket
    fn add_to_receive(&mut self, bytes: &[u8]) {
        let mut v = Vec::new();
        v.extend_from_slice(bytes);
        self.received.push(v)
    }
}

pub struct Connection<T> {
    ntt: ntt::Connection<T>,
    // this is a line of active connections open by the server/client
    // that have not been closed yet. Note that light connections are
    // unidirectional, and the same Li ghtId can be used for a
    // (unrelated) connection in both directions.
    server_cons: BTreeMap<LightId, LightConnection>,
    client_cons: BTreeMap<LightId, LightConnection>,
    // this is for the server to map from its own nodeid to the client lightid
    map_to_client: BTreeMap<ntt::protocol::NodeId, LightId>,
    // potentialy the server close its connection before we have time
    // to process it on the client, so keep the buffer alive here
    //server_dones: BTreeMap<LightId, LightConnection>,
    //await_reply: BTreeMap<ntt::protocol::NodeId, >
    next_light_id: LightId,

    latest_tip: Option<cardano::block::BlockHeader>,
}

//const INITIAL_LIGHT_ID : LightweightConnectionId::initial();

impl<T: Write + Read> Connection<T> {
    pub fn get_backend(&self) -> &T {
        self.ntt.get_backend()
    }

    fn get_free_light_id(&mut self) -> LightId {
        let id = self.next_light_id;
        self.next_light_id = id.next();
        id
    }

    pub fn new(ntt: ntt::Connection<T>) -> Self {
        Connection {
            ntt: ntt,
            server_cons: BTreeMap::new(),
            client_cons: BTreeMap::new(),
            map_to_client: BTreeMap::new(),
            //server_dones: BTreeMap::new(),
            next_light_id: LightweightConnectionId::initial().next(),
            latest_tip: None,
        }
    }

    pub fn handshake(&mut self, hs: &packet::Handshake) -> Result<()> {
        use ntt::protocol::{Command, ControlHeader};
        let lcid = LightweightConnectionId::initial();
        let lc = LightConnection::new_with_nodeid(lcid, self.ntt.get_nonce());

        /* create a connection, then send the handshake data, followed by the node id associated with this connection */
        self.ntt.create_light(lcid)?;
        self.send_bytes(lcid, &packet::send_handshake(hs))?;
        self.send_nodeid(lcid, &lc.node_id.unwrap())?;

        debug!("my node = {}", lc.node_id.unwrap());

        self.client_cons.insert(lcid, lc);

        // FIXME: should use process_message() here.

        /* wait answer from server, which should be a new light connection creation,
         * followed by the handshake data and then the node id
         */
        let siv = match self.ntt.recv()? {
            Command::Control(ControlHeader::CreateNewConnection, cid) => cid,
            _ => unimplemented!(),
        };

        fn data_recv_on<T: Read + Write>(
            con: &mut Connection<T>,
            expected_id: LightId,
        ) -> Result<Vec<u8>> {
            match con.ntt.recv()? {
                ntt::protocol::Command::Data(cid, len) => {
                    if cid == expected_id {
                        let bytes = con.ntt.recv_len(len)?;
                        Ok(bytes)
                    } else {
                        unimplemented!()
                    }
                }
                _ => unimplemented!(),
            }
        };

        info!("creating initial light connection {}", lcid);
        let server_bytes_hs = data_recv_on(self, siv)?;
        let _server_handshake: Handshake = RawCbor::from(&server_bytes_hs).deserialize()?;

        let server_bytes_nodeid = data_recv_on(self, siv)?;
        let server_nodeid = match ntt::protocol::NodeId::from_slice(&server_bytes_nodeid[..]) {
            None => unimplemented!(),
            Some(nodeid) => nodeid,
        };

        // TODO compare server_nodeid and client_id

        self.server_cons.insert(
            siv,
            LightConnection::new_expecting_nodeid(siv, server_nodeid),
        );

        Ok(())
    }

    pub fn new_light_connection(&mut self) -> Result<LightId> {
        let id = self.get_free_light_id();
        trace!("creating light connection: {}", id);

        self.ntt.create_light(id)?;

        let lc = LightConnection::new_with_nodeid(id, self.ntt.get_nonce());
        self.send_nodeid(id, &lc.node_id.unwrap())?;
        self.client_cons.insert(id, lc);
        Ok(id)
    }

    pub fn close_light_connection(&mut self, id: LightId) {
        self.client_cons.remove(&id);
        self.ntt.close_light(id).unwrap();
    }

    pub fn has_bytes_to_read_or_finish(&self, id: LightId) -> bool {
        match self.client_cons.get(&id) {
            None => false,
            Some(con) => con.pending_received() || con.eos,
        }
    }

    pub fn wait_msg(&mut self, id: LightId) -> Result<Vec<u8>> {
        while !self.has_bytes_to_read_or_finish(id) {
            self.process_message()?;
        }

        match self.client_cons.get_mut(&id) {
            None => panic!("oops"),
            Some(ref mut con) => match con.pop_received() {
                None => Err(Error::NoResponse),
                Some(yy) => Ok(yy),
            },
        }
    }

    // same as wait_msg, except returns a vector of result
    pub fn wait_msg_eos(&mut self, id: LightId) -> Result<Vec<Vec<u8>>> {
        let mut r = Vec::new();
        loop {
            while !self.has_bytes_to_read_or_finish(id) {
                self.process_message()?;
            }

            match self.client_cons.get_mut(&id) {
                None => panic!("oops"),
                Some(ref mut con) => match con.pop_received() {
                    None => {
                        if con.eos {
                            return Ok(r);
                        } else {
                            panic!("oops 2")
                        }
                    }
                    Some(yy) => r.push(yy),
                },
            }
        }
    }

    pub fn send_bytes(&mut self, id: LightId, bytes: &[u8]) -> Result<()> {
        self.ntt.light_send_data(id, bytes)?;
        Ok(())
    }

    pub fn send_message(&mut self, id: LightId, msg: &Message) -> Result<()> {
        let mut v = vec![];
        v.extend(&se::Serializer::new_vec().serialize(&msg.0)?.finalize());
        v.extend(&msg.1[..]);
        self.send_bytes(id, &v[..])?;
        Ok(())
    }

    pub fn send_nodeid(&mut self, id: LightId, nodeid: &ntt::protocol::NodeId) -> Result<()> {
        trace!("send NodeID {} associated to light id {}", nodeid, id);
        self.ntt.light_send_data(id, nodeid.as_ref())?;
        Ok(())
    }

    // TODO return some kind of opaque token
    pub fn send_bytes_ack(&mut self, id: LightId, bytes: &[u8]) -> Result<ntt::protocol::NodeId> {
        match self.client_cons.get(&id) {
            None => panic!("send bytes ack ERROR. connection doesn't exist"),
            Some(con) => {
                self.ntt.light_send_data(id, bytes)?;
                Ok(con.node_id.unwrap())
            }
        }
    }

    // Process one message from the connection. This is one of two type:
    // a control message or a data message
    //
    // control message control light stream creation and closing
    // whereas data message are associated to a light connection
    pub fn process_message(&mut self) -> Result<()> {
        use ntt::protocol::{Command, ControlHeader};
        match self.ntt.recv()? {
            Command::Control(ControlHeader::CloseConnection, id) => {
                debug!("received close of light connection {}", id);
                match &self.server_cons.remove(&id) {
                    Some(LightConnection { node_id: None, .. }) => Ok(()),
                    Some(LightConnection {
                        node_id: Some(node_id),
                        received,
                        ..
                    }) if node_id.is_syn() => {
                        let mut r = Vec::new();
                        for s in received {
                            r.extend(s.iter());
                        }
                        if r.len() > 0 {
                            self.process_async_message(r[0], &r[1..]);
                        }
                        Ok(())
                    }
                    Some(LightConnection {
                        node_id: Some(node_id),
                        ..
                    }) => {
                        match self.map_to_client.remove(&node_id) {
                            Some(lightid) => match self.client_cons.get_mut(&lightid) {
                                None => {}
                                Some(ref mut con) => con.eos = true,
                            },
                            None => {}
                        }
                        /*
                        if let Some(_) = v.received {
                            self.server_dones.insert(id, v);
                        }
                        */
                        Ok(())
                    }
                    None => {
                        // BUG, server asked to close connection but connection doesn't exists in tree
                        // TODO, we might wanto to warn about this, but this is not an error.
                        Ok(())
                    }
                }
            }
            Command::Control(ControlHeader::CreateNewConnection, id) => {
                if let Some(_) = self.server_cons.get(&id) {
                    // TODO report this as an error to the logger
                    error!("light id created twice, {}", id);
                    Err(Error::ServerCreatedLightIdTwice(id))
                } else {
                    self.server_cons.insert(id, LightConnection::new(id));
                    Ok(())
                }
            }
            Command::Control(ch, cid) => {
                error!("LightId({}) Unsupported control `{:?}`", cid, ch);
                Err(Error::UnsupportedControl(ch))
            }
            Command::Data(id, len) => {
                let bytes = self.ntt.recv_len(len).unwrap();
                match self.server_cons.get_mut(&id) {
                    // connection is established to a client side yet
                    // append the data to the receiving buffer
                    Some(
                        scon @ LightConnection {
                            node_id: Some(_), ..
                        },
                    ) => {
                        let node_id = scon.node_id.unwrap();
                        if node_id.is_syn() {
                            // Data on a server-initiated connection.
                            scon.add_to_receive(&bytes);
                            Ok(())
                        } else {
                            // Response to a client-initiated connection.
                            match self.map_to_client.get(&node_id) {
                                None => Err(Error::NodeIdNotFound(node_id)),
                                Some(client_id) => match self.client_cons.get_mut(client_id) {
                                    None => {
                                        Err(Error::ClientIdNotFoundFromNodeId(node_id, *client_id))
                                    }
                                    Some(con) => {
                                        con.add_to_receive(&bytes);
                                        Ok(())
                                    }
                                },
                            }
                        }
                    }
                    // connection is not established to client side yet
                    // wait for the nodeid and try to match to an existing client
                    // if matching, then we remove the establishing server connection and
                    // add a established connection and setup the routing to the client
                    Some(scon @ LightConnection { node_id: None, .. }) => {
                        let nodeid = match ntt::protocol::NodeId::from_slice(&bytes[..]) {
                            None => panic!("ERROR: expecting nodeid but received data"),
                            Some(nodeid) => nodeid,
                        };

                        scon.node_id = Some(nodeid);

                        if nodeid.is_syn() {
                            // The server opened a connection, so we
                            // have to ACK it on a separate, temporary
                            // connection.
                            info!("new async light connection {} from node {}", id, nodeid);
                            //let ack_conn_id = self.get_free_light_id(); // FIXME: mutable borrow of self
                            let ack_conn_id = self.next_light_id;
                            self.next_light_id = id.next();
                            self.ntt.create_light(ack_conn_id)?;
                            let ack = &nodeid.syn_to_ack();
                            debug!("sending ack {} on {}", ack, ack_conn_id);
                            //self.send_nodeid(ack_conn_id, ack)?; // FIXME: mutable borrow of self
                            self.ntt.light_send_data(ack_conn_id, ack.as_ref())?;
                            self.ntt.close_light(ack_conn_id).unwrap();
                        } else {
                            // This is an ACK, so it should correspond
                            // to a SYN sent by us.
                            match self
                                .client_cons
                                .iter()
                                .find(|&(_, v)| v.node_id.unwrap().match_ack(&nodeid))
                            {
                                None => {
                                    info!("server sent unexpected ACK {}", nodeid);
                                }
                                Some((z, _)) => {
                                    self.map_to_client.insert(nodeid, *z);
                                }
                            }
                        }

                        Ok(())
                    }
                    None => {
                        warn!("LightId({}) does not exist but received data", id);
                        Ok(())
                    }
                }
            }
        }
    }

    pub fn subscribe(&mut self) -> Result<()> {
        let id = LightId::initial();
        info!("subscribing on light connection {}", id);

        // FIXME: use keep-alive?
        self.send_message(id, &packet::send_msg_subscribe(false))?;

        Ok(())
    }

    // Process a message received from a peer via the subscription
    // mechanism.
    pub fn process_async_message(&mut self, msg_type: u8, msg: &[u8]) {
        if msg_type == packet::MsgType::MsgHeaders as u8 {
            self.process_async_headers(msg).unwrap(); // FIXME
        } else {
            warn!("Received unknown message type {:?} from peer", msg_type);
        }
    }

    // Process a 'Headers' message.
    pub fn process_async_headers(&mut self, msg: &[u8]) -> Result<()> {
        let mut headers = cardano::block::BlockHeaders::deserialize(&mut RawCbor::from(msg))?;

        info!("received {} asynchronous headers", headers.len());

        if let Some(latest_test) = headers.pop() {
            self.latest_tip = Some(latest_test);
        }

        Ok(())
    }

    pub fn get_latest_tip(&self) -> Option<cardano::block::BlockHeader> {
        self.latest_tip.clone()
    }
}

pub mod command {
    use super::{Connection, Error, LightId, Result};
    use cardano::{self, tx};
    use cbor_event::{self, de::RawCbor, se};
    use packet;
    use std::io::{Read, Write};

    pub trait Command<W: Read + Write> {
        type Output;
        fn command(&self, connection: &mut Connection<W>, id: LightId) -> Result<()>;
        fn result(&self, connection: &mut Connection<W>, id: LightId) -> Result<Self::Output>;

        fn initial(&self, connection: &mut Connection<W>) -> Result<LightId> {
            // FIXME: ensure that close_light_connection is always called.
            Ok(connection.new_light_connection()?)
        }
        fn execute(&self, connection: &mut Connection<W>) -> Result<Self::Output> {
            let id = Command::initial(self, connection)?;

            Command::command(self, connection, id)?;
            let ret = Command::result(self, connection, id)?;

            Command::terminate(self, connection, id)?;

            Ok(ret)
        }
        fn terminate(&self, connection: &mut Connection<W>, id: LightId) -> Result<()> {
            connection.close_light_connection(id);
            Ok(())
        }
    }

    pub fn stream_blocks<W: Read + Write, F>(
        connection: &mut Connection<W>,
        from: &[cardano::block::HeaderHash],
        to: cardano::block::HeaderHash,
        got_block: &mut F,
    ) -> Result<()>
    where
        F: FnMut(cardano::block::RawBlock) -> Result<()>,
    {
        let id = connection.new_light_connection()?;

        let window_size = 65536;

        connection.send_message(
            id,
            &packet::send_msg_stream_start(&from[..], &to, window_size),
        )?;

        let mut window_left = window_size;

        loop {
            let msg = connection.wait_msg(id)?;

            let mut msg = RawCbor::from(&msg);
            match cardano::cbor::hs::util::decode_sum_type(&mut msg)? {
                0 => {
                    // FIXME: we should at least check that we
                    // received valid CBOR.
                    let rblk = cardano::block::RawBlock::from_dat(msg.as_ref().to_vec());
                    got_block(rblk)?;
                }
                1 => {
                    return Err(Error::ServerError(msg.text()?));
                }
                2 => {
                    break;
                }
                _ => {
                    return Err(Error::UnexpectedResponse);
                }
            }

            // The peer will stop sending blocks when the window size
            // reaches zero. So periodically reset the window size by
            // sending MsgUpdate.
            // TODO: we may want to update the window size dynamically.
            window_left -= 1;
            if window_left < window_size / 2 {
                window_left = window_size;
                // Note: we don't prepend MsgStream here.
                connection.send_bytes(id, &packet::send_msg_stream_update(window_left).1)?;
            }
        }

        // FIXME
        connection.close_light_connection(id);
        Ok(())
    }

    #[derive(Debug)]
    pub struct GetBlockHeader {
        from: Vec<cardano::block::HeaderHash>,
        to: Option<cardano::block::HeaderHash>,
    }
    impl GetBlockHeader {
        pub fn tip() -> Self {
            GetBlockHeader {
                from: vec![],
                to: None,
            }
        }
        pub fn range(from: &[cardano::block::HeaderHash], to: cardano::block::HeaderHash) -> Self {
            GetBlockHeader {
                from: from.clone().to_vec(),
                to: Some(to),
            }
        }
    }

    impl<W> Command<W> for GetBlockHeader
    where
        W: Read + Write,
    {
        type Output = cardano::block::RawBlockHeaderMultiple;
        fn command(&self, connection: &mut Connection<W>, id: LightId) -> Result<()> {
            connection.send_message(id, &packet::send_msg_getheaders(&self.from[..], &self.to))?;
            Ok(())
        }
        fn result(&self, connection: &mut Connection<W>, id: LightId) -> Result<Self::Output> {
            // require the initial header
            let dat = connection.wait_msg(id)?;
            match decode_sum_type(&dat) {
                None => Err(Error::UnexpectedResponse),
                Some((0, dat)) => {
                    let mut v = Vec::new();
                    v.extend_from_slice(dat);
                    Ok(cardano::block::RawBlockHeaderMultiple::from_dat(v))
                }
                Some((1, dat)) => Err(Error::ServerError(RawCbor::from(dat).text()?)),
                Some((_n, _dat)) => Err(Error::UnexpectedResponse),
            }
        }
    }

    #[derive(Debug)]
    pub struct GetBlock {
        from: cardano::block::HeaderHash,
        to: cardano::block::HeaderHash,
    }
    impl GetBlock {
        pub fn only(hh: &cardano::block::HeaderHash) -> Self {
            GetBlock::from(&hh.clone(), &hh.clone())
        }
        pub fn from(from: &cardano::block::HeaderHash, to: &cardano::block::HeaderHash) -> Self {
            GetBlock {
                from: from.clone(),
                to: to.clone(),
            }
        }
    }

    fn strip_msg_response(msg: &[u8]) -> Result<cardano::block::RawBlock> {
        // here we unwrap the CBOR of Array(2, [uint(0), something]) to something
        match decode_sum_type(msg) {
            None => Err(Error::UnexpectedResponse),
            Some((0, dat)) => {
                let mut v = Vec::new();
                v.extend_from_slice(dat);
                Ok(cardano::block::RawBlock::from_dat(v))
            }
            Some((1, dat)) => Err(Error::ServerError(RawCbor::from(dat).text()?)),
            Some((_n, _dat)) => Err(Error::UnexpectedResponse),
        }
    }

    impl<W> Command<W> for GetBlock
    where
        W: Read + Write,
    {
        type Output = Vec<cardano::block::RawBlock>;
        fn command(&self, connection: &mut Connection<W>, id: LightId) -> Result<()> {
            // require the initial header
            connection.send_message(id, &packet::send_msg_getblocks(&self.from, &self.to))?;
            Ok(())
        }

        fn result(&self, connection: &mut Connection<W>, id: LightId) -> Result<Self::Output> {
            let msg_response = connection.wait_msg_eos(id)?;
            let mut msgs = Vec::new();
            for response in msg_response.iter() {
                let msg = strip_msg_response(&response[..])?;
                msgs.push(msg)
            }
            Ok(msgs)
        }
    }

    // FIXME: use cardano::decode_sum_type().
    fn decode_sum_type(input: &[u8]) -> Option<(u8, &[u8])> {
        if input.len() > 2 && input[0] == 0x82 && input[1] < 23 {
            Some((input[1], &input[2..]))
        } else {
            None
        }
    }

    #[derive(Debug)]
    pub struct SendTx(tx::TxAux);
    impl SendTx {
        pub fn new(tx: tx::TxAux) -> Self {
            SendTx(tx)
        }
    }
    impl<W> Command<W> for SendTx
    where
        W: Read + Write,
    {
        type Output = ();

        fn command(&self, connection: &mut Connection<W>, id: LightId) -> Result<()> {
            connection.send_message(id, &packet::send_msg_announcetx(&self.0.tx.id()))?;
            Ok(())
        }

        fn result(&self, connection: &mut Connection<W>, id: LightId) -> Result<Self::Output> {
            let dat = connection.wait_msg(id)?;
            match decode_sum_type(&dat) {
                None => Err(Error::UnexpectedResponse),
                Some((0, dat)) => {
                    let mut raw = RawCbor::from(dat);
                    if raw.array()? != cbor_event::Len::Len(1) {
                        return Err(Error::TransactionRejected);
                    }
                    let txid: tx::TxId = raw.deserialize()?;
                    assert_eq!(txid, self.0.tx.id());

                    // We now have to send the TxAux on the same connection.
                    let msg = se::Serializer::new_vec()
                        .write_array(cbor_event::Len::Len(2))?
                        .serialize(&1u8)? // == Right constructor of InvOrData (i.e. DataMsg)
                        .serialize(&self.0)?
                        .finalize();
                    connection.send_bytes(id, &msg[..])?;

                    // Receive the ResMsg data type.
                    let dat = connection.wait_msg(id)?;
                    let mut raw = RawCbor::from(&dat);
                    if raw.array()? != cbor_event::Len::Len(2) {
                        return Err(Error::UnexpectedResponse);
                    }
                    if raw.unsigned_integer()? != 1 {
                        return Err(Error::UnexpectedResponse);
                    }
                    let arr = raw.array()?;
                    if arr != cbor_event::Len::Len(2) {
                        return Err(Error::UnexpectedResponse);
                    }
                    let txid: tx::TxId = raw.deserialize()?;
                    assert_eq!(txid, self.0.tx.id());
                    let result = raw.bool()?;
                    if !result {
                        return Err(Error::TransactionRejected);
                    }

                    Ok(())
                }
                Some((1, dat)) => Err(Error::ServerError(RawCbor::from(dat).text()?)),
                Some((_n, _dat)) => Err(Error::UnexpectedResponse),
            }
        }
    }
}
