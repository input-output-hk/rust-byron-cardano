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
use bip44;
use bip44::{Addressing, AddrType, BIP44_PURPOSE, BIP44_COIN_TYPE};
use tx::fee::SelectionAlgorithm;

use std::{result, fmt};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Error {
    FeeCalculationError(tx::fee::Error),
    AddressingError(bip44::Error),
    WalletError(hdwallet::Error)
}
impl From<tx::fee::Error> for Error {
    fn from(j: tx::fee::Error) -> Self { Error::FeeCalculationError(j) }
}
impl From<hdwallet::Error> for Error {
    fn from(j: hdwallet::Error) -> Self { Error::WalletError(j) }
}
impl From<bip44::Error> for Error {
    fn from(e: bip44::Error) -> Self { Error::AddressingError(e) }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::FeeCalculationError(err) => {
                write!(f, "Fee calculation error: {}", err)
            },
            &Error::AddressingError(err) => {
                write!(f, "Addressing error: {}", err)
            },
            &Error::WalletError(err) => {
                write!(f, "HD Wallet error: {}", err)
            }
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

/// the Wallet object
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Wallet {
    pub cached_root_key: hdwallet::XPrv,

    pub config: config::Config,
    pub selection_policy: tx::fee::SelectionPolicy,
    pub derivation_scheme: hdwallet::DerivationScheme,
}

impl Wallet {
    pub fn new(cached_root_key: hdwallet::XPrv, config: config::Config, policy: tx::fee::SelectionPolicy) -> Self {
        Wallet {
            cached_root_key: cached_root_key,
            config: config,
            selection_policy: policy,
            derivation_scheme: hdwallet::DerivationScheme::V2,
        }
    }

    /// create a new wallet from the given seed
    pub fn new_from_seed(seed: &hdwallet::Seed) -> Self {
        Self::new_from_root_xprv(hdwallet::XPrv::generate_from_seed(&seed))
    }

    pub fn new_from_root_xprv(key: hdwallet::XPrv) -> Self {
        let derivation_scheme = hdwallet::DerivationScheme::default();
        Wallet {
            cached_root_key: key.derive(derivation_scheme, BIP44_PURPOSE).derive(derivation_scheme, BIP44_COIN_TYPE),
            config: config::Config::default(),
            selection_policy: tx::fee::SelectionPolicy::default(),
            derivation_scheme
        }
    }

    /// create a new wallet from the given seed
    pub fn new_from_bip39(seed: &bip39::Seed) -> Self {
        Self::new_from_root_xprv(hdwallet::XPrv::generate_from_bip39(&seed))
    }

    pub fn account(&self, account_index: u32) -> Result<Account> {
        let account = bip44::Account::new(account_index)?;
        let account_key = self.get_root_key().derive(self.derivation_scheme, account.get_scheme_value()).public();

        Ok(Account::new(account, account_key, self.derivation_scheme))
    }

    /// create an extended address from the given addressing
    ///
    pub fn gen_addresses(&self, account: u32, addr_type: AddrType, indices: Vec<u32>) -> Result<Vec<address::ExtendedAddr>>
    {
        self.account(account)?.gen_addresses(addr_type, indices)
    }

    /// Create all the witness associated with each selected inputs
    /// for a specific already constructed Tx
    ///
    /// internal API
    fn sign_tx(&self, tx: &tx::Tx, selected_inputs: &tx::Inputs) -> Vec<tx::TxInWitness> {
        let mut witnesses = vec![];

        let txid = tx.id();

        for input in selected_inputs {
            let key  = self.get_xprv(&input.addressing);

            let txwitness = tx::TxInWitness::new(&self.config, &key, &txid);
            witnesses.push(txwitness);
        }
        witnesses
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

        let witnesses = self.sign_tx(&tx, &selected_inputs);

        Ok((tx::TxAux::new(tx, witnesses), fee))
    }

    pub fn verify_transaction(&self, inputs: &tx::Inputs, txaux: &tx::TxAux) -> bool {
        let tx = &txaux.tx;

        assert!(inputs.len() == txaux.witnesses.len());

        for i in 0..inputs.len() {
            let addr = &inputs.as_slice()[i].value.address;
            if ! txaux.witnesses[i].verify(&self.config, addr, tx) {
                return false;
            }
        }

        true
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
    pub fn get_xprv(&self, addressing: &Addressing) -> hdwallet::XPrv {
        self.get_root_key()
            .derive(self.derivation_scheme, addressing.account.get_scheme_value())
            .derive(self.derivation_scheme, addressing.change)
            .derive(self.derivation_scheme, addressing.index.get_scheme_value())
    }
}

/// Account associated to a given wallet.
///
/// Already contains the derived public key for the account of the wallet (see bip44).
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Account {
    pub account: bip44::Account,
    pub cached_account_key: hdwallet::XPub,
    pub derivation_scheme: hdwallet::DerivationScheme,
}
impl Account {
    pub fn new(account: bip44::Account, cached_account_key: hdwallet::XPub, derivation_scheme: hdwallet::DerivationScheme) -> Self {
        Account { account, cached_account_key, derivation_scheme }
    }

