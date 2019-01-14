//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);

extern crate protocol_tokio as protocol;

pub mod server;
