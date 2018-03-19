use std::fmt;

extern crate rcw;

use self::rcw::digest::Digest;
use self::rcw::blake2b::Blake2b;
use self::rcw::sha3::Sha3;
use cbor;

use hdwallet::{XPub};

mod hs_cbor {
    use cbor::spec::{cbor_array_start, write_length_encoding, MajorType};

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
                    // TODO ? sumtype_start(1, 1, buf);
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

    const CRC_TABLE : [u32;256] = [
      0x00000000u32, 0x77073096u32, 0xee0e612cu32, 0x990951bau32, 0x076dc419u32,
      0x706af48fu32, 0xe963a535u32, 0x9e6495a3u32, 0x0edb8832u32, 0x79dcb8a4u32,
      0xe0d5e91eu32, 0x97d2d988u32, 0x09b64c2bu32, 0x7eb17cbdu32, 0xe7b82d07u32,
      0x90bf1d91u32, 0x1db71064u32, 0x6ab020f2u32, 0xf3b97148u32, 0x84be41deu32,
      0x1adad47du32, 0x6ddde4ebu32, 0xf4d4b551u32, 0x83d385c7u32, 0x136c9856u32,
      0x646ba8c0u32, 0xfd62f97au32, 0x8a65c9ecu32, 0x14015c4fu32, 0x63066cd9u32,
      0xfa0f3d63u32, 0x8d080df5u32, 0x3b6e20c8u32, 0x4c69105eu32, 0xd56041e4u32,
      0xa2677172u32, 0x3c03e4d1u32, 0x4b04d447u32, 0xd20d85fdu32, 0xa50ab56bu32,
      0x35b5a8fau32, 0x42b2986cu32, 0xdbbbc9d6u32, 0xacbcf940u32, 0x32d86ce3u32,
      0x45df5c75u32, 0xdcd60dcfu32, 0xabd13d59u32, 0x26d930acu32, 0x51de003au32,
      0xc8d75180u32, 0xbfd06116u32, 0x21b4f4b5u32, 0x56b3c423u32, 0xcfba9599u32,
      0xb8bda50fu32, 0x2802b89eu32, 0x5f058808u32, 0xc60cd9b2u32, 0xb10be924u32,
      0x2f6f7c87u32, 0x58684c11u32, 0xc1611dabu32, 0xb6662d3du32, 0x76dc4190u32,
      0x01db7106u32, 0x98d220bcu32, 0xefd5102au32, 0x71b18589u32, 0x06b6b51fu32,
      0x9fbfe4a5u32, 0xe8b8d433u32, 0x7807c9a2u32, 0x0f00f934u32, 0x9609a88eu32,
      0xe10e9818u32, 0x7f6a0dbbu32, 0x086d3d2du32, 0x91646c97u32, 0xe6635c01u32,
      0x6b6b51f4u32, 0x1c6c6162u32, 0x856530d8u32, 0xf262004eu32, 0x6c0695edu32,
      0x1b01a57bu32, 0x8208f4c1u32, 0xf50fc457u32, 0x65b0d9c6u32, 0x12b7e950u32,
      0x8bbeb8eau32, 0xfcb9887cu32, 0x62dd1ddfu32, 0x15da2d49u32, 0x8cd37cf3u32,
      0xfbd44c65u32, 0x4db26158u32, 0x3ab551ceu32, 0xa3bc0074u32, 0xd4bb30e2u32,
      0x4adfa541u32, 0x3dd895d7u32, 0xa4d1c46du32, 0xd3d6f4fbu32, 0x4369e96au32,
      0x346ed9fcu32, 0xad678846u32, 0xda60b8d0u32, 0x44042d73u32, 0x33031de5u32,
      0xaa0a4c5fu32, 0xdd0d7cc9u32, 0x5005713cu32, 0x270241aau32, 0xbe0b1010u32,
      0xc90c2086u32, 0x5768b525u32, 0x206f85b3u32, 0xb966d409u32, 0xce61e49fu32,
      0x5edef90eu32, 0x29d9c998u32, 0xb0d09822u32, 0xc7d7a8b4u32, 0x59b33d17u32,
      0x2eb40d81u32, 0xb7bd5c3bu32, 0xc0ba6cadu32, 0xedb88320u32, 0x9abfb3b6u32,
      0x03b6e20cu32, 0x74b1d29au32, 0xead54739u32, 0x9dd277afu32, 0x04db2615u32,
      0x73dc1683u32, 0xe3630b12u32, 0x94643b84u32, 0x0d6d6a3eu32, 0x7a6a5aa8u32,
      0xe40ecf0bu32, 0x9309ff9du32, 0x0a00ae27u32, 0x7d079eb1u32, 0xf00f9344u32,
      0x8708a3d2u32, 0x1e01f268u32, 0x6906c2feu32, 0xf762575du32, 0x806567cbu32,
      0x196c3671u32, 0x6e6b06e7u32, 0xfed41b76u32, 0x89d32be0u32, 0x10da7a5au32,
      0x67dd4accu32, 0xf9b9df6fu32, 0x8ebeeff9u32, 0x17b7be43u32, 0x60b08ed5u32,
      0xd6d6a3e8u32, 0xa1d1937eu32, 0x38d8c2c4u32, 0x4fdff252u32, 0xd1bb67f1u32,
      0xa6bc5767u32, 0x3fb506ddu32, 0x48b2364bu32, 0xd80d2bdau32, 0xaf0a1b4cu32,
      0x36034af6u32, 0x41047a60u32, 0xdf60efc3u32, 0xa867df55u32, 0x316e8eefu32,
      0x4669be79u32, 0xcb61b38cu32, 0xbc66831au32, 0x256fd2a0u32, 0x5268e236u32,
      0xcc0c7795u32, 0xbb0b4703u32, 0x220216b9u32, 0x5505262fu32, 0xc5ba3bbeu32,
      0xb2bd0b28u32, 0x2bb45a92u32, 0x5cb36a04u32, 0xc2d7ffa7u32, 0xb5d0cf31u32,
      0x2cd99e8bu32, 0x5bdeae1du32, 0x9b64c2b0u32, 0xec63f226u32, 0x756aa39cu32,
      0x026d930au32, 0x9c0906a9u32, 0xeb0e363fu32, 0x72076785u32, 0x05005713u32,
      0x95bf4a82u32, 0xe2b87a14u32, 0x7bb12baeu32, 0x0cb61b38u32, 0x92d28e9bu32,
      0xe5d5be0du32, 0x7cdcefb7u32, 0x0bdbdf21u32, 0x86d3d2d4u32, 0xf1d4e242u32,
      0x68ddb3f8u32, 0x1fda836eu32, 0x81be16cdu32, 0xf6b9265bu32, 0x6fb077e1u32,
      0x18b74777u32, 0x88085ae6u32, 0xff0f6a70u32, 0x66063bcau32, 0x11010b5cu32,
      0x8f659effu32, 0xf862ae69u32, 0x616bffd3u32, 0x166ccf45u32, 0xa00ae278u32,
      0xd70dd2eeu32, 0x4e048354u32, 0x3903b3c2u32, 0xa7672661u32, 0xd06016f7u32,
      0x4969474du32, 0x3e6e77dbu32, 0xaed16a4au32, 0xd9d65adcu32, 0x40df0b66u32,
      0x37d83bf0u32, 0xa9bcae53u32, 0xdebb9ec5u32, 0x47b2cf7fu32, 0x30b5ffe9u32,
      0xbdbdf21cu32, 0xcabac28au32, 0x53b39330u32, 0x24b4a3a6u32, 0xbad03605u32,
      0xcdd70693u32, 0x54de5729u32, 0x23d967bfu32, 0xb3667a2eu32, 0xc4614ab8u32,
      0x5d681b02u32, 0x2a6f2b94u32, 0xb40bbe37u32, 0xc30c8ea1u32, 0x5a05df1bu32,
      0x2d02ef8du32
    ];

