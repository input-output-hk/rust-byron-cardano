extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
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
use self::wallet_crypto::tx;
use self::wallet_crypto::tx::fee::Algorithm;
use self::wallet_crypto::config::{Config};

use self::wallet_crypto::cbor;
use self::wallet_crypto::cbor::{encode_to_cbor, decode_from_cbor};

use std::{mem, result, string, convert};
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
        hdwallet::XPrv::from_slice(xprv_slice).unwrap()
}

unsafe fn write_xprv(xprv: &hdwallet::XPrv, xprv_ptr: *mut c_uchar) {
        let out = std::slice::from_raw_parts_mut(xprv_ptr, hdwallet::XPRV_SIZE);
        out[0..hdwallet::XPRV_SIZE].clone_from_slice(xprv.as_ref());
}

unsafe fn read_xpub(xpub_ptr: *const c_uchar) -> hdwallet::XPub {
        let xpub_slice = std::slice::from_raw_parts(xpub_ptr, hdwallet::XPUB_SIZE);
        hdwallet::XPub::from_slice(xpub_slice).unwrap()
}

unsafe fn write_xpub(xpub: &hdwallet::XPub, xpub_ptr: *mut c_uchar) {
        let out = std::slice::from_raw_parts_mut(xpub_ptr, hdwallet::XPUB_SIZE);
        out[0..hdwallet::XPUB_SIZE].clone_from_slice(xpub.as_ref());
}

unsafe fn read_signature<T>(sig_ptr: *const c_uchar) -> hdwallet::Signature<T> {
        let signature_slice = std::slice::from_raw_parts(sig_ptr, hdwallet::SIGNATURE_SIZE);
        hdwallet::Signature::from_slice(signature_slice).unwrap()
}

unsafe fn write_signature<T>(signature: &hdwallet::Signature<T>, out_ptr: *mut c_uchar) {
        let out = std::slice::from_raw_parts_mut(out_ptr, hdwallet::SIGNATURE_SIZE);
        out[0..hdwallet::SIGNATURE_SIZE].clone_from_slice(signature.as_ref());
}

unsafe fn read_seed(seed_ptr: *const c_uchar) -> hdwallet::Seed {
        let seed_slice = std::slice::from_raw_parts(seed_ptr, hdwallet::SEED_SIZE);
        hdwallet::Seed::from_slice(seed_slice).unwrap()
}

#[no_mangle]
pub extern "C" fn wallet_from_seed(seed_ptr: *const c_uchar, out: *mut c_uchar) {
    let seed = unsafe { read_seed(seed_ptr) };
    let xprv = hdwallet::XPrv::generate_from_seed(&seed);
    unsafe { write_xprv(&xprv, out) }
}

#[no_mangle]
pub extern "C" fn wallet_to_public(xprv_ptr: *const c_uchar, out: *mut c_uchar) {
    let xprv = unsafe { read_xprv(xprv_ptr) };
    let xpub = xprv.public();
    unsafe { write_xpub(&xpub, out) }
}

#[no_mangle]
pub extern "C" fn wallet_derive_private(xprv_ptr: *const c_uchar, index: u32, out: *mut c_uchar) {
    let xprv = unsafe { read_xprv(xprv_ptr) };
    let child = xprv.derive(index);
    unsafe { write_xprv(&child, out) }
}

#[no_mangle]
pub extern "C" fn wallet_derive_public(xpub_ptr: *const c_uchar, index: u32, out: *mut c_uchar) -> bool {
    let xpub = unsafe { read_xpub(xpub_ptr) };
    match xpub.derive(index) {
        Ok(child) => { unsafe { write_xpub(&child, out) }; true }
        Err(_)    => { false }
    }
}

#[no_mangle]
pub extern "C" fn wallet_sign(xprv_ptr: *const c_uchar, msg_ptr: *const c_uchar, msg_sz: usize, out: *mut c_uchar) {
    let xprv = unsafe { read_xprv(xprv_ptr) };
    let msg = unsafe { read_data(msg_ptr, msg_sz) };
    let signature : hdwallet::Signature<Vec<u8>> = xprv.sign(&msg[..]);
    unsafe { write_signature(&signature, out) }
}

