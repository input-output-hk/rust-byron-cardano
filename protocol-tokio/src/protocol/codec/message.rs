use std::{ops::{Deref}};
use bytes::{BufMut, BytesMut, Bytes};

use cbor_event::{self, se, de::{self, RawCbor}};
use cardano::block::{self, HeaderHash};

use super::{NodeId};
use super::super::{nt};

pub type MessageCode = u32;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MessageType {
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
impl MessageType {
    #[inline]
    fn encode_with<T>(&self, cbor: &T) -> Bytes
        where T: se::Serialize
    {
        let bytes = se::Serializer::new_vec();
        let bytes = bytes.serialize(self).unwrap()
                .serialize(cbor).unwrap()
                .finalize();
        bytes.into()
    }
}
impl se::Serialize for MessageType {
    fn serialize<W>(&self, serializer: se::Serializer<W>) -> cbor_event::Result<se::Serializer<W>>
        where W: ::std::io::Write
    {
        serializer.serialize(&(*self as u32))
    }
}
impl de::Deserialize for MessageType {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let v = raw.unsigned_integer()? as u32;
        match v {
            4  => Ok(MessageType::MsgGetHeaders),
            5  => Ok(MessageType::MsgHeaders),
            6  => Ok(MessageType::MsgGetBlocks),
            7  => Ok(MessageType::MsgBlock),
            13 => Ok(MessageType::MsgSubscribe1),
            14 => Ok(MessageType::MsgSubscribe),
            15 => Ok(MessageType::MsgStream),
            16 => Ok(MessageType::MsgStreamBlock),
            37 => Ok(MessageType::MsgAnnounceTx),
            93 => Ok(MessageType::MsgTxMsgContents),
            v  => return Err(cbor_event::Error::CustomError(format!("Unsupported message type: {:20x}", v))),
        }
    }
}


#[derive(Clone, Debug)]
pub enum Message {
    CreateLightWeightConnectionId(nt::LightWeightConnectionId),
    CloseConnection(nt::LightWeightConnectionId),
    CloseEndPoint(nt::LightWeightConnectionId),
    CloseSocket(nt::LightWeightConnectionId),
    ProbeSocket(nt::LightWeightConnectionId),
    ProbeSocketAck(nt::LightWeightConnectionId),
    CreateNodeId(nt::LightWeightConnectionId, NodeId),
    AckNodeId(nt::LightWeightConnectionId, NodeId),

    GetBlockHeaders(nt::LightWeightConnectionId, GetBlockHeaders),
    BlockHeaders(nt::LightWeightConnectionId, Response<BlockHeaders, String>),
    Bytes(nt::LightWeightConnectionId, Bytes),
}
impl Message {
    pub fn to_nt_event(self) -> nt::Event {
        use self::nt::{Event::{*}, ControlHeader::{*}};
        match self {
            Message::CreateLightWeightConnectionId(lwcid) => Control(CreateNewConnection, lwcid),
            Message::CloseConnection(lwcid) => Control(CloseConnection, lwcid),
            Message::CloseEndPoint(lwcid)   => Control(CloseEndPoint, lwcid),
            Message::CloseSocket(lwcid)     => Control(CloseSocket, lwcid),
            Message::ProbeSocket(lwcid)     => Control(ProbeSocket, lwcid),
            Message::ProbeSocketAck(lwcid)  => Control(ProbeSocketAck, lwcid),
            Message::CreateNodeId(lwcid, node_id) => {
                let mut bytes = BytesMut::with_capacity(9);
                bytes.put_u8(0x53);
                bytes.put_u64_be(*node_id);
                Data(lwcid, bytes.freeze())
            },
            Message::AckNodeId(lwcid, node_id) => {
                let mut bytes = BytesMut::with_capacity(9);
                bytes.put_u8(0x41);
                bytes.put_u64_be(*node_id);
                Data(lwcid, bytes.freeze())
            },
            Message::GetBlockHeaders(lwcid, gbh) => {
                Data(lwcid, MessageType::MsgGetHeaders.encode_with(&gbh))
            },
            Message::BlockHeaders(lwcid, bh) => {
                Data(lwcid, MessageType::MsgHeaders.encode_with(&bh))
            },
            Message::Bytes(lwcid, bytes) => {
                Data(lwcid, bytes)
            }
        }
    }

    pub fn from_nt_event(event: nt::Event) -> Self {
        Message::expect_control(event)
            .or_else(Message::expect_bytes)
            .expect("If this was not a control it was a data related message")
    }

    pub fn expect_control(event: nt::Event) -> Result<Self, nt::Event> {
        use self::nt::{ControlHeader};

        let (ch, lwcid) = event.expect_control()?;
        Ok(match ch {
            ControlHeader::CreateNewConnection => Message::CreateLightWeightConnectionId(lwcid),
            ControlHeader::CloseConnection     => Message::CloseConnection(lwcid),
            ControlHeader::CloseEndPoint       => Message::CloseEndPoint(lwcid),
            ControlHeader::CloseSocket         => Message::CloseSocket(lwcid),
            ControlHeader::ProbeSocket         => Message::ProbeSocket(lwcid),
            ControlHeader::ProbeSocketAck      => Message::ProbeSocketAck(lwcid),
        })
    }

    pub fn expect_bytes(event: nt::Event) -> Result<Self, nt::Event> {
        let (lwcid, bytes) = event.expect_data()?;
        if let Some(msg) = decode_node_ack_or_syn(lwcid, &bytes) {
            return Ok(msg);
        }

        let mut cbor = de::RawCbor::from(bytes.deref());
        let msg_type : MessageType = cbor.deserialize().unwrap();
        match msg_type {
            MessageType::MsgGetHeaders => {
                Ok(Message::GetBlockHeaders(lwcid, cbor.deserialize_complete().unwrap()))
            },
            MessageType::MsgHeaders => {
                Ok(Message::BlockHeaders(lwcid, cbor.deserialize_complete().unwrap()))
            },
            _ => unimplemented!()
        }
    }
}

