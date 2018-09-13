use std::{fmt, str::{FromStr}, ops::{Deref}};
use hash::{Blake2b256};
use cbor_event::{self, de::RawCbor};
use util::try_from_slice::TryFromSlice;

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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct HeaderHash(Blake2b256);
impl HeaderHash {
    pub fn new(bytes: &[u8]) -> Self { HeaderHash(Blake2b256::new(bytes))  }
}
impl fmt::Display for HeaderHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl TryFromSlice for HeaderHash {
    type Error = <Blake2b256 as TryFromSlice>::Error;
    fn try_from_slice(slice: &[u8]) -> ::std::result::Result<Self, Self::Error> {
        Ok(Self::from(Blake2b256::try_from_slice(slice)?))
    }
}
impl Deref for HeaderHash {
    type Target = <Blake2b256 as Deref>::Target;
    fn deref(&self) -> &Self::Target { self.0.deref() }
}
impl AsRef<[u8]> for HeaderHash {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl From<HeaderHash> for [u8;Blake2b256::HASH_SIZE] {
    fn from(hash: HeaderHash) -> Self { hash.0.into() }
}
impl From<[u8;Blake2b256::HASH_SIZE]> for HeaderHash {
    fn from(hash: [u8;Blake2b256::HASH_SIZE]) -> Self { HeaderHash(Blake2b256::from(hash)) }
}
impl From<Blake2b256> for HeaderHash {
    fn from(hash: Blake2b256) -> Self { HeaderHash(hash) }
}
impl FromStr for HeaderHash {
    type Err = <Blake2b256 as FromStr>::Err;
    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        Ok(Self::from(Blake2b256::from_str(s)?))
    }
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
    pub application_version: u32
}

const MAX_APPLICATION_NAME_LENGTH : usize = 12;

impl SoftwareVersion {
    pub fn new(name: &str, version: u32) -> cbor_event::Result<Self> {
        if name.len() > MAX_APPLICATION_NAME_LENGTH {
            return Err(cbor_event::Error::CustomError(format!("Received application name '{}' is too long", name)));
        }
        Ok(SoftwareVersion {
            application_name: name.to_string(),
            application_version: version
        })
    }

    pub fn application_name(&self) -> &String {
        &self.application_name
    }
}
impl Default for SoftwareVersion {
    fn default() -> Self {
        SoftwareVersion::new(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap()
        ).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct BlockHeaderAttributes(cbor_event::Value);

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

pub type EpochId = u64; // == EpochIndex
pub type SlotId = u16; // == LocalSlotIndex

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochSlotId {
    pub epoch: EpochId,
    pub slotid: SlotId,
}
impl EpochSlotId {
    pub fn next(&self) -> Self {
        EpochSlotId { epoch: self.epoch, slotid: self.slotid + 1 }
    }
    pub fn slot_number(&self) -> usize {
        (self.epoch as usize) * 21600 + (self.slotid as usize)
    }
}
impl fmt::Display for EpochSlotId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.epoch, self.slotid)
    }
}

impl ::std::ops::Sub<EpochSlotId> for EpochSlotId {
    type Output = usize;
    fn sub(self, rhs: Self) -> Self::Output {
        self.slot_number() - rhs.slot_number()
    }
}

// **************************************************************************
// CBOR implementations
// **************************************************************************
impl cbor_event::se::Serialize for Version {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(3))?
            .write_unsigned_integer(self.major as u64)?
            .write_unsigned_integer(self.minor as u64)?
            .write_unsigned_integer(self.revision as u64)
    }
}
impl cbor_event::de::Deserialize for Version {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(3, "Version")?;
        let major = raw.unsigned_integer()? as u32;
        let minor = raw.unsigned_integer()? as u32;
        let revision = raw.unsigned_integer()? as u32;

        Ok(Version::new(major, minor, revision))
    }
}

impl cbor_event::se::Serialize for BlockVersion {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(3))?
            .write_unsigned_integer(self.0 as u64)?
            .write_unsigned_integer(self.1 as u64)?
            .write_unsigned_integer(self.2 as u64)
    }
}
impl cbor_event::de::Deserialize for BlockVersion {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(3, "BlockVersion")?;
        let major = raw.unsigned_integer()? as u16;
        let minor = raw.unsigned_integer()? as u16;
        let revision = raw.unsigned_integer()? as u8;

        Ok(BlockVersion::new(major, minor, revision))
    }
}

impl cbor_event::se::Serialize for SoftwareVersion {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(2))?
            .write_text(&self.application_name)?
            .write_unsigned_integer(self.application_version as u64)
    }
}
impl cbor_event::de::Deserialize for SoftwareVersion {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(2, "SoftwareVersion")?;
        let name  = raw.text()?;
        let version = raw.unsigned_integer()? as u32;

        Ok(SoftwareVersion::new(&name, version)?)
    }
}

impl cbor_event::se::Serialize for HeaderHash {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.serialize(&self.0)
    }
}
impl cbor_event::de::Deserialize for HeaderHash {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        cbor_event::de::Deserialize::deserialize(raw).map(|h| HeaderHash(h))
    }
}

impl cbor_event::se::Serialize for BlockHeaderAttributes {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.serialize(&self.0)
    }
}
impl cbor_event::de::Deserialize for BlockHeaderAttributes {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(BlockHeaderAttributes(raw.deserialize()?))
    }
}

