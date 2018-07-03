use std::fmt;
use serde;

use rcw::digest::Digest;
use rcw::blake2b::Blake2b;
use rcw::sha3::Sha3;

use redeem;
use util::{base58};
use cbor;
use cbor_event::{self, de::RawCbor, se::{Serializer}};
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
impl cbor_event::se::Serialize for DigestBlake2b224 {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.write_bytes(self.0.as_ref())
    }
}
impl cbor_event::de::Deserialize for DigestBlake2b224 {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let bytes = raw.bytes()?;
        match DigestBlake2b224::from_slice(&bytes) {
            Some(digest) => Ok(digest),
            None         => Err(cbor_event::Error::NotEnough(bytes.len(), 24)),
        }
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
impl cbor_event::se::Serialize for AddrType {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.write_unsigned_integer(self.to_byte() as u64)
    }
}
impl cbor_event::de::Deserialize for AddrType {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        match AddrType::from_u64(raw.unsigned_integer()?) {
            Some(addr_type) => Ok(addr_type),
            None => Err(cbor_event::Error::CustomError(format!("Invalid AddrType")))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct StakeholderId(DigestBlake2b224); // of publickey (block2b 256)
impl StakeholderId {
    pub fn new(pubk: &XPub) -> StakeholderId {
        let buf = cbor!(pubk).unwrap();
        StakeholderId(DigestBlake2b224::new(buf.as_ref()))
    }
}
impl cbor_event::se::Serialize for StakeholderId {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        cbor_event::se::Serialize::serialize(&self.0, serializer)
    }
}
impl cbor_event::de::Deserialize for StakeholderId {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(StakeholderId(cbor_event::de::Deserialize::deserialize(raw)?))
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
impl cbor_event::se::Serialize for StakeDistribution {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        let inner_serializer = match self {
            &StakeDistribution::BootstrapEraDistr => {
                Serializer::new_vec().write_array(cbor_event::Len::Len(1))?
                    .write_unsigned_integer(STAKE_DISTRIBUTION_TAG_BOOTSTRAP)?
            }
            &StakeDistribution::SingleKeyDistr(ref si) => {
                Serializer::new_vec().write_array(cbor_event::Len::Len(2))?
                    .write_unsigned_integer(STAKE_DISTRIBUTION_TAG_SINGLEKEY)?
                    .serialize(si)?
            }
        };
        serializer.write_bytes(&inner_serializer.finalize())
    }
}
impl cbor_event::de::Deserialize for StakeDistribution {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        // stake distribution is an encoded cbor in bytes of a sum_type...
        let mut raw = RawCbor::from(&raw.bytes()?);
        let len = raw.array()?;
        if len != cbor_event::Len::Len(1) && len != cbor_event::Len::Len(2) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Stakedistribution: recieved array of {:?} elements", len)));
        }

