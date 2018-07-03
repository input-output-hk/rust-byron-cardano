# CBOR Event library

This library provides a simple, yet efficient CBOR binary parser/serialiser.

While some library would provide an intermediate type representation,
this library focus on minimising the overhead costs to parsing CBOR.

This crate has 0 dependency (and should not need any in the furure). This
is a design choice in order to guarantee as much as possible of the
compatibility across multiple platform.

# Add dependency to your crate:

* in your `Cargo.toml`:
  ```toml
  [dependencies]
  cbor_event = "^0.1"
  ```
* in your `lib.rs` or `main.rs`:
  ```rust
  extern crate cbor_event;
  ```
