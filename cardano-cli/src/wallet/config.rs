use std::{path::PathBuf};
use cardano::{hdwallet::{self, DerivationScheme}};

use super::Error;
use super::Result;
use super::super::utils::password_encrypted::{self, Password};

/// directory where all the wallet will be in
pub const WALLETS_DIRECTORY : &'static str = "wallets";

/// handy function to compute the path to directory
/// where all the wallet metadata will lie.
pub fn directory( root_dir: PathBuf
                , name: &str
                ) -> PathBuf
{
    root_dir.join(WALLETS_DIRECTORY).join(name)
}

/// all the HDWallet supported models
///
/// * BIP44 will support a wallet with multiple accounts and sequential indices;
/// * RandomIndex2Levels will support a wallet, without accounts
///   and randomly selected indices (this will force us to encrypt the derivation
///   path in the address, making the address longer and increasing the fee sligthly)
///
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone)]
pub enum HDWalletModel {
    BIP44,
    RandomIndex2Levels
}

/// this is the wallet configuration and will be saved to the local disk
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// optional name of the local blockchain the wallet is attached to
    ///
    /// it is not necessary to have a blockchain attached to perform some operations
    /// such as signing data, importing redeem address, generating new addresses.
    pub attached_blockchain: Option<String>,

    /// this is necessary as the different derivation schemes won't output
    /// the same values for the same given derivation path.
    pub derivation_scheme: DerivationScheme,

    /// This is needed so we know what kind of wallet HD we are dealing with
    ///
    pub hdwallet_model: HDWalletModel
}
impl Default for Config {
    fn default() -> Self {
        Config {
            attached_blockchain: None,
            derivation_scheme: DerivationScheme::V2,
            hdwallet_model: HDWalletModel::BIP44
        }
    }
}

/// convenient function to encrypt a HDWallet XPrv with a password
///
pub fn encrypt_primary_key(password: &Password, xprv: &hdwallet::XPrv) -> Vec<u8> {
    password_encrypted::encrypt(password, xprv.as_ref())
}

/// convenient function to decrypt a HDWallet XPrv with a password
///
/// # Errors
///
/// This function may fail if:
///
/// * the password in invalid;
/// * the encrypted value did not represent a HDWallet XPrv
///
pub fn decrypt_primary_key(password: &Password, encrypted_key: &[u8]) -> Result<hdwallet::XPrv> {
    let xprv_vec = match password_encrypted::decrypt(password, encrypted_key) {
        None        => return Err(Error::CannotRetrievePrivateKeyInvalidPassword),
        Some(bytes) => bytes
    };

    if xprv_vec.len() != hdwallet::XPRV_SIZE {
        return Err(
            Error::CannotRetrievePrivateKey(
                hdwallet::Error::InvalidXPrvSize(xprv_vec.len())
            )
        )
    }

    let mut xprv_bytes = [0;hdwallet::XPRV_SIZE];
    xprv_bytes.copy_from_slice(&xprv_vec[..]);

    Ok(hdwallet::XPrv::from_bytes_verified(xprv_bytes)?)
}
