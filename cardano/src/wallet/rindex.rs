/// 2 Level of randomly chosen hard derivation indexes Wallet
///

use std::{ops::Deref, iter};
use cbor_event;
use cryptoxide;
use cryptoxide::digest::{Digest};
use bip::bip39;
use hdwallet::{self, XPrv, XPub, DerivationScheme};
use hdpayload;
use fee::{self, FeeAlgorithm};
use coin::{self, Coin};
use txutils::{self, OutputPolicy};
use tx::{self, TxAux, Tx, TxId, TxInWitness};
use address::{ExtendedAddr, Attributes, AddrType, SpendingData};
use config::ProtocolMagic;

use super::scheme::{self};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Addressing(pub u32, pub u32);
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

    derivation_scheme: DerivationScheme
}
impl Wallet {
    pub fn from_root_key(derivation_scheme: DerivationScheme, root_key: RootKey) -> Self {
        Wallet { root_key, derivation_scheme }
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
    pub fn from_daedalus_mnemonics<D>(derivation_scheme: DerivationScheme, dic: &D, mnemonics_phrase: String) -> Result<Self>
        where D: bip39::dictionary::Language
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
    pub fn check_address(&self, address: &ExtendedAddr) -> Option<Addressing>
    {
        let hdkey = hdpayload::HDKey::new(&self.root_key.public());

        // This wallet has has only one account
        let account : &RootKey = scheme::Wallet::list_accounts(self);
        if let &Some(ref hdpa) = &address.attributes.derivation_path {
            if let Ok(path) = hdkey.decrypt_path(hdpa) {
                let addressing = Addressing(path.as_ref()[0], path.as_ref()[1]);

                // regenerate the address to prevent HDAddressPayload reuse
                //
                // i.e. it is possible to a mean player to reuse existing
                // payload in their own addresses to make recipient believe
                // they have received funds. This check prevents that to happen.
                let addresses = scheme::Account::generate_addresses(account, [addressing].iter());

                debug_assert!(addresses.len() == 1, "we expect to generate only one address here...");

                if address == &addresses[0] {
                    return Some(addressing);
                }
            }
        }

        None
    }

    pub fn move_transaction(&self, protocol_magic: ProtocolMagic, inputs: &Vec<txutils::TxInInfo<Addressing>>, output_policy: &txutils::OutputPolicy) -> fee::Result<(TxAux, fee::Fee)> {

        if inputs.len() == 0 {
            return Err(fee::Error::NoInputs);
        }

        let input_addressing : Vec<_> = inputs.iter().map(|tii| tii.address_identified.clone()).collect();

        let alg = fee::LinearFee::default();

        let total_input : Coin = {
            let mut total = Coin::zero();
            for ref i in inputs.iter() {
                let acc = total + i.value;
                total = acc?
            }
            total
        };

        let tx_base = Tx::new_with( inputs.iter().cloned().map(|input| input.txin).collect()
                                      , vec![]);
        let fake_witnesses : Vec<tx::TxInWitness> = iter::repeat(tx::TxInWitness::fake()).take(inputs.len()).collect();

        let min_fee_for_inputs = alg.calculate_for_txaux_component(&tx_base, &fake_witnesses)?.to_coin();
        let mut out_total = match total_input - min_fee_for_inputs {
            Err(coin::Error::Negative) => return Err(fee::Error::NotEnoughInput),
            Err(err) => unreachable!("{}", err),
            Ok(c) => c,
        };

        loop {
            let mut tx = tx_base.clone();
            match output_policy {
                OutputPolicy::One(change_addr) => {
                    let txout = tx::TxOut::new(change_addr.clone(), out_total);
                    tx.add_output(txout);
                },
            };

            let current_diff = (total_input - tx.get_output_total()?).unwrap_or(Coin::zero());
            let txaux_fee : fee::Fee = alg.calculate_for_txaux_component(&tx, &fake_witnesses)?;

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
                let witnesses = scheme::Wallet::sign_tx(self, protocol_magic, &tx.id(), input_addressing.iter());
                assert_eq!(witnesses.len(), fake_witnesses.len());
                let txaux = tx::TxAux::new(tx, tx::TxWitness::from(witnesses));
                return Ok((txaux, txaux_fee))
            } else {
                // already above..
                if current_diff > txaux_fee.to_coin() {
                    let r = (out_total + Coin::unit())?;
                    out_total = r
                } else {
                    // not enough fee, so reduce the output_total
                    match out_total - Coin::unit() {
                        Err(coin::Error::Negative) => return Err(fee::Error::NotEnoughInput),
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
    fn deref(&self) -> &Self::Target { &self.root_key }
}

impl scheme::Wallet for Wallet {
    /// 2 Level of randomly chosen hard derivation indexes does not support Account model. Only one account: the root key.
    type Account     = RootKey;
    /// 2 Level of randomly chosen hard derivation indexes does not support Account model. Only one account: the root key.
    type Accounts    = Self::Account;
    /// 2 Level of randomly chosen hard derivation indexes derivation consists of 2 level of hard derivation, this is why
    /// it is not possible to have a public key account like in the bip44 model.
    type Addressing  = Addressing;

    fn create_account(&mut self, _: &str, _: u32) -> Self::Account {
        self.root_key.clone()
    }
    fn list_accounts<'a>(&'a self) -> &'a Self::Accounts  { &self.root_key }
    fn sign_tx<'a, I>(&'a self, protocol_magic: ProtocolMagic, txid: &TxId, addresses: I) -> Vec<TxInWitness>
        where I: Iterator<Item = &'a Self::Addressing>
    {
        let mut witnesses = vec![];

        for addressing in addresses {
            let key = self.root_key
                          .derive(self.derivation_scheme, addressing.0)
                          .derive(self.derivation_scheme, addressing.1);

            let tx_witness = TxInWitness::new(protocol_magic, &key, txid);
            witnesses.push(tx_witness);
        }
        witnesses
    }
}
impl scheme::Account for RootKey {
    type Addressing = Addressing;

    fn generate_addresses<'a, I>(&'a self, addresses: I) -> Vec<ExtendedAddr>
        where I: Iterator<Item = &'a Self::Addressing>
    {
        self.address_generator().iter_with(addresses).collect()
    }
}

#[derive(Debug)]
pub enum Error {
    Bip39Error(bip39::Error),
    DerivationError(hdwallet::Error),
    PayloadError(hdpayload::Error),
    CBorEncoding(cbor_event::Error), // Should not happen really

    /// the addressing decoded in the payload is invalid
    InvalidPayloadAddressing,

    /// we were not able to reconstruct the wallet's address
    /// it could be due to that:
    ///
    /// 1. this address is using a different derivation scheme;
    /// 2. the address has been falsified (someone copied
    ///    an HDPayload from another of the wallet's addresses and
    ///    put it in one of its address);
    /// 3. that the software needs to be updated.
    ///
    CannotReconstructAddress
}
impl From<bip39::Error> for Error {
    fn from(e: bip39::Error) -> Self { Error::Bip39Error(e) }
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self { Error::CBorEncoding(e) }
}
impl From<hdwallet::Error> for Error {
    fn from(e: hdwallet::Error) -> Self { Error::DerivationError(e) }
}
impl From<hdpayload::Error> for Error {
    fn from(e: hdpayload::Error) -> Self { Error::PayloadError(e) }
}

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone)]
pub struct RootKey {
    root_key: XPrv,
    derivation_scheme: DerivationScheme
}
impl RootKey {
    pub fn new(root_key: XPrv, derivation_scheme: DerivationScheme) -> Self {
        RootKey {
            root_key,
            derivation_scheme
        }
    }
    pub fn from_daedalus_mnemonics<D>(derivation_scheme: DerivationScheme, dic: &D, mnemonics_phrase: String) -> Result<Self>
        where D: bip39::dictionary::Language
    {
        let mnemonics = bip39::Mnemonics::from_string(dic, &mnemonics_phrase)?;
        let entropy = bip39::Entropy::from_mnemonics(&mnemonics)?;

        let entropy_bytes = cbor_event::Value::Bytes(Vec::from(entropy.as_ref()));
        let entropy_cbor = cbor!(&entropy_bytes)?;
        let seed : Vec<u8> = {
            let mut blake2b = cryptoxide::blake2b::Blake2b::new(32);
            blake2b.input(&entropy_cbor);
            let mut out = [0;32];
            blake2b.result(&mut out);
            cbor_event::se::Serializer::new_vec().write_bytes(&Vec::from(&out[..]))?.finalize()
        };

        let xprv = XPrv::generate_from_daedalus_seed(&seed);
        Ok(RootKey::new(xprv, derivation_scheme))
    }

