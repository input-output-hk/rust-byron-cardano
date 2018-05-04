use std::fmt;
use std::collections::BTreeMap;
use serde;

use rcw::digest::Digest;
use rcw::blake2b::Blake2b;
use rcw::sha3::Sha3;

use redeem;
use util::{base58};
use cbor;
use cbor::{ExtendedResult};
use hdwallet::{XPub};
use hdpayload::{HDAddressPayload};

/// Digest of the composition of `Blake2b_224 . Sha3_256`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct DigestBlake2b224([u8;28]);
impl DigestBlake2b224 {
    /// create digest from the given inputs by computing the SHA3_256 and
    /// then the Blake2b_224.
    ///
    pub fn new(buf: &[u8]) -> Self
    {
        let mut b2b = Blake2b::new(28);
        let mut sh3 = Sha3::sha3_256();
        let mut out1 = [0;32];
        let mut out2 = [0;28];
        sh3.input(buf);
        sh3.result(&mut out1);
        b2b.input(&out1);
        b2b.result(&mut out2);
        DigestBlake2b224::from_bytes(out2)
    }

    /// create a Digest from the given 224 bits
    pub fn from_bytes(bytes :[u8;28]) -> Self { DigestBlake2b224(bytes) }
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 28 { return None; }
        let mut buf = [0;28];

