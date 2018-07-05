// Copyright 2012-2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/*!
An implementation of the SHA-3 cryptographic hash algorithms.

There are 6 standard algorithms specified in the SHA-3 standard:

 * `SHA3-224`
 * `SHA3-256`
 * `SHA3-384`
 * `SHA3-512`
 * `SHAKE128`, an extendable output function (XOF)
 * `SHAKE256`, an extendable output function (XOF)
 * `Keccak224`, `Keccak256`, `Keccak384`, `Keccak512` (NIST submission without padding changes)

Based on an [implementation by Sébastien Martini](https://github.com/seb-m/crypto.rs/blob/master/src/sha3.rs)

# Usage

An example of using `SHA3-256` is:

```rust
use self::cryptoxide::digest::Digest;
use self::cryptoxide::sha3::Sha3;

// create a SHA3-256 object
let mut hasher = Sha3::sha3_256();

// write input message
hasher.input_str("abc");

// read hash digest
let hex = hasher.result_str();

assert_eq!(hex, "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532");
```

 */

use std::cmp;

use digest::Digest;
use cryptoutil::{write_u64v_le, read_u64v_le, zero};

const B: usize = 200;
const NROUNDS: usize = 24;
const RC: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008
];
const ROTC: [usize; 24] = [
    1, 3, 6, 10, 15, 21, 28, 36,
    45, 55, 2, 14, 27, 41, 56, 8,
    25, 43, 62, 18, 39, 61, 20, 44
];
const PIL: [usize; 24] = [
    10, 7, 11, 17, 18, 3, 5, 16,
    8, 21, 24, 4, 15, 23, 19, 13,
    12, 2, 20, 14, 22, 9, 6, 1
];
const M5: [usize; 10] = [
    0, 1, 2, 3, 4, 0, 1, 2, 3, 4
];

#[inline]
fn rotl64(v: u64, n: usize) -> u64 {
    ((v << (n % 64)) & 0xffffffffffffffff) ^ (v >> (64 - (n % 64)))
}

// Code based on Keccak-compact64.c from ref implementation.
fn keccak_f(state: &mut [u8]) {
    assert!(state.len() == B);

    let mut s: [u64; 25] = [0; 25];
    let mut t: [u64; 1] = [0; 1];
    let mut c: [u64; 5] = [0; 5];

    read_u64v_le(&mut s, state);

    for round in 0..NROUNDS {
        // Theta
        for x in 0..5 {
            c[x] = s[x] ^ s[5 + x] ^ s[10 + x] ^ s[15 + x] ^ s[20 + x];
        }
        for x in 0..5 {
            t[0] = c[M5[x + 4]] ^ rotl64(c[M5[x + 1]], 1);
            for y in 0..5 {
                s[y * 5 + x] = s[y * 5 + x] ^ t[0];
            }
        }

        // Rho Pi
        t[0] = s[1];
        for x in 0..24 {
            c[0] = s[PIL[x]];
            s[PIL[x]] = rotl64(t[0], ROTC[x]);
            t[0] = c[0];
        }

        // Chi
        for y in 0..5 {
            for x in 0..5 {
                c[x] = s[y * 5 + x];
            }
            for x in 0..5 {
                s[y * 5 + x] = c[x] ^ (!c[M5[x + 1]] & c[M5[x + 2]]);
            }
        }

        // Iota
        s[0] = s[0] ^ RC[round];
    }

    write_u64v_le(state, &s);
}


/// SHA-3 Modes.
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
pub enum Sha3Mode {
    Sha3_224,
    Sha3_256,
    Sha3_384,
    Sha3_512,
    Shake128,
    Shake256,
    Keccak224,
    Keccak256,
    Keccak384,
    Keccak512,
}

