/// BIP44 derivation scheme and address model
///

use hdwallet::{Result, XPrv, XPub, DerivationScheme, DerivationIndex};
use bip::bip44::{BIP44_PURPOSE, BIP44_COIN_TYPE, BIP44_SOFT_UPPER_BOUND};
use bip::bip39;
use tx::{TxId, TxInWitness};
use address::{ExtendedAddr};
use config::Config;
use std::{ops::Deref, collections::{BTreeMap}};

use super::scheme::{self};

pub use bip::bip44::{self, AddrType, Addressing, Change, Index};

/// BIP44 based wallet, i.e. using sequential indexing.
///
/// See [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)
/// specifications for more details.
///
pub struct Wallet {
    cached_root_key: RootLevel<XPrv>,
    accounts: BTreeMap<String, Account<XPrv>>,
    config: Config,
    derivation_scheme: DerivationScheme,
}
impl Wallet {
    /// load a wallet from a cached root key
    ///
    /// this is handy to reconstruct the wallet from a locally saved
    /// state (beware that the cached root key would need to be stored
    /// in a secure manner though).
    ///
    pub fn from_cached_key(cached_root_key: RootLevel<XPrv>, derivation_scheme: DerivationScheme, config: Config) -> Self {
        let accounts = BTreeMap::new();
        Wallet {
            cached_root_key,
            accounts,
            config,
            derivation_scheme
        }
    }

    /// construct a new `Wallet` from the given Root key. Not really meant
    /// to reconstruct the wallet from locally saved state, but more to allow
    /// generating root seed without using bip39 mnemonics as proposed in
    /// [`Wallet::from_bip39_mnemonics`](./struct.Wallet.html#method.from_bip39_mnemonics)
    /// constructor.
    ///
    pub fn from_root_key(root_key: XPrv, derivation_scheme: DerivationScheme, config: Config) -> Self {
        let cached_root_key = root_key.derive(derivation_scheme, BIP44_PURPOSE)
                                      .derive(derivation_scheme, BIP44_COIN_TYPE);
        Wallet::from_cached_key(RootLevel::from(cached_root_key), derivation_scheme, config)
    }

    /// helper to create a wallet from BIP39 Seed
    ///
    /// We assume the [`MnemonicString`](../../bip/bip39/struct.MnemonicString.html)
    /// so we don't have to handle error in this constructor.
    ///
    pub fn from_bip39_seed( seed: &bip39::Seed
                          , derivation_scheme: DerivationScheme
                          , config: Config
                          ) -> Self
    {
        let xprv = XPrv::generate_from_bip39(seed);

        Wallet::from_root_key(xprv, derivation_scheme, config)
    }

    /// helper to create a wallet from BIP39 mnemonics
    ///
    /// We assume the [`MnemonicString`](../../bip/bip39/struct.MnemonicString.html)
    /// so we don't have to handle error in this constructor.
    ///
    pub fn from_bip39_mnemonics( mnemonics_phrase: &bip39::MnemonicString
                               , password: &[u8]
                               , derivation_scheme: DerivationScheme
                               , config: Config
                               ) -> Self
    {
        let seed = bip39::Seed::from_mnemonic_string(mnemonics_phrase, password);

        Wallet::from_bip39_seed(&seed, derivation_scheme, config)
    }

    pub fn derivation_scheme(&self) -> DerivationScheme { self.derivation_scheme }
}
impl Deref for Wallet {
    type Target = RootLevel<XPrv>;
    fn deref(&self) -> &Self::Target { &self.cached_root_key }
}
impl scheme::Wallet for Wallet {
    type Account     = Account<XPrv>;
    type Accounts    = BTreeMap<String, Self::Account>;
    type Addressing  = Addressing;

    fn create_account(&mut self, alias: &str, id: u32) -> Self::Account {
        let account = self.cached_root_key.account(self.derivation_scheme, id);
        let account = Account { cached_root_key: account, derivation_scheme: self.derivation_scheme };
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
                          .account(self.derivation_scheme, addressing.account.get_scheme_value())
                          .change(self.derivation_scheme, addressing.address_type())
                          .index(self.derivation_scheme, addressing.index.get_scheme_value());

            let tx_witness = TxInWitness::new(&self.config, &key, txid);
            witnesses.push(tx_witness);
        }
        witnesses
    }
}

