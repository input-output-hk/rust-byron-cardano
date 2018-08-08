use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::{io, fmt, result};

use packet;
use packet::{Handshake};
use ntt;

use cbor_event::{self, de::{RawCbor}};

#[derive(Debug)]
pub enum Error {
    NttError(ntt::Error),
    IOError(io::Error),
    ByteEncodingError(cbor_event::Error),
    ServerCreatedLightIdTwice(LightId),
    UnsupportedControl(ntt::protocol::ControlHeader),
    NodeIdNotFound(ntt::protocol::NodeId),
    ClientIdNotFoundFromNodeId(ntt::protocol::NodeId, LightId),
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self { Error::ByteEncodingError(e) }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IOError(e) }
}
impl From<ntt::Error> for Error {
    fn from(e: ntt::Error) -> Self { Error::NttError(e) }
}

pub type Result<T> = result::Result<T, Error>;

/// Light ID create by the server or by the client
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct LightId(pub u32);
impl LightId {
    /// create a `LightId` from the given number
    ///
    /// identifier from 0 to 1023 are reserved.
    ///
    /// # Example
    ///
    /// ```
    /// use protocol::{LightId};
    /// let id = LightId::new(0x400);
    /// ```
    pub fn new(id: u32) -> Self {
        assert!(id >= 1024);
        LightId(id)
    }
    pub fn next(self) -> Self {
        LightId(self.0 + 1)
    }
}
impl fmt::Display for LightId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A client light connection will hold pending message to send or
/// awaiting to be read data
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct LightConnection {
    id: LightId,
    node_id: ntt::protocol::NodeId,
    received: Vec<Vec<u8>>,
    eos: bool,
}
impl LightConnection {
    pub fn new_with_nodeid(id: LightId, nonce: u64) -> Self {
        LightConnection {
            id: id,
            node_id: ntt::protocol::NodeId::make_syn(nonce),
            received: Vec::new(),
            eos: false,
        }
    }
    pub fn new_expecting_nodeid(id: LightId, node: ntt::protocol::NodeId) -> Self {
        LightConnection {
            id: id,
            node_id: node,
            received: Vec::new(),
            eos: false,
        }
    }

