use tx::*;
use hdpayload;
use coin::{self, Coin};
use address::{ExtendedAddr};

/// This is a TxIn with extra data associated:
///
/// * The number of coin associated for this utxo
/// * Optionally, way to derive the address for this txin
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct TxInInfo<Addressing> {
    pub txin: TxIn,
    pub value: Coin,
    pub address_identified: Addressing,
}

/// Output Policy chosen.
///
/// For now this is just a placeholder of a single address,
/// but adding a ratio driven list of addresses seems
/// a useful flexibility to have
pub enum OutputPolicy {
    One(ExtendedAddr),
}

/// This is a Resolved version of a `TxIn`.
///
/// It contains the `TxIn` which is the value we need to put in the
/// transaction to reference funds to input to the transation.
///
/// It also contains the `TxOut` the value present at the given
/// `TxIn`'s `TxId` and _index_ in the block chain.
///
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Input<Addressing> {
    pub ptr:   TxIn,
    pub value: TxOut,
    pub addressing: Addressing,
}
impl<Addressing> Input<Addressing> {
    pub fn new(ptr: TxIn, value: TxOut, addressing: Addressing) -> Self
    { Input { ptr: ptr, value: value, addressing: addressing } }

    pub fn value(&self) -> Coin { self.value.value }

    pub fn get_derivation_path(&self, key: &hdpayload::HDKey) -> Option<hdpayload::Path> {
        match &self.value.address.attributes.derivation_path {
            &Some(ref payload) => { key.decrypt_path(payload) },
            &None              => { None }
        }
    }
}

pub fn output_sum<'a, O: 'a + Iterator<Item = &'a TxOut>>(o: O) -> coin::Result<Coin> {
    o.fold(Coin::new(0), |acc, ref c| acc.and_then(|v| v + c.value))
}
