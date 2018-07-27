use std::{path::PathBuf, fs, io::{Read, Write}};
use cardano::{hdwallet::{self, DerivationScheme}, wallet, bip::bip39};
use storage::{tmpfile::{TmpFile}};
use serde_yaml;
use rand::random;

use utils::term::Term;
use utils::password_encrypted::{self, Password};

use blockchain::{self, Blockchain};

fn wallet_directory( root_dir: PathBuf
                   , name: &str
                   ) -> PathBuf
{
    root_dir.join("wallets").join(name)
}

pub fn command_new<D>( mut term: Term
                     , root_dir: PathBuf
                     , name: String
                     , wallet_scheme: HDWalletModel
                     , derivation_scheme: DerivationScheme
                     , mnemonic_size: bip39::Type
                     , languages: Vec<D>
                     )
    where D: bip39::dictionary::Language
{
    let config = Config {
        attached_blockchain: None,
        derivation_scheme: derivation_scheme,
        hdwallet_model: wallet_scheme
    };

    // 1. generate the mnemonics

    let entropy = bip39::Entropy::generate(mnemonic_size, random);
    // 2. perform the seed generation from the entropy

    term.info("You can add a recovery wallet password. You can set no password, however you won't benefit from plausible deniability\n").unwrap();
    let recovery_password              = term.password("recovery password: ").unwrap();
    let recovery_password_confirmation = term.password("confirm password: ").unwrap();
    if recovery_password != recovery_password_confirmation {
        term.error("Not the same password.").unwrap();
        ::std::process::exit(1);
    }

    let mut seed = [0;hdwallet::XPRV_SIZE];
    wallet::keygen::generate_seed(&entropy, recovery_password.as_bytes(), &mut seed);

    term.info("Please, note carefully the following mnemonic words. They will be needed to recover your wallet.\n").unwrap();
    for lang in languages {
        term.warn(&format!("{}: ", lang.name())).unwrap();
        let mnemonic_phrase = entropy.to_mnemonics().to_string(&lang);
        term.simply(&format!("{}\n", mnemonic_phrase)).unwrap();
    }

    // 3. normalize the seed to make it a valid private key

    let xprv = hdwallet::XPrv::normalize_bytes(seed);

    // 4. encrypt the private key
    term.info("Set a wallet password. This is for local usage only, allows you to protect your cached private key and prevent from creating non desired transactions.\n").unwrap();
    let password              = term.password("spending password: ").unwrap();
    let password_confirmation = term.password("confirm password: ").unwrap();
    if password != password_confirmation {
        term.error("Not the same password.").unwrap();
        ::std::process::exit(1);
    }
    let encrypted_xprv = encrypt_primary_key(password.as_bytes(), &xprv);

    // 5. create the wallet
    let wallet = Wallet::new(root_dir, name, config, encrypted_xprv);

    // 6. save the wallet
    wallet.save();

    term.success(&format!("wallet `{}' successfully created.\n", &wallet.name)).unwrap();
}

pub fn command_attach( mut term: Term
                     , root_dir: PathBuf
                     , name: String
                     , blockchain_name: String
                     )
{
    // load the wallet
    let mut wallet = Wallet::load(root_dir.clone(), name);

    // 1. is the wallet already attached
    if let Some(ref bn) = wallet.config.attached_blockchain {
        term.error(&format!("Wallet already attached to blockchain `{}'\n", bn)).unwrap();
        ::std::process::exit(1);
    }

    // 2. check the blockchain exists
    let blockchain_dir = blockchain::config::directory(root_dir.clone(), &blockchain_name);
    if let Err(err) = ::std::fs::read_dir(blockchain_dir) {
        term.error(&format!("Blockchain `{}' does not exists or you do not have user permissions", blockchain_name)).unwrap();
        ::std::process::exit(2);
    }
    let blockchain = Blockchain::load(root_dir, blockchain_name.clone());

    // 3. save the attached wallet
    wallet.config.attached_blockchain = Some(blockchain_name);
    wallet.save();

    // 4. set the wallet state tag to the genesis of the blockchain
    blockchain.set_wallet_tag(&wallet.name, &blockchain.config.genesis);

    term.success("Wallet successfully attached to blockchain.").unwrap()
}

