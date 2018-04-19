extern crate rcw;

use self::rcw::chacha20poly1305::{ChaCha20Poly1305};
use self::rcw::aead::{AeadEncryptor, AeadDecryptor};
use self::rcw::hmac::{Hmac};
use self::rcw::sha2::{Sha512};
use self::rcw::pbkdf2::{pbkdf2};

use std::iter::repeat;

use hdwallet::{XPub};
use cbor;
use cbor::{ExtendedResult};

const NONCE : &'static [u8] = b"serokellfore";
const SALT  : &'static [u8] = b"address-hashing";
const TAG_LEN : usize = 16;

const BIP44_PATH_LENGTH: usize = 5;
const BIP44_PURPOSE   : u32 = 0x8000002C;
const BIP44_COIN_TYPE : u32 = 0x80000717;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Path(Vec<u32>);
impl AsRef<[u32]> for Path {
    fn as_ref(&self) -> &[u32] { self.0.as_ref() }
}
impl Path {
    pub fn new(v: Vec<u32>) -> Self { Path(v) }
    fn from_cbor(bytes: &[u8]) -> cbor::Result<Self> {
        cbor::decode_from_cbor(bytes)
    }
    fn cbor(&self) -> Vec<u8> { cbor::encode_to_cbor(self).unwrap() }

    pub fn bip44_new(account: u32, change: u32, index: u32) -> Path {
        Path(vec![BIP44_PURPOSE, BIP44_COIN_TYPE, account, change, index])
    }
    pub fn bip44_acount(&self) -> u32 {
        assert!(self.as_ref().len() == BIP44_PATH_LENGTH);
        self.0[2]
    }
    pub fn bip44_change(&self) -> u32 {
        assert!(self.as_ref().len() == BIP44_PATH_LENGTH);
        self.0[3]
    }
    pub fn bip44_index(&self) -> u32 {
        assert!(self.as_ref().len() == BIP44_PATH_LENGTH);
        self.0[4]
    }
    pub fn bip44_next(&self) -> Path {
        assert!(self.as_ref().len() == BIP44_PATH_LENGTH);
        let index = self.as_ref()[4];
        Path::bip44_new(self.bip44_acount(), 0, self.bip44_index() + 1)
    }
    pub fn bip44_next_change(&self) -> Path {
        assert!(self.as_ref().len() == BIP44_PATH_LENGTH);
        let index = self.as_ref()[4];
        Path::bip44_new(self.bip44_acount(), 1, self.bip44_index() + 1)
    }
}
impl cbor::CborValue for Path {
    fn encode(&self) -> cbor::Value { cbor::Value::Array(self.0.iter().map(cbor::CborValue::encode).collect()) }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|vec| {
            let mut v = vec![];
            for el in vec.iter() { v.push(cbor::CborValue::decode(el.clone())?); }
            Ok(Path::new(v))
        }).embed("while decoding Path")
    }
}

pub const HDKEY_SIZE : usize = 32;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HDKey([u8;HDKEY_SIZE]);
impl AsRef<[u8]> for HDKey {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl HDKey {
    pub fn new(root_pub: &XPub) -> Self {
        let mut mac = Hmac::new(Sha512::new(), root_pub.as_ref());
        let mut result = [0;HDKEY_SIZE];
        let iters = 500;
        pbkdf2(&mut mac, &SALT[..], iters, &mut result);
        HDKey(result)
    }

    /// create a `HDKey` by taking ownership of the given bytes
    pub fn from_bytes(bytes: [u8;HDKEY_SIZE]) -> Self { HDKey(bytes) }
    /// create a `HDKey` fromt the given slice
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == HDKEY_SIZE {
            let mut v = [0u8;HDKEY_SIZE];
            v[0..HDKEY_SIZE].clone_from_slice(bytes);
            Some(HDKey::from_bytes(v))
        } else {
            None
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

    pub fn decrypt(&self, input: &[u8]) -> Option<Vec<u8>> {
        let len = input.len() - TAG_LEN;
        if len <= 0 { return None; };

        let mut ctx = ChaCha20Poly1305::new(self.as_ref(), &NONCE[..], &[]);

        let mut out: Vec<u8> = repeat(0).take(len).collect();

        if ctx.decrypt(&input[..len], &mut out[..], &input[len..]) {
            Some(out)
        } else {
            None
        }
    }

    pub fn encrypt_path(&self, derivation_path: &Path) -> HDAddressPayload {
        let input = derivation_path.cbor();
        let out = self.encrypt(&input);

        HDAddressPayload::from_vec(out)
    }

    pub fn decrypt_path(&self, payload: &HDAddressPayload) -> Option<Path> {
        let out = self.decrypt(payload.as_ref())?;
        Path::from_cbor(&out).ok()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
impl cbor::CborValue for HDAddressPayload {
    fn encode(&self) -> cbor::Value {
        let vec = cbor::encode_to_cbor(&cbor::Bytes::new(self.0.clone())).unwrap();
        cbor::Value::Bytes(cbor::Bytes::new(vec))
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.bytes().and_then(|bytes| {
            let b : cbor::Bytes = cbor::decode_from_cbor(bytes.as_ref()).embed("while decoding the serialised cbor")?;
            Ok(b.to_vec())
        }).map(HDAddressPayload::from_vec)
        .embed("while decoding HDAddressPayload")
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
        assert_eq!(Some(bytes), key.decrypt(&payload))
    }

    #[test]
    fn path_cbor_encoding() {
        let path = Path::new(vec![0,1,2]);
        let cbor = path.cbor();
        assert_eq!(Ok(path), Path::from_cbor(cbor.as_ref()));
    }

    #[test]
    fn hdpayload() {
        let path = Path::new(vec![0,1,2]);
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt_path(&path);
        assert_eq!(Some(path), key.decrypt_path(&payload))
    }
}
