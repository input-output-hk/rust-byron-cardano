//! Transaction types
//!
//! `TxoPointer` : Input
//! `TxOut` : Output
//! `Tx` : Input + Output
//! `TxInWitness`: Witness providing for TxoPointer (e.g. cryptographic signature)
//! `TxAux` : Signed Tx (Tx + Witness)
//!
use std::{
    fmt,
    io::{BufRead, Write},
};

use crate::{
    address::{AddrType, Attributes, ExtendedAddr, SpendingData},
    coin::{self, Coin},
    config::ProtocolMagic,
    hash::Blake2b256,
    hdwallet::{Signature, XPrv, XPub, SIGNATURE_SIZE, XPUB_SIZE},
    merkle, redeem,
    tags::SigningTag,
};

use cbor_event::{self, de::Deserializer, se::Serializer};
use chain_core::property;

// Transaction IDs are either a hash of the CBOR serialisation of a
// given Tx, or a hash of a redeem address.
pub type TxId = Blake2b256;

impl property::TransactionId for TxId {}

pub fn redeem_pubkey_to_txid(
    pubkey: &redeem::PublicKey,
    protocol_magic: ProtocolMagic,
) -> (TxId, ExtendedAddr) {
    let address = ExtendedAddr::new(
        AddrType::ATRedeem,
        SpendingData::RedeemASD(*pubkey),
        Attributes::new_bootstrap_era(None, protocol_magic.into()),
    );
    let txid = Blake2b256::new(&cbor!(&address).unwrap());
    (txid, address)
}

/// Tx Output composed of an address and a coin value
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
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
        TxOut {
            address: addr,
            value: value,
        }
    }
}
impl cbor_event::de::Deserialize for TxOut {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        reader.tuple(2, "TxOut")?;
        let addr = cbor_event::de::Deserialize::deserialize(reader)?;
        let val = cbor_event::de::Deserialize::deserialize(reader)?;
        Ok(TxOut::new(addr, val))
    }
}
impl cbor_event::se::Serialize for TxOut {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(2))?
            .serialize(&self.address)?
            .serialize(&self.value)
    }
}

type TODO = u8;
type ValidatorScript = TODO;
type RedeemerScript = TODO;

