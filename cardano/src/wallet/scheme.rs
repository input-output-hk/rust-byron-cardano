//! wallet scheme interfaces. provide common interfaces to manage wallets
//! generate addresses and sign transactions.
//!

use tx::{self, TxId, TxOut, TxInWitness};
use fee;
use input_selection::{self, InputSelectionAlgorithm};
use txutils::{Input, OutputPolicy};
use coin::Coin;
use config::{ProtocolMagic};
use address::{ExtendedAddr};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum SelectionPolicy {
    /// select the first inputs that matches, no optimisation
    FirstMatchFirst
}
impl Default for SelectionPolicy {
    fn default() -> Self { SelectionPolicy::FirstMatchFirst }
}


/// main wallet scheme, provides all the details to manage a wallet:
/// from managing wallet [`Account`](./trait.Account.html)s and
/// signing transactions.
///
pub trait Wallet {
    /// associated `Account` type, must implement the [`Account`](./trait.Account.html)
    /// trait.
    type Account : Account;

    /// the associated type for the stored accounts. Some wallet may
    /// provide different model to handle accounts.
    ///
    type Accounts;

    /// addressing model associated to this wallet scheme.
    ///
    /// provides a description about how to derive a public key
    /// from a wallet point of view.
    type Addressing: Clone;

    /// create an account with the associated alias.
    ///
    /// The alias may not be used in some wallets which does not support
    /// accounts such as the daedalus wallet.
    ///
    fn create_account(&mut self, alias: &str, id: u32) -> Self::Account;

    /// list all the accounts known of this wallet
    fn list_accounts<'a>(&'a self) -> &'a Self::Accounts;
    fn sign_tx<I>(&self, protocol_magic: ProtocolMagic, txid: &TxId, addresses: I) -> Vec<TxInWitness>
        where I: Iterator<Item = Self::Addressing>;


    /// function to create a ready to send transaction to the network
    ///
    /// it select the needed inputs, compute the fee and possible change
    /// signes every TxIn as needed.
    ///
    fn new_transaction<'a, I>( &self
                             , protocol_magic: ProtocolMagic
                             , selection_policy: SelectionPolicy
                             , inputs: I
                             , outputs: Vec<TxOut>
                             , output_policy: &OutputPolicy
                             )
            -> input_selection::Result<(tx::TxAux, fee::Fee)>
        where I : 'a + Iterator<Item = &'a Input<Self::Addressing>> + ExactSizeIterator
            , Self::Addressing: 'a
    {
        let fee_alg = fee::LinearFee::default();

        let selection_result = match selection_policy {
            SelectionPolicy::FirstMatchFirst => {
                let inputs : Vec<Input<Self::Addressing>> = inputs.cloned().collect();
                let mut alg = input_selection::FirstMatchFirst::from(inputs);
                alg.compute(fee_alg, outputs.clone(), output_policy)?
            }
        };

        let mut tx = tx::Tx::new_with(
            selection_result.selected_inputs.iter().map(|input| input.ptr.clone()).collect(),
            outputs
        );

        if let Some(change) = selection_result.estimated_change {
            if change > Coin::zero() {
               match output_policy {
                   OutputPolicy::One(change_addr) =>
                       tx.add_output(tx::TxOut::new(change_addr.clone(), change)),
               };
           }
        }

        let witnesses = self.sign_tx(
            protocol_magic,
            &tx.id(),
            selection_result.selected_inputs.into_iter().map(|input| input.addressing)
        );

        Ok((tx::TxAux::new(tx, tx::TxWitness::from(witnesses)), selection_result.estimated_fees))
    }
}

/// account level scheme, provides all the details to manage an account:
/// i.e. generate new addresses associated to this account.
pub trait Account {
    /// addressing model associated to this account scheme.
    ///
    /// provides a description about how to derive a public key
    /// from a wallet point of view.
    type Addressing;

    fn generate_addresses<'a, I>(&'a self, addresses: I) -> Vec<ExtendedAddr>
        where I: Iterator<Item = &'a Self::Addressing>;
}
