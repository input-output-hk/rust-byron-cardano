use std::fmt;

extern crate rcw;

use self::rcw::digest::Digest;
use self::rcw::blake2b::Blake2b;
use self::rcw::sha3::Sha3;
use cbor;

use hdwallet::{XPub};

mod hs_cbor {
    use cbor::{MajorType};
    use cbor::encode::{cbor_array_start, cbor_uint, cbor_bs, write_length_encoding};
    use cbor::decode;

    pub fn sumtype_start(tag: u64, nb_values: usize, buf: &mut Vec<u8>) -> () {
        cbor_array_start(nb_values + 1, buf);
        // tag value from 0
        cbor_uint(tag, buf);
    }

    pub fn dec_sumtype_start(decoder: &mut decode::Decoder) -> decode::Result<(u64, usize)> {
        let l = decoder.array_start()?;
        let t = decoder.uint()?;
        Ok((t, l - 1))
    }

    #[cfg(test)]
    pub fn encode_decode<T: ToCBOR+FromCBOR+Eq>(t: &T) -> bool {
        let mut buf = vec![];
        t.encode(&mut buf);

        let mut dec = decode::Decoder::new();
        dec.extend(&buf);
        let v = T::decode(&mut dec).unwrap();
        t == &v
    }

    // helper trait to write CBOR encoding
    pub trait ToCBOR {
        fn encode(&self, &mut Vec<u8>);
    }
    pub trait FromCBOR : Sized {
        fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self>;
    }
    impl ToCBOR for Vec<u8> {
        fn encode(&self, buf: &mut Vec<u8>) {
            cbor_bs(self, buf)
        }
    }
    impl FromCBOR for Vec<u8> {
        fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> {
            decoder.bs()
        }
    }
    impl<T: ToCBOR> ToCBOR for Option<T> {
        fn encode(&self, buf: &mut Vec<u8>) {
            match self {
                &None => sumtype_start(0, 0, buf),
                &Some(ref t) => {
                    // TODO ? sumtype_start(1, 1, buf);
                    t.encode(buf)
                }
            }
        }
    }
    impl <'a, 'b, A: ToCBOR, B: ToCBOR> ToCBOR for (&'a A, &'b B) {
        fn encode(&self, buf: &mut Vec<u8>) {
            cbor_array_start(2, buf);
            self.0.encode(buf);
            self.1.encode(buf);
        }
    }
    impl <A: FromCBOR + Sized, B: FromCBOR + Sized> FromCBOR for (A, B) {
        fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> {
            let l = decoder.array_start()?;
            let x = A::decode(decoder)?;
            let y = B::decode(decoder)?;
            Ok((x,y))
        }
    }
    impl <'a, 'b, 'c, A: ToCBOR, B: ToCBOR, C: ToCBOR> ToCBOR for (&'a A, &'b B, &'c C) {
        fn encode(&self, buf: &mut Vec<u8>) {
            write_length_encoding(MajorType::ARRAY, 3, buf);
            self.0.encode(buf);
            self.1.encode(buf);
            self.2.encode(buf);
        }
    }
    impl <A: FromCBOR + Sized, B: FromCBOR + Sized, C: FromCBOR + Sized> FromCBOR for (A, B, C) {
        fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> {
            let l = decoder.array_start()?;
            let x = A::decode(decoder)?;
            let y = B::decode(decoder)?;
            let z = C::decode(decoder)?;
            Ok((x,y,z))
        }
    }

    pub fn serialize<T: ToCBOR>(t: &T) -> Vec<u8> {
        let mut buf = vec![];
        t.encode(&mut buf);
        buf
    }
    pub fn deserialize<T: FromCBOR>(buf: &[u8]) -> decode::Result<T> {
        let mut dec = decode::Decoder::new();
        dec.extend(buf);
        T::decode(&mut dec)
    }

}

mod hs_cbor_util {
    use hdwallet::{XPub, XPUB_SIZE};
    use cbor::encode::{cbor_bs, cbor_array_start, cbor_tag, write_u32};
    use cbor::decode;
    use cbor::decode::{Decoder};
    use super::hs_cbor::{ToCBOR, FromCBOR, serialize, deserialize};
    use crc32::{crc32};

