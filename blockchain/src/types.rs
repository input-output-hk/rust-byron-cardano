use std::{fmt};
use cardano::{hash, hash::{HASH_SIZE, Blake2b256}};
use raw_cbor::{self, de::RawCbor, se::{Serializer}};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Version {
   major:    u32,
   minor:    u32,
   revision: u32,
}
impl Version {
    pub fn new(major: u32, minor: u32, revision: u32) -> Self {
        Version { major: major, minor: minor, revision: revision }
    }
}
impl Default for Version {
    fn default() -> Self { Version::new(0,1,0) }
}
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.revision)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct HeaderHash(Blake2b256);
impl AsRef<[u8]> for HeaderHash { fn as_ref(&self) -> &[u8] { self.0.as_ref() } }
impl fmt::Display for HeaderHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}
impl HeaderHash {
    pub fn bytes<'a>(&'a self) -> &'a [u8;HASH_SIZE] { self.0.bytes() }
    pub fn into_bytes(self) -> [u8;HASH_SIZE] { self.0.into_bytes() }
    pub fn from_bytes(bytes :[u8;HASH_SIZE]) -> Self { HeaderHash(Blake2b256::from_bytes(bytes)) }
    pub fn from_slice(bytes: &[u8]) -> hash::Result<Self> {
        Blake2b256::from_slice(bytes).map(|h| HeaderHash(h))
    }
    pub fn from_hex<S: AsRef<str>>(hex: &S) -> hash::Result<Self> {
        Blake2b256::from_hex(hex).map(|h| HeaderHash(h))
    }
    pub fn new(bytes: &[u8]) -> Self { HeaderHash(Blake2b256::new(bytes))  }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct BlockVersion(u16, u16, u8);
impl BlockVersion {
    pub fn new(major: u16, minor: u16, revision: u8) -> Self {
        BlockVersion(major, minor, revision)
    }
}
impl fmt::Debug for BlockVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}
impl fmt::Display for BlockVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Default for BlockVersion {
    fn default() -> Self { BlockVersion::new(0,1,0) }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct SoftwareVersion {
    application_name: String,
    application_version: u32
}
impl SoftwareVersion {
    pub fn new(name: String, version: u32) -> Self {
        SoftwareVersion {
            application_name: name,
            application_version: version
        }
    }
}
impl Default for SoftwareVersion {
    fn default() -> Self {
        SoftwareVersion::new(
            env!("CARGO_PKG_NAME").to_string(),
            env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap()
        )
    }
}

#[derive(Debug, Clone)]
pub struct BlockHeaderAttributes(raw_cbor::Value);

#[derive(Debug, Clone)]
pub struct HeaderExtraData {
    pub block_version: BlockVersion,
    pub software_version: SoftwareVersion,
    pub attributes: BlockHeaderAttributes,
    pub extra_data_proof: Blake2b256 // hash of the Extra body data
}
impl HeaderExtraData {
    pub fn new(block_version: BlockVersion, software_version: SoftwareVersion, attributes: BlockHeaderAttributes, extra_data_proof: Blake2b256) -> Self {
        HeaderExtraData {
            block_version: block_version,
            software_version: software_version,
            attributes: attributes,
            extra_data_proof: extra_data_proof
        }
    }
}

#[derive(Debug, Clone)]
pub enum SscProof {
    Commitments(Blake2b256, Blake2b256),
    Openings(Blake2b256, Blake2b256),
    Shares(Blake2b256, Blake2b256),
    Certificate(Blake2b256)
}

#[derive(Debug,Clone,Copy)]
pub struct ChainDifficulty(u64);

impl fmt::Display for ChainDifficulty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type EpochId = u32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotId {
    pub epoch: EpochId,
    pub slotid: u32,
}
impl SlotId {
    pub fn next(&self) -> Self {
        SlotId { epoch: self.epoch, slotid: self.slotid + 1 }
    }
}
impl fmt::Display for SlotId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.epoch, self.slotid)
    }
}



// **************************************************************************
// CBOR implementations
// **************************************************************************
impl raw_cbor::se::Serialize for Version {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.write_array(raw_cbor::Len::Len(3))?
            .write_unsigned_integer(self.major as u64)?
            .write_unsigned_integer(self.minor as u64)?
            .write_unsigned_integer(self.revision as u64)
    }
}
impl raw_cbor::de::Deserialize for Version {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(3) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid Version: recieved array of {:?} elements", len)));
        }
        let major = raw.unsigned_integer()? as u32;
        let minor = raw.unsigned_integer()? as u32;
        let revision = raw.unsigned_integer()? as u32;

        Ok(Version::new(major, minor, revision))
    }
}

impl raw_cbor::se::Serialize for BlockVersion {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.write_array(raw_cbor::Len::Len(3))?
            .write_unsigned_integer(self.0 as u64)?
            .write_unsigned_integer(self.1 as u64)?
            .write_unsigned_integer(self.2 as u64)
    }
}
impl raw_cbor::de::Deserialize for BlockVersion {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(3) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid BlockVersion: recieved array of {:?} elements", len)));
        }
        let major = raw.unsigned_integer()? as u16;
        let minor = raw.unsigned_integer()? as u16;
        let revision = raw.unsigned_integer()? as u8;

        Ok(BlockVersion::new(major, minor, revision))
    }
}

