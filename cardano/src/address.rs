//! Address creation and parsing
//!
//! Address components are:
//! * `HashedSpendingData` computed from `SpendingData`
//! * `Attributes`
//! * `AddrType`
//!
//! All this components form an `ExtendedAddr`, which serialized
//! to binary makes an `Addr`
//!
#[cfg(feature = "generic-serialization")]
use serde;

use hash::{Blake2b224, Sha3_256};

use cbor;
use cbor_event::{self, de::Deserializer, se::Serializer};
use config::NetworkMagic;
use hdpayload::HDAddressPayload;
use hdwallet::XPub;
use redeem;
use util::{base58, try_from_slice::TryFromSlice};

use std::{
    fmt,
    io::{BufRead, Write},
    str::FromStr,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum AddrType {
    ATPubKey,
    ATScript,
    ATRedeem,
}
impl fmt::Display for AddrType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddrType::ATPubKey => write!(f, "Public Key"),
            AddrType::ATScript => write!(f, "Script"),
            AddrType::ATRedeem => write!(f, "Redeem"),
        }
    }
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
            AddrType::ATRedeem => 2,
        }
    }
}
impl cbor_event::se::Serialize for AddrType {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_unsigned_integer(self.to_byte() as u64)
    }
}
impl cbor_event::de::Deserialize for AddrType {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        match AddrType::from_u64(reader.unsigned_integer()?) {
            Some(addr_type) => Ok(addr_type),
            None => Err(cbor_event::Error::CustomError(format!("Invalid AddrType"))),
        }
    }
}

/// StakeholderId is the transaction
///
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct StakeholderId(Blake2b224);
impl StakeholderId {
    pub fn new(pubk: &XPub) -> StakeholderId {
        // the reason for this unwrap is that we have to dynamically allocate 66 bytes
        // to serialize 64 bytes in cbor (2 bytes of cbor overhead).
        let buf = cbor!(pubk).unwrap();

        let hash = Sha3_256::new(&buf);
        StakeholderId(Blake2b224::new(hash.as_ref()))
    }

    pub fn as_hash_bytes(&self) -> &[u8; Blake2b224::HASH_SIZE] {
        self.0.as_hash_bytes()
    }
}
impl cbor_event::se::Serialize for StakeholderId {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        cbor_event::se::Serialize::serialize(&self.0, serializer)
    }
}
impl cbor_event::de::Deserialize for StakeholderId {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        Ok(StakeholderId(cbor_event::de::Deserialize::deserialize(
            reader,
        )?))
    }
}
impl fmt::Display for StakeholderId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl TryFromSlice for StakeholderId {
    type Error = <Blake2b224 as TryFromSlice>::Error;
    fn try_from_slice(slice: &[u8]) -> ::std::result::Result<Self, Self::Error> {
        Ok(Self::from(Blake2b224::try_from_slice(slice)?))
    }
}
impl AsRef<[u8]> for StakeholderId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl From<StakeholderId> for [u8; Blake2b224::HASH_SIZE] {
    fn from(hash: StakeholderId) -> Self {
        hash.0.into()
    }
}
impl From<[u8; Blake2b224::HASH_SIZE]> for StakeholderId {
    fn from(hash: [u8; Blake2b224::HASH_SIZE]) -> Self {
        StakeholderId(Blake2b224::from(hash))
    }
}
impl From<Blake2b224> for StakeholderId {
    fn from(hash: Blake2b224) -> Self {
        StakeholderId(hash)
    }
}
impl FromStr for StakeholderId {
    type Err = <Blake2b224 as FromStr>::Err;
    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        Ok(Self::from(Blake2b224::from_str(s)?))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum StakeDistribution {
    BootstrapEraDistr,
    SingleKeyDistr(StakeholderId),
}

const STAKE_DISTRIBUTION_TAG_BOOTSTRAP: u64 = 1;
const STAKE_DISTRIBUTION_TAG_SINGLEKEY: u64 = 0;

impl StakeDistribution {
    pub fn new_bootstrap_era() -> Self {
        StakeDistribution::BootstrapEraDistr
    }
    pub fn new_single_stakeholder(si: StakeholderId) -> Self {
        StakeDistribution::SingleKeyDistr(si)
    }
    pub fn new_single_key(pubk: &XPub) -> Self {
        StakeDistribution::new_single_stakeholder(StakeholderId::new(pubk))
    }
}
impl cbor_event::se::Serialize for StakeDistribution {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let inner_cbor = match self {
            &StakeDistribution::BootstrapEraDistr => {
                let mut se = Serializer::new_vec();
                se.write_array(cbor_event::Len::Len(1))?
                    .write_unsigned_integer(STAKE_DISTRIBUTION_TAG_BOOTSTRAP)?;
                se.finalize()
            }
            &StakeDistribution::SingleKeyDistr(ref si) => {
                let mut se = Serializer::new_vec();
                se.write_array(cbor_event::Len::Len(2))?
                    .write_unsigned_integer(STAKE_DISTRIBUTION_TAG_SINGLEKEY)?
                    .serialize(si)?;
                se.finalize()
            }
        };
        serializer.write_bytes(&inner_cbor)
    }
}
impl cbor_event::de::Deserialize for StakeDistribution {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        // stake distribution is an encoded cbor in bytes of a sum_type...
        let mut raw = Deserializer::from(std::io::Cursor::new(reader.bytes()?));
        let len = raw.array()?;
        if len != cbor_event::Len::Len(1) && len != cbor_event::Len::Len(2) {
            return Err(cbor_event::Error::CustomError(format!(
                "Invalid Stakedistribution: recieved array of {:?} elements",
                len
            )));
        }

