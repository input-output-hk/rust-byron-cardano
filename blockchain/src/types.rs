use std::{fmt};
use wallet_crypto::cbor::{ExtendedResult};
use wallet_crypto::{cbor, hash, hash::{HASH_SIZE, Blake2b256}};
use raw_cbor::{self, de::RawCbor};

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
pub struct BlockHeaderAttributes(cbor::Value);

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
impl cbor::CborValue for Version {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(
            vec![
                cbor::CborValue::encode(&self.major),
                cbor::CborValue::encode(&self.minor),
                cbor::CborValue::encode(&self.revision),
            ]
        )
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, major)    = cbor::array_decode_elem(array, 0).embed("major")?;
            let (array, minor)    = cbor::array_decode_elem(array, 0).embed("minor")?;
            let (array, revision) = cbor::array_decode_elem(array, 0).embed("revision")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(Version::new(major, minor, revision))
        }).embed("while decoding Version")
    }
}
impl raw_cbor::de::Deserialize for Version {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(3) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid Version: recieved array of {:?} elements", len)));
        }
        let major = *raw.unsigned_integer()? as u32;
        let minor = *raw.unsigned_integer()? as u32;
        let revision = *raw.unsigned_integer()? as u32;

        Ok(Version::new(major, minor, revision))
    }
}

impl cbor::CborValue for BlockVersion {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(
            vec![
                cbor::CborValue::encode(&self.0),
                cbor::CborValue::encode(&self.1),
                cbor::CborValue::encode(&self.2),
            ]
        )
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, major)    = cbor::array_decode_elem(array, 0).embed("major")?;
            let (array, minor)    = cbor::array_decode_elem(array, 0).embed("minor")?;
            let (array, revision) = cbor::array_decode_elem(array, 0).embed("revision")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(BlockVersion::new(major, minor, revision))
        }).embed("While decoding a BlockVersion")
    }
}
impl raw_cbor::de::Deserialize for BlockVersion {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(3) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid BlockVersion: recieved array of {:?} elements", len)));
        }
        let major = *raw.unsigned_integer()? as u16;
        let minor = *raw.unsigned_integer()? as u16;
        let revision = *raw.unsigned_integer()? as u8;

        Ok(BlockVersion::new(major, minor, revision))
    }
}

impl cbor::CborValue for SoftwareVersion {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(
            vec![
                cbor::CborValue::encode(&self.application_name),
                cbor::CborValue::encode(&self.application_version),
            ]
        )
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, name)    = cbor::array_decode_elem(array, 0).embed("name")?;
            let (array, version) = cbor::array_decode_elem(array, 0).embed("version")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(SoftwareVersion::new(name, version))
        }).embed("While decoding a SoftwareVersion")
    }
}
impl raw_cbor::de::Deserialize for SoftwareVersion {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(2) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid SoftwareVersion: recieved array of {:?} elements", len)));
        }
        let name  = raw.text()?;
        let version = *raw.unsigned_integer()? as u32;

        Ok(SoftwareVersion::new(name.to_string(), version))
    }
}

impl cbor::CborValue for HeaderHash {
    fn encode(&self) -> cbor::Value { cbor::CborValue::encode(&self.0) }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        cbor::CborValue::decode(value).map(|h| HeaderHash(h))
    }
}
impl raw_cbor::de::Deserialize for HeaderHash {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        raw_cbor::de::Deserialize::deserialize(raw).map(|h| HeaderHash(h))
    }
}

impl cbor::CborValue for BlockHeaderAttributes {
    fn encode(&self) -> cbor::Value {
        self.0.clone()
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        Ok(BlockHeaderAttributes(value))
    }
}
impl raw_cbor::de::Deserialize for BlockHeaderAttributes {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.map()?;
        if len != raw_cbor::Len::Len(0) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid BlockHeaderAttributes: recieved array of {:?} elements", len)));
        }
        Ok(BlockHeaderAttributes(cbor::Value::Object(::std::collections::BTreeMap::new())))
    }
}

