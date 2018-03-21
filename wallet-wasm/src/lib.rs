extern crate rcw;
extern crate wallet_crypto;

use self::rcw::hmac::{Hmac};
use self::rcw::sha2::{Sha256};
use self::rcw::pbkdf2::{pbkdf2};
use self::rcw::blake2b::{Blake2b};
use self::rcw::digest::{Digest};

use self::wallet_crypto::hdwallet;
use self::wallet_crypto::paperwallet;
use self::wallet_crypto::address;
use self::wallet_crypto::hdpayload;

use std::mem;
use std::ffi::{CStr, CString};
use std::os::raw::{c_uint, c_uchar, c_char, c_void};
use std::iter::repeat;
//use std::slice::{from_raw_parts};

// In order to work with the memory we expose (de)allocation methods
#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut c_void {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    return ptr as *mut c_void;
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut c_void, cap: usize) {
    unsafe  {
        let _buf = Vec::from_raw_parts(ptr, 0, cap);
    }
}

#[no_mangle]
pub extern "C" fn dealloc_str(ptr: *mut c_char) {
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

#[no_mangle]
pub extern "C" fn pbkdf2_sha256(password: *mut c_char, salt: *mut c_char, iters: u32, output: u32) -> *mut c_char {
    unsafe {

        let salt = CStr::from_ptr(salt);
        let password = CStr::from_ptr(password);

        let salt = salt.to_bytes();
        let password = password.to_bytes();

        let mut mac = Hmac::new(Sha256::new(), &password[..]);
        let mut result: Vec<u8> = repeat(0).take(output as usize).collect();
        pbkdf2(&mut mac, &salt[..], iters, &mut result);
        let s = CString::new(result).unwrap();
        s.into_raw()
    }
}

unsafe fn read_data(data_ptr: *const c_uchar, sz: usize) -> Vec<u8> {
        let data_slice = std::slice::from_raw_parts(data_ptr, sz);
        let mut data = Vec::with_capacity(sz);
        data.extend_from_slice(data_slice);
        data
}

unsafe fn write_data(data: &[u8], data_ptr: *mut c_uchar) {
        let sz = data.len();
        let out = std::slice::from_raw_parts_mut(data_ptr, sz);
        out[0..sz].clone_from_slice(data)
}

unsafe fn read_data_u32(data_ptr: *const c_uint, sz: usize) -> Vec<u32> {
    let data_slice = std::slice::from_raw_parts(data_ptr, sz);
    let mut data = Vec::with_capacity(sz);
    data.extend_from_slice(data_slice);
    data
}

unsafe fn write_data_u32(data: &[u32], data_ptr: *mut c_uint) {
        let sz = data.len();
        let out = std::slice::from_raw_parts_mut(data_ptr, sz);
        out[0..sz].clone_from_slice(data)
}

unsafe fn read_xprv(xprv_ptr: *const c_uchar) -> hdwallet::XPrv {
        let xprv_slice = std::slice::from_raw_parts(xprv_ptr, hdwallet::XPRV_SIZE);
        let mut xprv : hdwallet::XPrv = [0u8;hdwallet::XPRV_SIZE];
        xprv.clone_from_slice(xprv_slice);
        xprv
}

unsafe fn write_xprv(xprv: &hdwallet::XPrv, xprv_ptr: *mut c_uchar) {
        let out = std::slice::from_raw_parts_mut(xprv_ptr, hdwallet::XPRV_SIZE);
        out[0..hdwallet::XPRV_SIZE].clone_from_slice(xprv);
}

unsafe fn read_xpub(xpub_ptr: *const c_uchar) -> hdwallet::XPub {
        let xpub_slice = std::slice::from_raw_parts(xpub_ptr, hdwallet::XPUB_SIZE);
        let mut xpub : hdwallet::XPub = [0u8;hdwallet::XPUB_SIZE];
        xpub.clone_from_slice(xpub_slice);
        xpub
}

unsafe fn write_xpub(xpub: &hdwallet::XPub, xpub_ptr: *mut c_uchar) {
        let out = std::slice::from_raw_parts_mut(xpub_ptr, hdwallet::XPUB_SIZE);
        out[0..hdwallet::XPUB_SIZE].clone_from_slice(xpub);
}

unsafe fn read_signature(sig_ptr: *const c_uchar) -> hdwallet::Signature {
        let signature_slice = std::slice::from_raw_parts(sig_ptr, hdwallet::SIGNATURE_SIZE);
        let mut signature : hdwallet::XPub = [0u8;hdwallet::SIGNATURE_SIZE];
        signature.clone_from_slice(signature_slice);
        signature
}

unsafe fn write_signature(signature: &[u8], out_ptr: *mut c_uchar) {
        let out = std::slice::from_raw_parts_mut(out_ptr, hdwallet::SIGNATURE_SIZE);
        out[0..hdwallet::SIGNATURE_SIZE].clone_from_slice(signature);
}

unsafe fn read_seed(seed_ptr: *const c_uchar) -> hdwallet::Seed {
        let seed_slice = std::slice::from_raw_parts(seed_ptr, hdwallet::SEED_SIZE);
        let mut seed : hdwallet::Seed = [0u8;hdwallet::SEED_SIZE];
        seed.clone_from_slice(seed_slice);
        seed
}

#[no_mangle]
pub extern "C" fn wallet_from_seed(seed_ptr: *const c_uchar, out: *mut c_uchar) {
    let seed = unsafe { read_seed(seed_ptr) };
    let xprv = hdwallet::generate(&seed);
    unsafe { write_xprv(&xprv, out) }
}

#[no_mangle]
pub extern "C" fn wallet_to_public(xprv_ptr: *const c_uchar, out: *mut c_uchar) {
    let xprv = unsafe { read_xprv(xprv_ptr) };
    let xpub = hdwallet::to_public(&xprv);
    unsafe { write_xpub(&xpub, out) }
}

#[no_mangle]
pub extern "C" fn wallet_derive_private(xprv_ptr: *const c_uchar, index: u32, out: *mut c_uchar) {
    let xprv = unsafe { read_xprv(xprv_ptr) };
    let child = hdwallet::derive_private(&xprv, index);
    unsafe { write_xprv(&child, out) }
}

#[no_mangle]
pub extern "C" fn wallet_derive_public(xpub_ptr: *const c_uchar, index: u32, out: *mut c_uchar) -> bool {
    let xpub = unsafe { read_xpub(xpub_ptr) };
    match hdwallet::derive_public(&xpub, index) {
        Ok(child) => { unsafe { write_xpub(&child, out) }; true }
        Err(_)    => { false }
    }
}

#[no_mangle]
pub extern "C" fn wallet_sign(xprv_ptr: *const c_uchar, msg_ptr: *const c_uchar, msg_sz: usize, out: *mut c_uchar) {
    let xprv = unsafe { read_xprv(xprv_ptr) };
    let msg = unsafe { read_data(msg_ptr, msg_sz) };
    let signature = hdwallet::sign(&xprv, &msg[..]);
    unsafe { write_signature(&signature, out) }
}

#[no_mangle]
pub extern "C" fn wallet_verify(xpub_ptr: *const c_uchar, msg_ptr: *const c_uchar, msg_sz: usize, sig_ptr: *const c_uchar) -> bool {
    let xpub = unsafe { read_xpub(xpub_ptr) };
    let msg = unsafe { read_data(msg_ptr, msg_sz) };
    let signature = unsafe { read_signature(sig_ptr) };
    hdwallet::verify(&xpub, &msg, &signature)
}

#[no_mangle]
pub extern "C" fn paper_scramble(iv_ptr: *const c_uchar, pass_ptr: *const c_uchar, pass_sz: usize, input_ptr: *const c_uchar, input_sz: usize, out: *mut c_uchar) {
    let iv = unsafe { read_data(iv_ptr, 4) };
    let pass = unsafe { read_data(pass_ptr, pass_sz) };
    let input = unsafe { read_data(input_ptr, input_sz) };
    let output = paperwallet::scramble(&iv[..], &pass[..], &input[..]);
    unsafe { write_data(&output[..], out) }
}

#[no_mangle]
pub extern "C" fn paper_unscramble(pass_ptr: *const c_uchar, pass_sz: usize, input_ptr: *const c_uchar, input_sz: usize, out: *mut c_uchar) {
    let pass = unsafe { read_data(pass_ptr, pass_sz) };
    let input = unsafe { read_data(input_ptr, input_sz) };
    let output = paperwallet::unscramble(&pass[..], &input[..]);
    unsafe { write_data(&output[..], out) }
}

#[no_mangle]
pub extern "C" fn blake2b_256(msg_ptr: *const c_uchar, msg_sz: usize, out: *mut c_uchar) {
    let mut b2b = Blake2b::new(32);
    let mut outv = [0;32];
    let msg = unsafe { read_data(msg_ptr, msg_sz) };
    b2b.input(&msg);
    b2b.result(&mut outv);
    unsafe { write_data(&outv, out) }
}

#[no_mangle]
pub extern "C" fn wallet_public_to_address(xpub_ptr: *const c_uchar, payload_ptr: *const c_uchar, payload_sz: usize, out: *mut c_uchar) -> u32 {
    let xpub = unsafe { read_xpub(xpub_ptr) };
    let payload = unsafe { read_data(payload_ptr, payload_sz) };

    let hdap = address::HDAddressPayload::new(&payload);

    let addr_type = address::AddrType::ATPubKey;
    let sd = address::SpendingData::PubKeyASD(xpub.clone());
    let attrs = address::Attributes::new_single_key(&xpub, Some(hdap));
    let ea = address::ExtendedAddr::new(addr_type, sd, attrs);

    let ea_bytes = ea.to_bytes();

    unsafe { write_data(&ea_bytes, out) }

    return ea_bytes.len() as u32;
}

#[no_mangle]
pub extern "C" fn wallet_address_get_payload(addr_ptr: *const c_uchar, addr_sz: usize, out: *mut c_uchar) -> u32 {
    let addr_bytes = unsafe { read_data(addr_ptr, addr_sz) };
    match address::ExtendedAddr::from_bytes(&addr_bytes) {
        Err(_) => (-1i32) as u32,
        Ok(r)  => {
            match r.attributes.derivation_path {
                None        => 0,
                Some(dpath) => {
                    unsafe { write_data(dpath.as_ref(), out) };
                    dpath.as_ref().len() as u32
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn wallet_payload_initiate(xpub_ptr: *const c_uchar, out: *mut c_uchar) {
    let xpub = unsafe { read_xpub(xpub_ptr) };
    let hdkey = hdpayload::HDKey::new(&xpub);
    unsafe { write_data(hdkey.as_ref(), out); }
}

#[no_mangle]
pub extern "C" fn wallet_payload_encrypt(key_ptr: *const c_uchar, path_array: *const c_uint, path_sz: usize, out: *mut c_uchar) -> u32 {
    let key_bytes = unsafe { read_data(key_ptr, hdpayload::HDKEY_SIZE) };
    let path_vec = unsafe { read_data_u32(path_array, path_sz) };
    let hdkey = hdpayload::HDKey::from_slice(&key_bytes).unwrap();

    let path = hdpayload::Path::new(path_vec);

    let payload = hdkey.encrypt_path(&path);

    unsafe { write_data(payload.as_ref(), out) };
    payload.len() as u32
}

#[no_mangle]
pub extern "C" fn wallet_payload_decrypt(key_ptr: *const c_uchar, payload_ptr: *const c_uchar, payload_sz: usize, out: *mut c_uint) -> u32 {
    let key_bytes = unsafe { read_data(key_ptr, hdpayload::HDKEY_SIZE) };
    let payload_bytes = unsafe { read_data(payload_ptr, payload_sz) };

    let hdkey = hdpayload::HDKey::from_slice(&key_bytes).unwrap();
    let payload = hdpayload::HDAddressPayload::from_bytes(&payload_bytes);

    match hdkey.decrypt_path(&payload) {
        None       => 0xffffffff,
        Some(path) => {
            let v = path.as_ref();
            unsafe { write_data_u32(v, out) };
            v.len() as u32
        }
    }
}
