//! wallet scheme interfaces. provide common interfaces to manage wallets
//! generate addresses and sign transactions.
//!

use tx::{self, TxId, TxOut, TxInWitness};
use fee::{self, SelectionAlgorithm};
use txutils::{Input, OutputPolicy};
use coin::Coin;
use config::{ProtocolMagic};
use address::{ExtendedAddr};

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
    fn sign_tx<'a, I>(&'a self, protocol_magic: ProtocolMagic, txid: &TxId, addresses: I) -> Vec<TxInWitness>
        where I: Iterator<Item = &'a Self::Addressing>;


    /// function to create a ready to send transaction to the network
    ///
    /// it select the needed inputs, compute the fee and possible change
    /// signes every TxIn as needed.
    ///
    fn new_transaction<'a, I>( &self
                             , protocol_magic: ProtocolMagic
                             , selection_policy: fee::SelectionPolicy
                             , inputs: I
                             , outputs: Vec<TxOut>
                             , output_policy: &OutputPolicy
                             )
            -> fee::Result<(tx::TxAux, fee::Fee)>
        where I : 'a + Iterator<Item = &'a Input<Self::Addressing>> + ExactSizeIterator
            , Self::Addressing: 'a
    {
        let alg = fee::LinearFee::default();

        let (fee, selected_inputs, change)
            = alg.compute(selection_policy, inputs, outputs.iter(), output_policy)?;

        let addressings : Vec<Self::Addressing>
            = selected_inputs.iter().map(|si| si.addressing.clone()).collect();

        let mut tx = tx::Tx::new_with(
            selected_inputs.iter().map(|input| input.ptr.clone()).collect(),
            outputs
        );

        if change > Coin::zero() {
            match output_policy {
                OutputPolicy::One(change_addr) =>
                    tx.add_output(tx::TxOut::new(change_addr.clone(), change)),
            };
        }

        let witnesses = self.sign_tx(protocol_magic, &tx.id(), addressings.iter());

        Ok((tx::TxAux::new(tx, tx::TxWitness::from(witnesses)), fee))
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
