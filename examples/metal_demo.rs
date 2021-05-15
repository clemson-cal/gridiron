#[cfg(feature = "metal")]
fn main() {
    use gridiron::metal;
    unsafe {
        metal::say_hello_from_objc();
    }
}

#[cfg(not(feature = "metal"))]
fn main() {
    println!("metal feature is disabled");
}
