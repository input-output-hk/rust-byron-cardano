[![Build Status](https://travis-ci.org/input-output-hk/rust-cardano.svg?branch=master)](https://travis-ci.org/input-output-hk/rust-cardano)
[![Build status](https://ci.appveyor.com/api/projects/status/owl4qu760o6r0g1o?svg=true)](https://ci.appveyor.com/project/input-output-hk/rust-cardano)
[![Gitter chat](https://img.shields.io/badge/gitter-join%20chat%20%E2%86%92-brightgreen.svg)](https://gitter.im/input-output-hk/Cardano-Rust)

# rust implementation of cardano primitives, helpers, and related applications

## Related repositories

* [cardano-cli](https://github.com/input-output-hk/cardano-cli)

## Installation

If not already,
[install rust's toolchain](https://www.rust-lang.org/en-US/install.html).

we support `stable`, `unstable` and `nightly`.

We also support `wasm32` target.

## Build the Library

```
cargo build
```

## Run the tests

```
cargo test
```

### installation


## Notes

The rust code contains `cryptoxide/` a fork of [rust-crypto](https://github.com/DaGenix/rust-crypto)
without the dependencies that cannot be build easily in a `wasm` environment, and minus the
algorithms that are not useful for cardano.