impl Sha3Mode {
    /// Return the expected hash size in bytes specified for `mode`, or 0
    /// for modes with variable output as for shake functions.
    pub fn digest_length(&self) -> usize {
        match *self {
            Sha3Mode::Sha3_224 | Sha3Mode::Keccak224 => 28,
            Sha3Mode::Sha3_256 | Sha3Mode::Keccak256 => 32,
            Sha3Mode::Sha3_384 | Sha3Mode::Keccak384 => 48,
            Sha3Mode::Sha3_512 | Sha3Mode::Keccak512 => 64,
            Sha3Mode::Shake128 | Sha3Mode::Shake256 => 0
        }
    }

    /// Return `true` if `mode` is a SHAKE mode.
    pub fn is_shake(&self) -> bool {
        match *self {
            Sha3Mode::Shake128 | Sha3Mode::Shake256 => true,
            _ => false
        }
    }

    /// Return `true` if `mode` is a Keccak mode.
    pub fn is_keccak(&self) -> bool {
        match *self {
            Sha3Mode::Keccak224 | Sha3Mode::Keccak256 | Sha3Mode::Keccak384 | Sha3Mode::Keccak512 => true,
            _ => false
        }
    }

    /// Return the capacity in bytes.
    fn capacity(&self) -> usize {
        match *self {
            Sha3Mode::Sha3_224 | Sha3Mode::Keccak224 => 56,
            Sha3Mode::Sha3_256 | Sha3Mode::Keccak256 => 64,
            Sha3Mode::Sha3_384 | Sha3Mode::Keccak384 => 96,
            Sha3Mode::Sha3_512 | Sha3Mode::Keccak512 => 128,
            Sha3Mode::Shake128 => 32,
            Sha3Mode::Shake256 => 64
        }
    }
}


pub struct Sha3 {
    state: [u8; B],  // B bytes
    mode: Sha3Mode,
    can_absorb: bool,  // Can absorb
    can_squeeze: bool,  // Can squeeze
    offset: usize  // Enqueued bytes in state for absorb phase
                   // Squeeze offset for squeeze phase
}

impl Sha3 {
    /// New SHA-3 instanciated from specified SHA-3 `mode`.
    pub fn new(mode: Sha3Mode) -> Sha3 {
        Sha3 {
            state: [0; B],
            mode: mode,
            can_absorb: true,
            can_squeeze: true,
            offset: 0
        }
    }

    /// New SHA3-224 instance.
    pub fn sha3_224() -> Sha3 {
        Sha3::new(Sha3Mode::Sha3_224)
    }

    /// New SHA3-256 instance.
    pub fn sha3_256() -> Sha3 {
        Sha3::new(Sha3Mode::Sha3_256)
    }

    /// New SHA3-384 instance.
    pub fn sha3_384() -> Sha3 {
        Sha3::new(Sha3Mode::Sha3_384)
    }

    /// New SHA3-512 instance.
    pub fn sha3_512() -> Sha3 {
        Sha3::new(Sha3Mode::Sha3_512)
    }

    /// New SHAKE-128 instance.
    pub fn shake_128() -> Sha3 {
        Sha3::new(Sha3Mode::Shake128)
    }

    /// New SHAKE-256 instance.
    pub fn shake_256() -> Sha3 {
        Sha3::new(Sha3Mode::Shake256)
    }

    /// New Keccak224 instance.
    pub fn keccak224() -> Sha3 {
        Sha3::new(Sha3Mode::Keccak224)
    }

    /// New Keccak256 instance.
    pub fn keccak256() -> Sha3 {
        Sha3::new(Sha3Mode::Keccak256)
    }

    /// New Keccak384 instance.
    pub fn keccak384() -> Sha3 {
        Sha3::new(Sha3Mode::Keccak384)
    }

    /// New Keccak512 instance.
    pub fn keccak512() -> Sha3 {
        Sha3::new(Sha3Mode::Keccak512)
    }