        buf[0..28].clone_from_slice(bytes);
        Some(DigestBlake2b224::from_bytes(buf))
    }
}
impl fmt::Display for DigestBlake2b224 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.iter().for_each(|byte| {
            if byte < &0x10 {
                write!(f, "0{:x}", byte).unwrap()
            } else {
                write!(f, "{:x}", byte).unwrap()
            }
        });
        Ok(())
    }
}
impl cbor::CborValue for DigestBlake2b224 {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Bytes(cbor::Bytes::from_slice(self.0.as_ref()))
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.bytes().and_then(|bytes| {
            match DigestBlake2b224::from_slice(bytes.as_ref()) {
                Some(digest) => Ok(digest),
                None         => {
                    cbor::Result::bytes(bytes, cbor::Error::InvalidSize(28))
                }
            }
        }).embed("while decoding DigestBlake2b224")
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum AddrType {
    ATPubKey,
    ATScript,
    ATRedeem
}
// [TkListLen 1, TkInt (fromEnum t)]
impl AddrType {
    fn from_u64(v: u64) -> Option<Self> {
        match v {
            0 => Some(AddrType::ATPubKey),
            1 => Some(AddrType::ATScript),
            2 => Some(AddrType::ATRedeem),
            _ => None,
        }
    }
    fn to_byte(self) -> u8 {
        match self {
            AddrType::ATPubKey => 0,
            AddrType::ATScript => 1,
            AddrType::ATRedeem => 2
        }
    }
}
impl cbor::CborValue for AddrType {
    fn encode(&self) -> cbor::Value {
        cbor::Value::U64(self.to_byte() as u64)
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.u64().and_then(|v| {
            match AddrType::from_u64(v) {
                Some(addr_type) => Ok(addr_type),
                None            => cbor::Result::u64(v, cbor::Error::NotOneOf(&[cbor::Value::U64(0), cbor::Value::U64(1), cbor::Value::U64(2)]))
            }
        }).embed("while decoding AddrType")
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct StakeholderId(DigestBlake2b224); // of publickey (block2b 256)
impl StakeholderId {
    pub fn new(pubk: &XPub) -> StakeholderId {
        let buf = cbor::encode_to_cbor(pubk).unwrap();
        StakeholderId(DigestBlake2b224::new(buf.as_ref()))
    }
}
impl cbor::CborValue for StakeholderId {
    fn encode(&self) -> cbor::Value { cbor::CborValue::encode(&self.0) }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        cbor::CborValue::decode(value).map(|digest| { StakeholderId(digest) })
            .embed("while decoding StakeholderId")
    }
}
impl fmt::Display for StakeholderId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum StakeDistribution {
    BootstrapEraDistr,
    SingleKeyDistr(StakeholderId),
}

const STAKE_DISTRIBUTION_TAG_BOOTSTRAP : u64 = 1;
const STAKE_DISTRIBUTION_TAG_SINGLEKEY : u64 = 0;

impl StakeDistribution {
    pub fn new_bootstrap_era() -> Self { StakeDistribution::BootstrapEraDistr }
    pub fn new_single_stakeholder(si: StakeholderId) -> Self {
        StakeDistribution::SingleKeyDistr(si)
    }
    pub fn new_single_key(pubk: &XPub) -> Self {
        StakeDistribution::new_single_stakeholder(StakeholderId::new(pubk))
    }
}
impl cbor::CborValue for StakeDistribution {
    fn encode(&self) -> cbor::Value {
        let value = match self {
            &StakeDistribution::BootstrapEraDistr => {
                cbor::Value::Array(
                    vec![ cbor::Value::U64(STAKE_DISTRIBUTION_TAG_BOOTSTRAP)
                        ]
                )
            }
            &StakeDistribution::SingleKeyDistr(ref si) => {
                cbor::Value::Array(
                    vec![ cbor::Value::U64(STAKE_DISTRIBUTION_TAG_SINGLEKEY)
                        , cbor::CborValue::encode(si)
                        ]
                )
            }
        };
        let bytes = cbor::encode_to_cbor(&value).unwrap();
        cbor::Value::Bytes(cbor::Bytes::new(bytes))
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        let bytes = value.bytes()
            .embed("while decoding `StakeDistribution''s first level of indirection")?;
        let value = cbor::decode_from_cbor::<cbor::Value>(bytes.as_ref())
            .embed("while decoding `StakeDistribution`'s from cbor bytes")?;
        value.array().and_then(|sum_type| {
            let (sum_type, n) = cbor::array_decode_elem(sum_type, 0)
                .embed("while decoding `StakeDistribution`'s sumtype indice")?;
            if n == STAKE_DISTRIBUTION_TAG_BOOTSTRAP {
                Ok(StakeDistribution::new_bootstrap_era())
            } else if n == STAKE_DISTRIBUTION_TAG_SINGLEKEY {
                let (sum_type, k) = cbor::array_decode_elem(sum_type, 0)
                    .embed("while decoding single key stake distribution")?;
                if sum_type.len() != 0 {
                    return cbor::Result::array(sum_type, cbor::Error::UnparsedValues);
                }
                Ok(StakeDistribution::new_single_stakeholder(k))
            } else {
                cbor::Result::array(sum_type, cbor::Error::InvalidSumtype(n))
            }
        }).embed("while decoding `StakeDistribution`")
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Attributes {
    pub derivation_path: Option<HDAddressPayload>,
    pub stake_distribution: StakeDistribution
    // attr_remains ? whatever...
}
impl Attributes {
    pub fn new_bootstrap_era(hdap: Option<HDAddressPayload>) -> Self {
        Attributes {
            derivation_path: hdap,
            stake_distribution: StakeDistribution::BootstrapEraDistr
        }
    }
    pub fn new_single_key(pubk: &XPub, hdap: Option<HDAddressPayload>) -> Self {
        Attributes {
            derivation_path: hdap,
            stake_distribution: StakeDistribution::new_single_key(pubk)
        }
    }
}
const ATTRIBUTE_NAME_TAG_STAKE : u64 = 0;
const ATTRIBUTE_NAME_TAG_DERIVATION : u64 = 1;

impl cbor::CborValue for Attributes {
    fn encode(&self) -> cbor::Value {
        let mut map = BTreeMap::new();
        match &self.stake_distribution {
            &StakeDistribution::BootstrapEraDistr => { /**/ },
            &StakeDistribution::SingleKeyDistr(_) => {
                map.insert(
                    cbor::ObjectKey::Integer(ATTRIBUTE_NAME_TAG_STAKE),
                    cbor::CborValue::encode(&self.stake_distribution)
                );
            }
        };
        map.insert(
            cbor::ObjectKey::Integer(ATTRIBUTE_NAME_TAG_DERIVATION),
            cbor::CborValue::encode(&self.derivation_path)
        );
        cbor::Value::Object(map)
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.object().and_then(|object| {
            let (object, stake_distribution) = cbor::object_decode_elem(object, cbor::ObjectKey::Integer(ATTRIBUTE_NAME_TAG_STAKE))
                .or_else(|(val, _)| val.object().map(|obj| (obj, StakeDistribution::BootstrapEraDistr)))?;
            let (object, derivation_path) = cbor::object_decode_elem(object, cbor::ObjectKey::Integer(ATTRIBUTE_NAME_TAG_DERIVATION))
                .embed("expected the derivation_path")?;
            if object.len() != 0 {
                return cbor::Result::object(object, cbor::Error::UnparsedValues);
            }
            Ok(Attributes { derivation_path: derivation_path, stake_distribution: stake_distribution })
        }).embed("while decoding `Attributes`")
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Addr(DigestBlake2b224);
impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl cbor::CborValue for Addr {
    fn encode(&self) -> cbor::Value { cbor::CborValue::encode(&self.0) }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        cbor::CborValue::decode(value).map(|digest| { Addr(digest) })
            .embed("while decoding Addr")
    }
}
impl Addr {
    pub fn new(addr_type: AddrType, spending_data: &SpendingData, attrs: &Attributes) -> Addr {
        let d : (AddrType, SpendingData, Attributes) = (addr_type, spending_data.clone(), attrs.clone());
        let v = cbor::encode_to_cbor(&d).unwrap();
        Addr(DigestBlake2b224::new(v.as_slice()))
    }

    /// create a Digest from the given 224 bits
    pub fn from_bytes(bytes :[u8;28]) -> Self { Addr(DigestBlake2b224::from_bytes(bytes)) }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ExtendedAddr {
    pub addr: Addr,
    pub attributes: Attributes,
    pub addr_type: AddrType,
}
impl ExtendedAddr {
    pub fn new(ty: AddrType, sd: SpendingData, attrs: Attributes) -> Self {
        ExtendedAddr {
            addr: Addr::new(ty, &sd, &attrs),
            attributes: attrs,
            addr_type: ty
        }
    }

    /// encode an `ExtendedAddr` to cbor with the extra details and `crc32`
    ///
    /// ```
    /// use wallet_crypto::address::{AddrType, ExtendedAddr, SpendingData, Attributes, Addr};
    /// use wallet_crypto::hdwallet;
    /// use wallet_crypto::hdpayload::{HDAddressPayload};
    ///
    /// let seed = hdwallet::Seed::from_bytes([0;32]);
    /// let sk = hdwallet::XPrv::generate_from_seed(&seed);
    /// let pk = sk.public();
    ///
    /// let hdap = HDAddressPayload::from_vec(vec![1,2,3,4,5]);
    /// let addr_type = AddrType::ATPubKey;
    /// let sd = SpendingData::PubKeyASD(pk.clone());
    /// let attrs = Attributes::new_single_key(&pk, Some(hdap));
    ///
    /// let ea = ExtendedAddr::new(addr_type, sd, attrs);
    ///
    /// let out = ea.to_bytes();
    ///
    /// assert_eq!(out.len(), 86); // 86 is the length in this given case.
    /// ```
    ///
    pub fn to_bytes(&self) -> Vec<u8> {
        cbor::encode_to_cbor(self).unwrap()
    }

    /// decode an `ExtendedAddr` to cbor with the extra details and `crc32`
    ///
    /// ```
    /// use wallet_crypto::address::{AddrType, ExtendedAddr, SpendingData, Attributes, Addr};
    /// use wallet_crypto::hdwallet;
    /// use wallet_crypto::hdpayload::{HDAddressPayload};
    ///
    /// let seed = hdwallet::Seed::from_bytes([0;32]);
    /// let sk = hdwallet::XPrv::generate_from_seed(&seed);
    /// let pk = sk.public();
    ///
    /// let hdap = HDAddressPayload::from_vec(vec![1,2,3,4,5]);
    /// let addr_type = AddrType::ATPubKey;
    /// let sd = SpendingData::PubKeyASD(pk.clone());
    /// let attrs = Attributes::new_single_key(&pk, Some(hdap));
    ///
    /// let ea = ExtendedAddr::new(addr_type, sd, attrs);
    ///
    /// let out = ea.to_bytes();
    ///
    /// let r = ExtendedAddr::from_bytes(&out).unwrap();
    /// assert_eq!(ea, r);
    /// ```
    ///
    pub fn from_bytes(buf: &[u8]) -> cbor::Result<Self> {
        cbor::decode_from_cbor(buf)
    }
}
impl cbor::CborValue for ExtendedAddr {
    fn encode(&self) -> cbor::Value {
        cbor::hs::util::encode_with_crc32(&(self.addr.clone(), self.attributes.clone(), self.addr_type.clone()))
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        let (addr, attr, ty) = cbor::hs::util::decode_with_crc32(value)
            .embed("while decoding `ExtendedAddr`")?;
        Ok(ExtendedAddr{addr:addr, attributes: attr, addr_type: ty})
    }
}
impl fmt::Display for ExtendedAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", base58::encode(&self.to_bytes()))
    }
}
impl serde::Serialize for ExtendedAddr
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer,
    {
        let vec = cbor::encode_to_cbor(self).unwrap();
        if serializer.is_human_readable() {
            serializer.serialize_str(&base58::encode(&vec))
        } else {
            serializer.serialize_bytes(&vec)
        }
    }
}
struct XAddrVisitor();
impl XAddrVisitor { fn new() -> Self { XAddrVisitor {} } }
impl<'de> serde::de::Visitor<'de> for XAddrVisitor {
    type Value = ExtendedAddr;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expecting an Extended Address (`ExtendedAddr`)")
    }

    fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
        where E: serde::de::Error
    {
        let bytes = match base58::decode(v) {
            Err(err) => { return Err(E::custom(format!("invalid base58:{}", err))); },
            Ok(v) => v
        };

        match cbor::decode_from_cbor(&bytes) {
            Err((val, err)) => { Err(E::custom(format!("{:?}\n{:?}", err, val))) },
            Ok(v) => Ok(v)
        }
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
        where E: serde::de::Error
    {
        match cbor::decode_from_cbor(v) {
            Err((val, err)) => { Err(E::custom(format!("{:?}\n{:?}", err, val))) },
            Ok(v) => Ok(v)
        }
    }
}
impl<'de> serde::Deserialize<'de> for ExtendedAddr
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(XAddrVisitor::new())
        } else {
            deserializer.deserialize_bytes(XAddrVisitor::new())
        }
    }
}

pub type Script = [u8;32]; // TODO

const SPENDING_DATA_TAG_PUBKEY : u64 = 0;
const SPENDING_DATA_TAG_SCRIPT : u64 = 1; // TODO
const SPENDING_DATA_TAG_REDEEM : u64 = 2; // TODO

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum SpendingData {
    PubKeyASD (XPub),
    ScriptASD (Script),
    RedeemASD (redeem::PublicKey)
    // UnknownASD... whatever...
}
impl cbor::CborValue for SpendingData {
    fn encode(&self) -> cbor::Value {
        let mut v = vec![];
        match self {
            &SpendingData::PubKeyASD(ref pk) => {
                v.push(cbor::CborValue::encode(&SPENDING_DATA_TAG_PUBKEY));
                v.push(cbor::CborValue::encode(pk));
            },
            &SpendingData::ScriptASD(_)      => unimplemented!(),
            &SpendingData::RedeemASD(ref pk) => {
                v.push(cbor::CborValue::encode(&SPENDING_DATA_TAG_REDEEM));
                v.push(cbor::CborValue::encode(pk));
            }
        };
        cbor::Value::Array(v)
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|sum_type| {
            let (sum_type, n) = cbor::array_decode_elem(sum_type, 0)
                .embed("while retrieving the ID of the sum type")?;
            if n == SPENDING_DATA_TAG_PUBKEY {
                let (sum_type, pk) = cbor::array_decode_elem(sum_type, 0)
                    .embed("while decoding the public key")?;
                if sum_type.len() != 0 {
                    return cbor::Result::array(sum_type, cbor::Error::UnparsedValues);
                }
                Ok(SpendingData::PubKeyASD(pk))
            } else if n == SPENDING_DATA_TAG_REDEEM {
                let (sum_type, pk) = cbor::array_decode_elem(sum_type, 0)
                    .embed("while decoding the public key")?;
                if sum_type.len() != 0 {
                    return cbor::Result::array(sum_type, cbor::Error::UnparsedValues);
                }
                Ok(SpendingData::RedeemASD(pk))
            } else {
                cbor::Result::array(sum_type, cbor::Error::InvalidSumtype(n))
            }
        }).embed("while decoding `SpendingData`")
    }
}

