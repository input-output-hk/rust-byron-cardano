//! HD Payload
//!
//! The HD Payload is an Address attribute stored along the address
//! in encrypted form.
//!
//! This use chacha20poly1305 to auth-encrypt a BIP39 derivation
//! path, which is then stored in the address. The owner of the
//! symmetric key used to encrypt, can then decrypt the address
//! payload and find the derivation path associated with it.
//!
use cryptoxide::chacha20poly1305::{ChaCha20Poly1305};
use cryptoxide::hmac::{Hmac};
use cryptoxide::sha2::{Sha512};
use cryptoxide::pbkdf2::{pbkdf2};

use std::{iter::repeat, ops::{Deref}, fmt};

use hdwallet::{XPub};
use cbor_event::{self, de::RawCbor, se::{self, Serializer}};

use util::{securemem, hex};

const NONCE : &'static [u8] = b"serokellfore";
const SALT  : &'static [u8] = b"address-hashing";
const TAG_LEN : usize = 16;

#[derive(Debug)]
pub enum Error {
    InvalidHDKeySize(usize),
    CannotDecrypt,
    NotEnoughEncryptedData,
    CborError(cbor_event::Error)
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self { Error::CborError(e) }
}

pub type Result<T> = ::std::result::Result<T, Error>;

/// A derivation path of HD wallet derivation indices which uses a CBOR encoding
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Path(Vec<u32>);
impl Deref for Path {
    type Target = [u32];
    fn deref(&self) -> &Self::Target { self.0.deref() }
}
impl AsRef<[u32]> for Path {
    fn as_ref(&self) -> &[u32] { self.0.as_ref() }
}
impl Path {
    pub fn new(v: Vec<u32>) -> Self { Path(v) }
    fn from_cbor(bytes: &[u8]) -> Result<Self> {
        let mut raw = RawCbor::from(bytes);
        Ok(cbor_event::de::Deserialize::deserialize(&mut raw)?)
    }
    fn cbor(&self) -> Vec<u8> {
        cbor!(self)
            .expect("Serialize the given Path in cbor")
    }
}
impl cbor_event::se::Serialize for Path {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        se::serialize_indefinite_array(self.0.iter(), serializer)
    }
}
impl cbor_event::Deserialize for Path {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(Path(raw.deserialize()?))
    }
}

pub const HDKEY_SIZE : usize = 32;

/// The key to encrypt and decrypt HD payload
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HDKey([u8;HDKEY_SIZE]);
impl AsRef<[u8]> for HDKey {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl HDKey {
    /// Create a new `HDKey` from an extended public key
    pub fn new(root_pub: &XPub) -> Self {
        let mut mac = Hmac::new(Sha512::new(), root_pub.as_ref());
        let mut result = [0;HDKEY_SIZE];
        let iters = 500;
        pbkdf2(&mut mac, &SALT[..], iters, &mut result);
        HDKey(result)
    }

    /// create a `HDKey` by taking ownership of the given bytes
    pub fn from_bytes(bytes: [u8;HDKEY_SIZE]) -> Self { HDKey(bytes) }
    /// create a `HDKey` from the given slice
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() == HDKEY_SIZE {
            let mut v = [0u8;HDKEY_SIZE];
            v[0..HDKEY_SIZE].clone_from_slice(bytes);
            Ok(HDKey::from_bytes(v))
        } else {
            Err(Error::InvalidHDKeySize(bytes.len()))
        }
    }

    pub fn encrypt(&self, input: &[u8]) -> Vec<u8> {
        let mut ctx = ChaCha20Poly1305::new(self.as_ref(), &NONCE[..], &[]);

        let len = input.len();

        let mut out: Vec<u8> = repeat(0).take(len).collect();
        let mut tag = [0;TAG_LEN];

        ctx.encrypt(&input, &mut out[0..len], &mut tag);
        out.extend_from_slice(&tag[..]);
        out
    }

    pub fn decrypt(&self, input: &[u8]) -> Result<Vec<u8>> {
        let len = input.len() - TAG_LEN;
        if len <= 0 { return Err(Error::NotEnoughEncryptedData); };

        let mut ctx = ChaCha20Poly1305::new(self.as_ref(), &NONCE[..], &[]);

        let mut out: Vec<u8> = repeat(0).take(len).collect();

        if ctx.decrypt(&input[..len], &mut out[..], &input[len..]) {
            Ok(out)
        } else {
            Err(Error::CannotDecrypt)
        }
    }