impl raw_cbor::se::Serialize for SoftwareVersion {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.write_array(raw_cbor::Len::Len(2))?
            .write_text(&self.application_name)?
            .write_unsigned_integer(self.application_version as u64)
    }
}
impl raw_cbor::de::Deserialize for SoftwareVersion {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(2) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid SoftwareVersion: recieved array of {:?} elements", len)));
        }
        let name  = raw.text()?;
        let version = raw.unsigned_integer()? as u32;

        Ok(SoftwareVersion::new(name.to_string(), version))
    }
}

impl raw_cbor::se::Serialize for HeaderHash {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.serialize(&self.0)
    }
}
impl raw_cbor::de::Deserialize for HeaderHash {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        raw_cbor::de::Deserialize::deserialize(raw).map(|h| HeaderHash(h))
    }
}

impl raw_cbor::se::Serialize for BlockHeaderAttributes {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.serialize(&self.0)
    }
}
impl raw_cbor::de::Deserialize for BlockHeaderAttributes {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        Ok(BlockHeaderAttributes(raw.deserialize()?))
    }
}

impl raw_cbor::se::Serialize for HeaderExtraData {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.write_array(raw_cbor::Len::Len(4))?
            .serialize(&self.block_version)?
            .serialize(&self.software_version)?
            .serialize(&self.attributes)?
            .serialize(&self.extra_data_proof)
    }
}
impl raw_cbor::de::Deserialize for HeaderExtraData {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(4) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid HeaderExtraData: recieved array of {:?} elements", len)));
        }
        let block_version    = raw_cbor::de::Deserialize::deserialize(raw)?;
        let software_version = raw_cbor::de::Deserialize::deserialize(raw)?;
        let attributes       = raw_cbor::de::Deserialize::deserialize(raw)?;
        let extra_data_proof = raw_cbor::de::Deserialize::deserialize(raw)?;

        Ok(HeaderExtraData::new(block_version, software_version, attributes, extra_data_proof))
    }
}

impl raw_cbor::se::Serialize for SscProof {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        match self {
            &SscProof::Commitments(ref commhash, ref vss) => {
                serializer.write_array(raw_cbor::Len::Len(3))?
                    .write_unsigned_integer(0)?
                    .serialize(commhash)?
                    .serialize(vss)
            },
            &SscProof::Openings(ref commhash, ref vss) => {
                serializer.write_array(raw_cbor::Len::Len(3))?
                    .write_unsigned_integer(1)?
                    .serialize(commhash)?
                    .serialize(vss)
            },
            &SscProof::Shares(ref commhash, ref vss) => {
                serializer.write_array(raw_cbor::Len::Len(3))?
                    .write_unsigned_integer(2)?
                    .serialize(commhash)?
                    .serialize(vss)
            },
            &SscProof::Certificate(ref cert) => {
                serializer.write_array(raw_cbor::Len::Len(2))?
                    .write_unsigned_integer(3)?
                    .serialize(cert)
            },
        }
    }
}
impl raw_cbor::de::Deserialize for SscProof {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(2) && len != raw_cbor::Len::Len(3) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid SscProof: recieved array of {:?} elements", len)));
        }
        let sum_type_idx = raw.unsigned_integer()?;
        match sum_type_idx {
            0 => {
                let commhash = raw_cbor::de::Deserialize::deserialize(raw)?;
                let vss      = raw_cbor::de::Deserialize::deserialize(raw)?;
                Ok(SscProof::Commitments(commhash, vss))
            },
            1 => {
                let commhash = raw_cbor::de::Deserialize::deserialize(raw)?;
                let vss      = raw_cbor::de::Deserialize::deserialize(raw)?;
                Ok(SscProof::Openings(commhash, vss))
            },
            2 => {
                let commhash = raw_cbor::de::Deserialize::deserialize(raw)?;
                let vss      = raw_cbor::de::Deserialize::deserialize(raw)?;
                Ok(SscProof::Shares(commhash, vss))
            },
            3 => {
                let cert = raw_cbor::de::Deserialize::deserialize(raw)?;
                Ok(SscProof::Certificate(cert))
            },
            _ => {
                Err(raw_cbor::Error::CustomError(format!("Unsupported SccProof: {}", sum_type_idx)))
            }
        }
    }
}

impl raw_cbor::se::Serialize for ChainDifficulty {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.write_array(raw_cbor::Len::Len(1))?.write_unsigned_integer(self.0)
    }
}
impl raw_cbor::de::Deserialize for ChainDifficulty {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(1) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid ChainDifficulty: recieved array of {:?} elements", len)));
        }
        Ok(ChainDifficulty(raw.unsigned_integer()?))
    }
}

impl raw_cbor::se::Serialize for SlotId {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.write_array(raw_cbor::Len::Len(2))?
            .write_unsigned_integer(self.epoch as u64)?
            .write_unsigned_integer(self.slotid as u64)
    }
}
impl raw_cbor::de::Deserialize for SlotId {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(2) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid SlotId: recieved array of {:?} elements", len)));
        }
        let epoch  = raw.unsigned_integer()? as u32;
        let slotid = raw.unsigned_integer()? as u32;
        Ok(SlotId { epoch: epoch, slotid: slotid })
    }
}
