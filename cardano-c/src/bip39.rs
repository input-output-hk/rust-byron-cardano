use std::slice;

use cardano::bip::bip39;
use types::CardanoResult;

use std::{
    ptr,
    os::raw::{c_char, c_uchar, c_int, c_uint},
};

use std::ffi::CStr;

/// encode a entropy into its equivalent words represented by their index (0 to 2047) in the BIP39 dictionary
#[no_mangle]
pub extern "C" fn cardano_bip39_encode(
    entropy_raw: *const u8,             /* raw entropy */
    entropy_bytes: usize,               /* the number of bytes to encode */
    encoded: *mut bip39::MnemonicIndex, /* the encoded entropy */
    encoded_size: usize,
) -> CardanoResult {
    let in_slice = unsafe { slice::from_raw_parts(entropy_raw, entropy_bytes) };
    let out_slice = unsafe { slice::from_raw_parts_mut(encoded, encoded_size) };
    let entropy = match bip39::Entropy::from_slice(in_slice) {
        Ok(e) => e,
        Err(_) => return CardanoResult::failure(),
    };
    out_slice.copy_from_slice(entropy.to_mnemonics().as_ref());
    CardanoResult::success()
}

///
/// Error status:
///     0: Success
///     1: The words were not in the english dictionary
///     2: The checksum was invalid
/// 
#[no_mangle]
pub extern "C" fn cardano_entropy_from_mnemonics(
    mnemonics: *const c_char,
    entropy_ptr: *mut *const c_uchar,
    entropy_size: *mut c_uint 
) -> c_int {
    let rust_string = unsafe { CStr::from_ptr(mnemonics) }.to_string_lossy(); 

    let dictionary = bip39::dictionary::ENGLISH;

    let mnemonics = match bip39::Mnemonics::from_string(&dictionary, &rust_string)
    {
        Ok(m) => m,
        Err(_) => return 1,
    };

    let entropy = match bip39::Entropy::from_mnemonics(&mnemonics) {
        Ok(e) => e,
        Err(_) => return 2,
    };
    
    let mut entropy_vec = match entropy {
        bip39::Entropy::Entropy9(arr) => arr.to_vec(),
        bip39::Entropy::Entropy12(arr) => arr.to_vec(),
        bip39::Entropy::Entropy15(arr) => arr.to_vec(),
        bip39::Entropy::Entropy18(arr) => arr.to_vec(),
        bip39::Entropy::Entropy21(arr) => arr.to_vec(),
        bip39::Entropy::Entropy24(arr) => arr.to_vec(),
    };

    //Make sure the capacity is the same as the length to make deallocation simpler
    entropy_vec.shrink_to_fit();

    let pointer = entropy_vec.as_mut_ptr();
    let length = entropy_vec.len() as u32;

    //To avoid deallocation
    std::mem::forget(entropy_vec);

    //Write the array length
    unsafe { ptr::write(entropy_size, length) }

    //Copy the pointer to the out parameter
    unsafe { ptr::write(entropy_ptr, pointer) };

    0
}

//Deallocate the rust-allocated memory for a Entropy array
#[no_mangle]
pub extern "C" fn cardano_delete_entropy_array(ptr: *mut c_uchar, size: u32) {
    let len = size as usize;
    unsafe { drop(Vec::from_raw_parts(ptr, len, len)) };
}