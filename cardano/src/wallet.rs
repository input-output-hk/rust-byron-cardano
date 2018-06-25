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
use txutils;
use txutils::{OutputPolicy, TxInInfoAddr};
use config;
use bip39;
use bip44;
use bip44::{Addressing, AddrType, BIP44_PURPOSE, BIP44_COIN_TYPE};
use fee;
use fee::{SelectionAlgorithm, FeeAlgorithm};
use coin;

use std::{result, fmt, iter};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Error {
    FeeCalculationError(fee::Error),
    AddressingError(bip44::Error),
    WalletError(hdwallet::Error),
    CoinError(coin::Error),
}
impl From<fee::Error> for Error {
    fn from(j: fee::Error) -> Self { Error::FeeCalculationError(j) }
}
impl From<hdwallet::Error> for Error {
    fn from(j: hdwallet::Error) -> Self { Error::WalletError(j) }
}
impl From<bip44::Error> for Error {
    fn from(e: bip44::Error) -> Self { Error::AddressingError(e) }
}
impl From<coin::Error> for Error {
    fn from(j: coin::Error) -> Self { Error::CoinError(j) }
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
            &Error::CoinError(err) => {
                write!(f, "Coin error: {}", err)
            }
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

/// the Wallet object
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Wallet {
    pub root_key: hdwallet::XPrv,
    pub cached_root_key: hdwallet::XPrv,

    pub config: config::Config,
    pub selection_policy: fee::SelectionPolicy,
    pub derivation_scheme: hdwallet::DerivationScheme,
}

impl Wallet {
    #[deprecated]
    pub fn new(cached_root_key: hdwallet::XPrv, config: config::Config, policy: fee::SelectionPolicy) -> Self {
        Wallet {
            root_key: unimplemented!(),
            cached_root_key: cached_root_key,
            config: config,
            selection_policy: policy,
            derivation_scheme: hdwallet::DerivationScheme::V2,
        }
    }

    /// create a new wallet from the given seed
    pub fn new_from_seed(seed: &hdwallet::Seed) -> Self {
        Self::new_from_root_xprv(
            hdwallet::XPrv::generate_from_seed(&seed),
            hdwallet::DerivationScheme::default()
        )
    }

    pub fn new_from_root_xprv(key: hdwallet::XPrv, derivation_scheme: hdwallet::DerivationScheme) -> Self {
        let cached = key.derive(derivation_scheme, BIP44_PURPOSE).derive(derivation_scheme, BIP44_COIN_TYPE);
        Wallet {
            root_key: key,
            cached_root_key: cached,
            config: config::Config::default(),
            selection_policy: fee::SelectionPolicy::default(),
            derivation_scheme
        }
    }

    /// create a new wallet from the given seed
    pub fn new_from_bip39(seed: &bip39::Seed) -> Self {
        Self::new_from_root_xprv(
            hdwallet::XPrv::generate_from_bip39(&seed),
            hdwallet::DerivationScheme::default()
        )
    }

