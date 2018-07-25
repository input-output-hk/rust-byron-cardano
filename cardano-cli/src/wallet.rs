use std::path::PathBuf;
use cardano::{hdwallet::{self, DerivationScheme}, wallet};
use storage::{self, tmpfile::{TmpFile}};
use serde_yaml;

use utils::term::Term;
use utils::password_encrypted::{self, Password};

fn wallet_directory( root_dir: PathBuf
                   , name: &str
                   ) -> PathBuf
{
    root_dir.join("wallets").join(name)
}

pub fn command_new( mut term: Term
                  , root_dir: PathBuf
                  , name: String
                  )
{

    term.success(&format!("local blockchain `{}' created.\n", &name)).unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
enum HDWalletModel {
    BIP44,
    RandomIndex2Levels
}

// this is the wallet configuration and will be saved to the local disk
//
#[derive(Debug, Serialize, Deserialize)]
struct Config {
    // optional name of the local blockchain the wallet is attached to
    //
    // it is not necessary to have a blockchain attached to perform some operations
    // such as signing data, importing redeem address, generating new addresses.
    attached_blockchain: Option<String>,

    // this is necessary as the different derivation schemes won't output
    // the same values for the same given derivation path.
    derivation_scheme: DerivationScheme,

    // This is needed so we know what kind of wallet HD we are dealing with
    //
    hdwallet_model: HDWalletModel
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

struct Wallet {
    hdwallet_seed: Vec<u8>,

    root_dir: PathBuf,
    // conveniently keep the name given by the user to this wallet.
    name: String,

    config: Config
}
impl Wallet {
    fn new(root_dir: PathBuf, name: String, config: Config, encrypted_key: Vec<u8>) -> Self {
        Wallet {
            encrypted_key: encrypted_key,
            root_dir: root_dir,
            name: name,
            config: config
        }
    }
    fn save(&self) {
        let dir = wallet_directory(self.root_dir, self.name);
    }
    fn get_wallet_bip44(&self, password: &Password) -> Result<impl wallet::scheme::Wallet> {
        let xprv = decrypt_primary_key(password, &self.hdwallet_seed)?;
        Ok(wallet::bip44::Wallet::from_root_key(
            xprv,
            self.config.derivation_scheme
        ))
    }
    fn get_wallet_rindex(&self, password: &Password) -> Result<impl wallet::scheme::Wallet> {
        let xprv = decrypt_primary_key(password, &self.hdwallet_seed)?;
        let root_key = wallet::rindex::RootKey::new(xprv, self.config.derivation_scheme);
        Ok(wallet::rindex::Wallet::from_root_key(
            self.config.derivation_scheme,
            root_key
        ))
    }
}

type Result<T> = ::std::result::Result<T, Error>;
enum Error {
    CannotRetrievePrivateKeyInvalidPassword,
    CannotRetrievePrivateKey(hdwallet::Error),
}
impl From<hdwallet::Error> for Error {
    fn from(e: hdwallet::Error) -> Self { Error::CannotRetrievePrivateKey(e) }
}

fn decrypt_primary_key(password: &Password, encrypted_key: &[u8]) -> Result<XPrv> {
    let xprv_vec = match password_encrypted::decrypt(password, encrypted_key) {
        None        => return Err(Error::CannotRetrievePrivateKeyInvalidPassword),
        Some(bytes) => bytes
    };

    if xprv_bytes.len() != hdwallet::XPRV_SIZE {
        return Err(
            Error::CannotRetrievePrivateKey(
                hdwallet::Error::InvalidXPrvSize(xprv_bytes.len())
            )
        )
    }

    let mut xprv_bytes = [0;hdwallet::XPRV_SIZE];
    xprv_bytes.copy_from_slice(&xprv_vec[..]);

    Ok(XPrv::from_bytes_verified(xprv_bytes)?)
}
