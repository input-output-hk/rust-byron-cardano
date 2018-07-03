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

/// Raw bytes as a slice of given length of the original buffer.
///
/// [`Deref`]: https://doc.rust-lang.org/std/ops/trait.Deref.html
/// [`[u8]`]: https://doc.rust-lang.org/nightly/std/primitive.slice.html
///
/// # Deref
///
/// `Bytes` implements [`Deref`]`<Target = [u8]>` and so inherits all of
/// [`[u8]`]'s methods. In addition it means that you can pass a `Bytes` to
/// a function which takes a [`[u8]`] by using an ampersand (`&`):
///
/// ```
/// use cbor_event::{Bytes};
/// fn take_slice(slice: &[u8]) { /* ... */ }
///
/// let original_source : &'static [u8] = &[0u8,1u8,2u8];
/// let bytes = Bytes::from(original_source);
/// take_slice(&bytes);
/// ```
///
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Bytes<'a>(&'a [u8]);
impl<'a> Bytes<'a> {
    /// handy function to explicitely access the underlying slice
    /// but bound to the original lifetime, not the lifetime of
    /// the `Bytes` itself.
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

/// CBOR special (as in Special Primary Type).
#[derive(Debug, PartialEq, PartialOrd, Copy, Clone)]
pub enum Special {
    Bool(bool),
    Null,
    Undefined,
    /// Free to use values within: `[0..=13]` and `[24..=31]`
    Unassigned(u8),

    /// Float is not fully supported in this library and it is advised
    /// to avoid using it for now.
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
        for i in 0b0000_0000..=0b0001_1111 {
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
