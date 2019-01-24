use address::{AddrType, Attributes, ExtendedAddr, SpendingData};
use bip::bip39;
use cbor_event;
use coin::{self, Coin};
use config::{NetworkMagic, ProtocolMagic};
use cryptoxide;
use cryptoxide::digest::Digest;
use fee::{self, FeeAlgorithm};
use hdpayload;
use hdwallet::{self, DerivationScheme, XPrv, XPub};
use input_selection;
/// 2 Level of randomly chosen hard derivation indexes Wallet
///
use std::{error, fmt, iter, ops::Deref};
use tx::{self, Tx, TxAux, TxId, TxInWitness};
use txutils::{self, OutputPolicy};

use super::scheme;

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct Addressing(u32, u32);
impl Addressing {
    pub fn new(account: u32, index: u32) -> Self {
        Addressing(account, index)
    }
}
impl ::std::fmt::Display for Addressing {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

/// Implementation of 2 level randomly chosen derivation index wallet
///
/// This is for compatibility purpose with the existing 2 Level of
/// randomly chosen hard derivation indexes
/// wallet.
///
pub struct Wallet {
    root_key: RootKey,

    derivation_scheme: DerivationScheme,
}
impl Wallet {
    pub fn from_root_key(derivation_scheme: DerivationScheme, root_key: RootKey) -> Self {
        Wallet {
            root_key,
            derivation_scheme,
        }
    }

    /// Compatibility with daedalus mnemonic addresses
    ///
    /// > 2 Level of randomly chosen hard derivation indexes wallets uses the bip39 mnemonics but do not follow
    /// > the whole BIP39 specifications;
    ///
    /// 1. the mnemonic words are used to retrieve the entropy;
    /// 2. the entropy is serialized in CBOR;
    /// 3. the cbor serialised entropy is then hashed with Blake2b 256;
    /// 4. the blake2b digest is serialised in cbor;
    /// 5. the cbor serialised digest is then fed into HMAC sha256
    ///
    /// There are many things that can go wrong when implementing this
    /// process, it is all done correctly by this function: prefer using
    /// this function.
    pub fn from_daedalus_mnemonics<D>(
        derivation_scheme: DerivationScheme,
        dic: &D,
        mnemonics_phrase: &str,
    ) -> Result<Self>
    where
        D: bip39::dictionary::Language,
    {
        let root_key = RootKey::from_daedalus_mnemonics(derivation_scheme, dic, mnemonics_phrase)?;
        Ok(Wallet::from_root_key(derivation_scheme, root_key))
    }

    /// test that the given address belongs to the wallet.
    ///
    /// This only possible because addresses from this wallet contain
    /// a special metadata, the derivation path encrypted with
    /// the Wallet root public key.
    ///
    /// This function returns the addressing if the address belongs
    /// to this wallet, otherwise it returns `None`
    pub fn check_address(&self, address: &ExtendedAddr) -> Option<Addressing> {
        let hdkey = hdpayload::HDKey::new(&self.root_key.public());

        // This wallet has has only one account
        let account: &RootKey = scheme::Wallet::list_accounts(self);
        if let &Some(ref hdpa) = &address.attributes.derivation_path {
            if let Ok(path) = hdkey.decrypt_path(hdpa) {
                let addressing = Addressing(path.as_ref()[0], path.as_ref()[1]);

                // regenerate the address to prevent HDAddressPayload reuse
                //
                // i.e. it is possible to a mean player to reuse existing
                // payload in their own addresses to make recipient believe
                // they have received funds. This check prevents that to happen.
                let addresses = scheme::Account::generate_addresses(
                    account,
                    [addressing].iter(),
                    address.attributes.network_magic,
                );

                debug_assert!(
                    addresses.len() == 1,
                    "we expect to generate only one address here..."
                );

                if address == &addresses[0] {
                    return Some(addressing);
                }
            }
        }

        None
    }

