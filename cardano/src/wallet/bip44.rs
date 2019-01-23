use address::ExtendedAddr;
use bip::bip39;
use bip::bip44::{BIP44_COIN_TYPE, BIP44_PURPOSE, BIP44_SOFT_UPPER_BOUND};
use config::{NetworkMagic, ProtocolMagic};
/// BIP44 derivation scheme and address model
///
use hdwallet::{DerivationIndex, DerivationScheme, Result, XPrv, XPub, XPRV_SIZE};
use std::{collections::BTreeMap, ops::Deref};
use tx::{TxId, TxInWitness};

use super::keygen;
use super::scheme;

pub use bip::bip44::{self, AddrType, Addressing, Change, Error, Index};

/// BIP44 based wallet, i.e. using sequential indexing.
///
/// See [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)
/// specifications for more details.
///
pub struct Wallet {
    cached_root_key: RootLevel<XPrv>,
    accounts: BTreeMap<String, Account<XPrv>>,
    derivation_scheme: DerivationScheme,
}
impl Wallet {
    /// load a wallet from a cached root key
    ///
    /// this is handy to reconstruct the wallet from a locally saved
    /// state (beware that the cached root key would need to be stored
    /// in a secure manner though).
    ///
    pub fn from_cached_key(
        cached_root_key: RootLevel<XPrv>,
        derivation_scheme: DerivationScheme,
    ) -> Self {
        let accounts = BTreeMap::new();
        Wallet {
            cached_root_key,
            accounts,
            derivation_scheme,
        }
    }

    /// construct a new `Wallet` from the given Root key. Not really meant
    /// to reconstruct the wallet from locally saved state, but more to allow
    /// generating root seed without using bip39 mnemonics as proposed in
    /// [`Wallet::from_bip39_mnemonics`](./struct.Wallet.html#method.from_bip39_mnemonics)
    /// constructor.
    ///
    pub fn from_root_key(root_key: XPrv, derivation_scheme: DerivationScheme) -> Self {
        let cached_root_key = root_key
            .derive(derivation_scheme, BIP44_PURPOSE)
            .derive(derivation_scheme, BIP44_COIN_TYPE);
        Wallet::from_cached_key(RootLevel::from(cached_root_key), derivation_scheme)
    }

    /// helper to create a wallet from BIP39 Seed
    ///
    /// We assume the [`MnemonicString`](../../bip/bip39/struct.MnemonicString.html)
    /// so we don't have to handle error in this constructor.
    ///
    /// Prefer `from_entropy` unless BIP39 seed generation compatibility is needed.
    pub fn from_bip39_seed(seed: &bip39::Seed, derivation_scheme: DerivationScheme) -> Self {
        let xprv = XPrv::generate_from_bip39(seed);

        Wallet::from_root_key(xprv, derivation_scheme)
    }

    /// helper to create a wallet from BIP39 mnemonics
    ///
    /// We assume the [`MnemonicString`](../../bip/bip39/struct.MnemonicString.html)
    /// so we don't have to handle error in this constructor.
    ///
    /// Prefer `from_entropy` unless BIP39 seed generation compatibility is needed.
    pub fn from_bip39_mnemonics(
        mnemonics_phrase: &bip39::MnemonicString,
        password: &[u8],
        derivation_scheme: DerivationScheme,
    ) -> Self {
        let seed = bip39::Seed::from_mnemonic_string(mnemonics_phrase, password);

        Wallet::from_bip39_seed(&seed, derivation_scheme)
    }

    /// Create a new wallet from a root entropy
    ///
    /// This is the recommended method to create a wallet from initial generated value.
    ///
    /// Note this method, doesn't put the bip39 dictionary used in the cryptographic data,
    /// hence the way the mnemonics are displayed is independent of the language chosen.
    pub fn from_entropy(
        entropy: &bip39::Entropy,
        password: &[u8],
        derivation_scheme: DerivationScheme,
    ) -> Self {
        let mut seed = [0u8; XPRV_SIZE];
        keygen::generate_seed(entropy, password, &mut seed);
        let xprv = XPrv::normalize_bytes(seed);
        Wallet::from_root_key(xprv, derivation_scheme)
    }

