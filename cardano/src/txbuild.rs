//! Transaction Builder
//!
//! Simple way to build transaction step-by-step,
//! with some complicated parts being abstracted
//! in a simple api (add_output_policy).
//!
//! This also exposes generally raw API, which allow
//! total flexibility and abstraction/helpers.
//!

use coin::{Coin, CoinDiff};
use fee::{Fee, FeeAlgorithm};
use std::iter::Iterator;
use std::{error, fmt, iter, result};
use tx::{txaux_serialize_size, Tx, TxAux, TxInWitness, TxOut, TxWitness, TxoPointer};
use txutils::OutputPolicy;
use {coin, fee};

/// Transaction Builder composed of inputs, outputs
#[derive(Clone)]
pub struct TxBuilder {
    inputs: Vec<(TxoPointer, Coin)>,
    outputs: Vec<TxOut>,
}

#[derive(Debug)]
pub enum Error {
    TxInvalidNoInput,
    TxInvalidNoOutput,
    TxNotEnoughTotalInput,
    TxOverLimit(usize),
    /// this return as by-product the amount of spare coins left behind
    TxOutputPolicyNotEnoughCoins(Coin),
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
            Error::TxNotEnoughTotalInput => {
                write!(f, "Transaction is invalid, already not enough input coins.")
            }
            Error::TxOutputPolicyNotEnoughCoins(coins) => write!(
                f,
                "Output policy cannot be added, only {} currently leftover",
                coins
            ),
            Error::TxOverLimit(sz) => write!(
                f,
                "Transaction too big, current size is {} bytes but limit size is {}.",
                sz, TX_SIZE_LIMIT
            ),
            Error::TxSignaturesExceeded => write!(f, "Transaction has already enough signatures"),
            Error::TxSignaturesMismatch => write!(
                f,
                "Number of signatures does not match the number of witnesses"
            ),
            Error::CoinError(_) => write!(f, "Error while performing value operation"),
            Error::FeeError(_) => write!(f, "Error while performing fee operation"),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::CoinError(ref err) => Some(err),
            Error::FeeError(ref err) => Some(err),
            _ => None,
        }
    }
}

// TODO might be a network configurable value..
const TX_SIZE_LIMIT: usize = 65536;

pub type Result<T> = result::Result<T, Error>;

impl From<coin::Error> for Error {
    fn from(e: coin::Error) -> Error {
        Error::CoinError(e)
    }
}
impl From<fee::Error> for Error {
    fn from(e: fee::Error) -> Error {
        Error::FeeError(e)
    }
}

