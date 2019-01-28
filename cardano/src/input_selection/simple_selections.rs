use super::*;

/// Take the given input collections and select the inputs in the given order
///
/// This is the least interesting algorithm, it is however very simple and
/// provide the interesting property to be reproducible.
///
#[derive(Debug, Clone)]
pub struct HeadFirst<Addressing>(Vec<Input<Addressing>>);
impl<Addressing> From<Vec<Input<Addressing>>> for HeadFirst<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self {
        HeadFirst(inputs)
    }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for HeadFirst<Addressing> {
    fn select_input<F>(
        &mut self,
        _fee_algorithm: &F,
        _estimated_needed_output: Coin,
    ) -> Result<Option<Input<Addressing>>>
    where
        F: FeeAlgorithm,
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
    fn select_input<F>(
        &mut self,
        fee_algorithm: &F,
        estimated_needed_output: Coin,
    ) -> Result<Option<Input<Addressing>>>
    where
        F: FeeAlgorithm,
    {
        self.0.select_input(fee_algorithm, estimated_needed_output)
    }
}

#[derive(Debug, Clone, Copy)]
struct BasicRandom {
    state: u32,
}
impl BasicRandom {
    fn new(initial_state: u32) -> Self {
        BasicRandom {
            state: initial_state,
        }
    }

    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        return self.state;
    }
}

/// This input selection strategy will accumulates inputs until the target value
/// is matched, except it ignores the inputs that go over the target value
pub struct Blackjack<Addressing> {
    inputs: Vec<(bool, Input<Addressing>)>,
    total_input_selected: Coin,
    dust_threshold: Coin,
    random_generator: BasicRandom,
}
impl<Addressing> Blackjack<Addressing> {
    #[inline]
    fn find_index<I>(
        &mut self,
        mut inputs: I, // &[(usize, Input<Addressing>)]
    ) -> Option<usize>
    where
        I: Iterator<Item = usize> + ExactSizeIterator,
    {
        if inputs.len() == 0 {
            return None;
        }
        let index = self.random_generator.next() as usize % inputs.len();

        inputs.nth(index)
    }

