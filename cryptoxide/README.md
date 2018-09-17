# Cryptoxide

[![Build Status](https://travis-ci.org/input-output-hk/rust-cardano.svg?branch=master)](https://travis-ci.org/input-output-hk/rust-cardano)
![MIT or APACHE-2 licensed](https://img.shields.io/badge/licensed-MIT%20or%20APACHE--2-blue.svg)

A pure-rust implementation of various modern cryptographic algorithms, which no dependencies
and no foreign code (specially C or assembly), to work with WASM and embedded devices.

This crates is aiming to support as many architectures as possible with as
little dependencies as possible.

Disclaimer: As everything cryptographically related, no warranties.

## Fork information

This is a fork of [Rust-Crypto by DaGenix](https://github.com/DaGenix/rust-crypto),
which we owe a debt of gratitude for starting some good quality pure rust implementations
of various cryptographic algorithms.

Notable Differences with the original sources:

* Maintained.
* Extended ED25519 support for extended secret key (64 bytes) support.
* Proper implementation of ChaChaPoly1305 (according to spec).
* Many cryptographic algorithms removed: AES, Blowfish, Fortuna, RC4, RIPEMD160, Whirlpool, MD5, SHA1.

## Supported targets

```
rustup target add aarch64-apple-ios # or any target below
```

| Target                               | `test` |
|--------------------------------------|:------:|
| `aarch64-unknown-linux-gnu`          |   ✓    |
| `aarch64-linux-android`              |   ✓    |
| `aarch64-apple-ios`                  |   ✓    |
| `arm-unknown-linux-gnueabi`          |   ✓    |
| `arm-linux-androideabi`              |   ✓    |
| `armv7-unknown-linux-gnueabihf`      |   ✓    |
| `armv7-linux-androideabi`            |   ✓    |
| `armv7-apple-ios`                    |   ✓    |
| `armv7s-apple-ios`                   |   ✓    |
| `i686-unknown-linux-gnu`             |   ✓    |
| `i686-unknown-linux-musl`            |   ✓    |
| `i686-unknown-freebsd`               |   ✓    |
| `i686-apple-ios`                     |   ✓    |
| `i686-apple-darwin`                  |   ✓    |
| `i686-linux-android`                 |   ✓    |
| `x86_64-unknown-linux-gnu`           |   ✓    |
| `x86_64-unknown-linux-musl`          |   ✓    |
| `x86_64-linux-android`               |   ✓    |
| `x86_64-apple-darwin`                |   ✓    |
| `x86_64-apple-ios`                   |   ✓    |
| `x86_64-unknown-freebsd`             |   ✓    |
| `wasm32-unknown-emscripten`          |   ✓    |
| `wasm32-unknown-unknown`             |   ✓    |

## supported compiler versions

| Rust    | `test` |
|---------|:------:|
| stable  |   ✓    |
| beta    |   ✓    |
| nightly |   ✓    |

We will always aim to support the current stable. However it is likely
that older version of the rust compiler is supported too.

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.
