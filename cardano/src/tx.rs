//! Transaction types
//!
//! `TxIn` : Input
//! `TxOut` : Output
//! `Tx` : Input + Output
//! `TxInWitness`: Witness providing for TxIn (e.g. cryptographic signature)
//! `TxAux` : Signed Tx (Tx + Witness)
//!
use std::{fmt};

use hash::{Blake2b256};

use cbor_event::{self, de::RawCbor, se::{Serializer}};
use config::{ProtocolMagic};
use redeem;
use tags::{SigningTag};

use hdwallet::{Signature, XPub, XPrv, XPUB_SIZE, SIGNATURE_SIZE};
use address::{ExtendedAddr, SpendingData};
use coin::{self, Coin};

// TODO: this seems to be the hash of the serialisation CBOR of a given Tx.
// if this is confirmed, we need to make a proper type, wrapping it around
// to hash a `Tx` by serializing it cbor first.
pub type TxId = Blake2b256;

/// Tx Output composed of an address and a coin value
#[derive(Debug, PartialEq, Eq, Clone)]
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
impl cbor_event::de::Deserialize for TxOut {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(2, "TxOut")?;
        let addr = cbor_event::de::Deserialize::deserialize(raw)?;
        let val  = cbor_event::de::Deserialize::deserialize(raw)?;
        Ok(TxOut::new(addr, val))
    }
}
impl cbor_event::se::Serialize for TxOut {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(2))?
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
        let fakesig = Signature::from_bytes([0u8;SIGNATURE_SIZE]);
        TxInWitness::PkWitness(XPub::from_bytes([0u8;XPUB_SIZE]), fakesig)
    }

    /// create a TxInWitness from a given private key `XPrv` for the given transaction id `TxId`.
    pub fn new(protocol_magic: ProtocolMagic, key: &XPrv, txid: &TxId) -> Self {
        let vec = Serializer::new_vec()
            .write_unsigned_integer(1).expect("write byte 0x01")
            .serialize(&protocol_magic).expect("serialize protocol magic")
            .serialize(&txid).expect("serialize Tx's Id")
            .finalize();
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
    pub fn verify_tx(&self, protocol_magic: ProtocolMagic, tx: &Tx) -> bool {
        let vec = Serializer::new_vec()
            .write_unsigned_integer(self.get_sign_tag() as u64).expect("write sign tag")
            .serialize(&protocol_magic).expect("serialize protocol magic")
            .serialize(&tx.id()).expect("serialize Tx's Id")
            .finalize();
        match self {
            &TxInWitness::PkWitness(ref pk, ref sig)     => pk.verify(&vec, sig),
            &TxInWitness::ScriptWitness(_, _)            => unimplemented!(),
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
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        let mut serializer = serializer.write_array(cbor_event::Len::Len(2))?;
        let inner_serializer = match self {
            &TxInWitness::PkWitness(ref xpub, ref signature) => {
                serializer = serializer.write_unsigned_integer(0)?;
                Serializer::new_vec()
                    .write_array(cbor_event::Len::Len(2))?
                        .serialize(xpub)?.serialize(signature)?
            },
            &TxInWitness::ScriptWitness(_, _) => { unimplemented!() },
            &TxInWitness::RedeemWitness(ref pk, ref signature) => {
                serializer = serializer.write_unsigned_integer(2)?;
                Serializer::new_vec()
                    .write_array(cbor_event::Len::Len(2))?
                        .serialize(pk)?.serialize(signature)?
            }
        };
        serializer.write_tag(24)?
                .write_bytes(&inner_serializer.finalize())
    }
}
impl cbor_event::de::Deserialize for TxInWitness {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(2, "TxInWitness")?;
        let sum_type_idx = raw.unsigned_integer()?;
        match sum_type_idx {
            0 => {
                let tag = raw.tag()?;
                if tag != 24 {
                    return Err(cbor_event::Error::CustomError(format!("Invalid Tag: {} but expected 24", tag)));
                }
                let bytes = raw.bytes()?;
                let mut raw = RawCbor::from(&bytes);
                raw.tuple(2, "TxInWitness::PkWitness")?;
                let pk  = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                let sig = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                Ok(TxInWitness::PkWitness(pk, sig))
            },
            2 => {
                let tag = raw.tag()?;
                if tag != 24 {
                    return Err(cbor_event::Error::CustomError(format!("Invalid Tag: {} but expected 24", tag)));
                }
                let bytes = raw.bytes()?;
                let mut raw = RawCbor::from(&bytes);
                raw.tuple(2, "TxInWitness::PkRedeemWitness")?;
                let pk  = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                let sig = cbor_event::de::Deserialize::deserialize(&mut raw)?;
                Ok(TxInWitness::RedeemWitness(pk, sig))
            },
            _ => {
                Err(cbor_event::Error::CustomError(format!("Unsupported TxInWitness: {}", sum_type_idx)))
            }
        }
    }
}

