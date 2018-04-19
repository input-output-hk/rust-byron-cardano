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

use std::{result};

#[derive(Debug,PartialEq,Eq)]
pub enum Error {
    NotMyAddress_NoPayload,
    NotMyAddress_CannotDecodePayload,
    NotMyAddress_NotMyPublicKey,
}

pub type Result<T> = result::Result<T, Error>;

/// the Wallet object
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Wallet {
    seed: hdwallet::Seed,
    last_known_path: Option<hdpayload::Path>
}
impl Wallet {
    /// generate a new wallet
    ///
    pub fn new() -> Self { unimplemented!() }

    /// create a new wallet from the given seed
    pub fn new_from_seed(seed: hdwallet::Seed) -> Self {
        Wallet {
            seed: seed,
            last_known_path: None
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
    pub fn new_address(&mut self) -> address::ExtendedAddr {
        unimplemented!()
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
