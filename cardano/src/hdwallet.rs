//! Hierarchical Deterministic (HD) Wallet
//!
//! Follow the Ed25519-BIP32 paper
//!
//! Supports:
//! * Transform Seed to Extended Private key
//! * Hard and Soft derivation using 32 bits indices
//! * Derivation Scheme V2
//! * Derivation Scheme V1 (don't use for new code, only for compat)
//!
use cryptoxide::digest::Digest;
use cryptoxide::sha2::Sha512;
use cryptoxide::hmac::Hmac;
use cryptoxide::mac::Mac;
use cryptoxide::curve25519::{GeP3, ge_scalarmult_base, sc_reduce};
use cryptoxide::ed25519::signature_extended;
use cryptoxide::ed25519;
use cryptoxide::util::fixed_time_eq;

use bip::bip39;

use std::{fmt, result};
use std::marker::PhantomData;
use std::hash::{Hash, Hasher};
use util::{hex, securemem};

use cbor_event::{self, de::RawCbor, se::{Serializer}};

pub const SEED_SIZE: usize = 32;
pub const XPRV_SIZE: usize = 96;
pub const XPUB_SIZE: usize = 64;
pub const SIGNATURE_SIZE: usize = 64;

pub const PUBLIC_KEY_SIZE: usize = 32;
pub const CHAIN_CODE_SIZE: usize = 32;

