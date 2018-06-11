extern crate rcw;

use self::rcw::digest::Digest;
use self::rcw::sha2::Sha512;
use self::rcw::hmac::Hmac;
use self::rcw::mac::Mac;
use self::rcw::curve25519::{Fe, GeP3, ge_scalarmult_base};
use self::rcw::ed25519::signature_extended;
use self::rcw::ed25519;
use self::rcw::util::fixed_time_eq;

use bip39;

use std::{fmt, result};
use std::marker::PhantomData;
use util::{hex};
use cbor;
use cbor::{ExtendedResult};

use serde;

pub const SEED_SIZE: usize = 32;
pub const XPRV_SIZE: usize = 96;
pub const XPUB_SIZE: usize = 64;
pub const SIGNATURE_SIZE: usize = 64;

pub const PUBLIC_KEY_SIZE: usize = 32;
pub const CHAIN_CODE_SIZE: usize = 32;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
    InvalidSeedSize(usize),
    InvalidXPrvSize(usize),
    InvalidXPubSize(usize),
    InvalidSignatureSize(usize),
    HexadecimalError(hex::Error),
    ExpectedSoftDerivation,
    InvalidDerivation
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidSeedSize(sz) => {
               write!(f, "Invalid Seed Size, expected {} bytes, but received {} bytes.", SEED_SIZE, sz)
            },
            &Error::InvalidXPrvSize(sz) => {
               write!(f, "Invalid XPrv Size, expected {} bytes, but received {} bytes.", XPRV_SIZE, sz)
            },
            &Error::InvalidXPubSize(sz) => {
               write!(f, "Invalid XPub Size, expected {} bytes, but received {} bytes.", XPUB_SIZE, sz)
            },
            &Error::InvalidSignatureSize(sz) => {
               write!(f, "Invalid Signature Size, expected {} bytes, but received {} bytes.", SIGNATURE_SIZE, sz)
            },
            &Error::HexadecimalError(err) => {
               write!(f, "Invalid hexadecimal: {}.", err)
            },
            &Error::ExpectedSoftDerivation => {
               write!(f, "expected soft derivation")
            },
            &Error::InvalidDerivation => {
               write!(f, "invalid derivation")
            },
        }
    }
}
impl From<hex::Error> for Error {
    fn from(e: hex::Error) -> Error { Error::HexadecimalError(e) }
}

pub type Result<T> = result::Result<T, Error>;

/// Ed25519-bip32 Scheme Derivation version
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DerivationScheme {
    V1,
    V2,
}
impl Default for DerivationScheme {
    fn default() -> Self { DerivationScheme::V2 }
}

/// Seed used to generate the root private key of the HDWallet.
///
#[derive(Serialize, Deserialize, Debug)]
pub struct Seed([u8; SEED_SIZE]);
impl Seed {
    /// create a Seed by taking ownership of the given array
    ///
    /// ```
    /// use wallet_crypto::hdwallet::{Seed, SEED_SIZE};
    ///
    /// let bytes = [0u8;SEED_SIZE];
    /// let seed  = Seed::from_bytes(bytes);
    ///
    /// assert!(seed.as_ref().len() == SEED_SIZE);
    /// ```
    pub fn from_bytes(buf: [u8;SEED_SIZE]) -> Self { Seed(buf) }

    /// create a Seed by copying the given slice into a new array
    ///
    /// ```
    /// use wallet_crypto::hdwallet::{Seed, SEED_SIZE};
    ///
    /// let bytes = [0u8;SEED_SIZE];
    /// let wrong = [0u8;31];
    ///
    /// assert!(Seed::from_slice(&wrong[..]).is_err());
    /// assert!(Seed::from_slice(&bytes[..]).is_ok());
    /// ```
    pub fn from_slice(buf: &[u8]) -> Result<Self> {
        if buf.len() != SEED_SIZE {
            return Err(Error::InvalidSeedSize(buf.len()));
        }
        let mut v = [0u8;SEED_SIZE];
        v[..].clone_from_slice(buf);
        Ok(Seed::from_bytes(v))
    }
}
impl AsRef<[u8]> for Seed {
    fn as_ref(&self) -> &[u8] { &self.0 }
}

/// HDWallet private key
///
pub struct XPrv([u8; XPRV_SIZE]);
impl XPrv {
    /// create the Root private key `XPrv` of the HDWallet associated to this `Seed`
    ///
    /// This is a deterministic construction. The `XPrv` returned will always be the
    /// same for the same given `Seed`.
    ///
    /// ```
    /// use wallet_crypto::hdwallet::{Seed, SEED_SIZE, XPrv, XPRV_SIZE};
    ///
    /// let seed = Seed::from_bytes([0u8; SEED_SIZE]);
    /// let expected_xprv = XPrv::from_hex("301604045de9138b8b23b6730495f7e34b5151d29ba3456bc9b332f6f084a551d646bc30cf126fa8ed776c05a8932a5ab35c8bac41eb01bb9a16cfe229b94b405d3661deb9064f2d0e03fe85d68070b2fe33b4916059658e28ac7f7f91ca4b12").unwrap();
    ///
    /// assert_eq!(expected_xprv, XPrv::generate_from_seed(&seed));
    /// ```
    ///
    pub fn generate_from_seed(seed: &Seed) -> Self {
        let mut mac = Hmac::new(Sha512::new(), seed.as_ref());

        let mut iter = 1;
        let mut out = [0u8; XPRV_SIZE];

        loop {
            let s = format!("Root Seed Chain {}", iter);
            mac.reset();
            mac.input(s.as_bytes());
            let mut block = [0u8; 64];
            mac.raw_result(&mut block);
            mk_ed25519_extended(&mut out[0..64], &block[0..32]);

            if (out[31] & 0x20) == 0 {
                out[64..96].clone_from_slice(&block[32..64]);
                break;
            }
            iter = iter + 1;
        }

        Self::from_bytes(out)
    }

    pub fn generate_from_daedalus_seed(seed: &Seed) -> Self {
        let bytes = cbor::encode_to_cbor(&cbor::Value::Bytes(cbor::Bytes::from_slice(seed.as_ref()))).unwrap();
        let mut mac = Hmac::new(Sha512::new(), &bytes);

        let mut iter = 1;
        let mut out = [0u8; XPRV_SIZE];

        loop {
            let s = format!("Root Seed Chain {}", iter);
            mac.reset();
            mac.input(s.as_bytes());
            let mut block = [0u8; 64];
            mac.raw_result(&mut block);
            mk_ed25519_extended(&mut out[0..64], &block[0..32]);

            if (out[31] & 0x20) == 0 {
                out[64..96].clone_from_slice(&block[32..64]);
                break;
            }
            iter = iter + 1;
        }

        Self::from_bytes(out)
    }

    pub fn generate_from_bip39(bytes: &bip39::Seed) -> Self {
        let mut out = [0u8; XPRV_SIZE];

        mk_ed25519_extended(&mut out[0..64], &bytes.as_ref()[0..32]);
        out[31] &= 0b1101_1111; // set 3rd highest bit to 0 as per the spec
        out[64..96].clone_from_slice(&bytes.as_ref()[32..64]);

        Self::from_bytes(out)
    }

    /// create a `XPrv` by taking ownership of the given array
    ///
    pub fn from_bytes(bytes: [u8;XPRV_SIZE]) -> Self { XPrv(bytes) }

