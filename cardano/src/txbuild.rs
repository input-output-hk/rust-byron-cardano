//! Transaction Builder

use tx::{TxoPointer, TxOut, Tx, TxAux, TxWitness, TxInWitness, txaux_serialize_size};
use {coin,fee};
use coin::{Coin, CoinDiff};
use fee::{FeeAlgorithm, Fee};
use std::iter::Iterator;
use std::{result, iter, fmt, error};

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
    TxInvalidNoInput,
    TxInvalidNoOutput,
    TxOverLimit(usize),
    TxSignaturesExceeded,
    TxSignaturesMismatch,
    CoinError(coin::Error),
    FeeError(fee::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::TxInvalidNoInput => write!(f, "Transaction is invalid, no input."),
            Error::TxInvalidNoOutput => write!(f, "Transaction is invalid, no output."),
            Error::TxOverLimit(sz) => write!(f, "Transaction too big, current size is {} bytes but limit size is {}.", sz, TX_SIZE_LIMIT),
            Error::TxSignaturesExceeded => write!(f, "Transaction has already enough signatures"),
            Error::TxSignaturesMismatch => write!(f, "Number of signatures does not match the number of witnesses"),
            Error::CoinError(_) => write!(f, "Error while performing value operation"),
            Error::FeeError(_)  => write!(f, "Error while performing fee operation")
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::CoinError(ref err) => Some(err),
            Error::FeeError(ref err)  => Some(err),
            _ => None
        }
    }
}

// TODO might be a network configurable value..
const TX_SIZE_LIMIT : usize = 65536;

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
        let tx = self.clone().make_tx_nocheck();
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

    fn make_tx_nocheck(self) -> Tx {
        let inputs = self.inputs.iter().map(|(v, _)| v.clone()).collect();
        Tx::new_with(inputs, self.outputs)
    }

    pub fn make_tx(self) -> Result<Tx> {
        if self.inputs.len() == 0 {
            return Err(Error::TxInvalidNoInput)
        }
        if self.outputs.len() == 0 {
            return Err(Error::TxInvalidNoOutput)
        }
        Ok(self.make_tx_nocheck())
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
        let sz = txaux_serialize_size(&self.tx, &(*self.witnesses));
        if sz > TX_SIZE_LIMIT {
            return Err(Error::TxOverLimit(sz))
        }
        let txaux = TxAux::new(self.tx, self.witnesses);
        Ok(txaux)
    }
}