impl TxBuilder {
    /// Create a new empty transaction builder
    pub fn new() -> Self {
        TxBuilder {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Return the number of inputs in this builder
    pub fn number_inputs(&self) -> usize {
        self.inputs.len()
    }

    /// Add an input in a form of a txo pointer to the current state.
    ///
    /// Note that for calculation purpose, we need to know the
    /// associated value with the input, which is not available
    /// from the txo pointer structure.
    pub fn add_input(&mut self, iptr: &TxoPointer, ivalue: Coin) {
        self.inputs.push((iptr.clone(), ivalue))
    }

    /// Add an output (address + coin value) to the current state
    pub fn add_output_value(&mut self, o: &TxOut) {
        self.outputs.push(o.clone())
    }

    fn apply_policy_with(&mut self, output_policy: &OutputPolicy, leftover: Coin) -> Vec<TxOut> {
        match output_policy {
            OutputPolicy::One(change_addr) => {
                let txout = TxOut::new(change_addr.clone(), leftover);
                self.add_output_value(&txout);
                vec![txout]
            }
        }
    }

    /// This associate all the leftover values, if any to specific outputs decided by the output policy.
    ///
    /// It returns as side effect the addresses and values that have been created and added to the
    /// transaction building as the result of the calculation.
    ///
    /// If the transaction is already consuming all inputs in its outputs (perfectly balanced),
    /// then an empty array is returned.
    ///
    /// If there's not enough inputs value compared to the existing outputs, then TxNotEnoughTotalInput is returned
    /// If there's no way to "fit" the output policy in the transaction building, as the fee cannot cover
    /// the basic overhead, then TxOutputPoliyNotEnoughCoins is returned with the amount of leftover coins.
    ///
    /// Note: that the calculation is not done again if more inputs and outputs are added after this call,
    /// and in most typical cases this should be the last addition to the transaction.
    pub fn add_output_policy<'a, F: FeeAlgorithm>(
        &mut self,
        f: &'a F,
        o: &OutputPolicy,
    ) -> Result<Vec<TxOut>> {
        // first check if there's any output, or not enough coins to cover
        match self.balance(f)? {
            CoinDiff::Zero => return Ok(Vec::new()),
            CoinDiff::Negative(_) => return Err(Error::TxNotEnoughTotalInput),
            CoinDiff::Positive(max) => {
                // One possible situation is that the amount of extra coins is less
                // to the cost of applying the output policy is; first we try if assigning
                // the minimum amount of coin is actually possible, and if not
                // bail now.
                let start = {
                    let mut temp = self.clone();
                    let _ = temp.apply_policy_with(o, Coin::unit()); // 0 and 1 has roughly the same overhead
                    match temp.balance(f)? {
                        CoinDiff::Positive(v) => v,
                        CoinDiff::Negative(_) => {
                            return Err(Error::TxOutputPolicyNotEnoughCoins(max));
                        }
                        CoinDiff::Zero => Coin::unit(),
                    }
                };

                // now start looking for a perfect match, starting at the value 'max'
                // being the maximum that can be paid, considering
                // that the actual value is closer to max - cost(output_policy)
                let mut out_total_max = max;
                let mut out_total = start;
                loop {
                    let mut temp = self.clone();

                    let outs = temp.apply_policy_with(o, out_total);

                    // check the balance of output with the above output policy in place
                    match temp.balance(f)? {
                        // Found a perfect match zero, then update policy and finish.
                        CoinDiff::Zero => {
                            self.apply_policy_with(o, out_total);
                            return Ok(outs);
                        }
                        // Input > Output+Fees. Effectively paying too much into fees
                        // need to assign more to out_total
                        CoinDiff::Positive(_x) => {
                            let out_total_min = out_total;
                            if (out_total_min + Coin::unit())? == out_total_max {
                                self.apply_policy_with(o, out_total);
                                return Ok(outs);
                            }
                            out_total = (out_total + Coin::unit())?;
                        }
                        // Input < Output+Fees.
                        // need to assign less to out_total
                        CoinDiff::Negative(x) => {
                            out_total_max = out_total;
                            if x > Coin::unit() {
                                out_total = (out_total - (x - Coin::unit())?)?
                            } else {
                                out_total = (out_total - Coin::unit())?
                            }
                        }
                    }
                }
            }
        }
    }

    /// Calculate the Fee that *need* to be paid for the current state of the builder.alloc
    ///
    /// For the LinearFee, it is related to the number of bytes that the representant
    /// txaux serialize to, but different algorithms can evaluate different criterions.
    pub fn calculate_fee<'a, F: FeeAlgorithm>(&self, f: &'a F) -> Result<Fee> {
        let tx = self.clone().make_tx_nocheck();
        let fake_witnesses = iter::repeat(TxInWitness::fake())
            .take(self.inputs.len())
            .collect();
        let fee = f.calculate_for_txaux_component(&tx, &fake_witnesses)?;
        Ok(fee)
    }

    /// get the total of input coins
    pub fn get_input_total(&self) -> Result<Coin> {
        let total = self
            .inputs
            .iter()
            .fold(Coin::new(0), |acc, ref c| acc.and_then(|v| v + c.1))?;
        Ok(total)
    }

