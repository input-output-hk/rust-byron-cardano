use cardano::hdwallet;
use storage::utils::lock;

use super::state::log;

/// wallet errors
#[derive(Debug)]
pub enum Error {
    CannotRetrievePrivateKeyInvalidPassword,
    CannotRetrievePrivateKey(hdwallet::Error),
    WalletLogAlreadyLocked(u32),
    WalletLogNotFound,
    WalletLogError(log::Error)
}
impl From<hdwallet::Error> for Error {
    fn from(e: hdwallet::Error) -> Self { Error::CannotRetrievePrivateKey(e) }
}
impl From<log::Error> for Error {
    fn from(e: log::Error) -> Self {
        match e {
            log::Error::LogNotFound => Error::WalletLogNotFound,
            log::Error::LockError(lock::Error::AlreadyLocked(_, process_id)) => Error::WalletLogAlreadyLocked(process_id),
            e => Error::WalletLogError(e)
        }
    }
}
