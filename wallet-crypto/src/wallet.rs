//! unlike the hdwallet object, this the stateful wallet implementation
//!
//! # definition
//!
//! While other modules tries to be stateless as much as possible
//! here we want to provide all the logic one may want from a wallet.
//!

use hdwallet;
use address;
use tx;
use config;
use bip39;
use bip44::{Addressing, AddrType, BIP44_PURPOSE, BIP44_COIN_TYPE};
use tx::fee::Algorithm;

use std::{result, fmt};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Error {
    FeeCalculationError(tx::fee::Error),
    WalletError(hdwallet::Error)
}
impl From<tx::fee::Error> for Error {
    fn from(j: tx::fee::Error) -> Self { Error::FeeCalculationError(j) }
}
impl From<hdwallet::Error> for Error {
    fn from(j: hdwallet::Error) -> Self { Error::WalletError(j) }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::FeeCalculationError(err) => {
                write!(f, "Fee calculation error: {}", err)
            },
            &Error::WalletError(err) => {
                write!(f, "HD Wallet error: {}", err)
            }
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

/// the Wallet object
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Wallet {
    cached_root_key: hdwallet::XPrv,

    config: config::Config,
    selection_policy: tx::fee::SelectionPolicy,
}

impl Wallet {
    /// generate a new wallet
    ///
    pub fn new() -> Self { unimplemented!() }

    /// create a new wallet from the given seed
    pub fn new_from_seed(seed: &hdwallet::Seed) -> Self {
        Self::new_from_root_xprv(hdwallet::XPrv::generate_from_seed(&seed))
    }

    pub fn new_from_root_xprv(key: hdwallet::XPrv) -> Self {
        Wallet {
            cached_root_key: key.derive(BIP44_PURPOSE).derive(BIP44_COIN_TYPE),
            config: config::Config::default(),
            selection_policy: tx::fee::SelectionPolicy::default()
        }
    }

    /// create a new wallet from the given seed
    pub fn new_from_bip39(seed: &bip39::Seed) -> Self {
        Self::new_from_root_xprv(hdwallet::XPrv::generate_from_bip39(&seed))
    }

    pub fn account(&self, account: u32) -> Account {
        let account_key = self.get_root_key().derive(account).public();

        Account::new(account, account_key)
    }

    /// create an extended address from the given addressing
    ///
    pub fn gen_addresses(&self, account: u32, addr_type: AddrType, indices: Vec<u32>) -> Vec<address::ExtendedAddr>
    {
        let addressing = Addressing::new(account, addr_type).unwrap();

        let change_prv = self.get_root_key()
            .derive(addressing.account)
            .derive(addressing.change);

        let mut res = vec![];
        for index in indices {
            let pk = change_prv.derive(index).public();
            let addr_type = address::AddrType::ATPubKey;
            let sd = address::SpendingData::PubKeyASD(pk);
            let attrs = address::Attributes::new_bootstrap_era(None);
            res.push(address::ExtendedAddr::new(addr_type, sd, attrs));
        }
        res
    }

    /// function to create a ready to send transaction to the network
    ///
    /// it select the needed inputs, compute the fee and possible change
    /// signes every TxIn as needed.
    ///
    pub fn new_transaction( &self
                          , inputs: &tx::Inputs
                          , outputs: &tx::Outputs
                          , change_addr: &address::ExtendedAddr
                          )
        -> Result<(tx::TxAux, tx::fee::Fee)>
    {
        let alg = tx::fee::LinearFee::default();

        let (fee, selected_inputs, change) = alg.compute(self.selection_policy, inputs, outputs, change_addr)?;

        let mut tx = tx::Tx::new_with(
            selected_inputs.iter().cloned().map(|input| input.ptr).collect(),
            outputs.iter().cloned().collect()
        );

        tx.add_output(tx::TxOut::new(change_addr.clone(), change));

        let mut witnesses = vec![];

        for input in selected_inputs {
            let key  = self.get_xprv(&input.addressing);

            witnesses.push(tx::TxInWitness::new(&self.config, &key, &tx));
        }

        Ok((tx::TxAux::new(tx, witnesses), fee))
    }

    /// retrieve the root extended private key from the wallet but pre
    /// derived for the purpose and coin type.
    ///
    /// TODO: this function is not meant to be public
    fn get_root_key<'a>(&'a self) -> &'a hdwallet::XPrv {
        &self.cached_root_key
    }

    /// retrieve the key from the wallet and the given path
    ///
    /// TODO: this function is not meant to be public
    fn get_xprv(&self, addressing: &Addressing) -> hdwallet::XPrv {
        self.get_root_key()
            .derive(addressing.account)
            .derive(addressing.change)
            .derive(addressing.index)
    }
}

/// Account associated to a given wallet.
///
/// Already contains the derived public key for the account of the wallet (see bip44).
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Account {
    account: u32,
    cached_account_key: hdwallet::XPub
}
impl Account {
    fn new(account: u32, xpub: hdwallet::XPub) -> Self { Account { account: account, cached_account_key: xpub } }

    /// create an extended address from the given addressing
    ///
    pub fn gen_addresses(&self, addr_type: AddrType, indices: Vec<u32>) -> Result<Vec<address::ExtendedAddr>>
    {
        let addressing = Addressing::new(self.account, addr_type).unwrap();

        let change_prv = self.cached_account_key
            .derive(addressing.change)?;

        let mut res = vec![];
        for index in indices {
            let pk = change_prv.derive(index)?;
            let addr_type = address::AddrType::ATPubKey;
            let sd = address::SpendingData::PubKeyASD(pk);
            let attrs = address::Attributes::new_bootstrap_era(None);
            res.push(address::ExtendedAddr::new(addr_type, sd, attrs));
        }
        Ok(res)
    }
}