
use std::{fmt};
use cardano::util::{hex};

const PROTOCOL_VERSION : u32 = 0x00000000;

pub type LightweightConnectionId = u32;

pub const LIGHT_ID_MIN : u32 = 1024;

#[derive(Debug)]
pub enum ControlHeader {
    CreateNewConnection = 0,
    CloseConnection = 1,
    CloseSocket = 2,
    CloseEndPoint = 3,
    ProbeSocket = 4,
    ProbeSocketAck = 5,
}

impl ControlHeader {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(ControlHeader::CreateNewConnection),
            1 => Some(ControlHeader::CloseConnection),
            2 => Some(ControlHeader::CloseSocket),
            3 => Some(ControlHeader::CloseEndPoint),
            4 => Some(ControlHeader::ProbeSocket),
            5 => Some(ControlHeader::ProbeSocketAck),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Control(ControlHeader, super::LightweightConnectionId),
    Data(super::LightweightConnectionId, u32),
}

pub type Nonce = u64;

#[derive(Debug, PartialEq)]
pub enum NodeControlHeader {
    Syn,
    Ack,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct NodeId([u8;9]);
impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}

const NODEID_SYN : u8 = 0x53; // 'S'
const NODEID_ACK : u8 = 0x41; // 'A'

impl AsRef<[u8]> for NodeId {
    fn as_ref(&self) -> &[u8] { &self.0 }
}

// use make_syn_nodeid or make_ack_nodeid
fn make_nodeid(header: NodeControlHeader, nonce: Nonce) -> NodeId {
    let mut v = [0;9];
    v[0] = match header {
                NodeControlHeader::Syn => NODEID_SYN,
                NodeControlHeader::Ack => NODEID_ACK,
    };
    v[1] = (nonce >> 56) as u8;
    v[2] = (nonce >> 48) as u8;
    v[3] = (nonce >> 40) as u8;
    v[4] = (nonce >> 32) as u8;
    v[5] = (nonce >> 24) as u8;
    v[6] = (nonce >> 16) as u8;
    v[7] = (nonce >> 8) as u8;
    v[8] = nonce as u8;
    NodeId(v)
}

impl NodeId {
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice[0] != NODEID_SYN && slice[0] != NODEID_ACK { return None }
        if slice.len() != 9 { return None }
        let mut buf = [0u8;9];
        buf.clone_from_slice(slice);
        Some(NodeId(buf))
    }

    pub fn make_syn(nonce: Nonce) -> Self {
        make_nodeid(NodeControlHeader::Syn, nonce)
    }

    pub fn make_ack(nonce: Nonce) -> Self {
        make_nodeid(NodeControlHeader::Ack, nonce)
    }

    pub fn get_control_header(&self) -> NodeControlHeader {
        if self.0[0] == NODEID_ACK { NodeControlHeader::Ack } else { NodeControlHeader::Syn }
    }

    pub fn is_syn(&self) -> bool {
        self.0[0] == NODEID_SYN
    }

    // check if a SYN nodeid match a specific ACK nodeid
    pub fn match_ack(&self, ack_nodeid: &NodeId) -> bool {
        assert!(self.0[0] == NODEID_SYN);
        ack_nodeid.0[0] == NODEID_ACK && self.0[1..9] == ack_nodeid.0[1..9]
    }

    // Given a ACK nodeid, get the equivalent SYN nodeid
    pub fn ack_to_syn(&self) -> Self {
        assert!(self.0[0] == NODEID_ACK);
        let mut nodeid = self.clone();
        nodeid.0[0] = NODEID_SYN;
        nodeid
    }

    // Given a SYN nodeid, get the equivalent ACK nodeid
    pub fn syn_to_ack(&self) -> Self {
        assert!(self.0[0] == NODEID_SYN);
        let mut nodeid = self.clone();
        nodeid.0[0] = NODEID_ACK;
        nodeid
    }
}

pub fn handshake(buf: &mut Vec<u8>) {
    let handshake_length = 0;
    append_u32(PROTOCOL_VERSION, buf);
    append_u32(handshake_length, buf);
    append_u32(0, buf); // ourEndPointId
    append_u32(0, buf); // send length 0
    //append_u32(0, buf); // ignored but should be handshake length
    //append_u32(0, buf); // ignored but should be handshake length
}

/// encode an int32
/*
fn append_i32(v: i32, buf: &mut Vec<u8>) {
    buf.push((v >> 24) as u8);
    buf.push((v >> 16) as u8);
    buf.push((v >> 8) as u8);
    buf.push(v as u8);
}
*/

fn append_u32(v: u32, buf: &mut Vec<u8>) {
    buf.push((v >> 24) as u8);
    buf.push((v >> 16) as u8);
    buf.push((v >> 8) as u8);
    buf.push(v as u8);
}

pub fn append_lightweight_data(cid: super::LightweightConnectionId, len: u32, buf: &mut Vec<u8>) {
    assert!(cid >= 1024);
    append_u32(cid, buf);
    append_u32(len, buf);
}

pub fn create_conn(cid: super::LightweightConnectionId, buf: &mut Vec<u8>) {
    append_u32(ControlHeader::CreateNewConnection as u32, buf);
    append_u32(cid, buf);
}

pub fn delete_conn(cid: super::LightweightConnectionId, buf: &mut Vec<u8>) {
    append_u32(ControlHeader::CloseConnection as u32, buf);
    append_u32(cid, buf);
}

pub fn append_with_length(dat: &[u8], buf: &mut Vec<u8>) {
    append_u32(dat.len() as u32, buf);
    buf.extend_from_slice(dat);
}
