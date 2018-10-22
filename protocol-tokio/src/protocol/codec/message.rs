use bytes::{BufMut, BytesMut, Bytes};

use super::{NodeId};
use super::super::{nt};

pub type MessageCode = u32;

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
        if bytes.len() == 9 {
            use bytes::{IntoBuf, Buf};
            let mut buf = bytes.into_buf();
            let key = buf.get_u8();
            let v   = buf.get_u64_be();
            match key {
                0x53 => { Ok(Message::CreateNodeId(lwcid, NodeId::from(v))) },
                0x41 => { Ok(Message::AckNodeId(lwcid, NodeId::from(v))) },
                _    => { Ok(Message::Bytes(lwcid, buf.into_inner())) },
            }
        } else {
            Ok(Message::Bytes(lwcid, bytes))
        }
    }
}
