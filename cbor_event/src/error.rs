use std::{fmt, error};

use types::Type;
use len;

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
    ExpectedSetTag,
    /// this may happens when deserialising a [`RawCbor`](../de/struct.RawCbor.html);
    UnknownLenType(u8),
    IndefiniteLenNotSupported(Type),
    WrongLen(u64, len::Len, &'static str),
    InvalidTextError(::std::string::FromUtf8Error),
    CannotParse(Type, Vec<u8>),
    IoError(::std::io::Error),
    TrailingData,

    CustomError(String)
}
impl From<::std::string::FromUtf8Error> for Error {
    fn from(e: ::std::string::FromUtf8Error) -> Self { Error::InvalidTextError(e) }
}
impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Self { Error::IoError(e) }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            ExpectedU8 => write!(f, "Invalid cbor: expected 8bit long unsigned integer"),
            ExpectedU16 => write!(f, "Invalid cbor: expected 16bit long unsigned integer"),
            ExpectedU32 => write!(f, "Invalid cbor: expected 32bit long unsigned integer"),
            ExpectedU64 => write!(f, "Invalid cbor: expected 64bit long unsigned integer"),
            ExpectedI8 => write!(f, "Invalid cbor: expected 8bit long negative integer"),
            ExpectedI16 => write!(f, "Invalid cbor: expected 16bit long negative integer"),
            ExpectedI32 => write!(f, "Invalid cbor: expected 32bit long negative integer"),
            ExpectedI64 => write!(f, "Invalid cbor: expected 64bit long negative integer"),
            NotEnough(got, exp) => write!(f, "Invalid cbor: not enough bytes, expect {} bytes but received {} bytes.", exp, got),
            Expected(exp, got) => write!(f, "Invalid cbor: not the right type, expected `{:?}' byte received `{:?}'.", exp, got),
            ExpectedSetTag => write!(f, "Invalid cbor: expected set tag"),
            UnknownLenType(byte) => write!(f, "Invalid cbor: not the right sub type: 0b{:05b}", byte),
            IndefiniteLenNotSupported(t) => write!(f, "Invalid cbor: indefinite length not supported for cbor object of type `{:?}'.", t),
            WrongLen(expected_len, actual_len, error_location) =>
                write!(f, "Invalid cbor: expected tuple '{}' of length {} but got length {:?}.",
                       error_location, expected_len, actual_len),
            InvalidTextError(_utf8_error) => write!(f, "Invalid cbor: expected a valid utf8 string text."),
            CannotParse(t, bytes) => write!(f, "Invalid cbor: cannot parse the cbor object `{:?}' with the following bytes {:?}", t, bytes),
            IoError(_io_error) => write!(f, "Invalid cbor: I/O error"),
            TrailingData => write!(f, "Unexpected trailing data in CBOR"),
            CustomError(err) => write!(f, "Invalid cbor: {}", err)
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::IoError(ref error) => Some(error),
            Error::InvalidTextError(ref error) => Some(error),
            _ => None
        }
    }
}