    pub fn move_transaction(
        &self,
        protocol_magic: ProtocolMagic,
        inputs: &Vec<txutils::TxoPointerInfo<Addressing>>,
        output_policy: &txutils::OutputPolicy,
    ) -> input_selection::Result<(TxAux, fee::Fee)> {
        if inputs.len() == 0 {
            return Err(input_selection::Error::NoInputs);
        }

        let alg = fee::LinearFee::default();

        let total_input: Coin = {
            let mut total = Coin::zero();
            for ref i in inputs.iter() {
                let acc = total + i.value;
                total = acc?
            }
            total
        };

        let tx_base = Tx::new_with(
            inputs.iter().cloned().map(|input| input.txin).collect(),
            vec![],
        );
        let fake_witnesses: Vec<tx::TxInWitness> = iter::repeat(tx::TxInWitness::fake())
            .take(inputs.len())
            .collect();

        let min_fee_for_inputs = alg
            .calculate_for_txaux_component(&tx_base, &fake_witnesses)?
            .to_coin();
        let mut out_total = match total_input - min_fee_for_inputs {
            Err(coin::Error::Negative) => return Err(input_selection::Error::NotEnoughInput),
            Err(err) => unreachable!("{}", err),
            Ok(c) => c,
        };

        loop {
            let mut tx = tx_base.clone();
            match output_policy {
                OutputPolicy::One(change_addr) => {
                    let txout = tx::TxOut::new(change_addr.clone(), out_total);
                    tx.add_output(txout);
                }
            };

            let current_diff = (total_input - tx.get_output_total()?).unwrap_or(Coin::zero());
            let txaux_fee: fee::Fee = alg.calculate_for_txaux_component(&tx, &fake_witnesses)?;

            if current_diff == txaux_fee.to_coin() {
                // let witnesses = self.sign_tx(&tx, &inputs);
                /*
                match total_input - tx.get_output_total() {
                    None => {},
                    Some(fee) => {
                        assert_eq!(witnesses.len(), fake_witnesses.len());
                        let txaux = tx::TxAux::new(tx, witnesses);
                        return Ok((txaux, txaux_fee))
                    },
                }
                */
                let witnesses = scheme::Wallet::sign_tx(
                    self,
                    protocol_magic,
                    &tx.id(),
                    inputs.iter().map(|tii| tii.address_identified),
                );
                assert_eq!(witnesses.len(), fake_witnesses.len());
                let txaux = tx::TxAux::new(tx, tx::TxWitness::from(witnesses));
                return Ok((txaux, txaux_fee));
            } else {
                // already above..
                if current_diff > txaux_fee.to_coin() {
                    let r = (out_total + Coin::unit())?;
                    out_total = r
                } else {
                    // not enough fee, so reduce the output_total
                    match out_total - Coin::unit() {
                        Err(coin::Error::Negative) => {
                            return Err(input_selection::Error::NotEnoughInput);
                        }
                        Err(err) => unreachable!("{}", err),
                        Ok(o) => out_total = o,
                    }
                }
            }
        }
    }
}
impl Deref for Wallet {
    type Target = RootKey;
    fn deref(&self) -> &Self::Target {
        &self.root_key
    }
}

impl scheme::Wallet for Wallet {
    /// 2 Level of randomly chosen hard derivation indexes does not support Account model. Only one account: the root key.
    type Account = RootKey;
    /// 2 Level of randomly chosen hard derivation indexes does not support Account model. Only one account: the root key.
    type Accounts = Self::Account;
    /// 2 Level of randomly chosen hard derivation indexes derivation consists of 2 level of hard derivation, this is why
    /// it is not possible to have a public key account like in the bip44 model.
    type Addressing = Addressing;

    fn create_account(&mut self, _: &str, _: u32) -> Self::Account {
        self.root_key.clone()
    }
    fn list_accounts<'a>(&'a self) -> &'a Self::Accounts {
        &self.root_key
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
                .root_key
                .derive(self.derivation_scheme, addressing.0)
                .derive(self.derivation_scheme, addressing.1);

            let tx_witness = TxInWitness::new_extended_pk(protocol_magic, &key, txid);
            witnesses.push(tx_witness);
        }
        witnesses
    }
}
impl scheme::Account for RootKey {
    type Addressing = Addressing;

    fn generate_addresses<'a, I>(
        &'a self,
        addresses: I,
        network_magic: NetworkMagic,
    ) -> Vec<ExtendedAddr>
    where
        I: Iterator<Item = &'a Self::Addressing>,
    {
        self.address_generator()
            .iter_with(addresses, network_magic)
            .collect()
    }
}

