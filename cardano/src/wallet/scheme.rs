//! wallet scheme interfaces. provide common interfaces to manage wallets
//! generate addresses and sign transactions.
//!

use address::ExtendedAddr;
use coin::Coin;
use config::{NetworkMagic, ProtocolMagic};
use fee::{self, FeeAlgorithm};
use input_selection::{self, InputSelectionAlgorithm};
use tx::{self, TxId, TxInWitness, TxOut};
use txbuild::{self, TxBuilder, TxFinalized};
use txutils::{Input, OutputPolicy};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum SelectionPolicy {
    /// select the first inputs that matches, no optimization
    FirstMatchFirst,

    /// Order the given inputs from the largest input and pick the largest ones first
    LargestFirst,

    /// select only the inputs that are below the targeted output
    ///
    /// the value in this setting represents the accepted dust threshold
    /// to lose or ignore in fees.
    Blackjack(Coin),
}
impl Default for SelectionPolicy {
    fn default() -> Self {
        SelectionPolicy::FirstMatchFirst
    }
}

/// main wallet scheme, provides all the details to manage a wallet:
/// from managing wallet [`Account`](./trait.Account.html)s and
/// signing transactions.
///
pub trait Wallet {
    /// associated `Account` type, must implement the [`Account`](./trait.Account.html)
    /// trait.
    type Account: Account;

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
    fn sign_tx<I>(
        &self,
        protocol_magic: ProtocolMagic,
        txid: &TxId,
        addresses: I,
    ) -> Vec<TxInWitness>
    where
        I: Iterator<Item = Self::Addressing>;

    /// function to create a ready to send transaction to the network
    ///
    /// it select the needed inputs, compute the fee and possible change
    /// signes every TxIn as needed.
    ///
    fn new_transaction<'a, I>(
        &self,
        protocol_magic: ProtocolMagic,
        selection_policy: SelectionPolicy,
        inputs: I,
        outputs: Vec<TxOut>,
        output_policy: &OutputPolicy,
    ) -> input_selection::Result<(tx::TxAux, fee::Fee)>
    where
        I: 'a + Iterator<Item = &'a Input<Self::Addressing>> + ExactSizeIterator,
        Self::Addressing: 'a,
    {
        let fee_alg = fee::LinearFee::default();

        let selection_result = match selection_policy {
            SelectionPolicy::FirstMatchFirst => {
                let inputs: Vec<Input<Self::Addressing>> = inputs.cloned().collect();
                let mut alg = input_selection::HeadFirst::from(inputs);
                alg.compute(&fee_alg, outputs.clone(), output_policy)?
            }
            SelectionPolicy::LargestFirst => {
                let inputs: Vec<Input<Self::Addressing>> = inputs.cloned().collect();
                let mut alg = input_selection::LargestFirst::from(inputs);
                alg.compute(&fee_alg, outputs.clone(), output_policy)?
            }
            SelectionPolicy::Blackjack(dust) => {
                let inputs: Vec<Input<Self::Addressing>> = inputs.cloned().collect();
                let mut alg = input_selection::Blackjack::new(dust, inputs);
                alg.compute(&fee_alg, outputs.clone(), output_policy)?
            }
        };

        let mut txbuilder = TxBuilder::new();
        for input in selection_result.selected_inputs.iter() {
            txbuilder.add_input(&input.ptr, input.value.value)
        }
        for output in outputs.iter() {
            txbuilder.add_output_value(output);
        }

        // here we try to add the output policy, if it didn't work because
        // the amount of coin leftover is not enough to add the policy, then
        // we ignore the error
        match txbuilder.add_output_policy(&fee_alg, output_policy) {
            Err(txbuild::Error::TxOutputPolicyNotEnoughCoins(_)) => {}
            Err(e) => return Err(input_selection::Error::TxBuildError(e)),
            Ok(_) => {}
        };

        let tx = txbuilder
            .make_tx()
            .map_err(input_selection::Error::TxBuildError)?;
        let txid = tx.id();
        let mut txfinalized = TxFinalized::new(tx);

        let witnesses = self.sign_tx(
            protocol_magic,
            &txid,
            selection_result
                .selected_inputs
                .into_iter()
                .map(|input| input.addressing),
        );

        for witness in witnesses {
            txfinalized
                .add_witness(witness)
                .map_err(input_selection::Error::TxBuildError)?;
        }

        let txaux = txfinalized
            .make_txaux()
            .map_err(input_selection::Error::TxBuildError)?;

        let real_fee = fee_alg
            .calculate_for_txaux(&txaux)
            .map_err(input_selection::Error::FeeError)?;

        if real_fee > selection_result.estimated_fees {
            Err(input_selection::Error::NotEnoughFees)
        } else {
            Ok((txaux, selection_result.estimated_fees))
        }
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

    fn generate_addresses<'a, I>(
        &'a self,
        addresses: I,
        network_magic: NetworkMagic,
    ) -> Vec<ExtendedAddr>
    where
        I: Iterator<Item = &'a Self::Addressing>;
}