    pub fn encrypt_path(&self, derivation_path: &Path) -> HDAddressPayload {
        let input = derivation_path.cbor();
        let out = self.encrypt(&input);

        HDAddressPayload::from_vec(out)
    }

    pub fn decrypt_path(&self, payload: &HDAddressPayload) -> Result<Path> {
        let out = self.decrypt(payload.as_ref())?;
        Path::from_cbor(&out)
    }
}
impl Drop for HDKey {
    fn drop(&mut self) {
        securemem::zero(&mut self.0);
    }
}

/// The address attributes payload, that should contains an encrypted derivation path with a MAC tag
///
/// It's however possible to store anything in this attributes, including
/// non encrypted information.
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HDAddressPayload(Vec<u8>);
impl AsRef<[u8]> for HDAddressPayload {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl HDAddressPayload {
    pub fn from_vec(v: Vec<u8>) -> Self { HDAddressPayload(v) }
    pub fn from_bytes(bytes: &[u8]) -> Self {
        HDAddressPayload::from_vec(bytes.iter().cloned().collect())
    }
    pub fn len(&self) -> usize { self.0.len() }
}
impl cbor_event::se::Serialize for HDAddressPayload {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        se::serialize_cbor_in_cbor(self.0.as_slice(), serializer)
    }
}
impl cbor_event::de::Deserialize for HDAddressPayload {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let mut raw_encoded = RawCbor::from(&raw.bytes()?);
        Ok(HDAddressPayload::from_bytes(&mut raw_encoded.bytes()?))
    }
}
impl Deref for HDAddressPayload {
    type Target = [u8];
    fn deref(&self) -> &Self::Target { self.0.as_ref() }
}
impl fmt::Debug for HDAddressPayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hdwallet;

    #[test]
    fn encrypt() {
        let bytes = vec![42u8; 256];
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt(&bytes);
        assert_eq!(bytes, key.decrypt(&payload).unwrap())
    }

    #[test]
    fn path_cbor_encoding() {
        let path = Path::new(vec![0,1,2]);
        let cbor = path.cbor();
        assert_eq!(path, Path::from_cbor(cbor.as_ref()).unwrap());
    }

    #[test]
    fn hdpayload() {
        let path = Path::new(vec![0,1,2]);
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt_path(&path);
        assert_eq!(path, key.decrypt_path(&payload).unwrap())
    }

    #[test]
    fn unit1() {
        let key = HDKey::from_bytes([0u8;32]);
        let dat = [0x9f, 0x00, 0x01, 0x0ff];
        let expected = [0xda, 0xac, 0x4a, 0x55, 0xfc, 0xa7, 0x48, 0xf3, 0x2f, 0xfa, 0xf4, 0x9e, 0x2b, 0x41, 0xab, 0x86, 0xf3, 0x54, 0xdb, 0x96];
        let got = key.encrypt(&dat[..]);
        assert_eq!(&expected[..], &got[..])
    }

    #[test]
    fn unit2() {
        let path = Path::new(vec![0,1]);
        let expected = [0x9f, 0x00, 0x01, 0x0ff];
        let cbor = path.cbor();
        assert_eq!(&expected[..], &cbor[..])
    }
}

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench {
    use hdwallet;
    use hdpayload::{self, *};
    use test;

    #[bench]
    fn decrypt_fail(b: &mut test::Bencher) {
        let path = Path::new(vec![0,1]);
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt_path(&path);

        let seed = hdwallet::Seed::from_bytes([1;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let key = HDKey::new(&pk);
        b.iter(|| {
            let _ = key.decrypt(&payload);
        })
    }

    #[bench]
    fn decrypt_ok(b: &mut test::Bencher) {
        let path = Path::new(vec![0,1]);
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt_path(&path);

        b.iter(|| {
            let _ = key.decrypt(&payload);
        })
    }

    #[bench]
    fn decrypt_with_cbor(b: &mut test::Bencher) {
        let path = Path::new(vec![0,1]);
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt_path(&path);

        b.iter(|| {
            let _ = key.decrypt_path(&payload);
        })
    }
}
