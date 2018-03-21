extern crate rcw;

use self::rcw::digest::Digest;
use self::rcw::sha2::Sha512;
use self::rcw::hmac::Hmac;
use self::rcw::mac::Mac;
use self::rcw::curve25519::{GeP3, ge_scalarmult_base};
use self::rcw::ed25519::signature_extended;
use self::rcw::ed25519;

pub const SEED_SIZE: usize = 32;
pub const XPRV_SIZE: usize = 96;
pub const XPUB_SIZE: usize = 64;
pub const SIGNATURE_SIZE: usize = 64;

pub const PUBLIC_KEY_SIZE: usize = 32;
pub const CHAIN_CODE_SIZE: usize = 32;

pub type Seed = [u8; SEED_SIZE];
pub type XPrv = [u8; XPRV_SIZE];
pub type XPub = [u8; XPUB_SIZE];
pub type Signature = [u8; SIGNATURE_SIZE];

pub type ChainCode = [u8; CHAIN_CODE_SIZE];

type DerivationIndex = u32;

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

pub fn generate(seed: &Seed) -> XPrv {
    let mut mac = Hmac::new(Sha512::new(), &seed[..]);

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
    out
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

pub fn derive_private(xprv: &XPrv, index: DerivationIndex) -> XPrv {
    /*
     * If so (hardened child):
     *    let Z = HMAC-SHA512(Key = cpar, Data = 0x00 || ser256(left(kpar)) || ser32(i)).
     *    let I = HMAC-SHA512(Key = cpar, Data = 0x01 || ser256(left(kpar)) || ser32(i)).
     * If not (normal child):
     *    let Z = HMAC-SHA512(Key = cpar, Data = 0x02 || serP(point(kpar)) || ser32(i)).
     *    let I = HMAC-SHA512(Key = cpar, Data = 0x03 || serP(point(kpar)) || ser32(i)).
     **/

    let ekey = &xprv[0..64];
    let kl = &ekey[0..32];
    let kr = &ekey[32..64];
    let chaincode = &xprv[64..96];

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

    out
}

fn point_of_trunc28_mul8(sk: &[u8]) -> GeP3 {
    assert!(sk.len() == 32);
    let a = ge_scalarmult_base(sk);
    a
}

pub fn derive_public(xpub: &XPub, index: DerivationIndex) -> Result<XPub, ()> {
    let pk = &xpub[0..32];
    let chaincode = &xpub[32..64];

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
            return Err(());
        }
    };

    let mut zout = [0u8; 64];
    zmac.raw_result(&mut zout);
    let zl = &zout[0..32];
    let _zr = &zout[32..64];

    let a = match GeP3::from_bytes_negate_vartime(pk) {
        Some(g) => g,
        None => {
            return Err(());
        }
    };

    // left = kl + 8 * trunc28(zl)
    let left = a + point_of_trunc28_mul8(zl).to_cached();

    let mut iout = [0u8; 64];
    imac.raw_result(&mut iout);
    let cc = &iout[32..];

    let mut out = [0u8; XPUB_SIZE];
    mk_xpub(&mut out, &left.to_p2().to_bytes(), cc);

    imac.reset();
    zmac.reset();

    Ok(out)

}

pub fn sign(xprv: &XPrv, message: &[u8]) -> Signature {
    signature_extended(message, &xprv[0..64])
}

pub fn verify(xpub: &XPub, message: &[u8], signature: &Signature) -> bool {
    ed25519::verify(message, &xpub[0..32], &signature[..])
}

fn mk_public_key(extended_secret: &[u8]) -> [u8; PUBLIC_KEY_SIZE] {
    assert!(extended_secret.len() == 64);
    let a = ge_scalarmult_base(&extended_secret[0..32]);
    a.to_bytes()
}

pub fn to_public(xprv: &XPrv) -> XPub {
    let pk = mk_public_key(&xprv[0..64]);
    let mut out = [0u8; XPUB_SIZE];
    out[0..32].clone_from_slice(&pk);
    out[32..64].clone_from_slice(&xprv[64..]);
    out
}



#[cfg(test)]
mod tests {
    use hdwallet::{XPub, XPrv, DerivationIndex, generate, derive_public, derive_private, sign};

