use std::{fmt, ops, iter, vec, slice, convert};
use std::collections::{LinkedList, BTreeMap};

use hash::{Blake2b256};

use cbor;
use cbor::{ExtendedResult};
use config::{Config};
use redeem;

use hdwallet::{Signature, XPub, XPrv};
use address::{ExtendedAddr, SpendingData};
use hdpayload;
use bip44::{Addressing};
use coin;
use coin::{Coin};

// TODO: this seems to be the hash of the serialisation CBOR of a given Tx.
// if this is confirmed, we need to make a proper type, wrapping it around
// to hash a `Tx` by serializing it cbor first.
pub type TxId = Blake2b256;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct TxOut {
    pub address: ExtendedAddr,
    pub value: Coin,
}
impl fmt::Display for TxOut {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -> {}", self.address, self.value)
    }
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum TxInWitness {
    /// signature of the `Tx` with the associated `XPub`
    /// the `XPub` is the public key set in the AddrSpendingData
    PkWitness(XPub, Signature<Tx>),
    ScriptWitness(ValidatorScript, RedeemerScript),
    RedeemWitness(redeem::PublicKey, redeem::Signature),
}
impl fmt::Display for TxInWitness {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl TxInWitness {
    /// create a TxInWitness from a given private key `XPrv` for the given transaction `Tx`.
    pub fn new(cfg: &Config, key: &XPrv, tx: &Tx) -> Self {
        let txid = cbor::encode_to_cbor(&tx.id()).unwrap();

        let mut vec = vec![ 0x01 ]; // this is the tag for TxSignature
        vec.extend_from_slice(&cbor::encode_to_cbor(&cfg.protocol_magic).unwrap());
        vec.extend_from_slice(&txid);
        TxInWitness::PkWitness(key.public(), key.sign(&vec))
    }

    /// verify a given extended address is associated to the witness.
    ///
    pub fn verify_address(&self, address: &ExtendedAddr) -> bool {
        match self {
            &TxInWitness::PkWitness(ref pk, _) => {
                let sd = SpendingData::PubKeyASD(pk.clone());
                let ea = ExtendedAddr::new(address.addr_type, sd, address.attributes.clone());

                &ea == address
            },
            &TxInWitness::ScriptWitness(_, _) => { unimplemented!() },
            &TxInWitness::RedeemWitness(ref pk, _) => {
                let sd = SpendingData::RedeemASD(pk.clone());
                let ea = ExtendedAddr::new(address.addr_type, sd, address.attributes.clone());

                &ea == address
            },
        }
    }

    /// verify the signature against the given transation `Tx`
    ///
    pub fn verify_tx(&self, cfg: &Config, tx: &Tx) -> bool {
        match self {
            &TxInWitness::PkWitness(ref pk, ref sig) => {
                let txid = cbor::encode_to_cbor(&tx.id()).unwrap();

                let mut vec = vec![ 0x01 ]; // this is the tag for TxSignature
                vec.extend_from_slice(&cbor::encode_to_cbor(&cfg.protocol_magic).unwrap());
                vec.extend_from_slice(&txid);

                pk.verify(&vec, sig)
            },
            &TxInWitness::ScriptWitness(_, _) => { unimplemented!() },
            &TxInWitness::RedeemWitness(ref pk, ref sig) => {
                let txid = cbor::encode_to_cbor(&tx.id()).unwrap();

                let mut vec = vec![ 0x01 ]; // this is the tag for TxSignature
                vec.extend_from_slice(&cbor::encode_to_cbor(&cfg.protocol_magic).unwrap());
                vec.extend_from_slice(&txid);

                pk.verify(sig, &vec)
            },
        }
    }

