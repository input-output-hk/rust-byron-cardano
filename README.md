cardano rust / wasm experiments
===============================

Wasm related experiments

to build with wasm, there's a handy `build` script. the rust compiler/environment for wasm need
to be installed prior to running this.

Note: this contains `rwc/` a fork of [rust-crypto](https://github.com/DaGenix/rust-crypto)
without the dependencies that cannot be build easily in a wasm environment, and minus the algorithms
that is not useful.
