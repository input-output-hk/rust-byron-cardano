#![cfg_attr(feature = "with-bench", feature(test))]

extern crate rcw;
extern crate wallet_crypto;

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

#[macro_use]
extern crate serde_derive;
extern crate serde;

pub mod types;
pub mod genesis; /* genesis block related value */
pub mod normal; /* normal block related value */
pub mod block;

pub use types::*;
pub use block::*;
