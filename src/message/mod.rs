//! This module exports a minimal message-passing API, which is encapsulated
//! by a `Communicator` trait. Implementors only need to write `send` and
//! `recv` operations for a given transport layer (a pure-Rust TCP example is
//! included). The trait then provides default implementations for broadcast,
//! reduce, and reduce-all operations.

pub mod comm;
pub mod null;
pub mod tcp;
pub mod util;
