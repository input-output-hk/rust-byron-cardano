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
use bip44::{Addressing, AddrType, BIP44_PURPOSE, BIP44_COIN_TYPE};
use tx::fee::Algorithm;

use std::{result};

#[derive(Serialize, Deserialize, Debug,PartialEq,Eq)]
pub enum Error {
    NotMyAddress_NoPayload,
    NotMyAddress_CannotDecodePayload,
    NotMyAddress_NotMyPublicKey,
    NotMyAddress_InvalidAddressing,
    FeeCalculationError(tx::fee::Error)
}
impl From<tx::fee::Error> for Error {
    fn from(j: tx::fee::Error) -> Self { Error::FeeCalculationError(j) }
}

pub type Result<T> = result::Result<T, Error>;

/// the Wallet object
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Wallet {
    cached_root_key: hdwallet::XPrv,
    last_known_address: Option<Addressing>,
    last_known_change:  Option<Addressing>,

    config: config::Config,
    selection_policy: tx::fee::SelectionPolicy,
}
impl Wallet {
    /// generate a new wallet
    ///
    pub fn new() -> Self { unimplemented!() }

    /// create a new wallet from the given seed
    pub fn new_from_seed(seed: &hdwallet::Seed) -> Self {
        let key= hdwallet::XPrv::generate_from_seed(&seed)
                    .derive(BIP44_PURPOSE)
                    .derive(BIP44_COIN_TYPE);
        Wallet {
            cached_root_key: key,
            last_known_address: None,
            last_known_change: None,
            config: config::Config::default(),
            selection_policy: tx::fee::SelectionPolicy::default()
        }
    }

    /// this function sets the last known path used for generating addresses
    ///
    pub fn force_last_known_address(&mut self, addressing: Addressing) {
        self.last_known_address = Some(addressing);
    }

    /// this function sets the last known path used for generating change addresses
    ///
    pub fn force_last_known_change(&mut self, addressing: Addressing) {
        self.last_known_change = Some(addressing);
    }

    /// create a new extended address
    ///
    /// if you try to create address before being aware of all the
    /// existing address you have created used first this function will
    /// start from the beginning and may generate duplicated addresses.
    ///
    pub fn new_address(&mut self) -> address::ExtendedAddr {
        let addressing = match &self.last_known_address {
            &None => Addressing::new(0, AddrType::External),
            &Some(ref lkp) => {
                // for now we assume we will never generate 0x80000000 addresses
                lkp.incr(1).unwrap()
            }
        };

        self.force_last_known_address(addressing.clone());

        self.make_address(&addressing)
    }

    /// create a new extended address for change purpose
    ///
    /// if you try to create address before being aware of all the
    /// existing address you have created used first this function will
    /// start from the beginning and may generate duplicated addresses.
    ///
    pub fn new_change(&mut self) -> address::ExtendedAddr {
        let addressing = match &self.last_known_change {
            &None => Addressing::new(0, AddrType::Internal),
            &Some(ref lkp) => {
                // for now we assume we will never generate 0x80000000 addresses
                lkp.incr(1).unwrap()
            }
        };

        self.force_last_known_address(addressing.clone());

        self.make_address(&addressing)
    }

    /// create an extended address from the given addressing
    ///
    fn make_address(&mut self, addressing: &Addressing) -> address::ExtendedAddr {
        let pk = self.get_xprv(&addressing).public();
        let addr_type = address::AddrType::ATPubKey;
        let sd = address::SpendingData::PubKeyASD(pk.clone());
        let attrs = address::Attributes::new_single_key(&pk, None);

        address::ExtendedAddr::new(addr_type, sd, attrs)
    }

    /// function to create a ready to send transaction to the network
    ///
    /// it select the needed inputs, compute the fee and possible change
    /// signes every TxIn as needed.
    ///
    pub fn new_transaction( &mut self
                          , inputs: &tx::Inputs
                          , outputs: &tx::Outputs
                          , fee_addr: &address::ExtendedAddr
                          )
        -> Result<tx::TxAux>
    {
        let alg = tx::fee::LinearFee::default();
        let change_addr = self.new_change();

        let (fee, selected_inputs, change) = alg.compute(self.selection_policy, inputs, outputs, &change_addr, fee_addr)?;

        let mut tx = tx::Tx::new_with(
            selected_inputs.iter().cloned().map(|input| input.ptr).collect(),
            outputs.iter().cloned().collect()
        );

        tx.add_output(tx::TxOut::new(fee_addr.clone(), fee.to_coin()));
        tx.add_output(tx::TxOut::new(change_addr     , change));

        let mut witnesses = vec![];

        for input in selected_inputs {
            let key  = self.get_xprv(&input.addressing);

            witnesses.push(tx::TxInWitness::new(&self.config, &key, &tx));
        }

        Ok(tx::TxAux::new(tx, witnesses))
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