/// Provide a witness to a specific transaction, generally by revealing
/// all the hidden information from the tx and cryptographic signatures.
///
/// Witnesses are of types:
/// * PkWitness: a simple witness for a PubKeyASD type, which is composed
///              of the revealed XPub associated with the address and
///              the associated signature of the tx.
/// * ScriptWitness: a witness for ScriptASD.
/// * RedeemWitness: a witness for RedeemASD type, similar to PkWitness
///                  but for normal Public Key.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
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
    /// this is used to create a fake signature useful for fee evaluation
    pub fn fake() -> Self {
        let fakesig = Signature::from_bytes([0u8; SIGNATURE_SIZE]);
        TxInWitness::PkWitness(XPub::from_bytes([0u8; XPUB_SIZE]), fakesig)
    }

    /// create a TxInWitness from a given private key `XPrv` for the given transaction id `TxId`.
    #[deprecated(note = "use new_extended_pk method instead")]
    pub fn new(protocol_magic: ProtocolMagic, key: &XPrv, txid: &TxId) -> Self {
        Self::new_extended_pk(protocol_magic, key, txid)
    }

    /// create a TxInWitness from a given private key `XPrv` for the given transaction id `TxId`.
    pub fn new_extended_pk(protocol_magic: ProtocolMagic, key: &XPrv, txid: &TxId) -> Self {
        let vec = Self::prepare_byte_to_sign(protocol_magic, SigningTag::Tx, txid);
        TxInWitness::PkWitness(key.public(), key.sign(&vec))
    }

    /// create a TxInWitness from a given Redeem key
    pub fn new_redeem_pk(
        protocol_magic: ProtocolMagic,
        key: &redeem::PrivateKey,
        txid: &TxId,
    ) -> Self {
        let vec = Self::prepare_byte_to_sign(protocol_magic, SigningTag::RedeemTx, txid);
        TxInWitness::RedeemWitness(key.public(), key.sign(&vec))
    }

    fn prepare_byte_to_sign(
        protocol_magic: ProtocolMagic,
        sign_tag: SigningTag,
        txid: &TxId,
    ) -> Vec<u8> {
        let mut se = Serializer::new_vec();
        se.write_unsigned_integer(sign_tag as u64)
            .expect("write the sign tag")
            .serialize(&protocol_magic)
            .expect("serialize protocol magic")
            .serialize(txid)
            .expect("serialize Tx's Id");
        se.finalize()
    }

    /// verify a given extended address is associated to the witness.
    ///
    pub fn verify_address(&self, address: &ExtendedAddr) -> bool {
        match self {
            &TxInWitness::PkWitness(ref pk, _) => {
                let sd = SpendingData::PubKeyASD(pk.clone());
                let ea = ExtendedAddr::new(address.addr_type, sd, address.attributes.clone());

                &ea == address
            }
            &TxInWitness::ScriptWitness(_, _) => unimplemented!(),
            &TxInWitness::RedeemWitness(ref pk, _) => {
                let sd = SpendingData::RedeemASD(pk.clone());
                let ea = ExtendedAddr::new(address.addr_type, sd, address.attributes.clone());

                &ea == address
            }
        }
    }

    /// verify the signature against the given transation `Tx`
    ///
    pub fn verify_tx(&self, protocol_magic: ProtocolMagic, tx: &Tx) -> bool {
        let vec = Self::prepare_byte_to_sign(protocol_magic, self.get_sign_tag(), &tx.id());
        match self {
            &TxInWitness::PkWitness(ref pk, ref sig) => pk.verify(&vec, sig),
            &TxInWitness::ScriptWitness(_, _) => unimplemented!(),
            &TxInWitness::RedeemWitness(ref pk, ref sig) => pk.verify(sig, &vec),
        }
    }

    fn get_sign_tag(&self) -> SigningTag {
        match self {
            &TxInWitness::PkWitness(_, _) => SigningTag::Tx,
            &TxInWitness::ScriptWitness(_, _) => unimplemented!(),
            &TxInWitness::RedeemWitness(_, _) => SigningTag::RedeemTx,
        }
    }

    /// verify the address's public key and the transaction signature
    pub fn verify(&self, protocol_magic: ProtocolMagic, address: &ExtendedAddr, tx: &Tx) -> bool {
        self.verify_address(address) && self.verify_tx(protocol_magic, tx)
    }
}
impl cbor_event::se::Serialize for TxInWitness {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(2))?;
        let inner_serializer = match self {
            &TxInWitness::PkWitness(ref xpub, ref signature) => {
                serializer.write_unsigned_integer(0)?;
                let mut se = Serializer::new_vec();
                se.write_array(cbor_event::Len::Len(2))?
                    .serialize(xpub)?
                    .serialize(signature)?;
                se
            }
            &TxInWitness::ScriptWitness(_, _) => unimplemented!(),
            &TxInWitness::RedeemWitness(ref pk, ref signature) => {
                serializer.write_unsigned_integer(2)?;
                let mut se = Serializer::new_vec();
                se.write_array(cbor_event::Len::Len(2))?
                    .serialize(pk)?
                    .serialize(signature)?;
                se
            }
        };
        serializer
            .write_tag(24)?
            .write_bytes(&inner_serializer.finalize())
    }
}
impl cbor_event::de::Deserialize for TxInWitness {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(2, "TxInWitness")?;
        let sum_type_idx = raw.unsigned_integer()?;
        match sum_type_idx {
            0 => {
                let tag = raw.tag()?;
                if tag != 24 {
                    return Err(cbor_event::Error::CustomError(format!(
                        "Invalid Tag: {} but expected 24",
                        tag
                    )));
                }
                let bytes = raw.bytes()?;
                let mut raw = Deserializer::from(std::io::Cursor::new(bytes));
                raw.tuple(2, "TxInWitness::PkWitness")?;
                let pk = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                let sig = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                Ok(TxInWitness::PkWitness(pk, sig))
            }
            2 => {
                let tag = raw.tag()?;
                if tag != 24 {
                    return Err(cbor_event::Error::CustomError(format!(
                        "Invalid Tag: {} but expected 24",
                        tag
                    )));
                }
                let bytes = raw.bytes()?;
                let mut raw = Deserializer::from(std::io::Cursor::new(bytes));
                raw.tuple(2, "TxInWitness::PkRedeemWitness")?;
                let pk = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                let sig = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                Ok(TxInWitness::RedeemWitness(pk, sig))
            }
            _ => Err(cbor_event::Error::CustomError(format!(
                "Unsupported TxInWitness: {}",
                sum_type_idx
            ))),
        }
    }
}