#[no_mangle]
pub extern "C" fn wallet_verify(xpub_ptr: *const c_uchar, msg_ptr: *const c_uchar, msg_sz: usize, sig_ptr: *const c_uchar) -> bool {
    let xpub = unsafe { read_xpub(xpub_ptr) };
    let msg = unsafe { read_data(msg_ptr, msg_sz) };
    let signature = unsafe { read_signature::<Vec<u8>>(sig_ptr) };
    xpub.verify(&msg, &signature)
}

#[no_mangle]
pub extern "C" fn paper_scramble(iv_ptr: *const c_uchar, pass_ptr: *const c_uchar, pass_sz: usize, input_ptr: *const c_uchar, input_sz: usize, out: *mut c_uchar) {
    let iv = unsafe { read_data(iv_ptr, paperwallet::IV_SIZE) };
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

    let hdap = hdpayload::HDAddressPayload::from_vec(payload);

    let addr_type = address::AddrType::ATPubKey;
    let sd = address::SpendingData::PubKeyASD(xpub.clone());
    let attrs = address::Attributes::new_bootstrap_era(Some(hdap));
    let ea = address::ExtendedAddr::new(addr_type, sd, attrs);

    let ea_bytes = ea.to_bytes();

    unsafe { write_data(&ea_bytes, out) }

    return ea_bytes.len() as u32;
}