    pub fn derivation_scheme(&self) -> DerivationScheme {
        self.derivation_scheme
    }
}
impl Deref for Wallet {
    type Target = RootLevel<XPrv>;
    fn deref(&self) -> &Self::Target {
        &self.cached_root_key
    }
}
impl scheme::Wallet for Wallet {
    type Account = Account<XPrv>;
    type Accounts = BTreeMap<String, Self::Account>;
    type Addressing = Addressing;

    fn create_account(&mut self, alias: &str, id: u32) -> Self::Account {
        let account = self.cached_root_key.account(self.derivation_scheme, id);
        let account = Account {
            cached_root_key: account,
            derivation_scheme: self.derivation_scheme,
        };
        self.accounts.insert(alias.to_owned(), account.clone());
        account
    }
    fn list_accounts<'a>(&'a self) -> &'a Self::Accounts {
        &self.accounts
    }
    fn sign_tx<I>(
        &self,
        protocol_magic: ProtocolMagic,
        txid: &TxId,
        addresses: I,
    ) -> Vec<TxInWitness>
    where
        I: Iterator<Item = Self::Addressing>,
    {
        let mut witnesses = vec![];

        for addressing in addresses {
            let key = self
                .cached_root_key
                .account(
                    self.derivation_scheme,
                    addressing.account.get_scheme_value(),
                )
                .change(self.derivation_scheme, addressing.address_type())
                .index(self.derivation_scheme, addressing.index.get_scheme_value());

            let tx_witness = TxInWitness::new_extended_pk(protocol_magic, &key, txid);
            witnesses.push(tx_witness);
        }
        witnesses
    }
}

#[derive(Clone)]
pub struct Account<K> {
    cached_root_key: AccountLevel<K>,
    derivation_scheme: DerivationScheme,
}
impl<K> Account<K> {
    pub fn new(cached_root_key: AccountLevel<K>, derivation_scheme: DerivationScheme) -> Self {
        Account {
            cached_root_key,
            derivation_scheme,
        }
    }
}
impl Account<XPrv> {
    pub fn public(&self) -> Account<XPub> {
        Account {
            cached_root_key: self.cached_root_key.public(),
            derivation_scheme: self.derivation_scheme,
        }
    }