/// Structure used for addressing a specific output of a transaction
/// built from a TxId (hash of the tx) and the offset in the outputs of this
/// transaction.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct TxoPointer {
    pub id: TxId,
    pub index: u32,
}

/// old haskell name for TxoPointer
#[deprecated]
pub type TxIn = TxoPointer;

impl fmt::Display for TxoPointer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@{}", self.id, self.index)
    }
}
impl TxoPointer {
    pub fn new(id: TxId, index: u32) -> Self {
        TxoPointer {
            id: id,
            index: index,
        }
    }
}
impl cbor_event::se::Serialize for TxoPointer {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(2))?
            .write_unsigned_integer(0)?
            .write_tag(24)?
            .write_bytes(&cbor!(&(&self.id, &self.index))?)
    }
}
impl cbor_event::de::Deserialize for TxoPointer {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(2, "TxoPointer")?;
        let sum_type_idx = raw.unsigned_integer()?;
        if sum_type_idx != 0 {
            return Err(cbor_event::Error::CustomError(format!(
                "Unsupported TxoPointer: {}",
                sum_type_idx
            )));
        }
        let tag = raw.tag()?;
        if tag != 24 {
            return Err(cbor_event::Error::CustomError(format!(
                "Invalid Tag: {} but expected 24",
                tag
            )));
        }
        let bytes = raw.bytes()?;
        let mut raw = Deserializer::from(std::io::Cursor::new(bytes));
        raw.tuple(2, "TxoPointer")?;
        let id = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        let idx = raw.unsigned_integer()?;
        Ok(TxoPointer::new(id, idx as u32))
    }
}

/// A Transaction containing tx inputs and tx outputs.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct Tx {
    pub inputs: Vec<TxoPointer>,
    pub outputs: Vec<TxOut>,
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
    pub fn new() -> Self {
        Tx::new_with(Vec::new(), Vec::new())
    }
    pub fn new_with(ins: Vec<TxoPointer>, outs: Vec<TxOut>) -> Self {
        Tx {
            inputs: ins,
            outputs: outs,
        }
    }
    pub fn id(&self) -> TxId {
        let buf = cbor!(self).expect("encode Tx");
        TxId::new(&buf)
    }
    pub fn add_input(&mut self, i: TxoPointer) {
        self.inputs.push(i)
    }
    pub fn add_output(&mut self, o: TxOut) {
        self.outputs.push(o)
    }
    pub fn get_output_total(&self) -> coin::Result<Coin> {
        let mut total = Coin::zero();
        for ref o in self.outputs.iter() {
            total = (total + o.value)?;
        }
        Ok(total)
    }
}
impl cbor_event::se::Serialize for Tx {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(3))?;
        cbor_event::se::serialize_indefinite_array(self.inputs.iter(), serializer)?;
        cbor_event::se::serialize_indefinite_array(self.outputs.iter(), serializer)?;
        serializer.write_map(cbor_event::Len::Len(0))
    }
}
impl cbor_event::de::Deserialize for Tx {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(3, "Tx")?;

        // Note: these must be indefinite-size arrays.
        let inputs = cbor_event::de::Deserialize::deserialize(raw)?;
        let outputs = cbor_event::de::Deserialize::deserialize(raw)?;

        let map_len = raw.map()?;
        if !map_len.is_null() {
            return Err(cbor_event::Error::CustomError(format!(
                "Invalid Tx: we do not support Tx extra data... {:?} elements",
                map_len
            )));
        }
        Ok(Tx::new_with(inputs, outputs))
    }
}

/// A transaction witness is a vector of input witnesses
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct TxWitness(Vec<TxInWitness>);

impl TxWitness {
    pub fn new() -> Self {
        TxWitness(Vec::new())
    }
}
impl From<Vec<TxInWitness>> for TxWitness {
    fn from(v: Vec<TxInWitness>) -> Self {
        TxWitness(v)
    }
}
impl ::std::iter::FromIterator<TxInWitness> for TxWitness {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = TxInWitness>,
    {
        TxWitness(Vec::from_iter(iter))
    }
}
impl ::std::ops::Deref for TxWitness {
    type Target = Vec<TxInWitness>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ::std::ops::DerefMut for TxWitness {
    fn deref_mut(&mut self) -> &mut Vec<TxInWitness> {
        &mut self.0
    }
}

impl cbor_event::de::Deserialize for TxWitness {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        Ok(TxWitness(cbor_event::de::Deserialize::deserialize(raw)?))
    }
}

