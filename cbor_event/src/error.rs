use types::Type;

/// all expected error for cbor parsing and serialising
#[derive(Debug)]
pub enum Error {
    ExpectedU8,
    ExpectedU16,
    ExpectedU32,
    ExpectedU64,
    ExpectedI8,
    ExpectedI16,
    ExpectedI32,
    ExpectedI64,
    /// not enough data, the first element is the actual size, the second is
    /// the expected size.
    NotEnough(usize, usize),
    /// Were expecting a different [`Type`](../enum.Type.html). The first
    /// element is the expected type, the second is the current type.
    Expected(Type, Type),
    /// this may happens when deserialising a [`RawCbor`](../de/struct.RawCbor.html);
    UnknownLenType(u8),
    IndefiniteLenNotSupported(Type),
    InvalidTextError(::std::string::FromUtf8Error),
    CannotParse(Type, Vec<u8>),
    IoError(::std::io::Error),

    CustomError(String)
}
impl From<::std::string::FromUtf8Error> for Error {
    fn from(e: ::std::string::FromUtf8Error) -> Self { Error::InvalidTextError(e) }
}
impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Self { Error::IoError(e) }
}