pub fn command_detach( mut term: Term
                     , root_dir: PathBuf
                     , name: String
                     )
{
    // load the wallet
    let mut wallet = Wallet::load(root_dir.clone(), name);

    // 1. get the wallet's blockchain
    let blockchain = match wallet.config.attached_blockchain {
        None => {
            term.error("Wallet is not attached to any blockchain\n").unwrap();
            ::std::process::exit(1);
        },
        Some(blockchain) => {
            Blockchain::load(root_dir, blockchain)
        }
    };

    // 2. remove the wallet tag
    blockchain.remove_wallet_tag(&wallet.name);

    // 3. remove the blockchain name from the wallet config
    wallet.config.attached_blockchain = None;

    // TODO: clear the wallet log too, we are not linked to any blockchain
    //       we cannot keep UTxO or other logs associated to a blockchain
    //       as it may not be compatible with the next attached blockchain

    wallet.save();

    term.success("Wallet successfully attached to blockchain.").unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HDWalletModel {
    BIP44,
    RandomIndex2Levels
}

// this is the wallet configuration and will be saved to the local disk
//
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    // optional name of the local blockchain the wallet is attached to
    //
    // it is not necessary to have a blockchain attached to perform some operations
    // such as signing data, importing redeem address, generating new addresses.
    pub attached_blockchain: Option<String>,

    // this is necessary as the different derivation schemes won't output
    // the same values for the same given derivation path.
    pub derivation_scheme: DerivationScheme,

    // This is needed so we know what kind of wallet HD we are dealing with
    //
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

static WALLET_CONFIG_FILE : &'static str = "config.yml";
static WALLET_PRIMARY_KEY : &'static str = "wallet.key";

struct Wallet {
    encrypted_key: Vec<u8>,

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
        let dir = wallet_directory(self.root_dir.clone(), &self.name);
        fs::DirBuilder::new().recursive(true).create(dir.clone())
            .unwrap();

        // 1. save the configuration file
        let mut tmpfile = TmpFile::create(dir.clone())
            .unwrap();
        serde_yaml::to_writer(&mut tmpfile, &self.config)
            .unwrap();
        tmpfile.render_permanent(&dir.join(WALLET_CONFIG_FILE))
            .unwrap();

        // 2. save the encrypted key
        let mut tmpfile = TmpFile::create(dir.clone())
            .unwrap();
        tmpfile.write(&self.encrypted_key).unwrap();
        tmpfile.render_permanent(&dir.join(WALLET_PRIMARY_KEY))
            .unwrap();
    }

    fn load(root_dir: PathBuf, name: String) -> Self {
        let dir = wallet_directory(root_dir.clone(), &name);

        let mut file = fs::File::open(&dir.join(WALLET_CONFIG_FILE))
            .unwrap();
        let cfg = serde_yaml::from_reader(&mut file).unwrap();

        let mut file = fs::File::open(&dir.join(WALLET_PRIMARY_KEY))
            .unwrap();
        let mut key = Vec::with_capacity(150);
        file.read_to_end(&mut key).unwrap();

        Self::new(root_dir, name, cfg, key)
    }

    fn get_wallet_bip44(&self, password: &Password) -> Result<impl wallet::scheme::Wallet> {
        let xprv = decrypt_primary_key(password, &self.encrypted_key)?;
        Ok(wallet::bip44::Wallet::from_root_key(
            xprv,
            self.config.derivation_scheme
        ))
    }
    fn get_wallet_rindex(&self, password: &Password) -> Result<impl wallet::scheme::Wallet> {
        let xprv = decrypt_primary_key(password, &self.encrypted_key)?;
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

fn encrypt_primary_key(password: &Password, xprv: &hdwallet::XPrv) -> Vec<u8> {
    password_encrypted::encrypt(password, xprv.as_ref())
}
fn decrypt_primary_key(password: &Password, encrypted_key: &[u8]) -> Result<hdwallet::XPrv> {
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