impl cbor_event::se::Serialize for TxWitness {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        txwitness_serialize(&self.0, serializer)
    }
}

pub fn txwitness_serialize<'se, W>(
    in_witnesses: &Vec<TxInWitness>,
    serializer: &'se mut Serializer<W>,
) -> cbor_event::Result<&'se mut Serializer<W>>
where
    W: Write,
{
    cbor_event::se::serialize_fixed_array(in_witnesses.iter(), serializer)
}

/// A transaction witness is a vector of input witnesses
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct TxWitnesses {
    pub in_witnesses: Vec<TxWitness>,
}

impl TxWitnesses {
    pub fn new(in_witnesses: Vec<TxWitness>) -> Self {
        TxWitnesses {
            in_witnesses: in_witnesses,
        }
    }
}

impl ::std::ops::Deref for TxWitnesses {
    type Target = Vec<TxWitness>;
    fn deref(&self) -> &Self::Target {
        &self.in_witnesses
    }
}

impl cbor_event::se::Serialize for TxWitnesses {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        cbor_event::se::serialize_indefinite_array(self.iter(), serializer)
    }
}

/// Tx with the vector of witnesses
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct TxAux {
    pub tx: Tx,
    pub witness: TxWitness,
}
impl fmt::Display for TxAux {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Tx:\n{}", self.tx)?;
        writeln!(f, "witnesses: {:?}\n", self.witness)
    }
}
impl TxAux {
    pub fn new(tx: Tx, witness: TxWitness) -> Self {
        TxAux {
            tx: tx,
            witness: witness,
        }
    }
}
impl cbor_event::de::Deserialize for TxAux {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(2, "TxAux")?;
        let tx = cbor_event::de::Deserialize::deserialize(raw)?;
        let witness = cbor_event::de::Deserialize::deserialize(raw)?;
        Ok(TxAux::new(tx, witness))
    }
}
impl cbor_event::se::Serialize for TxAux {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        txaux_serialize(&self.tx, &self.witness, serializer)
    }
}

pub fn txaux_serialize<'se, W>(
    tx: &Tx,
    in_witnesses: &Vec<TxInWitness>,
    serializer: &'se mut Serializer<W>,
) -> cbor_event::Result<&'se mut Serializer<W>>
where
    W: Write,
{
    serializer
        .write_array(cbor_event::Len::Len(2))?
        .serialize(tx)?;
    txwitness_serialize(in_witnesses, serializer)
}

pub fn txaux_serialize_size(tx: &Tx, in_witnesses: &Vec<TxInWitness>) -> usize {
    use std::io::Write;

    struct Cborsize(usize);
    impl Write for Cborsize {
        fn write(&mut self, bytes: &[u8]) -> ::std::result::Result<usize, ::std::io::Error> {
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> ::std::result::Result<(), ::std::io::Error> {
            Ok(())
        }
    }

    let mut ser = cbor_event::se::Serializer::new(Cborsize(0));
    txaux_serialize(tx, in_witnesses, &mut ser).unwrap();
    let cborsize = ser.finalize();
    cborsize.0
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxProof {
    /// Number of Transactions in this tree
    pub number: u32,
    /// Root of the merkle tree of transactions
    pub root: merkle::Hash,
    /// Hash of Sequence of TxWitnesses encoded in CBOR
    pub witnesses_hash: Blake2b256,
}
impl TxProof {
    pub fn new(number: u32, root: merkle::Hash, witnesses_hash: Blake2b256) -> Self {
        TxProof {
            number: number,
            root: root,
            witnesses_hash: witnesses_hash,
        }
    }

    pub fn generate(txaux: &[TxAux]) -> Self {
        let txs: Vec<&Tx> = txaux.iter().map(|w| &w.tx).collect();
        let witnesses: Vec<&TxWitness> = txaux.iter().map(|w| &w.witness).collect();
        let mut ser = cbor_event::se::Serializer::new_vec();
        cbor_event::se::serialize_indefinite_array(witnesses.iter(), &mut ser).unwrap();
        let out = ser.finalize();
        TxProof {
            number: txs.len() as u32,
            root: merkle::MerkleTree::new(&txs[..]).get_root_hash(),
            witnesses_hash: Blake2b256::new(&out[..]),
        }
    }
}
impl fmt::Display for TxProof {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "number: {}, root: {}, witnesses: {}",
            self.number, self.root, self.witnesses_hash
        )
    }
}
impl cbor_event::se::Serialize for TxProof {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(3))?
            .write_unsigned_integer(self.number as u64)?
            .serialize(&self.root)?
            .serialize(&self.witnesses_hash)
    }
}
impl cbor_event::de::Deserialize for TxProof {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(3, "TxProof")?;
        let number = raw.unsigned_integer()?;
        let root = cbor_event::de::Deserialize::deserialize(raw)?;
        let witnesses = cbor_event::de::Deserialize::deserialize(raw)?;
        Ok(TxProof::new(number as u32, root, witnesses))
    }
}