    /// create an [`AddressGenerator`](./struct.AddressGenerator.html) iterator.
    ///
    /// an address iterator starts from the given index, and stop when
    /// the last soft derivation is reached
    /// ([`BIP44_SOFT_UPPER_BOUND`](../../bip/bip44/constant.BIP44_SOFT_UPPER_BOUND.html)).
    ///
    /// # Example:
    ///
    /// ```
    /// # use cardano::wallet::{bip44::{self, AddrType}, scheme::{Wallet}};
    /// # use cardano::bip::bip39::{MnemonicString, dictionary::ENGLISH};
    /// # use cardano::address::ExtendedAddr;
    /// # use cardano::util::base58;
    /// # use cardano::config::{NetworkMagic};
    ///
    /// let mnemonics = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    /// let mnemonics = MnemonicString::new(&ENGLISH, mnemonics.to_owned()).unwrap();
    ///
    /// let mut wallet = bip44::Wallet::from_bip39_mnemonics(&mnemonics, b"password", Default::default());
    /// let account = wallet.create_account("account 1", 0);
    ///
    /// // print only every two address (20 times)
    /// for (idx, xprv) in account.address_generator(AddrType::External, 0)
    ///                           .enumerate()
    ///                           .filter(|(idx, _)| idx % 2 == 0)
    ///                           .take(20)
    /// {
    ///   let address = ExtendedAddr::new_simple(*xprv.public(), NetworkMagic::from(1234));
    ///   println!("address index {}: {}", idx, address);
    /// }
    ///
    /// ```
    ///
    pub fn address_generator(&self, addr_type: AddrType, from: u32) -> AddressGenerator<XPrv> {
        AddressGenerator {
            cached_root_key: self
                .cached_root_key
                .change(self.derivation_scheme, addr_type),
            derivation_scheme: self.derivation_scheme,
            index: from,
        }
    }
}
impl Account<XPub> {
    /// create an [`AddressGenerator`](./struct.AddressGenerator.html) iterator.
    ///
    /// an address iterator starts from the given index, and stop when
    /// the last soft derivation is reached
    /// ([`BIP44_SOFT_UPPER_BOUND`](../../bip/bip44/constant.BIP44_SOFT_UPPER_BOUND.html)).
    ///
    /// # Example:
    ///
    /// ```
    /// # use cardano::wallet::{bip44::{self, AddrType}, scheme::{Wallet}};
    /// # use cardano::bip::bip39::{MnemonicString, dictionary::ENGLISH};
    /// # use cardano::address::ExtendedAddr;
    /// # use cardano::util::base58;
    /// # use cardano::config::{NetworkMagic};
    ///
    /// let mnemonics = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    /// let mnemonics = MnemonicString::new(&ENGLISH, mnemonics.to_owned()).unwrap();
    ///
    /// let mut wallet = bip44::Wallet::from_bip39_mnemonics(&mnemonics, b"password", Default::default());
    /// let account = wallet.create_account("account 1", 0).public();
    ///
    /// // print first 10 addresses from index 10000
    /// for (idx, xpub) in account.address_generator(AddrType::Internal, 10000).unwrap()
    ///                           .take(10)
    ///                           .enumerate()
    /// {
    ///   let address = ExtendedAddr::new_simple(*xpub.unwrap(), NetworkMagic::from(1234));
    ///   println!("address index {}: {}", idx, address);
    /// }
    ///
    /// ```
    ///
    pub fn address_generator(
        &self,
        addr_type: AddrType,
        from: u32,
    ) -> Result<AddressGenerator<XPub>> {
        Ok(AddressGenerator {
            cached_root_key: self
                .cached_root_key
                .change(self.derivation_scheme, addr_type)?,
            derivation_scheme: self.derivation_scheme,
            index: from,
        })
    }
}
impl Deref for Account<XPrv> {
    type Target = AccountLevel<XPrv>;
    fn deref(&self) -> &Self::Target {
        &self.cached_root_key
    }
}
impl Deref for Account<XPub> {
    type Target = AccountLevel<XPub>;
    fn deref(&self) -> &Self::Target {
        &self.cached_root_key
    }
}
impl scheme::Account for Account<XPub> {
    type Addressing = (bip44::AddrType, u32);

    fn generate_addresses<'a, I>(
        &'a self,
        addresses: I,
        network_magic: NetworkMagic,
    ) -> Vec<ExtendedAddr>
    where
        I: Iterator<Item = &'a Self::Addressing>,
    {
        let (hint_low, hint_max) = addresses.size_hint();
        let mut vec = Vec::with_capacity(hint_max.unwrap_or(hint_low));

        for addressing in addresses {
            let key = self
                .cached_root_key
                .change(self.derivation_scheme, addressing.0)
                .expect("cannot fail")
                .index(self.derivation_scheme, addressing.1)
                .expect("cannot fail");
            let addr = ExtendedAddr::new_simple(key.0, network_magic);
            vec.push(addr);
        }

        vec
    }
}
impl scheme::Account for Account<XPrv> {
    type Addressing = (bip44::AddrType, u32);

    fn generate_addresses<'a, I>(
        &'a self,
        addresses: I,
        network_magic: NetworkMagic,
    ) -> Vec<ExtendedAddr>
    where
        I: Iterator<Item = &'a Self::Addressing>,
    {
        let (hint_low, hint_max) = addresses.size_hint();
        let mut vec = Vec::with_capacity(hint_max.unwrap_or(hint_low));

        for addressing in addresses {
            let key = self
                .cached_root_key
                .change(self.derivation_scheme, addressing.0)
                .index(self.derivation_scheme, addressing.1)
                .public();
            let addr = ExtendedAddr::new_simple(key.0, network_magic);
            vec.push(addr);
        }

        vec
    }
}

/// create an `AddressGenerator`
///
/// an address iterator starts from the given index, and stop when
/// the last soft derivation is reached
/// ([`BIP44_SOFT_UPPER_BOUND`](../../bip/bip44/constant.BIP44_SOFT_UPPER_BOUND.html)).
///
/// see [`Account<XPrv>::address_generator`](./struct.Account.html#method.address_generator)
/// and [`Account<XPub>::address_generator`](./struct.Account.html#method.address_generator-1)
/// for example of use.
///
pub struct AddressGenerator<K> {
    cached_root_key: ChangeLevel<K>,
    derivation_scheme: DerivationScheme,
    index: u32,
}
impl Iterator for AddressGenerator<XPrv> {
    type Item = IndexLevel<XPrv>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= BIP44_SOFT_UPPER_BOUND {
            return None;
        }
        let index = self.index;
        self.index += 1;

