// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A pure-rust implementation of various cryptographic algorithms, which no dependencies
//! and no foreign code (specially C or assembly).
//!
//! Our goals is to support rust cryptography in various constrained environment like embedded devices and web assembly
//!
//! This is a fork of [Rust-Crypto by DaGenix](https://github.com/DaGenix/rust-crypto),
//! which we owe a debt of gratitude for starting some good quality pure rust implementations
//! of various cryptographic algorithms.
//!
//! Notable Differences with the original sources:
//!
//! * Maintained
//! * Extended ED25519 support for extended secret key (64 bytes) support
//! * Proper implementation of ChaChaPoly1305
//! * Many cryptographic algorithms removed: AES, Blowfish, Fortuna, RC4, RIPEMD160, Whirlpool, MD5, SHA1.
//!
//! As with everything cryptographic implementations, please make sure it suits your security requirements,
//! and review and audit before using.
//!
pub mod blake2b;
pub mod blake2s;
pub mod buffer;
pub mod symmetriccipher;
pub mod chacha20;
pub mod chacha20poly1305;
mod cryptoutil;
pub mod curve25519;
pub mod digest;
pub mod ed25519;
pub mod hmac;
pub mod hkdf;
pub mod mac;
pub mod pbkdf2;
pub mod poly1305;
pub mod sha2;
pub mod sha3;
mod simd;
pub mod util;
