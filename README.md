# gridiron

Gridiron is an adaptive mesh refinement (AMR) library for solving
time-dependent systems of conservation laws, like the Euler equations of
gas dynamics. It uses structured, rectilinear grid patches in the style of
Berger-Oliger AMR, where patches can be placed at different refinement
levels in flexible configurations: patches may overlap one another and
fine patches may cross coarse patch boundaries. This is in contrast to
more constrained quad-tree / oct-tree mesh topologies used e.g. in the
Flash code.

This library is a work-in-progress in early stages. Its goals are:

- Provide meshing and execution abstractions for hydrodynamics base
  schemes. If you have a scheme that works on logically Cartesian grid
  patches, this library can make that scheme suitable for AMR simulations.
- Be aggressively optimized in terms of computations, array traversals (no
  multi-dimensional indexing), and memory access patterns (optimal cache +
  heap utilization).
- Provide efficient strategies for hybrid parallelization based on
  shared memory and distributed multi-processing.
- Have minimal dependencies. The library can be used without any outside
  crates. Optional dependences include `rayon` (for its thread pool, although
  a custom thread pool is also included), `serde` (for message passing and
  checkpoints). Optional features that only effect performance are
  `crossbeam_channel` and `core_affinity`. The `examples/euler` sub-crate
  demonstrates ues of all the optional features.
- Have fast compile times. The debug cycle for physics simulations often
  requires frequent recompilation and inspection of results. Compile times of
  1-2 seconds are fine, but the code should not take 30 seconds to compile, as
  can happen with excessive use of generics, `async`, link-time optimizations,
  etc. For this reason the primary data structure (`patch::Patch`) is not
  generic over an array element type; it uses `f64` and a runtime-specified
  number of fields per grid cell location. It's encouraged to keep solver and
  physics code together in the same crate as the science application, because
  it allows `rustc` to optimize these modules together without link-time
  optimizations. There isn't much compute-intensive work done in the
  `gridiron` library functions, so there's no performance penalties for using
  it as a separate crate.
- Provide examples of stand-alone applications which use the library.

It does _not_ attempt to

- Be a complete application framework. Data input/output, user
  configurations, visualization and post-processing should be handled by
  separate crates or by applications written for a specific science
  problem.
- Provide lots of physics. The library will be written to facilitate
  multi-physics science applications that may require MHD, tracer particles,
  radiative transfer, self-gravity, and reaction networks. However, this
  library does not try to implement these things. The focus is the meshing and
  parallelization.

# Building with MPI (optional)
MPI is not required for distributed parallel calculations, because there is a
built-in message-passing module based on TCP sockets.

If you want to use MPI on an HPC cluster, just make sure you've loaded one of
their MPI modules with e.g. `module load mpi`. On your laptop or workstation,
you'll need to have either `OpenMPI` or `mpich` installed, in addition to
`automake`. On Mac, these can be installed with Homebrew:

```bash
brew install mpich
brew install automake
```