    /// create a `XPrv` from the given slice. This slice must be of size `XPRV_SIZE`
    /// otherwise it will return `Result`.
    ///
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != XPRV_SIZE {
            return Err(Error::InvalidXPrvSize(bytes.len()));
        }
        let mut buf = [0u8;XPRV_SIZE];
        buf[..].clone_from_slice(bytes);
        Ok(XPrv::from_bytes(buf))
    }

    /// create a `XPrv` from a given hexadecimal string
    ///
    /// ```
    /// use wallet_crypto::hdwallet::{XPrv};
    ///
    /// let xprv = XPrv::from_hex("301604045de9138b8b23b6730495f7e34b5151d29ba3456bc9b332f6f084a551d646bc30cf126fa8ed776c05a8932a5ab35c8bac41eb01bb9a16cfe229b94b405d3661deb9064f2d0e03fe85d68070b2fe33b4916059658e28ac7f7f91ca4b12");
    ///
    /// assert!(xprv.is_ok());
    /// ```
    ///
    pub fn from_hex(hex: &str) -> Result<Self> {
        let input = hex::decode(hex)?;
        Self::from_slice(&input)
    }

    /// get te associated `XPub`
    ///
    /// ```
    /// use wallet_crypto::hdwallet::{XPrv, XPub};
    ///
    /// let xprv = XPrv::from_hex("301604045de9138b8b23b6730495f7e34b5151d29ba3456bc9b332f6f084a551d646bc30cf126fa8ed776c05a8932a5ab35c8bac41eb01bb9a16cfe229b94b405d3661deb9064f2d0e03fe85d68070b2fe33b4916059658e28ac7f7f91ca4b12").unwrap();
    ///
    /// let xpub = xprv.public();
    /// ```
    pub fn public(&self) -> XPub {
        let pk = mk_public_key(&self.as_ref()[0..64]);
        let mut out = [0u8; XPUB_SIZE];
        out[0..32].clone_from_slice(&pk);
        out[32..64].clone_from_slice(&self.as_ref()[64..]);
        XPub::from_bytes(out)
    }

    /// sign the given message with the `XPrv`.
    ///
    /// ```
    /// use wallet_crypto::hdwallet::{XPrv, XPub, Signature};
    ///
    /// let xprv = XPrv::from_hex("301604045de9138b8b23b6730495f7e34b5151d29ba3456bc9b332f6f084a551d646bc30cf126fa8ed776c05a8932a5ab35c8bac41eb01bb9a16cfe229b94b405d3661deb9064f2d0e03fe85d68070b2fe33b4916059658e28ac7f7f91ca4b12").unwrap();
    /// let msg = b"Some message...";
    ///
    /// let signature : Signature<String> = xprv.sign(msg);
    /// assert!(xprv.verify(msg, &signature));
    /// ```
    pub fn sign<T>(&self, message: &[u8]) -> Signature<T> {
        Signature::from_bytes(signature_extended(message, &self.as_ref()[0..64]))
    }

    /// verify a given signature
    ///
    pub fn verify<T>(&self, message: &[u8], signature: &Signature<T>) -> bool {
        let xpub = self.public();
        xpub.verify(message, signature)
    }

    pub fn derive(&self, scheme: DerivationScheme, index: DerivationIndex) -> Self {
        derive_private(self, index, scheme)
    }
}
impl PartialEq for XPrv {
    fn eq(&self, rhs: &XPrv) -> bool { fixed_time_eq(self.as_ref(), rhs.as_ref()) }
}
impl Eq for XPrv {}
impl Clone for XPrv {
    fn clone(&self) -> Self { Self::from_slice(self.as_ref()).expect("it is already a safely constructed XPrv") }
}
impl fmt::Debug for XPrv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl fmt::Display for XPrv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl AsRef<[u8]> for XPrv {
    fn as_ref(&self) -> &[u8] { &self.0 }
}
impl serde::Serialize for XPrv
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(self.as_ref()))
        } else {
            serializer.serialize_bytes(self.as_ref())
        }
    }
}
struct XPrvVisitor();
impl XPrvVisitor { fn new() -> Self { XPrvVisitor {} } }
impl<'de> serde::de::Visitor<'de> for XPrvVisitor {
    type Value = XPrv;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expecting an Extended Private Key (`XPrv`) of {} bytes.", XPRV_SIZE)
    }

    fn visit_str<'a, E>(self, v: &'a str) -> result::Result<Self::Value, E>
        where E: serde::de::Error
    {
        match XPrv::from_hex(v) {
            Err(Error::HexadecimalError(err)) => Err(E::custom(format!("{}", err))),
            Err(Error::InvalidXPrvSize(sz)) => Err(E::invalid_length(sz, &"96 bytes")),
            Err(err) => panic!("unexpected error happended: {}", err),
            Ok(xpub) => Ok(xpub)
        }
    }
    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> result::Result<Self::Value, E>
        where E: serde::de::Error
    {
        match XPrv::from_slice(v) {
            Err(Error::InvalidXPrvSize(sz)) => Err(E::invalid_length(sz, &"96 bytes")),
            Err(err) => panic!("unexpected error happended: {}", err),
            Ok(xpub) => Ok(xpub)
        }
    }
}
impl<'de> serde::Deserialize<'de> for XPrv
{
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(XPrvVisitor::new())
        } else {
            deserializer.deserialize_bytes(XPrvVisitor::new())
        }
    }
}

#[derive(Clone, Copy)]
pub struct XPub([u8; XPUB_SIZE]);
impl XPub {
    /// create a `XPub` by taking ownership of the given array
    ///
    pub fn from_bytes(bytes: [u8;XPUB_SIZE]) -> Self { XPub(bytes) }

    /// create a `XPub` from the given slice. This slice must be of size `XPUB_SIZE`
    /// otherwise it will return `Option::None`.
    ///
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != XPUB_SIZE {
            return Err(Error::InvalidXPubSize(bytes.len()));
        }
        let mut buf = [0u8;XPUB_SIZE];
        buf[..].clone_from_slice(bytes);
        Ok(Self::from_bytes(buf))
    }

    /// create a `XPrv` from a given hexadecimal string
    ///
    /// ```
    /// use wallet_crypto::hdwallet::{XPub};
    ///
    /// let xpub = XPub::from_hex("1c0c3ae1825e90b6ddda3f40a122c007e1008e83b2e102c142baefb721d72c1a5d3661deb9064f2d0e03fe85d68070b2fe33b4916059658e28ac7f7f91ca4b12");
    ///
    /// assert!(xpub.is_ok());
    /// ```
    ///
    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = hex::decode(hex)?;
        Self::from_slice(&bytes)
    }

    /// verify a signature
    ///
    /// ```
    /// use wallet_crypto::hdwallet::{XPrv, XPub, Signature};
    ///
    /// let xprv = XPrv::from_hex("301604045de9138b8b23b6730495f7e34b5151d29ba3456bc9b332f6f084a551d646bc30cf126fa8ed776c05a8932a5ab35c8bac41eb01bb9a16cfe229b94b405d3661deb9064f2d0e03fe85d68070b2fe33b4916059658e28ac7f7f91ca4b12").unwrap();
    /// let xpub = xprv.public();
    /// let msg = b"Some message...";
    ///
    /// let signature : Signature<String> = xprv.sign(msg);
    /// assert!(xpub.verify(msg, &signature));
    /// ```
    pub fn verify<T>(&self, message: &[u8], signature: &Signature<T>) -> bool {
        ed25519::verify(message, &self.as_ref()[0..32], signature.as_ref())
    }

    pub fn derive(&self, scheme: DerivationScheme, index: DerivationIndex) -> Result<Self> {
        derive_public(self, index, scheme)
    }
}
impl PartialEq for XPub {
    fn eq(&self, rhs: &XPub) -> bool { fixed_time_eq(self.as_ref(), rhs.as_ref()) }
}
impl Eq for XPub {}
impl fmt::Display for XPub {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl fmt::Debug for XPub {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl AsRef<[u8]> for XPub {
    fn as_ref(&self) -> &[u8] { &self.0 }
}
impl cbor::CborValue for XPub {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Bytes(cbor::Bytes::from_slice(self.as_ref()))
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.bytes().and_then(|bytes| {
            match XPub::from_slice(bytes.as_ref()) {
                Ok(pk) => Ok(pk),
                Err(Error::InvalidXPubSize(_)) => cbor::Result::bytes(bytes, cbor::Error::InvalidSize(XPUB_SIZE)),
                Err(err) => panic!("unexpected error happended: {}", err),
            }
        }).embed("while decoding `XPub`")
    }
}
impl serde::Serialize for XPub
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(self.as_ref()))
        } else {
            serializer.serialize_bytes(self.as_ref())
        }
    }
}
struct XPubVisitor();
impl XPubVisitor { fn new() -> Self { XPubVisitor {} } }
impl<'de> serde::de::Visitor<'de> for XPubVisitor {
    type Value = XPub;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expecting an Extended Public Key (`XPub`) of {} bytes.", XPUB_SIZE)
    }

    fn visit_str<'a, E>(self, v: &'a str) -> result::Result<Self::Value, E>
        where E: serde::de::Error
    {
        match XPub::from_hex(v) {
            Err(Error::HexadecimalError(err)) => Err(E::custom(format!("{}", err))),
            Err(Error::InvalidXPubSize(sz)) => Err(E::invalid_length(sz, &"64 bytes")),
            Err(err) => panic!("unexpected error happended: {}", err),
            Ok(xpub) => Ok(xpub)
        }
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> result::Result<Self::Value, E>
        where E: serde::de::Error
    {
        match XPub::from_slice(v) {
            Ok(pk) => Ok(pk),
            Err(Error::InvalidXPubSize(sz)) => Err(E::invalid_length(sz, &"64 bytes")),
            Err(err) => panic!("unexpected error happended: {}", err),
        }
    }
}
impl<'de> serde::Deserialize<'de> for XPub
{
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(XPubVisitor::new())
        } else {
            deserializer.deserialize_bytes(XPubVisitor::new())
        }
    }
}

