use error::Error;

/// `Result` type for CBOR serialisation and deserialisation.
pub type Result<T> = ::std::result::Result<T, Error>;
