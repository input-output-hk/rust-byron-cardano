extern crate rcw;

use self::rcw::digest::Digest;
use self::rcw::blake2b::Blake2b;

use hdwallet::{XPub};

type DigestBlake2b = [u8;32];

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum AddrType {
    ATPubKey,
    ATScript,
    ATRedeem
}
// [TkListLen 1, TkInt (fromEnum t)]
impl AddrType {
    fn to_byte(self: Self) -> u8 {
        match self {
            ATPubKey => 0,
            ATScript => 1,
            ATRedeem => 2
        }
    }
}

mod cbor {
    #[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
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
        fn to_byte(self) -> u8 {
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
        for i in 0..bs.len() {
            buf.push(i as u8);
        }
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StakeholderId(DigestBlake2b); // of publickey (block2b 256)
impl StakeholderId {
    /// create a Sake
    /// exactly  ^^^^ what I need どうもありがとう
    pub fn new(pubk: &XPub) -> StakeholderId {
        let mut buf = Vec::new();

        hs_cbor_util::cbor_xpub(&pubk, &mut buf);
        StakeholderId(hash_frontend(&buf))
    }
}

pub enum StakeDistribution {
    BootstrapEraDistr,
    SingleKeyDistr(StakeholderId),
}

struct HDAddressPayload(Vec<u8>); // with the password of the user or something ?

pub struct Attributes {
    derivation_path: Option<HDAddressPayload>,
    stake_distribution: StakeDistribution
    // attr_remains ? whatever...
}

pub struct Addr(DigestBlake2b);
impl Addr {
    fn new(ty: AddrType, spending_data: SpendingData, attrs: Attributes) -> Addr {
        /* CBOR encode + HASH */
        let mut buff = vec![];
        hs_cbor::sumtype_start(ty.to_byte(), 0, &mut buff);
        match spending_data {
            SpendingData::PubKeyASD(xpub) => {
                hs_cbor::sumtype_start(0, 1, &mut buff);
                hs_cbor_util::cbor_xpub(&xpub, &mut buff);
            }
            SpendingData::ScriptASD(script) => {
                panic!();
            }
            SpendingData::RedeemASD(redeem_key) => {
                panic!();
            }
        };
        // TODO add attributes
        Addr(hash_frontend(buff.as_slice()))
    }
}

pub struct ExtendedAddr {
    addr: Addr,
    attributes: Attributes,
    type_: AddrType,
}

pub type Script = [u8;32]; // TODO
pub type RedeemPublicKey = [u8;32]; //TODO

pub enum SpendingData {
    PubKeyASD (XPub),
    ScriptASD (Script),
    RedeemASD (RedeemPublicKey)
    // UnknownASD... whatever...
}


// internal use only
//
// this function create the blake2b 256 digest of the given input
// This function is not responsible for the serialisation of the data
// in CBOR.
//
fn hash_frontend(buf: &[u8]) -> DigestBlake2b
{
    let mut b2b = Blake2b::new(32);
    let mut outv = [0;32];
    b2b.input(buf);
    b2b.result(&mut outv);
    outv
}