    /// get the total of output coins
    pub fn get_output_total(&self) -> Result<Coin> {
        let total = self
            .outputs
            .iter()
            .fold(Coin::new(0), |acc, ref c| acc.and_then(|v| v + c.value))?;
        Ok(total)
    }

    /// Try to return the differential between the outputs (including fees) and the inputs
    /// * Zero: we have a balanced transaction where inputs === outputs
    /// * Negative: (outputs+fees) > inputs. More inputs required.
    /// * Positive: inputs > (outputs+fees). Excessive input goes to larger fee.
    pub fn balance<'a, F: FeeAlgorithm>(&self, f: &'a F) -> Result<CoinDiff> {
        let fee = self.calculate_fee(f)?;
        let inputs = self.get_input_total()?;
        let outputs = self.get_output_total()?;
        let outputs_fees = (outputs + fee.to_coin())?;
        Ok(inputs.differential(outputs_fees))
    }

    /// Same as balance(), but don't include fees in the outputs
    pub fn balance_without_fees(&self) -> Result<CoinDiff> {
        let inputs = self.get_input_total()?;
        let outputs = self.get_output_total()?;
        Ok(inputs.differential(outputs))
    }

    fn make_tx_nocheck(self) -> Tx {
        let inputs = self.inputs.iter().map(|(v, _)| v.clone()).collect();
        Tx::new_with(inputs, self.outputs)
    }

    pub fn make_tx(self) -> Result<Tx> {
        if self.inputs.len() == 0 {
            return Err(Error::TxInvalidNoInput);
        }
        if self.outputs.len() == 0 {
            return Err(Error::TxInvalidNoOutput);
        }
        Ok(self.make_tx_nocheck())
    }
}

/// Transaction finalized
#[derive(Clone)]
pub struct TxFinalized {
    tx: Tx,
    witnesses: TxWitness,
}

impl TxFinalized {
    /// Take a transaction and create a working area for adding witnesses
    pub fn new(tx: Tx) -> Self {
        TxFinalized {
            tx: tx,
            witnesses: TxWitness::new(),
        }
    }

