use std::{
    collections::BTreeMap,
    fmt,
    io::{BufRead, Cursor, Write},
};

use cardano::{block, config::ProtocolMagic};
use cbor_event::{
    self,
    de::{self, Deserializer},
    se::{self, Serializer},
};

use super::{MessageCode, MessageType};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HandlerSpec(u16);
impl HandlerSpec {
    pub fn new(c: u16) -> Self {
        HandlerSpec(c)
    }
}
impl fmt::Display for HandlerSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl se::Serialize for HandlerSpec {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(2))?
            .write_unsigned_integer(0)?
            .write_tag(24)?
            .write_bytes({
                let mut se = se::Serializer::new_vec();
                se.write_unsigned_integer(self.0 as u64)?;
                se.finalize()
            })
    }
}
impl de::Deserialize for HandlerSpec {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(2, "HandlerSpec")?;
        let t = raw.unsigned_integer()?;
        if t != 0 {
            return Err(cbor_event::Error::CustomError(format!(
                "Invalid value, expected 0, received {}",
                t
            )));
        }
        let tag = raw.tag()?;
        if tag != 24 {
            return Err(cbor_event::Error::CustomError(format!(
                "Invalid tag, expected 24, received {}",
                tag
            )));
        }
        let mut inner = Deserializer::from(Cursor::new(raw.bytes()?));
        let v = inner.unsigned_integer()? as u16;
        Ok(HandlerSpec(v))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HandlerSpecs(BTreeMap<MessageCode, HandlerSpec>);
impl HandlerSpecs {
    pub fn default_ins() -> Self {
        let mut bm = BTreeMap::new();
        bm.insert(
            MessageType::MsgHeaders as u32,
            HandlerSpec::new(MessageType::MsgGetHeaders as u16),
        );
        HandlerSpecs(bm)
    }
    pub fn default_outs() -> Self {
        let mut bm = BTreeMap::new();
        bm.insert(
            MessageType::MsgGetHeaders as u32,
            HandlerSpec::new(MessageType::MsgHeaders as u16),
        );
        bm.insert(
            MessageType::MsgGetBlocks as u32,
            HandlerSpec::new(MessageType::MsgBlock as u16),
        );
        bm.insert(
            MessageType::MsgAnnounceTx as u32,
            HandlerSpec::new(MessageType::MsgTxMsgContents as u16),
        );
        bm.insert(MessageType::MsgSubscribe1 as u32, HandlerSpec::new(0x00));
        bm.insert(
            MessageType::MsgStream as u32,
            HandlerSpec::new(MessageType::MsgStreamBlock as u16),
        );
        HandlerSpecs(bm)
    }
}
impl se::Serialize for HandlerSpecs {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        se::serialize_fixed_map(self.0.iter(), serializer)
    }
}
impl de::Deserialize for HandlerSpecs {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
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
    pub in_handlers: HandlerSpecs,
    pub out_handlers: HandlerSpecs,
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
            version: block::Version::default(),
            in_handlers: HandlerSpecs::default_ins(),
            out_handlers: HandlerSpecs::default_outs(),
        }
    }
}
impl se::Serialize for Handshake {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(4))?
            .serialize(&self.protocol_magic)?
            .serialize(&self.version)?
            .serialize(&self.in_handlers)?
            .serialize(&self.out_handlers)
    }
}
impl cbor_event::de::Deserialize for Handshake {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(4, "Handshake")?;
        let pm = raw.deserialize()?;
        let v = raw.deserialize()?;
        let ins = raw.deserialize()?;
        let outs = raw.deserialize()?;

        Ok(Handshake {
            protocol_magic: pm,
            version: v,
            in_handlers: ins,
            out_handlers: outs,
        })
    }
}
