#[derive(Debug)]
pub enum Error {
    IoError(::std::io::Error),
    CborError(::cbor_event::Error),
    StorageError(::storage::Error),
}
impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Self { Error::IoError(e) }
}
impl From<::cbor_event::Error> for Error {
    fn from(e: ::cbor_event::Error) -> Self { Error::CborError(e) }
}
impl From<::storage::Error> for Error {
    fn from(e: ::storage::Error) -> Self { Error::StorageError(e) }
}

pub type Result<T> = ::std::result::Result<T, Error>;
