/// BIP44 derivation scheme and address model
///

use hdwallet::{Result, XPrv, XPub, DerivationScheme, DerivationIndex};
use bip::bip44::{self, BIP44_PURPOSE, BIP44_COIN_TYPE, BIP44_SOFT_UPPER_BOUND};
use bip::bip39;
use tx::{TxId, TxInWitness};
use address::{ExtendedAddr};
use config::Config;
use std::{ops::Deref, collections::{BTreeMap}};

use super::scheme::{self};

pub use self::bip44::{AddrType, Addressing, Account, Change, Index};

pub const DERIVATION_SCHEME : DerivationScheme = DerivationScheme::V2;

/// BIP44 based wallet, i.e. using sequential indexing.
///
/// See [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)
/// specifications for more details.
///
pub struct Wallet {
    cached_root_key: CoinLevel<XPrv>,
    accounts: BTreeMap<String, AccountLevel<XPrv>>,
    config: Config,
}
impl Wallet {
    /// load a wallet from a cached root key
    ///
    /// this is handy to reconstruct the wallet from a locally saved
    /// state (beware that the cached root key would need to be stored
    /// in a secure manner though).
    ///
    pub fn from_cached_key(cached_root_key: CoinLevel<XPrv>, config: Config) -> Self {
        let accounts = BTreeMap::new();
        Wallet {
            cached_root_key,
            accounts,
            config
        }
    }

    /// construct a new `Wallet` from the given Root key. Not really meant
    /// to reconstruct the wallet from locally saved state, but more to allow
    /// generating root seed without using bip39 mnemonics as proposed in
    /// [`Wallet::from_bip39_mnemonics`](./struct.Wallet.html#method.from_bip39_mnemonics)
    /// constructor.
    ///
    pub fn from_root_key(root_key: RootLevel<XPrv>, config: Config) -> Self {
        let cached_root_key = root_key.bip44().ada();
        Wallet::from_cached_key(cached_root_key, config)
    }

    /// helper to create a wallet from BIP39 Seed
    ///
    /// We assume the [`MnemonicString`](../../bip/bip39/struct.MnemonicString.html)
    /// so we don't have to handle error in this constructor.
    ///
    pub fn from_bip39_seed( seed: &bip39::Seed
                          , config: Config
                          ) -> Self
    {
        let xprv = XPrv::generate_from_bip39(seed);

        Wallet::from_root_key(RootLevel::from(xprv), config)
    }

    /// helper to create a wallet from BIP39 mnemonics
    ///
    /// We assume the [`MnemonicString`](../../bip/bip39/struct.MnemonicString.html)
    /// so we don't have to handle error in this constructor.
    ///
    pub fn from_bip39_mnemonics( mnemonics_phrase: &bip39::MnemonicString
                               , password: &[u8]
                               , config: Config
                               ) -> Self
    {
        let seed = bip39::Seed::from_mnemonic_string(mnemonics_phrase, password);

        Wallet::from_bip39_seed(&seed, config)
    }
}
impl AsRef<CoinLevel<XPrv>> for Wallet {
    fn as_ref(&self) -> &CoinLevel<XPrv> { &self.cached_root_key }
}
impl scheme::Wallet for Wallet {
    type Account     = AccountLevel<XPrv>;
    type Accounts    = BTreeMap<String, Self::Account>;
    type Addressing  = Addressing;

    fn create_account(&mut self, alias: &str, id: u32) -> Self::Account {
        let account = self.cached_root_key.account(id);
        self.accounts.insert(alias.to_owned(), account.clone());
        account
    }
    fn list_accounts<'a>(&'a self) -> &'a Self::Accounts  { &self.accounts }
    fn sign_tx<'a, I>(&'a self, txid: &TxId, addresses: I) -> Vec<TxInWitness>
        where I: Iterator<Item = &'a Self::Addressing>
    {
        let mut witnesses = vec![];

        for addressing in addresses {
            let key = self.cached_root_key
                          .account(addressing.account.get_scheme_value())
                          .change(addressing.address_type())
                          .index(addressing.index.get_scheme_value());

            let tx_witness = TxInWitness::new(&self.config, &key, txid);
            witnesses.push(tx_witness);
        }
        witnesses
    }
}
impl scheme::Account for AccountLevel<XPub> {
    type Addressing = (bip44::AddrType, u32);

    fn generate_addresses<'a, I>(&'a self, addresses: I) -> Vec<ExtendedAddr>
        where I: Iterator<Item = &'a Self::Addressing>
    {
        let (hint_low, hint_max) = addresses.size_hint();
        let mut vec = Vec::with_capacity(hint_max.unwrap_or(hint_low));

        for addressing in addresses {
            let key = self.change(addressing.0).expect("cannot fail")
                          .index(addressing.1).unwrap();
            let addr = ExtendedAddr::new_simple(key.0);
            vec.push(addr);
        }

        vec
    }
}
impl scheme::Account for AccountLevel<XPrv> {
    type Addressing = (bip44::AddrType, u32);