    /// verify the address's public key and the transaction signature
    pub fn verify(&self, cfg: &Config, address: &ExtendedAddr, tx: &Tx) -> bool {
        self.verify_address(address) && self.verify_tx(&cfg, tx)
    }
}
impl cbor::CborValue for TxInWitness {
    fn encode(&self) -> cbor::Value {
        let (i, bytes) = match self {
            &TxInWitness::PkWitness(ref pk, ref sig) => {
                let v = cbor::Value::Array(
                    vec![ cbor::CborValue::encode(pk)
                        , cbor::CborValue::encode(sig)
                        ]
                );
                (0u64, cbor::encode_to_cbor(&v).unwrap())
            },
            &TxInWitness::ScriptWitness(_, _) => { unimplemented!() },
            &TxInWitness::RedeemWitness(ref pk, ref sig) => {
                let v = cbor::Value::Array(
                    vec![ cbor::CborValue::encode(pk)
                        , cbor::CborValue::encode(sig)
                        ]
                );
                (2u64, cbor::encode_to_cbor(&v).unwrap())
            }
        };
        cbor::Value::Array(
            vec![ cbor::CborValue::encode(&i)
                , cbor::Value::Tag(24, Box::new(cbor::Value::Bytes(cbor::Bytes::new(bytes))))
                ]
        )
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|sum_type| {
            let (sum_type, v) = cbor::array_decode_elem(sum_type, 0).embed("sum_type's id")?;
            match v {
                0u64 => {
                    let (sum_type, tag) : (Vec<cbor::Value>, cbor::Value) = cbor::array_decode_elem(sum_type, 0).embed("sum_type's value")?;
                    if !sum_type.is_empty() { return cbor::Result::array(sum_type, cbor::Error::UnparsedValues); }
                    tag.tag().and_then(|(t, v)| {
                        if t != 24 { return cbor::Result::tag(t, v, cbor::Error::InvalidTag(t)); }
                        (*v).bytes()
                    }).and_then(|bytes| {
                        let (pk, sig) = cbor::decode_from_cbor(bytes.as_ref())?;
                        Ok(TxInWitness::PkWitness(pk, sig))
                    }).embed("while decoding `TxInWitness::PkWitness`")
                },
                2u64 => {
                    let (sum_type, tag) : (Vec<cbor::Value>, cbor::Value) = cbor::array_decode_elem(sum_type, 0).embed("sum_type's value")?;
                    if !sum_type.is_empty() { return cbor::Result::array(sum_type, cbor::Error::UnparsedValues); }
                    tag.tag().and_then(|(t, v)| {
                        if t != 24 { return cbor::Result::tag(t, v, cbor::Error::InvalidTag(t)); }
                        (*v).bytes()
                    }).and_then(|bytes| {
                        let (pk, sig) = cbor::decode_from_cbor(bytes.as_ref())?;
                        Ok(TxInWitness::RedeemWitness(pk, sig))
                    }).embed("while decoding `TxInWitness::RedeemWitness`")
                },
                _ => {
                    cbor::Result::array(sum_type, cbor::Error::InvalidSumtype(v))
                }
            }
        }).embed("While decoding TxInWitness")
    }
}

