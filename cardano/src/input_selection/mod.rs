use cbor_event;
use coin::{self, Coin};
use fee::{self, Fee, FeeAlgorithm};
use std::{fmt, result};
use tx::TxOut;
use txbuild::{self, TxBuilder};
use txutils::{output_sum, Input, OutputPolicy};

mod simple_selections;

pub use self::simple_selections::{Blackjack, HeadFirst, LargestFirst};

#[derive(Debug)]
pub enum Error {
    NoInputs,
    NoOutputs,
    NotEnoughInput,
    NotEnoughFees,
    TxBuildError(txbuild::Error),
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
            &Error::NotEnoughFees => write!(f, "Not enough fees"),
            &Error::TxBuildError(_) => write!(f, "TxBuild Error"),
            &Error::CoinError(_) => write!(f, "Error on coin operations"),
            &Error::CborError(_) => write!(f, "Error while performing cbor serialization"),
            &Error::FeeError(_) => write!(f, "Error on fee operations"),
        }
    }
}

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

impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Error {
        Error::CborError(e)
    }
}

impl ::std::error::Error for Error {
    fn cause(&self) -> Option<&::std::error::Error> {
        match self {
            Error::CoinError(ref err) => Some(err),
            Error::CborError(ref err) => Some(err),
            Error::FeeError(ref err) => Some(err),
            Error::TxBuildError(ref err) => Some(err),
            _ => None,
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

/// The input selection result structure
///
/// This allows to put a name (and a meaning) to the output.
///
#[derive(PartialEq, Eq, Clone)]
pub struct InputSelectionResult<Addressing> {
    /// The estimated fee by the input selection algorithm
    pub estimated_fees: Fee,

    /// the estimated total change (the left over to refund to the users)
    ///
    /// It reflects the total of all the changes used. See [`OutputPolicy`]
    /// for more details about the change addresses.
    ///
    /// [`OutputPolicy`]:
    pub estimated_change: Option<Coin>,

    /// the selected input
    pub selected_inputs: Vec<Input<Addressing>>,
}
impl<A: ::std::fmt::Debug> ::std::fmt::Debug for InputSelectionResult<A> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        writeln!(f, "InputSelection:")?;
        writeln!(f, "  estimated_fee: {:?}", self.estimated_fees)?;
        writeln!(f, "  estimated_change: {:?}:", self.estimated_change)?;
        writeln!(f, "  selected_inputs ({})", self.selected_inputs.len())?;
        for input in self.selected_inputs.iter() {
            writeln!(f, "    ptr:   {:?}", input.ptr)?;
            writeln!(
                f,
                "    value: {} {}",
                input.value.address, input.value.value
            )?;
            writeln!(f, "    addressing: {:?}", input.addressing)?;
        }
        Ok(())
    }
}

/// trait to implement the input selection algorithm
///
/// The trait is split into 2 main functions:
///
/// * `select_input`: the function that will select one input that will get us
///   closer or will match the `estimated_needed_output`.
/// * `compute`: this function will run the full input selection algorithm,
///   calling `select_input` as many time as necessary to cover both all the
///   output target and the necessary fee.
///
/// By default, `compute` is already implemented to call `select_input` in an
/// efficient manner. But it might be necessary, in the future, to implement
/// specific cases to compute the whole input selection algorithm for specific
/// cases.
///
pub trait InputSelectionAlgorithm<Addressing> {
    fn select_input<F>(
        &mut self,
        fee_algorithm: &F,
        estimated_needed_output: Coin,
    ) -> Result<Option<Input<Addressing>>>
    where
        F: FeeAlgorithm;

    fn compute<F>(
        &mut self,
        fee_algorithm: &F,
        outputs: Vec<TxOut>,
        output_policy: &OutputPolicy,
    ) -> Result<InputSelectionResult<Addressing>>
    where
        F: FeeAlgorithm,
    {
        let mut selected = Vec::new();
        let mut builder = TxBuilder::new();

        if outputs.is_empty() {
            return Err(Error::NoOutputs);
        }

        for output in outputs {
            builder.add_output_value(&output);
        }

        let total_output = builder.get_output_total().unwrap();
        let mut estimated_needed_output =
            (total_output + builder.calculate_fee(fee_algorithm).unwrap().to_coin()).unwrap();

        while let Some(input) = self.select_input(fee_algorithm, estimated_needed_output)? {
            builder.add_input(&input.ptr, input.value.value);
            selected.push(input);

            // update the estimated needed output every time we add an input
            // this is because every time we add an input, we add more to the transaction
            // and the fee increase
            estimated_needed_output =
                (total_output + builder.calculate_fee(fee_algorithm).unwrap().to_coin()).unwrap();

            match builder
                .clone()
                .add_output_policy(fee_algorithm, output_policy)
            {
                Err(txbuild::Error::TxNotEnoughTotalInput) => {
                    // here we don't have enough inputs, continue the loop
                    continue;
                }
                Err(txbuild::Error::TxOutputPolicyNotEnoughCoins(_)) => {
                    // we accept we might lose some dust here...
                    break;
                }
                Err(txbuild_err) => {
                    return Err(Error::TxBuildError(txbuild_err));
                }
                Ok(_) => {
                    break;
                }
            }
        }

        let (change, loss) = match builder.add_output_policy(fee_algorithm, output_policy) {
            Err(txbuild::Error::TxNotEnoughTotalInput) => {
                return Err(Error::NotEnoughInput);
            }
            Err(txbuild::Error::TxOutputPolicyNotEnoughCoins(loss)) => (None, Some(loss)),
            Err(txbuild_err) => {
                return Err(Error::TxBuildError(txbuild_err));
            }
            Ok(change_outputs) => (
                if change_outputs.is_empty() {
                    None
                } else {
                    Some(output_sum(change_outputs.iter())?)
                },
                None,
            ),
        };

        let fees = builder.calculate_fee(fee_algorithm).unwrap();
        let fees = if let Some(loss) = loss {
            Fee::new((fees.to_coin() + loss)?)
        } else {
            fees
        };
        let result = InputSelectionResult {
            estimated_fees: fees,
            estimated_change: change,
            selected_inputs: selected,
        };
        Ok(result)
    }
}