#[derive(Debug)]
pub enum Error {
    Bip39Error(bip39::Error),
    DerivationError(hdwallet::Error),
    PayloadError(hdpayload::Error),
    CBorEncoding(cbor_event::Error), // Should not happen really

    /// the addressing decoded in the payload is invalid
    InvalidPayloadAddressing(Vec<u32>),

    /// we were not able to reconstruct the wallet's address
    /// it could be due to that:
    ///
    /// 1. this address is using a different derivation scheme;
    /// 2. the address has been falsified (someone copied
    ///    an HDPayload from another of the wallet's addresses and
    ///    put it in one of its address);
    /// 3. that the software needs to be updated.
    ///
    CannotReconstructAddress(ExtendedAddr),
}
impl From<bip39::Error> for Error {
    fn from(e: bip39::Error) -> Self {
        Error::Bip39Error(e)
    }
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self {
        Error::CBorEncoding(e)
    }
}
impl From<hdwallet::Error> for Error {
    fn from(e: hdwallet::Error) -> Self {
        Error::DerivationError(e)
    }
}
impl From<hdpayload::Error> for Error {
    fn from(e: hdpayload::Error) -> Self {
        Error::PayloadError(e)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Bip39Error(_) => write!(f, "Wallet's Mnemonic Error"),
            Error::DerivationError(_) => write!(f, "Invalid key derivation"),
            Error::PayloadError(_) => write!(f, "Error while decoding an address' payload"),
            Error::CBorEncoding(_) => write!(f, "Error while encoding address in binary format"),
            Error::InvalidPayloadAddressing(path) => write!(f, "Payload has been decoded but is corrupted or of unexpected format (path: {:?})", path),
            Error::CannotReconstructAddress(addr) => write!(f, "The address cannot be reconstructed: the payload has been decoded but the public key hash seems different (expected: {})", addr)
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::Bip39Error(ref err) => Some(err),
            Error::DerivationError(ref err) => Some(err),
            Error::PayloadError(ref err) => Some(err),
            Error::CBorEncoding(ref err) => Some(err),
            Error::InvalidPayloadAddressing(_) => None,
            Error::CannotReconstructAddress(_) => None,
        }
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone)]
pub struct RootKey {
    root_key: XPrv,
    derivation_scheme: DerivationScheme,
}
impl RootKey {
    pub fn new(root_key: XPrv, derivation_scheme: DerivationScheme) -> Self {
        RootKey {
            root_key,
            derivation_scheme,
        }
    }
    pub fn from_daedalus_mnemonics<D>(
        derivation_scheme: DerivationScheme,
        dic: &D,
        mnemonics_phrase: &str,
    ) -> Result<Self>
    where
        D: bip39::dictionary::Language,
    {
        let mnemonics = bip39::Mnemonics::from_string(dic, mnemonics_phrase)?;
        let entropy = bip39::Entropy::from_mnemonics(&mnemonics)?;

        let entropy_bytes = cbor_event::Value::Bytes(Vec::from(entropy.as_ref()));
        let entropy_cbor = cbor!(&entropy_bytes)?;
        let seed: Vec<u8> = {
            let mut blake2b = cryptoxide::blake2b::Blake2b::new(32);
            blake2b.input(&entropy_cbor);
            let mut out = [0; 32];
            blake2b.result(&mut out);
            let mut se = cbor_event::se::Serializer::new_vec();
            se.write_bytes(&Vec::from(&out[..]))?;
            se.finalize()
        };

        let xprv = XPrv::generate_from_daedalus_seed(&seed);
        Ok(RootKey::new(xprv, derivation_scheme))
    }

    /// Converts into the inner `XPrv` value
    pub fn into_xprv(self) -> XPrv {
        self.root_key
    }

    pub fn address_generator(&self) -> AddressGenerator<XPrv> {
        AddressGenerator::<XPrv>::new(self.root_key.clone(), self.derivation_scheme)
    }
}
impl Deref for RootKey {
    type Target = XPrv;
    fn deref(&self) -> &Self::Target {
        &self.root_key
    }
}