fn decode_node_ack_or_syn(lwcid: nt::LightWeightConnectionId, bytes: &Bytes) -> Option<Message> {
    use bytes::{IntoBuf, Buf};
    let mut buf = bytes.into_buf();
    let key = buf.get_u8();
    let v   = buf.get_u64_be();
    match key {
        0x53 => { Some(Message::CreateNodeId(lwcid, NodeId::from(v))) },
        0x41 => { Some(Message::AckNodeId(lwcid, NodeId::from(v))) },
        _    => { None },
    }

}

#[derive(Clone, Debug)]
pub enum Response<T, E> {
    Ok(T),
    Err(E),
}
impl<T: se::Serialize, E: se::Serialize> se::Serialize for Response<T, E> {
    fn serialize<W>(&self, serializer: se::Serializer<W>) -> cbor_event::Result<se::Serializer<W>>
        where W: ::std::io::Write
    {
        let serializer = serializer.write_array(cbor_event::Len::Len(2))?;
        match self {
            &Response::Ok(ref t)  => {
                serializer.serialize(&0u64)?.serialize(t)
            },
            &Response::Err(ref e) => {
                serializer.serialize(&1u64)?.serialize(e)
            }
        }
    }
}
impl<T: de::Deserialize, E: de::Deserialize> de::Deserialize for Response<T, E> {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(2, "protocol::Response")?;
        let id = raw.deserialize()?;
        match id {
            0u64 => { Ok(Response::Ok(raw.deserialize()?)) },
            1u64 => { Ok(Response::Err(raw.deserialize()?)) },
            v    => {
                Err(cbor_event::Error::CustomError(format!("Invalid Response Enum header expected 0 or 1 but got {}", v)))
            }
        }
    }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GetBlockHeaders {
    from: Vec<HeaderHash>,
    to: Option<HeaderHash>
}
impl GetBlockHeaders {
    pub fn tip() -> Self {
        GetBlockHeaders {
            from: Vec::new(),
            to:   None,
        }
    }
    pub fn range(from: Vec<HeaderHash>, to: HeaderHash) -> Self {
        GetBlockHeaders {
            from: from,
            to: Some(to),
        }
    }
}
impl se::Serialize for GetBlockHeaders {
    fn serialize<W>(&self, serializer: se::Serializer<W>) -> cbor_event::Result<se::Serializer<W>>
        where W: ::std::io::Write
    {
        let serializer = serializer.write_array(cbor_event::Len::Len(2))?;
        let serializer = se::serialize_indefinite_array(self.from.iter(), serializer)?;
        match &self.to {
            &None    => serializer.write_array(cbor_event::Len::Len(0)),
            &Some(ref h) => {
                serializer.write_array(cbor_event::Len::Len(1))?
                    .serialize(h)
            }
        }
    }
}
impl de::Deserialize for GetBlockHeaders {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(2, "GetBlockHeader")?;
        let from = raw.deserialize()?;
        let to = {
            let len = raw.array()?;
            match len {
                cbor_event::Len::Len(0) => None,
                cbor_event::Len::Len(1) => {
                    let h = raw.deserialize()?;
                    Some(h)
                },
                len => return Err(cbor_event::Error::CustomError(format!("Len {:?} not supported for the `GetBlockHeader.to`", len))),
            }
        };
        Ok(GetBlockHeaders { from: from, to: to })
    }
}

#[derive(Clone, Debug)]
pub struct BlockHeaders(Vec<block::BlockHeader>);
impl se::Serialize for BlockHeaders {
    fn serialize<W>(&self, serializer: se::Serializer<W>) -> cbor_event::Result<se::Serializer<W>>
        where W: ::std::io::Write
    {
        se::serialize_fixed_array(self.0.iter(), serializer)
    }
}
impl de::Deserialize for BlockHeaders {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.deserialize().map(BlockHeaders)
    }
}


#[cfg(test)]
fn random_headerhash<G: ::quickcheck::Gen>(g: &mut G) -> HeaderHash {
    let bytes : Vec<u8> = ::quickcheck::Arbitrary::arbitrary(g);
    HeaderHash::new(&bytes)
}

#[cfg(test)]
fn random_to<G: ::quickcheck::Gen>(g: &mut G) -> Option<HeaderHash> {
    let value : Option<()> = ::quickcheck::Arbitrary::arbitrary(g);
    value.map(|_| random_headerhash(g))
}

#[cfg(test)]
fn random_from<G: ::quickcheck::Gen>(g: &mut G) -> Vec<HeaderHash> {
    let num : usize = ::quickcheck::Arbitrary::arbitrary(g);
    ::std::iter::repeat_with(|| random_headerhash(g)).take(num).collect()
}

#[cfg(test)]
impl ::quickcheck::Arbitrary for GetBlockHeaders {
    fn arbitrary<G: ::quickcheck::Gen>(g: &mut G) -> Self {
        let from = random_from(g);
        let to   = random_to(g);
        GetBlockHeaders { from: from, to: to }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    quickcheck!{
        fn command_encode_decode(command: GetBlockHeaders) -> bool {
            let encoded = cbor!(command).unwrap();
            let decoded : GetBlockHeaders = de::RawCbor::from(&encoded).deserialize_complete().unwrap();

            decoded == command
        }
    }
}
