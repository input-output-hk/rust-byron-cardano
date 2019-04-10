# C binding for the cardano library

[![Build Status](https://travis-ci.org/input-output-hk/rust-cardano.svg?branch=master)](https://travis-ci.org/input-output-hk/rust-cardano)
![MIT or APACHE-2 licensed](https://img.shields.io/badge/licensed-MIT%20or%20APACHE--2-blue.svg)
![Cardano Mainnet](https://img.shields.io/badge/Cardano%20Ada-mainnet-brightgreen.svg)
![Cardano Staging](https://img.shields.io/badge/Cardano%20Ada-staging-brightgreen.svg)
![Cardano Testnet](https://img.shields.io/badge/Cardano%20Ada-testnet-orange.svg)

Exports simple API to use in C library or to write bindings in
other languages.

# Cross compiling for different targets

To ease the process of cross-compiling to different plate-forms
and architectures we provide a build script,

```bash
./build.sh <TARGETS>
```

To see the list of supported platforms, see `rustup target list`.
Theoretically, all the targets are supported.

# Cross compiling for iOS

Use [cardano-lipo](https://github.com/TimNN/cargo-lipo)

# find linker for your targets

rust does not provide the linker for the targets, it is you to
provide it. For example, to cross compile for a given _`<target>`_:


- in `.cargo/config
  ```toml
  [target.<target>]
  linker = "/path/to/linker/for/<target>"
  ```
- then run build script:
  ```bash
  ./build.sh <target>
  ```

After successful completion of the script, you will find a directory `dist` at the root of this
crate with directories containing the differently built targets. In our example, something like:

```bash
$ tree dist/
dist/
└── cardano-c
    └── <target>
        ├── debug
        │   ├── libcardano_c.a
        │   └── libcardano_c.d
        └── release
            ├── libcardano_c.a
            └── libcardano_c.d
```

The `*.d` files are kept in case you want to integrate these in your build
and track changes to the rust library as well.

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

## supported compiler versions

| Rust    | `test` |
|---------|:------:|
| stable  |   ✓    |
| beta    |   ✓    |
| nightly |   ✓    |

We will always aim to support the current stable version. However, it is
likely that an older version of the Rust compiler is also supported.

# Running the tests

```bash
./test.sh
```

You can optionally run the tests with Valgrind by setting the environment variable **VALGRIND** to *true*

```bash
VALGRIND=true ./test.sh
```

# Documentation

Latest documentation generated from master: https://hydra.iohk.io/job/Cardano/rust-cardano/docs.cardano-c/latest

HTML documentation of the exposed functions can be generated with Doxygen by running

```bash
doxygen
```

The documentation is generated in the ./docs/html directory.

# License

This project is licensed under either of the following licenses:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

Please choose the licence you want to use.
