use address::ExtendedAddr;
use coin::{self, Coin};
use tx::*;

/// This is a TxoPointer with extra data associated:
///
/// * The number of coin associated for this utxo
/// * Optionally, way to derive the address for this TxoPointer
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct TxoPointerInfo<Addressing> {
    pub txin: TxoPointer,
    pub value: Coin,
    pub address_identified: Addressing,
}

/// Output Policy chosen.
///
/// For now this is just a placeholder of a single address,
/// but adding a ratio driven list of addresses seems
/// a useful flexibility to have
#[derive(Debug, Clone)]
pub enum OutputPolicy {
    One(ExtendedAddr),
}

/// This is a Resolved version of a `TxoPointer`.
///
/// It contains the `TxoPointer` which is the value we need to put in the
/// transaction to reference funds to input to the transation.
///
/// It also contains the `TxOut` the value present at the given
/// `TxoPointer`'s `TxId` and _index_ in the block chain.
///
#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct Input<Addressing> {
    pub ptr: TxoPointer,
    pub value: TxOut,
    pub addressing: Addressing,
}
impl<Addressing> Input<Addressing> {
    pub fn new(ptr: TxoPointer, value: TxOut, addressing: Addressing) -> Self {
        Input {
            ptr: ptr,
            value: value,
            addressing: addressing,
        }
    }

    pub fn value(&self) -> Coin {
        self.value.value
    }
}

pub fn output_sum<'a, O: 'a + Iterator<Item = &'a TxOut>>(o: O) -> coin::Result<Coin> {
    o.fold(Coin::new(0), |acc, ref c| acc.and_then(|v| v + c.value))
}
