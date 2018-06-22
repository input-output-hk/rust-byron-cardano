use cardano::{address, hash::{Blake2b256}};
use cardano::config::{ProtocolMagic};
use std::{fmt};

use raw_cbor::{self, de::RawCbor};
use types;
use types::{HeaderHash, ChainDifficulty};

#[derive(Debug, Clone)]
pub struct BodyProof(Blake2b256);

impl raw_cbor::se::Serialize for BodyProof {
    fn serialize(&self, serializer: raw_cbor::se::Serializer) -> raw_cbor::Result<raw_cbor::se::Serializer> {
        serializer.serialize(&self.0)
    }
}
impl raw_cbor::de::Deserialize for BodyProof {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        Ok(BodyProof(raw.deserialize()?))
    }
}

#[derive(Debug, Clone)]
pub struct Body {
    pub slot_leaders: Vec<address::StakeholderId>,
}
impl raw_cbor::se::Serialize for Body {
    fn serialize(&self, serializer: raw_cbor::se::Serializer) -> raw_cbor::Result<raw_cbor::se::Serializer> {
        raw_cbor::se::serialize_indefinite_array(self.slot_leaders.iter(), serializer)
    }
}
impl raw_cbor::de::Deserialize for Body {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        assert_eq!(len, raw_cbor::Len::Indefinite);
        let mut slot_leaders = Vec::new();
        while {
            let t = raw.cbor_type()?;
            if t == raw_cbor::Type::Special {
                let special = raw.special()?;
                assert_eq!(special, raw_cbor::Special::Break);
                false
            } else {
                slot_leaders.push(raw_cbor::de::Deserialize::deserialize(raw)?);
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
impl raw_cbor::se::Serialize for BlockHeader {
    fn serialize(&self, serializer: raw_cbor::se::Serializer) -> raw_cbor::Result<raw_cbor::se::Serializer> {
        serializer.write_array(raw_cbor::Len::Len(5))?
            .serialize(&self.protocol_magic)?
            .serialize(&self.previous_header)?
            .serialize(&self.body_proof)?
            .serialize(&self.consensus)?
            .serialize(&self.extra_data)
    }
}
impl raw_cbor::de::Deserialize for BlockHeader {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(5) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid BlockHeader: recieved array of {:?} elements", len)));
        }
        let p_magic    = raw_cbor::de::Deserialize::deserialize(raw)?;
        let prv_header = raw_cbor::de::Deserialize::deserialize(raw)?;
        let body_proof = raw_cbor::de::Deserialize::deserialize(raw)?;
        let consensus  = raw_cbor::de::Deserialize::deserialize(raw)?;
        let extra_data = raw_cbor::de::Deserialize::deserialize(raw)?;

        Ok(BlockHeader::new(p_magic, prv_header, body_proof, consensus, extra_data))
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub body: Body,
    pub extra: raw_cbor::Value
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.header)?;
        write!(f, "{:?}", self.body)
    }
}
impl raw_cbor::se::Serialize for Block {
    fn serialize(&self, serializer: raw_cbor::se::Serializer) -> raw_cbor::Result<raw_cbor::se::Serializer> {
        serializer.write_array(raw_cbor::Len::Len(3))?
            .serialize(&self.header)?
            .serialize(&self.body)?
            .serialize(&self.extra)
    }
}
impl raw_cbor::de::Deserialize for Block {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(3) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid Block: recieved array of {:?} elements", len)));
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
impl raw_cbor::se::Serialize for Consensus {
    fn serialize(&self, serializer: raw_cbor::se::Serializer) -> raw_cbor::Result<raw_cbor::se::Serializer> {
        serializer.write_array(raw_cbor::Len::Len(2))?
            .write_unsigned_integer(self.epoch as u64)?
            .serialize(&self.chain_difficulty)
    }
}
impl raw_cbor::de::Deserialize for Consensus {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(2) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid Consensus: recieved array of {:?} elements", len)));
        }
        let epoch = raw.unsigned_integer()? as u32;
        let chain_difficulty = raw_cbor::de::Deserialize::deserialize(raw)?;
        Ok(Consensus { epoch, chain_difficulty })
    }
}
