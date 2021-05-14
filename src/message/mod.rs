//! A minimal distributed message-passing API.
//! 
//! This module exports a minimal message-passing API, which is encapsulated
//! by the [`comm::Communicator`] trait. Implementors only need to write
//! `send` and `recv` operations for a given transport layer (a pure-Rust TCP
//! example is included in [`tcp::TcpCommunicator`]). The trait then provides
//! default implementations for broadcast, reduce, and reduce-all operations.

pub mod comm;
pub mod mpi;
pub mod null;
pub mod tcp_v1;
pub mod tcp_v2;
pub mod util;