    pub fn account(&self, account_index: u32) -> Result<Account> {
        let account = bip44::Account::new(account_index)?;
        let account_key = self.get_cached_key().derive(self.derivation_scheme, account.get_scheme_value()).public();

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
    fn sign_tx_old(&self, tx: &tx::Tx, selected_inputs: &txutils::Inputs) -> Vec<tx::TxInWitness> {
        let mut witnesses = vec![];

        let txid = tx.id();

        for input in selected_inputs {
            let key  = self.get_bip44_xprv(&input.addressing);

            let txwitness = tx::TxInWitness::new(&self.config, &key, &txid);
            witnesses.push(txwitness);
        }
        witnesses
    }

    /// Create all the witness associated with each selected inputs
    /// for a specific already constructed Tx
    ///
    /// internal API
    fn sign_tx(&self, tx: &tx::Tx, selected_inputs: &Vec<txutils::TxInInfo>) -> Vec<tx::TxInWitness> {
        let mut witnesses = vec![];

        let txid = tx.id();

        for input in selected_inputs {
            let key = match input.address_identified {
                None => unimplemented!(),
                Some(ref addr) => {
                    match addr {
                        TxInInfoAddr::Bip44(ref addressing) => self.get_bip44_xprv(addressing),
                        TxInInfoAddr::Level2(ref addressing) => self.get_2level_xprv(addressing),
                    }
                }
            };

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
                          , inputs: &txutils::Inputs
                          , outputs: &Vec<tx::TxOut>
                          , output_policy: &txutils::OutputPolicy
                          )
        -> Result<(tx::TxAux, fee::Fee)>
    {
        let alg = fee::LinearFee::default();

        let (fee, selected_inputs, change) = alg.compute(self.selection_policy, inputs, outputs, output_policy)?;

        let mut tx = tx::Tx::new_with(
            selected_inputs.iter().cloned().map(|input| input.ptr).collect(),
            outputs.clone()
        );

        if change > Coin::zero() {
            match output_policy {
                OutputPolicy::One(change_addr) => tx.add_output(tx::TxOut::new(change_addr.clone(), change)),
            };
        }

        let witnesses = self.sign_tx_old(&tx, &selected_inputs);

        Ok((tx::TxAux::new(tx, witnesses), fee))
    }


    pub fn move_transaction(&self, inputs: &Vec<txutils::TxInInfo>, output_policy: &txutils::OutputPolicy) -> Result<(tx::TxAux, fee::Fee)> {

        if inputs.len() == 0 {
            return Err(Error::FeeCalculationError(fee::Error::NoInputs));
        }

        let alg = fee::LinearFee::default();

        let total_input : coin::Coin = {
            let mut total = coin::Coin::new(0)?;
            for ref i in inputs.iter() {
                let acc = total + i.value;
                total = acc?
            }
            total
        };

        let tx_base = tx::Tx::new_with( inputs.iter().cloned().map(|input| input.txin).collect()
                                      , vec![]);
        let fake_witnesses : Vec<tx::TxInWitness> = iter::repeat(tx::TxInWitness::fake()).take(inputs.len()).collect();
        let txaux_base = tx::TxAux::new(tx_base.clone(), fake_witnesses.clone());

        let min_fee_for_inputs = alg.calculate_for_txaux(&txaux_base)?.to_coin();
        let mut out_total = match total_input - min_fee_for_inputs {
            None => return Err(Error::FeeCalculationError(fee::Error::NotEnoughInput)),
            Some(c) => c, 
        };
        
        loop {
            let mut tx = tx_base.clone();
            match output_policy {
                OutputPolicy::One(change_addr) => {
                    let txout = tx::TxOut::new(change_addr.clone(), out_total);
                    tx.add_output(txout);
                },
            };

            let current_diff = (total_input - tx.get_output_total()).unwrap_or(coin::Coin::zero());
            let txaux = tx::TxAux::new(tx.clone(), fake_witnesses.clone());
            let txaux_fee : fee::Fee = alg.calculate_for_txaux(&txaux)?;
            println!("in total {} out total {} current diff {} txaux fee {}", total_input, out_total, current_diff, txaux_fee.to_coin());

            if current_diff == txaux_fee.to_coin() {
                let witnesses = self.sign_tx(&tx, &inputs);
                match total_input - tx.get_output_total() {
                    None => {},
                    Some(fee) => {
                        assert_eq!(witnesses.len(), fake_witnesses.len());
                        let txaux = tx::TxAux::new(tx, witnesses);
                        return Ok((txaux, txaux_fee))
                    },
                }
            } else {
                // already above..
                if current_diff > txaux_fee.to_coin() {
                    let r = (out_total + coin::Coin::new(1).unwrap())?;
                    out_total = r
                } else {
                    // not enough fee, so reduce the output_total
                    match out_total - coin::Coin::new(1).unwrap() {
                        None => return Err(Error::FeeCalculationError(fee::Error::NotEnoughInput)),
                        Some(o) => out_total = o,
                    }
                }
            }

        }
    }

    pub fn verify_transaction(&self, inputs: &txutils::Inputs, txaux: &tx::TxAux) -> bool {
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
    fn get_cached_key<'a>(&'a self) -> &'a hdwallet::XPrv {
        &self.cached_root_key
    }

    /// retrieve the root extended private key
    fn get_root_key<'a>(&'a self) -> &'a hdwallet::XPrv {
        &self.root_key
    }

    /// retrieve the key from the wallet and the given path
    ///
    /// TODO: this function is not meant to be public
    pub fn get_bip44_xprv(&self, addressing: &Addressing) -> hdwallet::XPrv {
        self.get_cached_key()
            .derive(self.derivation_scheme, addressing.account.get_scheme_value())
            .derive(self.derivation_scheme, addressing.change)
            .derive(self.derivation_scheme, addressing.index.get_scheme_value())
    }

