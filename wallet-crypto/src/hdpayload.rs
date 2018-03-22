extern crate rcw;

use self::rcw::chacha20poly1305::{ChaCha20Poly1305};
use self::rcw::aead::{AeadEncryptor, AeadDecryptor};
use self::rcw::hmac::{Hmac};
use self::rcw::sha2::{Sha512};
use self::rcw::pbkdf2::{pbkdf2};

use std::iter::repeat;

use hdwallet::{XPub};
use cbor;
use cbor::hs::{ToCBOR, FromCBOR};

const NONCE : &'static [u8] = b"serokellfore";
const SALT  : &'static [u8] = b"address-hashing";
const TAG_LEN : usize = 16;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Path(Vec<u32>);
impl AsRef<[u32]> for Path {
    fn as_ref(&self) -> &[u32] { self.0.as_ref() }
}
impl Path {
    pub fn new(v: Vec<u32>) -> Self { Path(v) }
    fn from_cbor(bytes: &[u8]) -> Option<Self> {
        let mut cbor_decoder = cbor::decode::Decoder::new();
        let mut path = vec![];
        cbor_decoder.extend(bytes);

        let l = cbor_decoder.array_start().unwrap();
        for _ in 0..l {
            path.push(cbor_decoder.uint().unwrap() as u32);
        };
        Some(Path::new(path))
    }
    fn cbor(&self) -> Vec<u8> {
        let mut buf = vec![];
        cbor::encode::array_start(self.as_ref().len(), &mut buf);
        self.as_ref().iter().for_each(|b| cbor::encode::uint((b.clone() as u64), &mut buf));
        buf
    }
}

pub const HDKEY_SIZE : usize = 32;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
        Path::from_cbor(&out)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
impl ToCBOR for HDAddressPayload {
    fn encode(&self, buf: &mut Vec<u8>) {
        let mut vec = vec![];
        cbor::encode::bs(self.as_ref(), &mut vec);
        cbor::encode::bs(&vec         , buf);
    }
}
impl FromCBOR for HDAddressPayload {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let bs = decoder.bs()?;

        let mut dec = cbor::decode::Decoder::new();
        dec.extend(&bs);
        let vec = dec.bs()?;
        Ok(HDAddressPayload::from_vec(vec))
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
        assert_eq!(Some(path), Path::from_cbor(&cbor));
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
