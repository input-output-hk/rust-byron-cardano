extern crate rcw;

use self::rcw::chacha20poly1305::{ChaCha20Poly1305};
use self::rcw::aead::{AeadEncryptor, AeadDecryptor};
use self::rcw::hmac::{Hmac};
use self::rcw::sha2::{Sha512};
use self::rcw::pbkdf2::{pbkdf2};

use std::iter::repeat;

use hdwallet::{XPub};

type Path = Vec<u32>;

pub struct HDKey([u8;32]);

impl AsRef<[u8]> for HDKey {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}

type HDAddressPayload = Vec<u8>;

const NONCE : [u8;12] = [115,101,114,111,107,101,108,108,102,111,114,101]; // "serokellfore"
const SALT : [u8;15] = [97,100,100,114,101,115,115,45,104,97,115,104,105,110,103]; // "address-hashing"
const TAG_LEN : usize = 16;

pub fn make_hdkey(root_pub: &XPub) -> HDKey {
    let mut mac = Hmac::new(Sha512::new(), &root_pub[..]);
    let mut result = [0;32];
    let iters = 500;
    pbkdf2(&mut mac, &SALT[..], iters, &mut result);
    HDKey(result)
}

pub fn encrypt_derivation_path(key: &HDKey, derivation_path: &Path) -> HDAddressPayload {
    let mut ctx = ChaCha20Poly1305::new(key.as_ref(), &NONCE[..], &[]);
    let input = []; // encrypt CBOR path
    let len = input.len();

    // allocate input length + TAG of 16
    let mut out: Vec<u8> = repeat(0).take(len + TAG_LEN).collect();
    let mut tag = [0;TAG_LEN];

    ctx.encrypt(&input, &mut out[0..len], &mut tag);
    out.extend_from_slice(&tag[..]);
    out
}

pub fn decrypt_derivation_path(key: &HDKey, payload: &HDAddressPayload) -> Option<Path> {
    let len = payload.len();
    if len < TAG_LEN {
        None
    } else {
        let dataLen = len - TAG_LEN;
        let mut ctx = ChaCha20Poly1305::new(key.as_ref(), &NONCE[..], &[]);
        let mut out: Vec<u8> = repeat(0).take(dataLen).collect();
        match ctx.decrypt(&payload[0..dataLen], &mut out[..], &payload[dataLen..]) {
            True => None,
            False => None
        }
    }
}
