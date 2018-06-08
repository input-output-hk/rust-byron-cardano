use cbor;
use cbor::ExtendedResult;
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(cbor::Value);
impl cbor::CborValue for PublicKey {
    fn encode(&self) -> cbor::Value {
        unimplemented!() // FIXME crashes
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        Ok(PublicKey(value))
    }
}
// XXX: Bogus Ord implementation to satisfy PublicKey being used as a map-key (even though we don't actually decode it yet!)
impl Ord for PublicKey {
    fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        format!("{:?}", self).cmp(&format!("{:?}", other))
    }
}
impl PartialOrd for PublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
        Some(self.cmp(other))
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
impl cbor::CborValue for Signature {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Bytes(cbor::Bytes::from_slice(self.as_ref()))
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value
            .bytes()
            .and_then(|bytes| match Self::from_slice(bytes.as_ref()) {
                Ok(digest) => Ok(digest),
                Err(Error::InvalidSignatureSize(_)) => {
                    cbor::Result::bytes(bytes, cbor::Error::InvalidSize(SIGNATURE_SIZE))
                }
                Err(err) => panic!("unexpected error: {}", err),
            })
            .embed("while decoding Vss's Signature")
    }
}
