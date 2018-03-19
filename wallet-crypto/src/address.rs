use std::fmt;

extern crate rcw;

use self::rcw::digest::Digest;
use self::rcw::blake2b::Blake2b;

use hdwallet::{XPub};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct DigestBlake2b([u8;32]);
impl DigestBlake2b {
    /// this function create the blake2b 256 digest of the given input
    /// This function is not responsible for the serialisation of the data
    /// in CBOR.
    ///
    pub fn new(buf: &[u8]) -> Self
    {
        let mut b2b = Blake2b::new(32);
        let mut outv = [0;32];
        b2b.input(buf);
        b2b.result(&mut outv);
        DigestBlake2b::from_bytes(outv)
    }

    /// create a Digest from the given 256 bits
    pub fn from_bytes(bytes :[u8;32]) -> Self { DigestBlake2b(bytes) }

    fn cbor_store(&self, buf: &mut Vec<u8>) {
        cbor::cbor_bs(&self.0[..], buf)
    }
}
impl fmt::Display for DigestBlake2b {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &byte in self.0.iter() {
            write!(f, "{:x}", byte);
        };
        Ok(())
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

pub mod cbor {
    #[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
    pub enum MajorType {
        UINT,
        NINT,
        BYTES,
        TEXT,
        ARRAY,
        MAP,
        TAG,
        T7
    }

    const INLINE_ENCODING : u8 = 24;

    impl MajorType {
        // serialize a major type in its highest bit form
        fn to_byte(self) -> u8 {
            use self::MajorType::*;
            match self {
                UINT  => 0b0000_0000,
                NINT  => 0b0010_0000,
                BYTES => 0b0100_0000,
                TEXT  => 0b0110_0000,
                ARRAY => 0b1000_0000,
                MAP   => 0b1010_0000,
                TAG   => 0b1100_0000,
                T7    => 0b1110_0000
            }
        }
    }

    pub fn cbor_header(ty: MajorType, r: u8) -> u8 {
        ty.to_byte() | r & 0x1f
    }

    pub fn cbor_uint_small(v: u8, buf: &mut Vec<u8>) {
        assert!(v < INLINE_ENCODING);
        buf.push(cbor_header(MajorType::UINT, v));
    }

    pub fn cbor_u8(v: u8, buf: &mut Vec<u8>) {
        buf.push(cbor_header(MajorType::UINT, 24));
        buf.push(v);
    }

    /// convenient macro to get the given bytes of the given value
    ///
    /// does all the job: Big Endian, bit shift and convertion
    macro_rules! byte_slice {
        ($value:ident, $shift:expr) => ({
            ($value.to_be() >> $shift) as u8
        });
    }

    pub fn write_u8(v: u8, buf: &mut Vec<u8>) {
        write_header_u8(MajorType::UINT, v, buf);
    }
    pub fn write_u16(v: u16, buf: &mut Vec<u8>) {
        write_header_u16(MajorType::UINT, v, buf);
    }
    pub fn write_u32(v: u32, buf: &mut Vec<u8>) {
        write_header_u32(MajorType::UINT, v, buf);
    }
    pub fn write_u64(v: u64, buf: &mut Vec<u8>) {
        write_header_u64(MajorType::UINT, v, buf);
    }
    pub fn write_header_u8(ty: MajorType, v: u8, buf: &mut Vec<u8>) {
        buf.push(cbor_header(ty, 24));
        buf.push(v);
    }
    pub fn write_header_u16(ty: MajorType, v: u16, buf: &mut Vec<u8>) {
        buf.push(cbor_header(ty, 25));
        buf.push(byte_slice!(v, 8));
        buf.push(byte_slice!(v, 0));
    }
    pub fn write_header_u32(ty: MajorType, v: u32, buf: &mut Vec<u8>) {
        buf.push(cbor_header(ty, 26));
        buf.push(byte_slice!(v, 24));
        buf.push(byte_slice!(v, 16));
        buf.push(byte_slice!(v,  8));
        buf.push(byte_slice!(v,  0));
    }
    pub fn write_header_u64(ty: MajorType, v: u64, buf: &mut Vec<u8>) {
        buf.push(cbor_header(ty, 27));
        buf.push(byte_slice!(v, 56));
        buf.push(byte_slice!(v, 48));
        buf.push(byte_slice!(v, 40));
        buf.push(byte_slice!(v, 32));
        buf.push(byte_slice!(v, 24));
        buf.push(byte_slice!(v, 16));
        buf.push(byte_slice!(v,  8));
        buf.push(byte_slice!(v,  0));
    }

    pub fn write_length_encoding(ty: MajorType, nb_elems: usize, buf: &mut Vec<u8>) {
        if nb_elems < (INLINE_ENCODING as usize) {
            buf.push(cbor_header(ty, nb_elems as u8));
        } else {
            if nb_elems < 0x100 {
                write_header_u8(ty, nb_elems as u8, buf);
            } else if nb_elems < 0x10000 {
                write_header_u16(ty, nb_elems as u16, buf);
            } else if nb_elems < 0x100000000 {
                write_header_u32(ty, nb_elems as u32, buf);
            } else {
                write_header_u64(ty, nb_elems as u64, buf);
            }
        }
    }

    pub fn cbor_bs(bs: &[u8], buf: &mut Vec<u8>) {
        write_length_encoding(MajorType::BYTES, bs.len(), buf);
        buf.extend_from_slice(bs)
    }