    pub fn crc32(input: &[u8]) -> u32 {
        let mut v = 0 ^ 0xFFFFFFFFu32;
        v = input.iter().fold(v, |acc, &byte| {
            CRC_TABLE[((acc as u8) ^ byte) as usize] ^ (acc >> 8)
        });
        v ^ 0xFFFFFFFFu32
    }
}

mod hs_cbor_util {
    use hdwallet::{XPub};
    use cbor::spec::{cbor_bs, cbor_array_start, cbor_tag, write_u32};
    use super::hs_cbor::{ToCBOR, serialize, crc32};
    pub fn cbor_xpub(pubk: &XPub, buf: &mut Vec<u8>) {
        cbor_bs(&pubk[..], buf);
    }

    pub fn encode_with_crc32<T: ToCBOR>(t: &T, buf: &mut Vec<u8>) {
        let v = serialize(t);
        let crc = crc32(&v);
        cbor_array_start(2, buf);
        cbor_tag(24, buf);
        cbor_bs(&v, buf);
        write_u32(crc, buf);
    }
}

use self::hs_cbor::ToCBOR;

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
        cbor::spec::cbor_bs(&self.0[..], buf)
    }
}

pub fn print_to_hex(bytes: &[u8]) {
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
        cbor::spec::cbor_uint_small(self.to_byte(), buf);
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
        let mut vec = vec![];
        match self {
            &StakeDistribution::BootstrapEraDistr => hs_cbor::sumtype_start(1, 0, &mut vec),
            &StakeDistribution::SingleKeyDistr(ref si) => {
                hs_cbor::sumtype_start(0, 1, &mut vec);
                si.encode(&mut vec);
            }
        };
        cbor::spec::cbor_bs(&vec, buf);
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
        cbor::spec::cbor_bs(self.as_ref(), &mut vec);
        cbor::spec::cbor_bs(&vec         , buf);
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
        cbor::spec::cbor_map_start(2, buf);
        // TODO
        cbor::spec::cbor_uint_small(0, buf);
        self.stake_distribution.encode(buf);
        cbor::spec::cbor_uint_small(1, buf);
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

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = vec![];
        hs_cbor_util::encode_with_crc32(self, &mut vec);
        vec
    }
}
impl ToCBOR for ExtendedAddr {
    fn encode(&self, buf: &mut Vec<u8>) {
        cbor::spec::cbor_array_start(3, buf);
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

        let out = ea.to_bytes();

        println!("{:?}", ea);
        super::print_to_hex(&out);
        assert!(false);
    }
}
