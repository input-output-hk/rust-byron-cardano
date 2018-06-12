use raw_cbor::{self, de::RawCbor, se::{Serializer}};
use std::{fmt, result};
use util::hex;

const SIGNATURE_SIZE: usize = 64;

// XXX Error and Result copied with slight modifications from redeem.rs
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
    InvalidSignatureSize(usize),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidSignatureSize(sz) => write!(
                f,
                "invalid Signature size, expected {} but received {} bytes.",
                SIGNATURE_SIZE, sz
            ),
        }
    }
}
pub type Result<T> = result::Result<T, Error>;

// TODO: decode to 35 bytes public-key http://hackage.haskell.org/package/pvss/docs/Crypto-SCRAPE.html#t:Point
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublicKey(Vec<u8>);
impl raw_cbor::se::Serialize for PublicKey {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.write_bytes(&self.0)
    }
}
impl raw_cbor::de::Deserialize for PublicKey {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let bytes = raw.bytes()?;
        Ok(PublicKey(Vec::from(bytes.as_ref())))
    }
}

// XXX Signature and impls copied with slight modifications from redeem.rs
pub struct Signature([u8; SIGNATURE_SIZE]);
impl Clone for Signature {
    fn clone(&self) -> Self {
        Self::from_slice(self.as_ref()).unwrap()
    }
}
impl Signature {
    pub fn from_bytes(bytes: [u8; SIGNATURE_SIZE]) -> Self {
        Signature(bytes)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != SIGNATURE_SIZE {
            return Err(Error::InvalidSignatureSize(bytes.len()));
        }
        let mut buf = [0; SIGNATURE_SIZE];
        buf[0..SIGNATURE_SIZE].clone_from_slice(bytes);
        Ok(Self::from_bytes(buf))
    }

    pub fn to_bytes<'a>(&'a self) -> &'a [u8;SIGNATURE_SIZE] { &self.0 }
}
impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl raw_cbor::se::Serialize for Signature {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        serializer.write_bytes(self.as_ref())
    }
}
impl raw_cbor::de::Deserialize for Signature {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        match Self::from_slice(raw.bytes()?.as_ref()) {
            Ok(sig) => Ok(sig),
            Err(Error::InvalidSignatureSize(sz)) => {
                Err(raw_cbor::Error::NotEnough(SIGNATURE_SIZE, sz))
            },
            Err(err) => Err(raw_cbor::Error::CustomError(format!("unexpected error: {}", err))),
        }
    }
}
