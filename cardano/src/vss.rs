use cbor_event::{self, de::Deserializer, se::Serializer};
use std::{
    fmt,
    io::{BufRead, Write},
    result,
};
use util::hex;

const SIGNATURE_SIZE: usize = 64;

// XXX Error and Result copied with slight modifications from redeem.rs
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
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
impl ::std::error::Error for Error {}

// TODO: decode to 35 bytes public-key http://hackage.haskell.org/package/pvss/docs/Crypto-SCRAPE.html#t:Point
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublicKey(pub Vec<u8>);
impl cbor_event::se::Serialize for PublicKey {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_bytes(&self.0)
    }
}
impl cbor_event::de::Deserialize for PublicKey {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        let bytes = reader.bytes()?;
        Ok(PublicKey(bytes))
    }
}

// XXX Signature and impls copied with slight modifications from redeem.rs
pub struct Signature([u8; SIGNATURE_SIZE]);
impl Clone for Signature {
    fn clone(&self) -> Self {
        let mut bytes = [0; SIGNATURE_SIZE];
        bytes.copy_from_slice(&self.0);
        Signature(bytes)
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

    pub fn to_bytes<'a>(&'a self) -> &'a [u8; SIGNATURE_SIZE] {
        &self.0
    }
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
impl cbor_event::se::Serialize for Signature {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_bytes(self.as_ref())
    }
}
impl cbor_event::de::Deserialize for Signature {
    fn deserialize<R: BufRead>(reader: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        match Self::from_slice(reader.bytes()?.as_ref()) {
            Ok(sig) => Ok(sig),
            Err(Error::InvalidSignatureSize(sz)) => {
                Err(cbor_event::Error::NotEnough(SIGNATURE_SIZE, sz))
            } // Err(err) => Err(cbor_event::Error::CustomError(format!("unexpected error: {}", err))),
        }
    }
}
