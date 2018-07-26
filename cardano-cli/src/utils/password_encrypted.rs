//! interfaces for password encrypted data
//!
//! These functions provide useful ready to use

use rand;
use std::{io::{Write, Read}, iter::repeat};

use cryptoxide::{pbkdf2::pbkdf2, chacha20poly1305::ChaCha20Poly1305, sha2::Sha512, hmac::Hmac};

const PASSWORD_DERIVATION_ITERATIONS : u32 = 10_000;
const SALT_SIZE  : usize = 16;

const NONCE_SIZE : usize = 12;
const TAG_SIZE   : usize = 16;
const KEY_SIZE   : usize = 32;

pub type Password = [u8];
type Key   = [u8;KEY_SIZE];
type Salt  = [u8;SALT_SIZE];
type Nonce = [u8;NONCE_SIZE];

pub fn encrypt(password: &Password, data: &[u8]) -> Vec<u8> {
    let salt = generate_salt();
    let nonce = generate_nonce();
    let mut key = [0;KEY_SIZE];
    let mut tag = [0;TAG_SIZE];
    let len = data.len();

    let mut bytes = Vec::with_capacity(SALT_SIZE + NONCE_SIZE + len + TAG_SIZE);
    let mut encrypted : Vec<u8> = repeat(0).take(data.len()).collect();

    // here we can safely unwrap, `Vec::with_capacity` should have provided
    // enough pre-allocated memory. If not, then there is a memory issue,
    // and there is nothing we can do.
    bytes.write_all(&salt[..]).unwrap();
    bytes.write_all(&nonce[..]).unwrap();

    password_to_key(password, salt, &mut key);
    let mut ctx = ChaCha20Poly1305::new(&key[..], &nonce[..], &[]);

    ctx.encrypt(data, &mut encrypted[0..len], &mut tag);
    encrypted.extend_from_slice(&tag[..]);

    bytes.append(&mut encrypted);
    bytes
}

pub fn decrypt(password: &Password, data: &[u8]) -> Option<Vec<u8>> {
    let mut reader = data;
    let mut salt   = [0;SALT_SIZE];
    let mut nonce  = [0;NONCE_SIZE];
    let mut key    = [0;KEY_SIZE];
    let len = data.len() - TAG_SIZE - SALT_SIZE - NONCE_SIZE;
    let mut bytes : Vec<u8> = repeat(0).take(len).collect();

    reader.read_exact(&mut salt[..]).unwrap();
    reader.read_exact(&mut nonce[..]).unwrap();

    password_to_key(password, salt, &mut key);
    let mut ctx = ChaCha20Poly1305::new(&key[..], &nonce[..], &[]);
    if ctx.decrypt(&reader[0..len], &mut bytes[..], &reader[len..]) {
        Some(bytes)
    } else {
        None
    }
}

fn password_to_key(password: &Password, salt: Salt, key: &mut Key) {
    let mut mac = Hmac::new(Sha512::new(), password);

    pbkdf2(&mut mac, &salt[..], PASSWORD_DERIVATION_ITERATIONS, key);
}

fn generate_salt() -> Salt {
    rand::random()
}

fn generate_nonce() -> Nonce {
    rand::random()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encrypt_decrypt() {
        const PASSWORD       : &'static [u8] = b"my awesome password";
        const WRONG_PASSWORD : &'static [u8] = b"my invalid password";
        const DATA           : &'static [u8] = b"some data I need to protect";

        let encrypted = encrypt(PASSWORD, DATA);

        let decrypted = decrypt(PASSWORD, &encrypted)
            .expect("TO have decrypted the data");

        assert_eq!(DATA, decrypted.as_slice());
        assert!(decrypt(WRONG_PASSWORD, &encrypted).is_none());
    }
}