/// Structure used for addressing a specific output of a transaction
/// built from a TxId (hash of the tx) and the offset in the outputs of this
/// transaction.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct TxIn {
    pub id: TxId,
    pub index: u32,
}
impl fmt::Display for TxIn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@{}", self.id, self.index)
    }
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
            if !sum_type.is_empty() { return cbor::Result::array(sum_type, cbor::Error::UnparsedValues); }
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Tx {
    pub inputs: LinkedList<TxIn>,
    pub outputs: LinkedList<TxOut>,
    // attributes: TxAttributes
    //
    // So far, there is no TxAttributes... the structure contains only the unparsed/unknown stuff
}
impl fmt::Display for Tx {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for input in self.inputs.iter() {
            writeln!(f, "-> {}", input)?;
        }
        for output in self.outputs.iter() {
            writeln!(f, "   {} ->", output)?;
        }
        write!(f, "")
    }
}
impl Tx {
    pub fn new() -> Self { Tx::new_with(LinkedList::new(), LinkedList::new()) }
    pub fn new_with(ins: LinkedList<TxIn>, outs: LinkedList<TxOut>) -> Self {
        Tx { inputs: ins, outputs: outs }
    }
    pub fn id(&self) -> TxId {
        let buf = cbor::encode_to_cbor(self).expect("to cbor-encode a Tx in a vector in memory");
        TxId::new(&buf)
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

/// This is a Resolved version of a `TxIn`.
///
/// It contains the `TxIn` which is the value we need to put in the
/// transaction to reference funds to input to the transation.
///
/// It also contains the `TxOut` the value present at the given
/// `TxIn`'s `TxId` and _index_ in the block chain.
///
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Input {
    pub ptr:   TxIn,
    pub value: TxOut,
    pub addressing: Addressing
}
impl Input {
    pub fn new(ptr: TxIn, value: TxOut, addressing: Addressing) -> Self
    { Input { ptr: ptr, value: value, addressing: addressing } }

    pub fn value(&self) -> Coin { self.value.value }

    pub fn get_derivation_path(&self, key: &hdpayload::HDKey) -> Option<hdpayload::Path> {
        match &self.value.address.attributes.derivation_path {
            &Some(ref payload) => { key.decrypt_path(payload) },
            &None              => { None }
        }
    }
}

/// Collection of `Input` that will be used for creating a `Tx` and fee stabilisation
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Inputs(Vec<Input>);
impl Inputs {
    pub fn new() -> Self { Inputs(Vec::new()) }
    pub fn as_slice(&self) -> &[Input] { self.0.as_slice() }
    pub fn push(&mut self, i: Input) { self.0.push(i) }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn append(&mut self, other: &mut Self) { self.0.append(&mut other.0)}
}
impl convert::AsRef<Inputs> for Inputs {
    fn as_ref(&self) -> &Self { self }
}
impl convert::AsRef<[Input]> for Inputs {
    fn as_ref(&self) -> &[Input] { self.0.as_ref() }
}
impl ops::Deref for Inputs {
    type Target = [Input];

    fn deref(&self) -> &[Input] { self.0.deref() }
}
impl iter::FromIterator<Input> for Inputs {
    fn from_iter<I: IntoIterator<Item = Input>>(iter: I) -> Inputs {
        Inputs(iter::FromIterator::from_iter(iter))
    }
}
impl iter::Extend<Input> for Inputs {
    fn extend<I>(&mut self, i: I) where I: IntoIterator<Item=Input> {
        self.0.extend(i)
    }
}
impl IntoIterator for Inputs {
    type Item = Input;
    type IntoIter = vec::IntoIter<Input>;

    fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}
impl<'a> IntoIterator for &'a Inputs {
    type Item = &'a Input;
    type IntoIter = slice::Iter<'a, Input>;

    fn into_iter(self) -> Self::IntoIter { self.0.iter() }
}

/// Collection of `Input` that will be used for creating a `Tx` and fee stabilisation
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Outputs(Vec<TxOut>);
impl Outputs {
    pub fn new() -> Self { Outputs(Vec::new()) }
    pub fn as_slice(&self) -> &[TxOut] { self.0.as_slice() }
    pub fn push(&mut self, i: TxOut) { self.0.push(i) }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn append(&mut self, other: &mut Self) { self.0.append(&mut other.0)}

    pub fn total(&self) -> coin::Result<Coin> {
        self.iter().fold(Coin::new(0), |acc, ref c| acc.and_then(|v| v + c.value))
    }
}
impl convert::AsRef<Outputs> for Outputs {
    fn as_ref(&self) -> &Self { self }
}
impl convert::AsRef<[TxOut]> for Outputs {
    fn as_ref(&self) -> &[TxOut] { self.0.as_ref() }
}
impl ops::Deref for Outputs {
    type Target = [TxOut];

    fn deref(&self) -> &[TxOut] { self.0.deref() }
}
impl iter::FromIterator<TxOut> for Outputs {
    fn from_iter<I: IntoIterator<Item = TxOut>>(iter: I) -> Outputs {
        Outputs(iter::FromIterator::from_iter(iter))
    }
}
impl iter::Extend<TxOut> for Outputs {
    fn extend<I>(&mut self, i: I) where I: IntoIterator<Item=TxOut> {
        self.0.extend(i)
    }
}
impl IntoIterator for Outputs {
    type Item = TxOut;
    type IntoIter = vec::IntoIter<TxOut>;

    fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}
impl<'a> IntoIterator for &'a Outputs {
    type Item = &'a TxOut;
    type IntoIter = slice::Iter<'a, TxOut>;

    fn into_iter(self) -> Self::IntoIter { self.0.iter() }
}

pub mod fee {
    //! fee stabilisation related algorithm

    use std::{result, fmt};
    use super::*;

    /// fee
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
    pub struct Fee(Coin);
    impl Fee {
        pub fn new(coin: Coin) -> Self { Fee(coin) }
        pub fn to_coin(&self) -> Coin { self.0 }
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
    pub enum Error {
        NoInputs,
        NoOutputs,
        NotEnoughInput,
        CoinError(coin::Error)
    }
    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                &Error::NoInputs => write!(f, "No inputs given for fee estimation"),
                &Error::NoOutputs => write!(f, "No outputs given for fee estimation"),
                &Error::NotEnoughInput => write!(f, "Not enough funds to cover outputs and fees"),
                &Error::CoinError(err) => write!(f, "Error on coin operations: {}", err)
            }
        }
    }

