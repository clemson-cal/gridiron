use super::comm::Communicator;

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
}