    pub fn get_2level_xprv(&self, path: &[u32; 2]) -> hdwallet::XPrv {
        self.get_root_key()
            .derive(self.derivation_scheme, path[0])
            .derive(self.derivation_scheme, path[1])
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
    use txutils;
    use coin;
    use serde_json;
    use hash;
    use bip44::{Addressing, AddrType};

    const WALLET_JSON : &str = "
{
  \"root_key\":        \"e006b83e6d823350bb8214db696b501b0b1545a953acf1dfde7e6e7e2434c245372ccc4f5e5788488fae9b5d4b8f6c1a6e74d6440949c02bac3238e4fffcc12de2cf47583ab3d30bc186db0b5ce47b91ed92090115775cb9b91532f28ca56875\",
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
        let inputs : txutils::Inputs = serde_json::from_str(INPUTS_JSON).unwrap();
        let outputs : Vec<tx::TxOut> = serde_json::from_str(OUTPUTS_JSON).unwrap();
        let change_addr : ExtendedAddr = serde_json::from_str(CHANGE_ADDR_JSON).unwrap();

        let (aux, _) = wallet.new_transaction(&inputs, &outputs, &OutputPolicy::One(change_addr)).unwrap();

        assert!(wallet.verify_transaction(&inputs, &aux));
    }

    #[test]
    fn check_fee_transaction() {
        let wallet : Wallet = serde_json::from_str(WALLET_JSON).unwrap();
        let inputs : txutils::Inputs = serde_json::from_str(INPUTS_JSON).unwrap();
        let outputs : Vec<tx::TxOut> = serde_json::from_str(OUTPUTS_JSON).unwrap();
        let change_addr : ExtendedAddr = serde_json::from_str(CHANGE_ADDR_JSON).unwrap();

        let alg = fee::LinearFee::default();

        let (aux, fee) = wallet.new_transaction(&inputs, &outputs, &OutputPolicy::One(change_addr)).unwrap();

        let bytes = cbor!(&aux).unwrap();

        let expected = alg.estimate(bytes.len()).unwrap();

        println!("computed fee: {:?}", fee);
        println!("expected fee: {:?}", expected);
        assert!(fee >= expected);
    }

    #[test]
    fn check_move_transaction() {
        let wallet : Wallet = serde_json::from_str(WALLET_JSON).unwrap();
        let change_addr : ExtendedAddr = serde_json::from_str(CHANGE_ADDR_JSON).unwrap();
        let all_inputs = vec![
            txutils::TxInInfo {
                txin: tx::TxIn::new(hash::Blake2b256::new(&[1]), 0),
                value: coin::Coin::new(1000000).unwrap(),
                address_identified: Some(txutils::TxInInfoAddr::Bip44(Addressing::new(1, AddrType::Internal).unwrap())),
            },
            txutils::TxInInfo {
                txin: tx::TxIn::new(hash::Blake2b256::new(&[2]), 2),
                value: coin::Coin::new(3003030).unwrap(),
                address_identified: Some(txutils::TxInInfoAddr::Bip44(Addressing::new(2, AddrType::Internal).unwrap())),
            },
            txutils::TxInInfo {
                txin: tx::TxIn::new(hash::Blake2b256::new(&[3]), 4),
                value: coin::Coin::new(1003003030).unwrap(),
                address_identified: Some(txutils::TxInInfoAddr::Bip44(Addressing::new(2, AddrType::Internal).unwrap())),
            },
            txutils::TxInInfo {
                txin: tx::TxIn::new(hash::Blake2b256::new(&[4]), 6),
                value: coin::Coin::new(339).unwrap(),
                address_identified: Some(txutils::TxInInfoAddr::Bip44(Addressing::new(2, AddrType::Internal).unwrap())),
            },
            txutils::TxInInfo {
                txin: tx::TxIn::new(hash::Blake2b256::new(&[5]), 9),
                value: coin::Coin::new(23456789).unwrap(),
                address_identified: Some(txutils::TxInInfoAddr::Bip44(Addressing::new(10, AddrType::Internal).unwrap())),
            },
        ];

        for ti in 1..5 {
            let inputs = all_inputs.iter().cloned().take(ti).collect();
            let (aux, fee) = wallet.move_transaction(&inputs, &OutputPolicy::One(change_addr.clone())).unwrap(); 
            // verify fee is correct
            let alg = fee::LinearFee::default();
            assert_eq!(alg.calculate_for_txaux(&aux).unwrap(), fee)
        }

    }
}
