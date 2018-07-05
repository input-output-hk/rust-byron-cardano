use std::{fmt, result};

use cryptoxide::digest::Digest;
use cryptoxide::blake2b::Blake2b;

use util::hex;
use cbor_event::{self, de::RawCbor};

use serde;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
    InvalidHashSize(usize),
    HexadecimalError(hex::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidHashSize(sz) => {
                write!(f, "invalid hash size, expected {} but received {} bytes.", HASH_SIZE, sz)
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

pub const HASH_SIZE : usize = 32;

/// Blake2b 256 bits
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Blake2b256([u8;HASH_SIZE]);
impl AsRef<[u8]> for Blake2b256 {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl Blake2b256 {
    pub fn new(buf: &[u8]) -> Self
    {
        let mut b2b = Blake2b::new(HASH_SIZE);
        let mut out = [0;HASH_SIZE];
        b2b.input(buf);
        b2b.result(&mut out);
        Self::from_bytes(out)
    }

    pub fn bytes<'a>(&'a self) -> &'a [u8;HASH_SIZE] { &self.0 }
    pub fn into_bytes(self) -> [u8;HASH_SIZE] { self.0 }

    pub fn from_bytes(bytes :[u8;HASH_SIZE]) -> Self { Blake2b256(bytes) }
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != HASH_SIZE { return Err(Error::InvalidHashSize(bytes.len())); }
        let mut buf = [0;HASH_SIZE];

        buf[0..HASH_SIZE].clone_from_slice(bytes);
        Ok(Self::from_bytes(buf))
    }
    pub fn from_hex<S: AsRef<str>>(hex: &S) -> Result<Self> {
        let bytes = hex::decode(hex.as_ref())?;
        Self::from_slice(&bytes)
    }
}
impl fmt::Debug for Blake2b256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..]))
    }
}
impl fmt::Display for Blake2b256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..]))
    }
}
impl cbor_event::de::Deserialize for Blake2b256 {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let bytes = raw.bytes()?;
        match Blake2b256::from_slice(&bytes) {
            Ok(digest) => Ok(digest),
            Err(Error::InvalidHashSize(sz)) => Err(cbor_event::Error::NotEnough(sz, HASH_SIZE)),
            Err(err) => Err(cbor_event::Error::CustomError(format!("unexpected error: {:?}", err))),
        }
    }
}
impl cbor_event::se::Serialize for Blake2b256 {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_bytes(&self.0)
    }
}
impl serde::Serialize for Blake2b256
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(self.as_ref()))
        } else {
            serializer.serialize_bytes(&self.as_ref())
        }
    }
}
struct HashVisitor();
impl HashVisitor { fn new() -> Self { HashVisitor {} } }
impl<'de> serde::de::Visitor<'de> for HashVisitor {
    type Value = Blake2b256;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expecting a Blake2b_256 hash (`Hash`)")
    }

    fn visit_str<'a, E>(self, v: &'a str) -> result::Result<Self::Value, E>
        where E: serde::de::Error
    {
        match Self::Value::from_hex(&v) {
            Err(Error::HexadecimalError(err)) => Err(E::custom(format!("{}", err))),
            Err(Error::InvalidHashSize(sz)) => Err(E::invalid_length(sz, &"32 bytes")),
            Ok(h) => Ok(h)
        }
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> result::Result<Self::Value, E>
        where E: serde::de::Error
    {
        match Self::Value::from_slice(v) {
            Err(Error::InvalidHashSize(sz)) => Err(E::invalid_length(sz, &"32 bytes")),
            Err(err) => panic!("unexpected error: {}", err),
            Ok(h) => Ok(h)
        }
    }
}
impl<'de> serde::Deserialize<'de> for Blake2b256
{
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(HashVisitor::new())
        } else {
            deserializer.deserialize_bytes(HashVisitor::new())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cbor_event::{self};

    #[test]
    fn encode_decode() {
        assert!(cbor_event::test_encode_decode(&Blake2b256::new([0;32].as_ref())).unwrap())
    }
}
