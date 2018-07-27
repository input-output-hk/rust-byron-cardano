use cardano::hdwallet;

/// wallet errors
pub enum Error {
    CannotRetrievePrivateKeyInvalidPassword,
    CannotRetrievePrivateKey(hdwallet::Error),
}
impl From<hdwallet::Error> for Error {
    fn from(e: hdwallet::Error) -> Self { Error::CannotRetrievePrivateKey(e) }
}
