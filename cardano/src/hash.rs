//! module to provide some handy interfaces atop the hashes so we have
//! the common interfaces for the project to work with.

use std::{
    fmt,
    hash::{Hash, Hasher},
    io::{BufRead, Write},
    result,
    str::FromStr,
};

use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest;
use cryptoxide::sha3::Sha3;

use cbor_event::{self, de::Deserializer, se::Serializer};
use util::{hex, try_from_slice::TryFromSlice};

#[cfg(feature = "generic-serialization")]
use serde;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum Error {
    InvalidHashSize(usize, usize),
    HexadecimalError(hex::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidHashSize(sz, expected) => write!(
                f,
                "invalid hash size, expected {} but received {} bytes.",
                expected, sz
            ),
            &Error::HexadecimalError(_) => write!(f, "Invalid hexadecimal"),
        }
    }
}
impl From<hex::Error> for Error {
    fn from(e: hex::Error) -> Error {
        Error::HexadecimalError(e)
    }
}
impl ::std::error::Error for Error {
    fn cause(&self) -> Option<&::std::error::Error> {
        match self {
            Error::HexadecimalError(ref err) => Some(err),
            Error::InvalidHashSize(_, _) => None,
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

/// defines a blake2b object
macro_rules! define_blake2b_new {
    ($hash_ty:ty) => {
        impl $hash_ty {
            pub fn new(buf: &[u8]) -> Self {
                let mut b2b = Blake2b::new(Self::HASH_SIZE);
                let mut out = [0; Self::HASH_SIZE];
                b2b.input(buf);
                b2b.result(&mut out);
                Self::from(out)
            }
        }
    };
}
macro_rules! define_hash_object {
    ($hash_ty:ty, $constructor:expr, $hash_size:ident) => {
        impl $hash_ty {
            pub const HASH_SIZE: usize = $hash_size;

            pub fn as_hash_bytes(&self) -> &[u8; Self::HASH_SIZE] {
                &self.0
            }
        }
        impl AsRef<[u8]> for $hash_ty {
            fn as_ref(&self) -> &[u8] {
                self.0.as_ref()
            }
        }
        impl From<$hash_ty> for [u8; $hash_size] {
            fn from(bytes: $hash_ty) -> Self {
                bytes.0
            }
        }
        impl From<[u8; Self::HASH_SIZE]> for $hash_ty {
            fn from(bytes: [u8; Self::HASH_SIZE]) -> Self {
                $constructor(bytes)
            }
        }
        impl TryFromSlice for $hash_ty {
            type Error = Error;
            fn try_from_slice(slice: &[u8]) -> result::Result<Self, Self::Error> {
                if slice.len() != Self::HASH_SIZE {
                    return Err(Error::InvalidHashSize(slice.len(), Self::HASH_SIZE));
                }
                let mut buf = [0; Self::HASH_SIZE];

                buf[0..Self::HASH_SIZE].clone_from_slice(slice);
                Ok(Self::from(buf))
            }
        }
        impl Hash for $hash_ty {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.0.hash(state)
            }
        }
        impl fmt::Display for $hash_ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", hex::encode(self.as_ref()))
            }
        }
        impl fmt::Debug for $hash_ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(concat!(stringify!($hash_ty), "(0x"))?;
                write!(f, "{}", hex::encode(self.as_ref()))?;
                f.write_str(")")
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
            fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
                let bytes = reader.bytes()?;
                match Self::try_from_slice(&bytes) {
                    Ok(digest) => Ok(digest),
                    Err(Error::InvalidHashSize(sz, expected)) => {
                        Err(cbor_event::Error::NotEnough(sz, expected))
                    }
                    Err(err) => Err(cbor_event::Error::CustomError(format!(
                        "unexpected error: {:?}",
                        err
                    ))),
                }
            }
        }
        impl cbor_event::se::Serialize for $hash_ty {
            fn serialize<'se, W: Write>(
                &self,
                serializer: &'se mut Serializer<W>,
            ) -> cbor_event::Result<&'se mut Serializer<W>> {
                serializer.write_bytes(self.as_ref())
            }
        }

        #[cfg(feature = "generic-serialization")]
        impl serde::Serialize for $hash_ty {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                if serializer.is_human_readable() {
                    serializer.serialize_str(&hex::encode(self.as_ref()))
                } else {
                    serializer.serialize_bytes(&self.as_ref())
                }
            }
        }
        #[cfg(feature = "generic-serialization")]
        impl<'de> serde::Deserialize<'de> for $hash_ty {
            fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct HashVisitor;
                impl<'de> serde::de::Visitor<'de> for HashVisitor {
                    type Value = $hash_ty;

                    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                        write!(fmt, "Expecting a Blake2b_256 hash (`Hash`)")
                    }

                    fn visit_str<'a, E>(self, v: &'a str) -> result::Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match Self::Value::from_str(&v) {
                            Err(Error::HexadecimalError(err)) => Err(E::custom(format!("{}", err))),
                            Err(Error::InvalidHashSize(sz, _)) => {
                                Err(E::invalid_length(sz, &"32 bytes"))
                            }
                            Ok(h) => Ok(h),
                        }
                    }

                    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> result::Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match Self::Value::try_from_slice(v) {
                            Err(Error::InvalidHashSize(sz, _)) => {
                                Err(E::invalid_length(sz, &"32 bytes"))
                            }
                            Err(err) => panic!("unexpected error: {}", err),
                            Ok(h) => Ok(h),
                        }
                    }
                }

                if deserializer.is_human_readable() {
                    deserializer.deserialize_str(HashVisitor)
                } else {
                    deserializer.deserialize_bytes(HashVisitor)
                }
            }
        }
    };
}

pub const HASH_SIZE_224: usize = 28;

pub const HASH_SIZE_256: usize = 32;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Blake2b224([u8; HASH_SIZE_224]);
define_hash_object!(Blake2b224, Blake2b224, HASH_SIZE_224);
define_blake2b_new!(Blake2b224);

/// Blake2b 256 bits
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Blake2b256([u8; HASH_SIZE_256]);
define_hash_object!(Blake2b256, Blake2b256, HASH_SIZE_256);
define_blake2b_new!(Blake2b256);

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Sha3_256([u8; HASH_SIZE_256]);
define_hash_object!(Sha3_256, Sha3_256, HASH_SIZE_256);
impl Sha3_256 {
    pub fn new(buf: &[u8]) -> Self {
        let mut sh3 = Sha3::sha3_256();
        let mut out = [0; Self::HASH_SIZE];
        sh3.input(buf.as_ref());
        sh3.result(&mut out);
        Self::from(out)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cbor_event;

    #[test]
    fn cbor_encode_decode_blake2b_224() {
        assert!(cbor_event::test_encode_decode(&Blake2b256::new([0; 512].as_ref())).unwrap())
    }

    #[test]
    fn cbor_encode_decode_blake2b_256() {
        assert!(cbor_event::test_encode_decode(&Blake2b256::new([0; 256].as_ref())).unwrap())
    }

    #[test]
    fn debug_blake2b_224() {
        let h = Blake2b224::new([0; 28].as_ref());
        assert_eq!(
            format!("{:?}", h),
            "Blake2b224(0x317512db8239e1f9c2549b04e8071f965983c938d3e649cec78532c7)",
        );
    }
}