        let sum_type_idx = raw.unsigned_integer()?;
        match sum_type_idx {
            STAKE_DISTRIBUTION_TAG_BOOTSTRAP => Ok(StakeDistribution::new_bootstrap_era()),
            STAKE_DISTRIBUTION_TAG_SINGLEKEY => {
                let k = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                Ok(StakeDistribution::new_single_stakeholder(k))
            }
            _ => Err(cbor_event::Error::CustomError(format!(
                "Unsupported StakeDistribution: {}",
                sum_type_idx
            ))),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct Attributes {
    pub derivation_path: Option<HDAddressPayload>,
    pub stake_distribution: StakeDistribution,
    pub network_magic: NetworkMagic,
    // attr_remains ? whatever...
}
impl Attributes {
    pub fn new_bootstrap_era(hdap: Option<HDAddressPayload>, network_magic: NetworkMagic) -> Self {
        Attributes {
            derivation_path: hdap,
            stake_distribution: StakeDistribution::BootstrapEraDistr,
            network_magic,
        }
    }
    pub fn new_single_key(
        pubk: &XPub,
        hdap: Option<HDAddressPayload>,
        network_magic: NetworkMagic,
    ) -> Self {
        Attributes {
            derivation_path: hdap,
            stake_distribution: StakeDistribution::new_single_key(pubk),
            network_magic,
        }
    }
}

const ATTRIBUTE_NAME_TAG_STAKE: u64 = 0;
const ATTRIBUTE_NAME_TAG_DERIVATION: u64 = 1;
const ATTRIBUTE_NAME_TAG_NETWORK_MAGIC: u64 = 2;

impl cbor_event::se::Serialize for Attributes {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let mut len = 0;
        match &self.stake_distribution {
            &StakeDistribution::BootstrapEraDistr => {}
            &StakeDistribution::SingleKeyDistr(_) => len += 1,
        };
        if let Some(_) = &self.derivation_path {
            len += 1
        };
        if let NetworkMagic::Magic(_) = &self.network_magic {
            len += 1
        };
        let serializer = serializer.write_map(cbor_event::Len::Len(len))?;
        let serializer = match &self.stake_distribution {
            &StakeDistribution::BootstrapEraDistr => serializer,
            &StakeDistribution::SingleKeyDistr(_) => serializer
                .write_unsigned_integer(ATTRIBUTE_NAME_TAG_STAKE)?
                .serialize(&self.stake_distribution)?,
        };
        let serializer = match &self.derivation_path {
            &None => serializer,
            &Some(ref dp) => serializer
                .write_unsigned_integer(ATTRIBUTE_NAME_TAG_DERIVATION)?
                .serialize(dp)?,
        };
        let serializer = match &self.network_magic {
            &NetworkMagic::NoMagic => serializer,
            &NetworkMagic::Magic(network_magic) => serializer
                .write_unsigned_integer(ATTRIBUTE_NAME_TAG_NETWORK_MAGIC)?
                .write_bytes(cbor!(&(network_magic as u32))?)?,
        };
        Ok(serializer)
    }
}
impl cbor_event::de::Deserialize for Attributes {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        let len = reader.map()?;
        let mut len = match len {
            cbor_event::Len::Indefinite => {
                return Err(cbor_event::Error::CustomError(format!(
                    "Invalid Attributes: received map of {:?} elements",
                    len
                )));
            }
            cbor_event::Len::Len(len) => len,
        };
        let mut stake_distribution = StakeDistribution::BootstrapEraDistr;
        let mut derivation_path = None;
        let mut network_magic = NetworkMagic::NoMagic;
        while len > 0 {
            let key = reader.unsigned_integer()?;
            match key {
                ATTRIBUTE_NAME_TAG_STAKE => stake_distribution = reader.deserialize()?,
                ATTRIBUTE_NAME_TAG_DERIVATION => derivation_path = Some(reader.deserialize()?),
                ATTRIBUTE_NAME_TAG_NETWORK_MAGIC => {
                    // Yes, this is an integer encoded as CBOR encoded as Bytes in CBOR.
                    let bytes = reader.bytes()?;
                    let n = Deserializer::from(std::io::Cursor::new(bytes)).deserialize::<u32>()?;
                    network_magic = NetworkMagic::Magic(n);
                }
                _ => {
                    return Err(cbor_event::Error::CustomError(format!(
                        "invalid Attribute key {}",
                        key
                    )));
                }
            }
            len -= 1;
        }
        Ok(Attributes {
            derivation_path,
            stake_distribution,
            network_magic,
        })
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct HashedSpendingData(Blake2b224);

impl HashedSpendingData {
    pub fn new(addr_type: AddrType, spending_data: &SpendingData, attrs: &Attributes) -> Self {
        // the reason for this unwrap is that we have to dynamically allocate 66 bytes
        // to serialize 64 bytes in cbor (2 bytes of cbor overhead).
        let buf = cbor!(&(&addr_type, spending_data, attrs))
            .expect("serialize the HashedSpendingData's digest data");

        let hash = Sha3_256::new(&buf);
        HashedSpendingData(Blake2b224::new(hash.as_ref()))
    }