    fn generate_addresses<'a, I>(&'a self, addresses: I) -> Vec<ExtendedAddr>
        where I: Iterator<Item = &'a Self::Addressing>
    {
        let (hint_low, hint_max) = addresses.size_hint();
        let mut vec = Vec::with_capacity(hint_max.unwrap_or(hint_low));

        for addressing in addresses {
            let key = self.change(addressing.0)
                          .index(addressing.1)
                          .public();
            let addr = ExtendedAddr::new_simple(key.0);
            vec.push(addr);
        }

        vec
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootLevel<T>(T);
impl RootLevel<XPrv> {
    pub fn bip44(&self) -> PurposeLevel<XPrv> {
        PurposeLevel::from(self.0.derive(DERIVATION_SCHEME, BIP44_PURPOSE))
    }
}
impl<T> Deref for RootLevel<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}
impl From<XPrv> for RootLevel<XPrv> {
    fn from(xprv: XPrv) -> Self { RootLevel(xprv) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurposeLevel<T>(T);
impl PurposeLevel<XPrv> {
    pub fn ada(&self) -> CoinLevel<XPrv> {
        CoinLevel::from(self.0.derive(DERIVATION_SCHEME, BIP44_COIN_TYPE))
    }
}
impl<T> Deref for PurposeLevel<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}
impl From<XPrv> for PurposeLevel<XPrv> {
    fn from(xprv: XPrv) -> Self { PurposeLevel(xprv) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoinLevel<T>(T);
impl CoinLevel<XPrv> {
    pub fn account(&self, id: u32) -> AccountLevel<XPrv>
    {
        assert!(id < BIP44_SOFT_UPPER_BOUND);
        AccountLevel::from(self.0.derive(DERIVATION_SCHEME, BIP44_SOFT_UPPER_BOUND | id))
    }
}
impl<T> Deref for CoinLevel<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}
impl From<XPrv> for CoinLevel<XPrv> {
    fn from(xprv: XPrv) -> Self { CoinLevel(xprv) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountLevel<T>(T);
impl AccountLevel<XPrv> {
    pub fn external(&self) -> ChangeLevel<XPrv> {
        ChangeLevel::from(self.0.derive(DERIVATION_SCHEME, 0))
    }
    pub fn internal(&self) -> ChangeLevel<XPrv> {
        ChangeLevel::from(self.0.derive(DERIVATION_SCHEME, 1))
    }
    pub fn change(&self, addr_type: AddrType) -> ChangeLevel<XPrv> {
        match addr_type {
            AddrType::Internal => self.internal(),
            AddrType::External => self.external(),
        }
    }
    pub fn public(&self) -> AccountLevel<XPub> {
        AccountLevel::from(self.0.public())
    }
}
impl From<XPrv> for AccountLevel<XPrv> {
    fn from(xprv: XPrv) -> Self { AccountLevel(xprv) }
}
impl AccountLevel<XPub> {
    pub fn internal(&self) -> Result<ChangeLevel<XPub>> {
        Ok(ChangeLevel::from(self.0.derive(DERIVATION_SCHEME, 1)?))
    }
    pub fn external(&self) -> Result<ChangeLevel<XPub>> {
        Ok(ChangeLevel::from(self.0.derive(DERIVATION_SCHEME, 0)?))
    }
    pub fn change(&self, addr_type: AddrType) -> Result<ChangeLevel<XPub>> {
        match addr_type {
            AddrType::Internal => self.internal(),
            AddrType::External => self.external(),
        }
    }
}
impl From<XPub> for AccountLevel<XPub> {
    fn from(xpub: XPub) -> Self { AccountLevel(xpub) }
}
impl<T> Deref for AccountLevel<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeLevel<T>(T);
impl ChangeLevel<XPrv> {
    pub fn index(&self, index: DerivationIndex) -> IndexLevel<XPrv>
    {
        assert!(index < BIP44_SOFT_UPPER_BOUND);
        IndexLevel::from(self.0.derive(DERIVATION_SCHEME, index))
    }
    pub fn public(&self) -> ChangeLevel<XPub> {
        ChangeLevel::from(self.0.public())
    }
}
impl From<XPrv> for ChangeLevel<XPrv> {
    fn from(xprv: XPrv) -> Self { ChangeLevel(xprv) }
}
impl ChangeLevel<XPub> {
    pub fn index(&self, index: DerivationIndex) -> Result<IndexLevel<XPub>>
    {
        assert!(index < BIP44_SOFT_UPPER_BOUND);
        Ok(IndexLevel::from(self.0.derive(DERIVATION_SCHEME, index)?))
    }
}
impl From<XPub> for ChangeLevel<XPub> {
    fn from(xpub: XPub) -> Self { ChangeLevel(xpub) }
}
impl<T> Deref for ChangeLevel<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexLevel<T>(T);
impl IndexLevel<XPrv> {
    pub fn public(&self) -> IndexLevel<XPub> {
        IndexLevel::from(self.0.public())
    }
}
impl From<XPrv> for IndexLevel<XPrv> {
    fn from(xprv: XPrv) -> Self { IndexLevel(xprv) }
}
impl From<XPub> for IndexLevel<XPub> {
    fn from(xpub: XPub) -> Self { IndexLevel(xpub) }
}
impl<T> Deref for IndexLevel<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}
