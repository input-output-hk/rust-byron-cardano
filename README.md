[![Build Status](https://travis-ci.org/input-output-hk/rust-cardano.svg?branch=master)](https://travis-ci.org/input-output-hk/rust-cardano)
[![Build status](https://ci.appveyor.com/api/projects/status/owl4qu760o6r0g1o?svg=true)](https://ci.appveyor.com/project/input-output-hk/rust-cardano)

# rust implementation of cardano wallet and crypto

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

The rust code contains `rwc/` a fork of [rust-crypto](https://github.com/DaGenix/rust-crypto)
without the dependencies that cannot be build easily in a `wasm` environment, and minus the
algorithms that are not useful.