/// structure to create addresses
///
/// The model is fairly simple in this case, one can generate addresses
/// from this structure. The convenient element here is that it interfaces
/// both private key and public key derivation. So one does not need to
/// have the private key to generate addresses, the public key may suffice
/// in this case.
///
/// It is handy to hold this structure in memory during heavy address generation
/// or tight loop of address generation as the scheme to retrieve the encryption
/// key to encrypt for the address Payload is costly.
///
pub struct AddressGenerator<K> {
    hdkey: hdpayload::HDKey,
    cached_key: K,
    derivation_scheme: DerivationScheme,
}
impl<K> AddressGenerator<K> {
    /// create an address iterator from the address generator.
    ///
    /// # Example
    ///
    /// TODO
    pub fn iter_with<'a, I>(self, iter: I, network_magic: NetworkMagic) -> AddressIterator<K, I>
    where
        I: Iterator<Item = &'a Addressing>,
    {
        AddressIterator::new(self, iter, network_magic)
    }

    pub fn try_get_addressing(&self, address: &ExtendedAddr) -> Result<Option<Addressing>> {
        if let Some(ref epath) = address.attributes.derivation_path {
            let path = match self.hdkey.decrypt_path(epath) {
                Ok(path) => path,
                Err(hdpayload::Error::CannotDecrypt) => {
                    // we could not decrypt it, there was no _error_.
                    return Ok(None);
                }
                Err(err) => return Err(Error::from(err)),
            };
            if path.len() == 2 {
                let path = Addressing(path[0], path[1]);

                Ok(Some(path))
            } else {
                Err(Error::InvalidPayloadAddressing(path.to_vec()))
            }
        } else {
            Ok(None)
        }
    }

    fn compare_address_with_pubkey(
        &self,
        address: &ExtendedAddr,
        path: &Addressing,
        key: XPub,
    ) -> Result<()> {
        let payload = self
            .hdkey
            .encrypt_path(&hdpayload::Path::new(vec![path.0, path.1]));

        let mut attributes = address.attributes.clone();
        attributes.derivation_path = Some(payload);

        let expected =
            ExtendedAddr::new(AddrType::ATPubKey, SpendingData::PubKeyASD(key), attributes);
        if &expected == address {
            Ok(())
        } else {
            Err(Error::CannotReconstructAddress(expected))
        }
    }
}
impl AddressGenerator<XPub> {
    pub fn new(key: XPub, derivation_scheme: DerivationScheme) -> Self {
        let hdkey = hdpayload::HDKey::new(&key);

        AddressGenerator {
            hdkey,
            cached_key: key,
            derivation_scheme,
        }
    }

    pub fn key(&self, path: &Addressing) -> Result<XPub> {
        Ok(self
            .cached_key
            .derive(self.derivation_scheme, path.0)?
            .derive(self.derivation_scheme, path.1)?)
    }

    /// attempt the reconstruct the address with the same metadata
    pub fn compare_address(&self, address: &ExtendedAddr, path: &Addressing) -> Result<()> {
        let key = self.key(path)?;
        self.compare_address_with_pubkey(address, path, key)
    }

    /// create an address with the given addressing
    pub fn address(&self, path: &Addressing, network_magic: NetworkMagic) -> Result<ExtendedAddr> {
        let key = self.key(path)?;

        let payload = self
            .hdkey
            .encrypt_path(&hdpayload::Path::new(vec![path.0, path.1]));
        let attributes = Attributes::new_bootstrap_era(Some(payload), network_magic);
        Ok(ExtendedAddr::new(
            AddrType::ATPubKey,
            SpendingData::PubKeyASD(key),
            attributes,
        ))
    }
}
impl AddressGenerator<XPrv> {
    pub fn new(key: XPrv, derivation_scheme: DerivationScheme) -> Self {
        let hdkey = hdpayload::HDKey::new(&key.public());

        AddressGenerator {
            hdkey,
            cached_key: key,
            derivation_scheme,
        }
    }

    pub fn public(self) -> AddressGenerator<XPub> {
        AddressGenerator {
            hdkey: self.hdkey,
            cached_key: self.cached_key.public(),
            derivation_scheme: self.derivation_scheme,
        }
    }

    pub fn key(&self, path: &Addressing) -> XPrv {
        self.cached_key
            .derive(self.derivation_scheme, path.0)
            .derive(self.derivation_scheme, path.1)
    }

