use std::{fmt, result};
use coin;
use coin::{Coin};
use tx::{TxOut, Tx, TxAux};
use txutils::{Inputs, OutputPolicy, output_sum};

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

pub trait FeeAlgorithm {
    fn calculate_for_txaux(&self, txaux: &TxAux) -> Result<Fee>;
}

impl FeeAlgorithm for LinearFee {
    fn calculate_for_txaux(&self, txaux: &TxAux) -> Result<Fee> {
        let txbytes = cbor!(txaux).unwrap();
        self.estimate(txbytes.len())
    }
}

impl Default for LinearFee {
    fn default() -> Self { LinearFee::new(155381.0, 43.946) }
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
        if outputs.is_empty() { return Err(Error::NoOutputs); }

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
                    match output_policy {
                        OutputPolicy::One(change_addr) => tx.add_output(TxOut::new(change_addr.clone(), change_value)),
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
