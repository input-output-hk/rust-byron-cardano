use std::os::raw::{c_char, c_int};
use std::{ffi, ptr};

use cardano::{
    address::ExtendedAddr,
    config::ProtocolMagic,
    util::{base58, try_from_slice::TryFromSlice},
};

use super::{AddressPtr, XPubPtr};

// FFI helper internal call
pub fn ffi_address_to_base58(address: &ExtendedAddr) -> ffi::CString {
    let address = format!("{}", address);
    // generate a C String (null byte terminated string)
    let c_address = ffi::CString::new(address).expect("base58 strings only contains ASCII chars");
    c_address
}

/// Take a string as parameter and returns whether or not it's a valid base58 address
///
/// On valid address, the return value is 0
/// On invalid address, the return value is different from 0.
///
/// Invalid cases returns different code depending on the issue
///
#[no_mangle]
pub extern "C" fn cardano_address_is_valid(c_address: *mut c_char) -> c_int {
    let address_base58 = unsafe { ffi::CStr::from_ptr(c_address).to_bytes() };
    if let Ok(address_raw) = base58::decode_bytes(address_base58) {
        if let Ok(_) = ExtendedAddr::try_from_slice(&address_raw[..]) {
            return 0;
        } else {
            return 2;
        }
    } else {
        return 1;
    }
}

#[no_mangle]
pub extern "C" fn cardano_address_new_from_pubkey(
    c_xpubkey: XPubPtr,
    protocol_magic: ProtocolMagic,
) -> AddressPtr {
    let xpub = unsafe { c_xpubkey.as_ref() }.expect("Not a NULL PTR");
    let ea = ExtendedAddr::new_simple(xpub.clone(), protocol_magic.into());
    let address = Box::new(ea);
    Box::into_raw(address)
}

#[no_mangle]
pub extern "C" fn cardano_address_delete(c_addr: AddressPtr) {
    unsafe { Box::from_raw(c_addr) };
}

#[no_mangle]
pub extern "C" fn cardano_address_import_base58(c_address: *mut c_char) -> AddressPtr {
    let address_base58 = unsafe { ffi::CStr::from_ptr(c_address).to_bytes() };
    if let Ok(address_raw) = base58::decode_bytes(address_base58) {
        if let Ok(ea) = ExtendedAddr::try_from_slice(&address_raw[..]) {
            let address = Box::new(ea);
            Box::into_raw(address)
        } else {
            ptr::null_mut()
        }
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn cardano_address_export_base58(c_addr: AddressPtr) -> *const c_char {
    let address = unsafe { c_addr.as_ref() }.expect("Not a NULL PTR");
    ffi_address_to_base58(address).into_raw()
}
