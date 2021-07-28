//! A minimal distributed message-passing API.
//!
//! This module exports a minimal message-passing API, which is encapsulated
//! by the [`comm::Communicator`] trait. Implementors only need to write
//! `send` and `recv` operations for a given transport layer (a pure-Rust TCP
//! example is included in [`tcp::TcpCommunicator`]). The trait then provides
//! default implementations for broadcast, reduce, and reduce-all operations.

mod comm;
mod mpi;
mod null;
mod tcp;
mod util;

pub use comm::Communicator;
pub use tcp::TcpCommunicator;
pub use null::NullCommunicator;
#[cfg(feature = "mpi")]
pub use mpi::MpiCommunicator;
