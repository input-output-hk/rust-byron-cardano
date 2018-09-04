use cardano::{coin::{Coin}, tx::{TxIn, TxId, TxOut}, address::{ExtendedAddr}};
use std::{fmt, collections::{BTreeMap}};

/// Unspent Transaction Output (aka. UTxO). This is a transaction
/// that may be spent, that is, as far as known of the state of the
/// wallet, unspent yet.
///
/// The type parameter of this structure represents the address,
/// known by the wallet or as known by the blockchain (i.e. anonymized
/// in the original format.)
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTxO<A> {
    /// in which transaction of the blockchain the unspent output (TxOut)
    /// is.
    pub transaction_id: TxId,
    /// a Transaction is a list of inputs and outputs. `index_in_transaction`
    /// is the index within the outputs of the unspent transaction.
    pub index_in_transaction: u32,

    /// this is the credited address, it can have multiple forms:
    ///
    /// * it can be the `ExtendedAddr`, as seen raw in the blockchain,
    /// * it can be the _derivation path_ as known by the wallet.
    ///
    /// This double representation will allow to create strongly typed
    /// representation of the UTxO blockchain as it should be known by
    /// the wallet or by other tool that could be working on the transactions
    /// without needing to use the fund credited in this `UTxO`.
    ///
    pub credited_address: ExtendedAddr,

    pub credited_addressing: A,

    /// the amount credited in this `UTxO`
    pub credited_value: Coin,
}
impl<A> UTxO<A> {
    /// extract the `TxIn` from the `UTxO`. The output `TxIn` is meant to
    /// be used in a new transaction, and to spend the fund credited
    /// by this `UTxO`
    pub fn extract_txin(&self) -> TxIn {
        TxIn {
            id: self.transaction_id,
            index: self.index_in_transaction
        }
    }

    pub fn map<B, F>(self, f: F) -> UTxO<B>
        where F: FnOnce(A) -> B
    {
        UTxO {
            transaction_id: self.transaction_id,
            index_in_transaction: self.index_in_transaction,
            credited_value: self.credited_value,
            credited_addressing: f(self.credited_addressing),
            credited_address: self.credited_address
        }
    }

    /// This `TxOut` is equal to the one that can be found in the original
    /// blockchain.
    pub fn extract_txout(&self) -> TxOut {
        TxOut {
            address: self.credited_address.clone(),
            value:   self.credited_value,
        }
    }
}
impl<A: fmt::Display> fmt::Display for UTxO<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!( f
              , "{} received {}Ada-Lovelace in transaction id `{}.{}'"
              , self.credited_address
              , self.credited_value
              , self.transaction_id
              , self.index_in_transaction
              )
    }
}

/// collections for quick lookup of `UTxO` by `TxId`
pub type UTxOs<A> = BTreeMap<TxIn, UTxO<A>>;
