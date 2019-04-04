use cardano::hdwallet;
use std::{ptr, slice};
use types::{CardanoResult, XPrvPtr, XPubPtr};

#[no_mangle]
pub extern "C" fn cardano_xprv_derive(c_xprv: XPrvPtr, index: u32) -> XPrvPtr {
    let xprv = unsafe { c_xprv.as_mut() }.expect("Not a NULL PTR");
    let child = xprv.derive(hdwallet::DerivationScheme::V2, index);
    let child = Box::new(child);
    Box::into_raw(child)
}

#[no_mangle]
pub extern "C" fn cardano_xprv_from_bytes(
    c_xprv: *const u8,
    xprv_out: *mut XPrvPtr,
) -> CardanoResult {
    let xprv_data = unsafe { slice::from_raw_parts(c_xprv, hdwallet::XPRV_SIZE) };
    let array = {
        let mut array = [0u8; 96];
        array.copy_from_slice(xprv_data);
        array
    };
    match hdwallet::XPrv::from_bytes_verified(array) {
        Ok(r) => {
            let xprv = Box::new(r);
            unsafe { ptr::write(xprv_out, Box::into_raw(xprv)) };
            CardanoResult::success()
        }
        Err(_) => CardanoResult::failure(),
    }
}

#[no_mangle]
pub extern "C" fn cardano_xprv_to_bytes(c_xprv: XPrvPtr) -> *const u8 {
    //Get the inner byte array without taking ownership
    let slice: &[u8] = unsafe { (*c_xprv).as_ref() };

    let mut vector: Vec<u8> = Vec::with_capacity(hdwallet::XPRV_SIZE);
    vector.extend_from_slice(slice);

    //Get pointer to the inner value
    let ptr = vector.as_ptr();

    //Avoid running the destructor
    std::mem::forget(vector);
    ptr
}

#[no_mangle]
pub extern "C" fn cardano_xprv_bytes_delete(bytes: *mut u8) {
    let mut vector =
        unsafe { Vec::from_raw_parts(bytes, hdwallet::XPRV_SIZE, hdwallet::XPRV_SIZE) };
    cardano::util::securemem::zero(&mut vector);
    std::mem::drop(vector)
}

#[no_mangle]
pub extern "C" fn cardano_xprv_to_xpub(c_xprv: XPrvPtr) -> XPubPtr {
    let xprv = unsafe { c_xprv.as_mut() }.expect("Not a NULL PTR");
    let xpub = Box::new(xprv.public());
    Box::into_raw(xpub)
}

#[no_mangle]
pub extern "C" fn cardano_xprv_delete(c_xprv: XPrvPtr) {
    unsafe { Box::from_raw(c_xprv) };
}

#[no_mangle]
pub extern "C" fn cardano_xpub_derive(c_xpub: XPubPtr, index: u32) -> XPubPtr {
    let xpub = unsafe { c_xpub.as_mut() }.expect("Not a NULL PTR");
    match xpub.derive(hdwallet::DerivationScheme::V2, index) {
        Ok(r) => {
            let child = Box::new(r);
            Box::into_raw(child)
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn cardano_xpub_delete(c_xpub: XPubPtr) {
    unsafe { Box::from_raw(c_xpub) };
}
