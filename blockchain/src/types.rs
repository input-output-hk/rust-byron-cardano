use std::{fmt};
use wallet_crypto::cbor::{ExtendedResult};
use wallet_crypto::{cbor, util, tx};
use rcw::blake2b::Blake2b;
use rcw::digest::Digest;

const HASH_SIZE : usize = 32;

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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeaderHash([u8;HASH_SIZE]);
impl AsRef<[u8]> for HeaderHash { fn as_ref(&self) -> &[u8] { self.0.as_ref() } }
impl fmt::Debug for HeaderHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", util::hex::encode(self.as_ref()))
    }
}
impl fmt::Display for HeaderHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", util::hex::encode(self.as_ref()))
    }
}

impl HeaderHash {
    pub fn bytes<'a>(&'a self) -> &'a [u8;HASH_SIZE] { &self.0 }
    pub fn into_bytes(self) -> [u8;HASH_SIZE] { self.0 }
    pub fn from_bytes(bytes :[u8;HASH_SIZE]) -> Self { HeaderHash(bytes) }
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != HASH_SIZE { return None; }
        let mut buf = [0;HASH_SIZE];

        buf[0..HASH_SIZE].clone_from_slice(bytes);
        Some(Self::from_bytes(buf))
    }

    pub fn new(bytes: &[u8]) -> Self {
        let mut b2b = Blake2b::new(HASH_SIZE);
        let mut out = [0;HASH_SIZE];
        b2b.input(bytes);
        b2b.result(&mut out);
        Self::from_bytes(out)
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
    pub extra_data_proof: tx::Hash // hash of the Extra body data
}
impl HeaderExtraData {
    pub fn new(block_version: BlockVersion, software_version: SoftwareVersion, attributes: BlockHeaderAttributes, extra_data_proof: tx::Hash) -> Self {
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
    Commitments(tx::Hash, tx::Hash),
    Openings(tx::Hash, tx::Hash),
    Shares(tx::Hash, tx::Hash),
    Certificate(tx::Hash)
}

#[derive(Debug,Clone,Copy)]
pub struct ChainDifficulty(u64);

impl fmt::Display for ChainDifficulty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct SlotId {
    pub epoch: u32,
    pub slotid: u32,
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

impl cbor::CborValue for HeaderHash {
    fn encode(&self) -> cbor::Value { cbor::Value::Bytes(cbor::Bytes::from_slice(self.as_ref())) }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.bytes().and_then(|bytes| {
            match Self::from_slice(bytes.as_ref()) {
                Some(digest) => Ok(digest),
                None         => {
                    cbor::Result::bytes(bytes, cbor::Error::InvalidSize(HASH_SIZE))
                }
            }
        }).embed("while decoding Hash")
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