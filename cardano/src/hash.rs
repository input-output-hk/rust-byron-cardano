//! module to provide some handy interfaces atop the hashes so we have
//! the common interfaces for the project to work with.

use std::{fmt, result, str::{FromStr}, ops::{Deref}};

use cryptoxide::digest::Digest;
use cryptoxide::blake2b::Blake2b;
use cryptoxide::sha3::Sha3;

use util::{hex, try_from_slice::{TryFromSlice}};
use cbor_event::{self, de::RawCbor};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
    InvalidHashSize(usize, usize),
    HexadecimalError(hex::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidHashSize(sz, expected) => {
                write!(f, "invalid hash size, expected {} but received {} bytes.", expected, sz)
            },
            &Error::HexadecimalError(err) => {
                write!(f, "Invalid hexadecimal input: {}", err)
            }
        }
    }
}
impl From<hex::Error> for Error {
    fn from(e: hex::Error) -> Error { Error::HexadecimalError(e) }
}

pub type Result<T> = result::Result<T, Error>;


/// defines a blake2b object
macro_rules! define_blake2b_new {
    ($hash_ty:ty) => {
        impl $hash_ty {
            pub fn new(buf: &[u8]) -> Self {
                let mut b2b = Blake2b::new(Self::HASH_SIZE);
                let mut out = [0;Self::HASH_SIZE];
                b2b.input(buf);
                b2b.result(&mut out);
                Self::from(out)
            }
        }
    }
}
macro_rules! define_hash_object {
    ($hash_ty:ty, $constructor:expr, $hash_size:ident) => {
        impl $hash_ty {
            pub const HASH_SIZE: usize = $hash_size;
        }
        impl Deref for $hash_ty {
            type Target = [u8; Self::HASH_SIZE];
            fn deref(&self) -> &Self::Target { &self.0 }
        }
        impl AsRef<[u8]> for $hash_ty {
            fn as_ref(&self) -> &[u8] { self.0.as_ref() }
        }
        impl From<$hash_ty> for [u8;$hash_size] {
            fn from(bytes: $hash_ty) -> Self { bytes.0 }
        }
        impl From<[u8;Self::HASH_SIZE]> for $hash_ty {
            fn from(bytes: [u8;Self::HASH_SIZE]) -> Self { $constructor(bytes) }
        }
        impl TryFromSlice for $hash_ty {
            type Error = Error;
            fn try_from_slice(slice: &[u8]) -> result::Result<Self, Self::Error> {
                if slice.len() != Self::HASH_SIZE { return Err(Error::InvalidHashSize(slice.len(), Self::HASH_SIZE)); }
                let mut buf = [0;Self::HASH_SIZE];

                buf[0..Self::HASH_SIZE].clone_from_slice(slice);
                Ok(Self::from(buf))
            }
        }
        impl fmt::Display for $hash_ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", hex::encode(self.as_ref()))
            }
        }
        impl FromStr for $hash_ty {
            type Err = Error;
            fn from_str(s: &str) -> result::Result<Self, Self::Err> {
                let bytes = hex::decode(s)?;
                Self::try_from_slice(&bytes)
            }
        }
        impl cbor_event::de::Deserialize for $hash_ty {
            fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
                let bytes = raw.bytes()?;
                match Self::try_from_slice(&bytes) {
                    Ok(digest) => Ok(digest),
                    Err(Error::InvalidHashSize(sz, expected)) => Err(cbor_event::Error::NotEnough(sz, expected)),
                    Err(err) => Err(cbor_event::Error::CustomError(format!("unexpected error: {:?}", err))),
                }
            }
        }
        impl cbor_event::se::Serialize for $hash_ty {
            fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
                serializer.write_bytes(self.as_ref())
            }
        }
    }
}

pub const HASH_SIZE_224 : usize = 28;

pub const HASH_SIZE_256 : usize = 32;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Blake2b224([u8;HASH_SIZE_224]);
define_hash_object!(Blake2b224, Blake2b224, HASH_SIZE_224);
define_blake2b_new!(Blake2b224);

/// Blake2b 256 bits
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Blake2b256([u8;HASH_SIZE_256]);
define_hash_object!(Blake2b256, Blake2b256, HASH_SIZE_256);
define_blake2b_new!(Blake2b256);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Sha3_256([u8;HASH_SIZE_256]);
define_hash_object!(Sha3_256, Sha3_256, HASH_SIZE_256);
impl Sha3_256 {
    pub fn new(buf: &[u8]) -> Self {
        let mut sh3 = Sha3::sha3_256();
        let mut out = [0;Self::HASH_SIZE];
        sh3.input(buf.as_ref());
        sh3.result(&mut out);
        Self::from(out)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use cbor_event::{self};

    #[test]
    fn cbor_encode_decode_blake2b_224() {
        assert!(cbor_event::test_encode_decode(&Blake2b256::new([0;512].as_ref())).unwrap())
    }

    #[test]
    fn cbor_encode_decode_blake2b_256() {
        assert!(cbor_event::test_encode_decode(&Blake2b256::new([0;256].as_ref())).unwrap())
    }

}
