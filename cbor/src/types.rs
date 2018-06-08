use std::{ops::{Deref}};
use result::Result;
use error::Error;

/// CBOR Major Types
///
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Type {
    UnsignedInteger,
    NegativeInteger,
    Bytes,
    Text,
    Array,
    Map,
    Tag,
    Special
}
impl Type {
    pub fn to_byte(self, len: u8) -> u8 {
        assert!(len <= 0b0001_1111);

        len | match self {
            Type::UnsignedInteger => 0b0000_0000,
            Type::NegativeInteger => 0b0010_0000,
            Type::Bytes           => 0b0100_0000,
            Type::Text            => 0b0110_0000,
            Type::Array           => 0b1000_0000,
            Type::Map             => 0b1010_0000,
            Type::Tag             => 0b1100_0000,
            Type::Special         => 0b1110_0000
        }
    }
    pub fn from_byte(byte: u8) -> Type {
        match byte & 0b1110_0000 {
            0b0000_0000 => Type::UnsignedInteger,
            0b0010_0000 => Type::NegativeInteger,
            0b0100_0000 => Type::Bytes,
            0b0110_0000 => Type::Text,
            0b1000_0000 => Type::Array,
            0b1010_0000 => Type::Map,
            0b1100_0000 => Type::Tag,
            0b1110_0000 => Type::Special,
            _           => unreachable!()
        }
    }
}
impl From<u8> for Type {
    fn from(byte: u8) -> Type { Type::from_byte(byte) }
}

/// CBOR Unsigned Integer
///
/// it can be any unsigned integer: u8, u16, u32 or u64. However
/// the decode does not retrain the details of the encoding.
/// It is the user's responsibility to check for integer overflow.
///
/// # Example
///
/// ```
/// use raw_cbor::de::{*};
///
/// let bytes = vec![0x1A, 0x31, 0x6D, 0xD6, 0xE6];
/// let mut raw = RawCbor::from(&bytes);
///
/// let integer = raw.unsigned_integer().unwrap();
/// assert!(*integer <= u32::max_value() as u64);
/// ```
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct UnsignedInteger(u64);
impl From<u64> for UnsignedInteger { fn from(v: u64) -> Self { UnsignedInteger(v) } }
impl Deref for UnsignedInteger {
    type Target = u64;
    fn deref(&self) -> &Self::Target { &self.0 }
}

/// CBOR Unsigned Integer
///
/// it can be any signed integer: i8, i16, i32 or i64. However
/// the decode does not retrain the details of the encoding.
/// It is the user's responsibility to check for integer overflow.
///
/// # Example
///
/// ```
/// use raw_cbor::de::{*};
///
/// let bytes = vec![0x3A, 0x31, 0x6C, 0xC5, 0x76];
/// let mut raw = RawCbor::from(&bytes);
///
/// let integer = raw.negative_integer().unwrap();
/// assert!(*integer >= i32::min_value() as i64);
/// ```
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct NegativeInteger(i64);
impl From<i64> for NegativeInteger { fn from(v: i64) -> Self { NegativeInteger(v) } }
impl Deref for NegativeInteger {
    type Target = i64;
    fn deref(&self) -> &Self::Target { &self.0 }
}

/// CBOR Raw bytes
///
/// Simply a slice of a given length of the original buffer.
///
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Bytes<'a>(&'a [u8]);
impl<'a> Bytes<'a> {
    pub fn bytes<'b>(&'b self) -> &'a [u8] { self.0 }
}
impl<'a> From<&'a [u8]> for Bytes<'a> { fn from(v: &'a[u8]) -> Self { Bytes(v) } }
impl<'a> Deref for Bytes<'a> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target { self.0 }
}
impl<'a> AsRef<[u8]> for Bytes<'a> {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}

/// CBOR UTF8 String
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Text(String);
impl From<String> for Text { fn from(v: String) -> Self { Text(v) } }
impl Deref for Text {
    type Target = str;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl AsRef<str> for Text {
    fn as_ref(&self) -> &str { self.0.as_ref() }
}

pub struct Tag(pub u64);
impl From<u64> for Tag { fn from(v: u64) -> Self { Tag(v) } }
impl Deref for Tag {
    type Target = u64;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Special {
    Bool(bool),
    Null,
    Undefined,
    /// Free to use values within: `[0..=13]` and `[24..=31]`
    Unassigned(u8),

    #[warn()]
    Float(f64),
    /// mark the stop of a given indefinite-length item
    Break
}
impl Special {
    #[inline]
    pub fn unwrap_bool(&self) -> Result<bool> {
        match self {
            Special::Bool(b) => Ok(*b),
            _                => Err(Error::CustomError(format!("Expected Special::Bool, received {:?}", self)))
        }
    }

    #[inline]
    pub fn unwrap_null(&self) -> Result<()> {
        match self {
            Special::Null => Ok(()),
            _             => Err(Error::CustomError(format!("Expected Special::Null, received {:?}", self)))
        }
    }

    #[inline]
    pub fn unwrap_undefined(&self) -> Result<()> {
        match self {
            Special::Undefined => Ok(()),
            _                  => Err(Error::CustomError(format!("Expected Special::Undefined, received {:?}", self)))
        }
    }

    #[inline]
    pub fn unwrap_unassigned(&self) -> Result<u8> {
        match self {
            Special::Unassigned(v) => Ok(*v),
            _                      => Err(Error::CustomError(format!("Expected Special::Unassigned, received {:?}", self)))
        }
    }

    #[inline]
    pub fn unwrap_float(&self) -> Result<f64> {
        match self {
            Special::Float(f) => Ok(*f),
            _                 => Err(Error::CustomError(format!("Expected Special::Float, received {:?}", self)))
        }
    }

    #[inline]
    pub fn unwrap_break(&self) -> Result<()> {
        match self {
            Special::Break => Ok(()),
            _              => Err(Error::CustomError(format!("Expected Special::Break, received {:?}", self)))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_type_byte_encoding() {
        for i in 0b0000_0000..0b0001_1111 {
            assert!(Type::UnsignedInteger == Type::from_byte(Type::to_byte(Type::UnsignedInteger, i)));
            assert!(Type::NegativeInteger == Type::from_byte(Type::to_byte(Type::NegativeInteger, i)));
            assert!(Type::Bytes           == Type::from_byte(Type::to_byte(Type::Bytes,           i)));
            assert!(Type::Text            == Type::from_byte(Type::to_byte(Type::Text,            i)));
            assert!(Type::Array           == Type::from_byte(Type::to_byte(Type::Array,           i)));
            assert!(Type::Map             == Type::from_byte(Type::to_byte(Type::Map,             i)));
            assert!(Type::Tag             == Type::from_byte(Type::to_byte(Type::Tag,             i)));
            assert!(Type::Special         == Type::from_byte(Type::to_byte(Type::Special,         i)));
        }
    }
}
