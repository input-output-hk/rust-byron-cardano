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
    pub fn recognize_address(&mut self, addr: &address::ExtendedAddr) -> Option<hdpayload::Path> {
        unimplemented!()
    }
}
