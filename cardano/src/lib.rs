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

#[macro_use]
extern crate serde_derive;
extern crate serde;
#[cfg(test)]
extern crate serde_json;

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

extern crate cryptoxide;
#[macro_use]
extern crate cbor_event;

mod crc32;
pub mod util;
pub mod config;
pub mod hdwallet;
pub mod paperwallet;
pub mod address;
pub mod hdpayload;
pub mod tx;
pub mod txutils;
pub mod txbuild;
pub mod fee;
pub mod input_selection;
pub mod coin;
pub mod redeem;
pub mod hash;

mod cbor;
pub mod bip;
pub mod wallet;
pub mod block;

pub mod vss;
pub mod merkle;
pub mod tags;
