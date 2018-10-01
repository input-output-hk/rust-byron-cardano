use std::{fmt, result};
use coin::{self, Coin};
use tx::{TxOut};
use txutils::{Input, OutputPolicy, output_sum};
use txbuild::{self, TxBuilder};
use cbor_event;
use fee::{self, Fee, LinearFee};

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
        // kind reminder to update the algorithm once we start to implement
        // better kind of selection policy
        debug_assert!(
            policy == SelectionPolicy::FirstMatchFirst,
            "Only supported selection policy here is FirstMatchFirst"
        );

        // note: we cannot use `is_empty()` because it is an `ExactSizeIterator`
        //       and it does not expose this function directly.
        if inputs.len() == 0 { return Err(Error::NoInputs); }

        let mut selected = Vec::new();
        let mut builder = TxBuilder::new();
        for output in outputs { builder.add_output_value(output); }

        let mut change = Coin::zero();

        for input in inputs {
            selected.push(input);
            builder.add_input(&input.ptr, input.value.value);

            match builder.clone().add_output_policy(self, output_policy) {
                Err(txbuild::Error::TxNotEnoughTotalInput) => {
                    // here we don't have enough inputs, continue the loop
                    continue;
                },
                Err(txbuild_err) => {
                    return Err(Error::TxBuildError(txbuild_err));
                },
                Ok(outputs) => {
                    change = output_sum(outputs.iter())?;
                    break;
                }
            }
        }

        if let Err(error) = builder.add_output_policy(self, output_policy) {
            return match error {
                txbuild::Error::TxNotEnoughTotalInput => {
                    Err(Error::NotEnoughInput)
                },
                txbuild_err => {
                    Err(Error::TxBuildError(txbuild_err))
                },
            }
        }

        let fees = builder.calculate_fee(self).unwrap();

        Ok((fees, selected, change))
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

#[cfg(test)]
mod test {
    use super::*;
    use txutils::{Input};
    use address::{ExtendedAddr, AddrType, SpendingData, Attributes};
    use hdpayload::HDAddressPayload;
    use fee::FeeAlgorithm;
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

    fn test_fee_with(inputs: Vec<Input<()>>, outputs: Vec<TxOut>) {
        let change_address = mk_random_daedalus_style_address();

        let fee_alg = LinearFee::default();

        let (fee, selected, change) = fee_alg.compute(
            SelectionPolicy::FirstMatchFirst,
            inputs.iter(),
            outputs.iter(),
            &OutputPolicy::One(change_address.clone())
        ).unwrap();

        // check this is exactly the expected fee
        let mut tx = Tx::new_with(selected.iter().map(|input| input.ptr.clone()).collect(), outputs.clone());
        tx.add_output(TxOut::new(change_address, change));
        let witnesses : Vec<_> = ::std::iter::repeat(TxInWitness::fake()).take(selected.len()).collect();
        let expected_fee =  fee_alg.calculate_for_txaux_component(&tx, &witnesses).unwrap();
        assert_eq!(expected_fee, fee);

        // check the transaction is balanced
        let total_input = sum_coins(selected.iter().map(|input| input.value.value)).unwrap();
        let total_output = output_sum(tx.outputs.iter()).unwrap();
        let fee = fee.to_coin();
        assert_eq!(total_input, (total_output + fee).unwrap());
    }

    #[test]
    fn random_large_amount_ada() {
        let input1  = mk_icarus_style_input(Coin::new(25_029_238_000000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(                1).unwrap());

        let inputs = vec![input1];
        let outputs = vec![output1];

        test_fee_with(inputs, outputs);
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

        test_fee_with(inputs, outputs);
    }

    #[test]
    fn random_small_amount_ada_2() {
        let input1  = mk_icarus_style_input(Coin::new(3_000_000).unwrap());
        let input2  = mk_icarus_style_input(Coin::new(2_000_000).unwrap());
        let output1 = mk_icarus_style_txout(Coin::new(1_000_000).unwrap());

        let inputs = vec![input1, input2];
        let outputs = vec![output1];

        test_fee_with(inputs, outputs);
    }
}
