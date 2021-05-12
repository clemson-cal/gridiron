//! Provides a message-passing communicator that does nothing.
//! 
//! Useful for testing and for execution strategies that require a
//! communicator of some type.

use super::comm::Communicator;

/// A message-passing communicator that does nothing. The `rank` and `size`
/// members are functioning but `send` and `recv` are `unimplemented`.
pub struct NullCommunicator {}

impl NullCommunicator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Communicator for NullCommunicator {
    fn rank(&self) -> usize {
        0
    }

    fn size(&self) -> usize {
        1
    }

    fn send(&self, _rank: usize, _message: Vec<u8>) {
        unimplemented!("cannot send on a null communicator")
    }

    fn recv(&self) -> Vec<u8> {
        unimplemented!("cannot recv on a null communicator")
    }

    fn next_time_stamp(&mut self) {        
    }
}

impl Default for NullCommunicator {
    fn default() -> Self {
        Self::new()
    }
}