#[cfg(test)]
mod tests {
    use address::*;
    use hdwallet;
    use util::base58;

    #[test]
    fn test_make_address() {
        let v    = [ 0x2a, 0xc3, 0xcc, 0x97, 0xbb, 0xec, 0x47, 0x64, 0x96, 0xe8, 0x48, 0x07
                   , 0xf3, 0x5d, 0xf7, 0x34, 0x9a, 0xcf, 0xba, 0xec, 0xe2, 0x00, 0xa2, 0x4b
                   , 0x7e, 0x26, 0x25, 0x0c];
        let addr = Addr::from_bytes(v);

        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let hdap = HDAddressPayload::from_vec(vec![1,2,3,4,5]);
        let addr_type = AddrType::ATPubKey;
        let sd = SpendingData::PubKeyASD(pk.clone());
        let attrs = Attributes::new_single_key(&pk, Some(hdap));

        let ea = ExtendedAddr::new(addr_type, sd, attrs);

        assert_eq!(ea.addr, addr);
    }

    #[test]
    fn test_encode_extended_address() {
        let v = vec![ 0x82, 0xd8, 0x18, 0x58, 0x4c, 0x83, 0x58, 0x1c, 0x2a, 0xc3, 0xcc, 0x97
                    , 0xbb, 0xec, 0x47, 0x64, 0x96, 0xe8, 0x48, 0x07, 0xf3, 0x5d, 0xf7, 0x34
                    , 0x9a, 0xcf, 0xba, 0xec, 0xe2, 0x00, 0xa2, 0x4b, 0x7e, 0x26, 0x25, 0x0c
                    , 0xa2, 0x00, 0x58, 0x20, 0x82, 0x00, 0x58, 0x1c, 0xa6, 0xd9, 0xae, 0xf4
                    , 0x75, 0xf3, 0x41, 0x89, 0x67, 0xe8, 0x7f, 0x7e, 0x93, 0xf2, 0x0f, 0x99
                    , 0xd8, 0xc7, 0xaf, 0x40, 0x6c, 0xba, 0x14, 0x6a, 0xff, 0xdb, 0x71, 0x91
                    , 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x89, 0xa5
                    , 0x93, 0x71
                    ];

        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let hdap = HDAddressPayload::from_vec(vec![1,2,3,4,5]);
        let addr_type = AddrType::ATPubKey;
        let sd = SpendingData::PubKeyASD(pk.clone());
        let attrs = Attributes::new_single_key(&pk, Some(hdap));

        let ea = ExtendedAddr::new(addr_type, sd, attrs);

        let out = cbor::encode_to_cbor(&ea).unwrap();

        v.iter().for_each(|b| {
            if *b < 0x10 { print!("0{:x}", b); } else { print!("{:x}", b); }
        });
        println!("");
        out.iter().for_each(|b| {
            if *b < 0x10 { print!("0{:x}", b); } else { print!("{:x}", b); }
        });
        println!("");

        assert_eq!(v, out);

        let r = ExtendedAddr::from_bytes(&out).unwrap();
        assert_eq!(ea, r);
    }

