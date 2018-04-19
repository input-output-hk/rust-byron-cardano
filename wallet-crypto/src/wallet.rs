//! unlike the hdwallet object, this the stateful wallet implementation
//!
//! # definition
//!
//! While other modules tries to be stateless as much as possible
//! here we want to provide all the logic one may want from a wallet.
//!

use hdwallet;
use hdpayload;
use address;
use tx;
use config;
use tx::fee::Algorithm;

use std::{result};

#[derive(Serialize, Deserialize, Debug,PartialEq,Eq)]
pub enum Error {
    NotMyAddress_NoPayload,
    NotMyAddress_CannotDecodePayload,
    NotMyAddress_NotMyPublicKey,
    FeeCalculationError(tx::fee::Error)
}
impl From<tx::fee::Error> for Error {
    fn from(j: tx::fee::Error) -> Self { Error::FeeCalculationError(j) }
}

pub type Result<T> = result::Result<T, Error>;

/// the Wallet object
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Wallet {
    seed: hdwallet::Seed,
    last_known_path: Option<hdpayload::Path>,

    config: config::Config,
    selection_policy: tx::fee::SelectionPolicy,
}
impl Wallet {
    /// generate a new wallet
    ///
    pub fn new() -> Self { unimplemented!() }

    /// create a new wallet from the given seed
    pub fn new_from_seed(seed: hdwallet::Seed) -> Self {
        Wallet {
            seed: seed,
            last_known_path: None,
            config: config::Config::default(),
            selection_policy: tx::fee::SelectionPolicy::default()
        }
    }

    /// this function sets the last known path used for generating addresses
    ///
    pub fn force_last_known_path(&mut self, path: hdpayload::Path) {
        self.last_known_path = Some(path);
    }

    /// create a new extended address
    ///
    /// if you try to create address before being aware of all the
    /// existing address you have created used first this function will
    /// start from the beginning and may generate duplicated addresses.
    ///
    pub fn new_address(&mut self, is_change: bool) -> address::ExtendedAddr {
        let path = match &self.last_known_path {
            &None => hdpayload::Path::bip44_new(0x80000000,if is_change { 1 } else { 0 },0),
            &Some(ref lkp) => if is_change { lkp.bip44_next_change() } else { lkp.bip44_next() }
        };

        self.force_last_known_path(path.clone());

        let pk = self.get_xprv(&path).public();
        let hdap = self.get_hdkey().encrypt_path(&path);
        let addr_type = address::AddrType::ATPubKey;
        let sd = address::SpendingData::PubKeyASD(pk.clone());
        let attrs = address::Attributes::new_single_key(&pk, Some(hdap));

        address::ExtendedAddr::new(addr_type, sd, attrs)
    }

    /// return the path of the given address *if*:
    ///
    /// - the hdpayload is actually ours
    /// - the public key is actually ours
    ///
    /// if the address is actually ours, we return the `hdpayload::Path` and
    /// update the `Wallet` internal state.
    ///
    pub fn recognize_address(&mut self, addr: &address::ExtendedAddr) -> Result<hdpayload::Path> {
        // retrieve the key to decrypt the payload from the extended address
        let hdkey = self.get_hdkey();

        // try to decrypt the path, if it fails, it is not one of our address
        let hdpa = match addr.attributes.derivation_path.clone() {
            Some(hdpa) => hdpa,
            None => return Err(Error::NotMyAddress_NoPayload)
        };
        let path = match hdkey.decrypt_path(&hdpa) {
            Some(path) => path,
            None => return Err(Error::NotMyAddress_CannotDecodePayload)
        };

        // now we have the path, we can retrieve the associated XPub
        let xpub = self.get_xprv(&path).public();
        let addr2 = address::ExtendedAddr::new(
            addr.addr_type.clone(),
            address::SpendingData::PubKeyASD(xpub),
            addr.attributes.clone()
        );
        if addr != &addr2 { return Err(Error::NotMyAddress_NotMyPublicKey); }

        // retrieve
        match self.last_known_path.clone() {
            None => self.force_last_known_path(path.clone()),
            Some(lkp) => {
                if lkp < path { self.force_last_known_path(path.clone()) }
            }
        }

        Ok(path)
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
        let change_addr = self.new_address(true);

        let (fee, selected_inputs, change) = alg.compute(self.selection_policy, inputs, outputs, &change_addr, fee_addr)?;

        let mut tx = tx::Tx::new_with(
            selected_inputs.iter().cloned().map(|input| input.ptr).collect(),
            outputs.iter().cloned().collect()
        );

        tx.add_output(tx::TxOut::new(fee_addr.clone(), fee.to_coin()));
        tx.add_output(tx::TxOut::new(change_addr     , change));

        let mut witnesses = vec![];

        for input in selected_inputs {
            let path = self.recognize_input(&input)?;
            let key  = self.get_xprv(&path);

            witnesses.push(tx::TxInWitness::new(&self.config, &key, &tx));
        }

        Ok(tx::TxAux::new(tx, witnesses))
    }

    /// check if the given transaction input is one of ours
    ///
    /// and retuns the associated Path
    fn recognize_input(&mut self, input: &tx::Input) -> Result<hdpayload::Path> {
        self.recognize_address(&input.value.address)
    }


    /// retrieve the root extended private key from the wallet
    ///
    /// TODO: this function is not meant to be public
    fn get_root_key(&self) -> hdwallet::XPrv {
        hdwallet::XPrv::generate_from_seed(&self.seed)
    }

    /// retrieve the HD key from the wallet.
    ///
    /// TODO: this function is not meant to be public
    fn get_hdkey(&self) -> hdpayload::HDKey {
        hdpayload::HDKey::new(&self.get_root_key().public())
    }

    /// retrieve the key from the wallet and the given path
    ///
    /// TODO: this function is not meant to be public
    fn get_xprv(&self, path: &hdpayload::Path) -> hdwallet::XPrv {
        path.as_ref().iter().cloned().fold(self.get_root_key(), |k, i| k.derive(i))
    }
}