    const D1: XPrv =
        [0xf8, 0xa2, 0x92, 0x31, 0xee, 0x38, 0xd6, 0xc5, 0xbf, 0x71, 0x5d, 0x5b, 0xac, 0x21, 0xc7,
         0x50, 0x57, 0x7a, 0xa3, 0x79, 0x8b, 0x22, 0xd7, 0x9d, 0x65, 0xbf, 0x97, 0xd6, 0xfa, 0xde,
         0xa1, 0x5a, 0xdc, 0xd1, 0xee, 0x1a, 0xbd, 0xf7, 0x8b, 0xd4, 0xbe, 0x64, 0x73, 0x1a, 0x12,
         0xde, 0xb9, 0x4d, 0x36, 0x71, 0x78, 0x41, 0x12, 0xeb, 0x6f, 0x36, 0x4b, 0x87, 0x18, 0x51,
         0xfd, 0x1c, 0x9a, 0x24, 0x73, 0x84, 0xdb, 0x9a, 0xd6, 0x00, 0x3b, 0xbd, 0x08, 0xb3, 0xb1,
         0xdd, 0xc0, 0xd0, 0x7a, 0x59, 0x72, 0x93, 0xff, 0x85, 0xe9, 0x61, 0xbf, 0x25, 0x2b, 0x33,
         0x12, 0x62, 0xed, 0xdf, 0xad, 0x0d];

    const D1_H0: XPrv =
        [0x60, 0xd3, 0x99, 0xda, 0x83, 0xef, 0x80, 0xd8, 0xd4, 0xf8, 0xd2, 0x23, 0x23, 0x9e, 0xfd,
         0xc2, 0xb8, 0xfe, 0xf3, 0x87, 0xe1, 0xb5, 0x21, 0x91, 0x37, 0xff, 0xb4, 0xe8, 0xfb, 0xde,
         0xa1, 0x5a, 0xdc, 0x93, 0x66, 0xb7, 0xd0, 0x03, 0xaf, 0x37, 0xc1, 0x13, 0x96, 0xde, 0x9a,
         0x83, 0x73, 0x4e, 0x30, 0xe0, 0x5e, 0x85, 0x1e, 0xfa, 0x32, 0x74, 0x5c, 0x9c, 0xd7, 0xb4,
         0x27, 0x12, 0xc8, 0x90, 0x60, 0x87, 0x63, 0x77, 0x0e, 0xdd, 0xf7, 0x72, 0x48, 0xab, 0x65,
         0x29, 0x84, 0xb2, 0x1b, 0x84, 0x97, 0x60, 0xd1, 0xda, 0x74, 0xa6, 0xf5, 0xbd, 0x63, 0x3c,
         0xe4, 0x1a, 0xdc, 0xee, 0xf0, 0x7a];

    const MSG: &str = "Hello World";

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

    fn seed_xprv_eq(seed: [u8; 32], expected_xprv: [u8; 96]) {
        let xprv = generate(&seed);
        compare_xprv(&xprv, &expected_xprv);
    }

    #[test]
    fn seed_cases() {
        seed_xprv_eq([0xe3, 0x55, 0x24, 0xa5, 0x18, 0x03, 0x4d, 0xdc, 0x11, 0x92, 0xe1, 0xda,
                      0xcd, 0x32, 0xc1, 0xed, 0x3e, 0xaa, 0x3c, 0x3b, 0x13, 0x1c, 0x88, 0xed,
                      0x8e, 0x7e, 0x54, 0xc4, 0x9a, 0x5d, 0x09, 0x98],
                     D1)
    }

    fn derive_xprv_eq(parent_xprv: [u8; 96], idx: DerivationIndex, expected_xprv: [u8; 96]) {
        let child_xprv: [u8; 96] = derive_private(&parent_xprv, idx);
        compare_xprv(&child_xprv, &expected_xprv);
    }

    #[test]
    fn xprv_derive() {
        derive_xprv_eq(D1, 0x80000000, D1_H0);
    }

    fn do_sign(xprv: [u8; 96], expected_signature: &[u8]) {
        let signature = sign(&xprv, MSG.as_bytes());
        assert_eq!(&signature[..], expected_signature);
    }

    #[test]
    fn xprv_sign() {
        do_sign(D1_H0, &D1_H0_SIGNATURE);
    }
}