    pub fn get_id(&self) -> LightId { self.id }

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
        if self.received.len() > 0 { Some(self.received.remove(0)) } else { None }
    }

    /// add data to the received bucket
    fn add_to_receive(&mut self, bytes: &[u8]) {
        let mut v = Vec::new();
        v.extend_from_slice(bytes);
        self.received.push(v)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum ServerLightConnection {
    Establishing,
    Established(ntt::protocol::NodeId),
}

pub struct Connection<T> {
    ntt: ntt::Connection<T>,
    // this is a line of active connections open by the server/client
    // that have not been closed yet.
    server_cons: BTreeMap<LightId, ServerLightConnection>,
    client_cons: BTreeMap<LightId, LightConnection>,
    // this is for the server to map from its own nodeid to the client lightid
    map_to_client: BTreeMap<ntt::protocol::NodeId, LightId>,
    // potentialy the server close its connection before we have time
    // to process it on the client, so keep the buffer alive here
    //server_dones: BTreeMap<LightId, LightConnection>,
    //await_reply: BTreeMap<ntt::protocol::NodeId, >

    next_light_id: LightId
}

impl<T: Write+Read> Connection<T> {

    pub fn get_backend(&self) -> &T {
        self.ntt.get_backend()
    }

    // search for the next free LIGHT ID in the client connection map
    fn find_next_connection_id(&self) -> LightId {
        let mut x = LightId(ntt::LIGHT_ID_MIN);
        while self.client_cons.contains_key(&x) {
            x = x.next();
        }
        return x;
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
            next_light_id: LightId::new(0x401)
        }
    }

    pub fn handshake(&mut self, hs: &packet::Handshake) -> Result<()> {
        use ntt::protocol::{ControlHeader, Command};
        let lcid = self.find_next_connection_id();
        let lc = LightConnection::new_with_nodeid(lcid, self.ntt.get_nonce());

        /* create a connection, then send the handshake data, followed by the node id associated with this connection */
        self.ntt.create_light(lcid.0)?;
        self.send_bytes(lcid, &packet::send_handshake(hs))?;
        self.send_nodeid(lcid, &lc.node_id)?;

        self.client_cons.insert(lcid, lc);

        /* wait answer from server, which should be a new light connection creation,
         * followed by the handshake data and then the node id
         */
        let siv = match self.ntt.recv()? {
            Command::Control(ControlHeader::CreatedNewConnection, cid) => { LightId::new(cid) },
            _ => { unimplemented!() }
        };

        fn data_recv_on<T: Read+Write>(con: &mut Connection<T>, expected_id: LightId) -> Result<Vec<u8>> {
            match con.ntt.recv()? {
                ntt::protocol::Command::Data(cid, len) => {
                    if cid == expected_id.0 {
                        let bytes = con.ntt.recv_len(len)?;
                        Ok(bytes)
                    } else {
                        unimplemented!()
                    }
                }
                _ => { unimplemented!() }
            }
        };

        let server_bytes_hs = data_recv_on(self, siv)?;
        let _server_handshake : Handshake = RawCbor::from(&server_bytes_hs).deserialize()?;

        let server_bytes_nodeid = data_recv_on(self, siv)?;
        let server_nodeid = match ntt::protocol::NodeId::from_slice(&server_bytes_nodeid[..]) {
            None   => unimplemented!(),
            Some(nodeid) => nodeid,
        };

        // TODO compare server_nodeid and client_id

        let _scon = LightConnection::new_expecting_nodeid(siv, server_nodeid);
        self.server_cons.insert(siv, ServerLightConnection::Established(server_nodeid));

        Ok(())
    }

    pub fn new_light_connection(&mut self, id: LightId) -> Result<()> {
        self.ntt.create_light(id.0)?;

        let lc = LightConnection::new_with_nodeid(id, self.ntt.get_nonce());
        self.send_nodeid(id, &lc.node_id)?;
        self.client_cons.insert(id, lc);
        Ok(())
    }

    pub fn close_light_connection(&mut self, id: LightId) {
        self.client_cons.remove(&id);
        self.ntt.close_light(id.0).unwrap();
    }

    pub fn has_bytes_to_read_or_finish(&self, id: LightId) -> bool {
        match self.client_cons.get(&id) {
            None => false,
            Some(con) => con.pending_received() || con.eos,
        }
    }

    pub fn wait_msg(&mut self, id: LightId) -> Result<Vec<u8>> {
        while !self.has_bytes_to_read_or_finish(id) {
            self.process_messages()?;
        }

        match self.client_cons.get_mut(&id) {
            None => panic!("oops"),
            Some(ref mut con) => {
                match con.pop_received() {
                    None => panic!("oops 2"),
                    Some(yy) => Ok(yy),
                }
            },
        }
    }

    // same as wait_msg, except returns a vector of result
    pub fn wait_msg_eos(&mut self, id: LightId) -> Result<Vec<Vec<u8>>> {
        let mut r = Vec::new();
        loop {
            while !self.has_bytes_to_read_or_finish(id) {
                self.process_messages()?;
            }

            match self.client_cons.get_mut(&id) {
                None => panic!("oops"),
                Some(ref mut con) => {
                    match con.pop_received() {
                        None => { if con.eos { return Ok(r) } else { panic!("oops 2") } },
                        Some(yy) => r.push(yy),
                    }
                },
            }
        }
    }

    pub fn send_bytes(&mut self, id: LightId, bytes: &[u8]) -> Result<()> {
        self.ntt.light_send_data(id.0, bytes)?;
        Ok(())
    }

    pub fn send_nodeid(&mut self, id: LightId, nodeid: &ntt::protocol::NodeId) -> Result<()> {
        trace!("send NodeID {} associated to light id {}", nodeid, id);
        self.ntt.light_send_data(id.0, nodeid.as_ref())?;
        Ok(())
    }

    // TODO return some kind of opaque token
    pub fn send_bytes_ack(&mut self, id: LightId, bytes: &[u8]) -> Result<ntt::protocol::NodeId> {
        match self.client_cons.get(&id) {
            None => panic!("send bytes ack ERROR. connection doesn't exist"),
            Some(con) => {
                self.ntt.light_send_data(id.0, bytes)?;
                Ok(con.node_id)
            }
        }
    }

    // Process one message from the connection. This is one of two type:
    // a control message or a data message
    //
    // control message control light stream creation and closing
    // whereas data message are associated to a light connection
    pub fn process_messages(&mut self) -> Result<()> {
        use ntt::protocol::{ControlHeader, Command};
        match self.ntt.recv()? {
            Command::Control(ControlHeader::CloseConnection, cid) => {
                let id = LightId::new(cid);
                match self.server_cons.remove(&id) {
                    Some(ServerLightConnection::Establishing) => {
                        Ok(())
                    },
                    Some(ServerLightConnection::Established(v)) => {
                        match self.map_to_client.remove(&v) {
                            Some(lightid) => {
                                match self.client_cons.get_mut(&lightid) {
                                    None => {},
                                    Some (ref mut con) => {
                                        con.eos = true
                                    },
                                }
                            },
                            None          => {},
                        }
                        /*
                        if let Some(_) = v.received {
                            self.server_dones.insert(id, v);
                        }
                        */
                        Ok(())
                    },
                    None    => {
                        // BUG, server asked to close connection but connection doesn't exists in tree
                        // TODO, we might wanto to warn about this, but this is not an error.
                        Ok(())
                    },
                }
            },
            Command::Control(ControlHeader::CreatedNewConnection, cid) => {
                let id = LightId::new(cid);
                if let Some(_) = self.server_cons.get(&id) {
                    // TODO report this as an error to the logger
                    error!("light id created twice, {}", id);
                    Err(Error::ServerCreatedLightIdTwice(id))
                } else {
                    //let con = LightConnection::new_expecting_nodeid(id);
                    self.server_cons.insert(id, ServerLightConnection::Establishing);
                    Ok(())
                }
            },
            Command::Control(ch, cid) => {
                error!("LightId({}) Unsupported control `{:?}`", cid, ch);
                Err(Error::UnsupportedControl(ch))
            },
            ntt::protocol::Command::Data(server_id, len) => {
                let id = LightId::new(server_id);
                let v = match self.server_cons.get(&id) {
                    None      => None,
                    Some(slc) => Some(slc.clone())
                };
                match v {
                    // connection is established to a client side yet
                    // append the data to the receiving buffer
                    Some(ServerLightConnection::Established(nodeid)) => {
                        match self.map_to_client.get(&nodeid) {
                            None => Err(Error::NodeIdNotFound(nodeid)),
                            Some(client_id) => {
                                match self.client_cons.get_mut(client_id) {
                                    None => Err(Error::ClientIdNotFoundFromNodeId(nodeid, *client_id)),
                                    Some(con) => {
                                        let bytes = self.ntt.recv_len(len).unwrap();
                                        con.add_to_receive(&bytes);
                                        Ok(())
                                    }
                                }
                            },
                        }
                    },
                    // connection is not established to client side yet
                    // wait for the nodeid and try to match to an existing client
                    // if matching, then we remove the establishing server connection and
                    // add a established connection and setup the routing to the client
                    Some(ServerLightConnection::Establishing) => {
                        let bytes = self.ntt.recv_len(len).unwrap();
                        let nodeid = match ntt::protocol::NodeId::from_slice(&bytes[..]) {
                            None         => panic!("ERROR: expecting nodeid but receive stuff"),
                            Some(nodeid) => nodeid,
                        };

                        //let scon = LightConnection::new_expecting_nodeid(id, &nodeid);
                        self.server_cons.remove(&id);
                        self.server_cons.insert(id, ServerLightConnection::Established(nodeid.clone()));

                        match self.client_cons.iter().find(|&(_,v)| v.node_id.match_ack(&nodeid)) {
                            None        => { Ok(()) },
                            Some((z,_)) => {
                                self.map_to_client.insert(nodeid, *z);
                                Ok(())
                            }
                        }
                    },
                    None => {
                        warn!("LightId({}) does not exists but received data", server_id);
                        Ok(())
                    },
                }
            },
        }
    }
}