impl chain_core::property::Serialize for Tx {
    type Error = cbor_event::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        let mut serializer = cbor_event::se::Serializer::new(writer);
        serializer.serialize(self)?;
        serializer.finalize();
        Ok(())
    }
}

impl chain_core::property::Deserialize for Tx {
    type Error = cbor_event::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let mut deserializer = cbor_event::de::Deserializer::from(reader);
        deserializer.deserialize::<Self>()
    }
}

impl chain_core::property::Serialize for TxAux {
    type Error = cbor_event::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        let mut serializer = cbor_event::se::Serializer::new(writer);
        serializer.serialize(self)?;
        serializer.finalize();
        Ok(())
    }
}

impl chain_core::property::Deserialize for TxAux {
    type Error = cbor_event::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let mut deserializer = cbor_event::de::Deserializer::from(reader);
        deserializer.deserialize::<Self>()
    }
}

impl chain_core::property::Transaction for Tx {
    type Input = TxoPointer;
    type Output = TxOut;
    type Inputs = [TxoPointer];
    type Outputs = [TxOut];

    fn inputs(&self) -> &Self::Inputs {
        &self.inputs
    }
    fn outputs(&self) -> &Self::Outputs {
        &self.outputs
    }
}
impl chain_core::property::Transaction for TxAux {
    type Input = TxoPointer;
    type Output = TxOut;
    type Inputs = [TxoPointer];
    type Outputs = [TxOut];

    fn inputs(&self) -> &Self::Inputs {
        &self.tx.inputs
    }
    fn outputs(&self) -> &Self::Outputs {
        &self.tx.outputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use address;
    use cbor_event::{self, de::Deserializer};
    use config::NetworkMagic;
    use hdpayload;
    use hdwallet;

    const SEED: [u8; hdwallet::SEED_SIZE] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];

    const HDPAYLOAD: &'static [u8] = &[1, 2, 3, 4, 5];

