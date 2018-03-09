cardano rust / wasm experiments
===============================

Installation
------------
```
# install rustup
curl https://sh.rustup.rs -sSf | sh
# use nightly version
rustup install nightly
rustup default nightly
# with wasm32 target
rustup target add wasm32-unknown-unknown --toolchain nightly
```

Build the Library
-----------------

To transpile the rust crypto to a Web Assembly (WASM) module and build JS library run the `./build` script.
(only necessary if you want to build locally)

Run the Example
-------------------
There is a simple example application in `js-example` that can be run to test some of the features.

### installation

within `js-example/` folder

2. `npm install`
3. `npm run install`

### Building
within `js-example/` run `npm run build`

### Running
open `js-example/index.html` in any browser

Use the Crypto Library
----------------------

### Install
You can either build the library locally on your machine
to test the latest version with your project or install via NPM.

#### Install locally:
in the root of this repo: `npm link`
in the root of your project: `npm link rust-cardano-crypto`

#### Install via NPM:
in the root of your project: `npm install rust-cardano-crypto`

### Import the API in Your Code
```js
// Import like this:
import CardanoCrypto from 'rust-cardano-crypto'
// Or access as global in browsers:
CardanoCrypto.PaperWallet.scramble(iv, password, input)
```

Notes
-----

The rust code contains `rwc/` a fork of [rust-crypto](https://github.com/DaGenix/rust-crypto)
without the dependencies that cannot be build easily in a wasm environment, and minus the 
algorithms that is not useful.