    pub fn xpub_to_cbor(pubk: &XPub, buf: &mut Vec<u8>) {
        cbor_bs(&pubk[..], buf);
    }
    pub fn xpub_from_cbor(decoder: &mut Decoder) -> decode::Result<XPub> {
        let mut buf = [0u8;XPUB_SIZE];
        let bs = decoder.bs()?;
        assert!(bs.len() == XPUB_SIZE);
        buf[0..XPUB_SIZE].clone_from_slice(&bs);
        Ok(buf)
    }

    pub fn encode_with_crc32<T: ToCBOR>(t: &T, buf: &mut Vec<u8>) {
        let v = serialize(t);

        cbor_array_start(2, buf);
        cbor_tag(24, buf);
        cbor_bs(&v, buf);

        write_u32(crc32(&v), buf);
    }
    pub fn decode_with_crc32<T: FromCBOR>(decoder: &mut Decoder) -> decode::Result<T> {
        let len = decoder.array_start()?;
        assert!(len == 2);
        let tag = decoder.tag()?;
        assert!(tag == 24);
        let buf = decoder.bs()?;
        let crc = decoder.u32()?;

        assert!(crc == crc32(&buf));

        deserialize(&buf)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use cbor::decode::{Decoder};

        #[test]
        fn crc32() {
            let bytes : Vec<u8> = b"some random string".iter().cloned().collect();
            let mut dest = vec![];
            encode_with_crc32::<Vec<u8>>(&bytes, &mut dest);
            let mut decoder = Decoder::new();
            decoder.extend(&dest);
            let r : Vec<u8> = decode_with_crc32::<Vec<u8>>(&mut decoder).unwrap();
            assert_eq!(bytes, r);
        }
    }
}