    /// create an address with the given addressing
    pub fn address(&self, path: &Addressing, network_magic: NetworkMagic) -> ExtendedAddr {
        let key = self.key(path).public();

        let payload = self
            .hdkey
            .encrypt_path(&hdpayload::Path::new(vec![path.0, path.1]));
        let attributes = Attributes::new_bootstrap_era(Some(payload), network_magic);
        ExtendedAddr::new(AddrType::ATPubKey, SpendingData::PubKeyASD(key), attributes)
    }

    /// attempt the reconstruct the address with the same metadata
    pub fn compare_address(&self, address: &ExtendedAddr, path: &Addressing) -> Result<()> {
        let key = self.key(path).public();
        self.compare_address_with_pubkey(address, path, key)
    }
}

pub struct AddressIterator<K, I> {
    generator: AddressGenerator<K>,
    iter: I,
    network_magic: NetworkMagic,
}
impl<K, I> AddressIterator<K, I> {
    fn new(generator: AddressGenerator<K>, iter: I, network_magic: NetworkMagic) -> Self {
        AddressIterator {
            generator,
            iter,
            network_magic,
        }
    }
}
impl<'a, I> Iterator for AddressIterator<XPrv, I>
where
    I: Iterator<Item = &'a Addressing>,
{
    type Item = ExtendedAddr;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|path| self.generator.address(path, self.network_magic))
    }
}
impl<'a, I> Iterator for AddressIterator<XPub, I>
where
    I: Iterator<Item = &'a Addressing>,
{
    type Item = Result<ExtendedAddr>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|path| self.generator.address(path, self.network_magic))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::ProtocolMagic;
    use crate::tx::TxoPointer;
    use crate::wallet::rindex;
    use crate::wallet::scheme::Wallet;

    const MNEMONICS: &'static str =
        "edge club wrap where juice nephew whip entry cover bullet cause jeans";

    lazy_static! {
        static ref OUTPUT: ExtendedAddr = {
            use std::str::FromStr;
            ExtendedAddr::from_str("Ae2tdPwUPEZ81gMkWH2PgB55y18pp2hxDxM2cmzBNnQtyLhJHqUp622zVgz")
                .unwrap()
        };
        static ref PROTOCOL_MAGIC: ProtocolMagic = ProtocolMagic::default();
        static ref ADDRESSES: Vec<ExtendedAddr> = {
            let mut wallet = rindex::Wallet::from_daedalus_mnemonics(
                DerivationScheme::V1,
                &bip39::dictionary::ENGLISH,
                MNEMONICS,
            )
            .unwrap();
            let generator = wallet.create_account("", 0).address_generator();
            generator
                .iter_with(
                    [
                        Addressing::new(0, 1),
                        Addressing::new(0, 2),
                        Addressing::new(0, 3),
                        Addressing::new(0, 4),
                    ]
                    .iter(),
                    PROTOCOL_MAGIC.clone().into(),
                )
                .collect()
        };
        static ref INPUTS: Vec<txutils::TxoPointerInfo<Addressing>> = {
            vec![
                random_txo_pointer_info(0, 1),
                random_txo_pointer_info(0, 2),
                random_txo_pointer_info(0, 3),
                random_txo_pointer_info(0, 4),
            ]
        };
    }

    fn random_txo_pointer_info(account: u32, index: u32) -> txutils::TxoPointerInfo<Addressing> {
        let txin = TxoPointer {
            id: TxId::new("".as_bytes()),
            index: 0,
        };

        txutils::TxoPointerInfo {
            txin: txin,
            value: Coin::from(1_000_000u32),
            address_identified: Addressing::new(account, index),
        }
    }

    #[test]
    fn test_move_rindex_wallet() {
        let wallet = rindex::Wallet::from_daedalus_mnemonics(
            DerivationScheme::V1,
            &bip39::dictionary::ENGLISH,
            MNEMONICS,
        )
        .unwrap();
        let policy = OutputPolicy::One(OUTPUT.clone());
        let (txaux, _) = wallet
            .move_transaction(*PROTOCOL_MAGIC, &INPUTS, &policy)
            .unwrap();

        for (witness, address) in txaux.witness.iter().zip(ADDRESSES.iter()) {
            assert!(witness.verify_address(address));
            assert!(witness.verify_tx(*PROTOCOL_MAGIC, &txaux.tx));
        }
    }
}