#[no_mangle]
pub extern "C" fn wallet_address_get_payload(addr_ptr: *const c_uchar, addr_sz: usize, out: *mut c_uchar) -> u32 {
    let addr_bytes = unsafe { read_data(addr_ptr, addr_sz) };
    match address::ExtendedAddr::from_bytes(&addr_bytes).ok() {
        None => (-1i32) as u32,
        Some(r)  => {
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

#[no_mangle]
pub extern "C" fn wallet_txin_create(txid_ptr: *const c_uchar, index: u32, out: *mut c_uchar) -> u32 {
    let txid_bytes = unsafe { read_data(txid_ptr, tx::HASH_SIZE) };

    let txid = tx::TxId::from_slice(&txid_bytes).unwrap();

    let txin = tx::TxIn::new(txid, index);
    let out_buf = encode_to_cbor(&txin).unwrap();

    unsafe { write_data(&out_buf, out) }
    out_buf.len() as u32
}

#[no_mangle]
pub extern "C" fn wallet_txout_create(ea_ptr: *const c_uchar, ea_sz: usize, amount: u32, out: *mut c_uchar) -> u32 {
    let ea_bytes = unsafe { read_data(ea_ptr, ea_sz) };

    let ea = address::ExtendedAddr::from_bytes(&ea_bytes).unwrap();
    let coin = tx::Coin::new(amount as u64).unwrap();

    let txout = tx::TxOut::new(ea, coin);
    let out_buf = encode_to_cbor(&txout).unwrap();

    unsafe { write_data(&out_buf, out) }
    out_buf.len() as u32
}

#[no_mangle]
pub extern "C" fn wallet_tx_new(out: *mut c_uchar) -> u32 {
    let tx = tx::Tx::new();
    let out_buf = encode_to_cbor(&tx).unwrap();
    unsafe { write_data(&out_buf, out) }
    out_buf.len() as u32
}

#[no_mangle]
pub extern "C" fn wallet_tx_add_txin(tx_ptr: *const c_uchar, tx_sz: usize, txin_ptr: *const c_uchar, txin_sz: usize, out: *mut c_uchar) -> u32 {
    let tx_bytes = unsafe { read_data(tx_ptr, tx_sz) };
    let txin_bytes = unsafe { read_data(txin_ptr, txin_sz) };

    let mut tx : tx::Tx = decode_from_cbor(&tx_bytes).unwrap();
    let txin = decode_from_cbor(&txin_bytes).unwrap();

    tx.add_input(txin);

    let out_buf = encode_to_cbor(&tx).unwrap();
    unsafe { write_data(&out_buf, out) }
    out_buf.len() as u32
}

#[no_mangle]
pub extern "C" fn wallet_tx_add_txout(tx_ptr: *const c_uchar, tx_sz: usize, txout_ptr: *const c_uchar, txout_sz: usize, out: *mut c_uchar) -> u32 {
    let tx_bytes = unsafe { read_data(tx_ptr, tx_sz) };
    let txout_bytes = unsafe { read_data(txout_ptr, txout_sz) };

    let mut tx : tx::Tx = decode_from_cbor(&tx_bytes).unwrap();
    let txout = decode_from_cbor(&txout_bytes).unwrap();

    tx.add_output(txout);

    let out_buf = encode_to_cbor(&tx).unwrap();
    unsafe { write_data(&out_buf, out) }
    out_buf.len() as u32
}

#[no_mangle]
pub extern "C" fn wallet_tx_sign(cfg_ptr: *const c_uchar, cfg_size: usize, xprv_ptr: *const c_uchar, tx_ptr: *const c_uchar, tx_sz: usize, out: *mut c_uchar) {
    let cfg_bytes : Vec<u8> = unsafe { read_data(cfg_ptr, cfg_size) };
    let cfg_str = String::from_utf8(cfg_bytes).unwrap();
    let cfg : Config = serde_json::from_str(cfg_str.as_str()).unwrap();
    let xprv = unsafe { read_xprv(xprv_ptr) };
    let tx_bytes = unsafe { read_data(tx_ptr, tx_sz) };

    let tx = decode_from_cbor(&tx_bytes).unwrap();

    let txinwitness = tx::TxInWitness::new(&cfg, &xprv, &tx);

    let signature = match txinwitness {
        tx::TxInWitness::PkWitness(_, sig) => sig,
        _ => unimplemented!() // this should never happen as we are signing for the tx anyway
    };
    unsafe { write_signature(&signature, out) }
}

#[no_mangle]
pub extern "C" fn wallet_tx_verify(cfg_ptr: *const c_uchar, cfg_size: usize, xpub_ptr: *const c_uchar, tx_ptr: *const c_uchar, tx_sz: usize, sig_ptr: *const c_uchar) -> i32 {
    let cfg_bytes : Vec<u8> = unsafe { read_data(cfg_ptr, cfg_size) };
    let cfg_str = String::from_utf8(cfg_bytes).unwrap();
    let cfg : Config = serde_json::from_str(cfg_str.as_str()).unwrap();
    let xpub = unsafe { read_xpub(xpub_ptr) };
    let signature = unsafe { read_signature(sig_ptr) };

    let tx_bytes = unsafe { read_data(tx_ptr, tx_sz) };
    let tx = decode_from_cbor(&tx_bytes).unwrap();

    let txinwitness = tx::TxInWitness::PkWitness(xpub, signature);

    if txinwitness.verify_tx(&cfg, &tx) { 0 } else { -1 }
}

mod jrpc {
    use serde::{Serialize};
    use serde_json;
    use std::os::raw::{c_uchar};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    struct Error {
        failed: bool,
        loc: String,
        msg: String
    }
    impl Error {
        fn new(loc: String, msg: String) -> Self { Error { failed: true, loc: loc, msg: msg } }
    }

    pub fn fail(output_ptr: *mut c_uchar, file: &str, line: u32, msg: String) -> i32 {
        let error = Error::new(format!("{} {}", file, line), msg);

        let output = serde_json::to_string(&error).unwrap();
        let output_bytes = output.into_bytes();

        unsafe { super::write_data(&output_bytes, output_ptr) };
        output_bytes.len() as i32
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    struct Success<T> {
        failed: bool,
        result: T
    }
    impl<T: Serialize> Success<T> {
        fn new(result: T) -> Self { Success { failed: false, result: result } }
    }

    pub fn ok<T>(output_ptr: *mut c_uchar, result: T) -> i32
        where T: Serialize
    {
        let succ = Success::new(result);

        let output = serde_json::to_string(&succ).unwrap();
        let output_bytes = output.into_bytes();

        unsafe { super::write_data(&output_bytes, output_ptr) };
        output_bytes.len() as i32
    }
}

/// Entry point of jrpc error reporting
macro_rules! jrpc_fail {
    ($output_ptr:ident) => (
        jrpc_fail!($output_ptr, "unknown error")
    );
    ($output_ptr:ident, ) => (
        jrpc_fail!($output_ptr)
    );
    ($output_ptr:ident, $msg:expr) => ({
        jrpc::fail($output_ptr, file!(), line!(), $msg)
    });
    ($output_ptr:ident, $msg:expr, ) => ({
        jrpc_fail!($output_ptr, $msg)
    });
    ($output_ptr:ident, $fmt:expr, $($arg:tt)+) => ({
        jrpc::fail($output_ptr, file!(), line!(), format!($fmt, $($arg)*))
    });
}

macro_rules! jrpc_ok {
    ($output_ptr:ident, $result:expr) => ({
        jrpc::ok($output_ptr, $result)
    });
    ($output_ptr:ident, $result:expr,) => ({
        jrpc_ok!($output_ptr, $result)
    });
}

macro_rules! jrpc_try {
    ($output_ptr:ident, $expr:expr) => (match $expr {
        Ok(val) => val,
        Err(err) => { return jrpc_fail!($output_ptr, "{:?}", err); }
    });
    ($output_ptr:ident, $expr:expr,) => (jrpc_try!($output_ptr, $expr));
}

#[derive(Debug)]
enum Error {
    ErrorUtf8(string::FromUtf8Error),
    ErrorJSON(serde_json::error::Error),
    ErrorCBOR(cbor::Error),
    ErrorFEE(tx::fee::Error),
}
impl convert::From<string::FromUtf8Error> for Error {
    fn from(j: string::FromUtf8Error) -> Self { Error::ErrorUtf8(j) }
}
impl convert::From<serde_json::error::Error> for Error {
    fn from(j: serde_json::error::Error) -> Self { Error::ErrorJSON(j) }
}
impl convert::From<cbor::Error> for Error {
    fn from(j: cbor::Error) -> Self { Error::ErrorCBOR(j) }
}
impl convert::From<tx::fee::Error> for Error {
    fn from(j: tx::fee::Error) -> Self { Error::ErrorFEE(j) }
}

type Result<T> = result::Result<T, Error>;

fn input_string_(input_ptr: *const c_uchar, input_sz: usize) -> Result<String> {
    let input_bytes : Vec<u8> = unsafe { read_data(input_ptr, input_sz) };
    let input = String::from_utf8(input_bytes)?;

    Ok(input)
}

macro_rules! input_json {
    ($output_ptr:ident, $input_ptr:ident, $input_sz:ident) => ({
        let input = jrpc_try!($output_ptr, input_string_($input_ptr, $input_sz));
        jrpc_try!($output_ptr, serde_json::from_str(input.as_str()))
    });
    ($output_ptr:ident, $input_ptr:ident, $input_sz:ident,) => ({
        input_json!($output_ptr, $input_ptr, $input_sz)
    });
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct FeeStabilisationInput {
    inputs: tx::Inputs,
    outputs: tx::Outputs,
    selection_policy: tx::fee::SelectionPolicy,
    change_addr: address::ExtendedAddr,
    fee_addr: address::ExtendedAddr,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct FeeStabilisationOutput {
    inputs: tx::Inputs,
    fee:    tx::fee::Fee,
}

#[no_mangle]
pub extern "C" fn wallet_filter_utxos(input_ptr: *const c_uchar, input_sz: usize, output_ptr: *mut c_uchar) -> i32 {
    let input : FeeStabilisationInput = input_json!(output_ptr, input_ptr, input_sz);

    let algo = tx::fee::LinearFee::default();
    let (fee, selection, _) = jrpc_try!(
        output_ptr,
        algo.compute(input.selection_policy, &input.inputs, &input.outputs, &input.change_addr, &input.fee_addr)
    );

    let output = FeeStabilisationOutput { fee: fee, inputs: selection };
    jrpc_ok!(output_ptr, output)
}
