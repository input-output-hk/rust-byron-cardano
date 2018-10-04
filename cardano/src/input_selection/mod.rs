use std::{fmt, result};
use coin::{self, Coin};
use tx::{TxOut};
use txutils::{Input, OutputPolicy, output_sum};
use txbuild::{self, TxBuilder};
use cbor_event;
use fee::{self, Fee, FeeAlgorithm};

mod simple_selections;

pub use self::simple_selections::{HeadFirst, LargestFirst, Blackjack, BlackjackWithBackupPlan};

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
            Error::TxBuildError(ref err) => Some(err),
            _ => None
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
    pub selected_inputs: Vec<Input<Addressing>>
}
impl<A: ::std::fmt::Debug> ::std::fmt::Debug for InputSelectionResult<A> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        writeln!(f, "InputSelection:")?;
        writeln!(f, "  estimated_fee: {:?}", self.estimated_fees)?;
        writeln!(f, "  estimated_change: {:?}:", self.estimated_change)?;
        writeln!(f, "  selected_inputs ({})", self.selected_inputs.len())?;
        for input in self.selected_inputs.iter() {
            writeln!(f, "    ptr:   {:?}", input.ptr)?;
            writeln!(f, "    value: {} {}", input.value.address, input.value.value)?;
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
    fn select_input<F>( &mut self
                      , fee_algorithm: &F
                      , estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
      where F: FeeAlgorithm;

    fn compute<F>( &mut self
                 , fee_algorithm: &F
                 , outputs: Vec<TxOut>
                 , output_policy: &OutputPolicy
                 )
        -> Result<InputSelectionResult<Addressing>>
      where F: FeeAlgorithm
    {
        let mut selected = Vec::new();
        let mut builder = TxBuilder::new();
        for output in outputs { builder.add_output_value(&output); }

        let total_output = builder.get_output_total().unwrap();

        while let Some(input) = self.select_input(fee_algorithm, total_output)? {
            builder.add_input(&input.ptr, input.value.value);
            selected.push(input);

            match builder.clone().add_output_policy(fee_algorithm, output_policy) {
                Err(txbuild::Error::TxNotEnoughTotalInput) => {
                    // here we don't have enough inputs, continue the loop
                    continue;
                },
                Err(txbuild_err) => { return Err(Error::TxBuildError(txbuild_err)); },
                Ok(_) => { break; }
            }
        }

        match builder.add_output_policy(fee_algorithm, output_policy) {
            Err(txbuild::Error::TxNotEnoughTotalInput) => {
                Err(Error::NotEnoughInput)
            },
            Err(txbuild_err) => {
                Err(Error::TxBuildError(txbuild_err))
            },
            Ok(change_outputs) => {
                let fees = builder.calculate_fee(fee_algorithm).unwrap();
                let change = if change_outputs.is_empty() { None } else { Some(output_sum(change_outputs.iter())?) };
                let result = InputSelectionResult {
                    estimated_fees: fees,
                    estimated_change: change,
                    selected_inputs: selected
                };

                Ok(result)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use txutils::{Input};
    use address::{ExtendedAddr, AddrType, SpendingData, Attributes};
    use hdpayload::HDAddressPayload;
    use fee::{FeeAlgorithm, LinearFee};
    use hdwallet::{XPub, XPUB_SIZE};
    use tx::{Tx, TxInWitness, TxoPointer, TxOut, TxId};
    use coin::sum_coins;
    extern crate rand;
    use self::rand::random;

    fn mk_random_xpub() -> XPub {
        let mut xpub = [0;XPUB_SIZE];
        for byte in xpub.iter_mut() { *byte = random(); }
        XPub::from_bytes(xpub)
    }

    fn mk_random_daedalus_style_address() -> ExtendedAddr {
        let xpub = mk_random_xpub();
        let bytes : Vec<u8> = ::std::iter::repeat_with(random).take(32).collect();
        let payload = HDAddressPayload::from_vec(bytes);
        ExtendedAddr::new(
            AddrType::ATPubKey,
            SpendingData::PubKeyASD(xpub),
            Attributes::new_bootstrap_era(Some(payload))
        )
    }

    fn mk_random_icarus_style_address() -> ExtendedAddr {
        let xpub = mk_random_xpub();
        ExtendedAddr::new(
            AddrType::ATPubKey,
            SpendingData::PubKeyASD(xpub),
            Attributes::new_bootstrap_era(None)
        )
    }

    fn mk_daedalus_style_input(value: Coin) -> Input<()> {
        let txid = TxId::new(&vec![random(), random()]);
        let txoptr = TxoPointer::new(txid, random());
        let address = mk_random_daedalus_style_address();
        let txout = TxOut::new(address, value);
        Input::new(txoptr, txout, ())
    }

    fn mk_icarus_style_input(value: Coin) -> Input<()> {
        let txid = TxId::new(&vec![random(), random()]);
        let txoptr = TxoPointer::new(txid, random());
        let address = mk_random_icarus_style_address();
        let txout = TxOut::new(address, value);
        Input::new(txoptr, txout, ())
    }

    fn mk_daedalus_style_txout(value: Coin) -> TxOut {
        let address = mk_random_daedalus_style_address();
        TxOut::new(address, value)
    }

    fn mk_icarus_style_txout(value: Coin) -> TxOut {
        let address = mk_random_icarus_style_address();
        TxOut::new(address, value)
    }

    fn test_no_enough<F>(mut input_selection_scheme: F, outputs: Vec<TxOut>)
        where F: InputSelectionAlgorithm<()>
    {
        let change_address = mk_random_daedalus_style_address();

        let error = input_selection_scheme.compute(
            &LinearFee::default(),
            outputs.clone(),
            &OutputPolicy::One(change_address.clone())
        ).expect_err("Expecting error to occur");
        match error {
            Error::NotEnoughInput => (),
            err => panic!("Expected to fail with `not enough input`, but failed with {:#?}", err)
        }
    }

    fn test_fee<F>(mut input_selection_scheme: F, selected: Vec<Input<()>>, outputs: Vec<TxOut>)
        where F: InputSelectionAlgorithm<()>
    {
        let change_address = mk_random_icarus_style_address();

        let fee_alg = LinearFee::default();

        let input_selection_result = input_selection_scheme.compute(
            &fee_alg,
            outputs.clone(),
            &OutputPolicy::One(change_address.clone())
        ).unwrap();

        println!("{:#?}", input_selection_result);

        // check this is exactly the expected fee
        let mut tx = Tx::new_with(
            input_selection_result.selected_inputs.iter().map(|input| input.ptr.clone()).collect(),
            outputs
        );
        if let Some(change) = input_selection_result.estimated_change {
           tx.add_output(TxOut::new(change_address, change));
        }
        let witnesses : Vec<_> = ::std::iter::repeat(TxInWitness::fake()).take(input_selection_result.selected_inputs.len()).collect();
        let expected_fee =  fee_alg.calculate_for_txaux_component(&tx, &witnesses).unwrap();
        assert_eq!(expected_fee, input_selection_result.estimated_fees);

        // check the transaction is balanced
        let total_input = sum_coins(input_selection_result.selected_inputs.iter().map(|input| input.value.value)).unwrap();
        let total_output = output_sum(tx.outputs.iter()).unwrap();
        let fee = input_selection_result.estimated_fees.to_coin();
        assert_eq!(total_input, (total_output + fee).unwrap());

        // check the expected selected are correct
        assert_eq!(input_selection_result.selected_inputs, selected);
    }

    #[test]
    fn random_large_amount_ada_first_match_first() {
        let input1  = mk_icarus_style_input(Coin::new(25_029_238_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(                1).unwrap());

        let inputs = vec![input1.clone()];
        let outputs = vec![output1];

        let selected = vec![input1];

        test_fee(HeadFirst::from(inputs), selected, outputs);
    }

    #[test]
    fn not_enough_ada_first_match_first_1() {
        let input1  = mk_icarus_style_input(Coin::new(1).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(2).unwrap());

        let inputs = vec![input1];
        let outputs = vec![output1];

        test_no_enough(HeadFirst::from(inputs), outputs);
    }

    #[test]
    fn not_enough_ada_fisrt_match_first_2() {
        let input1  = mk_icarus_style_input(Coin::new(9_018_922_0000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(9_018_922_0000000).unwrap());

        let inputs = vec![input1];
        let outputs = vec![output1];

        test_no_enough(HeadFirst::from(inputs), outputs);
    }

    #[test]
    fn random_small_amount_ada_first_match_first() {
        let input1  = mk_daedalus_style_input(Coin::new(1).unwrap());
        let input2  = mk_daedalus_style_input(Coin::new(2).unwrap());
        let input3  = mk_icarus_style_input(Coin::new(3).unwrap());
        let input4  = mk_daedalus_style_input(Coin::new(1_000_000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1).unwrap());
        let output2 = mk_daedalus_style_txout(Coin::new(2).unwrap());

        let inputs = vec![input1.clone(), input2.clone(), input3.clone(), input4.clone()];
        let outputs = vec![output1, output2];

        let selected = vec![input1, input2, input3, input4];

        test_fee(HeadFirst::from(inputs), selected, outputs);
    }

    #[test]
    fn random_small_amount_ada_first_match_first_2() {
        let input1  = mk_icarus_style_input(Coin::new(3_000_000).unwrap());
        let input2  = mk_icarus_style_input(Coin::new(2_000_000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1_000_000).unwrap());

        let inputs = vec![input1.clone(), input2];
        let outputs = vec![output1];

        let selected = vec![input1];

        test_fee(HeadFirst::from(inputs), selected, outputs);
    }

    #[test]
    fn random_large_amount_ada_blackjack() {
        let input1  = mk_icarus_style_input(Coin::new(25_029_238_000000).unwrap());
        let input2  = mk_icarus_style_input(Coin::new(    19_928_000000).unwrap());
        let input3  = mk_icarus_style_input(Coin::new(         2_000000).unwrap());
        let input4  = mk_icarus_style_input(Coin::new(         1_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(         2_000000).unwrap());

        let inputs = vec![input1, input2, input3.clone(), input4.clone()];
        let outputs = vec![output1];

        let selected = vec![input3, input4];

        test_fee(Blackjack::from(inputs), selected, outputs);
    }

    #[test]
    fn not_enough_ada_blackjack() {
        let input1  = mk_icarus_style_input(Coin::new(25_029_238_000000).unwrap());
        let input2  = mk_icarus_style_input(Coin::new(    19_928_000000).unwrap());
        let input3  = mk_icarus_style_input(Coin::new(         2_000000).unwrap());
        let input4  = mk_icarus_style_input(Coin::new(         2_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(         1_000000).unwrap());

        let inputs = vec![input1, input2, input3.clone(), input4.clone()];
        let outputs = vec![output1];

        test_no_enough(Blackjack::from(inputs), outputs);
    }

    #[test]
    fn not_enough_ada_blackjack_with_backup() {
        let input1  = mk_icarus_style_input(Coin::new(2_000000).unwrap());
        let input2  = mk_icarus_style_input(Coin::new(2_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(5_000000).unwrap());

        let inputs = vec![input1, input2];
        let outputs = vec![output1];

        test_no_enough(BlackjackWithBackupPlan::from(inputs), outputs);
    }

    #[test]
    fn ada_blackjack_with_backup() {
        let input1  = mk_icarus_style_input(Coin::new(42_000000).unwrap());
        let input2  = mk_icarus_style_input(Coin::new( 2_000000).unwrap());
        let input3  = mk_icarus_style_input(Coin::new( 2_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new( 5_000000).unwrap());

        let inputs = vec![input1.clone(), input2.clone(), input3.clone()];
        let outputs = vec![output1];

        let selected = vec![input2, input3, input1];

        test_fee(BlackjackWithBackupPlan::from(inputs), selected, outputs);
    }

    #[test]
    fn ermurgo_1() {
        let input1  = mk_icarus_style_input(Coin::new(25_999_999_656409).unwrap());
        let output1 = mk_daedalus_style_txout(Coin::new(1_000000).unwrap());

        let inputs = vec![input1.clone()];
        let outputs = vec![output1];

        let selected = vec![input1];

        test_fee(HeadFirst::from(inputs), selected, outputs);

        assert!(false);
    }
}