    pub fn cbor_array_start(nb_elems: usize, buf: &mut Vec<u8>) {
        write_length_encoding(MajorType::ARRAY, nb_elems, buf);
    }
}

mod hs_cbor {
    use super::cbor::{cbor_array_start, write_length_encoding, MajorType};

    pub fn sumtype_start(tag: u8, nb_values: usize, buf: &mut Vec<u8>) -> () {
        cbor_array_start(nb_values + 1, buf);
        // tag value from 0
        write_length_encoding(MajorType::UINT, tag as usize, buf);
    }
}

mod hs_cbor_util {
    use hdwallet::{XPub};
    use super::cbor::{cbor_bs};
    pub fn cbor_xpub(pubk: &XPub, buf: &mut Vec<u8>) {
        cbor_bs(&pubk[..], buf);
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct StakeholderId(DigestBlake2b); // of publickey (block2b 256)
impl StakeholderId {
    pub fn new(pubk: &XPub) -> StakeholderId {
        let mut buf = Vec::new();

        hs_cbor_util::cbor_xpub(&pubk, &mut buf);
        StakeholderId(DigestBlake2b::new(&buf))
    }
    fn cbor_store(&self, buf: &mut Vec<u8>) {
        self.0.cbor_store(buf)
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
impl StakeDistribution {
    pub fn new_era() -> Self { StakeDistribution::BootstrapEraDistr }
    pub fn new_single_stakeholder(si: StakeholderId) -> Self {
        StakeDistribution::SingleKeyDistr(si)
    }
    pub fn new_single_key(pubk: &XPub) -> Self {
        StakeDistribution::new_single_stakeholder(StakeholderId::new(pubk))
    }
    fn cbor_store(&self, buf: &mut Vec<u8>) {
        match self {
            &StakeDistribution::BootstrapEraDistr => hs_cbor::sumtype_start(0, 0, buf),
            &StakeDistribution::SingleKeyDistr(ref si) => {
                hs_cbor::sumtype_start(1, 1, buf);
                si.cbor_store(buf);
            }
        };
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct HDAddressPayload(Vec<u8>); // with the password of the user or something ?
impl AsRef<[u8]> for HDAddressPayload {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
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
    pub fn new_single_key(pubk: &XPub) -> Self {
        Attributes {
            derivation_path: None,
            stake_distribution: StakeDistribution::new_single_key(pubk)
        }
    }

    fn cbor_store(&self, buf: &mut Vec<u8>) {
        match &self.derivation_path {
            &None => hs_cbor::sumtype_start(0, 0, buf),
            &Some(ref v) => {
                hs_cbor::sumtype_start(1, 1, buf);
                cbor::cbor_bs(v.as_ref(),buf)
            }
        };
        self.stake_distribution.cbor_store(buf)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Addr(DigestBlake2b);
impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl Addr {
    pub fn new(ty: AddrType, spending_data: &SpendingData, attrs: &Attributes) -> Addr {
        /* CBOR encode + HASH */
        let mut buff = vec![];
        hs_cbor::sumtype_start(ty.to_byte(), 0, &mut buff);
        match spending_data {
            &SpendingData::PubKeyASD(ref xpub) => {
                hs_cbor::sumtype_start(0, 1, &mut buff);
                hs_cbor_util::cbor_xpub(&xpub, &mut buff);
            }
            &SpendingData::ScriptASD(ref _script) => {
                panic!();
            }
            &SpendingData::RedeemASD(ref _redeem_key) => {
                panic!();
            }
        };
        attrs.cbor_store(&mut buff);
        Addr(DigestBlake2b::new(buff.as_slice()))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ExtendedAddr {
    addr: Addr,
    attributes: Attributes,
    type_: AddrType,
}
impl ExtendedAddr {
    pub fn new(ty: AddrType, sd: SpendingData, attrs: Attributes) -> Self {
        ExtendedAddr {
            addr: Addr::new(ty, &sd, &attrs),
            attributes: attrs,
            type_: ty
        }
    }
}

pub type Script = [u8;32]; // TODO
pub type RedeemPublicKey = [u8;32]; //TODO

pub enum SpendingData {
    PubKeyASD (XPub),
    ScriptASD (Script),
    RedeemASD (RedeemPublicKey)
    // UnknownASD... whatever...
}

#[cfg(test)]
mod tests {
    use address::{AddrType, ExtendedAddr, SpendingData, Attributes};
    use hdwallet;

    const SEED : hdwallet::Seed =
        [ 0xe3, 0x55, 0x24, 0xa5, 0x18, 0x03, 0x4d, 0xdc, 0x11, 0x92, 0xe1
        , 0xda, 0xcd, 0x32, 0xc1, 0xed, 0x3e, 0xaa, 0x3c, 0x3b, 0x13, 0x1c
        , 0x88, 0xed, 0x8e, 0x7e, 0x54, 0xc4, 0x9a, 0x5d, 0x09, 0x98
        ];

    #[test]
    fn test1() {
        let sk = hdwallet::generate(&SEED);
        let pk = hdwallet::to_public(&sk);

        let addr_type = AddrType::ATPubKey;
        let sd = SpendingData::PubKeyASD(pk.clone());
        let attrs = Attributes::new_single_key(&pk);

        let ea = ExtendedAddr::new(addr_type, sd, attrs);

        println!("{:?}", ea);
        println!("addr: {:}", ea.addr);
        assert!(false);
    }
}
