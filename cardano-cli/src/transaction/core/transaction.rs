use super::{Operation, Input, Output};
use cardano::{tx::{TxIn}};

/// describe a transaction in its most reduce representation
///
/// Transaction are not meant to be edited from this representation
/// as this is a read only object.
///
/// There is 2 way to construct a transaction:
///
/// 1. by creating an empty transaction and updating it with operations;
/// 2. by collecting it from an iterator over `Operation` (see `FromIterator` trait);
///
/// Keeping private the transaction will allow us to control the state of the transaction
/// and to guarantee some levels of integrity (preventing errors).
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}
impl Transaction {
    /// create an empty transaction
    pub fn new() -> Self {
        Transaction {
            inputs: Vec::new(),
            outputs: Vec::new()
        }
    }

    /// update the transaction with the given operation
    pub fn update_with(&mut self, operation: Operation) -> &mut Self {
        match operation {
            Operation::AddInput(input)     => self.inputs.push(input),
            Operation::AddOutput(output)   => self.outputs.push(output),
            Operation::RemoveInput(txin)   => self.remove_input(txin),
            Operation::RemoveOutput(index) => self.remove_output(index)
        }

        self
    }

    /// accessor to all of the transaction's inputs.
    pub fn inputs<'a>(&'a self) -> &'a [Input] { self.inputs.as_ref() }

    /// accessor to all of the transaction's outputs. Ordered as it is in the
    /// transaction.
    pub fn outputs<'a>(&'a self) -> &'a [Output] { self.outputs.as_ref() }

    fn remove_output(&mut self, index: u32) {
        let output = self.outputs.remove(index as usize);

        debug!("removing outputs {:#?}", output);
    }

    /// lookup the inputs for the given `TxIn`
    pub fn lookup_input(&self, txin: TxIn) -> Option<usize> {
        self.inputs().iter().position(|input| &input.extract_txin() == &txin)
    }

    fn remove_input(&mut self, txin: TxIn) {
        // Here we could have used Drain Filter, but the feature is still not stable.
        // [see rust lang's issue #43244](https://github.com/rust-lang/rust/issues/43244).
        //
        // In the meanwhile the following is just as good.

        let mut index = 0;

        // we are not using `0..inputs.len()` because we are potentially removing
        // items as we go along
        while index != self.inputs.len() {
            if self.inputs[index].extract_txin() == txin {
                let input = self.inputs.remove(index);
                debug!("removing input: {:#?}", input);
            } else { index += 1; }
        }
    }
}
impl Default for Transaction {
    fn default() -> Self { Transaction::new() }
}
impl ::std::iter::FromIterator<Operation> for Transaction {
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = Operation>
    {
        let mut transaction = Self::default();
        iter.into_iter()
            .fold( &mut transaction
                 , |transaction, operation| transaction.update_with(operation)
                 );
        transaction
    }
}
