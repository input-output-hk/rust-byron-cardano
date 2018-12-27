use bip::bip39;

use cryptoxide::hmac::Hmac;
use cryptoxide::pbkdf2::pbkdf2;
use cryptoxide::sha2::Sha512;

pub type Entropy = bip39::Entropy;

/// Create a new seed from entropy and password
///
/// The output size of pbkdf2 is associated with the size of the slice, allowing
/// to generate a seed of the size required for various specific cryptographic object
pub fn generate_seed(entropy: &Entropy, password: &[u8], output: &mut [u8]) {
    const ITER: u32 = 4096;
    let mut mac = Hmac::new(Sha512::new(), password);
    pbkdf2(&mut mac, entropy.as_ref(), ITER, output)
}
