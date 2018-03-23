use std::marker::PhantomData;
use std::fmt;

use rcw::digest::Digest;
use rcw::blake2b::Blake2b;

use cbor;
use cbor::hs::{FromCBOR};
use crc32::{crc32};

use hdwallet::{Signature, XPub};
use address::ExtendedAddr;
use merkle;

/// Blake2b 256 bits
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Hash<T> {
    digest: [u8;32],
    _phantom: PhantomData<T>
}
impl<T> Hash<T> {
    pub fn new(buf: &[u8]) -> Self
    {
        let mut b2b = Blake2b::new(32);
        let mut out = [0;32];
        b2b.input(buf);
        b2b.result(&mut out);
        Self::from_bytes(out)
    }

    pub fn from_bytes(bytes :[u8;32]) -> Self { Hash { digest: bytes, _phantom: PhantomData } }
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 32 { return None; }
        let mut buf = [0;32];

        buf[0..32].clone_from_slice(bytes);
        Some(Self::from_bytes(buf))
    }
}
impl<T> fmt::Display for Hash<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.digest.iter().for_each(|byte| {
            if byte < &0x10 {
                write!(f, "0{:x}", byte).unwrap()
            } else {
                write!(f, "{:x}", byte).unwrap()
            }
        });
        Ok(())
    }
}
impl<T> cbor::CborValue for Hash<T> {
    fn encode(&self) -> cbor::Value { cbor::Value::Bytes(self.digest.iter().cloned().collect()) }
    fn decode(value: &cbor::Value) -> Option<Self> {
        unimplemented!()
    }
}
impl<T> FromCBOR for Hash<T> {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let bs = decoder.bs()?;
        match Self::from_slice(&bs) {
            None => Err(cbor::decode::Error::Custom("invalid length for Hash")),
            Some(v) => Ok(v)
        }
    }
}

// TODO: this seems to be the hash of the serialisation CBOR of a given Tx.
// if this is confirmed, we need to make a proper type, wrapping it around
// to hash a `Tx` by serializing it cbor first.
pub type TxId = Hash<Tx>;

const MAX_COIN: u64 = 45000000000000000;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Coin(u64);
impl Coin {
    pub fn new(v: u64) -> Option<Self> {
        if v <= MAX_COIN { Some(Coin(v)) } else { None }
    }
}
impl cbor::CborValue for Coin {
    fn encode(&self) -> cbor::Value { cbor::Value::U64(self.0) }
    fn decode(value: &cbor::Value) -> Option<Self> {
        unimplemented!()
    }
}
impl FromCBOR for Coin {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let value = decoder.uint()?;
        match Self::new(value) {
            None => Err(cbor::decode::Error::Custom("Coin out of bound")),
            Some(v) => Ok(v)
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct TxOut {
    pub address: ExtendedAddr,
    pub value: Coin,
}
impl TxOut {
    pub fn new(addr: ExtendedAddr, value: Coin) -> Self {
        TxOut { address: addr, value: value }
    }
}
impl cbor::CborValue for TxOut {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(
            vec![ cbor::CborValue::encode(&self.address)
                , cbor::CborValue::encode(&self.value)
                ]
        )
    }
    fn decode(value: &cbor::Value) -> Option<Self> {
        unimplemented!()
    }
}
impl FromCBOR for TxOut {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        // check we have an array of 2 elements
        let l = decoder.array_start()?;
        if l != 2 {
            return Err(cbor::decode::Error::Custom("TxOut should contains 2 elements"));
        }
        // decode the ExtendedAddr with its crc32 (and check the crc32)
        let addr = cbor::hs::util::decode_with_crc32(decoder)?;
        // decode the coin
        let value = Coin::decode(decoder)?;
        Ok(TxOut::new(addr, value))
    }
}

type TODO = u8;
type ValidatorScript = TODO;
type RedeemerScript = TODO;
type RedeemPublicKey = TODO;
type RedeemSignature = TODO;

enum TxInWitness {
    /// signature of the `TxIn` with the associated `XPub`
    /// the `XPub` is the public key set in the AddrSpendingData
    PkWitness(XPub, Signature<Tx>),
    ScriptWitness(ValidatorScript, RedeemerScript),
    RedeemWitness(RedeemPublicKey, RedeemSignature),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TxIn {
    pub id: TxId,
    pub index: u32,
}
impl TxIn {
    pub fn new(id: TxId, index: u32) -> Self { TxIn { id: id, index: index } }
}
impl cbor::CborValue for TxIn {
    fn encode(&self) -> cbor::Value {
        let v = cbor::encode_to_cbor(&(self.id.clone(), self.index)).unwrap();
        cbor::Value::Array(
            vec![ cbor::CborValue::encode(&0u64)
                , cbor::Value::Tag(24, Box::new(cbor::Value::Bytes(v)))
                ]
        )
    }
    fn decode(value: &cbor::Value) -> Option<Self> {
        unimplemented!()
    }
}
impl FromCBOR for TxIn {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        println!("TxIn::decode 1");
        assert!(decoder.array_start()? == 2);
        println!("TxIn::decode 2");
        assert!(decoder.uint()? == 0);
        println!("TxIn::decode 3");
        assert!(decoder.tag()? == 24);
        println!("TxIn::decode 4");
        let buf = decoder.bs()?;
        println!("TxIn::decode 5");

