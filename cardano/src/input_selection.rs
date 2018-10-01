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
