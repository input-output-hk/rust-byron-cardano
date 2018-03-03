cardano rust / wasm experiments
===============================

Installation
------------

# install rustup
curl https://sh.rustup.rs -sSf | sh
# use nightly version
rustup install nightly
rustup default nightly
# with wasm32 target
rustup target add wasm32-unknown-unknown --toolchain nightly

Wasm related experiments
------------------------

to build with wasm, there's a handy `build` script. the rust compiler/environment for wasm need
to be installed prior to running this.

Note: this contains `rwc/` a fork of [rust-crypto](https://github.com/DaGenix/rust-crypto)
without the dependencies that cannot be build easily in a wasm environment, and minus the algorithms
that is not useful.
