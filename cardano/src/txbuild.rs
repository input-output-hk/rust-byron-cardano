//! Transaction Builder

use tx::{TxoPointer, TxOut, Tx, TxAux, TxWitness, TxInWitness};
use {coin,fee};
use coin::{Coin, CoinDiff};
use fee::{FeeAlgorithm, Fee};
use std::iter::Iterator;
use std::{result, iter};

/// Transaction Builder composed of inputs, outputs
#[derive(Clone)]
pub struct TxBuilder {
    inputs: Vec<(TxoPointer, Coin)>,
    outputs: Vec<TxOut>,
}

/// Balance during the transaction building process
pub enum BuildingBalance {
    Negative(u64),
    Exact,
    Positive(u64),
}

#[derive(Debug)]
pub enum Error {
    TxOverLimit(usize),
    TxSignaturesExceeded,
    TxSignaturesMismatch,
    CoinError(coin::Error),
    FeeError(fee::Error),
}

pub type Result<T> = result::Result<T, Error>;

impl From<coin::Error> for Error {
    fn from(e: coin::Error) -> Error { Error::CoinError(e) }
}
impl From<fee::Error> for Error {
    fn from(e: fee::Error) -> Error { Error::FeeError(e) }
}

impl TxBuilder {
    /// Create a new empty transaction builder
    pub fn new() -> Self {
        TxBuilder { inputs: Vec::new(), outputs: Vec::new() }
    }

    pub fn add_input(&mut self, iptr: &TxoPointer, ivalue: Coin) {
        self.inputs.push((iptr.clone(), ivalue))
    }

    pub fn add_output(&mut self, o: &TxOut) {
        self.outputs.push(o.clone())
    }

    pub fn calculate_fee<F: FeeAlgorithm>(&self, f: F) -> Result<Fee> {
        let tx = self.clone().make_tx();
        let fake_witnesses = iter::repeat(TxInWitness::fake()).take(self.inputs.len()).collect();
        let fee = f.calculate_for_txaux_component(&tx, &fake_witnesses)?;
        Ok(fee)
    }

    pub fn get_input_total(&self) -> Result<Coin> {
        let total = self.inputs.iter().fold(Coin::new(0), |acc, ref c| acc.and_then(|v| v + c.1))?;
        Ok(total)
    }

    pub fn get_output_total(&self) -> Result<Coin> {
        let total = self.outputs.iter().fold(Coin::new(0), |acc, ref c| acc.and_then(|v| v + c.value))?;
        Ok(total)
    }

    pub fn balance_without_fees(&self) -> Result<CoinDiff> {
        let inputs = self.get_input_total()?;
        let outputs = self.get_output_total()?;
        Ok(inputs.differential(outputs))
    }

    pub fn balance<F: FeeAlgorithm>(&self, f: F) -> Result<CoinDiff> {
        let fee = self.calculate_fee(f)?;
        let inputs = self.get_input_total()?;
        let outputs = self.get_output_total()?;
        let outputs_fees = (outputs + fee.to_coin())?;
        Ok(inputs.differential(outputs_fees))
    }

    pub fn make_tx(self) -> Tx {
        let inputs = self.inputs.iter().map(|(v, _)| v.clone()).collect();
        Tx::new_with(inputs, self.outputs)
    }
}

/// Transaction finalized
pub struct TxFinalized {
    tx: Tx,
    witnesses: TxWitness,
}

impl TxFinalized {
    pub fn new(tx: Tx) -> Self {
        TxFinalized { tx: tx, witnesses: TxWitness::new() }
    }

    pub fn add_witness(&mut self, witness: TxInWitness) -> Result<()> {
        if self.witnesses.len() >= self.tx.inputs.len() {
            return Err(Error::TxSignaturesExceeded);
        }
        self.witnesses.push(witness);
        Ok(())
    }

    pub fn make_txaux(self) -> Result<TxAux> {
        if self.witnesses.len() != self.tx.inputs.len() {
            return Err(Error::TxSignaturesMismatch);
        }
        Ok(TxAux::new(self.tx, self.witnesses))
    }
}