    type Result<T> = result::Result<T, Error>;

    impl From<coin::Error> for Error {
        fn from(e: coin::Error) -> Error { Error::CoinError(e) }
    }

    pub trait Algorithm {
        fn compute(&self, policy: SelectionPolicy, inputs: &Inputs, outputs: &Outputs, change_addr: &ExtendedAddr) -> Result<(Fee, Inputs, Coin)>;
    }

    #[derive(Serialize, Deserialize, PartialEq, PartialOrd, Debug, Clone, Copy)]
    pub struct LinearFee {
        /// this is the minimal fee
        constant: f64,
        /// the transaction's size coefficient fee
        coefficient: f64
    }
    impl LinearFee {
        pub fn new(constant: f64, coefficient: f64) -> Self {
            LinearFee { constant: constant, coefficient: coefficient }
        }

        pub fn estimate(&self, sz: usize) -> Result<Fee> {
            let fee = self.constant + self.coefficient * (sz as f64);
            let coin = Coin::new(fee as u64)?;
            Ok(Fee(coin))
        }
    }
    impl Default for LinearFee {
        fn default() -> Self { LinearFee::new(155381.0, 43.946) }
    }

    const TX_IN_WITNESS_CBOR_SIZE: usize = 140;
    const CBOR_TXAUX_OVERHEAD: usize = 51;
    impl Algorithm for LinearFee {
        fn compute( &self
                  , policy: SelectionPolicy
                  , inputs: &Inputs
                  , outputs: &Outputs
                  , change_addr: &ExtendedAddr
                  )
            -> Result<(Fee, Inputs, Coin)>
        {
            if inputs.is_empty() { return Err(Error::NoInputs); }
            if outputs.is_empty() { return Err(Error::NoOutputs); }

            let output_value = outputs.total()?;
            let mut fee = self.estimate(0)?;
            let mut input_value = Coin::zero();
            let mut selected_inputs = Inputs::new();

            // create the Tx on the fly
            let mut txins = LinkedList::new();
            let     txouts : LinkedList<TxOut> = outputs.iter().cloned().collect();

            // for now we only support this selection algorithm
            // we need to remove this assert when we extend to more
            // granulated selection policy
            assert!(policy == SelectionPolicy::FirstMatchFirst);

            for input in inputs.iter() {
                input_value = (input_value + input.value())?;
                selected_inputs.push(input.clone());
                txins.push_back(input.ptr.clone());

                // calculate fee from the Tx serialised + estimated size for signing
                let mut tx = Tx::new_with(txins.clone(), txouts.clone());
                let txbytes = cbor::encode_to_cbor(&tx).unwrap();

                let estimated_fee = (self.estimate(txbytes.len() + CBOR_TXAUX_OVERHEAD + (TX_IN_WITNESS_CBOR_SIZE * selected_inputs.len())))?;

                // add the change in the estimated fee
                match output_value - input_value - estimated_fee.to_coin() {
                    None => {},
                    Some(change_value) => {
                        tx.add_output(TxOut::new(change_addr.clone(), change_value))
                    }
                };

                let txbytes = cbor::encode_to_cbor(&tx).unwrap();
                let corrected_fee = self.estimate(txbytes.len() + CBOR_TXAUX_OVERHEAD + (TX_IN_WITNESS_CBOR_SIZE * selected_inputs.len()));

                fee = corrected_fee?;

                if Ok(input_value) >= (output_value + fee.to_coin()) { break; }
            }

            if Ok(input_value) < (output_value + fee.to_coin()) {
                return Err(Error::NotEnoughInput);
            }

            Ok((fee, selected_inputs, (input_value - output_value - fee.to_coin()).unwrap()))
        }
    }