/// Structure used for addressing a specific output of a transaction
/// built from a TxId (hash of the tx) and the offset in the outputs of this
/// transaction.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
impl cbor_event::se::Serialize for TxIn {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(2))?
            .write_unsigned_integer(0)?
            .write_tag(24)?
                .write_bytes(&cbor!(&(&self.id, &self.index))?)
    }
}
impl cbor_event::de::Deserialize for TxIn {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(2, "TxIn")?;
        let sum_type_idx = raw.unsigned_integer()?;
        if sum_type_idx != 0 {
            return Err(cbor_event::Error::CustomError(format!("Unsupported TxIn: {}", sum_type_idx)));
        }
        let tag = raw.tag()?;
        if tag != 24 {
            return Err(cbor_event::Error::CustomError(format!("Invalid Tag: {} but expected 24", tag)));
        }
        let bytes = raw.bytes()?;
        let mut raw = RawCbor::from(&bytes);
        raw.tuple(2, "TxIn")?;
        let id  = cbor_event::de::Deserialize::deserialize(&mut raw)?;
        let idx = raw.unsigned_integer()?;
        Ok(TxIn::new(id, idx as u32))
    }
}

/// A Transaction containing tx inputs and tx outputs.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Tx {
    pub inputs: Vec<TxIn>,
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
    pub fn new() -> Self { Tx::new_with(Vec::new(), Vec::new()) }
    pub fn new_with(ins: Vec<TxIn>, outs: Vec<TxOut>) -> Self {
        Tx { inputs: ins, outputs: outs }
    }
    pub fn id(&self) -> TxId {
        let buf = cbor!(self).expect("encode Tx");
        TxId::new(&buf)
    }
    pub fn add_input(&mut self, i: TxIn) {
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
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        let serializer = serializer.write_array(cbor_event::Len::Len(3))?;
        let serializer = cbor_event::se::serialize_indefinite_array(self.inputs.iter(), serializer)?;
        let serializer = cbor_event::se::serialize_indefinite_array(self.outputs.iter(), serializer)?;
        serializer.write_map(cbor_event::Len::Len(0))
    }
}
impl cbor_event::de::Deserialize for Tx {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(3, "Tx")?;

        // Note: these must be indefinite-size arrays.
        let inputs = cbor_event::de::Deserialize::deserialize(raw)?;
        let outputs = cbor_event::de::Deserialize::deserialize(raw)?;

        let map_len = raw.map()?;
        if ! map_len.is_null() {
            return Err(cbor_event::Error::CustomError(format!("Invalid Tx: we do not support Tx extra data... {:?} elements", map_len)));
        }
        Ok(Tx::new_with(inputs, outputs))
    }
}

/// A transaction witness is a vector of input witnesses
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TxWitness(Vec<TxInWitness>);

impl TxWitness {
    pub fn new() -> Self { TxWitness(Vec::new()) }
}
impl From<Vec<TxInWitness>> for TxWitness {
    fn from(v: Vec<TxInWitness>) -> Self { TxWitness(v) }
}
impl ::std::iter::FromIterator<TxInWitness> for TxWitness {
    fn from_iter<I>(iter: I) -> Self
        where I: IntoIterator<Item = TxInWitness>
    {
        TxWitness(Vec::from_iter(iter))
    }
}
impl ::std::ops::Deref for TxWitness {
    type Target = Vec<TxInWitness>;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl ::std::ops::DerefMut for TxWitness {
    fn deref_mut(&mut self) -> &mut Vec<TxInWitness> { &mut self.0 }
}

impl cbor_event::de::Deserialize for TxWitness {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(TxWitness(cbor_event::de::Deserialize::deserialize(raw)?))
    }
}

impl cbor_event::se::Serialize for TxWitness {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        txwitness_serialize(&self.0, serializer)
    }
}

pub fn txwitness_serialize<W>(in_witnesses: &Vec<TxInWitness>, serializer: Serializer<W>)
    -> cbor_event::Result<Serializer<W>>
    where W: ::std::io::Write
{
    cbor_event::se::serialize_fixed_array(in_witnesses.iter(), serializer)
}

/// A transaction witness is a vector of input witnesses
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TxWitnesses {
    pub in_witnesses: Vec<TxWitness>
}

impl TxWitnesses {
    pub fn new(in_witnesses: Vec<TxWitness>) -> Self {
        TxWitnesses { in_witnesses: in_witnesses }
    }
}

