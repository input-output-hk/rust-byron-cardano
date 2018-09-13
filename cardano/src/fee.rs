//! Fee calculation and fee algorithms

use std::{result, ops::{Add, Mul}};
use coin;
use coin::{Coin};
use tx::{Tx, TxInWitness, TxAux, txaux_serialize};
use cbor_event;

/// A fee value that represent either a fee to pay, or a fee paid.
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct Fee(Coin);
impl Fee {
    pub fn new(coin: Coin) -> Self { Fee(coin) }
    pub fn to_coin(&self) -> Coin { self.0 }
}

#[derive(Debug)]
pub enum Error {
    CoinError(coin::Error),
    CborError(cbor_event::Error),
}

pub type Result<T> = result::Result<T, Error>;

impl From<coin::Error> for Error {
    fn from(e: coin::Error) -> Error { Error::CoinError(e) }
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Error { Error::CborError(e) }
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Debug, Clone, Copy)]
pub struct Milli (pub u64);
impl Milli {
    pub fn new(i: u64, f: u64) -> Self { Milli(i * 1000 + f % 1000) }
    pub fn integral(i: u64) -> Self { Milli(i*1000) }
    pub fn to_integral(self) -> u64 {
        // note that we want the ceiling
        if self.0 % 1000 == 0 { self.0 / 1000 } else { (self.0 / 1000) + 1 }
    }
    pub fn to_integral_trunc(self) -> u64 { self.0 / 1000 }
}

impl Add for Milli {
    type Output = Milli;
    fn add(self, other: Self) -> Self {
        Milli(self.0 + other.0)
    }
}
impl Mul for Milli {
    type Output = Milli;
    fn mul(self, other: Self) -> Self {
        let v = self.0 as u128 * other.0 as u128;
        Milli((v / 1000) as u64)
        /*
        let ai = self.integral * other.integral;
        let af = self.floating * other.floating;
        let a1 = self.integral * NANO_MASK * other.floating;
        let a2 = self.floating * NANO_MASK * other.integral;
        Nano {
            integral: ai * NANO_MASK + af / NANO_MASK + a1 / NANO_MASK + a2 / NANO_MASK,
            floating: af % NANO_MASK + a1 % NANO_MASK + a2 % NANO_MASK,
        }
        */
    }
}

/// Linear fee using the basic affine formula `A * bytes(txaux) + CONSTANT`
#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Debug, Clone, Copy)]
pub struct LinearFee {
    /// this is the minimal fee
    constant: Milli,
    /// the transaction's size coefficient fee
    coefficient: Milli,
}
impl LinearFee {
    pub fn new(constant: Milli, coefficient: Milli) -> Self {
        LinearFee { constant: constant, coefficient: coefficient }
    }

    pub fn estimate(&self, sz: usize) -> Result<Fee> {
        let msz = Milli::integral(sz as u64);
        let fee = self.constant + self.coefficient * msz;
        let coin = Coin::new(fee.to_integral())?;
        Ok(Fee(coin))
    }
}

/// Calculation of fees for a specific chosen algorithm
pub trait FeeAlgorithm {
    fn calculate_for_txaux(&self, txaux: &TxAux) -> Result<Fee>;
    fn calculate_for_txaux_component(&self, tx: &Tx, witnesses: &Vec<TxInWitness>) -> Result<Fee>;
}

impl FeeAlgorithm for LinearFee {
    fn calculate_for_txaux(&self, txaux: &TxAux) -> Result<Fee> {
        // the only reason the cbor serialisation would fail is if there was
        // no more memory free to allocate.
        let txbytes = cbor!(txaux)?;
        self.estimate(txbytes.len())
    }
    fn calculate_for_txaux_component(&self, tx: &Tx, witnesses: &Vec<TxInWitness>) -> Result<Fee> {
        let ser = cbor_event::se::Serializer::new_vec();
        let txbytes = txaux_serialize(tx, witnesses, ser)?.finalize();
        self.estimate(txbytes.len())
    }
}

impl Default for LinearFee {
    fn default() -> Self { LinearFee::new(Milli::integral(155381), Milli::new(43,946)) }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_milli_add_eq(v1: u64, v2: u64) {
        let v = v1 + v2;
        let n1 = Milli::new(v1 / 1000, v1 % 1000);
        let n2 = Milli::new(v2 / 1000, v2 % 1000);
        let n = n1 + n2;
        assert_eq!(v / 1000, n.to_integral_trunc());
    }

    fn test_milli_mul_eq(v1: u64, v2: u64) {
        let v = v1 as u128 * v2 as u128;
        let n1 = Milli::new(v1 / 1000, v1 % 1000);
        let n2 = Milli::new(v2 / 1000, v2 % 1000);
        let n = n1 * n2;
        assert_eq!((v / 1000000) as u64, n.to_integral_trunc());
    }

    #[test]
    fn check_fee_add() {
        test_milli_add_eq(10124128_192, 802_504);
        test_milli_add_eq( 1124128_915, 124802_192);
        test_milli_add_eq(         241, 900001_901);
        test_milli_add_eq(         241,        407);
    }

    #[test]
    fn check_fee_mul() {
        test_milli_mul_eq(10124128_192, 802_192);
        test_milli_mul_eq( 1124128_192, 124802_192);
        test_milli_mul_eq(         241, 900001_900);
        test_milli_mul_eq(         241,        400);
    }
}
