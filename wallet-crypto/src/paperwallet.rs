extern crate rcw;
use self::rcw::digest::Digest;
use self::rcw::sha2::Sha512;
use self::rcw::hmac::Hmac;
use self::rcw::pbkdf2::{pbkdf2};

const ITERS : u32 = 10000;
const CONST : &str = "IOHK";

fn gen(iv: &[u8], password: &[u8], buf: &mut [u8]) {
    assert!(iv.len() == 4);
    let mut salt = [0u8;8];
    salt[0..4].clone_from_slice(iv);
    salt[4..8].clone_from_slice(CONST.as_bytes());
    let mut mac = Hmac::new(Sha512::new(), password);
    pbkdf2(&mut mac, &salt[..], ITERS, buf);
}

pub fn scramble(iv: &[u8], password: &[u8], input: &[u8]) -> Vec<u8> {
    assert!(iv.len() == 4);
    let sz = 4 + input.len();
    let mut out = Vec::with_capacity(sz);

    out.extend_from_slice(iv);
    gen(iv, password, &mut out[4..sz]);

    for i in 4..sz {
        out[i] = out[i] ^ input[i];
    }
    out
}

pub fn unscramble(password: &[u8], input: &[u8]) -> Vec<u8>{
    assert!(input.len() > 4);

    let out_sz = input.len() - 4;

    let mut out = Vec::with_capacity(out_sz);

    gen(&input[0..4], password, &mut out[0..out_sz]);
    for i in 0..out_sz {
        out[i] = out[i] ^ input[4+i];
    }
    out
}