    /// Add a witness associated with the next input.
    ///
    /// Witness need to be added in the same order to the inputs,
    /// otherwise protocol level mismatch will happen, and the
    /// transaction will be rejected
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
            return Err(Error::TxOverLimit(sz));
        }
        let txaux = TxAux::new(self.tx, self.witnesses);
        Ok(txaux)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use address::ExtendedAddr;
    use fee::LinearFee;
    use hash::Blake2b256;
    use tx::{TxId, TxOut};
    use util::{base58, try_from_slice::TryFromSlice};

    const RADDRS : [&str;3] =
        [ "DdzFFzCqrhsyhumccfGyEj3WZzztSPr92ntRWB6UVVwzcMTpwoafVQ5vD9mdZ5Xind8ycugbmA8esxmo7NycjQFGSbDeKrxabTz8MVzf"
        , "DdzFFzCqrhsi8XFMabbnHecVusaebqQCkXTqDnCumx5esKB1pk1zbhX5BtdAivZbQePFVujgzNCpBVXactPSmphuHRC5Xk8qmBd49QjW"
        , "Ae2tdPwUPEZKmwoy3AU3cXb5Chnasj6mvVNxV1H11997q3VW5ihbSfQwGpm"
        ];

    fn decode_addr(addr_str: &str) -> ExtendedAddr {
        let bytes = base58::decode(addr_str).unwrap();
        ExtendedAddr::try_from_slice(&bytes).unwrap()
    }

    // create a new builder with inputs and outputs
    fn build_input_outputs(inputs: &[(TxoPointer, Coin)], outputs: &[TxOut]) -> TxBuilder {
        let mut builder = TxBuilder::new();
        for (i, value) in inputs {
            builder.add_input(i, *value);
        }
        for o in outputs {
            builder.add_output_value(o);
        }
        builder
    }

    // create a txaux with fake witnesses from a txbuilder
    fn build_finalize(builder: TxBuilder) -> Result<TxAux> {
        let nb_inputs = builder.number_inputs();
        let tx = builder.make_tx()?;
        let mut finalizer = TxFinalized::new(tx);
        for _ in [0..nb_inputs].iter() {
            finalizer.add_witness(TxInWitness::fake())?
        }
        finalizer.make_txaux()
    }

    fn test_build(inputs: &[(TxoPointer, Coin)], outputs: &[TxOut]) -> Result<TxAux> {
        let builder = build_input_outputs(inputs, outputs);
        build_finalize(builder)
    }

    fn fee_is_minimal(coindiff: CoinDiff) {
        match coindiff {
            CoinDiff::Zero => {}
            CoinDiff::Positive(c) => assert_eq!(c, 1u32.into(), "fee is positive {}", c),
            CoinDiff::Negative(c) => {
                assert!(false, "fee is negative {}, expecting zero or positive", c)
            }
        }
    }

    fn fee_is_acceptable(coindiff: CoinDiff) {
        match coindiff {
            CoinDiff::Zero => {}
            CoinDiff::Positive(c) => {
                let max_fee_overhead = 5_000u32.into();
                assert!(
                    c < max_fee_overhead,
                    "fee is much greater than expected {}, expected less than {}",
                    c,
                    max_fee_overhead
                );
            }
            CoinDiff::Negative(c) => {
                assert!(false, "fee is negative {}, expecting zero or positive", c)
            }
        }
    }

    fn fake_id() -> TxId {
        Blake2b256::new(&[1, 2])
    }
    fn fake_txopointer_val(coin: Coin) -> (TxoPointer, Coin) {
        (TxoPointer::new(fake_id(), 1), coin)
    }

    #[test]
    fn txbuild_simple() {
        let inputs = vec![fake_txopointer_val(100000u32.into())];
        let outputs = vec![TxOut::new(decode_addr(RADDRS[1]), 8000u32.into())];
        let res = test_build(&inputs[..], &outputs[..]);
        assert!(res.is_ok())
    }

    #[test]
    fn txbuild_auto() {
        let inputs = vec![fake_txopointer_val(300000u32.into())];
        let alg = LinearFee::default();
        let out_policy = OutputPolicy::One(decode_addr(RADDRS[2]));
        for out_value in [8000u32.into(), 12004u32.into(), 51235u32.into()].iter() {
            let outputs = vec![TxOut::new(decode_addr(RADDRS[1]), *out_value)];
            let mut builder = build_input_outputs(&inputs[..], &outputs[..]);
            builder.add_output_policy(&alg, &out_policy).unwrap();

            fee_is_minimal(builder.balance(&alg).unwrap());
            assert!(build_finalize(builder).is_ok())
        }
    }

    #[test]
    fn txbuild_auto_2() {
        let inputs = vec![fake_txopointer_val(1_000_000u32.into())];
        let alg = LinearFee::default();
        let out_policy = OutputPolicy::One(decode_addr(RADDRS[2]));
        let out_policy_length_expected = |x: Vec<TxOut>| x.len() == 1;
        for out_value in [831_999u32.into()].iter() {
            let outputs = vec![TxOut::new(decode_addr(RADDRS[1]), *out_value)];
            let mut builder = build_input_outputs(&inputs[..], &outputs[..]);
            match builder.add_output_policy(&alg, &out_policy) {
                Ok(x) => {
                    assert!(out_policy_length_expected(x));
                    fee_is_minimal(builder.balance(&alg).unwrap())
                }
                Err(Error::TxOutputPolicyNotEnoughCoins(_c)) => {
                    // here we don't check that the fee is minimal, since we need to burn extra coins
                    fee_is_acceptable(builder.balance(&alg).unwrap())
                }
                Err(e) => panic!("{}", e),
            }

            assert!(build_finalize(builder).is_ok())
        }
    }
}