pub mod command {
    use std::io::{Read, Write};
    use super::{LightId, Connection};
    use cardano;
    use packet;
    use cbor_event::de::RawCbor;

    pub trait Command<W: Read+Write> {
        type Output;
        fn command(&self, connection: &mut Connection<W>, id: LightId) -> Result<(), &'static str>;
        fn result(&self, connection: &mut Connection<W>, id: LightId) -> Result<Self::Output, String>;

        fn initial(&self, connection: &mut Connection<W>) -> Result<LightId, &'static str> {
            let id = connection.get_free_light_id();
            trace!("creating light connection: {}", id);

            connection.new_light_connection(id).unwrap();
            Ok(id)
        }
        fn execute(&self, connection: &mut Connection<W>) -> Result<Self::Output, String> {
            let id = Command::initial(self, connection)?;

            Command::command(self, connection, id)?;
            let ret = Command::result(self, connection, id)?;

            Command::terminate(self, connection, id)?;

            Ok(ret)
        }
        fn terminate(&self, connection: &mut Connection<W>, id: LightId) -> Result<(), &'static str> {
            connection.close_light_connection(id);
            Ok(())
        }
    }

    #[derive(Debug)]
    pub struct GetBlockHeader {
        from: Vec<cardano::block::HeaderHash>,
        to: Option<cardano::block::HeaderHash>
    }
    impl GetBlockHeader {
        pub fn tip() -> Self { GetBlockHeader { from: vec![], to: None } }
        pub fn range(from: &[cardano::block::HeaderHash], to: cardano::block::HeaderHash) -> Self {
            let mut vec = Vec::new();
            for f in from.iter() {
                vec.push(f.clone());
            }
            GetBlockHeader { from: vec, to: Some(to) }
        }
    }

    impl<W> Command<W> for GetBlockHeader where W: Read+Write {
        type Output = cardano::block::RawBlockHeaderMultiple;
        fn command(&self, connection: &mut Connection<W>, id: LightId) -> Result<(), &'static str> {
            let (get_header_id, get_header_dat) = packet::send_msg_getheaders(&self.from[..], &self.to);
            connection.send_bytes(id, &[get_header_id]).unwrap();
            connection.send_bytes(id, &get_header_dat[..]).unwrap();
            Ok(())
        }
        fn result(&self, connection: &mut Connection<W>, id: LightId) -> Result<Self::Output, String> {
            // require the initial header
            let dat = connection.wait_msg(id).unwrap();
            match decode_sum_type(&dat) {
                None => Err(String::from("message block decoder failed with something unexpected")),
                Some((0, dat)) => {
                    let mut v = Vec::new();
                    v.extend_from_slice(dat);
                    Ok(cardano::block::RawBlockHeaderMultiple::from_dat(v))
                },
                Some((1, dat)) => {
                    Err(format!("server returned an error for GetHeaders: {}",
                                RawCbor::from(dat).text().unwrap()))
                },
                Some((_n, _dat)) => {
                    Err(String::from("message block decoder failed with something unexpected"))
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct GetBlock {
        from: cardano::block::HeaderHash,
        to:   cardano::block::HeaderHash
    }
    impl GetBlock {
        pub fn only(hh: &cardano::block::HeaderHash) -> Self { GetBlock::from(&hh.clone(), &hh.clone()) }
        pub fn from(from: &cardano::block::HeaderHash, to: &cardano::block::HeaderHash) -> Self { GetBlock { from: from.clone(), to: to.clone() } }
    }

    fn strip_msg_response(msg: &[u8]) -> Result<cardano::block::RawBlock, &'static str> {
        // here we unwrap the CBOR of Array(2, [uint(0), something]) to something
        match decode_sum_type(msg) {
            None => Err("message block decoder failed with something unexpected"),
            Some((sumval, dat)) => {
                if sumval == 0 {
                    let mut v = Vec::new();
                    v.extend_from_slice(dat);
                    Ok(cardano::block::RawBlock::from_dat(v))
                } else {
                    Err("message block decoder failed with something unexpected")
                }
            },
        }
    }

    impl<W> Command<W> for GetBlock where W: Read+Write {
        type Output = Vec<cardano::block::RawBlock>;
        fn command(&self, connection: &mut Connection<W>, id: LightId) -> Result<(), &'static str> {
            // require the initial header
            let (get_header_id, get_header_dat) = packet::send_msg_getblocks(&self.from, &self.to);
            connection.send_bytes(id, &[get_header_id]).unwrap();
            connection.send_bytes(id, &get_header_dat[..]).unwrap();
            Ok(())
        }

        fn result(&self, connection: &mut Connection<W>, id: LightId) -> Result<Self::Output, String> {
            let msg_response = connection.wait_msg_eos(id).unwrap();
            let mut msgs = Vec::new();
            for response in msg_response.iter() {
                let msg = strip_msg_response(&response[..])?;
                msgs.push(msg)
            }
            Ok(msgs)
        }
    }


    fn decode_sum_type(input: &[u8]) -> Option<(u8, &[u8])> {
        if input.len() > 2 && input[0] == 0x82 && input[1] < 23 {
            Some((input[1], &input[2..]))
        } else {
            None
        }
    }
}