        let index = self.cached_root_key.index(self.derivation_scheme, index);
        Some(index)
    }
}
impl Iterator for AddressGenerator<XPub> {
    type Item = Result<IndexLevel<XPub>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= BIP44_SOFT_UPPER_BOUND {
            return None;
        }
        let index = self.index;
        self.index += 1;

        let index = self.cached_root_key.index(self.derivation_scheme, index);
        Some(index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootLevel<T>(T);
impl RootLevel<XPrv> {
    pub fn account(&self, derivation_scheme: DerivationScheme, id: u32) -> AccountLevel<XPrv> {
        AccountLevel::from(
            self.0
                .derive(derivation_scheme, BIP44_SOFT_UPPER_BOUND | id),
        )
    }
}
impl<T> Deref for RootLevel<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
impl From<XPrv> for RootLevel<XPrv> {
    fn from(xprv: XPrv) -> Self {
        RootLevel(xprv)
    }
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
    pub fn change(
        &self,
        derivation_scheme: DerivationScheme,
        addr_type: AddrType,
    ) -> ChangeLevel<XPrv> {
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
    fn from(xprv: XPrv) -> Self {
        AccountLevel(xprv)
    }
}
impl AccountLevel<XPub> {
    pub fn internal(&self, derivation_scheme: DerivationScheme) -> Result<ChangeLevel<XPub>> {
        Ok(ChangeLevel::from(self.0.derive(derivation_scheme, 1)?))
    }
    pub fn external(&self, derivation_scheme: DerivationScheme) -> Result<ChangeLevel<XPub>> {
        Ok(ChangeLevel::from(self.0.derive(derivation_scheme, 0)?))
    }
    pub fn change(
        &self,
        derivation_scheme: DerivationScheme,
        addr_type: AddrType,
    ) -> Result<ChangeLevel<XPub>> {
        match addr_type {
            AddrType::Internal => self.internal(derivation_scheme),
            AddrType::External => self.external(derivation_scheme),
        }
    }
}
impl From<XPub> for AccountLevel<XPub> {
    fn from(xpub: XPub) -> Self {
        AccountLevel(xpub)
    }
}
impl<T> Deref for AccountLevel<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeLevel<T>(T);
impl ChangeLevel<XPrv> {
    pub fn index(
        &self,
        derivation_scheme: DerivationScheme,
        index: DerivationIndex,
    ) -> IndexLevel<XPrv> {
        IndexLevel::from(self.0.derive(derivation_scheme, index))
    }
    pub fn public(&self) -> ChangeLevel<XPub> {
        ChangeLevel::from(self.0.public())
    }
}
impl From<XPrv> for ChangeLevel<XPrv> {
    fn from(xprv: XPrv) -> Self {
        ChangeLevel(xprv)
    }
}
impl ChangeLevel<XPub> {
    pub fn index(
        &self,
        derivation_scheme: DerivationScheme,
        index: DerivationIndex,
    ) -> Result<IndexLevel<XPub>> {
        Ok(IndexLevel::from(self.0.derive(derivation_scheme, index)?))
    }
}
impl From<XPub> for ChangeLevel<XPub> {
    fn from(xpub: XPub) -> Self {
        ChangeLevel(xpub)
    }
}
impl<T> Deref for ChangeLevel<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexLevel<T>(T);
impl IndexLevel<XPrv> {
    pub fn public(&self) -> IndexLevel<XPub> {
        IndexLevel::from(self.0.public())
    }
}
impl From<XPrv> for IndexLevel<XPrv> {
    fn from(xprv: XPrv) -> Self {
        IndexLevel(xprv)
    }
}
impl From<XPub> for IndexLevel<XPub> {
    fn from(xpub: XPub) -> Self {
        IndexLevel(xpub)
    }
}
impl<T> Deref for IndexLevel<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