use self::hs_cbor::{ToCBOR, FromCBOR};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct DigestBlake2b([u8;28]);
impl DigestBlake2b {
    /// this function create the blake2b 224 digest of the given input
    /// This function is not responsible for the serialisation of the data
    /// in CBOR.
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
        DigestBlake2b::from_bytes(out2)
    }

    /// create a Digest from the given 224 bits
    pub fn from_bytes(bytes :[u8;28]) -> Self { DigestBlake2b(bytes) }
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 28 { return None; }
        let mut buf = [0;28];

        buf[0..28].clone_from_slice(bytes);
        Some(DigestBlake2b::from_bytes(buf))
    }
}
impl fmt::Display for DigestBlake2b {
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
impl ToCBOR for DigestBlake2b {
    fn encode(&self, buf: &mut Vec<u8>) {
        cbor::encode::cbor_bs(&self.0[..], buf)
    }
}
impl FromCBOR for DigestBlake2b {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let bs = decoder.bs()?;
        match DigestBlake2b::from_slice(&bs) {
            None => Err(cbor::decode::Error::Custom("invalid length for DigestBlake2b")),
            Some(v) => Ok(v)
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum AddrType {
    ATPubKey,
    ATScript,
    ATRedeem
}
// [TkListLen 1, TkInt (fromEnum t)]
impl AddrType {
    fn to_byte(self) -> u8 {
        match self {
            AddrType::ATPubKey => 0,
            AddrType::ATScript => 1,
            AddrType::ATRedeem => 2
        }
    }
}
impl ToCBOR for AddrType {
    fn encode(&self, buf: &mut Vec<u8>) {
        cbor::encode::cbor_uint(self.to_byte() as u64, buf);
    }
}
impl FromCBOR for AddrType {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let b = decoder.uint()?;
        match b {
            0 => Ok(AddrType::ATPubKey),
            1 => Ok(AddrType::ATScript),
            2 => Ok(AddrType::ATRedeem),
            _ => Err(cbor::decode::Error::Custom("Unknown AddrType..."))
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct StakeholderId(DigestBlake2b); // of publickey (block2b 256)
impl StakeholderId {
    pub fn new(pubk: &XPub) -> StakeholderId {
        let mut buf = Vec::new();

        hs_cbor_util::xpub_to_cbor(&pubk, &mut buf);
        StakeholderId(DigestBlake2b::new(&buf))
    }
}
impl ToCBOR for StakeholderId {
    fn encode(&self, buf: &mut Vec<u8>) {
        self.0.encode(buf)
    }
}
impl FromCBOR for StakeholderId {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let digest = DigestBlake2b::decode(decoder)?;
        Ok(StakeholderId(digest))
    }
}
impl fmt::Display for StakeholderId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum StakeDistribution {
    BootstrapEraDistr,
    SingleKeyDistr(StakeholderId),
}

const STAKE_DISTRIBUTION_TAG_BOOTSTRAP : u64 = 1;
const STAKE_DISTRIBUTION_TAG_SINGLEKEY : u64 = 0;

impl StakeDistribution {
    pub fn new_era() -> Self { StakeDistribution::BootstrapEraDistr }
    pub fn new_single_stakeholder(si: StakeholderId) -> Self {
        StakeDistribution::SingleKeyDistr(si)
    }
    pub fn new_single_key(pubk: &XPub) -> Self {
        StakeDistribution::new_single_stakeholder(StakeholderId::new(pubk))
    }
}
impl ToCBOR for StakeDistribution {
    fn encode(&self, buf: &mut Vec<u8>) {
        let mut vec = vec![];
        match self {
            &StakeDistribution::BootstrapEraDistr => hs_cbor::sumtype_start(STAKE_DISTRIBUTION_TAG_BOOTSTRAP, 0, &mut vec),
            &StakeDistribution::SingleKeyDistr(ref si) => {
                hs_cbor::sumtype_start(STAKE_DISTRIBUTION_TAG_SINGLEKEY, 1, &mut vec);
                si.encode(&mut vec);
            }
        };
        cbor::encode::cbor_bs(&vec, buf);
    }
}
impl FromCBOR for StakeDistribution {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let bs = decoder.bs()?;

        let mut dec = cbor::decode::Decoder::new();
        dec.extend(&bs);
        match hs_cbor::dec_sumtype_start(&mut dec)? {
            (STAKE_DISTRIBUTION_TAG_BOOTSTRAP, 0) => {
                Ok(StakeDistribution::new_era())
            },
            (STAKE_DISTRIBUTION_TAG_SINGLEKEY, 1) => {
                let si = StakeholderId::decode(&mut dec)?;
                Ok(StakeDistribution::new_single_stakeholder(si))
            },
            _ => Err(cbor::decode::Error::Custom("Invalid stake StakeDistribution"))
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HDAddressPayload(Vec<u8>); // with the password of the user or something ?
impl AsRef<[u8]> for HDAddressPayload {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl HDAddressPayload {
    pub fn new(buf: &[u8]) -> Self { HDAddressPayload(buf.iter().cloned().collect()) }
}
impl ToCBOR for HDAddressPayload {
    fn encode(&self, buf: &mut Vec<u8>) {
        let mut vec = vec![];
        cbor::encode::cbor_bs(self.as_ref(), &mut vec);
        cbor::encode::cbor_bs(&vec         , buf);
    }
}
impl FromCBOR for HDAddressPayload {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let bs = decoder.bs()?;

        let mut dec = cbor::decode::Decoder::new();
        dec.extend(&bs);
        let vec = dec.bs()?;
        Ok(HDAddressPayload::new(&vec))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Attributes {
    derivation_path: Option<HDAddressPayload>,
    stake_distribution: StakeDistribution
    // attr_remains ? whatever...
}
impl Attributes {
    pub fn new_era() -> Self {
        Attributes {
            derivation_path: None,
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

impl ToCBOR for Attributes {
    fn encode(&self, buf: &mut Vec<u8>) {
        cbor::encode::cbor_map_start(2, buf);
        cbor::encode::cbor_uint(ATTRIBUTE_NAME_TAG_STAKE, buf);
        self.stake_distribution.encode(buf);
        cbor::encode::cbor_uint(ATTRIBUTE_NAME_TAG_DERIVATION, buf);
        self.derivation_path.encode(buf);
    }
}
impl FromCBOR for Attributes {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let _ = decoder.map_start()?;
        let x = decoder.uint()?;
        assert!(x == ATTRIBUTE_NAME_TAG_STAKE);
        let sd = StakeDistribution::decode(decoder)?;
        let y = decoder.uint()?;
        let dp = HDAddressPayload::decode(decoder)?;
        Ok(Attributes{ derivation_path: Some(dp), stake_distribution: sd })
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Addr(DigestBlake2b);
impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl ToCBOR for Addr {
    fn encode(&self, buf: &mut Vec<u8>) {
        self.0.encode(buf)
    }
}
impl FromCBOR for Addr {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let db2b = DigestBlake2b::decode(decoder)?;
        Ok(Addr(db2b))
    }
}
impl Addr {
    pub fn new(addr_type: AddrType, spending_data: &SpendingData, attrs: &Attributes) -> Addr {
        /* CBOR encode + HASH */
        let mut buff = vec![];
        (&addr_type, spending_data, attrs).encode(&mut buff);
        Addr(DigestBlake2b::new(buff.as_slice()))
    }

    /// create a Digest from the given 224 bits
    pub fn from_bytes(bytes :[u8;28]) -> Self { Addr(DigestBlake2b::from_bytes(bytes)) }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ExtendedAddr {
    addr: Addr,
    attributes: Attributes,
    addr_type: AddrType,
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
    /// use wallet_crypto::address::{AddrType, ExtendedAddr, SpendingData, Attributes, HDAddressPayload, Addr};
    /// use wallet_crypto::hdwallet;
    ///
    /// let sk = hdwallet::generate(&[0;32]);
    /// let pk = hdwallet::to_public(&sk);
    ///
    /// let hdap = HDAddressPayload::new(&[1,2,3,4,5]);
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
        let mut vec = vec![];
        hs_cbor_util::encode_with_crc32(self, &mut vec);
        vec
    }

    /// decode an `ExtendedAddr` to cbor with the extra details and `crc32`
    ///
    /// ```
    /// use wallet_crypto::address::{AddrType, ExtendedAddr, SpendingData, Attributes, HDAddressPayload, Addr};
    /// use wallet_crypto::hdwallet;
    ///
    /// let sk = hdwallet::generate(&[0;32]);
    /// let pk = hdwallet::to_public(&sk);
    ///
    /// let hdap = HDAddressPayload::new(&[1,2,3,4,5]);
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
    pub fn from_bytes(buf: &[u8]) -> cbor::decode::Result<Self> {
        let mut decoder = cbor::decode::Decoder::new();
        decoder.extend(buf);
        hs_cbor_util::decode_with_crc32(&mut decoder)
    }
}
impl ToCBOR for ExtendedAddr {
    fn encode(&self, buf: &mut Vec<u8>) {
        (&self.addr, &self.attributes, &self.addr_type).encode(buf);
    }
}
impl FromCBOR for ExtendedAddr {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let (a,b,c) = FromCBOR::decode(decoder)?;
        Ok(ExtendedAddr {
            addr: a,
            attributes: b,
            addr_type: c
        })
    }
}
impl fmt::Display for ExtendedAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

pub type Script = [u8;32]; // TODO
pub type RedeemPublicKey = [u8;32]; //TODO

const SPENDING_DATA_TAG_PUBKEY : u64 = 0;
const SPENDING_DATA_TAG_SCRIPT : u64 = 1; // TODO
const SPENDING_DATA_TAG_REDEEM : u64 = 2; // TODO

pub enum SpendingData {
    PubKeyASD (XPub),
    ScriptASD (Script),
    RedeemASD (RedeemPublicKey)
    // UnknownASD... whatever...
}
impl ToCBOR for SpendingData {
    fn encode(&self, buf: &mut Vec<u8>) {
        match self {
            &SpendingData::PubKeyASD(ref xpub) => {
                hs_cbor::sumtype_start(SPENDING_DATA_TAG_PUBKEY, 1, buf);
                hs_cbor_util::xpub_to_cbor(xpub, buf);
            }
            &SpendingData::ScriptASD(ref _script) => {
                unimplemented!()
            }
            &SpendingData::RedeemASD(ref _redeem_key) => {
                unimplemented!()
            }
        }
    }
}
impl FromCBOR for SpendingData {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let r = hs_cbor::dec_sumtype_start(decoder)?;
        match r {
            (SPENDING_DATA_TAG_PUBKEY, 1) => {
                let xpub = hs_cbor_util::xpub_from_cbor(decoder)?;
                Ok(SpendingData::PubKeyASD(xpub))
            },
            (SPENDING_DATA_TAG_SCRIPT, _) => {
                unimplemented!()
            },
            (SPENDING_DATA_TAG_REDEEM, _) => {
                unimplemented!()
            },
            _ => Err(cbor::decode::Error::Custom("invalid SpendingData..."))
        }
    }
}

#[cfg(test)]
mod tests {
    use address::*;
    use hdwallet;

    const SEED : hdwallet::Seed = [0;32];

    #[test]
    fn test_make_address() {
        let v    = [ 0x2a, 0xc3, 0xcc, 0x97, 0xbb, 0xec, 0x47, 0x64, 0x96, 0xe8, 0x48, 0x07
                   , 0xf3, 0x5d, 0xf7, 0x34, 0x9a, 0xcf, 0xba, 0xec, 0xe2, 0x00, 0xa2, 0x4b
                   , 0x7e, 0x26, 0x25, 0x0c];
        let addr = Addr::from_bytes(v);

        let sk = hdwallet::generate(&SEED);
        let pk = hdwallet::to_public(&sk);

        let hdap = HDAddressPayload::new(&[1,2,3,4,5]);
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

        let sk = hdwallet::generate(&SEED);
        let pk = hdwallet::to_public(&sk);

        let hdap = HDAddressPayload::new(&[1,2,3,4,5]);
        let addr_type = AddrType::ATPubKey;
        let sd = SpendingData::PubKeyASD(pk.clone());
        let attrs = Attributes::new_single_key(&pk, Some(hdap));

        let ea = ExtendedAddr::new(addr_type, sd, attrs);

        let out = ea.to_bytes();

        assert_eq!(out, v);

        let r = ExtendedAddr::from_bytes(&out).unwrap();
        assert_eq!(ea, r);
    }

    #[test]
    fn encode_decode_digest_blake2b() {
        let b = b"some random bytes...";
        let digest = DigestBlake2b::new(b"some random bytes...");
        assert!(hs_cbor::encode_decode(&digest))
    }
    #[test]
    fn encode_decode_addr_type() {
        let addr_type_1 = AddrType::ATPubKey;
        let addr_type_2 = AddrType::ATScript;
        let addr_type_3 = AddrType::ATRedeem;
        assert!(hs_cbor::encode_decode(&addr_type_1));
        assert!(hs_cbor::encode_decode(&addr_type_2));
        assert!(hs_cbor::encode_decode(&addr_type_3));
    }
    #[test]
    fn encode_decode_stakeholderid() {
        use hdwallet;
        let sk = hdwallet::generate(&[0;hdwallet::SEED_SIZE]);
        let pk = hdwallet::to_public(&sk);
        let si = StakeholderId::new(&pk);
        assert!(hs_cbor::encode_decode(&si));
    }
    #[test]
    fn encode_decode_stakedistribution() {
        use hdwallet;
        let sk = hdwallet::generate(&[0;hdwallet::SEED_SIZE]);
        let pk = hdwallet::to_public(&sk);
        let sd_1 = StakeDistribution::new_era();
        let sd_2 = StakeDistribution::new_single_key(&pk);
        assert!(hs_cbor::encode_decode(&sd_1));
        assert!(hs_cbor::encode_decode(&sd_2));
    }
}
