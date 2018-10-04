use std::{fmt, result};
use coin::{self, Coin};
use tx::{TxOut};
use txutils::{Input, OutputPolicy, output_sum};
use txbuild::{self, TxBuilder};
use cbor_event;
use fee::{self, Fee, FeeAlgorithm};

#[derive(Debug)]
pub enum Error {
    NoInputs,
    NoOutputs,
    NotEnoughInput,
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
#[derive(Debug, PartialEq, Eq, Clone)]
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

/// Take the given input collections and select the inputs in the given order
///
/// This is the least interesting algorithm, it is however very simple and
/// provide the interesting property to be reproducible.
///
#[derive(Debug, Clone)]
pub struct FirstMatchFirst<Addressing>(Vec<Input<Addressing>>);
impl<Addressing> From<Vec<Input<Addressing>>> for FirstMatchFirst<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self { FirstMatchFirst(inputs) }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for FirstMatchFirst<Addressing> {
    fn select_input<F>( &mut self
                      , _fee_algorithm: &F
                      , _estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
      where F: FeeAlgorithm
    {
        if self.0.is_empty() {
            Ok(None)
        } else {
            Ok(Some(self.0.remove(0)))
        }
    }
}

/// Takes the large inputs first.
///
/// About the same as `FirstMatchFirst` but sort the input list
/// to take the largest inputs first.
///
#[derive(Debug, Clone)]
pub struct LargeInputFirst<Addressing>(FirstMatchFirst<Addressing>);
impl<Addressing> From<Vec<Input<Addressing>>> for LargeInputFirst<Addressing> {
    fn from(mut inputs: Vec<Input<Addressing>>) -> Self {
        inputs.sort_unstable_by(|i1, i2| i2.value.value.cmp(&i1.value.value));
        LargeInputFirst(FirstMatchFirst::from(inputs))
    }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for LargeInputFirst<Addressing> {
    fn select_input<F>( &mut self
                      , fee_algorithm: &F
                      , estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
      where F: FeeAlgorithm
    {
        self.0.select_input(fee_algorithm, estimated_needed_output)
    }
}

/// This input selection strategy will accumulates inputs until the target value
/// is matched, except it ignores the inputs that go over the target value
pub struct Blackjack<Addressing>(LargeInputFirst<Addressing>);
impl<Addressing> Blackjack<Addressing> {
    #[inline]
    fn find_index_where_value_less_than(&self, needed_output: Coin) -> Option<usize> {
        ((self.0).0).0.iter().position(|input| input.value.value <= needed_output)
    }
}
impl<Addressing> From<Vec<Input<Addressing>>> for Blackjack<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self {
        Blackjack(LargeInputFirst::from(inputs))
    }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for Blackjack<Addressing> {
    fn select_input<F>( &mut self
                      , _fee_algorithm: &F
                      , estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
      where F: FeeAlgorithm
    {
        let index = self.find_index_where_value_less_than(estimated_needed_output);
        match index {
            None => Ok(None),
            Some(index) => {
                Ok(Some(((self.0).0).0.remove(index)))
            }
        }
    }
}

/// Blackjack with Backup (Large input first)
///
/// Considering a collection of input (ordered large input to small input), we will take
/// the first inputs that are below the expected amount. This is in order to minimise using
/// large inputs for small transactions.
///
/// Once there is no longer inputs below the targeted output, it will fallback to `LargeInputFirst`.
///
enum BlackjackWithBackupPlanE<Addressing> {
    Blackjack(Blackjack<Addressing>),
    BackupPlan(LargeInputFirst<Addressing>)
}
pub struct BlackjackWithBackupPlan<Addressing>(BlackjackWithBackupPlanE<Addressing>);
impl<Addressing> From<Vec<Input<Addressing>>> for BlackjackWithBackupPlan<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self {
        BlackjackWithBackupPlan(
        BlackjackWithBackupPlanE::Blackjack(
            Blackjack::from(inputs)
        ))
    }
}
impl<Addressing: Clone> InputSelectionAlgorithm<Addressing> for BlackjackWithBackupPlan<Addressing> {
    fn select_input<F>( &mut self
                      , fee_algorithm: &F
                      , estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
      where F: FeeAlgorithm
    {
        let input_1 = match &mut self.0 {
            BlackjackWithBackupPlanE::Blackjack(ref mut v) => {
                v.select_input(fee_algorithm, estimated_needed_output)?
            }
            BlackjackWithBackupPlanE::BackupPlan(ref mut v) => {
                v.select_input(fee_algorithm, estimated_needed_output)?
            }
        };

        if input_1.is_none() {
            let backup = if let BlackjackWithBackupPlanE::Blackjack(Blackjack(lif)) = &self.0 {
                lif.clone()
            } else {
                return Ok(None)
            };
            self.0 = BlackjackWithBackupPlanE::BackupPlan(backup);
            self.select_input(fee_algorithm, estimated_needed_output)
        } else {
            Ok(input_1)
        }
    }
}


/*
pub struct Custom<Addressing> {
    available_inputs: Vec<Input<Addressing>>,
    selected_inputs: Vec<Input<Addressing>>,
    custom_selector: Box< FnMut(&mut Vec<Input<Addressing>>, &mut Vec<Input<Addressing>>, Coin) -> Result<Option<Input<Addressing>>> >
}
impl<Addressing: Clone> InputSelectionAlgorithm<Addressing> for Custom<Addressing> {
    fn select_input<F>( &mut self
                      , fee_algorithm: &F
                      , estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
    {
        (self.custom_selector)(&mut self.available_inputs, &mut self.selected_inputs, estimated_needed_output)
    }
}
*/

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
        let change_address = mk_random_daedalus_style_address();

        let fee_alg = LinearFee::default();

        let input_selection_result = input_selection_scheme.compute(
            &fee_alg,
            outputs.clone(),
            &OutputPolicy::One(change_address.clone())
        ).unwrap();

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

        test_fee(FirstMatchFirst::from(inputs), selected, outputs);
    }

    #[test]
    fn not_enough_ada_first_match_first_1() {
        let input1  = mk_icarus_style_input(Coin::new(1).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(2).unwrap());

        let inputs = vec![input1];
        let outputs = vec![output1];

        test_no_enough(FirstMatchFirst::from(inputs), outputs);
    }

    #[test]
    fn not_enough_ada_fisrt_match_first_2() {
        let input1  = mk_icarus_style_input(Coin::new(9_018_922_0000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(9_018_922_0000000).unwrap());

        let inputs = vec![input1];
        let outputs = vec![output1];

        test_no_enough(FirstMatchFirst::from(inputs), outputs);
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

        test_fee(FirstMatchFirst::from(inputs), selected, outputs);
    }

    #[test]
    fn random_small_amount_ada_first_match_first_2() {
        let input1  = mk_icarus_style_input(Coin::new(3_000_000).unwrap());
        let input2  = mk_icarus_style_input(Coin::new(2_000_000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1_000_000).unwrap());

        let inputs = vec![input1.clone(), input2];
        let outputs = vec![output1];

        let selected = vec![input1];

        test_fee(FirstMatchFirst::from(inputs), selected, outputs);
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
}
