use {address, hash::{Blake2b256}};
use config::{ProtocolMagic};
use std::{fmt};

use cbor_event::{self, de::RawCbor};
use super::types;
use super::types::{HeaderHash, ChainDifficulty};

#[derive(Debug, Clone)]
pub struct BodyProof(Blake2b256);

impl cbor_event::se::Serialize for BodyProof {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.serialize(&self.0)
    }
}
impl cbor_event::de::Deserialize for BodyProof {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(BodyProof(raw.deserialize()?))
    }
}

#[derive(Debug, Clone)]
pub struct Body {
    pub slot_leaders: Vec<address::StakeholderId>,
}
impl cbor_event::se::Serialize for Body {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        cbor_event::se::serialize_indefinite_array(self.slot_leaders.iter(), serializer)
    }
}
impl cbor_event::de::Deserialize for Body {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        assert_eq!(len, cbor_event::Len::Indefinite);
        let mut slot_leaders = Vec::new();
        while {
            let t = raw.cbor_type()?;
            if t == cbor_event::Type::Special {
                let special = raw.special()?;
                assert_eq!(special, cbor_event::Special::Break);
                false
            } else {
                slot_leaders.push(cbor_event::de::Deserialize::deserialize(raw)?);
                true
            }
        } {}
        Ok(Body { slot_leaders })
    }
}

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub protocol_magic: ProtocolMagic,
    pub previous_header: HeaderHash,
    pub body_proof: BodyProof,
    pub consensus: Consensus,
    pub extra_data: types::BlockHeaderAttributes,
}
impl fmt::Display for BlockHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!( f
            , "Magic: 0x{:?} Previous Header: {}"
            , self.protocol_magic
            , self.previous_header
            )
    }
}
impl BlockHeader {
    pub fn new(pm: ProtocolMagic, pb: HeaderHash, bp: BodyProof, c: Consensus, ed: types::BlockHeaderAttributes) -> Self {
        BlockHeader {
            protocol_magic: pm,
            previous_header: pb,
            body_proof: bp,
            consensus: c,
            extra_data: ed
        }
    }
}
impl cbor_event::se::Serialize for BlockHeader {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(5))?
            .serialize(&self.protocol_magic)?
            .serialize(&self.previous_header)?
            .serialize(&self.body_proof)?
            .serialize(&self.consensus)?
            .serialize(&self.extra_data)
    }
}
impl cbor_event::de::Deserialize for BlockHeader {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(5) {
            return Err(cbor_event::Error::CustomError(format!("Invalid BlockHeader: recieved array of {:?} elements", len)));
        }
        let p_magic    = cbor_event::de::Deserialize::deserialize(raw)?;
        let prv_header = cbor_event::de::Deserialize::deserialize(raw)?;
        let body_proof = cbor_event::de::Deserialize::deserialize(raw)?;
        let consensus  = cbor_event::de::Deserialize::deserialize(raw)?;
        let extra_data = cbor_event::de::Deserialize::deserialize(raw)?;

        Ok(BlockHeader::new(p_magic, prv_header, body_proof, consensus, extra_data))
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub body: Body,
    pub extra: cbor_event::Value
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.header)?;
        write!(f, "{:?}", self.body)
    }
}
impl cbor_event::se::Serialize for Block {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(3))?
            .serialize(&self.header)?
            .serialize(&self.body)?
            .serialize(&self.extra)
    }
}
impl cbor_event::de::Deserialize for Block {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(3) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Block: recieved array of {:?} elements", len)));
        }
        let header = raw.deserialize()?;
        let body   = raw.deserialize()?;
        let extra  = raw.deserialize()?;
        Ok(Block { header, body, extra })
    }
}

#[derive(Debug, Clone)]
pub struct Consensus {
    pub epoch: types::EpochId,
    pub chain_difficulty: ChainDifficulty,
}
impl cbor_event::se::Serialize for Consensus {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(2))?
            .write_unsigned_integer(self.epoch as u64)?
            .serialize(&self.chain_difficulty)
    }
}
impl cbor_event::de::Deserialize for Consensus {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(2) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Consensus: recieved array of {:?} elements", len)));
        }
        let epoch = raw.unsigned_integer()? as u32;
        let chain_difficulty = cbor_event::de::Deserialize::deserialize(raw)?;
        Ok(Consensus { epoch, chain_difficulty })
    }
}