    pub fn address_generator(&self) -> AddressGenerator<XPrv>
    {
        AddressGenerator::<XPrv>::new(self.root_key.clone(), self.derivation_scheme)
    }
}
impl Deref for RootKey {
    type Target = XPrv;
    fn deref(&self) -> &Self::Target { &self.root_key }
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
    pub fn iter_with<'a, I>(self, iter: I) -> AddressIterator<K, I>
        where I: Iterator<Item = &'a Addressing>
    {
        AddressIterator::new(self, iter)
    }

    pub fn try_get_addressing(&self, address: &ExtendedAddr) -> Result<Option<Addressing>> {
        if let Some(ref epath) = address.attributes.derivation_path {
            let path = match self.hdkey.decrypt_path(epath) {
                Ok(path) => path,
                Err(hdpayload::Error::CannotDecrypt) => {
                    // we could not decrypt it, there was no _error_.
                    return Ok(None);
                },
                Err(err) => return Err(Error::from(err))
            };
            if path.len() == 2 {
                let path = Addressing(path[0], path[1]);

                Ok(Some(path))
            } else {
                Err(Error::InvalidPayloadAddressing)
            }
        } else { Ok(None) }
    }

    fn compare_address_with_pubkey(&self, address: &ExtendedAddr, path: &Addressing, key: XPub) -> Result<()> {
        let payload = self.hdkey.encrypt_path(&hdpayload::Path::new(vec![path.0, path.1]));

        let mut attributes = address.attributes.clone();
        attributes.derivation_path = Some(payload);

        let expected = ExtendedAddr::new(AddrType::ATPubKey, SpendingData::PubKeyASD(key), attributes);
        if &expected == address {
            Ok(())
        } else {
            Err(Error::CannotReconstructAddress)
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

    fn key(&self, path: &Addressing) -> Result<XPub> {
        Ok(
            self.cached_key
                .derive(self.derivation_scheme, path.0)?
                .derive(self.derivation_scheme, path.1)?
        )
    }

    /// attempt the reconstruct the address with the same metadata
    pub fn compare_address(&self, address: &ExtendedAddr, path: &Addressing) -> Result<()> {
        let key = self.key(path)?;
        self.compare_address_with_pubkey(address, path, key)
    }

    /// create an address with the given addressing
    pub fn address(&self, path: &Addressing) -> Result<ExtendedAddr> {
        let key = self.key(path)?;

        let payload = self.hdkey.encrypt_path(&hdpayload::Path::new(vec![path.0, path.1]));
        let attributes = Attributes::new_bootstrap_era(Some(payload));
        Ok(ExtendedAddr::new(AddrType::ATPubKey, SpendingData::PubKeyASD(key), attributes))
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

    fn key(&self, path: &Addressing) -> XPrv {
        self.cached_key
            .derive(self.derivation_scheme, path.0)
            .derive(self.derivation_scheme, path.1)
    }

    /// create an address with the given addressing
    pub fn address(&self, path: &Addressing) -> ExtendedAddr {
        let key = self.key(path).public();

        let payload = self.hdkey.encrypt_path(&hdpayload::Path::new(vec![path.0, path.1]));
        let attributes = Attributes::new_bootstrap_era(Some(payload));
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

    iter: I
}
impl<K, I> AddressIterator<K, I> {
    fn new(generator: AddressGenerator<K>, iter: I) -> Self {
        AddressIterator {
            generator,
            iter
        }
    }
}
impl<'a, I> Iterator for AddressIterator<XPrv, I>
    where I: Iterator<Item = &'a Addressing>
{
    type Item = ExtendedAddr;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|path| { self.generator.address(path) })
    }
}
impl<'a, I> Iterator for AddressIterator<XPub, I>
    where I: Iterator<Item = &'a Addressing>
{
    type Item = Result<ExtendedAddr>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|path| { self.generator.address(path) })
    }
}
