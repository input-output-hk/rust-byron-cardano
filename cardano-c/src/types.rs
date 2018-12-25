use cardano::address;
use cardano::hdwallet;
use cardano::tx;
use cardano::txbuild;
use cardano::wallet::bip44;
use std::os::raw::c_int;

/// C result type, where 0 is success and !0 is failure
#[repr(C)]
pub struct CardanoResult(c_int);

impl CardanoResult {
    pub fn success() -> CardanoResult {
        CardanoResult(0)
    }
    pub fn failure() -> CardanoResult {
        CardanoResult(1)
    }
}

/// C pointer to an Extended Private Key
pub type XPrvPtr = *mut hdwallet::XPrv;

/// C pointer to an Extended Public Key
pub type XPubPtr = *mut hdwallet::XPub;

/// C pointer to a signature
pub type SignaturePtr = *mut hdwallet::Signature<tx::Tx>;

/// C pointer to a (parsed) Extended Address
pub type AddressPtr = *mut address::ExtendedAddr;

/// C pointer to a Wallet
pub type WalletPtr = *mut bip44::Wallet;

/// C pointer to an Account;
pub type AccountPtr = *mut bip44::Account<hdwallet::XPub>;

/// C pointer to a Transaction output pointer;
pub type TransactionOutputPointerPtr = *mut tx::TxoPointer;

/// C pointer to a Transaction output;
pub type TransactionOutputPtr = *mut tx::TxOut;

/// C pointer to a Transaction;
pub type TransactionPtr = *mut tx::Tx;

/// C pointer to a signed Transaction;
pub type SignedTransactionPtr = *mut tx::TxAux;

/// C pointer to a Transaction builder;
pub type TransactionBuilderPtr = *mut txbuild::TxBuilder;

/// C pointer to a Transaction finalized;
pub type TransactionFinalizedPtr = *mut txbuild::TxFinalized;
