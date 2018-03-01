extern crate rcw;
extern crate wallet_crypto;

use self::rcw::hmac::{Hmac};
use self::rcw::sha2::{Sha256, Sha512};
use self::rcw::pbkdf2::{pbkdf2};

use self::wallet_crypto::hdwallet;

use std::mem;
use std::ffi::{CStr, CString};
use std::os::raw::{c_uchar, c_char, c_void};
use std::iter::repeat;
//use std::slice::{from_raw_parts};

use hdwallet::{generate};

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

unsafe fn write_signature(signature: &[u8], out_ptr: *mut c_uchar) {
        let out = std::slice::from_raw_parts_mut(out_ptr, 64);
        out[0..64].clone_from_slice(signature);
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
pub extern "C" fn wallet_verify(xpub_ptr: *const c_uchar, msg_ptr: *const c_uchar, out: *mut c_uchar) {
    let xpub = unsafe { read_xprv(xpub_ptr) };
}
