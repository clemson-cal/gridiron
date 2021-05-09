/// An object that can encode and decode to and from a `Vec<u8>`. The implementation
/// can be `serde` or anything else.
pub trait Coder {
    type Type;
    fn encode(&self, inst: Self::Type) -> Vec<u8>;
    fn decode(&self, data: Vec<u8>) -> Self::Type;
}

/// Shim implementation of `Coder`.
pub struct NullCoder<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> NullCoder<T> {
    pub fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData::<T> {} 
        }
    }
}

impl<T> Coder for NullCoder<T>
{
    type Type = T;

    fn encode(&self, _: Self::Type) -> Vec<u8> {
        unimplemented!()
    }

    fn decode(&self, _: Vec<u8>) -> Self::Type {
        unimplemented!()
    }
}

impl<T> Default for NullCoder<T> {
    fn default() -> Self {
        Self::new()
    }
}
