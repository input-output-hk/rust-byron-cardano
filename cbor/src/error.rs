use types::Type;

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
    NotEnough(usize, usize),
    Expected(Type, Type),
    UnknownLenType(u8),
    IndefiniteLenNotSupported(Type),
    InvalidTextError(::std::string::FromUtf8Error),
    CannotParse(Type, Vec<u8>),

    CustomError(String)
}
impl From<::std::string::FromUtf8Error> for Error {
    fn from(e: ::std::string::FromUtf8Error) -> Self { Error::InvalidTextError(e) }
}
