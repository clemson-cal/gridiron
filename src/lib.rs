//! Gridiron is an adaptive mesh refinement (AMR) library for solving
//! time-dependent systems of conservation laws, like the Euler equations of
//! gas dynamics. It uses structured, rectilinear grid patches in the style of
//! Berger-Oliger AMR, where patches can be placed at different refinement
//! levels in flexible configurations: patches may overlap one another and
//! fine patches may cross coarse patch boundaries. This is in contrast to
//! more constrained quad-tree / oct-tree mesh topologies used e.g. in the
//! Flash code.
//! 
//! This library is a work-in-progress in early stages. Its goals are:
//! 
//! - Provide meshing and execution abstractions for hydrodynamics base
//!   schemes. If you have a scheme that works on logically Cartesian grid
//!   patches, this library can make that scheme suitable for AMR simulations.
//! - Be aggressively optimized in terms of computations (eliminating
//!   redundancy), array traversals (no multi-dimensional indexing), memory
//!   access patterns (optimal cache + heap utilization), and parallel
//!   execution strategies.
//! - Provide efficient strategies for hybrid parallelization based on
//!   shared memory and distributed multi-processing.
//! - Not to depend on other work-in-progress crates. Many HPC-oriented Rust
//!   crates look promising, but are still in flux; this crate should not
//!   depend on things like `ndarray`, `hdf5`, or `rsmpi`. Dependencies are
//!   currently limited to `rayon` and `crossbeam_channel`. `serde` is
//!   presently required in the library to support the examples, but it should
//!   be made optional.
//! - Have fast compile times. The debug cycle for physics simulations often
//!   requires frequent recompilation and inspection of results. Compile times
//!   of 1-2 seconds are fine, but the code should not take 30 seconds to
//!   compile, as can happen with lots of generics, excessive use of `async`,
//!   link-time optimizations, etc. For this reason the primary data structure
//!   [`patch::Patch`] is not generic over an array element type; it uses
//!   `f64` and a runtime-specified number of fields per array element.
//! - Provide examples of stand-alone applications which use the library.
//!   
//! It does _not_ attempt to
//! 
//! - Be a complete application framework. Data input/output, user
//!   configurations, visualization and post-processing should be handled by
//!   separate crates or by applications written for a specific science
//!   problem.
//! - Provide lots of physics. The library will be written to support
//!   multi-physics applications which require things like MHD, tracer
//!   particles, radiative transfer, self-gravity, and reaction networks.
//!   However, this library does not try to implement these things. The focus
//!   is on abstractions for meshing and execution.

pub mod adjacency_list;
pub mod aug_node;
pub mod automaton;
pub mod coder;
pub mod index_space;
pub mod interval_map;
pub mod interval_set;
pub mod meshing;
pub mod message;
pub mod metal;
pub mod mpi;
pub mod num_vec;
pub mod overlap;
pub mod patch;
pub mod rect_map;
pub mod thread_pool;
