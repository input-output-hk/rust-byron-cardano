use cardano::{tx::{TxId, TxIn}, coin::{Coin}, address::{ExtendedAddr}};
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
}
impl Operation {
    // For now, Operation will be serialised in YAML (thanks to serde).
    //
    // This will make parsing the data easier (we don't have to code any
    // custom format) and the debugging of the data too (we can open the file
    // and check what is being written)


    /// serialisation of the operation within the transaction
    ///
    /// This is mainly for internal purpose only
    pub fn serialize(&self) -> Vec<u8> {
        serde_yaml::to_vec(self).unwrap()
    }

    /// deserialisation of the operation within the transaction
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
    /// the name of the wallet the Input is associated from
    pub wallet: String,

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
    /// The wallet name where we are sending funds to.
    ///
    /// while not strictly necessary this will be handy to have for debug
    /// purpose.
    pub wallet: String,

    /// the address we are sending funds to.
    pub address: ExtendedAddr,

    /// The desired amount to send to the associated address
    pub amount: Coin
}