        let (id, index) = cbor::hs::deserialize(&buf)?;
        println!("TxIn::decode 6");
        Ok(TxIn::new(id, index))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Tx {
    inputs: Vec<TxIn>,
    outputs: Vec<TxOut>,
    // attributes: TxAttributes
    //
    // So far, there is no TxAttributes... the structure contains only the unparsed/unknown stuff
}
impl Tx {
    pub fn new() -> Self { Tx::new_with(vec![], vec![]) }
    pub fn new_with(ins: Vec<TxIn>, outs: Vec<TxOut>) -> Self {
        Tx { inputs: ins, outputs: outs }
    }
}
//impl ToCBOR for Tx {
//    fn encode(&self, buf: &mut Vec<u8>) {
//        cbor::encode::array_start(3, buf);
//        { // inputs
//            cbor::encode::array_start_indefinite(buf);
//            self.inputs.iter().for_each(|v| v.encode(buf));
//            cbor::encode::break_(buf);
//        }
//        { // outputs
//            cbor::encode::array_start_indefinite(buf);
//            self.outputs.iter().for_each(|v| v.encode(buf));
//            cbor::encode::break_(buf);
//        }
//        { // attributes
//            cbor::encode::map_start(0, buf);
//        }
//    }
//}

struct TxAux {
    tx: Tx,
    witnesses: Vec<TxInWitness>,
}

struct TxProof {
    number: u32,
    root: merkle::Root<Tx>,
    witnesses_hash: Hash<Vec<TxInWitness>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use address;
    use hdpayload;
    use hdwallet;
    use cbor;

    // CBOR encoded TxOut
    const TX_OUT: &'static [u8] = &[0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee, 0xa1, 0xb5, 0xec, 0x8e, 0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45, 0xfd, 0x6d, 0x7b, 0x9e, 0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a];
    const TX_IN:  &'static [u8] = &[0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a, 0x13, 0xb5, 0x0a, 0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6, 0x7b, 0xbb, 0x3f, 0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19, 0x02, 0x9a];

    const TX: &'static [u8] = &[/* TODO: insert TX here */];
    const BLOCK: &'static [u8] = &[ /* TODO: insert Block here */ ];

    #[test]
    fn txout_decode() {
        let mut decoder = cbor::decode::Decoder::new();
        decoder.extend(TX_OUT);
        let txout = TxOut::decode(&mut decoder).expect("to retrieve a TxOut");

        let hdap = hdpayload::HDAddressPayload::from_vec(vec![1,2,3,4,5]);
        assert_eq!(Coin::new(42).unwrap(), txout.value);
        assert_eq!(address::AddrType::ATPubKey, txout.address.addr_type);
        assert_eq!(address::StakeDistribution::new_bootstrap_era(), txout.address.attributes.stake_distribution);
        assert_eq!(txout.address.attributes.derivation_path, Some(hdap));
    }

    fn txout_encode_decode() {
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let hdap = hdpayload::HDAddressPayload::from_vec(vec![1,2,3,4,5]);
        let addr_type = address::AddrType::ATPubKey;
        let sd = address::SpendingData::PubKeyASD(pk.clone());
        let attrs = address::Attributes::new_single_key(&pk, Some(hdap));

        let ea = address::ExtendedAddr::new(addr_type, sd, attrs);
        let value = Coin::new(42).unwrap();

        assert!(cbor::hs::encode_decode(&TxOut::new(ea, value)));
    }

    #[test]
    fn txin_decode() {
        let mut decoder = cbor::decode::Decoder::new();
        decoder.extend(TX_IN);
        let txin = TxIn::decode(&mut decoder).expect("to retrive a TxIn");
    }

    #[test]
    fn txin_encode_decode() {
        let txid = TxId::new(&[0;32]);
        assert!(cbor::hs::encode_decode(&TxIn::new(txid, 666)));
    }

    #[test]
    fn tx_decode() {
        // TODO test we can decode a cbor Tx
        unimplemented!()
    }

    #[test]
    fn block_decode() {
        unimplemented!()
    }
}
