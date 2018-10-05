use super::*;

/// Take the given input collections and select the inputs in the given order
///
/// This is the least interesting algorithm, it is however very simple and
/// provide the interesting property to be reproducible.
///
#[derive(Debug, Clone)]
pub struct HeadFirst<Addressing>(Vec<Input<Addressing>>);
impl<Addressing> From<Vec<Input<Addressing>>> for HeadFirst<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self { HeadFirst(inputs) }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for HeadFirst<Addressing> {
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
pub struct LargestFirst<Addressing>(HeadFirst<Addressing>);
impl<Addressing> From<Vec<Input<Addressing>>> for LargestFirst<Addressing> {
    fn from(mut inputs: Vec<Input<Addressing>>) -> Self {
        inputs.sort_unstable_by(|i1, i2| i2.value.value.cmp(&i1.value.value));
        LargestFirst(HeadFirst::from(inputs))
    }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for LargestFirst<Addressing> {
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
pub struct Blackjack<Addressing>(LargestFirst<Addressing>);
impl<Addressing> Blackjack<Addressing> {
    #[inline]
    fn find_index_where_value_less_than(&self, needed_output: Coin) -> Option<usize> {
        ((self.0).0).0.iter().position(|input| input.value.value <= needed_output)
    }
}
impl<Addressing> From<Vec<Input<Addressing>>> for Blackjack<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self {
        Blackjack(LargestFirst::from(inputs))
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
    BackupPlan(LargestFirst<Addressing>)
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

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::{BTreeMap};
    use config::{ProtocolMagic};

    use fee::{FeeAlgorithm, LinearFee};
    use address::{ExtendedAddr};
    use super::super::super::{util::arbitrary::Wrapper, tx};
    use tx::{TxoPointer};
    use hdwallet::{XPrv};

    use quickcheck::{Gen, Arbitrary};

    const MAX_NUM_INPUTS  : usize = 254;
    const MAX_NUM_OUTPUTS : usize = 64;
    const TX_SIZE_LIMIT : usize = 65536;

    #[derive(Clone, Debug)]
    struct Inputs {
        private_keys: BTreeMap<TxoPointer, XPrv>,
        inputs: Vec<Input<()>>
    }
    impl Arbitrary for Inputs {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut inputs = Vec::new();
            let mut private_keys = BTreeMap::new();
            let mut total_input = Coin::zero();
            let num_inputs = <usize as Arbitrary>::arbitrary(g) % MAX_NUM_INPUTS;
            for _ in 0..num_inputs {
                let value : Wrapper<(_, _)> = Arbitrary::arbitrary(g);
                let value : (XPrv, Input<()>) = value.unwrap();

                // here we check that the total inputs never overflow the
                // total number of coins
                total_input = match total_input + value.1.value.value {
                    Err(_) => break,
                    Ok(v)  => v
                };
                private_keys.insert(value.1.ptr.clone(), value.0);
                inputs.push(value.1);
            }
            Inputs {
                private_keys,
                inputs
            }
        }
    }

    #[derive(Clone, Debug)]
    struct Outputs {
        outputs: Vec<tx::TxOut>,
        change_address: ExtendedAddr,
    }
    impl Arbitrary for Outputs {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let num_outputs = <usize as Arbitrary>::arbitrary(g) % MAX_NUM_OUTPUTS;
            let mut total_output = Coin::zero();
            let mut outputs = Vec::new();
            for _ in 0..num_outputs {
                let value = <Wrapper<(XPrv, TxOut)> as Arbitrary>::arbitrary(g);
                let value = value.unwrap();

                // make sure that the generated total output does not exceed
                // the total Ada
                total_output = match total_output + value.1.value {
                    Err(_) => break,
                    Ok(v)  => v
                };
                outputs.push(value.1)
            }
            let change_address : Wrapper<(_, ExtendedAddr)> = Arbitrary::arbitrary(g);
            Outputs {
                outputs,
                change_address: change_address.unwrap().1
            }
        }
    }

