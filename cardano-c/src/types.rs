use std::os::raw::c_int;
use cardano::address;
use cardano::hdwallet;
use cardano::wallet::bip44;

/// C result type, where 0 is success and !0 is failure
#[repr(C)]
pub struct CardanoResult(c_int);

impl CardanoResult {
    pub fn success() -> CardanoResult { CardanoResult(0) }
    pub fn failure() -> CardanoResult { CardanoResult(1) }
}

/// C pointer to an Extended Private Key
pub type XPrvPtr = *mut hdwallet::XPrv;

/// C pointer to an Extended Public Key
pub type XPubPtr = *mut hdwallet::XPub;

/// C pointer to a (parsed) Extended Address
pub type AddressPtr = *mut address::ExtendedAddr;

/// C pointer to a Wallet
pub type WalletPtr = *mut bip44::Wallet;

/// C pointer to an Account;
pub type AccountPtr = *mut bip44::Account<hdwallet::XPub>;