    /// the input selection method.
    ///
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
    pub enum SelectionPolicy {
        /// select the first inputs that matches, no optimisation
        FirstMatchFirst
    }
    impl Default for SelectionPolicy {
        fn default() -> Self { SelectionPolicy::FirstMatchFirst }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct TxAux {
    pub tx: Tx,
    pub witnesses: Vec<TxInWitness>,
}
impl fmt::Display for TxAux {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Tx:\n{}", self.tx)?;
        writeln!(f, "witnesses: {:?}\n", self.witnesses)
    }
}
impl TxAux {
    pub fn new(tx: Tx, witnesses: Vec<TxInWitness>) -> Self {
        TxAux { tx: tx, witnesses: witnesses }
    }
}
impl cbor::CborValue for TxAux {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(
            vec![ cbor::CborValue::encode(&self.tx)
                , cbor::CborValue::encode(&self.witnesses)
                ]
        )
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, tx)        = cbor::array_decode_elem(array, 0).embed("decoding Tx")?;
            let (array, witnesses) = cbor::array_decode_elem(array, 0).embed("decoding vector of witnesses")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(TxAux::new(tx, witnesses))
        }).embed("While decoding TxAux.")
    }
}

#[derive(Debug, Clone)]
pub struct TxProof {
    pub number: u32,
    pub root: Blake2b256,
    pub witnesses_hash: Blake2b256,
}
impl TxProof {
    pub fn new(number: u32, root: Blake2b256, witnesses_hash: Blake2b256) -> Self {
        TxProof {
            number: number,
            root: root,
            witnesses_hash: witnesses_hash
        }
    }
}
impl fmt::Display for TxProof {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "number: {}, root: {}, witnesses: {}", self.number, self.root, self.witnesses_hash)
    }
}
impl cbor::CborValue for TxProof {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(
            vec![ cbor::CborValue::encode(&self.number)
                , cbor::CborValue::encode(&self.root)
                , cbor::CborValue::encode(&self.witnesses_hash)
                ]
        )
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, number)    = cbor::array_decode_elem(array, 0).embed("number")?;
            let (array, root)      = cbor::array_decode_elem(array, 0).embed("root")?;
            let (array, witnesses) = cbor::array_decode_elem(array, 0).embed("witnesses")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(TxProof::new(number, root, witnesses))
        }).embed("While decoding TxAux.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use address;
    use hdpayload;
    use hdwallet;
    use cbor;
    use config::{Config};

    const SEED: [u8;hdwallet::SEED_SIZE] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

    const HDPAYLOAD: &'static [u8] = &[1,2,3,4,5];

    // CBOR encoded TxOut
    const TX_OUT: &'static [u8] = &[0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee, 0xa1, 0xb5, 0xec, 0x8e, 0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45, 0xfd, 0x6d, 0x7b, 0x9e, 0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a];
    const TX_IN:  &'static [u8] = &[0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a, 0x13, 0xb5, 0x0a, 0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6, 0x7b, 0xbb, 0x3f, 0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19, 0x02, 0x9a];

    const TX: &'static [u8] = &[0x83, 0x9f, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a, 0x13, 0xb5, 0x0a, 0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6, 0x7b, 0xbb, 0x3f, 0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19, 0x02, 0x9a, 0xff, 0x9f, 0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee, 0xa1, 0xb5, 0xec, 0x8e, 0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45, 0xfd, 0x6d, 0x7b, 0x9e, 0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a, 0xff, 0xa0];

    const TX_IN_WITNESS: &'static [u8] = &[0x82, 0x00, 0xd8, 0x18, 0x58, 0x85, 0x82, 0x58, 0x40, 0x1c, 0x0c, 0x3a, 0xe1, 0x82, 0x5e, 0x90, 0xb6, 0xdd, 0xda, 0x3f, 0x40, 0xa1, 0x22, 0xc0, 0x07, 0xe1, 0x00, 0x8e, 0x83, 0xb2, 0xe1, 0x02, 0xc1, 0x42, 0xba, 0xef, 0xb7, 0x21, 0xd7, 0x2c, 0x1a, 0x5d, 0x36, 0x61, 0xde, 0xb9, 0x06, 0x4f, 0x2d, 0x0e, 0x03, 0xfe, 0x85, 0xd6, 0x80, 0x70, 0xb2, 0xfe, 0x33, 0xb4, 0x91, 0x60, 0x59, 0x65, 0x8e, 0x28, 0xac, 0x7f, 0x7f, 0x91, 0xca, 0x4b, 0x12, 0x58, 0x40, 0x9d, 0x6d, 0x91, 0x1e, 0x58, 0x8d, 0xd4, 0xfb, 0x77, 0xcb, 0x80, 0xc2, 0xc6, 0xad, 0xbc, 0x2b, 0x94, 0x2b, 0xce, 0xa5, 0xd8, 0xa0, 0x39, 0x22, 0x0d, 0xdc, 0xd2, 0x35, 0xcb, 0x75, 0x86, 0x2c, 0x0c, 0x95, 0xf6, 0x2b, 0xa1, 0x11, 0xe5, 0x7d, 0x7c, 0x1a, 0x22, 0x1c, 0xf5, 0x13, 0x3e, 0x44, 0x12, 0x88, 0x32, 0xc1, 0x49, 0x35, 0x4d, 0x1e, 0x57, 0xb6, 0x80, 0xfe, 0x57, 0x2d, 0x76, 0x0c];

    const TX_AUX : &'static [u8] = &[0x82, 0x83, 0x9f, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a, 0x13, 0xb5, 0x0a, 0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6, 0x7b, 0xbb, 0x3f, 0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19, 0x02, 0x9a, 0xff, 0x9f, 0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee, 0xa1, 0xb5, 0xec, 0x8e, 0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45, 0xfd, 0x6d, 0x7b, 0x9e, 0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a, 0xff, 0xa0, 0x81, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x85, 0x82, 0x58, 0x40, 0x1c, 0x0c, 0x3a, 0xe1, 0x82, 0x5e, 0x90, 0xb6, 0xdd, 0xda, 0x3f, 0x40, 0xa1, 0x22, 0xc0, 0x07, 0xe1, 0x00, 0x8e, 0x83, 0xb2, 0xe1, 0x02, 0xc1, 0x42, 0xba, 0xef, 0xb7, 0x21, 0xd7, 0x2c, 0x1a, 0x5d, 0x36, 0x61, 0xde, 0xb9, 0x06, 0x4f, 0x2d, 0x0e, 0x03, 0xfe, 0x85, 0xd6, 0x80, 0x70, 0xb2, 0xfe, 0x33, 0xb4, 0x91, 0x60, 0x59, 0x65, 0x8e, 0x28, 0xac, 0x7f, 0x7f, 0x91, 0xca, 0x4b, 0x12, 0x58, 0x40, 0x9d, 0x6d, 0x91, 0x1e, 0x58, 0x8d, 0xd4, 0xfb, 0x77, 0xcb, 0x80, 0xc2, 0xc6, 0xad, 0xbc, 0x2b, 0x94, 0x2b, 0xce, 0xa5, 0xd8, 0xa0, 0x39, 0x22, 0x0d, 0xdc, 0xd2, 0x35, 0xcb, 0x75, 0x86, 0x2c, 0x0c, 0x95, 0xf6, 0x2b, 0xa1, 0x11, 0xe5, 0x7d, 0x7c, 0x1a, 0x22, 0x1c, 0xf5, 0x13, 0x3e, 0x44, 0x12, 0x88, 0x32, 0xc1, 0x49, 0x35, 0x4d, 0x1e, 0x57, 0xb6, 0x80, 0xfe, 0x57, 0x2d, 0x76, 0x0c];

    #[test]
    fn txout_decode() {
        let txout : TxOut = cbor::decode_from_cbor(TX_OUT).unwrap();

        let hdap = hdpayload::HDAddressPayload::from_bytes(HDPAYLOAD);
        assert_eq!(Coin::new(42).unwrap(), txout.value);
        assert_eq!(address::AddrType::ATPubKey, txout.address.addr_type);
        assert_eq!(address::StakeDistribution::new_bootstrap_era(), txout.address.attributes.stake_distribution);
        assert_eq!(txout.address.attributes.derivation_path, Some(hdap));
    }

    #[test]
    fn txout_encode_decode() {
        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let hdap = hdpayload::HDAddressPayload::from_bytes(HDPAYLOAD);
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

        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let hdap = hdpayload::HDAddressPayload::from_bytes(HDPAYLOAD);
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
    fn txinwitness_decode() {
        let cfg = Config::default();
        let txinwitness : TxInWitness = cbor::decode_from_cbor(TX_IN_WITNESS).expect("to decode a `TxInWitness`");
        let tx : Tx = cbor::decode_from_cbor(TX).expect("to decode a `Tx`");

        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);

        assert!(txinwitness == TxInWitness::new(&cfg, &sk, &tx));
    }

    #[test]
    fn txinwitness_encode_decode() {
        let cfg = Config::default();
        let tx : Tx = cbor::decode_from_cbor(TX).expect("to decode a `Tx`");

        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);

        let txinwitness = TxInWitness::new(&cfg, &sk, &tx);

        assert!(cbor::hs::encode_decode(&txinwitness));
    }

    #[test]
    fn txinwitness_sign_verify() {
        let cfg = Config::default();
        // create wallet's keys
        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        // create an Address
        let hdap = hdpayload::HDAddressPayload::from_bytes(HDPAYLOAD);
        let addr_type = address::AddrType::ATPubKey;
        let sd = address::SpendingData::PubKeyASD(pk.clone());
        let attrs = address::Attributes::new_single_key(&pk, Some(hdap));
        let ea = address::ExtendedAddr::new(addr_type, sd, attrs);

        // create a transaction
        let txid = TxId::new(&[0;32]);
        let txin = TxIn::new(txid, 666);
        let value = Coin::new(42).unwrap();
        let txout = TxOut::new(ea.clone(), value);
        let mut tx = Tx::new();
        tx.add_input(txin);
        tx.add_output(txout);

        // here we pretend that `ea` is the address we find from the found we want
        // to take. In the testing case, it is not important that it is also the
        // txout of this given transation

        // create a TxInWitness (i.e. sign the given transaction)
        let txinwitness = TxInWitness::new(&cfg, &sk, &tx);

        // check the address is the correct one
        assert!(txinwitness.verify_address(&ea));
        assert!(txinwitness.verify_tx(&cfg, &tx));
        assert!(txinwitness.verify(&cfg, &ea, &tx));
    }

    #[test]
    fn txaux_decode() {
        let _txaux : TxAux = cbor::decode_from_cbor(TX_AUX).expect("to decode a TxAux");
    }

    #[test]
    fn txaux_encode_decode() {
        let tx : Tx = cbor::decode_from_cbor(TX).expect("to decode a `Tx`");
        let txinwitness : TxInWitness = cbor::decode_from_cbor(TX_IN_WITNESS).expect("to decode a `TxInWitness`");

        let witnesses = vec![txinwitness];

        let txaux = TxAux::new(tx, witnesses);

        assert!(cbor::hs::encode_decode(&txaux));
    }
}
