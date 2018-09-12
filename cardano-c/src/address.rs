use std::ffi;
use std::os::raw::{c_char, c_int};

use cardano::{address::ExtendedAddr, util::base58};

use super::types::{AddressPtr, XPubPtr};

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
        if let Ok(_) = ExtendedAddr::from_bytes(&address_raw[..]) {
            return 0;
        } else {
            return 2;
        }
    } else {
        return 1;
    }
}

pub extern "C" fn cardano_address_new_from_pubkey(c_xpubkey: XPubPtr) -> AddressPtr {
    unimplemented!()
}

pub extern "C" fn cardano_address_delete(c_addr: AddressPtr) {
    unsafe { Box::from_raw(c_addr) };
}

pub extern "C" fn cardano_address_import_base58(c_addr: *mut c_char) -> AddressPtr {
    unimplemented!()
}

pub extern "C" fn cardano_address_export_base58(c_addr: AddressPtr) -> *const c_char {
    unimplemented!()
}