    // CBOR encoded TxOut
    const TX_OUT: &'static [u8] = &[
        0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee, 0xa1, 0xb5, 0xec, 0x8e,
        0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45, 0xfd, 0x6d, 0x7b, 0x9e,
        0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04,
        0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a,
    ];
    const TX_IN: &'static [u8] = &[
        0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a, 0x13, 0xb5, 0x0a,
        0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6, 0x7b, 0xbb, 0x3f,
        0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19, 0x02, 0x9a,
    ];

    const TX: &'static [u8] = &[
        0x83, 0x9f, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a, 0x13,
        0xb5, 0x0a, 0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6, 0x7b,
        0xbb, 0x3f, 0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19, 0x02,
        0x9a, 0xff, 0x9f, 0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee, 0xa1,
        0xb5, 0xec, 0x8e, 0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45, 0xfd,
        0x6d, 0x7b, 0x9e, 0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45, 0x01,
        0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a, 0xff, 0xa0,
    ];

    const TX_IN_WITNESS: &'static [u8] = &[
        0x82, 0x00, 0xd8, 0x18, 0x58, 0x85, 0x82, 0x58, 0x40, 0x1c, 0x0c, 0x3a, 0xe1, 0x82, 0x5e,
        0x90, 0xb6, 0xdd, 0xda, 0x3f, 0x40, 0xa1, 0x22, 0xc0, 0x07, 0xe1, 0x00, 0x8e, 0x83, 0xb2,
        0xe1, 0x02, 0xc1, 0x42, 0xba, 0xef, 0xb7, 0x21, 0xd7, 0x2c, 0x1a, 0x5d, 0x36, 0x61, 0xde,
        0xb9, 0x06, 0x4f, 0x2d, 0x0e, 0x03, 0xfe, 0x85, 0xd6, 0x80, 0x70, 0xb2, 0xfe, 0x33, 0xb4,
        0x91, 0x60, 0x59, 0x65, 0x8e, 0x28, 0xac, 0x7f, 0x7f, 0x91, 0xca, 0x4b, 0x12, 0x58, 0x40,
        0x9d, 0x6d, 0x91, 0x1e, 0x58, 0x8d, 0xd4, 0xfb, 0x77, 0xcb, 0x80, 0xc2, 0xc6, 0xad, 0xbc,
        0x2b, 0x94, 0x2b, 0xce, 0xa5, 0xd8, 0xa0, 0x39, 0x22, 0x0d, 0xdc, 0xd2, 0x35, 0xcb, 0x75,
        0x86, 0x2c, 0x0c, 0x95, 0xf6, 0x2b, 0xa1, 0x11, 0xe5, 0x7d, 0x7c, 0x1a, 0x22, 0x1c, 0xf5,
        0x13, 0x3e, 0x44, 0x12, 0x88, 0x32, 0xc1, 0x49, 0x35, 0x4d, 0x1e, 0x57, 0xb6, 0x80, 0xfe,
        0x57, 0x2d, 0x76, 0x0c,
    ];

    const TX_AUX: &'static [u8] = &[
        0x82, 0x83, 0x9f, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a,
        0x13, 0xb5, 0x0a, 0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6,
        0x7b, 0xbb, 0x3f, 0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19,
        0x02, 0x9a, 0xff, 0x9f, 0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee,
        0xa1, 0xb5, 0xec, 0x8e, 0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45,
        0xfd, 0x6d, 0x7b, 0x9e, 0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45,
        0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a, 0xff, 0xa0,
        0x81, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x85, 0x82, 0x58, 0x40, 0x1c, 0x0c, 0x3a, 0xe1, 0x82,
        0x5e, 0x90, 0xb6, 0xdd, 0xda, 0x3f, 0x40, 0xa1, 0x22, 0xc0, 0x07, 0xe1, 0x00, 0x8e, 0x83,
        0xb2, 0xe1, 0x02, 0xc1, 0x42, 0xba, 0xef, 0xb7, 0x21, 0xd7, 0x2c, 0x1a, 0x5d, 0x36, 0x61,
        0xde, 0xb9, 0x06, 0x4f, 0x2d, 0x0e, 0x03, 0xfe, 0x85, 0xd6, 0x80, 0x70, 0xb2, 0xfe, 0x33,
        0xb4, 0x91, 0x60, 0x59, 0x65, 0x8e, 0x28, 0xac, 0x7f, 0x7f, 0x91, 0xca, 0x4b, 0x12, 0x58,
        0x40, 0x9d, 0x6d, 0x91, 0x1e, 0x58, 0x8d, 0xd4, 0xfb, 0x77, 0xcb, 0x80, 0xc2, 0xc6, 0xad,
        0xbc, 0x2b, 0x94, 0x2b, 0xce, 0xa5, 0xd8, 0xa0, 0x39, 0x22, 0x0d, 0xdc, 0xd2, 0x35, 0xcb,
        0x75, 0x86, 0x2c, 0x0c, 0x95, 0xf6, 0x2b, 0xa1, 0x11, 0xe5, 0x7d, 0x7c, 0x1a, 0x22, 0x1c,
        0xf5, 0x13, 0x3e, 0x44, 0x12, 0x88, 0x32, 0xc1, 0x49, 0x35, 0x4d, 0x1e, 0x57, 0xb6, 0x80,
        0xfe, 0x57, 0x2d, 0x76, 0x0c,
    ];

    #[test]
    fn txout_decode() {
        // let txout : TxOut = cbor::decode_from_cbor(TX_OUT).unwrap();
        let mut raw = Deserializer::from(std::io::Cursor::new(TX_OUT));
        let txout: TxOut = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();

        let hdap = hdpayload::HDAddressPayload::from_bytes(HDPAYLOAD);
        assert_eq!(Coin::new(42).unwrap(), txout.value);
        assert_eq!(address::AddrType::ATPubKey, txout.address.addr_type);
        assert_eq!(
            address::StakeDistribution::new_bootstrap_era(),
            txout.address.attributes.stake_distribution
        );
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
        let attrs = address::Attributes::new_single_key(&pk, Some(hdap), NetworkMagic::NoMagic);

        let ea = address::ExtendedAddr::new(addr_type, sd, attrs);
        let value = Coin::new(42).unwrap();
        let txout = TxOut::new(ea, value);

        assert!(cbor_event::test_encode_decode(&txout).expect("encode/decode TxOut"));
    }

    #[test]
    fn txin_decode() {
        let mut raw = Deserializer::from(std::io::Cursor::new(TX_IN));
        let txo: TxoPointer = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();

        assert!(txo.index == 666);
    }

    #[test]
    fn txin_encode_decode() {
        let txid = TxId::new(&[0; 32]);
        assert!(cbor_event::test_encode_decode(&TxoPointer::new(txid, 666)).unwrap());
    }

    #[test]
    fn tx_decode() {
        let mut raw = Deserializer::from(std::io::Cursor::new(TX_IN));
        let txo: TxoPointer = raw.deserialize().unwrap();
        let mut raw = Deserializer::from(std::io::Cursor::new(TX_OUT));
        let txout: TxOut = raw.deserialize().unwrap();
        let mut raw = Deserializer::from(std::io::Cursor::new(TX));
        let mut tx: Tx = raw.deserialize().unwrap();

        assert!(tx.inputs.len() == 1);
        assert_eq!(Some(txo), tx.inputs.pop());
        assert!(tx.outputs.len() == 1);
        assert_eq!(Some(txout), tx.outputs.pop());
    }

    #[test]
    fn tx_encode_decode() {
        let txid = TxId::new(&[0; 32]);
        let txo = TxoPointer::new(txid, 666);

        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let hdap = hdpayload::HDAddressPayload::from_bytes(HDPAYLOAD);
        let addr_type = address::AddrType::ATPubKey;
        let sd = address::SpendingData::PubKeyASD(pk.clone());
        let attrs = address::Attributes::new_single_key(&pk, Some(hdap), NetworkMagic::NoMagic);
        let ea = address::ExtendedAddr::new(addr_type, sd, attrs);
        let value = Coin::new(42).unwrap();
        let txout = TxOut::new(ea, value);

        let mut tx = Tx::new();
        tx.add_input(txo);
        tx.add_output(txout);

        assert!(cbor_event::test_encode_decode(&tx).expect("encode/decode Tx"));
    }

    #[test]
    fn txinwitness_decode() {
        let protocol_magic = ProtocolMagic::default();
        let mut raw = Deserializer::from(std::io::Cursor::new(TX));
        let tx: Tx = raw.deserialize().expect("to decode a `Tx`");
        let mut raw = Deserializer::from(std::io::Cursor::new(TX_IN_WITNESS));
        let txinwitness: TxInWitness = raw.deserialize().expect("TxInWitness");

        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);

        assert_eq!(
            txinwitness,
            TxInWitness::new_extended_pk(protocol_magic, &sk, &tx.id())
        );
    }

    #[test]
    fn txinwitness_encode_decode() {
        let protocol_magic = ProtocolMagic::default();
        let mut raw = Deserializer::from(std::io::Cursor::new(TX));
        let tx: Tx = raw.deserialize().expect("to decode a `Tx`");

        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);

        let txinwitness = TxInWitness::new_extended_pk(protocol_magic, &sk, &tx.id());

        assert!(cbor_event::test_encode_decode(&txinwitness).expect("encode/decode TxInWitness"));
    }

    #[test]
    fn txinwitness_sign_verify() {
        let protocol_magic = ProtocolMagic::default();
        // create wallet's keys
        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        // create an Address
        let hdap = hdpayload::HDAddressPayload::from_bytes(HDPAYLOAD);
        let addr_type = address::AddrType::ATPubKey;
        let sd = address::SpendingData::PubKeyASD(pk.clone());
        let attrs = address::Attributes::new_single_key(&pk, Some(hdap), protocol_magic.into());
        let ea = address::ExtendedAddr::new(addr_type, sd, attrs);

        // create a transaction
        let txid = TxId::new(&[0; 32]);
        let txo = TxoPointer::new(txid, 666);
        let value = Coin::new(42).unwrap();
        let txout = TxOut::new(ea.clone(), value);
        let mut tx = Tx::new();
        tx.add_input(txo);
        tx.add_output(txout);

        // here we pretend that `ea` is the address we find from the found we want
        // to take. In the testing case, it is not important that it is also the
        // txout of this given transation

        // create a TxInWitness (i.e. sign the given transaction)
        let txinwitness = TxInWitness::new_extended_pk(protocol_magic, &sk, &tx.id());

        // check the address is the correct one
        assert!(txinwitness.verify_address(&ea));
        assert!(txinwitness.verify_tx(protocol_magic, &tx));
        assert!(txinwitness.verify(protocol_magic, &ea, &tx));
    }

    #[test]
    fn txaux_decode() {
        let mut raw = Deserializer::from(std::io::Cursor::new(TX_AUX));
        let _txaux: TxAux = raw.deserialize().expect("to decode a TxAux");
        let mut raw = Deserializer::from(std::io::Cursor::new(TX_AUX));
        let _txaux: TxAux = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();
    }

    #[test]
    fn txaux_encode_decode() {
        let mut raw = Deserializer::from(std::io::Cursor::new(TX));
        let tx: Tx = raw.deserialize().expect("to decode a `Tx`");
        let mut raw = Deserializer::from(std::io::Cursor::new(TX_IN_WITNESS));
        let txinwitness: TxInWitness = raw.deserialize().expect("to decode a `TxInWitness`");

        let txaux = TxAux::new(tx, TxWitness::from(vec![txinwitness]));

        assert!(cbor_event::test_encode_decode(&txaux).expect("encode/decode TxAux"));
    }
}