/// a signature with an associated type tag
///
#[derive(Clone)]
pub struct Signature<T> {
    bytes: [u8; SIGNATURE_SIZE],
    _phantom: PhantomData<T>,
}
impl<T> Signature<T> {
    pub fn from_bytes(bytes: [u8;SIGNATURE_SIZE]) -> Self {
        Signature { bytes: bytes, _phantom: PhantomData }
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self>  {
        if bytes.len() != SIGNATURE_SIZE {
            return Err(Error::InvalidSignatureSize(bytes.len()))
        }
        let mut buf = [0u8;SIGNATURE_SIZE];
        buf[..].clone_from_slice(bytes);
        Ok(Self::from_bytes(buf))
    }

    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = hex::decode(hex)?;
        Self::from_slice(&bytes)
    }

    pub fn coerce<R>(self) -> Signature<R> {
        Signature::<R>::from_bytes(self.bytes)
    }

    pub fn to_bytes<'a>(&'a self) -> &'a [u8;SIGNATURE_SIZE] { &self.bytes }
}
impl<T> PartialEq for Signature<T> {
    fn eq(&self, rhs: &Signature<T>) -> bool { fixed_time_eq(self.as_ref(), rhs.as_ref()) }
}
impl<T> Eq for Signature<T> {}
impl<T> fmt::Display for Signature<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl<T> fmt::Debug for Signature<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl<T> AsRef<[u8]> for Signature<T> {
    fn as_ref(&self) -> &[u8] { &self.bytes }
}
impl<T> cbor::CborValue for Signature<T> {
    fn encode(&self) -> cbor::Value { cbor::Value::Bytes(cbor::Bytes::from_slice(self.as_ref())) }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.bytes().and_then(|bytes| {
            match Signature::from_slice(bytes.as_ref()) {
                Ok(sign) => Ok(sign),
                Err(Error::InvalidSignatureSize(_)) => cbor::Result::bytes(bytes, cbor::Error::InvalidSize(SIGNATURE_SIZE)),
                Err(err) => panic!("unexpected error happended: {}", err),
            }
        }).embed("while decoding Signature<T>")
    }
}
impl<T> serde::Serialize for Signature<T>
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(self.as_ref()))
        } else {
            serializer.serialize_bytes(self.as_ref())
        }
    }
}
struct SignatureVisitor<T>(PhantomData<T>);
impl<T> SignatureVisitor<T> { fn new() -> Self { SignatureVisitor (PhantomData) } }
impl<'de, T> serde::de::Visitor<'de> for SignatureVisitor<T> {
    type Value = Signature<T>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expected a signature (`Signature`) of {} bytes.", SIGNATURE_SIZE)
    }

    fn visit_str<'a, E>(self, v: &'a str) -> result::Result<Self::Value, E>
        where E: serde::de::Error
    {
        match Signature::from_hex(v) {
            Err(Error::HexadecimalError(err)) => Err(E::custom(format!("{}", err))),
            Err(Error::InvalidSignatureSize(sz)) => Err(E::invalid_length(sz, &"64 bytes")),
            Err(err) => panic!("unexpected error happended: {}", err),
            Ok(xpub) => Ok(xpub)
        }
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> result::Result<Self::Value, E>
        where E: serde::de::Error
    {
        match Signature::from_slice(v) {
            Ok(sign) => Ok(sign),
            Err(Error::InvalidSignatureSize(sz)) => Err(E::invalid_length(sz, &"64 bytes")),
            Err(err) => panic!("unexpected error happended: {}", err),
        }
    }
}
impl<'de, T> serde::Deserialize<'de> for Signature<T>
{
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(SignatureVisitor::new())
        } else {
            deserializer.deserialize_bytes(SignatureVisitor::new())
        }
    }
}

pub type ChainCode = [u8; CHAIN_CODE_SIZE];

type DerivationIndex = u32;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
enum DerivationType {
    Soft(u32),
    Hard(u32),
}

fn to_type(index: DerivationIndex) -> DerivationType {
    if index >= 0x80000000 {
        DerivationType::Hard(index)
    } else {
        DerivationType::Soft(index)
    }
}

fn mk_ed25519_extended(extended_out: &mut [u8], secret: &[u8]) {
    assert!(extended_out.len() == 64);
    assert!(secret.len() == 32);
    let mut hasher = Sha512::new();
    hasher.input(secret);
    hasher.result(extended_out);
    extended_out[0] &= 248;
    extended_out[31] &= 63;
    extended_out[31] |= 64;
}

fn be32(i: u32) -> [u8; 4] {
    [(i >> 24) as u8, (i >> 8) as u8, (i >> 16) as u8, i as u8]
}

fn le32(i: u32) -> [u8; 4] {
    [i as u8, (i >> 8) as u8, (i >> 16) as u8, (i >> 24) as u8]
}

fn serialize_index(i: u32, derivation_scheme: DerivationScheme) -> [u8; 4] {
    match derivation_scheme {
        DerivationScheme::V1 => be32(i),
        DerivationScheme::V2 => le32(i),
    }
}

fn mk_xprv(out: &mut [u8; XPRV_SIZE], kl: &[u8], kr: &[u8], cc: &[u8]) {
    assert!(kl.len() == 32);
    assert!(kr.len() == 32);
    assert!(cc.len() == CHAIN_CODE_SIZE);

    out[0..32].clone_from_slice(kl);
    out[32..64].clone_from_slice(kr);
    out[64..96].clone_from_slice(cc);
}

fn mk_xpub(out: &mut [u8; XPUB_SIZE], pk: &[u8], cc: &[u8]) {
    assert!(pk.len() == 32);
    assert!(cc.len() == CHAIN_CODE_SIZE);

    out[0..32].clone_from_slice(pk);
    out[32..64].clone_from_slice(cc);
}

fn add_256bits_v1(x: &[u8], y: &[u8]) -> [u8; 32] {
    assert!(x.len() == 32);
    assert!(y.len() == 32);

    let mut out = [0u8; 32];
    for i in 0..32 {
        let r = x[i] as u16 + y[i] as u16;
        out[i] = r as u8;
    }
    out
}

fn add_256bits_v2(x: &[u8], y: &[u8]) -> [u8; 32] {
    assert!(x.len() == 32);
    assert!(y.len() == 32);

    let mut carry: u16 = 0;
    let mut out = [0u8; 32];
    for i in 0..32 {
        let r = (x[i] as u16) + (y[i] as u16) + carry;
        out[i] = r as u8;
        carry = r >> 8;
    }
    out
}

fn add_256bits(x: &[u8], y: &[u8], scheme: DerivationScheme) -> [u8; 32] {
    match scheme {
        DerivationScheme::V1 => add_256bits_v1(x, y),
        DerivationScheme::V2 => add_256bits_v2(x, y),
    }
}

