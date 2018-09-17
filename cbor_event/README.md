# CBOR Event library

[![Build Status](https://travis-ci.org/input-output-hk/rust-cardano.svg?branch=master)](https://travis-ci.org/input-output-hk/rust-cardano)
![MIT or APACHE-2 licensed](https://img.shields.io/badge/licensed-MIT%20or%20APACHE--2-blue.svg)

This library provides a simple, yet efficient CBOR binary parser/serialiser.

While some libraries provide an intermediate type representation,
this crate has zero dependencies (and should not need any in the future).
This is a design choice in order to guarantee as much compatibility as possible
across multiple platforms.

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

We will always aim to support the current stable version. However, it is
likely that an older version of the Rust compiler is also supported.

# License

This project is licensed under either of the following licenses:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

Please choose the licence you want to use.
