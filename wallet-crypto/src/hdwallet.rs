extern crate rcw;

use self::rcw::digest::Digest;
use self::rcw::sha2::Sha512;
use self::rcw::hmac::Hmac;
use self::rcw::mac::Mac;
use self::rcw::curve25519::{GeP3, ge_scalarmult_base};
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

    pub fn derive(&self, index: DerivationIndex) -> Self {
        derive_private(self, index, DerivationScheme::V2)
    }
}
impl PartialEq for XPrv {
    fn eq(&self, rhs: &XPrv) -> bool { fixed_time_eq(self.as_ref(), rhs.as_ref()) }
}
impl Eq for XPrv {}
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

    pub fn derive(&self, index: DerivationIndex) -> Result<Self> {
        derive_public(self, index, DerivationScheme::V2)
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

fn le32(i: u32) -> [u8; 4] {
    [i as u8, (i >> 8) as u8, (i >> 16) as u8, (i >> 24) as u8]
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

fn add_256bits(x: &[u8], y: &[u8]) -> [u8; 32] {
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

fn add_28_mul8(x: &[u8], y: &[u8]) -> [u8; 32] {
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

#[derive(Clone, Copy)]
pub enum DerivationScheme {
    V1,
    V2,
}

fn derive_private(xprv: &XPrv, index: DerivationIndex, _scheme: DerivationScheme) -> XPrv {
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
    let seri = le32(index);
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
    let left = add_28_mul8(kl, zl);
    // right = zr + kr
    let right = add_256bits(kr, zr);

    let mut iout = [0u8; 64];
    imac.raw_result(&mut iout);
    let cc = &iout[32..];

    let mut out = [0u8; XPRV_SIZE];
    mk_xprv(&mut out, &left, &right, cc);

    imac.reset();
    zmac.reset();

    XPrv::from_bytes(out)
}

fn point_of_trunc28_mul8(sk: &[u8]) -> [u8;32] {
    assert!(sk.len() == 32);
    let copy = add_28_mul8(&[0u8;32], sk);
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

fn derive_public(xpub: &XPub, index: DerivationIndex, _scheme: DerivationScheme) -> Result<XPub> {
    let pk = &xpub.as_ref()[0..32];
    let chaincode = &xpub.as_ref()[32..64];

    let mut zmac = Hmac::new(Sha512::new(), &chaincode);
    let mut imac = Hmac::new(Sha512::new(), &chaincode);
    let seri = le32(index);
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
    let left = point_plus(pk, &point_of_trunc28_mul8(zl))?;

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
    fn xpub_derive()  {
        let derivation_index = 0x10000000;
        let prv = XPrv::from_bytes(D1);
        let xpub = prv.public();
        let child_prv = prv.derive(derivation_index);
        let child_xpub = xpub.derive(derivation_index).unwrap();
        assert_eq!(child_prv.public(), child_xpub);
    }

    #[test]
    fn xprv_sign() {
        let prv = XPrv::from_bytes(D1_H0);
        do_sign(&prv, &D1_H0_SIGNATURE);
    }
}