#[derive(Clone)]
pub struct Account<K> {
    cached_root_key: AccountLevel<K>,
    derivation_scheme: DerivationScheme
}
impl<K> Account<K> {
    pub fn new(cached_root_key: AccountLevel<K>, derivation_scheme: DerivationScheme) -> Self {
        Account { cached_root_key, derivation_scheme }
    }
}
impl Account<XPrv> {
    pub fn public(&self) -> Account<XPub> {
        Account {
            cached_root_key: self.cached_root_key.public(),
            derivation_scheme: self.derivation_scheme
        }
    }
}
impl Deref for Account<XPrv> {
    type Target = AccountLevel<XPrv>;
    fn deref(&self) -> &Self::Target { &self.cached_root_key }
}
impl Deref for Account<XPub> {
    type Target = AccountLevel<XPub>;
    fn deref(&self) -> &Self::Target { &self.cached_root_key }
}
impl scheme::Account for Account<XPub> {
    type Addressing = (bip44::AddrType, u32);

    fn generate_addresses<'a, I>(&'a self, addresses: I) -> Vec<ExtendedAddr>
        where I: Iterator<Item = &'a Self::Addressing>
    {
        let (hint_low, hint_max) = addresses.size_hint();
        let mut vec = Vec::with_capacity(hint_max.unwrap_or(hint_low));

        for addressing in addresses {
            let key = self.cached_root_key
                          .change(self.derivation_scheme, addressing.0).expect("cannot fail")
                          .index(self.derivation_scheme, addressing.1).unwrap();
            let addr = ExtendedAddr::new_simple(key.0);
            vec.push(addr);
        }

        vec
    }
}
impl scheme::Account for Account<XPrv> {
    type Addressing = (bip44::AddrType, u32);

    fn generate_addresses<'a, I>(&'a self, addresses: I) -> Vec<ExtendedAddr>
        where I: Iterator<Item = &'a Self::Addressing>
    {
        let (hint_low, hint_max) = addresses.size_hint();
        let mut vec = Vec::with_capacity(hint_max.unwrap_or(hint_low));

        for addressing in addresses {
            let key = self.cached_root_key
                          .change(self.derivation_scheme, addressing.0)
                          .index(self.derivation_scheme, addressing.1)
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
    pub fn account(&self, derivation_scheme: DerivationScheme, id: u32) -> AccountLevel<XPrv>
    {
        assert!(id < BIP44_SOFT_UPPER_BOUND);
        AccountLevel::from(self.0.derive(derivation_scheme, BIP44_SOFT_UPPER_BOUND | id))
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
pub struct AccountLevel<T>(T);
impl AccountLevel<XPrv> {
    pub fn external(&self, derivation_scheme: DerivationScheme) -> ChangeLevel<XPrv> {
        ChangeLevel::from(self.0.derive(derivation_scheme, 0))
    }
    pub fn internal(&self, derivation_scheme: DerivationScheme) -> ChangeLevel<XPrv> {
        ChangeLevel::from(self.0.derive(derivation_scheme, 1))
    }
    pub fn change(&self, derivation_scheme:DerivationScheme, addr_type: AddrType) -> ChangeLevel<XPrv> {
        match addr_type {
            AddrType::Internal => self.internal(derivation_scheme),
            AddrType::External => self.external(derivation_scheme),
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
    pub fn internal(&self, derivation_scheme: DerivationScheme) -> Result<ChangeLevel<XPub>> {
        Ok(ChangeLevel::from(self.0.derive(derivation_scheme, 1)?))
    }
    pub fn external(&self, derivation_scheme: DerivationScheme) -> Result<ChangeLevel<XPub>> {
        Ok(ChangeLevel::from(self.0.derive(derivation_scheme, 0)?))
    }
    pub fn change(&self, derivation_scheme: DerivationScheme, addr_type: AddrType) -> Result<ChangeLevel<XPub>> {
        match addr_type {
            AddrType::Internal => self.internal(derivation_scheme),
            AddrType::External => self.external(derivation_scheme),
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
    pub fn index(&self, derivation_scheme: DerivationScheme, index: DerivationIndex) -> IndexLevel<XPrv>
    {
        assert!(index < BIP44_SOFT_UPPER_BOUND);
        IndexLevel::from(self.0.derive(derivation_scheme, index))
    }
    pub fn public(&self) -> ChangeLevel<XPub> {
        ChangeLevel::from(self.0.public())
    }
}
impl From<XPrv> for ChangeLevel<XPrv> {
    fn from(xprv: XPrv) -> Self { ChangeLevel(xprv) }
}
impl ChangeLevel<XPub> {
    pub fn index(&self, derivation_scheme: DerivationScheme, index: DerivationIndex) -> Result<IndexLevel<XPub>>
    {
        assert!(index < BIP44_SOFT_UPPER_BOUND);
        Ok(IndexLevel::from(self.0.derive(derivation_scheme, index)?))
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