    pub fn as_hash_bytes(&self) -> &[u8; Blake2b224::HASH_SIZE] {
        self.0.as_hash_bytes()
    }
}
impl fmt::Display for HashedSpendingData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl cbor_event::se::Serialize for HashedSpendingData {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.serialize(&self.0)
    }
}
impl cbor_event::de::Deserialize for HashedSpendingData {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        cbor_event::de::Deserialize::deserialize(reader).map(|digest| HashedSpendingData(digest))
    }
}
impl TryFromSlice for HashedSpendingData {
    type Error = <Blake2b224 as TryFromSlice>::Error;
    fn try_from_slice(slice: &[u8]) -> ::std::result::Result<Self, Self::Error> {
        Ok(Self::from(Blake2b224::try_from_slice(slice)?))
    }
}
impl AsRef<[u8]> for HashedSpendingData {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl From<HashedSpendingData> for [u8; Blake2b224::HASH_SIZE] {
    fn from(hash: HashedSpendingData) -> Self {
        hash.0.into()
    }
}
impl From<[u8; Blake2b224::HASH_SIZE]> for HashedSpendingData {
    fn from(hash: [u8; Blake2b224::HASH_SIZE]) -> Self {
        HashedSpendingData(Blake2b224::from(hash))
    }
}
impl From<Blake2b224> for HashedSpendingData {
    fn from(hash: Blake2b224) -> Self {
        HashedSpendingData(hash)
    }
}
impl FromStr for HashedSpendingData {
    type Err = <Blake2b224 as FromStr>::Err;
    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        Ok(Self::from(Blake2b224::from_str(s)?))
    }
}

/// A valid cardano Address that is displayed in base58
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Addr(Vec<u8>);

impl Addr {
    pub fn deconstruct(&self) -> ExtendedAddr {
        let mut raw = Deserializer::from(std::io::Cursor::new(&self.0));
        cbor_event::de::Deserialize::deserialize(&mut raw).unwrap() // unwrap should never fail from addr to extended addr
    }
}