    #[test]
    fn encode_decode_digest_blake2b() {
        let digest = DigestBlake2b224::new(b"some random bytes...");
        assert!(cbor::hs::encode_decode(&digest))
    }
    #[test]
    fn encode_decode_addr_type() {
        let addr_type_1 = AddrType::ATPubKey;
        let addr_type_2 = AddrType::ATScript;
        let addr_type_3 = AddrType::ATRedeem;
        assert!(cbor::hs::encode_decode(&addr_type_1));
        assert!(cbor::hs::encode_decode(&addr_type_2));
        assert!(cbor::hs::encode_decode(&addr_type_3));
    }
    #[test]
    fn encode_decode_stakeholderid() {
        use hdwallet;
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let si = StakeholderId::new(&pk);
        assert!(cbor::hs::encode_decode(&si));
    }
    #[test]
    fn encode_decode_stakedistribution() {
        use hdwallet;
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let sd_1 = StakeDistribution::new_bootstrap_era();
        let sd_2 = StakeDistribution::new_single_key(&pk);
        assert!(cbor::hs::encode_decode(&sd_1));
        assert!(cbor::hs::encode_decode(&sd_2));
    }

    #[test]
    fn decode_address_1() {
        let addr_str  = "DdzFFzCqrhsyhumccfGyEj3WZzztSPr92ntRWB6UVVwzcMTpwoafVQ5vD9mdZ5Xind8ycugbmA8esxmo7NycjQFGSbDeKrxabTz8MVzf";
        let bytes     = base58::decode(addr_str).unwrap();

        let r = ExtendedAddr::from_bytes(&bytes).unwrap();

        assert_eq!(r.addr_type, AddrType::ATPubKey);
        assert_eq!(r.attributes.stake_distribution, StakeDistribution::BootstrapEraDistr);
    }

    #[test]
    fn decode_address_2() {
        let addr_str  = "DdzFFzCqrhsi8XFMabbnHecVusaebqQCkXTqDnCumx5esKB1pk1zbhX5BtdAivZbQePFVujgzNCpBVXactPSmphuHRC5Xk8qmBd49QjW";
        let bytes     = base58::decode(addr_str).unwrap();

        let r = ExtendedAddr::from_bytes(&bytes).unwrap();

        let b = r.to_bytes();
        assert_eq!(addr_str, base58::encode(&b));

        assert_eq!(r.addr_type, AddrType::ATPubKey);
        assert_eq!(r.attributes.stake_distribution, StakeDistribution::BootstrapEraDistr);
    }
}