/// HDWallet errors
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
    /// the given seed is of invalid size, the parameter is given the given size
    ///
    /// See `SEED_SIZE` for details about the expected size.
    InvalidSeedSize(usize),
    /// the given extended private key is of invalid size. The parameter is the given size.
    ///
    /// See `XPRV_SIZE` for the expected size.
    InvalidXPrvSize(usize),
    /// the given extended public key is of invalid size. The parameter is the given size.
    ///
    /// See `XPUB_SIZE`
    InvalidXPubSize(usize),
    /// the given siganture is of invalid size. The parameter is the given size.
    ///
    /// See `SIGNATURE_SIZE` for the expected size.
    InvalidSignatureSize(usize),
    /// The given extended private key is of invalid format for our usage of ED25519.
    ///
    /// This is not a problem of the size, see `Error::InvalidXPrvSize`
    InvalidXPrv(&'static str),
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
            &Error::InvalidXPrv(ref err) => {
               write!(f, "Invalid XPrv: {}", err)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivationScheme {
    V1,
    V2,
}
impl Default for DerivationScheme {
    fn default() -> Self { DerivationScheme::V2 }
}

/// Seed used to generate the root private key of the HDWallet.
///
#[derive(Debug)]
pub struct Seed([u8; SEED_SIZE]);
impl Seed {
    /// create a Seed by taking ownership of the given array
    ///
    /// ```
    /// use cardano::hdwallet::{Seed, SEED_SIZE};
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
    /// use cardano::hdwallet::{Seed, SEED_SIZE};
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
impl Drop for Seed {
    fn drop(&mut self) {
        securemem::zero(&mut self.0);
    }
}

/// HDWallet extended private key
///
/// Effectively this is ed25519 extended secret key (64 bytes) followed by a chain code (32 bytes)
pub struct XPrv([u8; XPRV_SIZE]);
impl XPrv {
    /// create the Root private key `XPrv` of the HDWallet associated to this `Seed`
    ///
    /// This is a deterministic construction. The `XPrv` returned will always be the
    /// same for the same given `Seed`.
    ///
    /// ```
    /// use cardano::hdwallet::{Seed, SEED_SIZE, XPrv, XPRV_SIZE};
    ///
    /// let seed = Seed::from_bytes([0u8; SEED_SIZE]);
    /// let xprv = XPrv::generate_from_seed(&seed);
    /// ```
    ///
    pub fn generate_from_seed(seed: &Seed) -> Self {
        Self::generate_from_daedalus_seed(seed.as_ref())
    }

    /// for some unknown design reasons Daedalus seeds are encoded in cbor
    /// We then expect the input here to be cbor encoded before hande.
    ///
    pub fn generate_from_daedalus_seed(bytes: &[u8]) -> Self {
        let mut mac = Hmac::new(Sha512::new(), bytes);

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

    /// takes the given raw bytes and perform some modifications to normalize
    /// it properly to a XPrv.
    ///
    pub fn normalize_bytes(mut bytes: [u8;XPRV_SIZE]) -> Self {
        bytes[0]  &= 0b1111_1000;
        bytes[31] &= 0b0001_1111;
        bytes[31] |= 0b0100_0000;;

        Self::from_bytes(bytes)
    }

    // Create a XPrv from the given bytes.
    //
    // This function does not perform any validity check and should not be used outside
    // of this module.
    fn from_bytes(bytes: [u8;XPRV_SIZE]) -> Self { XPrv(bytes) }

    /// Create a `XPrv` by taking ownership of the given array
    ///
    /// This function may returns an error if it does not have the expected
    /// format.
    pub fn from_bytes_verified(bytes: [u8;XPRV_SIZE]) -> Result<Self> {
        let scalar = &bytes[0..32];
        let last   = scalar[31];
        let first  = scalar[0];

        if (last & 0b1110_0000) != 0b0100_0000 {
            return Err(Error::InvalidXPrv("expected 3 highest bits to be 0b010"))
        }
        if (first & 0b0000_0111) != 0b0000_0000 {
            return Err(Error::InvalidXPrv("expected 3 lowest bits to be 0b000"))
        }

        Ok(XPrv(bytes))
    }

    /// Create a `XPrv` from the given slice. This slice must be of size `XPRV_SIZE`
    /// otherwise it will return `Err`.
    ///
    fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != XPRV_SIZE {
            return Err(Error::InvalidXPrvSize(bytes.len()));
        }
        let mut buf = [0u8;XPRV_SIZE];
        buf[..].clone_from_slice(bytes);
        Ok(XPrv::from_bytes(buf))
    }

    /// Create a `XPrv` from a given hexadecimal string
    ///
    fn from_hex(hex: &str) -> Result<Self> {
        let input = hex::decode(hex)?;
        Self::from_slice(&input)
    }

    /// Get the associated `XPub`
    ///
    /// ```
    /// use cardano::hdwallet::{XPrv, XPub, Seed};
    ///
    /// let seed = Seed::from_bytes([0;32]) ;
    /// let xprv = XPrv::generate_from_seed(&seed);
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
    /// use cardano::hdwallet::{XPrv, XPub, Signature, Seed};
    ///
    /// let seed = Seed::from_bytes([0;32]) ;
    /// let xprv = XPrv::generate_from_seed(&seed);
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
impl Drop for XPrv {
    fn drop(&mut self) {
        securemem::zero(&mut self.0);
    }
}

/// Extended Public Key (Point + ChainCode)
#[derive(Clone, Copy)]
pub struct XPub([u8; XPUB_SIZE]);
impl XPub {
    /// create a `XPub` by taking ownership of the given array
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

    /// create a `XPub` from a given hexadecimal string
    ///
    /// ```
    /// use cardano::hdwallet::{XPub};
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
    /// use cardano::hdwallet::{XPrv, XPub, Seed, Signature};
    ///
    /// let seed = Seed::from_bytes([0;32]);
    /// let xprv = XPrv::generate_from_seed(&seed);
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
impl Hash for XPub {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0)
    }
}
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
impl cbor_event::se::Serialize for XPub {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.write_bytes(self.as_ref())
    }
}
impl cbor_event::de::Deserialize for XPub {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let bytes = raw.bytes()?;
        match XPub::from_slice(&bytes) {
            Ok(pk) => Ok(pk),
            Err(Error::InvalidXPubSize(sz)) => Err(cbor_event::Error::NotEnough(sz, XPUB_SIZE)),
            Err(err) => Err(cbor_event::Error::CustomError(format!("unexpected error: {:?}", err))),
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
impl<T> cbor_event::se::Serialize for Signature<T> {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.write_bytes(self.as_ref())
    }
}
impl<T> cbor_event::de::Deserialize for Signature<T> {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let bytes = raw.bytes()?;
        match Signature::from_slice(&bytes) {
            Ok(signature) => Ok(signature),
            Err(Error::InvalidSignatureSize(sz)) => Err(cbor_event::Error::NotEnough(sz, SIGNATURE_SIZE)),
            Err(err) => Err(cbor_event::Error::CustomError(format!("unexpected error: {:?}", err))),
        }
    }
}

pub type ChainCode = [u8; CHAIN_CODE_SIZE];

pub type DerivationIndex = u32;

#[derive(Debug, PartialEq, Eq)]
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
    [(i >> 24) as u8, (i >> 16) as u8, (i >> 8) as u8, i as u8]
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
        let r = x[i].wrapping_add(y[i]);
        out[i] = r;
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
        let mut out = [0u8; 64];
        for i in 0..32 {
            out[i] = (y[i] << 3) + (acc & 0x8);
            acc = y[i] >> 5;
        }
        out
    };

    let mut r32 = [0u8;32];
    let mut r = [0u8;64];
    let mut carry = 0u16;
    for i in 0..32 {
        let v = x[i] as u16 + yfe8[i] as u16 + carry;
        r[i] = v as u8;
        carry = v >> 8;
    }
    if carry > 0 {
        r[32] = carry as u8;
    }
    sc_reduce(&mut r);
    r32.clone_from_slice(&r[0..32]);
    r32
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
        let prv = XPrv::from_bytes_verified(D1).unwrap();
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
        let _ = prv.derive(DerivationScheme::V1, derivation_index);
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
        let prv = XPrv::from_bytes_verified(D1).unwrap();
        let xpub = prv.public();
        let child_prv = prv.derive(DerivationScheme::V2, derivation_index);
        let child_xpub = xpub.derive(DerivationScheme::V2, derivation_index).unwrap();
        assert_eq!(child_prv.public(), child_xpub);
    }

    #[test]
    fn xprv_sign() {
        let prv = XPrv::from_bytes_verified(D1_H0).unwrap();
        do_sign(&prv, &D1_H0_SIGNATURE);
    }

    #[test]
    fn normalize_bytes() {
        let entropies = vec![
            super::super::bip::bip39::Entropy::from_slice(&[0;16]).unwrap(),
            super::super::bip::bip39::Entropy::from_slice(&[0x1f;20]).unwrap(),
            super::super::bip::bip39::Entropy::from_slice(&[0xda;24]).unwrap(),
            super::super::bip::bip39::Entropy::from_slice(&[0x2a;28]).unwrap(),
            super::super::bip::bip39::Entropy::from_slice(&[0xff;32]).unwrap(),
        ];
        for entropy in entropies {
            let mut bytes = [0; XPRV_SIZE];
            super::super::wallet::keygen::generate_seed(&entropy, b"trezor", &mut bytes);
            let xprv = XPrv::normalize_bytes(bytes);
            let bytes = xprv.0;
            // calling the from_bytes verified to check the xprv
            // is valid
            let _ = XPrv::from_bytes_verified(bytes).unwrap();
        }
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
    use bip::bip39;
    use cryptoxide::{blake2b::Blake2b};
    use cbor_event;

 #[allow(non_snake_case)]
 #[allow(dead_code)]
struct TestVector {
    /// BIP39 Seed
    seed: &'static [u8],
    /// Wallet's extended signature
    signature: &'static [u8;64],
    /// Wallet's extended private key
    // xPriv: &'static [u8;96],
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

    fn check_derivation(test_index: usize, test: &TestVector) {
        let mut xprv = XPrv::generate_from_daedalus_seed(&test.seed);

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

    fn check_mnemonics(test_index: usize, test: &TestVector) {
        let mnemonics = bip39::Mnemonics::from_string(&bip39::dictionary::ENGLISH, test.words)
            .expect("retrieve the mnemonics from the string");
        let entropy = bip39::Entropy::from_mnemonics(&mnemonics)
            .expect("retrieve the entropy from the mnemonics");

        let entropy_bytes = cbor_event::Value::Bytes(Vec::from(entropy.as_ref()));
        let entropy_cbor = cbor!(&entropy_bytes).expect("encode entropy in cbor");
        let seed = {
            let mut blake2b = Blake2b::new(32);
            Digest::input(&mut blake2b, &entropy_cbor);
            let mut out = [0;32];
            Digest::result(&mut blake2b, &mut out);
            Seed::from_bytes(out)
        };
        let seed_ref_hex = hex::encode(&test.seed[2..]);
        let seed_hex = hex::encode(seed.as_ref());

        assert_eq!(seed_ref_hex, seed_hex, "seed from test {}", test_index);
    }

    #[test]
    fn derivation() {
        let mut test_index = 0;
        for test in TEST_VECTORS.iter() {
            check_derivation(test_index, test);
            test_index += 1;
        }
    }

    #[test]
    fn mnemonics() {
        let mut test_index = 0;
        for test in TEST_VECTORS.iter() {
            check_mnemonics(test_index, test);
            test_index += 1;
        }
    }

const TEST_VECTORS : [TestVector;14] =
    [ TestVector {
        data_to_sign: "Hello World",
        path: & [],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 69, 177, 167, 95, 227, 17, 158, 19, 198, 246, 10, 185, 186, 103, 75, 66, 249, 70, 253, 197, 88, 224, 124, 131, 223, 160, 117, 28, 46, 186, 105, 199, 147, 49, 189, 138, 74, 151, 86, 98, 178, 54, 40, 164, 56, 160, 235, 167, 99, 103, 228, 76, 18, 202, 145, 179, 158, 197, 144, 99, 248, 96, 241, 13],
        // xPriv: & [ 96, 101, 169, 86, 177, 179, 65, 69, 196, 65, 111, 220, 59, 163, 39, 104, 1, 133, 14, 145, 167, 122, 49, 167, 190, 120, 36, 99, 40, 138, 234, 83, 96, 186, 110, 37, 177, 160, 33, 87, 251, 105, 197, 209, 215, 185, 108, 70, 25, 115, 110, 84, 84, 71, 6, 154, 106, 111, 11, 169, 8, 68, 188, 142, 100, 178, 15, 160, 130, 179, 20, 61, 107, 94, 237, 66, 198, 239, 99, 249, 149, 153, 208, 136, 138, 254, 6, 6, 32, 171, 193, 179, 25, 147, 95, 225, 115, 159, 75, 60, 172, 164, 201, 173, 79, 205, 75, 220, 46, 244, 44, 134, 1, 175, 141, 105, 70, 153, 158, 248, 94, 246, 174, 132, 246, 110, 114, 235],
        xPub: & [ 100, 178, 15, 160, 130, 179, 20, 61, 107, 94, 237, 66, 198, 239, 99, 249, 149, 153, 208, 136, 138, 254, 6, 6, 32, 171, 193, 179, 25, 147, 95, 225, 115, 159, 75, 60, 172, 164, 201, 173, 79, 205, 75, 220, 46, 244, 44, 134, 1, 175, 141, 105, 70, 153, 158, 248, 94, 246, 174, 132, 246, 110, 114, 235],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648 ],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 242, 201, 23, 23, 130, 231, 223, 118, 101, 18, 106, 197, 69, 174, 83, 176, 89, 100, 176, 22, 5, 54, 239, 219, 84, 94, 36, 96, 219, 190, 194, 177, 158, 198, 179, 56, 184, 241, 191, 77, 254, 233, 67, 96, 237, 2, 75, 17, 94, 55, 177, 215, 230, 243, 249, 174, 75, 235, 121, 83, 148, 40, 86, 15],
        // xPriv: & [ 231, 210, 117, 22, 83, 132, 3, 165, 58, 139, 4, 22, 86, 163, 245, 112, 144, 157, 246, 65, 160, 171, 129, 31, 231, 216, 124, 155, 160, 42, 131, 12, 121, 74, 44, 84, 173, 139, 82, 91, 120, 23, 115, 200, 125, 56, 203, 244, 25, 118, 54, 188, 66, 122, 157, 85, 19, 104, 40, 111, 228, 194, 148, 164, 149, 187, 130, 255, 213, 112, 119, 22, 188, 101, 23, 10, 180, 232, 218, 254, 237, 144, 251, 224, 206, 146, 88, 113, 59, 119, 81, 233, 98, 217, 49, 223, 103, 85, 203, 130, 232, 146, 214, 97, 76, 0, 122, 94, 251, 206, 178, 29, 149, 165, 36, 78, 38, 157, 14, 32, 107, 72, 185, 164, 149, 57, 11, 3],
        xPub: & [ 149, 187, 130, 255, 213, 112, 119, 22, 188, 101, 23, 10, 180, 232, 218, 254, 237, 144, 251, 224, 206, 146, 88, 113, 59, 119, 81, 233, 98, 217, 49, 223, 103, 85, 203, 130, 232, 146, 214, 97, 76, 0, 122, 94, 251, 206, 178, 29, 149, 165, 36, 78, 38, 157, 14, 32, 107, 72, 185, 164, 149, 57, 11, 3],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483649 ],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 43, 161, 67, 154, 230, 72, 167, 232, 218, 124, 154, 177, 238, 109, 169, 79, 212, 235, 227, 122, 189, 9, 120, 48, 110, 143, 186, 42, 250, 143, 17, 26, 136, 169, 147, 219, 240, 8, 190, 218, 233, 22, 127, 79, 104, 64, 158, 76, 157, 218, 240, 44, 186, 18, 65, 132, 71, 177, 132, 137, 7, 173, 128, 15],
        // xPriv: & [ 155, 90, 61, 154, 76, 96, 188, 212, 155, 182, 75, 114, 192, 130, 177, 100, 49, 77, 15, 97, 216, 66, 242, 87, 95, 209, 212, 251, 48, 162, 138, 12, 176, 147, 227, 118, 244, 30, 183, 191, 128, 171, 205, 0, 115, 165, 36, 85, 210, 91, 93, 33, 129, 91, 199, 88, 229, 246, 248, 21, 54, 174, 222, 187, 121, 252, 129, 84, 85, 75, 151, 228, 197, 110, 242, 249, 219, 180, 193, 66, 31, 241, 149, 9, 104, 137, 49, 161, 233, 100, 189, 165, 222, 192, 241, 159, 71, 162, 66, 113, 59, 209, 134, 8, 35, 17, 71, 192, 102, 182, 8, 59, 252, 30, 144, 102, 254, 201, 246, 33, 132, 76, 132, 254, 214, 34, 138, 52],
        xPub: & [ 121, 252, 129, 84, 85, 75, 151, 228, 197, 110, 242, 249, 219, 180, 193, 66, 31, 241, 149, 9, 104, 137, 49, 161, 233, 100, 189, 165, 222, 192, 241, 159, 71, 162, 66, 113, 59, 209, 134, 8, 35, 17, 71, 192, 102, 182, 8, 59, 252, 30, 144, 102, 254, 201, 246, 33, 132, 76, 132, 254, 214, 34, 138, 52],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 12, 211, 79, 132, 224, 210, 252, 177, 128, 11, 219, 14, 134, 155, 144, 65, 52, 153, 85, 206, 214, 106, 237, 190, 107, 218, 24, 126, 190, 141, 54, 166, 42, 5, 179, 150, 71, 233, 47, 204, 66, 170, 122, 115, 104, 23, 66, 64, 175, 186, 8, 184, 200, 31, 152, 26, 34, 249, 66, 214, 189, 120, 22, 2],
        // xPriv: & [ 82, 224, 201, 138, 166, 0, 207, 220, 209, 255, 40, 252, 218, 82, 39, 237, 135, 6, 63, 74, 152, 84, 122, 120, 183, 113, 5, 44, 241, 2, 180, 12, 108, 24, 217, 248, 7, 91, 26, 106, 24, 51, 84, 6, 7, 71, 155, 213, 139, 123, 235, 138, 131, 210, 187, 1, 202, 122, 224, 36, 82, 162, 88, 3, 220, 144, 124, 124, 6, 230, 49, 78, 237, 217, 225, 140, 159, 108, 111, 156, 196, 226, 5, 251, 28, 112, 218, 96, 130, 52, 195, 25, 241, 247, 176, 214, 214, 121, 132, 145, 185, 250, 70, 18, 55, 10, 229, 239, 60, 98, 58, 11, 104, 114, 243, 173, 143, 38, 151, 8, 133, 250, 103, 200, 59, 220, 66, 94],
        xPub: & [ 220, 144, 124, 124, 6, 230, 49, 78, 237, 217, 225, 140, 159, 108, 111, 156, 196, 226, 5, 251, 28, 112, 218, 96, 130, 52, 195, 25, 241, 247, 176, 214, 214, 121, 132, 145, 185, 250, 70, 18, 55, 10, 229, 239, 60, 98, 58, 11, 104, 114, 243, 173, 143, 38, 151, 8, 133, 250, 103, 200, 59, 220, 66, 94],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 2147483650],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 228, 31, 115, 219, 47, 141, 40, 150, 166, 135, 128, 43, 43, 231, 107, 124, 171, 183, 61, 251, 180, 137, 20, 148, 136, 58, 12, 189, 155, 187, 158, 95, 157, 62, 20, 210, 208, 176, 108, 102, 116, 51, 53, 8, 73, 109, 182, 96, 147, 103, 55, 192, 239, 217, 81, 21, 20, 20, 125, 172, 121, 250, 73, 5],
        // xPriv: & [ 17, 253, 100, 98, 163, 169, 43, 53, 194, 39, 3, 246, 241, 193, 36, 221, 207, 54, 183, 194, 176, 156, 194, 120, 79, 50, 14, 28, 250, 18, 236, 4, 194, 120, 88, 3, 198, 28, 70, 174, 202, 25, 42, 27, 177, 183, 178, 10, 140, 76, 199, 250, 1, 219, 87, 252, 93, 29, 138, 84, 115, 64, 35, 82, 131, 151, 117, 164, 24, 118, 227, 40, 152, 106, 162, 97, 104, 149, 139, 186, 17, 118, 230, 120, 25, 179, 87, 238, 168, 74, 252, 234, 184, 177, 219, 120, 65, 105, 162, 163, 46, 54, 24, 169, 3, 233, 48, 189, 26, 113, 48, 51, 163, 143, 146, 56, 144, 147, 64, 131, 148, 226, 154, 195, 122, 23, 82, 234],
        xPub: & [ 131, 151, 117, 164, 24, 118, 227, 40, 152, 106, 162, 97, 104, 149, 139, 186, 17, 118, 230, 120, 25, 179, 87, 238, 168, 74, 252, 234, 184, 177, 219, 120, 65, 105, 162, 163, 46, 54, 24, 169, 3, 233, 48, 189, 26, 113, 48, 51, 163, 143, 146, 56, 144, 147, 64, 131, 148, 226, 154, 195, 122, 23, 82, 234],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 2147483650, 2147483650],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 99, 16, 21, 53, 124, 238, 48, 81, 17, 107, 76, 47, 244, 209, 197, 190, 177, 59, 110, 80, 35, 99, 90, 161, 238, 176, 86, 60, 173, 240, 212, 251, 193, 11, 213, 227, 27, 74, 66, 32, 198, 120, 117, 85, 140, 65, 181, 204, 3, 40, 16, 74, 227, 156, 199, 255, 32, 255, 12, 43, 218, 89, 137, 6],
        // xPriv: & [ 91, 30, 92, 173, 2, 39, 75, 164, 97, 244, 112, 141, 133, 152, 211, 73, 127, 175, 143, 227, 232, 148, 163, 121, 87, 58, 166, 172, 58, 3, 229, 5, 186, 23, 157, 46, 60, 103, 170, 187, 72, 108, 72, 209, 96, 2, 181, 26, 211, 46, 171, 67, 76, 115, 138, 21, 80, 150, 35, 19, 176, 112, 152, 205, 117, 235, 141, 25, 126, 200, 98, 124, 133, 175, 136, 230, 106, 161, 228, 144, 101, 221, 138, 201, 142, 216, 153, 29, 181, 46, 206, 1, 99, 93, 251, 118, 58, 233, 201, 154, 89, 37, 203, 162, 220, 241, 33, 186, 243, 160, 37, 79, 61, 234, 35, 193, 41, 249, 235, 112, 168, 167, 232, 137, 124, 81, 153, 186],
        xPub: & [ 117, 235, 141, 25, 126, 200, 98, 124, 133, 175, 136, 230, 106, 161, 228, 144, 101, 221, 138, 201, 142, 216, 153, 29, 181, 46, 206, 1, 99, 93, 251, 118, 58, 233, 201, 154, 89, 37, 203, 162, 220, 241, 33, 186, 243, 160, 37, 79, 61, 234, 35, 193, 41, 249, 235, 112, 168, 167, 232, 137, 124, 81, 153, 186],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 2147483650, 2147483650, 3147483648],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 29, 225, 210, 117, 66, 139, 169, 73, 26, 67, 60, 212, 115, 205, 7, 108, 2, 127, 97, 231, 168, 181, 57, 29, 249, 222, 165, 203, 75, 200, 141, 138, 87, 176, 149, 144, 106, 48, 177, 62, 104, 37, 152, 81, 168, 221, 63, 87, 182, 240, 255, 163, 122, 93, 63, 252, 23, 18, 64, 242, 212, 4, 249, 1],
        // xPriv: & [ 98, 75, 71, 21, 15, 88, 223, 164, 66, 132, 251, 198, 60, 159, 153, 185, 183, 159, 128, 140, 73, 85, 164, 97, 240, 226, 190, 68, 235, 11, 229, 13, 9, 122, 160, 6, 214, 148, 177, 101, 239, 55, 207, 35, 86, 46, 89, 103, 201, 110, 73, 37, 93, 47, 32, 250, 174, 71, 141, 238, 131, 170, 91, 2, 5, 136, 88, 156, 217, 181, 29, 252, 2, 140, 242, 37, 103, 64, 105, 203, 229, 46, 14, 112, 222, 176, 45, 196, 91, 121, 178, 110, 227, 84, 139, 0, 21, 196, 80, 184, 109, 215, 221, 131, 179, 25, 81, 217, 238, 3, 235, 26, 121, 37, 22, 29, 129, 123, 213, 23, 198, 156, 240, 158, 54, 113, 241, 202],
        xPub: & [ 5, 136, 88, 156, 217, 181, 29, 252, 2, 140, 242, 37, 103, 64, 105, 203, 229, 46, 14, 112, 222, 176, 45, 196, 91, 121, 178, 110, 227, 84, 139, 0, 21, 196, 80, 184, 109, 215, 221, 131, 179, 25, 81, 217, 238, 3, 235, 26, 121, 37, 22, 29, 129, 123, 213, 23, 198, 156, 240, 158, 54, 113, 241, 202],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 2147483650, 2147483650, 3147483648],
        derivation_scheme: "derivation-scheme2",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 6, 89, 180, 164, 55, 100, 90, 197, 228, 99, 111, 18, 9, 34, 98, 119, 122, 151, 211, 67, 121, 168, 12, 35, 60, 186, 191, 232, 1, 90, 221, 180, 147, 194, 151, 220, 180, 115, 9, 65, 61, 181, 80, 124, 45, 104, 112, 202, 209, 158, 142, 19, 187, 217, 107, 181, 211, 51, 193, 184, 222, 61, 57, 13],
        // xPriv: & [ 104, 2, 173, 107, 239, 61, 246, 71, 223, 77, 29, 112, 228, 114, 67, 206, 153, 109, 165, 96, 170, 124, 53, 82, 82, 134, 181, 243, 63, 138, 234, 83, 150, 156, 245, 199, 46, 17, 22, 241, 37, 65, 239, 33, 116, 233, 250, 109, 14, 245, 89, 83, 180, 162, 205, 192, 1, 253, 49, 51, 131, 103, 202, 176, 92, 231, 23, 39, 87, 99, 212, 40, 3, 64, 177, 124, 34, 102, 71, 224, 202, 42, 227, 84, 191, 18, 48, 46, 205, 171, 79, 104, 214, 15, 117, 189, 144, 116, 171, 55, 6, 15, 138, 48, 131, 1, 110, 111, 55, 85, 222, 88, 1, 111, 32, 159, 106, 113, 3, 214, 59, 31, 128, 197, 63, 153, 219, 153],
        xPub: & [ 92, 231, 23, 39, 87, 99, 212, 40, 3, 64, 177, 124, 34, 102, 71, 224, 202, 42, 227, 84, 191, 18, 48, 46, 205, 171, 79, 104, 214, 15, 117, 189, 144, 116, 171, 55, 6, 15, 138, 48, 131, 1, 110, 111, 55, 85, 222, 88, 1, 111, 32, 159, 106, 113, 3, 214, 59, 31, 128, 197, 63, 153, 219, 153],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649],
        derivation_scheme: "derivation-scheme2",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 57, 187, 18, 182, 103, 242, 87, 134, 98, 255, 102, 125, 155, 187, 145, 12, 221, 198, 44, 73, 21, 53, 159, 133, 170, 109, 6, 135, 86, 239, 14, 75, 99, 242, 18, 34, 17, 88, 99, 17, 248, 105, 73, 160, 76, 197, 10, 251, 220, 189, 88, 169, 235, 183, 255, 197, 61, 164, 15, 79, 80, 156, 255, 11],
        // xPriv: & [ 56, 253, 152, 176, 208, 42, 170, 209, 15, 213, 202, 201, 202, 73, 83, 136, 147, 101, 2, 23, 135, 76, 98, 143, 107, 237, 4, 241, 45, 138, 234, 83, 53, 242, 101, 169, 96, 134, 204, 21, 130, 160, 33, 138, 38, 175, 170, 57, 109, 126, 185, 66, 146, 91, 89, 26, 92, 59, 107, 25, 125, 167, 246, 151, 105, 115, 241, 204, 85, 27, 87, 42, 250, 27, 209, 180, 179, 170, 176, 182, 52, 39, 101, 41, 243, 111, 218, 111, 7, 1, 149, 145, 7, 127, 95, 161, 245, 169, 113, 47, 193, 23, 102, 163, 253, 216, 157, 247, 104, 159, 78, 137, 30, 230, 64, 44, 230, 44, 37, 146, 6, 156, 209, 38, 9, 200, 169, 28],
        xPub: & [ 105, 115, 241, 204, 85, 27, 87, 42, 250, 27, 209, 180, 179, 170, 176, 182, 52, 39, 101, 41, 243, 111, 218, 111, 7, 1, 149, 145, 7, 127, 95, 161, 245, 169, 113, 47, 193, 23, 102, 163, 253, 216, 157, 247, 104, 159, 78, 137, 30, 230, 64, 44, 230, 44, 37, 146, 6, 156, 209, 38, 9, 200, 169, 28],
      }
    , TestVector {
        data_to_sign: "Data",
        path: & [ 2147483648, 2147483649, 24, 2000],
        derivation_scheme: "derivation-scheme2",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 181, 219, 221, 11, 145, 249, 5, 65, 41, 224, 207, 65, 95, 81, 185, 150, 126, 153, 51, 193, 131, 62, 144, 138, 149, 65, 52, 121, 184, 243, 57, 234, 58, 147, 249, 249, 227, 29, 201, 172, 12, 86, 26, 55, 29, 99, 133, 159, 196, 186, 1, 236, 14, 31, 232, 228, 85, 204, 166, 150, 63, 68, 13, 1],
        // xPriv: & [ 40, 5, 55, 1, 199, 248, 236, 183, 0, 132, 51, 206, 61, 43, 112, 78, 30, 24, 122, 183, 67, 60, 98, 28, 46, 72, 240, 52, 53, 138, 234, 83, 202, 169, 77, 38, 56, 83, 130, 169, 50, 116, 108, 113, 166, 195, 245, 168, 247, 166, 221, 40, 114, 87, 188, 32, 182, 52, 66, 172, 71, 172, 34, 58, 227, 18, 13, 24, 35, 120, 212, 160, 131, 244, 47, 144, 169, 196, 186, 2, 114, 189, 10, 99, 41, 227, 137, 106, 177, 148, 140, 253, 169, 185, 4, 32, 60, 0, 11, 80, 63, 132, 79, 227, 236, 34, 198, 198, 91, 205, 196, 203, 69, 170, 186, 152, 165, 202, 252, 5, 171, 37, 176, 67, 96, 73, 66, 19],
        xPub: & [ 227, 18, 13, 24, 35, 120, 212, 160, 131, 244, 47, 144, 169, 196, 186, 2, 114, 189, 10, 99, 41, 227, 137, 106, 177, 148, 140, 253, 169, 185, 4, 32, 60, 0, 11, 80, 63, 132, 79, 227, 236, 34, 198, 198, 91, 205, 196, 203, 69, 170, 186, 152, 165, 202, 252, 5, 171, 37, 176, 67, 96, 73, 66, 19],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483649, 24, 2147485648],
        derivation_scheme: "derivation-scheme2",
        passphrase: "",
        words: "ring crime symptom enough erupt lady behave ramp apart settle citizen junk",
        seed: & [ 88, 32, 46, 212, 199, 29, 145, 188, 104, 199, 181, 15, 238, 181, 188, 122, 120, 95, 232, 132, 221, 10, 237, 220, 224, 41, 223, 61, 97, 44, 211, 104, 15, 211],
        signature: & [ 53, 131, 252, 13, 24, 244, 25, 23, 4, 7, 248, 138, 199, 199, 4, 201, 78, 48, 209, 29, 105, 131, 38, 131, 26, 64, 43, 231, 65, 164, 182, 236, 92, 70, 78, 252, 57, 172, 210, 33, 58, 67, 63, 210, 79, 203, 33, 33, 153, 129, 42, 238, 233, 26, 42, 236, 217, 4, 60, 212, 215, 191, 152, 10],
        // xPriv: & [ 248, 134, 213, 60, 151, 76, 45, 190, 216, 35, 65, 29, 206, 217, 59, 194, 255, 72, 111, 225, 107, 227, 10, 180, 71, 72, 163, 243, 53, 138, 234, 83, 72, 71, 12, 249, 133, 216, 113, 36, 40, 165, 137, 111, 189, 121, 249, 201, 19, 232, 235, 46, 136, 104, 1, 112, 154, 251, 23, 69, 64, 97, 193, 152, 53, 86, 55, 241, 36, 158, 11, 182, 196, 84, 9, 114, 137, 131, 98, 242, 71, 217, 242, 185, 244, 171, 117, 222, 13, 148, 237, 136, 0, 81, 74, 27, 117, 134, 67, 112, 95, 234, 81, 191, 233, 49, 109, 141, 108, 209, 49, 91, 65, 79, 231, 171, 37, 21, 148, 156, 184, 138, 204, 197, 236, 203, 150, 228],
        xPub: & [ 53, 86, 55, 241, 36, 158, 11, 182, 196, 84, 9, 114, 137, 131, 98, 242, 71, 217, 242, 185, 244, 171, 117, 222, 13, 148, 237, 136, 0, 81, 74, 27, 117, 134, 67, 112, 95, 234, 81, 191, 233, 49, 109, 141, 108, 209, 49, 91, 65, 79, 231, 171, 37, 21, 148, 156, 184, 138, 204, 197, 236, 203, 150, 228],
      }
      , TestVector {
        data_to_sign: "Hello World",
        path: & [],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "leaf immune metal phrase river cool domain snow year below result three",
        seed: & [ 88, 32, 125, 97, 13, 1, 77, 51, 0, 85, 70, 52, 144, 202, 73, 13, 215, 83, 233, 244, 211, 149, 250, 162, 176, 35, 122, 23, 245, 216, 254, 190, 172, 68],
        signature: & [ 206, 16, 29, 142, 121, 242, 95, 165, 43, 154, 79, 144, 190, 78, 191, 253, 124, 100, 58, 186, 156, 96, 188, 51, 93, 19, 117, 96, 145, 135, 201, 60, 161, 14, 7, 202, 81, 14, 176, 22, 97, 177, 181, 227, 132, 59, 91, 181, 176, 46, 248, 135, 2, 250, 4, 129, 179, 217, 110, 229, 37, 251, 4, 5],
        // xPriv: & [ 80, 209, 181, 37, 129, 173, 239, 163, 233, 144, 37, 173, 232, 247, 24, 147, 24, 225, 233, 172, 47, 10, 29, 102, 217, 161, 200, 111, 57, 8, 202, 95, 225, 165, 224, 136, 102, 181, 0, 169, 160, 225, 29, 72, 196, 29, 187, 73, 87, 197, 80, 180, 24, 231, 181, 198, 201, 165, 49, 171, 55, 3, 124, 53, 199, 220, 27, 150, 169, 206, 224, 8, 2, 183, 91, 246, 133, 197, 39, 0, 95, 195, 223, 210, 10, 43, 92, 114, 121, 254, 13, 146, 234, 81, 191, 3, 208, 233, 236, 170, 180, 87, 200, 222, 165, 86, 187, 46, 244, 62, 197, 156, 201, 67, 177, 42, 219, 57, 201, 211, 141, 77, 144, 86, 59, 144, 20, 167],
        xPub: & [ 199, 220, 27, 150, 169, 206, 224, 8, 2, 183, 91, 246, 133, 197, 39, 0, 95, 195, 223, 210, 10, 43, 92, 114, 121, 254, 13, 146, 234, 81, 191, 3, 208, 233, 236, 170, 180, 87, 200, 222, 165, 86, 187, 46, 244, 62, 197, 156, 201, 67, 177, 42, 219, 57, 201, 211, 141, 77, 144, 86, 59, 144, 20, 167],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648 ],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "leaf immune metal phrase river cool domain snow year below result three",
        seed: & [ 88, 32, 125, 97, 13, 1, 77, 51, 0, 85, 70, 52, 144, 202, 73, 13, 215, 83, 233, 244, 211, 149, 250, 162, 176, 35, 122, 23, 245, 216, 254, 190, 172, 68],
        signature: & [ 69, 183, 75, 168, 122, 123, 22, 8, 13, 113, 83, 197, 82, 35, 26, 46, 225, 183, 153, 146, 176, 96, 24, 200, 139, 245, 80, 251, 189, 177, 205, 87, 198, 45, 108, 23, 113, 68, 52, 31, 94, 184, 199, 123, 1, 243, 114, 206, 181, 229, 94, 155, 22, 142, 105, 250, 73, 77, 2, 197, 192, 53, 67, 6],
        // xPriv: & [ 14, 11, 245, 52, 7, 46, 253, 178, 231, 57, 165, 216, 33, 39, 172, 179, 151, 177, 26, 221, 31, 195, 45, 86, 202, 242, 8, 104, 178, 160, 10, 8, 175, 176, 132, 22, 150, 190, 214, 17, 212, 174, 28, 204, 254, 38, 190, 56, 165, 214, 223, 164, 240, 85, 23, 252, 105, 178, 17, 62, 211, 53, 193, 31, 22, 78, 242, 8, 99, 42, 109, 131, 55, 79, 197, 182, 219, 254, 28, 158, 166, 222, 30, 198, 116, 34, 155, 216, 123, 206, 210, 38, 236, 42, 245, 1, 200, 74, 50, 232, 107, 238, 130, 102, 131, 239, 62, 8, 4, 205, 95, 43, 81, 182, 112, 247, 114, 85, 195, 197, 129, 173, 212, 120, 157, 128, 156, 63],
        xPub: & [ 22, 78, 242, 8, 99, 42, 109, 131, 55, 79, 197, 182, 219, 254, 28, 158, 166, 222, 30, 198, 116, 34, 155, 216, 123, 206, 210, 38, 236, 42, 245, 1, 200, 74, 50, 232, 107, 238, 130, 102, 131, 239, 62, 8, 4, 205, 95, 43, 81, 182, 112, 247, 114, 85, 195, 197, 129, 173, 212, 120, 157, 128, 156, 63],
      }
    , TestVector {
        data_to_sign: "Hello World",
        path: & [ 2147483648, 2147483648],
        derivation_scheme: "derivation-scheme1",
        passphrase: "",
        words: "leaf immune metal phrase river cool domain snow year below result three",
        seed: & [ 88, 32, 125, 97, 13, 1, 77, 51, 0, 85, 70, 52, 144, 202, 73, 13, 215, 83, 233, 244, 211, 149, 250, 162, 176, 35, 122, 23, 245, 216, 254, 190, 172, 68],
        signature: & [ 13, 208, 10, 118, 61, 241, 62, 187, 68, 2, 96, 15, 171, 8, 102, 166, 179, 139, 201, 25, 71, 71, 79, 129, 96, 89, 226, 42, 66, 1, 66, 107, 229, 83, 149, 131, 34, 32, 149, 167, 69, 173, 15, 243, 96, 114, 28, 241, 67, 125, 76, 194, 123, 122, 163, 37, 128, 45, 109, 140, 249, 7, 126, 4],
        // xPriv: & [ 69, 160, 184, 196, 99, 247, 209, 218, 88, 170, 159, 18, 121, 102, 201, 227, 15, 218, 138, 69, 200, 43, 54, 198, 66, 235, 225, 200, 42, 145, 51, 8, 202, 164, 17, 3, 254, 189, 173, 206, 151, 111, 213, 29, 138, 21, 88, 235, 8, 239, 162, 116, 148, 230, 190, 201, 94, 107, 170, 37, 244, 126, 194, 39, 173, 84, 27, 134, 66, 198, 63, 6, 174, 99, 11, 118, 133, 226, 104, 43, 38, 66, 35, 85, 9, 180, 106, 212, 178, 55, 173, 214, 167, 136, 254, 127, 159, 116, 94, 167, 137, 90, 206, 219, 4, 95, 183, 240, 108, 111, 107, 66, 21, 138, 134, 209, 203, 224, 229, 238, 128, 35, 176, 238, 17, 51, 148, 199],
        xPub: & [ 173, 84, 27, 134, 66, 198, 63, 6, 174, 99, 11, 118, 133, 226, 104, 43, 38, 66, 35, 85, 9, 180, 106, 212, 178, 55, 173, 214, 167, 136, 254, 127, 159, 116, 94, 167, 137, 90, 206, 219, 4, 95, 183, 240, 108, 111, 107, 66, 21, 138, 134, 209, 203, 224, 229, 238, 128, 35, 176, 238, 17, 51, 148, 199],
      }
    ];
}