impl AsRef<[u8]> for Addr {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl TryFromSlice for Addr {
    type Error = cbor_event::Error;
    fn try_from_slice(slice: &[u8]) -> ::std::result::Result<Self, Self::Error> {
        let mut v = Vec::new();
        // TODO we only want validation of slice here, but we don't have api to do that yet.
        {
            let mut raw = Deserializer::from(std::io::Cursor::new(&slice));
            let _: ExtendedAddr = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        }
        v.extend_from_slice(slice);
        Ok(Addr(v))
    }
}

impl From<ExtendedAddr> for Addr {
    fn from(ea: ExtendedAddr) -> Self {
        ea.to_address()
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", base58::encode(&self.0))
    }
}

impl cbor_event::se::Serialize for Addr {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        // Addr is already serialized
        serializer.write_raw_bytes(&self.0)
    }
}
impl cbor_event::de::Deserialize for Addr {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        let ea: ExtendedAddr = cbor_event::de::Deserialize::deserialize(reader)?;
        Ok(ea.to_address())
    }
}

#[cfg(feature = "generic-serialization")]
impl serde::Serialize for Addr {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let vec = cbor!(self).unwrap();
        if serializer.is_human_readable() {
            serializer.serialize_str(&base58::encode(&vec))
        } else {
            serializer.serialize_bytes(&vec)
        }
    }
}
#[cfg(feature = "generic-serialization")]
impl<'de> serde::Deserialize<'de> for Addr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct XAddrVisitor;
        impl<'de> serde::de::Visitor<'de> for XAddrVisitor {
            type Value = Addr;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "Expecting an Address (`Addr`)")
            }

            fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let bytes = match base58::decode(v) {
                    Err(err) => {
                        return Err(E::custom(format!("invalid base58:{}", err)));
                    }
                    Ok(v) => v,
                };

                match Self::Value::try_from_slice(&bytes) {
                    Err(err) => Err(E::custom(format!("unable to parse Addr: {:?}", err))),
                    Ok(v) => Ok(v),
                }
            }

            fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match Self::Value::try_from_slice(v) {
                    Err(err) => Err(E::custom(format!("unable to parse Addr: {:?}", err))),
                    Ok(v) => Ok(v),
                }
            }
        }
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(XAddrVisitor)
        } else {
            deserializer.deserialize_bytes(XAddrVisitor)
        }
    }
}

/// A valid cardano address deconstructed
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExtendedAddr {
    pub addr: HashedSpendingData,
    pub attributes: Attributes,
    pub addr_type: AddrType,
}
impl ExtendedAddr {
    pub fn new(ty: AddrType, sd: SpendingData, attrs: Attributes) -> Self {
        ExtendedAddr {
            addr: HashedSpendingData::new(ty, &sd, &attrs),
            attributes: attrs,
            addr_type: ty,
        }
    }

    // bootstrap era + no hdpayload address
    pub fn new_simple(xpub: XPub, network_magic: NetworkMagic) -> Self {
        ExtendedAddr::new(
            AddrType::ATPubKey,
            SpendingData::PubKeyASD(xpub),
            Attributes::new_bootstrap_era(None, network_magic),
        )
    }

    pub fn to_address(&self) -> Addr {
        Addr(cbor!(self).unwrap()) // unwrap should never fail from strongly typed extended addr to addr
    }
}
#[derive(Debug)]
pub enum ParseExtendedAddrError {
    EncodingError(cbor_event::Error),
    Base58Error(base58::Error),
}
impl ::std::str::FromStr for ExtendedAddr {
    type Err = ParseExtendedAddrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = base58::decode(s).map_err(ParseExtendedAddrError::Base58Error)?;

        Self::try_from_slice(&bytes).map_err(ParseExtendedAddrError::EncodingError)
    }
}
impl TryFromSlice for ExtendedAddr {
    type Error = cbor_event::Error;
    fn try_from_slice(slice: &[u8]) -> ::std::result::Result<Self, Self::Error> {
        let mut raw = Deserializer::from(std::io::Cursor::new(slice));
        cbor_event::de::Deserialize::deserialize(&mut raw)
    }
}
impl cbor_event::se::Serialize for ExtendedAddr {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        cbor::hs::util::encode_with_crc32_(
            &(&self.addr, &self.attributes, &self.addr_type),
            serializer,
        )?;
        Ok(serializer)
    }
}
impl cbor_event::de::Deserialize for ExtendedAddr {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        let bytes = cbor::hs::util::raw_with_crc32(reader)?;
        let mut raw = Deserializer::from(std::io::Cursor::new(bytes));
        raw.tuple(3, "ExtendedAddr")?;
        let addr = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        let attributes = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        let addr_type = cbor_event::de::Deserialize::deserialize(&mut raw)?;

