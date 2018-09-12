use cardano::hdwallet;
use std::{
    ffi,
    os::raw::{c_char, c_int},
};
use types::{XPrvPtr, XPubPtr};

pub extern "C" fn cardano_xprv_from_bytes(c_xprv: *const c_char) -> XPrvPtr {
    unimplemented!()
}

pub extern "C" fn cardano_xprv_to_bytes(c_xprv: XPrvPtr) -> *const c_char {
    unimplemented!()
}

pub extern "C" fn cardano_xprv_to_xpub(c_xprv: XPrvPtr) -> XPubPtr {
    let xprv = unsafe { c_xprv.as_mut() }.expect("Not a NULL PTR");
    let xpub = Box::new(xprv.public());
    Box::into_raw(xpub)
}

pub extern "C" fn cardano_xprv_delete(c_xpub: XPrvPtr) {
    unsafe { Box::from_raw(c_xprv) };
}

pub extern "C" fn cardano_xpub_delete(c_xpub: XPubPtr) {
    unsafe { Box::from_raw(c_xpub) };
}
