use std::{fmt, result};
use coin::{self, Coin};
use tx::{TxOut, Tx};
use txutils::{Input, OutputPolicy, output_sum};
use cbor_event;
use fee::{self, Fee, LinearFee};

#[derive(Debug)]
pub enum Error {
    NoInputs,
    NoOutputs,
    NotEnoughInput,
    CoinError(coin::Error),
    FeeError(fee::Error),
    CborError(cbor_event::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::NoInputs => write!(f, "No inputs given for fee estimation"),
            &Error::NoOutputs => write!(f, "No outputs given for fee estimation"),
            &Error::NotEnoughInput => write!(f, "Not enough funds to cover outputs and fees"),
            &Error::CoinError(_) => write!(f, "Error on coin operations"),
            &Error::CborError(_) => write!(f, "Error while performing cbor serialization"),
            &Error::FeeError(_) => write!(f, "Error on fee operations"),
        }
    }
}

impl From<coin::Error> for Error {
    fn from(e: coin::Error) -> Error { Error::CoinError(e) }
}

impl From<fee::Error> for Error {
    fn from(e: fee::Error) -> Error { Error::FeeError(e) }
}

impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Error { Error::CborError(e) }
}

impl ::std::error::Error for Error {
    fn cause(&self) -> Option<& ::std::error::Error> {
        match self {
            Error::CoinError(ref err) => Some(err),
            Error::CborError(ref err) => Some(err),
            Error::FeeError(ref err)  => Some(err),
            _ => None
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

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
    fn compute<'a, 'b, I, O, Addressing>( &self
                                        , policy: SelectionPolicy
                                        , inputs: I
                                        , outputs: O
                                        , output_policy: &OutputPolicy
                                        )
            -> Result<(Fee, Vec<&'a Input<Addressing>>, Coin)>
        where I : 'a + Iterator<Item = &'a Input<Addressing>> + ExactSizeIterator
            , O : 'b + Iterator<Item = &'b TxOut> + Clone
            , Addressing: 'a
    ;
}

const TX_IN_WITNESS_CBOR_SIZE: usize = 140;
const CBOR_TXAUX_OVERHEAD: usize = 51;
impl SelectionAlgorithm for LinearFee {
    fn compute<'a, 'b, I, O, Addressing>( &self
                                        , policy: SelectionPolicy
                                        , inputs: I
                                        , outputs: O
                                        , output_policy: &OutputPolicy
                                        )
            -> Result<(Fee, Vec<&'a Input<Addressing>>, Coin)>
        where I : 'a + Iterator<Item = &'a Input<Addressing>> + ExactSizeIterator
            , O : 'b + Iterator<Item = &'b TxOut> + Clone
            , Addressing: 'a
    {
        if inputs.len() == 0 { return Err(Error::NoInputs); }

        let output_value = output_sum(outputs.clone())?;
        let mut fee = self.estimate(0)?;
        let mut input_value = Coin::zero();
        let mut selected_inputs = Vec::new();

        // create the Tx on the fly
        let mut txins = Vec::new();
        let     txouts : Vec<TxOut> = outputs.cloned().collect();

        // for now we only support this selection algorithm
        // we need to remove this assert when we extend to more
        // granulated selection policy
        assert!(policy == SelectionPolicy::FirstMatchFirst);

        for input in inputs {
            input_value = (input_value + input.value())?;
            selected_inputs.push(input);
            txins.push(input.ptr.clone());

            // calculate fee from the Tx serialised + estimated size for signing
            let mut tx = Tx::new_with(txins.clone(), txouts.clone());
            let txbytes = cbor!(&tx)?;

            let estimated_fee = (self.estimate(txbytes.len() + CBOR_TXAUX_OVERHEAD + (TX_IN_WITNESS_CBOR_SIZE * selected_inputs.len())))?;

            // add the change in the estimated fee
            if let Ok(change_value) = output_value - input_value - estimated_fee.to_coin() {
                if change_value > Coin::zero() {
                    match output_policy {
                        OutputPolicy::One(change_addr) => tx.add_output(TxOut::new(change_addr.clone(), change_value)),
                    }
                }
            };

            let txbytes = cbor!(&tx)?;
            let corrected_fee = self.estimate(txbytes.len() + CBOR_TXAUX_OVERHEAD + (TX_IN_WITNESS_CBOR_SIZE * selected_inputs.len()));

            fee = corrected_fee?;

            if Ok(input_value) >= (output_value + fee.to_coin()) { break; }
        }

        if Ok(input_value) < (output_value + fee.to_coin()) {
            return Err(Error::NotEnoughInput);
        }

        Ok((fee, selected_inputs, (input_value - output_value - fee.to_coin())?))
    }
}

/// the input selection method.
///
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum SelectionPolicy {
    /// select the first inputs that matches, no optimisation
    FirstMatchFirst
}
impl Default for SelectionPolicy {
    fn default() -> Self { SelectionPolicy::FirstMatchFirst }
}