        Ok(ExtendedAddr {
            addr,
            addr_type,
            attributes,
        })
    }
}
impl fmt::Display for ExtendedAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_address())
    }
}
#[cfg(feature = "generic-serialization")]
impl serde::Serialize for ExtendedAddr {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let vec = cbor!(self).unwrap();
        if serializer.is_human_readable() {
            serializer.serialize_str(&base58::encode(&vec))
        } else {
            serializer.serialize_bytes(&vec)
        }
    }
}
#[cfg(feature = "generic-serialization")]
impl<'de> serde::Deserialize<'de> for ExtendedAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct XAddrVisitor;
        impl<'de> serde::de::Visitor<'de> for XAddrVisitor {
            type Value = ExtendedAddr;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "Expecting an Extended Address (`ExtendedAddr`)")
            }

            fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let bytes = match base58::decode(v) {
                    Err(err) => {
                        return Err(E::custom(format!("invalid base58:{}", err)));
                    }
                    Ok(v) => v,
                };

                match Self::Value::try_from_slice(&bytes) {
                    Err(err) => Err(E::custom(format!(
                        "unable to parse ExtendedAddr: {:?}",
                        err
                    ))),
                    Ok(v) => Ok(v),
                }
            }

            fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match Self::Value::try_from_slice(v) {
                    Err(err) => Err(E::custom(format!(
                        "unable to parse ExtendedAddr: {:?}",
                        err
                    ))),
                    Ok(v) => Ok(v),
                }
            }
        }
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(XAddrVisitor)
        } else {
            deserializer.deserialize_bytes(XAddrVisitor)
        }
    }
}

pub type Script = [u8; 32]; // TODO

const SPENDING_DATA_TAG_PUBKEY: u64 = 0;
const SPENDING_DATA_TAG_SCRIPT: u64 = 1; // TODO
const SPENDING_DATA_TAG_REDEEM: u64 = 2;

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum SpendingData {
    PubKeyASD(XPub),
    ScriptASD(Script),
    RedeemASD(redeem::PublicKey), // UnknownASD... whatever...
}
impl cbor_event::se::Serialize for SpendingData {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        match self {
            &SpendingData::PubKeyASD(ref pk) => serializer
                .write_array(cbor_event::Len::Len(2))?
                .write_unsigned_integer(SPENDING_DATA_TAG_PUBKEY)?
                .serialize(pk),
            &SpendingData::ScriptASD(_) => {
                serializer
                    .write_array(cbor_event::Len::Len(2))?
                    .write_unsigned_integer(SPENDING_DATA_TAG_SCRIPT)?;
                unimplemented!()
            }
            &SpendingData::RedeemASD(ref pk) => serializer
                .write_array(cbor_event::Len::Len(2))?
                .write_unsigned_integer(SPENDING_DATA_TAG_REDEEM)?
                .serialize(pk),
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
        let v = [
            0x2a, 0xc3, 0xcc, 0x97, 0xbb, 0xec, 0x47, 0x64, 0x96, 0xe8, 0x48, 0x07, 0xf3, 0x5d,
            0xf7, 0x34, 0x9a, 0xcf, 0xba, 0xec, 0xe2, 0x00, 0xa2, 0x4b, 0x7e, 0x26, 0x25, 0x0c,
        ];
        let addr = HashedSpendingData::from(v);