    /// create an extended address from the given addressing
    ///
    pub fn gen_addresses(&self, addr_type: AddrType, indices: Vec<u32>) -> Result<Vec<address::ExtendedAddr>>
    {
        let addressing = self.account.change(addr_type)?.index(0)?;

        let change_prv = self.cached_account_key
            .derive(self.derivation_scheme, addressing.change)?;

        let mut res = vec![];
        for index in indices {
            let pk = change_prv.derive(self.derivation_scheme, index)?;
            let addr_type = address::AddrType::ATPubKey;
            let sd = address::SpendingData::PubKeyASD(pk);
            let attrs = address::Attributes::new_bootstrap_era(None);
            res.push(address::ExtendedAddr::new(addr_type, sd, attrs));
        }
        Ok(res)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use address::ExtendedAddr;
    use tx;
    use coin;
    use serde_json;

    const WALLET_JSON : &str = "
{
  \"cached_root_key\": \"e006b83e6d823350bb8214db696b501b0b1545a953acf1dfde7e6e7e2434c245372ccc4f5e5788488fae9b5d4b8f6c1a6e74d6440949c02bac3238e4fffcc12de2cf47583ab3d30bc186db0b5ce47b91ed92090115775cb9b91532f28ca56875\",
  \"config\": {
    \"protocol_magic\": 633343913
  },
  \"selection_policy\": \"FirstMatchFirst\",
  \"derivation_scheme\": \"V2\"
}
    ";

    const INPUTS_JSON : &str = "
[{
  \"ptr\": {
    \"index\": 1,
    \"id\": \"5a32a92201e2fdd066559389223507bf0a0bdfab71c423fc3f25f95b93028d3a\"
  },
  \"value\": {
    \"address\": \"Ae2tdPwUPEZ6zd3CNebhUggZHVN1CzcP2uVdoGFAUcHaLGw3yf7gwVTXw44\",
    \"value\": 1000000
  },
  \"addressing\": {
    \"account\": 0,
    \"change\": 0,
    \"index\": 0
  }
}]
    ";

    const OUTPUTS_JSON : &str = "
[{
  \"address\": \"DdzFFzCqrhtB8bzt1u6zvhsMS2QsNLMssP3rCrjAiwRJj587seCpxzzsnzUMyVLUzkXSFfgm57dhBJyqA1JaVgC6cqdsvAuhdPTD476y\",
  \"value\": 1
}]
    ";

    const CHANGE_ADDR_JSON : &str = "\"Ae2tdPwUPEZ6zd3CNebhUggZHVN1CzcP2uVdoGFAUcHaLGw3yf7gwVTXw44\"";

    #[test]
    fn check_pk_witnesses_of_transaction() {
        let wallet : Wallet = serde_json::from_str(WALLET_JSON).unwrap();
        let inputs : tx::Inputs = serde_json::from_str(INPUTS_JSON).unwrap();
        let outputs : tx::Outputs = serde_json::from_str(OUTPUTS_JSON).unwrap();
        let change_addr : ExtendedAddr = serde_json::from_str(CHANGE_ADDR_JSON).unwrap();

        let (aux, _) = wallet.new_transaction(&inputs, &outputs, &change_addr).unwrap();

        assert!(wallet.verify_transaction(&inputs, &aux));
    }

    #[test]
    fn check_fee_transaction() {
        let wallet : Wallet = serde_json::from_str(WALLET_JSON).unwrap();
        let inputs : tx::Inputs = serde_json::from_str(INPUTS_JSON).unwrap();
        let outputs : tx::Outputs = serde_json::from_str(OUTPUTS_JSON).unwrap();
        let change_addr : ExtendedAddr = serde_json::from_str(CHANGE_ADDR_JSON).unwrap();

        let (aux, fee) = wallet.new_transaction(&inputs, &outputs, &change_addr).unwrap();

        let bytes = cbor!(&aux).unwrap();

        let expected = coin::Coin::new(bytes.len() as u64 * 44 + 155381).unwrap();

        println!("computed fee: {:?}", fee.to_coin());
        println!("expected fee: {:?}", expected);
        assert!(fee.to_coin() >= expected);
    }
}