        let sum_type_idx = raw.unsigned_integer()?;
        match sum_type_idx {
            STAKE_DISTRIBUTION_TAG_BOOTSTRAP => Ok(StakeDistribution::new_bootstrap_era()),
            STAKE_DISTRIBUTION_TAG_SINGLEKEY => {
                let k = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                Ok(StakeDistribution::new_single_stakeholder(k))
            },
            _ => {
                Err(cbor_event::Error::CustomError(format!("Unsupported StakeDistribution: {}", sum_type_idx)))
            }
        }
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

impl cbor_event::se::Serialize for Attributes {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        let mut len = 0;
        match &self.stake_distribution {
            &StakeDistribution::BootstrapEraDistr => {},
            &StakeDistribution::SingleKeyDistr(_) => {len += 1 }
        };
        match &self.derivation_path {
            &None => { },
            &Some(_) => { len += 1 }
        };
        let serializer = serializer.write_map(cbor_event::Len::Len(len))?;
        let serializer = match &self.stake_distribution {
            &StakeDistribution::BootstrapEraDistr => { serializer },
            &StakeDistribution::SingleKeyDistr(_) => {
                serializer.write_unsigned_integer(ATTRIBUTE_NAME_TAG_STAKE)?
                          .serialize(&self.stake_distribution)?
            },
        };
        match &self.derivation_path {
            &None => { Ok(serializer) },
            &Some(ref dp) => {
                serializer.write_unsigned_integer(ATTRIBUTE_NAME_TAG_DERIVATION)?
                          .serialize(dp)
            }
        }
    }
}
impl cbor_event::de::Deserialize for Attributes {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.map()?;
        let mut len = match len {
            cbor_event::Len::Indefinite => {
               return Err(cbor_event::Error::CustomError(format!("Invalid Attribytes: recieved map of {:?} elements", len)));
            },
            cbor_event::Len::Len(len) => len
        };
        let mut stake_distribution = StakeDistribution::BootstrapEraDistr;
        let mut derivation_path = None;
        while len > 0 {
            let key = raw.unsigned_integer()?;
            match key {
                0 => stake_distribution = cbor_event::de::Deserialize::deserialize(raw)?,
                1 => derivation_path    = Some(cbor_event::de::Deserialize::deserialize(raw)?),
                _ => {
                    return Err(cbor_event::Error::CustomError(format!("invalid Attribute key {}", key)));
                }
            }
            len -= 1;
        }
        Ok(Attributes { derivation_path, stake_distribution })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Addr(DigestBlake2b224);
impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl cbor_event::se::Serialize for Addr {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.serialize(&self.0)
    }
}
impl cbor_event::de::Deserialize for Addr {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        cbor_event::de::Deserialize::deserialize(raw).map(|digest| Addr(digest))
    }
}
impl Addr {
    pub fn new(addr_type: AddrType, spending_data: &SpendingData, attrs: &Attributes) -> Self {
        Addr(
            DigestBlake2b224::new(
                &cbor!(&(&addr_type, spending_data, attrs))
                    .expect("serialize the Addr's digest data")
            )
        )
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

    // bootstrap era + no hdpayload address
    pub fn new_simple(xpub: XPub) -> Self {
        ExtendedAddr::new(AddrType::ATPubKey, SpendingData::PubKeyASD(xpub), Attributes::new_bootstrap_era(None))
    }

    /// encode an `ExtendedAddr` to cbor with the extra details and `crc32`
    ///
    /// ```
    /// use cardano::address::{AddrType, ExtendedAddr, SpendingData, Attributes, Addr};
    /// use cardano::hdwallet;
    /// use cardano::hdpayload::{HDAddressPayload};
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
        cbor!(self).expect("serialising ExtendedAddr into cbor")
    }

    /// decode an `ExtendedAddr` to cbor with the extra details and `crc32`
    ///
    /// ```
    /// use cardano::address::{AddrType, ExtendedAddr, SpendingData, Attributes, Addr};
    /// use cardano::hdwallet;
    /// use cardano::hdpayload::{HDAddressPayload};
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
    pub fn from_bytes(buf: &[u8]) -> cbor_event::Result<Self> {
        let mut raw = RawCbor::from(buf);
        cbor_event::de::Deserialize::deserialize(&mut raw)
    }
}
impl cbor_event::se::Serialize for ExtendedAddr {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        cbor::hs::util::encode_with_crc32_(&(&self.addr, &self.attributes, &self.addr_type), serializer)
    }
}
impl cbor_event::de::Deserialize for ExtendedAddr {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let bytes = cbor::hs::util::raw_with_crc32(raw)?;
        let mut raw = RawCbor::from(&bytes);
        let len = raw.array()?;
        if len != cbor_event::Len::Len(3) {
            return Err(cbor_event::Error::CustomError(format!("Invalid ExtendedAddr: recieved array of {:?} elements", len)));
        }
        let addr = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        let attributes = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        let addr_type = cbor_event::de::Deserialize::deserialize(&mut raw)?;

        Ok(ExtendedAddr { addr, addr_type, attributes })
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
        let vec = self.to_bytes();
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