    // this is the test that will be run to check that the input selection
    // returns only valid results
    //
    fn test_fee<A, F, IS>( value: (Wrapper<ProtocolMagic>, Inputs, Outputs)
                         , into_input_selection: F
                         , fee_alg: A
                         , max_fee: Fee
                         )
        -> bool
      where F: FnOnce(Vec<Input<()>>) -> IS
          , IS: InputSelectionAlgorithm<()>
          , A: FeeAlgorithm
    {
        let total_input = coin::sum_coins(value.1.inputs.iter().map(|v| v.value.value)).unwrap();
        let total_output = coin::sum_coins(value.2.outputs.iter().map(|v| v.value)).unwrap();
        let mut input_selection_scheme  = into_input_selection(value.1.inputs);
        let private_keys = value.1.private_keys;
        let outputs = value.2.outputs;
        let change_address = value.2.change_address;
        let protocol_magic = *value.0;

        let input_selection_result = input_selection_scheme.compute(
            &fee_alg,
            outputs.clone(),
            &OutputPolicy::One(change_address.clone())
        );
        let input_selection_result = match input_selection_result {
            Ok(r) => r, // Continue checking
            Err(Error::NoOutputs) => { return outputs.is_empty(); },
            Err(Error::NotEnoughInput) => {
                return total_input < (total_output + max_fee.to_coin()).unwrap();
            },
            Err(err) => {
                eprintln!("{}", err);
                return false
            }
        };

        // check this is exactly the expected fee
        let mut tx = tx::Tx::new_with(
            input_selection_result.selected_inputs.iter().map(|input| input.ptr.clone()).collect(),
            outputs
        );
        if let Some(change) = input_selection_result.estimated_change {
           tx.add_output(TxOut::new(change_address, change));
        }
        let txid = tx.id();
        let mut witnesses = Vec::with_capacity(input_selection_result.selected_inputs.len());
        for input in input_selection_result.selected_inputs.iter() {
            let key = private_keys.get(&input.ptr).expect("this should always successfully finds the private key");
            let witness = tx::TxInWitness::new(protocol_magic, key, &txid);
            witnesses.push(witness);
        }
        let expected_fee =  fee_alg.calculate_for_txaux_component(&tx, &witnesses).unwrap();
        if expected_fee != input_selection_result.estimated_fees { return false; }

        // check the transaction is balanced
        let total_input = coin::sum_coins(input_selection_result.selected_inputs.iter().map(|input| input.value.value)).unwrap();
        let total_output = output_sum(tx.outputs.iter()).unwrap();
        let fee = input_selection_result.estimated_fees.to_coin();
        if total_input != (total_output + fee).unwrap() { return false; }

        true
    }

    quickcheck! {
        fn head_first(value: (Wrapper<ProtocolMagic>, Inputs, Outputs)) -> bool {
            let fee_alg = LinearFee::default();
            let max_fee = fee_alg.estimate(TX_SIZE_LIMIT).unwrap();
            test_fee(value, HeadFirst::from, fee_alg, max_fee)
        }

        fn largest_first(value: (Wrapper<ProtocolMagic>, Inputs, Outputs)) -> bool {
            let fee_alg = LinearFee::default();
            let max_fee = fee_alg.estimate(TX_SIZE_LIMIT).unwrap();
            test_fee(value, LargestFirst::from, fee_alg, max_fee)
        }

        fn blackjack(value: (Wrapper<ProtocolMagic>, Inputs, Outputs)) -> bool {
            let fee_alg = LinearFee::default();
            let max_fee = fee_alg.estimate(TX_SIZE_LIMIT).unwrap();
            test_fee(value, Blackjack::from, fee_alg, max_fee)
        }

        fn blackjack_with_backup(value: (Wrapper<ProtocolMagic>, Inputs, Outputs)) -> bool {
            let fee_alg = LinearFee::default();
            let max_fee = fee_alg.estimate(TX_SIZE_LIMIT).unwrap();
            test_fee(value, BlackjackWithBackupPlan::from, fee_alg, max_fee)
        }
    }
}