#[cfg(feature = "with-bench")]
#[cfg(test)]
mod bench {
    use super::*;
    use cbor_event::de::RawCbor;
    use test;

    const TX_AUX: &'static [u8] = &[
        0x82, 0x83, 0x9f, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a,
        0x13, 0xb5, 0x0a, 0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6,
        0x7b, 0xbb, 0x3f, 0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19,
        0x02, 0x9a, 0xff, 0x9f, 0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee,
        0xa1, 0xb5, 0xec, 0x8e, 0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45,
        0xfd, 0x6d, 0x7b, 0x9e, 0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45,
        0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a, 0xff, 0xa0,
        0x81, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x85, 0x82, 0x58, 0x40, 0x1c, 0x0c, 0x3a, 0xe1, 0x82,
        0x5e, 0x90, 0xb6, 0xdd, 0xda, 0x3f, 0x40, 0xa1, 0x22, 0xc0, 0x07, 0xe1, 0x00, 0x8e, 0x83,
        0xb2, 0xe1, 0x02, 0xc1, 0x42, 0xba, 0xef, 0xb7, 0x21, 0xd7, 0x2c, 0x1a, 0x5d, 0x36, 0x61,
        0xde, 0xb9, 0x06, 0x4f, 0x2d, 0x0e, 0x03, 0xfe, 0x85, 0xd6, 0x80, 0x70, 0xb2, 0xfe, 0x33,
        0xb4, 0x91, 0x60, 0x59, 0x65, 0x8e, 0x28, 0xac, 0x7f, 0x7f, 0x91, 0xca, 0x4b, 0x12, 0x58,
        0x40, 0x9d, 0x6d, 0x91, 0x1e, 0x58, 0x8d, 0xd4, 0xfb, 0x77, 0xcb, 0x80, 0xc2, 0xc6, 0xad,
        0xbc, 0x2b, 0x94, 0x2b, 0xce, 0xa5, 0xd8, 0xa0, 0x39, 0x22, 0x0d, 0xdc, 0xd2, 0x35, 0xcb,
        0x75, 0x86, 0x2c, 0x0c, 0x95, 0xf6, 0x2b, 0xa1, 0x11, 0xe5, 0x7d, 0x7c, 0x1a, 0x22, 0x1c,
        0xf5, 0x13, 0x3e, 0x44, 0x12, 0x88, 0x32, 0xc1, 0x49, 0x35, 0x4d, 0x1e, 0x57, 0xb6, 0x80,
        0xfe, 0x57, 0x2d, 0x76, 0x0c,
    ];

    #[bench]
    fn encode_txaux_cbor_raw(b: &mut test::Bencher) {
        let mut raw = cbor_event::de::RawCbor::from(TX_AUX);
        let txaux: TxAux = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();
        b.iter(|| {
            let _ = cbor!(txaux).unwrap();
        })
    }
    #[bench]
    fn decode_txaux_cbor_raw(b: &mut test::Bencher) {
        b.iter(|| {
            let _: TxAux = RawCbor::from(TX_AUX).deserialize().unwrap();
        })
    }
}
