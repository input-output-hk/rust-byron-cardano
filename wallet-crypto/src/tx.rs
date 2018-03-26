use std::marker::PhantomData;
use std::fmt;
use std::collections::{LinkedList, BTreeMap};

use rcw::digest::Digest;
use rcw::blake2b::Blake2b;

use cbor;
use cbor::{ExtendedResult};

use hdwallet::{Signature, XPub};
use address::ExtendedAddr;
use merkle;

/// Blake2b 256 bits
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Hash<T> {
    digest: [u8;32],
    _phantom: PhantomData<T>
}
impl<T> AsRef<[u8]> for Hash<T> {
    fn as_ref(&self) -> &[u8] { self.digest.as_ref() }
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
    fn encode(&self) -> cbor::Value { cbor::Value::Bytes(cbor::Bytes::from_slice(self.as_ref())) }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.bytes().and_then(|bytes| {
            match Hash::from_slice(bytes.as_ref()) {
                Some(digest) => Ok(digest),
                None         => {
                    cbor::Result::bytes(bytes, cbor::Error::InvalidSize(32))
                }
            }
        }).embed("while decoding Hash")
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
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.u64().and_then(|v| {
            match Coin::new(v) {
                Some(coin) => Ok(coin),
                None       => cbor::Result::u64(v, cbor::Error::Between(0, MAX_COIN))
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TxOut {
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
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, addr) = cbor::array_decode_elem(array, 0)?;
            let (array, val)  = cbor::array_decode_elem(array, 0)?;
            if !array.is_empty() {
                cbor::Result::array(array, cbor::Error::UnparsedValues)
            } else {
                Ok(TxOut::new(addr, val))
            }
        })
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
                , cbor::Value::Tag(24, Box::new(cbor::Value::Bytes(cbor::Bytes::new(v))))
                ]
        )
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|sum_type| {
            let (sum_type, v) = cbor::array_decode_elem(sum_type, 0).embed("sum_type id")?;
            if v != 0u64 { return cbor::Result::array(sum_type, cbor::Error::InvalidSumtype(v)); }
            let (sum_type, tag) : (Vec<cbor::Value>, cbor::Value) = cbor::array_decode_elem(sum_type, 0).embed("sum_type's value")?;
            if sum_type.len() > 2 { return cbor::Result::array(sum_type, cbor::Error::UnparsedValues); }
            tag.tag().and_then(|(t, v)| {
                if t != 24 { return cbor::Result::tag(t, v, cbor::Error::InvalidTag(t)); }
                (*v).bytes()
            }).and_then(|bytes| {
                let (id, index) = cbor::decode_from_cbor(bytes.as_ref())?;
                Ok(TxIn::new(id, index))
            }).embed("while decoding `TxIn's inner sumtype`")
        }).embed("while decoding TxIn")
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Tx {
    inputs: LinkedList<TxIn>,
    outputs: LinkedList<TxOut>,
    // attributes: TxAttributes
    //
    // So far, there is no TxAttributes... the structure contains only the unparsed/unknown stuff
}
impl Tx {
    pub fn new() -> Self { Tx::new_with(LinkedList::new(), LinkedList::new()) }
    pub fn new_with(ins: LinkedList<TxIn>, outs: LinkedList<TxOut>) -> Self {
        Tx { inputs: ins, outputs: outs }
    }
    pub fn add_input(&mut self, i: TxIn) {
        self.inputs.push_back(i)
    }
    pub fn add_output(&mut self, o: TxOut) {
        self.outputs.push_back(o)
    }
}
impl cbor::CborValue for Tx {
    fn encode(&self) -> cbor::Value {
        let inputs  = cbor::CborValue::encode(&self.inputs);
        let outputs = cbor::CborValue::encode(&self.outputs);
        let attr    = cbor::Value::Object(BTreeMap::new());
        cbor::Value::Array(
            vec![ inputs
                , outputs
                , attr
                ]
        )
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.decode().and_then(|(input_values, output_values, _attributes) : (cbor::Value, cbor::Value, cbor::Value)| {
            let inputs  = input_values.decode().embed("while decoding Tx's TxIn")?;
            let outputs = output_values.decode().embed("while decoding Tx's TxOut")?;
            Ok(Tx::new_with(inputs, outputs))
        }).embed("while decoding Tx")
    }

}

pub struct TxAux {
    tx: Tx,
    witnesses: Vec<TxInWitness>,
}

pub struct TxProof {
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

    const TX: &'static [u8] = &[0x83, 0x9f, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a, 0x13, 0xb5, 0x0a, 0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6, 0x7b, 0xbb, 0x3f, 0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19, 0x02, 0x9a, 0xff, 0x9f, 0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee, 0xa1, 0xb5, 0xec, 0x8e, 0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45, 0xfd, 0x6d, 0x7b, 0x9e, 0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a, 0xff, 0xa0];
    const BLOCK: &'static [u8] = &[ /* TODO: insert Block here */ ];

    #[test]
    fn txout_decode() {
        let txout : TxOut = cbor::decode_from_cbor(TX_OUT).unwrap();

        let hdap = hdpayload::HDAddressPayload::from_vec(vec![1,2,3,4,5]);
        assert_eq!(Coin::new(42).unwrap(), txout.value);
        assert_eq!(address::AddrType::ATPubKey, txout.address.addr_type);
        assert_eq!(address::StakeDistribution::new_bootstrap_era(), txout.address.attributes.stake_distribution);
        assert_eq!(txout.address.attributes.derivation_path, Some(hdap));
    }

    #[test]
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
        let txin : TxIn = cbor::decode_from_cbor(TX_IN).unwrap();

        assert!(txin.index == 666);
    }

    #[test]
    fn txin_encode_decode() {
        let txid = TxId::new(&[0;32]);
        assert!(cbor::hs::encode_decode(&TxIn::new(txid, 666)));
    }

    #[test]
    fn tx_decode() {
        let txin : TxIn = cbor::decode_from_cbor(TX_IN).unwrap();
        let txout : TxOut = cbor::decode_from_cbor(TX_OUT).unwrap();
        let mut tx : Tx = cbor::decode_from_cbor(TX)
            .expect("Expecting to decode a `Tx`");

        assert!(tx.inputs.len() == 1);
        assert_eq!(Some(txin), tx.inputs.pop_front());
        assert!(tx.outputs.len() == 1);
        assert_eq!(Some(txout), tx.outputs.pop_front());
    }

    #[test]
    fn tx_encode_decode() {
        let txid = TxId::new(&[0;32]);
        let txin = TxIn::new(txid, 666);

        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let hdap = hdpayload::HDAddressPayload::from_vec(vec![1,2,3,4,5]);
        let addr_type = address::AddrType::ATPubKey;
        let sd = address::SpendingData::PubKeyASD(pk.clone());
        let attrs = address::Attributes::new_single_key(&pk, Some(hdap));
        let ea = address::ExtendedAddr::new(addr_type, sd, attrs);
        let value = Coin::new(42).unwrap();
        let txout = TxOut::new(ea, value);

        let mut tx = Tx::new();
        tx.add_input(txin);
        tx.add_output(txout);

        assert!(cbor::hs::encode_decode(&tx));
    }

    #[test]
    fn block_decode() {
        unimplemented!()
    }
}
