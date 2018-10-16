use std::{ops::{Deref}, fmt, collections::{BTreeMap}};

use bytes::{BufMut, BytesMut};

use cbor_event::{self, se, de::{self, RawCbor}};
use cardano::{
    config::{ProtocolMagic},
    block::{self},
};

use super::{nt};

pub type MessageCode = u32;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HandlerSpec(u16);
impl HandlerSpec {
    pub fn new(c: u16) -> Self { HandlerSpec(c) }
}
impl fmt::Display for HandlerSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl se::Serialize for HandlerSpec {
    fn serialize<W>(&self, serializer: se::Serializer<W>) -> cbor_event::Result<se::Serializer<W>>
        where W: ::std::io::Write
    {
        serializer.write_array(cbor_event::Len::Len(2))?
            .write_unsigned_integer(0)?
            .write_tag(24)?
            .write_bytes(se::Serializer::new_vec().write_unsigned_integer(self.0 as u64)?.finalize())
    }
}
impl de::Deserialize for HandlerSpec {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(2, "HandlerSpec")?;
        let t = raw.unsigned_integer()?;
        if t != 0 {
            return Err(cbor_event::Error::CustomError(format!("Invalid value, expected 0, received {}", t)));
        }
        let tag = raw.tag()?;
        if tag != 24 {
            return Err(cbor_event::Error::CustomError(format!("Invalid tag, expected 24, received {}", tag)));
        }
        let v = RawCbor::from(&raw.bytes()?).unsigned_integer()? as u16;
        Ok(HandlerSpec(v))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HandlerSpecs(BTreeMap<MessageCode, HandlerSpec>);
impl HandlerSpecs {
    pub fn default_ins() -> Self {
        let mut bm = BTreeMap::new();
        bm.insert(MsgType::MsgHeaders as u32, HandlerSpec::new(MsgType::MsgGetHeaders as u16));
        HandlerSpecs(bm)
    }
    pub fn default_outs() -> Self {
        let mut bm = BTreeMap::new();
        bm.insert(MsgType::MsgGetHeaders as u32, HandlerSpec::new(MsgType::MsgHeaders as u16));
        bm.insert(MsgType::MsgGetBlocks as u32, HandlerSpec::new(MsgType::MsgBlock as u16));
        bm.insert(MsgType::MsgAnnounceTx as u32, HandlerSpec::new(MsgType::MsgTxMsgContents as u16));
        bm.insert(MsgType::MsgSubscribe1 as u32, HandlerSpec::new(0x00));
        bm.insert(MsgType::MsgStream as u32, HandlerSpec::new(MsgType::MsgStreamBlock as u16));
        HandlerSpecs(bm)
    }
}
impl se::Serialize for HandlerSpecs {
    fn serialize<W>(&self, serializer: se::Serializer<W>) -> cbor_event::Result<se::Serializer<W>>
        where W: ::std::io::Write
    {
        se::serialize_fixed_map(self.0.iter(), serializer)
    }
}
impl de::Deserialize for HandlerSpecs {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(HandlerSpecs(raw.deserialize()?))
    }
}
impl fmt::Display for HandlerSpecs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for kv in self.0.iter() {
            write!(f, "  * {} -> {}\n", kv.0, kv.1)?;
        }
        write!(f, "")
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Handshake {
    pub protocol_magic: ProtocolMagic,
    pub version: block::Version,
    pub in_handlers:  HandlerSpecs,
    pub out_handlers: HandlerSpecs
}

impl fmt::Display for Handshake {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "protocol magic: {:?}", self.protocol_magic)?;
        writeln!(f, "version: {}", self.version)?;
        writeln!(f, "in handlers:\n{}", self.in_handlers)?;
        writeln!(f, "out handlers:\n{}", self.out_handlers)
    }
}
impl Default for Handshake {
    fn default() -> Self {
        Handshake {
            protocol_magic: ProtocolMagic::default(),
            version:        block::Version::default(),
            in_handlers:    HandlerSpecs::default_ins(),
            out_handlers:   HandlerSpecs::default_outs(),
        }
    }
}
impl se::Serialize for Handshake {
    fn serialize<W>(&self, serializer: se::Serializer<W>) -> cbor_event::Result<se::Serializer<W>>
        where W: ::std::io::Write
    {
        serializer.write_array(cbor_event::Len::Len(4))?
            .serialize(&self.protocol_magic)?
            .serialize(&self.version)?
            .serialize(&self.in_handlers)?
            .serialize(&self.out_handlers)
    }
}
impl cbor_event::de::Deserialize for Handshake {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(4, "Handshake")?;
        let pm   = raw.deserialize()?;
        let v    = raw.deserialize()?;
        let ins  = raw.deserialize()?;
        let outs = raw.deserialize()?;

        Ok(Handshake { protocol_magic: pm, version: v, in_handlers: ins, out_handlers: outs })
    }
}

pub fn send_handshake(hs: &Handshake) -> Vec<u8> { cbor!(hs).unwrap() }

pub enum MsgType {
    MsgGetHeaders = 4,
    MsgHeaders = 5,
    MsgGetBlocks = 6,
    MsgBlock = 7,
    MsgSubscribe1 = 13,
    MsgSubscribe = 14,
    MsgStream = 15,
    MsgStreamBlock = 16,
    MsgAnnounceTx = 37, // == InvOrData key TxMsgContents
    MsgTxMsgContents = 94,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u64);
impl NodeId {
    pub fn next(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}
impl From<u64> for NodeId { fn from(v: u64) -> Self { NodeId(v) } }
impl Default for NodeId { fn default() -> Self { NodeId(0) } }
impl Deref for NodeId {
    type Target = u64;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Clone, Debug)]
pub enum Message {
    CreateLightWeightConnectionId(nt::LightWeightConnectionId),
    CreateNodeId(nt::LightWeightConnectionId, NodeId),
    AckNodeId(nt::LightWeightConnectionId, NodeId),
    Handshake(nt::LightWeightConnectionId, Handshake),
}
impl Message {
    pub fn to_nt_event(&self) -> nt::Event {
        use self::Message::*;
        use self::nt::{Event::{*}, ControlHeader::{*}};
        match self {
            CreateLightWeightConnectionId(lwcid) => Control(CreateNewConnection, *lwcid),
            CreateNodeId(lwcid, node_id) => {
                let mut bytes = BytesMut::with_capacity(9);
                bytes.put_u8(0x53);
                bytes.put_u64_be(**node_id);
                Data(*lwcid, bytes.freeze())
            },
            AckNodeId(lwcid, node_id) => {
                let mut bytes = BytesMut::with_capacity(9);
                bytes.put_u8(0x41);
                bytes.put_u64_be(**node_id);
                Data(*lwcid, bytes.freeze())
            },
            Handshake(lwcid, handshake) => {
                let bytes = cbor!(handshake).unwrap().into();
                Data(*lwcid, bytes)
            }
        }
    }

    pub fn expect_control(event: nt::Event) -> Result<Self, nt::Event> {
        use self::nt::{ControlHeader};

        let (ch, lwcid) = event.expect_control()?;
        Ok(match ch {
            ControlHeader::CreateNewConnection => Message::CreateLightWeightConnectionId(lwcid),
            ControlHeader::CloseConnection     => unimplemented!(),
            ControlHeader::CloseEndPoint       => unimplemented!(),
            ControlHeader::CloseSocket         => unimplemented!(),
            ControlHeader::ProbeSocket         => unimplemented!(),
            ControlHeader::ProbeSocketAck      => unimplemented!(),
        })
    }
}
