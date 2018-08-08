/// 2 Level of randomly chosen hard derivation indexes Wallet
///

use std::{ops::Deref, iter};
use cbor_event;
use cryptoxide;
use cryptoxide::digest::{Digest};
use bip::bip39;
use hdwallet::{XPrv, DerivationScheme};
use hdpayload;
use fee::{self, FeeAlgorithm};
use coin::{self, Coin};
use txutils::{self, OutputPolicy};
use tx::{self, TxAux, Tx, TxId, TxInWitness};
use address::{ExtendedAddr, Attributes, AddrType, SpendingData};
use config::ProtocolMagic;

use super::scheme::{self};

pub type Addressing = (u32, u32);

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
                let addressing = (path.as_ref()[0], path.as_ref()[1]);

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
                let txaux = tx::TxAux::new(tx, witnesses);
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
        let (hint_low, hint_max) = addresses.size_hint();
        let mut vec = Vec::with_capacity(hint_max.unwrap_or(hint_low));

        let hdkey = hdpayload::HDKey::new(&self.public());

        for addressing in addresses {
            let key = self.derive(self.derivation_scheme, addressing.0)
                          .derive(self.derivation_scheme, addressing.1)
                          .public();

            let payload = hdkey.encrypt_path(&hdpayload::Path::new(vec![addressing.0, addressing.1]));
            let attributes = Attributes::new_bootstrap_era(Some(payload));
            let addr = ExtendedAddr::new(AddrType::ATPubKey, SpendingData::PubKeyASD(key), attributes);
            vec.push(addr);
        }

        vec
    }
}

#[derive(Debug)]
pub enum Error {
    Bip39Error(bip39::Error),
    CBorEncoding(cbor_event::Error) // Should not happen really
}
impl From<bip39::Error> for Error {
    fn from(e: bip39::Error) -> Self { Error::Bip39Error(e) }
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self { Error::CBorEncoding(e) }
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
    fn from_daedalus_mnemonics<D>(derivation_scheme: DerivationScheme, dic: &D, mnemonics_phrase: String) -> Result<Self>
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
}
impl Deref for RootKey {
    type Target = XPrv;
    fn deref(&self) -> &Self::Target { &self.root_key }
}
