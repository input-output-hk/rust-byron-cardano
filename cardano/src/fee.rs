//! Fee calculation and fee algorithms

use std::{fmt, result, ops::{Add, Mul}};
use coin;
use coin::{Coin};
use tx::{TxOut, Tx, TxInWitness, TxAux, txaux_serialize};
use txutils::{Inputs, OutputPolicy, output_sum};
use cbor_event;

/// A fee value that represent either a fee to pay, or a fee paid.
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
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
    CoinError(coin::Error),
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

/// Algorithm trait for input selections
pub trait SelectionAlgorithm {
    /// This takes from input:
    /// * Selection Policy
    /// * The tx inputs with at minimum 1 entry
    /// * The tx outputs with at minimum 1 entry
    /// * Extended address of where to send the remain
    ///
    /// It returns on success:
    ///
    /// * The computed fee associated
    /// * The inputs selected
    /// * The number of coin remaining that will be associated to the extended address specified
    fn compute(&self, policy: SelectionPolicy, inputs: &Inputs, outputs: &Vec<TxOut>, change_addr: &OutputPolicy) -> Result<(Fee, Inputs, Coin)>;
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
        let txbytes = cbor!(txaux).unwrap();
        self.estimate(txbytes.len())
    }
    fn calculate_for_txaux_component(&self, tx: &Tx, witnesses: &Vec<TxInWitness>) -> Result<Fee> {
        let ser = cbor_event::se::Serializer::new_vec();
        let txbytes = txaux_serialize(tx, witnesses, ser).unwrap().finalize();
        self.estimate(txbytes.len())
    }
}

impl Default for LinearFee {
    fn default() -> Self { LinearFee::new(Milli::integral(155381), Milli::new(43,946)) }
}

const TX_IN_WITNESS_CBOR_SIZE: usize = 140;
const CBOR_TXAUX_OVERHEAD: usize = 51;
impl SelectionAlgorithm for LinearFee {
    fn compute( &self
              , policy: SelectionPolicy
              , inputs: &Inputs
              , outputs: &Vec<TxOut>
              , output_policy: &OutputPolicy
              )
        -> Result<(Fee, Inputs, Coin)>
    {
        if inputs.is_empty() { return Err(Error::NoInputs); }

        let output_value = output_sum(outputs)?;
        let mut fee = self.estimate(0)?;
        let mut input_value = Coin::zero();
        let mut selected_inputs = Inputs::new();

        // create the Tx on the fly
        let mut txins = Vec::new();
        let     txouts : Vec<TxOut> = outputs.iter().cloned().collect();

        // for now we only support this selection algorithm
        // we need to remove this assert when we extend to more
        // granulated selection policy
        assert!(policy == SelectionPolicy::FirstMatchFirst);

        for input in inputs.iter() {
            input_value = (input_value + input.value())?;
            selected_inputs.push(input.clone());
            txins.push(input.ptr.clone());

            // calculate fee from the Tx serialised + estimated size for signing
            let mut tx = Tx::new_with(txins.clone(), txouts.clone());
            let txbytes = cbor!(&tx).unwrap();

            let estimated_fee = (self.estimate(txbytes.len() + CBOR_TXAUX_OVERHEAD + (TX_IN_WITNESS_CBOR_SIZE * selected_inputs.len())))?;

            // add the change in the estimated fee
            match output_value - input_value - estimated_fee.to_coin() {
                None => {},
                Some(change_value) => {
                    if change_value > Coin::zero() {
                        match output_policy {
                            OutputPolicy::One(change_addr) => tx.add_output(TxOut::new(change_addr.clone(), change_value)),
                        }
                    }
                }
            };

            let txbytes = cbor!(&tx).unwrap();
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
