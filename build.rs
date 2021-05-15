fn main() {
    #[cfg(feature = "mpi")]
    {
        println!("cargo:rustc-link-lib=mpi");
        cc::Build::new().file("src/mpi/mpi.c").compile("mpi.a");
    }

    #[cfg(feature = "metal")]
    {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=Cocoa");
        cc::Build::new().file("src/metal/metal.m").compile("metal.a");
    }
}
