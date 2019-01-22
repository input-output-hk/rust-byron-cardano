//! Cardano Basic types and manipulation functions
//!
//! Features:
//!
//! * Address generation and parsing
//! * Block types and parsing
//! * HDWallet (ED25519-BIP32)
//! * BIP39 codec (Including dictionaries: English, Japanese, French, Spanish, Chinese)
//! * BIP44 wallet addressing scheme
//! * Paperwallet V1
//! * Transaction creation, parsing, signing
//! * Fee calculation
//! * Redeem Key
//! * Wallet abstraction
//!
#![cfg_attr(feature = "with-bench", feature(test))]

#[cfg(feature = "generic-serialization")]
#[macro_use]
extern crate serde_derive;
#[cfg(feature = "generic-serialization")]
extern crate serde;

#[cfg(test)]
extern crate serde_json;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[cfg(test)]
extern crate rand;

extern crate cryptoxide;
#[macro_use]
extern crate cbor_event;

extern crate chain_core;

#[cfg(test)]
extern crate base64;

pub mod address;
pub mod coin;
pub mod config;
mod crc32;
pub mod fee;
pub mod hash;
pub mod hdpayload;
pub mod hdwallet;
pub mod input_selection;
pub mod paperwallet;
pub mod redeem;
pub mod tx;
pub mod txbuild;
pub mod txutils;
pub mod util;

pub mod bip;
pub mod block;
pub mod cbor;
pub mod wallet;

pub mod merkle;
pub mod tags;
pub mod vss;