fn add_28_mul8_v1(x: &[u8], y: &[u8]) -> [u8; 32] {
    assert!(x.len() == 32);
    assert!(y.len() == 32);

    let yfe8 = {
        let mut acc = 0;
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[i] = (y[i] << 3) + acc & 0x8;
            acc = y[i] >> 5;
        }
        Fe::from_bytes(&out)
    };

    let xfe = Fe::from_bytes(x);
    let r = xfe + yfe8;
    r.to_bytes()
}


fn add_28_mul8_v2(x: &[u8], y: &[u8]) -> [u8; 32] {
    assert!(x.len() == 32);
    assert!(y.len() == 32);

    let mut carry: u16 = 0;
    let mut out = [0u8; 32];

    for i in 0..28 {
        let r = x[i] as u16 + ((y[i] as u16) << 3) + carry;
        out[i] = (r & 0xff) as u8;
        carry = r >> 8;
    }
    for i in 28..32 {
        let r = x[i] as u16 + carry;
        out[i] = (r & 0xff) as u8;
        carry = r >> 8;
    }
    out
}

fn add_28_mul8(x: &[u8], y: &[u8], scheme: DerivationScheme) -> [u8; 32] {
    match scheme {
        DerivationScheme::V1 => add_28_mul8_v1(x, y),
        DerivationScheme::V2 => add_28_mul8_v2(x, y),
    }
}

fn derive_private(xprv: &XPrv, index: DerivationIndex, scheme: DerivationScheme) -> XPrv {
    /*
     * If so (hardened child):
     *    let Z = HMAC-SHA512(Key = cpar, Data = 0x00 || ser256(left(kpar)) || ser32(i)).
     *    let I = HMAC-SHA512(Key = cpar, Data = 0x01 || ser256(left(kpar)) || ser32(i)).
     * If not (normal child):
     *    let Z = HMAC-SHA512(Key = cpar, Data = 0x02 || serP(point(kpar)) || ser32(i)).
     *    let I = HMAC-SHA512(Key = cpar, Data = 0x03 || serP(point(kpar)) || ser32(i)).
     **/

    let ekey = &xprv.as_ref()[0..64];
    let kl = &ekey[0..32];
    let kr = &ekey[32..64];
    let chaincode = &xprv.as_ref()[64..96];

    let mut zmac = Hmac::new(Sha512::new(), &chaincode);
    let mut imac = Hmac::new(Sha512::new(), &chaincode);
    let seri = serialize_index(index, scheme);
    match to_type(index) {
        DerivationType::Soft(_) => {
            let pk = mk_public_key(ekey);
            zmac.input(&[0x2]);
            zmac.input(&pk);
            zmac.input(&seri);
            imac.input(&[0x3]);
            imac.input(&pk);
            imac.input(&seri);
        }
        DerivationType::Hard(_) => {
            zmac.input(&[0x0]);
            zmac.input(ekey);
            zmac.input(&seri);
            imac.input(&[0x1]);
            imac.input(ekey);
            imac.input(&seri);
        }
    };

    let mut zout = [0u8; 64];
    zmac.raw_result(&mut zout);
    let zl = &zout[0..32];
    let zr = &zout[32..64];

    // left = kl + 8 * trunc28(zl)
    let left = add_28_mul8(kl, zl, scheme);
    // right = zr + kr
    let right = add_256bits(kr, zr, scheme);

    let mut iout = [0u8; 64];
    imac.raw_result(&mut iout);
    let cc = &iout[32..];

    let mut out = [0u8; XPRV_SIZE];
    mk_xprv(&mut out, &left, &right, cc);

    imac.reset();
    zmac.reset();

    XPrv::from_bytes(out)
}

fn point_of_trunc28_mul8(sk: &[u8], scheme: DerivationScheme) -> [u8;32] {
    assert!(sk.len() == 32);
    let copy = add_28_mul8(&[0u8;32], sk, scheme);
    let a = ge_scalarmult_base(&copy);
    a.to_bytes()
}

fn point_plus(p1: &[u8], p2: &[u8]) -> Result<[u8;32]> {
    let a = match GeP3::from_bytes_negate_vartime(p1) {
        Some(g) => g,
        None    => { return Err(Error::InvalidDerivation); }
    };
    let b = match GeP3::from_bytes_negate_vartime(p2) {
        Some(g) => g,
        None    => { return Err(Error::InvalidDerivation); }
    };
    let r = a + b.to_cached();
    let mut r = r.to_p2().to_bytes();
    r[31] ^= 0x80;
    Ok(r)
}

fn derive_public(xpub: &XPub, index: DerivationIndex, scheme: DerivationScheme) -> Result<XPub> {
    let pk = &xpub.as_ref()[0..32];
    let chaincode = &xpub.as_ref()[32..64];

    let mut zmac = Hmac::new(Sha512::new(), &chaincode);
    let mut imac = Hmac::new(Sha512::new(), &chaincode);
    let seri = serialize_index(index, scheme);
    match to_type(index) {
        DerivationType::Soft(_) => {
            zmac.input(&[0x2]);
            zmac.input(&pk);
            zmac.input(&seri);
            imac.input(&[0x3]);
            imac.input(&pk);
            imac.input(&seri);
        }
        DerivationType::Hard(_) => {
            return Err(Error::ExpectedSoftDerivation);
        }
    };

    let mut zout = [0u8; 64];
    zmac.raw_result(&mut zout);
    let zl = &zout[0..32];
    let _zr = &zout[32..64];

    // left = kl + 8 * trunc28(zl)
    let left = point_plus(pk, &point_of_trunc28_mul8(zl, scheme))?;

    let mut iout = [0u8; 64];
    imac.raw_result(&mut iout);
    let cc = &iout[32..];

    let mut out = [0u8; XPUB_SIZE];
    mk_xpub(&mut out, &left, cc);

    imac.reset();
    zmac.reset();

    Ok(XPub::from_bytes(out))

}

