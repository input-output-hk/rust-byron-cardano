use cardano::{tx::{TxId, TxOut, TxIn, TxInWitness}, coin::{Coin}, address::{ExtendedAddr}};
use serde_yaml;

#[derive(Debug)]
pub enum ParsingOperationError {
    Yaml(String)
}

/// here are the operations that we will record in staging transactions.
///
/// By design we want the transaction to be stored as an append file only
/// this will allow us to guarantee that the transaction file is not
/// corrupted.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    AddInput(Input),
    AddOutput(Output),
    AddChange(Change),

    /// for now the unique identifier of a change address is the address itself
    RemoveChange(ExtendedAddr),

    /// use the unique identifier of the input. While as for the
    /// remove output we could use the index position within the
    /// collection of inputs it is better to identify the input
    /// as a unique txin for guarantee purpose.
    RemoveInput(TxIn),

    /// here we have chose to simply use the index of the output
    /// (i.e. the position order in which the output is within the
    /// sequential collection of outputs).debug
    ///
    /// We are not using the address as identifier as it would mean
    /// we consider the address to be used only once in the output
    /// of the transaction, but this is not the case, it is perfectly
    /// valid to have multiple times the same address in the same
    /// transaction's outputs.
    ///
    RemoveOutput(u32),

    /// add a transaction signature
    Signature(TxInWitness),

    /// operation to finalize a transaction
    Finalize
}
impl Operation {
    // For now, Operation will be serialized in YAML (thanks to serde).
    //
    // This will make parsing the data easier (we don't have to code any
    // custom format) and the debugging of the data too (we can open the file
    // and check what is being written)


    /// serialization of the operation within the transaction
    ///
    /// This is mainly for internal purpose only
    pub fn serialize(&self) -> Vec<u8> {
        serde_yaml::to_vec(self).unwrap()
    }

    /// deserialization of the operation within the transaction
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ParsingOperationError> {
        serde_yaml::from_slice(bytes).map_err(|e|
            ParsingOperationError::Yaml(format!("operation format error: {:?}", e))
        )
    }
}

/// Input within a transaction.
///
/// This structure shall contains enough information to collect
/// and credit transactions.
///
/// Along with the wallet's state, we can retrieve the desired
/// derivation path associated to the input address and sign the
/// transaction later on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    /// the transaction Id, along with the index in the transaction
    /// this will be enough to retrieve the exact transaction from
    /// the wallet logs
    pub transaction_id: TxId,

    /// the index in the transaction to use funds from.
    pub index_in_transaction: u32,

    /// the expected amount to spend
    pub expected_value: Coin
}
impl Input {
    /// collect the transaction input. By design this `TxIn` represents
    /// a unique identifier to the input funds (or the unspent transaction output)
    ///
    pub fn extract_txin(&self) -> TxIn {
        TxIn {
            id: self.transaction_id,
            index: self.index_in_transaction
        }
    }
}

/// the output of a given transaction, contains all the necessary details
/// to create the final transaction.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    /// the address we are sending funds to.
    pub address: ExtendedAddr,

    /// The desired amount to send to the associated address
    pub amount: Coin
}
impl From<Output> for TxOut {
    fn from(o: Output) -> Self {
        TxOut {
            address: o.address,
            value: o.amount
        }
    }
}
impl<'a> From<&'a Output> for TxOut {
    fn from(o: &'a Output) -> Self {
        TxOut {
            address: o.address.clone(),
            value: o.amount
        }
    }
}

/// a change address in the transaction model
///
/// TODO: adds support for percentage of the change to distribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// the address we are sending funds to.
    pub address: ExtendedAddr,
}
impl From<ExtendedAddr> for Change {
    fn from(o: ExtendedAddr) -> Self {
        Change { address: o }
    }
}