    pub fn new(dust_threshold: Coin, inputs: Vec<Input<Addressing>>) -> Self {
        let seed = inputs.len() as u64 + u64::from(dust_threshold);
        Blackjack {
            inputs: inputs.into_iter().map(|i| (false, i)).collect(),
            total_input_selected: Coin::zero(),
            dust_threshold: dust_threshold,
            random_generator: BasicRandom::new(seed as u32),
        }
    }
}
impl<Addressing: Clone> InputSelectionAlgorithm<Addressing> for Blackjack<Addressing> {
    fn select_input<F>(
        &mut self,
        fee_algorithm: &F,
        estimated_needed_output: Coin,
    ) -> Result<Option<Input<Addressing>>>
    where
        F: FeeAlgorithm,
    {
        use tx::{TxId, TxInWitness};

        const MAX_OVERHEAD_INDEX: usize = 5; // 32bits + 1 bytes of CBOR...
        const MAX_OVERHEAD_TXID: usize = TxId::HASH_SIZE + 2; // 2 bytes of Cbor...
        const MAX_OVERHEAD_TXIN: usize = MAX_OVERHEAD_INDEX + MAX_OVERHEAD_TXID + 2; // 2 bytes of cbor

        let signature_cost = fee_algorithm
            .estimate_overhead(cbor!(TxInWitness::fake()).unwrap().len())?
            .unwrap_or(Fee::new(Coin::zero()))
            .to_coin();

        let overhead_input = fee_algorithm
            .estimate_overhead(MAX_OVERHEAD_TXIN)?
            .unwrap_or(Fee::new(Coin::zero()))
            .to_coin();
        let max_value = (((estimated_needed_output + overhead_input)? + self.dust_threshold)?
            + signature_cost
            - self.total_input_selected)?;

        let filtered_inputs: Vec<usize> = self
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(i, (b, input))| {
                if !*b && input.value.value <= max_value {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        let index = self.find_index(filtered_inputs.into_iter());
        match index {
            None => Ok(None),
            Some(index) => match self.inputs.get_mut(index) {
                Some(ref mut input) => {
                    input.0 = true;
                    self.total_input_selected = (self.total_input_selected + input.1.value.value)?;
                    Ok(Some(input.1.clone()))
                }
                None => unreachable!(),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use config::ProtocolMagic;
    use std::collections::BTreeMap;

    use super::super::super::{tx, util::arbitrary::Wrapper};
    use address::ExtendedAddr;
    use fee::{FeeAlgorithm, LinearFee};
    use hdwallet::XPrv;
    use tx::TxoPointer;

    use quickcheck::{Arbitrary, Gen};

    const MAX_NUM_INPUTS: usize = 254;
    const MAX_NUM_OUTPUTS: usize = 64;
    const TX_SIZE_LIMIT: usize = 65536;

    #[derive(Clone, Debug)]
    struct Inputs {
        private_keys: BTreeMap<TxoPointer, XPrv>,
        inputs: Vec<Input<()>>,
    }
    impl Arbitrary for Inputs {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut inputs = Vec::new();
            let mut private_keys = BTreeMap::new();
            let mut total_input = Coin::zero();
            let num_inputs = <usize as Arbitrary>::arbitrary(g) % MAX_NUM_INPUTS;
            for _ in 0..num_inputs {
                let value: Wrapper<(_, _)> = Arbitrary::arbitrary(g);
                let value: (XPrv, Input<()>) = value.unwrap();

                // here we check that the total inputs never overflow the
                // total number of coins
                total_input = match total_input + value.1.value.value {
                    Err(_) => break,
                    Ok(v) => v,
                };
                private_keys.insert(value.1.ptr.clone(), value.0);
                inputs.push(value.1);
            }
            Inputs {
                private_keys,
                inputs,
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
                    Ok(v) => v,
                };
                outputs.push(value.1)
            }
            let change_address: Wrapper<(_, ExtendedAddr)> = Arbitrary::arbitrary(g);
            Outputs {
                outputs,
                change_address: change_address.unwrap().1,
            }
        }
    }

    // this is the test that will be run to check that the input selection
    // returns only valid results
    //
    fn test_fee<A, F, IS>(
        value: (Wrapper<ProtocolMagic>, Inputs, Outputs),
        into_input_selection: F,
        fee_alg: A,
        max_fee: Fee,
    ) -> bool
    where
        F: FnOnce(Vec<Input<()>>) -> IS,
        IS: InputSelectionAlgorithm<()>,
        A: FeeAlgorithm,
    {
        // prepare the different inputs and values

        let total_input =
            coin::sum_coins(value.1.inputs.iter().map(|v| v.value.value)).expect("total input");
        let total_output =
            coin::sum_coins(value.2.outputs.iter().map(|v| v.value)).expect("total output");
        let mut input_selection_scheme = into_input_selection(value.1.inputs);
        let private_keys = value.1.private_keys;
        let outputs = value.2.outputs;
        let change_address = value.2.change_address;
        let protocol_magic = *value.0;

        // run the input selection algorithm

        let input_selection_result = input_selection_scheme.compute(
            &fee_alg,
            outputs.clone(),
            &OutputPolicy::One(change_address.clone()),
        );

        // check the return value make sense

        let input_selection_result = match input_selection_result {
            Ok(r) => {
                // no error returned, we will check the returned values
                // are consistent
                r
            }
            Err(Error::NoOutputs) => {
                // check that actually no outputs where given
                return outputs.is_empty();
            }
            Err(Error::NotEnoughInput) => {
                // the algorithm said there was not enough inputs to cover the transaction
                // check it is true and there there was not enough inputs
                // to cover the whole transaction (outputs + fee)
                return total_input < (total_output + max_fee.to_coin()).expect("valid coin sum");
            }
            Err(err) => {
                // this may happen with an unexpected error
                eprintln!("{}", err);
                return false;
            }
        };

        // ------- Then check that successful input selection are values -----------

        // build the tx and witnesses

        let mut tx = tx::Tx::new_with(
            input_selection_result
                .selected_inputs
                .iter()
                .map(|input| input.ptr.clone())
                .collect(),
            outputs,
        );
        if let Some(change) = input_selection_result.estimated_change {
            tx.add_output(TxOut::new(change_address, change));
        }
        let txid = tx.id();
        let mut witnesses = Vec::with_capacity(input_selection_result.selected_inputs.len());
        for input in input_selection_result.selected_inputs.iter() {
            let key = private_keys
                .get(&input.ptr)
                .expect("this should always successfully finds the private key");
            let witness = tx::TxInWitness::new_extended_pk(protocol_magic, key, &txid);
            witnesses.push(witness);
        }
        let expected_fee = fee_alg
            .calculate_for_txaux_component(&tx, &witnesses)
            .expect("calculate fee for txaux components");

        // check the expected fee is exactly the estimated fees
        if expected_fee != input_selection_result.estimated_fees {
            return false;
        }

        // check the transaction is balanced

        let total_input = coin::sum_coins(
            input_selection_result
                .selected_inputs
                .iter()
                .map(|input| input.value.value),
        )
        .expect("total selected inputs");
        let total_output = output_sum(tx.outputs.iter()).expect("transaction outputs");
        let fee = input_selection_result.estimated_fees.to_coin();
        if total_input != (total_output + fee).expect("valid sum") {
            return false;
        }

        true
    }

    quickcheck! {
        fn head_first(value: (Wrapper<ProtocolMagic>, Inputs, Outputs)) -> bool {
            let fee_alg = LinearFee::default();
            let max_fee = fee_alg.estimate(TX_SIZE_LIMIT).expect("max fee");
            test_fee(value, HeadFirst::from, fee_alg, max_fee)
        }

        fn largest_first(value: (Wrapper<ProtocolMagic>, Inputs, Outputs)) -> bool {
            let fee_alg = LinearFee::default();
            let max_fee = fee_alg.estimate(TX_SIZE_LIMIT).expect("max fee");
            test_fee(value, LargestFirst::from, fee_alg, max_fee)
        }

        fn blackjack(value: (Wrapper<ProtocolMagic>, Inputs, Outputs)) -> bool {
            let fee_alg = LinearFee::default();
            let max_fee = fee_alg.estimate(TX_SIZE_LIMIT).expect("max fee");
            test_fee(value, |i| Blackjack::new(Coin::from(100_000), i), fee_alg, max_fee)
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use address::{AddrType, Attributes, ExtendedAddr, SpendingData};
    use coin::sum_coins;
    use config::NetworkMagic;
    use fee::{FeeAlgorithm, LinearFee};
    use hdpayload::HDAddressPayload;
    use hdwallet::{XPub, XPUB_SIZE};
    use tx::{Tx, TxId, TxInWitness, TxOut, TxoPointer};
    use txutils::Input;
    extern crate rand;
    use self::rand::random;

    fn mk_random_xpub() -> XPub {
        let mut xpub = [0; XPUB_SIZE];
        for byte in xpub.iter_mut() {
            *byte = random();
        }
        XPub::from_bytes(xpub)
    }

    fn mk_random_daedalus_style_address() -> ExtendedAddr {
        let xpub = mk_random_xpub();
        let bytes: Vec<u8> = ::std::iter::repeat_with(random).take(32).collect();
        let payload = HDAddressPayload::from_vec(bytes);
        ExtendedAddr::new(
            AddrType::ATPubKey,
            SpendingData::PubKeyASD(xpub),
            Attributes::new_bootstrap_era(Some(payload), NetworkMagic::NoMagic),
        )
    }

    fn mk_random_icarus_style_address() -> ExtendedAddr {
        let xpub = mk_random_xpub();
        ExtendedAddr::new(
            AddrType::ATPubKey,
            SpendingData::PubKeyASD(xpub),
            Attributes::new_bootstrap_era(None, NetworkMagic::NoMagic),
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
    where
        F: InputSelectionAlgorithm<()>,
    {
        let change_address = mk_random_daedalus_style_address();

        let error = input_selection_scheme
            .compute(
                &LinearFee::default(),
                outputs.clone(),
                &OutputPolicy::One(change_address.clone()),
            )
            .expect_err("Expecting error to occur");
        match error {
            Error::NotEnoughInput => (),
            err => panic!(
                "Expected to fail with `not enough input`, but failed with {:#?}",
                err
            ),
        }
    }

    fn test_fee<F>(mut input_selection_scheme: F, selected: Vec<Input<()>>, outputs: Vec<TxOut>)
    where
        F: InputSelectionAlgorithm<()>,
    {
        let change_address = mk_random_icarus_style_address();

        let fee_alg = LinearFee::default();

        let input_selection_result = input_selection_scheme
            .compute(
                &fee_alg,
                outputs.clone(),
                &OutputPolicy::One(change_address.clone()),
            )
            .expect("to run the input selection scheme successfully");

        println!("{:#?}", input_selection_result);

        // check this is exactly the expected fee
        let mut tx = Tx::new_with(
            input_selection_result
                .selected_inputs
                .iter()
                .map(|input| input.ptr.clone())
                .collect(),
            outputs,
        );
        if let Some(change) = input_selection_result.estimated_change {
            tx.add_output(TxOut::new(change_address, change));
        }
        let witnesses: Vec<_> = ::std::iter::repeat(TxInWitness::fake())
            .take(input_selection_result.selected_inputs.len())
            .collect();
        let expected_fee = fee_alg
            .calculate_for_txaux_component(&tx, &witnesses)
            .unwrap();
        assert_eq!(expected_fee, input_selection_result.estimated_fees);

        // check the transaction is balanced
        let total_input = sum_coins(
            input_selection_result
                .selected_inputs
                .iter()
                .map(|input| input.value.value),
        )
        .unwrap();
        let total_output = output_sum(tx.outputs.iter()).unwrap();
        let fee = input_selection_result.estimated_fees.to_coin();
        assert_eq!(total_input, (total_output + fee).unwrap());

        // check the expected selected are correct
        assert_eq!(input_selection_result.selected_inputs, selected);
    }

    #[test]
    fn random_large_amount_ada_first_match_first() {
        let input1 = mk_icarus_style_input(Coin::new(25_029_238_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1).unwrap());

        let inputs = vec![input1.clone()];
        let outputs = vec![output1];

        let selected = vec![input1];

        test_fee(HeadFirst::from(inputs), selected, outputs);
    }

    #[test]
    fn not_enough_ada_first_match_first_1() {
        let input1 = mk_icarus_style_input(Coin::new(1).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(2).unwrap());

        let inputs = vec![input1];
        let outputs = vec![output1];

        test_no_enough(HeadFirst::from(inputs), outputs);
    }

    #[test]
    fn not_enough_ada_fisrt_match_first_2() {
        let input1 = mk_icarus_style_input(Coin::new(9_018_922_0000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(9_018_922_0000000).unwrap());

        let inputs = vec![input1];
        let outputs = vec![output1];

        test_no_enough(HeadFirst::from(inputs), outputs);
    }

    #[test]
    fn random_small_amount_ada_first_match_first() {
        let input1 = mk_daedalus_style_input(Coin::new(1).unwrap());
        let input2 = mk_daedalus_style_input(Coin::new(2).unwrap());
        let input3 = mk_icarus_style_input(Coin::new(3).unwrap());
        let input4 = mk_daedalus_style_input(Coin::new(1_000_000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1).unwrap());
        let output2 = mk_daedalus_style_txout(Coin::new(2).unwrap());

        let inputs = vec![
            input1.clone(),
            input2.clone(),
            input3.clone(),
            input4.clone(),
        ];
        let outputs = vec![output1, output2];

        let selected = vec![input1, input2, input3, input4];

        test_fee(HeadFirst::from(inputs), selected, outputs);
    }

    #[test]
    fn random_small_amount_ada_first_match_first_2() {
        let input1 = mk_icarus_style_input(Coin::new(3_000_000).unwrap());
        let input2 = mk_icarus_style_input(Coin::new(2_000_000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1_000_000).unwrap());

        let inputs = vec![input1.clone(), input2];
        let outputs = vec![output1];

        let selected = vec![input1];

        test_fee(HeadFirst::from(inputs), selected, outputs);
    }

    #[test]
    fn random_large_amount_ada_blackjack() {
        let input1 = mk_icarus_style_input(Coin::new(25_029_238_000000).unwrap());
        let input2 = mk_icarus_style_input(Coin::new(19_928_000000).unwrap());
        let input3 = mk_icarus_style_input(Coin::new(1_200000).unwrap());
        let input4 = mk_icarus_style_input(Coin::new(1_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(2_000000).unwrap());

        let inputs = vec![input1, input2, input3.clone(), input4.clone()];
        let outputs = vec![output1];

        let selected = vec![input4, input3];

        test_fee(
            Blackjack::new(Coin::from(150_000), inputs),
            selected,
            outputs,
        );
    }

    #[test]
    fn not_enough_ada_blackjack() {
        let input1 = mk_icarus_style_input(Coin::new(25_029_238_000000).unwrap());
        let input2 = mk_icarus_style_input(Coin::new(19_928_000000).unwrap());
        let input3 = mk_icarus_style_input(Coin::new(2_000000).unwrap());
        let input4 = mk_icarus_style_input(Coin::new(2_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1_000000).unwrap());

        let inputs = vec![input1, input2, input3.clone(), input4.clone()];
        let outputs = vec![output1];

        test_no_enough(Blackjack::new(Coin::from(150_000), inputs), outputs);
    }
}