impl cbor_event::se::Serialize for HeaderExtraData {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(4))?
            .serialize(&self.block_version)?
            .serialize(&self.software_version)?
            .serialize(&self.attributes)?
            .serialize(&self.extra_data_proof)
    }
}
impl cbor_event::de::Deserialize for HeaderExtraData {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(4, "HeaderExtraData")?;
        let block_version    = cbor_event::de::Deserialize::deserialize(raw)?;
        let software_version = cbor_event::de::Deserialize::deserialize(raw)?;
        let attributes       = cbor_event::de::Deserialize::deserialize(raw)?;
        let extra_data_proof = cbor_event::de::Deserialize::deserialize(raw)?;

        Ok(HeaderExtraData::new(block_version, software_version, attributes, extra_data_proof))
    }
}

impl cbor_event::se::Serialize for SscProof {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        match self {
            &SscProof::Commitments(ref commhash, ref vss) => {
                serializer.write_array(cbor_event::Len::Len(3))?
                    .write_unsigned_integer(0)?
                    .serialize(commhash)?
                    .serialize(vss)
            },
            &SscProof::Openings(ref commhash, ref vss) => {
                serializer.write_array(cbor_event::Len::Len(3))?
                    .write_unsigned_integer(1)?
                    .serialize(commhash)?
                    .serialize(vss)
            },
            &SscProof::Shares(ref commhash, ref vss) => {
                serializer.write_array(cbor_event::Len::Len(3))?
                    .write_unsigned_integer(2)?
                    .serialize(commhash)?
                    .serialize(vss)
            },
            &SscProof::Certificate(ref cert) => {
                serializer.write_array(cbor_event::Len::Len(2))?
                    .write_unsigned_integer(3)?
                    .serialize(cert)
            },
        }
    }
}
impl cbor_event::de::Deserialize for SscProof {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(2) && len != cbor_event::Len::Len(3) {
            return Err(cbor_event::Error::CustomError(format!("Invalid SscProof: recieved array of {:?} elements", len)));
        }
        let sum_type_idx = raw.unsigned_integer()?;
        match sum_type_idx {
            0 => {
                let commhash = cbor_event::de::Deserialize::deserialize(raw)?;
                let vss      = cbor_event::de::Deserialize::deserialize(raw)?;
                Ok(SscProof::Commitments(commhash, vss))
            },
            1 => {
                let commhash = cbor_event::de::Deserialize::deserialize(raw)?;
                let vss      = cbor_event::de::Deserialize::deserialize(raw)?;
                Ok(SscProof::Openings(commhash, vss))
            },
            2 => {
                let commhash = cbor_event::de::Deserialize::deserialize(raw)?;
                let vss      = cbor_event::de::Deserialize::deserialize(raw)?;
                Ok(SscProof::Shares(commhash, vss))
            },
            3 => {
                let cert = cbor_event::de::Deserialize::deserialize(raw)?;
                Ok(SscProof::Certificate(cert))
            },
            _ => {
                Err(cbor_event::Error::CustomError(format!("Unsupported SccProof: {}", sum_type_idx)))
            }
        }
    }
}

impl cbor_event::se::Serialize for ChainDifficulty {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(1))?.write_unsigned_integer(self.0)
    }
}
impl cbor_event::de::Deserialize for ChainDifficulty {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(1, "ChainDifficulty")?;
        Ok(ChainDifficulty(raw.unsigned_integer()?))
    }
}

impl cbor_event::se::Serialize for EpochSlotId {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.serialize(&(&self.epoch, &self.slotid))
    }
}
impl cbor_event::de::Deserialize for EpochSlotId {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(2, "SlotId")?;
        let epoch  = raw.deserialize()?;
        let slotid = raw.deserialize()?;
        Ok(EpochSlotId { epoch: epoch, slotid: slotid })
    }
}

pub type Attributes = cbor_event::Value; // TODO

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct CoinPortion(u64);

pub const COIN_PORTION_DENOMINATOR: u64 = 1_000_000_000_000_000;

impl CoinPortion {
    pub fn new(n: u64) -> cbor_event::Result<Self> {
        if n > COIN_PORTION_DENOMINATOR {
            return Err(cbor_event::Error::CustomError(format!("Coin portion {} is greater than {}", n, COIN_PORTION_DENOMINATOR)));
        }
        Ok(CoinPortion(n))
    }
}

impl cbor_event::se::Serialize for CoinPortion {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.serialize(&self.0)
    }
}

impl cbor_event::de::Deserialize for CoinPortion {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(CoinPortion::new(raw.deserialize()?)?)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct SystemTag(String);

const MAX_SYSTEM_TAG_LENGTH : usize = 10;

impl SystemTag {
    pub fn new(s: String) -> cbor_event::Result<Self> {
        if s.len() > MAX_SYSTEM_TAG_LENGTH {
            return Err(cbor_event::Error::CustomError(format!("System tag '{}' is too long", s)));
        }
        if !s.chars().all(|c| char::is_ascii(&c)) {
            return Err(cbor_event::Error::CustomError(format!("System tag '{}' contains non-ASCII characters", s)));
        }
        Ok(SystemTag(s))
    }
}

impl cbor_event::se::Serialize for SystemTag {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.serialize(&self.0)
    }
}

impl cbor_event::de::Deserialize for SystemTag {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(SystemTag::new(raw.deserialize()?)?)
    }
}
