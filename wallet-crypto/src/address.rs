use std::fmt;

extern crate rcw;

use self::rcw::digest::Digest;
use self::rcw::blake2b::Blake2b;
use self::rcw::sha3::Sha3;

use hdwallet::{XPub};

// internal mobule to encode the address metadata in cbor to
// hash them.
//
mod cbor {
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
    pub fn cbor_map_start(nb_elems: usize, buf: &mut Vec<u8>) {
        write_length_encoding(MajorType::MAP, nb_elems, buf);
    }

}

mod hs_cbor {
    use super::cbor::{cbor_array_start, write_length_encoding, MajorType};

    pub fn sumtype_start(tag: u8, nb_values: usize, buf: &mut Vec<u8>) -> () {
        cbor_array_start(nb_values + 1, buf);
        // tag value from 0
        write_length_encoding(MajorType::UINT, tag as usize, buf);
    }

    // helper trait to write CBOR encoding
    pub trait ToCBOR {
        fn encode(&self, &mut Vec<u8>);
    }
    impl<T: ToCBOR> ToCBOR for Option<T> {
        fn encode(&self, buf: &mut Vec<u8>) {
            match self {
                &None => sumtype_start(0, 0, buf),
                &Some(ref t) => {
                    sumtype_start(1, 1, buf);
                    t.encode(buf)
                }
            }
        }
    }
    impl <'a, 'b, A: ToCBOR, B: ToCBOR> ToCBOR for (&'a A, &'b B) {
        fn encode(&self, buf: &mut Vec<u8>) {
            write_length_encoding(MajorType::ARRAY, 2, buf);
            self.0.encode(buf);
            self.1.encode(buf);
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

    pub fn serialize<T: ToCBOR>(t: &T) -> Vec<u8> {
        let mut buf = vec![];
        t.encode(&mut buf);
        buf
    }
}

mod hs_cbor_util {
    use hdwallet::{XPub};
    use super::cbor::{cbor_bs};
    pub fn cbor_xpub(pubk: &XPub, buf: &mut Vec<u8>) {
        cbor_bs(&pubk[..], buf);
    }
}

use self::hs_cbor::ToCBOR;

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
        let mut sh3 = Sha3::sha3_256();
        let mut outv = [0;32];
        sh3.input(buf);
        sh3.result(&mut outv);
        b2b.input(&outv);
        b2b.result(&mut outv);
        DigestBlake2b::from_bytes(outv)
    }

    /// create a Digest from the given 256 bits
    pub fn from_bytes(bytes :[u8;32]) -> Self { DigestBlake2b(bytes) }
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
        cbor::cbor_bs(&self.0[..], buf)
    }
}

fn print_to_hex(bytes: &[u8]) {
    bytes.iter().for_each(|byte| {
        if byte.clone() < 0x10 {
            print!("0{:x}", byte)
        } else {
            print!("{:x}", byte)
        }
    });
    println!("");
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
        cbor::cbor_uint_small(self.to_byte(), buf);
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
}
impl ToCBOR for StakeholderId {
    fn encode(&self, buf: &mut Vec<u8>) {
        self.0.encode(buf)
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
}
impl ToCBOR for StakeDistribution {
    fn encode(&self, buf: &mut Vec<u8>) {
        match self {
            &StakeDistribution::BootstrapEraDistr => hs_cbor::sumtype_start(0, 0, buf),
            &StakeDistribution::SingleKeyDistr(ref si) => {
                hs_cbor::sumtype_start(1, 1, buf);
                si.encode(buf);
            }
        };
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
        cbor::cbor_bs(self.as_ref(), &mut vec);
        cbor::cbor_bs(&vec         , buf);
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
impl ToCBOR for Attributes {
    fn encode(&self, buf: &mut Vec<u8>) {
        cbor::cbor_map_start(2, buf);
        // TODO
        cbor::cbor_uint_small(0, buf);
        self.stake_distribution.encode(buf);
        cbor::cbor_uint_small(1, buf);
        self.derivation_path.encode(buf);
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
impl Addr {
    pub fn new(addr_type: AddrType, spending_data: &SpendingData, attrs: &Attributes) -> Addr {
        /* CBOR encode + HASH */
        let mut buff = vec![];
        (&addr_type, spending_data, attrs).encode(&mut buff);
        Addr(DigestBlake2b::new(buff.as_slice()))
    }
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
}
impl ToCBOR for ExtendedAddr {
    fn encode(&self, buf: &mut Vec<u8>) {
        self.addr.encode(buf);
        self.attributes.encode(buf);
        self.addr_type.encode(buf);
    }
}
impl fmt::Display for ExtendedAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
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
impl ToCBOR for SpendingData {
    fn encode(&self, buf: &mut Vec<u8>) {
        match self {
            &SpendingData::PubKeyASD(ref xpub) => {
                hs_cbor::sumtype_start(0, 1, buf);
                hs_cbor_util::cbor_xpub(xpub, buf);
            }
            &SpendingData::ScriptASD(ref _script) => {
                panic!();
            }
            &SpendingData::RedeemASD(ref _redeem_key) => {
                panic!();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use address::{AddrType, ExtendedAddr, SpendingData, Attributes, HDAddressPayload};
    use hdwallet;

    const SEED : hdwallet::Seed = [0;32];

    #[test]
    fn test1() {
        let sk = hdwallet::generate(&SEED);
        let pk = hdwallet::to_public(&sk);

        let hdap = HDAddressPayload::new(&[1,2,3,4,5]);
        let addr_type = AddrType::ATPubKey;
        let sd = SpendingData::PubKeyASD(pk.clone());
        let attrs = Attributes::new_single_key(&pk, Some(hdap));

        let ea = ExtendedAddr::new(addr_type, sd, attrs);

        println!("{:?}", ea);
        println!("addr: {:}", ea.addr);
        assert!(false);
    }
}