        let seed = hdwallet::Seed::from_bytes([0; hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let hdap = HDAddressPayload::from_vec(vec![1, 2, 3, 4, 5]);
        let addr_type = AddrType::ATPubKey;
        let sd = SpendingData::PubKeyASD(pk.clone());
        let attrs = Attributes::new_single_key(&pk, Some(hdap), NetworkMagic::NoMagic);

        let ea = ExtendedAddr::new(addr_type, sd, attrs);

        assert_eq!(ea.addr, addr);
    }

    #[test]
    fn test_encode_extended_address() {
        let v = vec![
            0x82, 0xd8, 0x18, 0x58, 0x4c, 0x83, 0x58, 0x1c, 0x2a, 0xc3, 0xcc, 0x97, 0xbb, 0xec,
            0x47, 0x64, 0x96, 0xe8, 0x48, 0x07, 0xf3, 0x5d, 0xf7, 0x34, 0x9a, 0xcf, 0xba, 0xec,
            0xe2, 0x00, 0xa2, 0x4b, 0x7e, 0x26, 0x25, 0x0c, 0xa2, 0x00, 0x58, 0x20, 0x82, 0x00,
            0x58, 0x1c, 0xa6, 0xd9, 0xae, 0xf4, 0x75, 0xf3, 0x41, 0x89, 0x67, 0xe8, 0x7f, 0x7e,
            0x93, 0xf2, 0x0f, 0x99, 0xd8, 0xc7, 0xaf, 0x40, 0x6c, 0xba, 0x14, 0x6a, 0xff, 0xdb,
            0x71, 0x91, 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x89, 0xa5,
            0x93, 0x71,
        ];

        let seed = hdwallet::Seed::from_bytes([0; hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let hdap = HDAddressPayload::from_vec(vec![1, 2, 3, 4, 5]);
        let addr_type = AddrType::ATPubKey;
        let sd = SpendingData::PubKeyASD(pk.clone());
        let attrs = Attributes::new_single_key(&pk, Some(hdap), NetworkMagic::NoMagic);

        let ea = ExtendedAddr::new(addr_type, sd, attrs);

        let out = cbor!(ea).unwrap();

        v.iter().for_each(|b| {
            if *b < 0x10 {
                print!("0{:x}", b);
            } else {
                print!("{:x}", b);
            }
        });
        println!("");
        out.iter().for_each(|b| {
            if *b < 0x10 {
                print!("0{:x}", b);
            } else {
                print!("{:x}", b);
            }
        });
        println!("");

        assert_eq!(v, out);

        let r = ExtendedAddr::try_from_slice(&out).unwrap();
        assert_eq!(ea, r);
    }

    #[test]
    fn encode_decode_addr_type() {
        let addr_type_1 = AddrType::ATPubKey;
        let addr_type_2 = AddrType::ATScript;
        let addr_type_3 = AddrType::ATRedeem;
        assert!(
            cbor_event::test_encode_decode(&addr_type_1).expect("encode/decode AddrType::ATPubKey")
        );
        assert!(
            cbor_event::test_encode_decode(&addr_type_2).expect("encode/decode AddrType::ATScript")
        );
        assert!(
            cbor_event::test_encode_decode(&addr_type_3).expect("encode/decode AddrType::ATRedeem")
        );
    }
    #[test]
    fn encode_decode_stakeholderid() {
        use hdwallet;
        let seed = hdwallet::Seed::from_bytes([0; hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let si = StakeholderId::new(&pk);
        assert!(cbor_event::test_encode_decode(&si).expect("encode/decode StakeholderId"));
    }
    #[test]
    fn encode_decode_stakedistribution() {
        use hdwallet;
        let seed = hdwallet::Seed::from_bytes([0; hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let sd_1 = StakeDistribution::new_bootstrap_era();
        let sd_2 = StakeDistribution::new_single_key(&pk);
        assert!(cbor_event::test_encode_decode(&sd_1)
            .expect("encode/decode StakeDistribution::BootstrapEra"));
        assert!(cbor_event::test_encode_decode(&sd_2)
            .expect("encode/decode StakeDistribution::SingleKey"));
    }

    #[test]
    fn decode_address_1() {
        let addr_str  = "DdzFFzCqrhsyhumccfGyEj3WZzztSPr92ntRWB6UVVwzcMTpwoafVQ5vD9mdZ5Xind8ycugbmA8esxmo7NycjQFGSbDeKrxabTz8MVzf";
        let bytes = base58::decode(addr_str).unwrap();

        let r = ExtendedAddr::try_from_slice(&bytes).unwrap();

        assert_eq!(r.addr_type, AddrType::ATPubKey);
        assert_eq!(
            r.attributes.stake_distribution,
            StakeDistribution::BootstrapEraDistr
        );
        assert_eq!(r.attributes.network_magic, NetworkMagic::NoMagic);
    }

    #[test]
    fn decode_address_2() {
        let addr_str  = "DdzFFzCqrhsi8XFMabbnHecVusaebqQCkXTqDnCumx5esKB1pk1zbhX5BtdAivZbQePFVujgzNCpBVXactPSmphuHRC5Xk8qmBd49QjW";
        let bytes = base58::decode(addr_str).unwrap();

        let r = ExtendedAddr::try_from_slice(&bytes).unwrap();

        let b = cbor!(r).unwrap();
        assert_eq!(addr_str, base58::encode(&b));

        assert_eq!(r.addr_type, AddrType::ATPubKey);
        assert_eq!(
            r.attributes.stake_distribution,
            StakeDistribution::BootstrapEraDistr
        );
        assert_eq!(r.attributes.network_magic, NetworkMagic::NoMagic);
    }

    #[test]
    fn decode_address_no_derivation_path() {
        let bytes = vec![
            0x82, 0xd8, 0x18, 0x58, 0x21, 0x83, 0x58, 0x1c, 0x10, 0x2a, 0x74, 0xca, 0x44, 0x05,
            0xb8, 0xc1, 0x8d, 0x20, 0x84, 0x1e, 0x8c, 0x66, 0x4f, 0xe1, 0xde, 0x7d, 0x66, 0x07,
            0x48, 0x08, 0x70, 0x4f, 0x91, 0x79, 0xe0, 0xfa, 0xa0, 0x00, 0x1a, 0xad, 0xf7, 0x10,
            0x68,
        ];

        let r = ExtendedAddr::try_from_slice(&bytes).unwrap();

        assert_eq!(r.addr_type, AddrType::ATPubKey);
        assert_eq!(
            r.attributes.stake_distribution,
            StakeDistribution::BootstrapEraDistr
        );
        assert_eq!(r.attributes.network_magic, NetworkMagic::NoMagic);
        assert_eq!(bytes, cbor!(r).unwrap())
    }

    #[test]
    fn decode_address_network_magic() {
        let bytes = include_bytes!("../test-vectors/network-magic.cbor");

        let r = ExtendedAddr::try_from_slice(bytes).unwrap();

        assert_eq!(r.addr_type, AddrType::ATPubKey);
        assert_eq!(
            r.attributes.stake_distribution,
            StakeDistribution::BootstrapEraDistr
        );
        assert_eq!(r.attributes.network_magic, NetworkMagic::Magic(1097911063));
        assert_eq!(&bytes[..], &cbor!(r).unwrap()[..])
    }
}

#[cfg(feature = "with-bench")]
#[cfg(test)]
mod bench {
    use address::*;
    use cbor_event::de::RawCbor;
    use hdwallet;
    use util::base58;

    const CBOR: &[u8] = &[
        0x82, 0xd8, 0x18, 0x58, 0x4c, 0x83, 0x58, 0x1c, 0x2a, 0xc3, 0xcc, 0x97, 0xbb, 0xec, 0x47,
        0x64, 0x96, 0xe8, 0x48, 0x07, 0xf3, 0x5d, 0xf7, 0x34, 0x9a, 0xcf, 0xba, 0xec, 0xe2, 0x00,
        0xa2, 0x4b, 0x7e, 0x26, 0x25, 0x0c, 0xa2, 0x00, 0x58, 0x20, 0x82, 0x00, 0x58, 0x1c, 0xa6,
        0xd9, 0xae, 0xf4, 0x75, 0xf3, 0x41, 0x89, 0x67, 0xe8, 0x7f, 0x7e, 0x93, 0xf2, 0x0f, 0x99,
        0xd8, 0xc7, 0xaf, 0x40, 0x6c, 0xba, 0x14, 0x6a, 0xff, 0xdb, 0x71, 0x91, 0x01, 0x46, 0x45,
        0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x89, 0xa5, 0x93, 0x71,
    ];

    use test;

    #[bench]
    fn encode_address_cbor_raw(b: &mut test::Bencher) {
        let mut raw = cbor_event::de::RawCbor::from(CBOR);
        let addr: ExtendedAddr = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();
        b.iter(|| {
            let _ = cbor!(addr).unwrap();
        })
    }
    #[bench]
    fn decode_address_cbor_raw(b: &mut test::Bencher) {
        b.iter(|| {
            let _: ExtendedAddr = RawCbor::from(CBOR).deserialize().unwrap();
        })
    }
}
