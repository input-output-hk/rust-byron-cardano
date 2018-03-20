extern crate rcw;

use self::rcw::chacha20poly1305::{ChaCha20Poly1305};
use self::rcw::aead::{AeadEncryptor, AeadDecryptor};
use self::rcw::hmac::{Hmac};
use self::rcw::sha2::{Sha512};
use self::rcw::pbkdf2::{pbkdf2};

use std::iter::repeat;

use hdwallet::{XPub};

const NONCE : [u8;12] = [115,101,114,111,107,101,108,108,102,111,114,101]; // "serokellfore"
const SALT : [u8;15] = [97,100,100,114,101,115,115,45,104,97,115,104,105,110,103]; // "address-hashing"
const TAG_LEN : usize = 16;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Path(Vec<u32>);
impl AsRef<[u32]> for Path {
    fn as_ref(&self) -> &[u32] { self.0.as_ref() }
}
impl Path {
    pub fn new(v: Vec<u32>) -> Self { Path(v) }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HDKey([u8;32]);
impl AsRef<[u8]> for HDKey {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl HDKey {
    pub fn new(root_pub: &XPub) -> Self {
        let mut mac = Hmac::new(Sha512::new(), &root_pub[..]);
        let mut result = [0;32];
        let iters = 500;
        pbkdf2(&mut mac, &SALT[..], iters, &mut result);
        HDKey(result)
    }

    pub fn encrypt(&self, derivation_path: &Path) -> HDAddressPayload {
        let mut ctx = ChaCha20Poly1305::new(self.as_ref(), &NONCE[..], &[]);
        let input = []; // encrypt CBOR path
        let len = input.len();

        // allocate input length + TAG of 16
        let mut out: Vec<u8> = repeat(0).take(len + TAG_LEN).collect();
        let mut tag = [0;TAG_LEN];

        ctx.encrypt(&input, &mut out[0..len], &mut tag);
        out.extend_from_slice(&tag[..]);
        HDAddressPayload::from_vec(out)
    }

    pub fn decrypt(&self, payload: &HDAddressPayload) -> Option<Path> {
        let len = payload.len();
        if len < TAG_LEN {
            None
        } else {
            let dataLen = len - TAG_LEN;
            let mut ctx = ChaCha20Poly1305::new(self.as_ref(), &NONCE[..], &[]);
            let mut out: Vec<u8> = repeat(0).take(dataLen).collect();
            match ctx.decrypt(&payload.as_ref()[0..dataLen], &mut out[..], &payload.as_ref()[dataLen..]) {
                True => None,
                False => None
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HDAddressPayload(Vec<u8>);
impl AsRef<[u8]> for HDAddressPayload {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl HDAddressPayload {
    fn from_vec(v: Vec<u8>) -> Self { HDAddressPayload(v) }
    fn from_bytes(bytes: &[u8]) -> Self {
        HDAddressPayload::from_vec(bytes.iter().cloned().collect())
    }
    pub fn len(&self) -> usize { self.0.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hdwallet;

    #[test]
    fn hdpayload() {
        let path = Path::new(vec![0,1,2]);
        let sk = hdwallet::generate(&[0;32]);
        let pk = hdwallet::to_public(&sk);

        let key = HDKey::new(&pk);
        let payload = key.encrypt(&path);
        assert_eq!(Some(path), key.decrypt(&payload))
    }
}