impl ::std::ops::Deref for TxWitnesses {
    type Target = Vec<TxWitness>;
    fn deref(&self) -> &Self::Target { &self.in_witnesses }
}

impl cbor_event::se::Serialize for TxWitnesses
{
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        cbor_event::se::serialize_indefinite_array(self.iter(), serializer)
    }
}

/// Tx with the vector of witnesses
#[derive(Debug, PartialEq, Eq, Clone)]
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
        TxAux { tx: tx, witness: witness }
    }
}
impl cbor_event::de::Deserialize for TxAux {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(2, "TxAux")?;
        let tx = cbor_event::de::Deserialize::deserialize(raw)?;
        let witness = cbor_event::de::Deserialize::deserialize(raw)?;
        Ok(TxAux::new(tx, witness))
    }
}
impl cbor_event::se::Serialize for TxAux {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        txaux_serialize(&self.tx, &self.witness, serializer)
    }
}

pub fn txaux_serialize<W>(tx: &Tx, in_witnesses: &Vec<TxInWitness>, serializer: Serializer<W>)
    -> cbor_event::Result<Serializer<W>>
    where W: ::std::io::Write
{
    let serializer = serializer.write_array(cbor_event::Len::Len(2))?
                .serialize(tx)?;
    txwitness_serialize(in_witnesses, serializer)
}

pub fn txaux_serialize_size(tx: &Tx, in_witnesses: &Vec<TxInWitness>) -> usize {
    // TODO don't actually produce any bytes, but instead just count.
    // we don't expect error here, a real counter would not error..
    let ser = cbor_event::se::Serializer::new_vec();
    let bytes = txaux_serialize(tx, in_witnesses, ser).unwrap().finalize();
    bytes.len()
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
impl cbor_event::se::Serialize for TxProof {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(3))?
            .write_unsigned_integer(self.number as u64)?
            .serialize(&self.root)?
            .serialize(&self.witnesses_hash)
    }
}
impl cbor_event::de::Deserialize for TxProof {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.tuple(3, "TxProof")?;
        let number = raw.unsigned_integer()?;
        let root   = cbor_event::de::Deserialize::deserialize(raw)?;
        let witnesses = cbor_event::de::Deserialize::deserialize(raw)?;
        Ok(TxProof::new(number as u32, root, witnesses))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use address;
    use hdpayload;
    use hdwallet;
    use cbor_event::{self, de::RawCbor};

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
        // let txout : TxOut = cbor::decode_from_cbor(TX_OUT).unwrap();
        let mut raw = RawCbor::from(TX_OUT);
        let txout : TxOut = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();

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
        let txout = TxOut::new(ea, value);