fn mk_public_key(extended_secret: &[u8]) -> [u8; PUBLIC_KEY_SIZE] {
    assert!(extended_secret.len() == 64);
    ed25519::to_public(extended_secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D1: [u8;XPRV_SIZE] =
        [0xf8, 0xa2, 0x92, 0x31, 0xee, 0x38, 0xd6, 0xc5, 0xbf, 0x71, 0x5d, 0x5b, 0xac, 0x21, 0xc7,
         0x50, 0x57, 0x7a, 0xa3, 0x79, 0x8b, 0x22, 0xd7, 0x9d, 0x65, 0xbf, 0x97, 0xd6, 0xfa, 0xde,
         0xa1, 0x5a, 0xdc, 0xd1, 0xee, 0x1a, 0xbd, 0xf7, 0x8b, 0xd4, 0xbe, 0x64, 0x73, 0x1a, 0x12,
         0xde, 0xb9, 0x4d, 0x36, 0x71, 0x78, 0x41, 0x12, 0xeb, 0x6f, 0x36, 0x4b, 0x87, 0x18, 0x51,
         0xfd, 0x1c, 0x9a, 0x24, 0x73, 0x84, 0xdb, 0x9a, 0xd6, 0x00, 0x3b, 0xbd, 0x08, 0xb3, 0xb1,
         0xdd, 0xc0, 0xd0, 0x7a, 0x59, 0x72, 0x93, 0xff, 0x85, 0xe9, 0x61, 0xbf, 0x25, 0x2b, 0x33,
         0x12, 0x62, 0xed, 0xdf, 0xad, 0x0d];

    const D1_H0: [u8;XPRV_SIZE] =
        [0x60, 0xd3, 0x99, 0xda, 0x83, 0xef, 0x80, 0xd8, 0xd4, 0xf8, 0xd2, 0x23, 0x23, 0x9e, 0xfd,
         0xc2, 0xb8, 0xfe, 0xf3, 0x87, 0xe1, 0xb5, 0x21, 0x91, 0x37, 0xff, 0xb4, 0xe8, 0xfb, 0xde,
         0xa1, 0x5a, 0xdc, 0x93, 0x66, 0xb7, 0xd0, 0x03, 0xaf, 0x37, 0xc1, 0x13, 0x96, 0xde, 0x9a,
         0x83, 0x73, 0x4e, 0x30, 0xe0, 0x5e, 0x85, 0x1e, 0xfa, 0x32, 0x74, 0x5c, 0x9c, 0xd7, 0xb4,
         0x27, 0x12, 0xc8, 0x90, 0x60, 0x87, 0x63, 0x77, 0x0e, 0xdd, 0xf7, 0x72, 0x48, 0xab, 0x65,
         0x29, 0x84, 0xb2, 0x1b, 0x84, 0x97, 0x60, 0xd1, 0xda, 0x74, 0xa6, 0xf5, 0xbd, 0x63, 0x3c,
         0xe4, 0x1a, 0xdc, 0xee, 0xf0, 0x7a];

    const MSG: &'static [u8] = b"Hello World";

    const D1_H0_SIGNATURE: [u8; 64] =
        [0x90, 0x19, 0x4d, 0x57, 0xcd, 0xe4, 0xfd, 0xad, 0xd0, 0x1e, 0xb7, 0xcf, 0x16, 0x17, 0x80,
         0xc2, 0x77, 0xe1, 0x29, 0xfc, 0x71, 0x35, 0xb9, 0x77, 0x79, 0xa3, 0x26, 0x88, 0x37, 0xe4,
         0xcd, 0x2e, 0x94, 0x44, 0xb9, 0xbb, 0x91, 0xc0, 0xe8, 0x4d, 0x23, 0xbb, 0xa8, 0x70, 0xdf,
         0x3c, 0x4b, 0xda, 0x91, 0xa1, 0x10, 0xef, 0x73, 0x56, 0x38, 0xfa, 0x7a, 0x34, 0xea, 0x20,
         0x46, 0xd4, 0xbe, 0x04];

    fn compare_xprv(xprv: &[u8], expected_xprv: &[u8]) {
        assert_eq!(xprv[64..].to_vec(),
                   expected_xprv[64..].to_vec(),
                   "chain code");
        assert_eq!(xprv[..64].to_vec(),
                   expected_xprv[..64].to_vec(),
                   "extended key");
    }

    fn seed_xprv_eq(seed: &Seed, expected_xprv: &[u8;XPRV_SIZE]) {
        let xprv = XPrv::generate_from_seed(&seed);
        compare_xprv(xprv.as_ref(), expected_xprv);
    }

    #[test]
    fn seed_cases() {
        let bytes =  [0xe3, 0x55, 0x24, 0xa5, 0x18, 0x03, 0x4d, 0xdc, 0x11, 0x92, 0xe1, 0xda,
                      0xcd, 0x32, 0xc1, 0xed, 0x3e, 0xaa, 0x3c, 0x3b, 0x13, 0x1c, 0x88, 0xed,
                      0x8e, 0x7e, 0x54, 0xc4, 0x9a, 0x5d, 0x09, 0x98];
        let seed = Seed::from_bytes(bytes);
        seed_xprv_eq(&seed, &D1);
    }

    fn derive_xprv_eq(parent_xprv: &XPrv, idx: DerivationIndex, expected_xprv: [u8; 96]) {
        let child_xprv = derive_private(parent_xprv, idx, DerivationScheme::V2);
        compare_xprv(child_xprv.as_ref(), &expected_xprv);
    }

    #[test]
    fn xprv_derive() {
        let prv = XPrv::from_bytes(D1);
        derive_xprv_eq(&prv, 0x80000000, D1_H0);
    }

    fn do_sign(xprv: &XPrv, expected_signature: &[u8]) {
        let signature : Signature<Vec<u8>> = xprv.sign(MSG);
        assert_eq!(signature.as_ref(), expected_signature);
    }

    #[test]
    fn xpub_derive_v1_hardened()  {
        let derivation_index = 0x1;
        let seed = Seed::from_bytes([0;32]);
        let prv = XPrv::generate_from_seed(&seed);
        let child_prv = prv.derive(DerivationScheme::V1, derivation_index);
    }

    #[test]
    fn xpub_derive_v1_soft()  {
        let derivation_index = 0x10000000;
        let seed = Seed::from_bytes([0;32]);
        let prv = XPrv::generate_from_seed(&seed);
        let xpub = prv.public();
        let child_prv = prv.derive(DerivationScheme::V1, derivation_index);
        let child_xpub = xpub.derive(DerivationScheme::V1, derivation_index).unwrap();
        assert_eq!(child_prv.public(), child_xpub);
    }

    #[test]
    fn xpub_derive_v2()  {
        let derivation_index = 0x10000000;
        let prv = XPrv::from_bytes(D1);
        let xpub = prv.public();
        let child_prv = prv.derive(DerivationScheme::V2, derivation_index);
        let child_xpub = xpub.derive(DerivationScheme::V2, derivation_index).unwrap();
        assert_eq!(child_prv.public(), child_xpub);
    }

    #[test]
    fn xprv_sign() {
        let prv = XPrv::from_bytes(D1_H0);
        do_sign(&prv, &D1_H0_SIGNATURE);
    }

    #[test]
    fn unit_derivation_v1() {
        let seed = Seed::from_bytes([ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let xprv0 = XPrv::generate_from_seed(&seed);

        let xpub0 = xprv0.public();
        let xpub0_ref = XPub::from_bytes([ 28, 12, 58, 225, 130, 94, 144, 182, 221, 218, 63, 64, 161, 34, 192, 7, 225, 0, 142, 131, 178, 225, 2, 193, 66, 186, 239, 183, 33, 215, 44, 26, 93, 54, 97, 222, 185, 6, 79, 45, 14, 3, 254, 133, 214, 128, 112, 178, 254, 51, 180, 145, 96, 89, 101, 142, 40, 172, 127, 127, 145, 202, 75, 18]);
        assert_eq!(xpub0_ref, xpub0);

        let xprv1 = xprv0.derive(DerivationScheme::V1, 0x80000000);
        let xpub1 = xprv1.public();
        let xpub1_ref = XPub::from_bytes([ 155, 186, 125, 76, 223, 83, 124, 115, 51, 236, 62, 66, 30, 151, 236, 155, 157, 73, 110, 160, 25, 204, 222, 170, 46, 185, 166, 187, 220, 65, 18, 182, 194, 224, 222, 91, 65, 119, 17, 215, 53, 147, 168, 219, 125, 51, 13, 233, 35, 212, 226, 241, 0, 36, 245, 198, 28, 19, 91, 74, 49, 43, 106, 167]);

        assert_eq!(xpub1_ref, xpub1);
    }
}

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench {
    use super::*;
    use test;

    #[bench]
    fn derivate_hard_v1(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0;SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        b.iter(|| {
            let _ = sk.derive(DerivationScheme::V1, 0x80000000);
        })
    }
    #[bench]
    fn derivate_hard_v2(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0;SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        b.iter(|| {
            let _ = sk.derive(DerivationScheme::V2, 0x80000000);
        })
    }

    #[bench]
    fn derivate_soft_v1_xprv(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0;SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        b.iter(|| {
            let _ = sk.derive(DerivationScheme::V1, 0);
        })
    }
    #[bench]
    fn derivate_soft_v2_xprv(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0;SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        b.iter(|| {
            let _ = sk.derive(DerivationScheme::V2, 0);
        })
    }
    #[bench]
    fn derivate_soft_v1_xpub(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0;SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        b.iter(|| {
            let _ = pk.derive(DerivationScheme::V1, 0);
        })
    }
    #[bench]
    fn derivate_soft_v2_xpub(b: &mut test::Bencher) {
        let seed = Seed::from_bytes([0;SEED_SIZE]);
        let sk = XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        b.iter(|| {
            let _ = pk.derive(DerivationScheme::V2, 0);
        })
    }
}

#[cfg(test)]
mod golden_tests {
    use super::*;

struct TestVector {
    /// BIP39 Seed
    seed: &'static [u8;32],
    /// Wallet's extended signature
    signature: &'static [u8;64],
    /// Wallet's extended private key
    // commented out because it is encrypted in the Haskell library... xPriv: &'static [u8;96],
    /// Wallet's extended public key
    xPub: &'static [u8;64],
    /// UTF8 string
    data_to_sign: &'static str,
    /// Derivation Chain code path: list of derivation path.
    path: &'static [u32],
    /// Wallet's derivation schemes: String either "derivation-scheme1" or "derivation-scheme2"
    derivation_scheme: &'static str,
    /// UTF8 string
    passphrase: &'static str,
    /// BIP39 mnemonic sentence (in English) of 12 BIP39 Enlighs words
    words: &'static str,
}
    fn check_test(test_index: usize, test: &TestVector) {
        let seed = Seed::from_slice(test.seed).expect("failed to read the seed slice from test");
        let mut xprv = XPrv::generate_from_seed(&seed);

        let scheme = match test.derivation_scheme {
            "derivation-scheme1" => DerivationScheme::V1,
            "derivation-scheme2" => DerivationScheme::V2,
            _                    => panic!("Unnown derivation scheme: {}, from test{}", test.derivation_scheme, test_index),
        };

        for derivation_index in test.path {
            xprv = xprv.derive(scheme, *derivation_index);
        }

        let xpub = xprv.public();
        let ref_xpub = XPub::from_slice(test.xPub).expect("failed to read the xpub from the test");
        assert_eq!(ref_xpub, xpub, "xpub from test {}", test_index);

        let ref_signature : Signature<Vec<u8>> = Signature::from_slice(test.signature)
            .expect("retrieve signature from the golden test");
        let signature = xprv.sign(test.data_to_sign.as_bytes());
        assert_eq!(ref_signature, signature, "xpub from test {}", test_index);
    }

    #[test]
    fn test() {
        let mut test_index = 0;
        for test in TEST_VECTORS.iter() {
            check_test(test_index, test);
            test_index += 1;
        }
    }

const TEST_VECTORS : [TestVector;11] =
    [ TestVector {
        data_to_sign: "Hello World",
        path: & [],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 239, 59, 133, 220, 61, 92, 115, 250, 200, 37, 84, 131, 153, 236, 106, 13, 230, 138, 99, 25, 31, 225, 86, 164, 72, 219, 158, 212, 203, 1, 105, 202, 209, 29, 125, 142, 21, 120, 214, 67, 163, 163, 59, 96, 244, 223, 180, 51, 6, 23, 238, 19, 201, 250, 31, 139, 48, 152, 160, 140, 129, 88, 154, 10],
        // xPriv: & [ 192, 61, 168, 132, 192, 165, 111, 73, 239, 203, 72, 170, 192, 155, 213, 67, 66, 53, 245, 198, 120, 13, 74, 80, 82, 51, 230, 34, 47, 108, 183, 77, 40, 241, 107, 224, 95, 67, 29, 133, 233, 204, 248, 160, 190, 32, 118, 225, 220, 39, 131, 159, 168, 247, 182, 105, 177, 97, 37, 160, 241, 0, 241, 107, 109, 9, 151, 97, 58, 206, 111, 150, 235, 158, 26, 239, 199, 223, 135, 172, 119, 112, 214, 70, 218, 39, 35, 157, 3, 15, 88, 233, 31, 203, 135, 72, 1, 64, 159, 184, 142, 255, 146, 33, 22, 19, 173, 105, 126, 82, 126, 198, 129, 193, 193, 123, 103, 203, 244, 185, 239, 227, 190, 105, 71, 2, 139, 65],
        xPub: & [ 109, 9, 151, 97, 58, 206, 111, 150, 235, 158, 26, 239, 199, 223, 135, 172, 119, 112, 214, 70, 218, 39, 35, 157, 3, 15, 88, 233, 31, 203, 135, 72, 1, 64, 159, 184, 142, 255, 146, 33, 22, 19, 173, 105, 126, 82, 126, 198, 129, 193, 193, 123, 103, 203, 244, 185, 239, 227, 190, 105, 71, 2, 139, 65],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648 ],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 218, 33, 50, 114, 134, 254, 79, 160, 242, 191, 56, 75, 184, 68, 177, 90, 237, 209, 200, 7, 3, 195, 30, 222, 175, 37, 180, 184, 73, 119, 238, 122, 157, 131, 231, 12, 208, 181, 58, 136, 223, 45, 200, 222, 28, 69, 61, 4, 95, 112, 132, 11, 201, 97, 163, 75, 114, 160, 202, 129, 234, 251, 28, 1],
        // xPriv: & [ 54, 103, 63, 20, 54, 126, 93, 0, 62, 53, 216, 21, 162, 234, 176, 12, 153, 53, 245, 38, 73, 190, 146, 144, 34, 44, 239, 74, 151, 68, 56, 6, 231, 253, 134, 212, 25, 153, 5, 177, 143, 50, 57, 38, 147, 248, 209, 254, 77, 193, 240, 220, 137, 68, 2, 118, 99, 98, 137, 99, 151, 128, 106, 7, 18, 200, 248, 31, 81, 188, 126, 37, 29, 111, 100, 192, 46, 59, 76, 129, 56, 88, 62, 19, 219, 147, 154, 40, 246, 129, 25, 166, 54, 217, 227, 227, 138, 95, 17, 6, 228, 64, 76, 16, 186, 203, 93, 220, 224, 37, 130, 196, 60, 156, 22, 232, 121, 111, 187, 86, 142, 68, 87, 127, 189, 223, 112, 75],
        xPub: & [ 18, 200, 248, 31, 81, 188, 126, 37, 29, 111, 100, 192, 46, 59, 76, 129, 56, 88, 62, 19, 219, 147, 154, 40, 246, 129, 25, 166, 54, 217, 227, 227, 138, 95, 17, 6, 228, 64, 76, 16, 186, 203, 93, 220, 224, 37, 130, 196, 60, 156, 22, 232, 121, 111, 187, 86, 142, 68, 87, 127, 189, 223, 112, 75],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483649 ],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 24, 225, 23, 235, 58, 216, 17, 156, 191, 59, 142, 77, 25, 60, 95, 163, 53, 118, 151, 254, 21, 184, 125, 184, 125, 249, 72, 192, 96, 147, 84, 239, 98, 249, 248, 22, 201, 111, 82, 240, 142, 5, 74, 67, 64, 75, 146, 151, 165, 145, 126, 143, 23, 123, 212, 131, 81, 140, 236, 90, 235, 219, 172, 5],
        // xPriv: & [ 5, 115, 111, 130, 8, 88, 207, 193, 43, 202, 43, 94, 34, 103, 93, 154, 178, 189, 253, 22, 57, 78, 26, 233, 194, 59, 198, 163, 63, 236, 15, 6, 17, 96, 239, 104, 143, 45, 7, 73, 194, 74, 23, 40, 171, 18, 198, 63, 231, 109, 110, 237, 44, 82, 51, 57, 179, 132, 177, 126, 11, 225, 122, 162, 188, 133, 171, 241, 117, 143, 184, 222, 140, 118, 174, 50, 157, 59, 134, 208, 195, 57, 56, 201, 177, 209, 44, 84, 86, 178, 184, 202, 45, 54, 240, 80, 107, 46, 202, 183, 122, 179, 152, 57, 1, 201, 65, 134, 200, 253, 122, 129, 142, 20, 29, 195, 0, 92, 4, 193, 216, 196, 118, 232, 25, 105, 23, 62],
        xPub: & [ 188, 133, 171, 241, 117, 143, 184, 222, 140, 118, 174, 50, 157, 59, 134, 208, 195, 57, 56, 201, 177, 209, 44, 84, 86, 178, 184, 202, 45, 54, 240, 80, 107, 46, 202, 183, 122, 179, 152, 57, 1, 201, 65, 134, 200, 253, 122, 129, 142, 20, 29, 195, 0, 92, 4, 193, 216, 196, 118, 232, 25, 105, 23, 62],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 86, 185, 175, 209, 93, 1, 13, 24, 200, 87, 2, 176, 8, 170, 17, 199, 5, 214, 238, 86, 175, 201, 7, 169, 174, 89, 182, 52, 200, 217, 105, 224, 23, 102, 129, 4, 143, 205, 230, 2, 197, 105, 36, 198, 51, 20, 158, 130, 60, 154, 231, 96, 43, 155, 209, 156, 90, 78, 145, 77, 192, 19, 161, 4],
        // xPriv: & [ 215, 51, 86, 14, 247, 220, 94, 8, 59, 247, 81, 53, 182, 189, 204, 94, 25, 214, 229, 127, 33, 167, 27, 177, 26, 197, 183, 107, 119, 109, 216, 14, 215, 73, 112, 186, 54, 78, 205, 230, 1, 65, 0, 92, 209, 217, 231, 93, 38, 189, 193, 95, 17, 107, 1, 237, 210, 44, 80, 180, 90, 128, 28, 31, 118, 2, 186, 16, 101, 247, 118, 161, 28, 169, 231, 125, 107, 218, 64, 196, 17, 234, 71, 21, 26, 89, 98, 61, 225, 166, 245, 155, 47, 16, 122, 137, 188, 96, 97, 105, 68, 97, 233, 75, 71, 187, 157, 93, 39, 211, 102, 93, 158, 31, 121, 8, 56, 127, 216, 241, 1, 235, 183, 71, 96, 78, 68, 22],
        xPub: & [ 118, 2, 186, 16, 101, 247, 118, 161, 28, 169, 231, 125, 107, 218, 64, 196, 17, 234, 71, 21, 26, 89, 98, 61, 225, 166, 245, 155, 47, 16, 122, 137, 188, 96, 97, 105, 68, 97, 233, 75, 71, 187, 157, 93, 39, 211, 102, 93, 158, 31, 121, 8, 56, 127, 216, 241, 1, 235, 183, 71, 96, 78, 68, 22],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 2147483650],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 189, 71, 31, 170, 71, 63, 229, 120, 15, 130, 99, 131, 232, 68, 222, 75, 41, 50, 128, 81, 195, 12, 247, 93, 115, 179, 190, 235, 8, 124, 81, 85, 13, 39, 178, 220, 174, 153, 237, 34, 90, 200, 45, 140, 192, 210, 245, 108, 4, 23, 125, 215, 188, 4, 205, 228, 197, 247, 92, 90, 59, 252, 220, 2],
        // xPriv: & [ 28, 225, 190, 140, 60, 183, 67, 184, 92, 31, 56, 161, 211, 9, 229, 93, 48, 142, 118, 160, 97, 143, 4, 194, 74, 205, 55, 52, 8, 30, 129, 7, 46, 250, 67, 24, 129, 227, 94, 53, 42, 238, 83, 240, 96, 194, 39, 126, 238, 151, 88, 112, 205, 49, 60, 35, 15, 208, 70, 69, 240, 236, 126, 251, 201, 211, 24, 112, 186, 65, 23, 186, 173, 43, 122, 115, 222, 56, 155, 6, 207, 231, 73, 82, 13, 245, 230, 220, 90, 74, 88, 117, 110, 137, 155, 236, 34, 17, 55, 173, 90, 241, 198, 48, 200, 1, 37, 217, 248, 47, 37, 229, 118, 74, 234, 111, 6, 50, 39, 104, 158, 207, 31, 37, 69, 184, 74, 118],
        xPub: & [ 201, 211, 24, 112, 186, 65, 23, 186, 173, 43, 122, 115, 222, 56, 155, 6, 207, 231, 73, 82, 13, 245, 230, 220, 90, 74, 88, 117, 110, 137, 155, 236, 34, 17, 55, 173, 90, 241, 198, 48, 200, 1, 37, 217, 248, 47, 37, 229, 118, 74, 234, 111, 6, 50, 39, 104, 158, 207, 31, 37, 69, 184, 74, 118],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 2147483650, 2147483650],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 13, 124, 245, 57, 189, 254, 141, 22, 161, 139, 90, 88, 252, 250, 119, 252, 3, 44, 4, 32, 174, 159, 59, 5, 222, 143, 23, 46, 13, 206, 184, 73, 82, 217, 180, 208, 132, 147, 197, 205, 147, 246, 225, 138, 65, 228, 143, 156, 137, 104, 153, 69, 132, 128, 32, 11, 222, 190, 60, 109, 75, 28, 240, 4],
        // xPriv: & [ 188, 242, 137, 37, 143, 70, 246, 175, 135, 146, 222, 97, 186, 124, 181, 112, 47, 166, 118, 216, 225, 239, 148, 50, 243, 181, 40, 189, 104, 126, 57, 0, 233, 225, 37, 230, 54, 175, 133, 14, 128, 90, 31, 181, 250, 161, 93, 220, 95, 167, 179, 65, 23, 227, 219, 137, 245, 70, 3, 215, 108, 81, 223, 138, 87, 147, 161, 17, 168, 119, 170, 197, 204, 96, 65, 59, 122, 55, 242, 162, 154, 82, 116, 156, 101, 18, 35, 91, 118, 24, 34, 68, 46, 226, 174, 227, 129, 129, 100, 28, 28, 144, 25, 217, 161, 187, 67, 111, 144, 115, 71, 175, 181, 117, 196, 216, 147, 135, 96, 98, 254, 51, 89, 247, 48, 59, 150, 27],
        xPub: & [ 87, 147, 161, 17, 168, 119, 170, 197, 204, 96, 65, 59, 122, 55, 242, 162, 154, 82, 116, 156, 101, 18, 35, 91, 118, 24, 34, 68, 46, 226, 174, 227, 129, 129, 100, 28, 28, 144, 25, 217, 161, 187, 67, 111, 144, 115, 71, 175, 181, 117, 196, 216, 147, 135, 96, 98, 254, 51, 89, 247, 48, 59, 150, 27],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 2147483650, 2147483650, 3147483648],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 5, 170, 94, 199, 231, 248, 187, 211, 25, 36, 43, 129, 229, 164, 205, 147, 242, 71, 20, 93, 226, 74, 84, 28, 14, 24, 30, 237, 227, 175, 66, 126, 223, 31, 53, 31, 8, 230, 62, 195, 43, 40, 35, 64, 84, 36, 20, 100, 84, 38, 53, 70, 213, 91, 191, 71, 208, 32, 140, 18, 132, 44, 6, 3],
        // xPriv: & [ 5, 127, 64, 71, 64, 93, 55, 80, 245, 212, 135, 105, 23, 8, 137, 170, 175, 246, 198, 176, 194, 24, 29, 51, 75, 190, 104, 125, 137, 30, 178, 0, 216, 113, 32, 90, 213, 143, 157, 132, 219, 240, 240, 86, 116, 205, 130, 23, 166, 67, 12, 203, 191, 57, 17, 56, 181, 144, 126, 243, 114, 123, 44, 90, 174, 79, 211, 112, 192, 15, 36, 216, 244, 153, 220, 105, 101, 113, 147, 82, 131, 14, 77, 141, 36, 206, 109, 159, 231, 255, 24, 134, 125, 39, 232, 161, 138, 227, 228, 53, 92, 34, 159, 217, 244, 0, 185, 70, 134, 96, 76, 156, 130, 165, 38, 72, 43, 171, 63, 245, 253, 137, 237, 244, 212, 238, 242, 125],
        xPub: & [ 174, 79, 211, 112, 192, 15, 36, 216, 244, 153, 220, 105, 101, 113, 147, 82, 131, 14, 77, 141, 36, 206, 109, 159, 231, 255, 24, 134, 125, 39, 232, 161, 138, 227, 228, 53, 92, 34, 159, 217, 244, 0, 185, 70, 134, 96, 76, 156, 130, 165, 38, 72, 43, 171, 63, 245, 253, 137, 237, 244, 212, 238, 242, 125],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 2147483650, 2147483650, 3147483648],
        derivation_scheme: "derivation-scheme2",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 14, 207, 86, 134, 156, 43, 34, 18, 232, 13, 221, 135, 46, 8, 144, 83, 172, 106, 206, 19, 200, 89, 123, 121, 52, 23, 126, 241, 126, 169, 128, 167, 189, 92, 197, 15, 236, 205, 54, 232, 38, 25, 25, 208, 140, 133, 50, 129, 252, 254, 236, 19, 89, 158, 175, 102, 122, 98, 159, 95, 11, 175, 123, 11],
        // xPriv: & [ 240, 139, 57, 134, 218, 154, 177, 130, 22, 250, 167, 143, 99, 92, 71, 34, 153, 16, 240, 39, 44, 134, 63, 185, 145, 45, 143, 129, 65, 108, 183, 77, 194, 58, 70, 221, 206, 45, 77, 153, 229, 187, 171, 167, 244, 252, 62, 119, 183, 223, 206, 26, 132, 152, 72, 189, 240, 198, 9, 28, 38, 197, 128, 138, 41, 246, 201, 23, 186, 239, 210, 80, 1, 118, 69, 114, 215, 195, 169, 64, 1, 163, 108, 5, 54, 135, 209, 106, 118, 69, 73, 247, 112, 24, 212, 230, 209, 239, 211, 192, 190, 78, 31, 162, 20, 44, 123, 179, 89, 93, 131, 151, 27, 89, 229, 117, 253, 133, 117, 160, 148, 224, 48, 41, 205, 127, 57, 17],
        xPub: & [ 41, 246, 201, 23, 186, 239, 210, 80, 1, 118, 69, 114, 215, 195, 169, 64, 1, 163, 108, 5, 54, 135, 209, 106, 118, 69, 73, 247, 112, 24, 212, 230, 209, 239, 211, 192, 190, 78, 31, 162, 20, 44, 123, 179, 89, 93, 131, 151, 27, 89, 229, 117, 253, 133, 117, 160, 148, 224, 48, 41, 205, 127, 57, 17],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649],
        derivation_scheme: "derivation-scheme2",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 147, 105, 196, 89, 49, 123, 124, 166, 234, 76, 13, 62, 99, 31, 129, 84, 140, 229, 214, 174, 249, 237, 72, 141, 5, 117, 239, 114, 98, 160, 236, 47, 110, 36, 254, 93, 122, 114, 3, 141, 187, 165, 235, 59, 3, 187, 155, 110, 215, 73, 187, 27, 45, 94, 35, 28, 92, 106, 242, 174, 143, 176, 200, 6],
        // xPriv: & [ 16, 40, 91, 111, 27, 203, 202, 137, 51, 219, 47, 233, 39, 91, 8, 46, 8, 195, 70, 167, 210, 23, 254, 241, 254, 150, 190, 112, 55, 108, 183, 77, 47, 207, 180, 173, 48, 159, 7, 59, 124, 50, 233, 241, 181, 157, 11, 92, 72, 93, 228, 46, 230, 201, 0, 101, 134, 163, 86, 101, 214, 236, 161, 25, 182, 235, 126, 87, 181, 95, 6, 129, 44, 16, 27, 41, 225, 179, 165, 144, 107, 93, 179, 178, 246, 251, 252, 18, 229, 98, 248, 72, 181, 74, 204, 54, 231, 248, 221, 204, 29, 249, 209, 249, 35, 236, 12, 93, 227, 250, 75, 119, 57, 144, 168, 58, 242, 214, 207, 38, 37, 96, 22, 181, 107, 43, 22, 102],
        xPub: & [ 182, 235, 126, 87, 181, 95, 6, 129, 44, 16, 27, 41, 225, 179, 165, 144, 107, 93, 179, 178, 246, 251, 252, 18, 229, 98, 248, 72, 181, 74, 204, 54, 231, 248, 221, 204, 29, 249, 209, 249, 35, 236, 12, 93, 227, 250, 75, 119, 57, 144, 168, 58, 242, 214, 207, 38, 37, 96, 22, 181, 107, 43, 22, 102],
      }
    , TestVector {
        data_to_sign: "Data",
        path: & [ 2147483648, 2147483649, 24, 2000],
        derivation_scheme: "derivation-scheme2",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 122, 16, 158, 238, 147, 69, 74, 179, 153, 217, 74, 146, 81, 202, 78, 118, 206, 86, 82, 181, 218, 250, 24, 97, 214, 236, 202, 60, 146, 147, 61, 214, 117, 154, 132, 215, 21, 255, 4, 129, 110, 54, 105, 136, 11, 2, 129, 94, 156, 182, 189, 167, 244, 84, 27, 176, 197, 181, 215, 230, 112, 78, 90, 5],
        // xPriv: & [ 24, 172, 91, 199, 63, 172, 5, 253, 67, 82, 246, 182, 173, 167, 76, 143, 194, 129, 3, 45, 111, 13, 249, 49, 80, 118, 83, 88, 61, 108, 183, 77, 215, 19, 33, 84, 195, 92, 219, 137, 198, 187, 2, 183, 236, 133, 144, 167, 208, 151, 14, 29, 144, 96, 121, 192, 190, 230, 3, 251, 128, 11, 156, 87, 153, 140, 190, 197, 187, 181, 147, 13, 99, 169, 219, 5, 193, 226, 24, 132, 89, 244, 77, 243, 125, 78, 33, 46, 63, 218, 91, 24, 113, 3, 206, 121, 75, 218, 114, 144, 198, 123, 31, 233, 172, 206, 42, 135, 124, 172, 54, 122, 46, 100, 92, 102, 150, 110, 105, 103, 212, 125, 50, 171, 41, 3, 152, 100],
        xPub: & [ 153, 140, 190, 197, 187, 181, 147, 13, 99, 169, 219, 5, 193, 226, 24, 132, 89, 244, 77, 243, 125, 78, 33, 46, 63, 218, 91, 24, 113, 3, 206, 121, 75, 218, 114, 144, 198, 123, 31, 233, 172, 206, 42, 135, 124, 172, 54, 122, 46, 100, 92, 102, 150, 110, 105, 103, 212, 125, 50, 171, 41, 3, 152, 100],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 24, 2147485648],
        derivation_scheme: "derivation-scheme2",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 201, 228, 234, 47, 140, 161, 163, 37, 1, 226, 73, 37, 112, 166, 29, 195, 15, 53, 65, 44, 184, 75, 95, 37, 2, 80, 12, 23, 127, 34, 219, 90],
        signature: & [ 255, 67, 163, 199, 128, 73, 217, 191, 113, 83, 23, 236, 247, 136, 85, 222, 133, 206, 134, 162, 60, 137, 181, 153, 74, 86, 119, 106, 203, 167, 120, 85, 22, 188, 220, 159, 226, 24, 246, 32, 206, 198, 85, 248, 60, 127, 205, 56, 155, 189, 74, 157, 191, 134, 148, 152, 200, 85, 23, 139, 82, 84, 111, 13],
        // xPriv: & [ 192, 85, 40, 91, 217, 7, 19, 8, 236, 86, 74, 255, 12, 238, 20, 246, 187, 121, 158, 160, 34, 217, 147, 126, 86, 109, 175, 142, 66, 108, 183, 77, 193, 93, 182, 127, 129, 82, 203, 136, 123, 128, 176, 204, 138, 107, 74, 97, 122, 45, 200, 141, 186, 199, 81, 91, 133, 94, 103, 5, 105, 235, 57, 179, 176, 101, 227, 152, 44, 251, 250, 35, 5, 21, 75, 162, 79, 3, 244, 128, 207, 64, 7, 77, 25, 236, 94, 139, 159, 239, 46, 73, 111, 39, 84, 145, 139, 201, 195, 157, 165, 62, 62, 242, 157, 48, 247, 244, 79, 41, 209, 243, 116, 151, 23, 113, 68, 69, 73, 48, 157, 29, 72, 72, 178, 180, 25, 150],
        xPub: & [ 176, 101, 227, 152, 44, 251, 250, 35, 5, 21, 75, 162, 79, 3, 244, 128, 207, 64, 7, 77, 25, 236, 94, 139, 159, 239, 46, 73, 111, 39, 84, 145, 139, 201, 195, 157, 165, 62, 62, 242, 157, 48, 247, 244, 79, 41, 209, 243, 116, 151, 23, 113, 68, 69, 73, 48, 157, 29, 72, 72, 178, 180, 25, 150],
      }
    ];
}
