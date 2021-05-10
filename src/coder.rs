/// An object that can encode a particular type to, and decode it from, a
/// `Vec<u8>`. The implementation can be based on a `serde` data format, or
/// anything else.
pub trait Coder {
    type Type;

    /// Consume an instance of the encodable type and convert it to bytes.
    fn encode(&self, inst: Self::Type) -> Vec<u8>;

    /// Consume a buffer of bytes and decode it to the decodable type.
    fn decode(&self, data: Vec<u8>) -> Self::Type;
}

/// Shim implementation of `Coder`. Calling `encode` or `decode` results in
/// `unimplemented` type panic.
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
