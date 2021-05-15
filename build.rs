fn main() {
    #[cfg(feature = "mpi")]
    {
        println!("cargo:rustc-link-lib=mpi");
        cc::Build::new().file("src/mpi/mpi.c").compile("mpi.a");
    }
}
