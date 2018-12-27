use std::slice;

use cardano::bip::bip39;
use types::CardanoResult;

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
