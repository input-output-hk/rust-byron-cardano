# CBOR Event library

This library provides a simple, yet efficient CBOR binary parser/serialiser.

While some library would provide an intermediate type representation,
this library focus on minimising the overhead costs to parsing CBOR.

## How to use

### Add dependency to your crate:

* in your `Cargo.toml`:
  ```toml
  [dependencies]
  cbor_event = "^0.1"
  ```
* in your `lib.rs` or `main.rs`:
  ```rust
  extern crate cbor_event;
  ```
