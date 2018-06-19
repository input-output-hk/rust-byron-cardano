use std::{ops, iter, vec, slice, convert};
use tx::*;
use bip44;
use hdwallet;
use hdpayload;
use coin::{Coin};
use coin;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum TxInInfoAddr {
    Bip44(bip44::Addressing),
    Level2([hdwallet::DerivationIndex;2]),
}

/// This is a TxIn with extra data associated:
///
/// * The number of coin associated for this utxo
/// * Optionally, way to derive the address for this txin
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct TxInInfo {
    pub txin: TxIn,
    pub value: Coin,
    pub address_identified: Option<TxInInfoAddr>,
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
    pub addressing: bip44::Addressing,
}
impl Input {
    pub fn new(ptr: TxIn, value: TxOut, addressing: bip44::Addressing) -> Self
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

/// Collection of `Output` that will be used for creating a `Tx` and fee stabilisation
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
