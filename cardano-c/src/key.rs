use cardano::hdwallet;
use std::{
    ffi,
    ptr,
    os::raw::{c_char, c_int},
};
use types::{XPrvPtr, XPubPtr};

#[no_mangle]
pub extern "C" fn cardano_xprv_derive(c_xprv: XPrvPtr, index: u32) -> XPrvPtr {
    let xprv = unsafe { c_xprv.as_mut() }.expect("Not a NULL PTR");
    let child = xprv.derive(hdwallet::DerivationScheme::V2, index);
    let child = Box::new(child);
    Box::into_raw(child)
}

#[no_mangle]
pub extern "C" fn cardano_xprv_from_bytes(c_xprv: *const u8) -> XPrvPtr {
    let xprv_data = unsafe { slice::from_raw_parts(c_xprv, hdwallet::XPRV_SIZE) };
    match hdwallet::XPrv::from_slice(xprv_data) {
        Ok(r) => {
            let xprv = Box::new(r);
            Box::into_raw(xprv)
        },
        Err(_) => {
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn cardano_xprv_to_bytes(c_xprv: XPrvPtr) -> *const u8 {
    unimplemented!()
}

#[no_mangle]
pub extern "C" fn cardano_xprv_to_xpub(c_xprv: XPrvPtr) -> XPubPtr {
    let xprv = unsafe { c_xprv.as_mut() }.expect("Not a NULL PTR");
    let xpub = Box::new(xprv.public());
    Box::into_raw(xpub)
}

#[no_mangle]
pub extern "C" fn cardano_xprv_delete(c_xpub: XPrvPtr) {
    unsafe { Box::from_raw(c_xprv) };
}

#[no_mangle]
pub extern "C" fn cardano_xpub_derive(c_xprv: XPubPtr, index: u32) -> XPubPtr {
    let xpub = unsafe { c_xpub.as_mut() }.expect("Not a NULL PTR");
    match xpub.derive(hdwallet::DerivationScheme::V2, index) {
        Ok(r) => {
            let child = Box::new(r);
            Box::into_raw(child)
        },
        Err(_) => {
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn cardano_xpub_delete(c_xpub: XPubPtr) {
    unsafe { Box::from_raw(c_xpub) };
}
