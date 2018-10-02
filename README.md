[![Build Status](https://travis-ci.org/input-output-hk/rust-cardano.svg?branch=master)](https://travis-ci.org/input-output-hk/rust-cardano)
[![Build status](https://ci.appveyor.com/api/projects/status/owl4qu760o6r0g1o?svg=true)](https://ci.appveyor.com/project/input-output-hk/rust-cardano)
[![Gitter chat](https://img.shields.io/badge/gitter-join%20chat%20%E2%86%92-brightgreen.svg)](https://gitter.im/input-output-hk/Cardano-Rust)

# Rust implementation of Cardano primitives, helpers, and related applications
Cardano Rust is a modular toolbox of Cardano’s cryptographic primitives, a library of wallet functions and a future alternative Cardano node implementation written in Rust. It can be used by any third-party to build wallet applications and interact with the Cardano blockchain. 

## Related repositories

* [cardano-cli](https://github.com/input-output-hk/cardano-cli)

## Installation

If this is a new installation:
[install rust's toolchain](https://www.rust-lang.org/en-US/install.html).

We support the following states; `stable`, `unstable` and `nightly`.

We also support the `wasm32` target.

## Building the Library

To build the library, use:

```
cargo build
```

## Running the tests

To run the tests, use:

```
cargo test
```

## How to integrate the Rust library in your project

Information will be available soon on crates.io

In the mean time, it is possible to add the project using git submodules:

```git submodule add https://github.com/input-output-hk/rust-cardano cardano-deps```

And then by adding the following to your Cargo.toml:

```[dependencies]
cardano = { path = "cardano-deps/cardano" }```


## Notes

The rust code contains `cryptoxide/` a fork of [rust-crypto](https://github.com/DaGenix/rust-crypto)
without the dependencies that cannot be build easily in a `wasm` environment, and minus the
algorithms that are not useful for cardano.