impl cbor::CborValue for HeaderExtraData {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(
            vec![
                cbor::CborValue::encode(&self.block_version),
                cbor::CborValue::encode(&self.software_version),
                cbor::CborValue::encode(&self.attributes),
                cbor::CborValue::encode(&self.extra_data_proof),
            ]
        )
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, block_version)    = cbor::array_decode_elem(array, 0).embed("block version")?;
            let (array, software_version) = cbor::array_decode_elem(array, 0).embed("software version")?;
            let (array, attributes)       = cbor::array_decode_elem(array, 0).embed("attributes")?;
            let (array, extra_data_proof) = cbor::array_decode_elem(array, 0).embed("extra data proof")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(HeaderExtraData::new(block_version, software_version, attributes, extra_data_proof))
        }).embed("While decoding a HeaderExtraData")
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

impl cbor::CborValue for SscProof {
    fn encode(&self) -> cbor::Value {
        match self {
            &SscProof::Commitments(ref commhash, ref vss) =>
                cbor::Value::Array(vec![ cbor::Value::U64(0u64), cbor::CborValue::encode(commhash), cbor::CborValue::encode(vss) ]),
            &SscProof::Openings(ref commhash, ref vss) =>
                cbor::Value::Array(vec![ cbor::Value::U64(1u64), cbor::CborValue::encode(commhash), cbor::CborValue::encode(vss) ]),
            &SscProof::Shares(ref commhash, ref vss) =>
                cbor::Value::Array(vec![ cbor::Value::U64(2u64), cbor::CborValue::encode(commhash), cbor::CborValue::encode(vss) ]),
            &SscProof::Certificate(ref cert) =>
                cbor::Value::Array(vec![ cbor::Value::U64(3u64), cbor::CborValue::encode(cert) ]),
        }
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, code)  = cbor::array_decode_elem(array, 0).embed("enumeration code")?;
            if code == 0u64 {
                let (array, commhash) = cbor::array_decode_elem(array, 0)?;
                let (array, vss)      = cbor::array_decode_elem(array, 0)?;
                if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
                Ok(SscProof::Commitments(commhash, vss))
            } else if code == 1u64 {
                let (array, commhash) = cbor::array_decode_elem(array, 0)?;
                let (array, vss)      = cbor::array_decode_elem(array, 0)?;
                if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
                Ok(SscProof::Openings(commhash, vss))
            } else if code == 2u64 {
                let (array, commhash) = cbor::array_decode_elem(array, 0)?;
                let (array, vss)      = cbor::array_decode_elem(array, 0)?;
                if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
                Ok(SscProof::Shares(commhash, vss))
            } else if code == 3u64 {
                let (array, cert)      = cbor::array_decode_elem(array, 0)?;
                if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
                Ok(SscProof::Certificate(cert))
            } else {
                cbor::Result::array(array, cbor::Error::InvalidSumtype(code))
            }
        }).embed("While decoding SscProof")
    }
}
impl raw_cbor::de::Deserialize for SscProof {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(2) && len != raw_cbor::Len::Len(3) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid SscProof: recieved array of {:?} elements", len)));
        }
        let sum_type_idx = raw.unsigned_integer()?;
        match *sum_type_idx {
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
                Err(raw_cbor::Error::CustomError(format!("Unsupported SccProof: {}", *sum_type_idx)))
            }
        }
    }
}

impl cbor::CborValue for ChainDifficulty {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(vec![ cbor::Value::U64(self.0)])
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, difficulty) = cbor::array_decode_elem(array, 0).embed("epoch")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(ChainDifficulty(difficulty))
        }).embed("While decoding ChainDifficulty")
    }
}
impl raw_cbor::de::Deserialize for ChainDifficulty {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(1) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid ChainDifficulty: recieved array of {:?} elements", len)));
        }
        Ok(ChainDifficulty(*raw.unsigned_integer()?))
    }
}

impl cbor::CborValue for SlotId {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(vec![ cbor::Value::U64(self.epoch as u64), cbor::Value::U64(self.slotid as u64) ])
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, epoch) = cbor::array_decode_elem(array, 0).embed("epoch")?;
            let (array, slotid) = cbor::array_decode_elem(array, 0).embed("slotid")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(SlotId { epoch: epoch, slotid: slotid })
        }).embed("While decoding Slotid")
    }
}
impl raw_cbor::de::Deserialize for SlotId {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let len = raw.array()?;
        if len != raw_cbor::Len::Len(2) {
            return Err(raw_cbor::Error::CustomError(format!("Invalid SlotId: recieved array of {:?} elements", len)));
        }
        let epoch  = *raw.unsigned_integer()? as u32;
        let slotid = *raw.unsigned_integer()? as u32;
        Ok(SlotId { epoch: epoch, slotid: slotid })
    }
}