        assert!(cbor_event::test_encode_decode(&txout).expect("encode/decode TxOut"));
    }

    #[test]
    fn txin_decode() {
        let mut raw = RawCbor::from(TX_IN);
        let txin : TxIn = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();

        assert!(txin.index == 666);
    }

    #[test]
    fn txin_encode_decode() {
        let txid = TxId::new(&[0;32]);
        assert!(cbor_event::test_encode_decode(&TxIn::new(txid, 666)).unwrap());
    }

    #[test]
    fn tx_decode() {
        let txin  : TxIn  = RawCbor::from(TX_IN).deserialize().unwrap();
        let txout : TxOut = RawCbor::from(TX_OUT).deserialize().unwrap();
        let mut tx : Tx   = RawCbor::from(TX).deserialize().unwrap();

        assert!(tx.inputs.len() == 1);
        assert_eq!(Some(txin), tx.inputs.pop());
        assert!(tx.outputs.len() == 1);
        assert_eq!(Some(txout), tx.outputs.pop());
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

        assert!(cbor_event::test_encode_decode(&tx).expect("encode/decode Tx"));
    }

    #[test]
    fn txinwitness_decode() {
        let protocol_magic = ProtocolMagic::default();
        let tx : Tx = RawCbor::from(TX).deserialize().expect("to decode a `Tx`");
        let txinwitness : TxInWitness = RawCbor::from(TX_IN_WITNESS).deserialize().expect("TxInWitness");

        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);

        assert_eq!(txinwitness, TxInWitness::new(protocol_magic, &sk, &tx.id()));
    }

    #[test]
    fn txinwitness_encode_decode() {
        let protocol_magic = ProtocolMagic::default();
        let tx : Tx = RawCbor::from(TX).deserialize().expect("to decode a `Tx`");

        let seed = hdwallet::Seed::from_bytes(SEED);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);

        let txinwitness = TxInWitness::new(protocol_magic, &sk, &tx.id());

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
        let txinwitness = TxInWitness::new(protocol_magic, &sk, &tx.id());

        // check the address is the correct one
        assert!(txinwitness.verify_address(&ea));
        assert!(txinwitness.verify_tx(protocol_magic, &tx));
        assert!(txinwitness.verify(protocol_magic, &ea, &tx));
    }

    #[test]
    fn txaux_decode() {
        let _txaux : TxAux = RawCbor::from(TX_AUX).deserialize().expect("to decode a TxAux");
        let mut raw = RawCbor::from(TX_AUX);
        let _txaux : TxAux = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();
    }

    #[test]
    fn txaux_encode_decode() {
        let tx : Tx = RawCbor::from(TX).deserialize().expect("to decode a `Tx`");
        let txinwitness : TxInWitness = RawCbor::from(TX_IN_WITNESS).deserialize().expect("to decode a `TxInWitness`");

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

    const TX_AUX : &'static [u8] = &[0x82, 0x83, 0x9f, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x26, 0x82, 0x58, 0x20, 0xaa, 0xd7, 0x8a, 0x13, 0xb5, 0x0a, 0x01, 0x4a, 0x24, 0x63, 0x3c, 0x7d, 0x44, 0xfd, 0x8f, 0x8d, 0x18, 0xf6, 0x7b, 0xbb, 0x3f, 0xa9, 0xcb, 0xce, 0xdf, 0x83, 0x4a, 0xc8, 0x99, 0x75, 0x9d, 0xcd, 0x19, 0x02, 0x9a, 0xff, 0x9f, 0x82, 0x82, 0xd8, 0x18, 0x58, 0x29, 0x83, 0x58, 0x1c, 0x83, 0xee, 0xa1, 0xb5, 0xec, 0x8e, 0x80, 0x26, 0x65, 0x81, 0x46, 0x4a, 0xee, 0x0e, 0x2d, 0x6a, 0x45, 0xfd, 0x6d, 0x7b, 0x9e, 0x1a, 0x98, 0x3a, 0x50, 0x48, 0xcd, 0x15, 0xa1, 0x01, 0x46, 0x45, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x1a, 0x9d, 0x45, 0x88, 0x4a, 0x18, 0x2a, 0xff, 0xa0, 0x81, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x85, 0x82, 0x58, 0x40, 0x1c, 0x0c, 0x3a, 0xe1, 0x82, 0x5e, 0x90, 0xb6, 0xdd, 0xda, 0x3f, 0x40, 0xa1, 0x22, 0xc0, 0x07, 0xe1, 0x00, 0x8e, 0x83, 0xb2, 0xe1, 0x02, 0xc1, 0x42, 0xba, 0xef, 0xb7, 0x21, 0xd7, 0x2c, 0x1a, 0x5d, 0x36, 0x61, 0xde, 0xb9, 0x06, 0x4f, 0x2d, 0x0e, 0x03, 0xfe, 0x85, 0xd6, 0x80, 0x70, 0xb2, 0xfe, 0x33, 0xb4, 0x91, 0x60, 0x59, 0x65, 0x8e, 0x28, 0xac, 0x7f, 0x7f, 0x91, 0xca, 0x4b, 0x12, 0x58, 0x40, 0x9d, 0x6d, 0x91, 0x1e, 0x58, 0x8d, 0xd4, 0xfb, 0x77, 0xcb, 0x80, 0xc2, 0xc6, 0xad, 0xbc, 0x2b, 0x94, 0x2b, 0xce, 0xa5, 0xd8, 0xa0, 0x39, 0x22, 0x0d, 0xdc, 0xd2, 0x35, 0xcb, 0x75, 0x86, 0x2c, 0x0c, 0x95, 0xf6, 0x2b, 0xa1, 0x11, 0xe5, 0x7d, 0x7c, 0x1a, 0x22, 0x1c, 0xf5, 0x13, 0x3e, 0x44, 0x12, 0x88, 0x32, 0xc1, 0x49, 0x35, 0x4d, 0x1e, 0x57, 0xb6, 0x80, 0xfe, 0x57, 0x2d, 0x76, 0x0c];

    #[bench]
    fn encode_txaux_cbor_raw(b: &mut test::Bencher) {
        let mut raw = cbor_event::de::RawCbor::from(TX_AUX);
        let txaux : TxAux = cbor_event::de::Deserialize::deserialize(&mut raw).unwrap();
        b.iter(|| {
            let _ = cbor!(txaux).unwrap();
        })
    }
    #[bench]
    fn decode_txaux_cbor_raw(b: &mut test::Bencher) {
        b.iter(|| {
            let _ : TxAux = RawCbor::from(TX_AUX).deserialize().unwrap();
        })
    }
}