        match Self::Value::from_bytes(&bytes) {
            Err(err) => { Err(E::custom(format!("unable to parse ExtendedAddr: {:?}", err))) },
            Ok(v) => Ok(v)
        }
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
        where E: serde::de::Error
    {
        match Self::Value::from_bytes(v) {
            Err(err) => { Err(E::custom(format!("unable to parse ExtendedAddr: {:?}", err))) },
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
const SPENDING_DATA_TAG_REDEEM : u64 = 2;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum SpendingData {
    PubKeyASD (XPub),
    ScriptASD (Script),
    RedeemASD (redeem::PublicKey)
    // UnknownASD... whatever...
}
impl cbor_event::se::Serialize for SpendingData {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        match self {
            &SpendingData::PubKeyASD(ref pk) => {
                serializer.write_array(cbor_event::Len::Len(2))?
                          .write_unsigned_integer(SPENDING_DATA_TAG_PUBKEY)?
                          .serialize(pk)
            },
            &SpendingData::ScriptASD(_)      => unimplemented!(),
            &SpendingData::RedeemASD(ref pk) => {
                serializer.write_array(cbor_event::Len::Len(2))?
                          .write_unsigned_integer(SPENDING_DATA_TAG_REDEEM)?
                          .serialize(pk)
            }
        }
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

        let out = ea.to_bytes();

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
        assert!(cbor_event::test_encode_decode(&digest).expect("encode/decode DigestBlake2b224"));
    }
    #[test]
    fn encode_decode_addr_type() {
        let addr_type_1 = AddrType::ATPubKey;
        let addr_type_2 = AddrType::ATScript;
        let addr_type_3 = AddrType::ATRedeem;
        assert!(cbor_event::test_encode_decode(&addr_type_1).expect("encode/decode AddrType::ATPubKey"));
        assert!(cbor_event::test_encode_decode(&addr_type_2).expect("encode/decode AddrType::ATScript"));
        assert!(cbor_event::test_encode_decode(&addr_type_3).expect("encode/decode AddrType::ATRedeem"));
    }
    #[test]
    fn encode_decode_stakeholderid() {
        use hdwallet;
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let si = StakeholderId::new(&pk);
        assert!(cbor_event::test_encode_decode(&si).expect("encode/decode StakeholderId"));
    }
    #[test]
    fn encode_decode_stakedistribution() {
        use hdwallet;
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let sd_1 = StakeDistribution::new_bootstrap_era();
        let sd_2 = StakeDistribution::new_single_key(&pk);
        assert!(cbor_event::test_encode_decode(&sd_1).expect("encode/decode StakeDistribution::BootstrapEra"));
        assert!(cbor_event::test_encode_decode(&sd_2).expect("encode/decode StakeDistribution::SingleKey"));
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

    #[test]
    fn decode_address_no_derivation_path() {
        let bytes     = vec![0x82, 0xd8, 0x18, 0x58, 0x21, 0x83, 0x58, 0x1c, 0x10, 0x2a, 0x74, 0xca, 0x44, 0x05, 0xb8, 0xc1, 0x8d, 0x20, 0x84, 0x1e, 0x8c, 0x66, 0x4f, 0xe1, 0xde, 0x7d, 0x66, 0x07, 0x48, 0x08, 0x70, 0x4f, 0x91, 0x79, 0xe0, 0xfa, 0xa0, 0x00, 0x1a, 0xad, 0xf7, 0x10, 0x68];

        let r = ExtendedAddr::from_bytes(&bytes).unwrap();

        assert_eq!(r.addr_type, AddrType::ATPubKey);
        assert_eq!(r.attributes.stake_distribution, StakeDistribution::BootstrapEraDistr);
        assert_eq!(bytes, r.to_bytes());
    }
}

#[cfg(feature = "with-bench")]
#[cfg(test)]
mod bench {
    use address::*;
    use hdwallet;
    use util::base58;
    use cbor_event::de::{RawCbor};

    const CBOR : &[u8] =
        &[ 0x82, 0xd8, 0x18, 0x58, 0x4c, 0x83, 0x58, 0x1c, 0x2a, 0xc3, 0xcc, 0x97
         , 0xbb, 0xec, 0x47, 0x64, 0x96, 0xe8, 0x48, 0x07, 0xf3, 0x5d, 0xf7, 0x34
         , 0x9a, 0xcf, 0xba, 0xec, 0xe2, 0x00, 0xa2, 0x4b, 0x7e, 0x26, 0x25, 0x0c
         , 0xa2, 0x00, 0x58, 0x20, 0x82, 0x00, 0x58, 0x1c, 0xa6, 0xd9, 0xae, 0xf4
         , 0x75, 0xf3, 0x41, 0x89, 0x67, 0xe8, 0x7f, 0x7e, 0x93, 0xf2, 0x0f, 0x99
         , 0xd8, 0xc7, 0xaf, 0x40, 0x6c, 0xba, 0x14, 0x6a, 0xff, 0xdb, 0x71, 0x91
         , 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x89, 0xa5
         , 0x93, 0x71
         ];

    use test;

    #[bench]
    fn encode_address_cbor_raw(b: &mut test::Bencher) {
        let mut raw = cbor_event::de::RawCbor::from(CBOR);
        let addr : ExtendedAddr = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();
        b.iter(|| {
            let _ = cbor!(addr).unwrap();
        })
    }
    #[bench]
    fn decode_address_cbor_raw(b: &mut test::Bencher) {
        b.iter(|| {
            let _ : ExtendedAddr = RawCbor::from(CBOR).deserialize().unwrap();
        })
    }
}
