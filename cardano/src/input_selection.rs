use std::{fmt, result, ops::{Deref, DerefMut}};
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

pub struct InputSelectionResult<Addressing> {
    pub estimated_fees: Fee,
    pub estimated_change: Option<Coin>,
    pub selected_inputs: Vec<Input<Addressing>>
}

pub trait InputSelectionAlgorithm<Addressing> {
    fn select_input( &mut self
                   , estimated_needed_output: Coin
                   )
        -> Result<Option<Input<Addressing>>>;

    fn compute<F: FeeAlgorithm>( &mut self
                               , fee_algorithm: F
                               , outputs: Vec<TxOut>
                               , output_policy: &OutputPolicy
                               )
        -> Result<InputSelectionResult<Addressing>>
    {
        let mut selected = Vec::new();
        let mut builder = TxBuilder::new();
        for output in outputs { builder.add_output_value(&output); }

        let total_output = builder.get_output_total().unwrap();

        while let Some(input) = self.select_input(total_output)? {
            builder.add_input(&input.ptr, input.value.value);
            selected.push(input);

            match builder.clone().add_output_policy(&fee_algorithm, output_policy) {
                Err(txbuild::Error::TxNotEnoughTotalInput) => {
                    // here we don't have enough inputs, continue the loop
                    continue;
                },
                Err(txbuild_err) => { return Err(Error::TxBuildError(txbuild_err)); },
                Ok(_) => { break; }
            }
        }

        match builder.add_output_policy(&fee_algorithm, output_policy) {
            Err(txbuild::Error::TxNotEnoughTotalInput) => {
                Err(Error::NotEnoughInput)
            },
            Err(txbuild_err) => {
                Err(Error::TxBuildError(txbuild_err))
            },
            Ok(change_outputs) => {
                let fees = builder.calculate_fee(&fee_algorithm).unwrap();
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

#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct FirstMatchFirst<Addressing>(Vec<Input<Addressing>>);
impl<Addressing> From<Vec<Input<Addressing>>> for FirstMatchFirst<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self { FirstMatchFirst(inputs) }
}
impl<Addressing> Deref for FirstMatchFirst<Addressing> {
    type Target = Vec<Input<Addressing>>;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl<Addressing> DerefMut for FirstMatchFirst<Addressing> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for FirstMatchFirst<Addressing> {
    fn select_input( &mut self
                   , _estimated_needed_output: Coin
                   )
        -> Result<Option<Input<Addressing>>>
    {
        Ok(self.pop())
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

    fn test_fee_with_first_match_first(inputs: Vec<Input<()>>, outputs: Vec<TxOut>) {
        let change_address = mk_random_daedalus_style_address();
        let mut input_selection_scheme = FirstMatchFirst::from(inputs);

        let fee_alg = LinearFee::default();

        let input_selection_result = input_selection_scheme.compute(
            LinearFee::default(),
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
    }

    #[test]
    fn random_large_amount_ada() {
        let input1  = mk_icarus_style_input(Coin::new(25_029_238_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(                1).unwrap());

        let inputs = vec![input1];
        let outputs = vec![output1];

        test_fee_with_first_match_first(inputs, outputs);
    }

    #[test]
    fn random_small_amount_ada() {
        let input1  = mk_daedalus_style_input(Coin::new(1).unwrap());
        let input2  = mk_daedalus_style_input(Coin::new(2).unwrap());
        let input3  = mk_icarus_style_input(Coin::new(3).unwrap());
        let input4  = mk_daedalus_style_input(Coin::new(1_000_000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1).unwrap());
        let output2 = mk_daedalus_style_txout(Coin::new(2).unwrap());

        let inputs = vec![input1, input2, input3, input4];
        let outputs = vec![output1, output2];

        test_fee_with_first_match_first(inputs, outputs);
    }

    #[test]
    fn random_small_amount_ada_2() {
        let input1  = mk_icarus_style_input(Coin::new(3_000_000).unwrap());
        let input2  = mk_icarus_style_input(Coin::new(2_000_000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1_000_000).unwrap());

        let inputs = vec![input1, input2];
        let outputs = vec![output1];

        test_fee_with_first_match_first(inputs, outputs);
    }
}