    fn finalize(&mut self) {
        assert!(self.can_absorb);

        let output_bits = self.output_bits();

        let ds_len = if self.mode.is_keccak() {
            0
        } else if output_bits != 0 {
            2
        } else {
            4
        };

        fn set_domain_sep(out_len: usize, buf: &mut [u8]) {
            assert!(!buf.is_empty());
            if out_len != 0 {
                // 01...
                buf[0] &= 0xfe;
                buf[0] |= 0x2;
            } else {
                // 1111...
                buf[0] |= 0xf;
            }
        }

        // All parameters are expected to be in bits.
        fn pad_len(ds_len: usize, offset: usize, rate: usize) -> usize {
            assert!(rate % 8 == 0 && offset % 8 == 0);
            let r: i64 = rate as i64;
            let m: i64 = (offset + ds_len) as i64;
            let zeros = (((-m - 2) + 2 * r) % r) as usize;
            assert!((m as usize + zeros + 2) % 8 == 0);
            (ds_len as usize + zeros + 2) / 8
        }

        fn set_pad(offset: usize, buf: &mut [u8]) {
            assert!(buf.len() as f32 >= ((offset + 2) as f32 / 8.0).ceil());
            let s = offset / 8;
            let buflen = buf.len();
            buf[s] |= 1 << (offset % 8);
            for i in (offset % 8) + 1..8 {
                buf[s] &= !(1 << i);
            }
            for i in s + 1..buf.len() {
                buf[i] = 0;
            }
            buf[buflen - 1] |= 0x80;
        }

        let p_len = pad_len(ds_len, self.offset * 8, self.rate() * 8);

        let mut p: Vec<u8> = vec![0; p_len];

        if ds_len != 0 {
            set_domain_sep(self.output_bits(), &mut p);
        }

        set_pad(ds_len, &mut p);

        self.input(&p);
        self.can_absorb = false;
    }

    fn rate(&self) -> usize {
        B - self.mode.capacity()
    }
}

impl Digest for Sha3 {
    fn input(&mut self, data: &[u8]) {
        if !self.can_absorb {
            panic!("Invalid state, absorb phase already finalized.");
        }

        let r = self.rate();
        assert!(self.offset < r);

        let in_len = data.len();
        let mut in_pos: usize = 0;

        // Absorb
        while in_pos < in_len {
            let offset = self.offset;
            let nread = cmp::min(r - offset, in_len - in_pos);
            for i in 0..nread {
                self.state[offset + i] = self.state[offset + i] ^ data[in_pos + i];
            }
            in_pos += nread;

            if offset + nread != r {
                self.offset += nread;
                break;
            }

            self.offset = 0;
            keccak_f(&mut self.state);
        }
    }

    fn result(&mut self, out: &mut [u8]) {
        if !self.can_squeeze {
            panic!("Nothing left to squeeze.");
        }

        if self.can_absorb {
            self.finalize();
        }

        let r = self.rate();
        let out_len = self.mode.digest_length();
        if out_len != 0 {
            assert!(self.offset < out_len);
        } else {
            assert!(self.offset < r);
        }

        let in_len = out.len();
        let mut in_pos: usize = 0;

        // Squeeze
        while in_pos < in_len {
            let offset = self.offset % r;
            let mut nread = cmp::min(r - offset, in_len - in_pos);
            if out_len != 0 {
                nread = cmp::min(nread, out_len - self.offset);
            }

            for i in 0..nread {
                out[in_pos + i] = self.state[offset + i];
            }
            in_pos += nread;

            if offset + nread != r {
                self.offset += nread;
                break;
            }

            if out_len == 0 {
                self.offset = 0;
            } else {
                self.offset += nread;
            }

            keccak_f(&mut self.state);
        }

        if out_len != 0 && out_len == self.offset {
            self.can_squeeze = false;
        }
    }

    fn reset(&mut self) {
        self.can_absorb = true;
        self.can_squeeze = true;
        self.offset = 0;

        zero(&mut self.state);
    }

    fn output_bits(&self) -> usize {
        self.mode.digest_length() * 8
    }

    fn block_size(&self) -> usize {
        B - self.mode.capacity()
    }
}

impl Copy for Sha3 {

}

impl Clone for Sha3 {
    fn clone(&self) -> Self {
        *self
    }
}
