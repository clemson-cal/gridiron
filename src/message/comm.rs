//! Exports the `Communicator` message-passing trait.

use super::util;

/// Interface for a group of processes that can exchange messages over a
/// network.
///
/// The underlying transport can in principle be TCP, UDP, or another
/// abstraction layer like MPI. The communicator is stateful, in the sense
/// that it maintains a time stamp to keep messages from distinct generations
/// (computational stages) separated from each other. Sent messages must
/// encode the current time stamp somehow, and the received messages also need
/// to match it. This implies that if a message for a future time stamp is
/// read from the socket, it needs to be stored for a future receive call once
/// the communicator has been advanced to that time stamp. If the communicator
/// is used by a distributed executor, it is the responsibility of the
/// executor to advance the time stamp after each stage of computation.
pub trait Communicator {
    /// Must be implemented to return the rank of this process within the
    /// communicator.
    fn rank(&self) -> usize;

    /// Must be implemented to return the number of peers processes in this
    /// communicator.
    fn size(&self) -> usize;

    /// Must be implemented to send a message to a peer. This method must
    /// return immediately, in other words it is not allowed to block until a
    /// matching receive is posted.
    fn send(&self, rank: usize, message: Vec<u8>);

    /// Must be implemented to receive a message from any of the peers. This
    /// method is allowed to block until a message is ready to be received
    fn recv(&self) -> Vec<u8>;

    /// Must be implemented to advance the communicator's internal time stamp.
    fn next_time_stamp(&mut self);

    /// Implements a binomial tree broadcast from the root node. The message
    /// buffer must be `Some` if this is the root node, and it must be `None`
    /// otherwise.
    fn broadcast(&self, value: Option<Vec<u8>>) -> Vec<u8> {
        let r = self.rank();
        let p = self.size();

        let value = match value {
            Some(value) => value,
            None => self.recv(),
        };
        for level in (0..util::ceil_log2(p)).rev() {
            let one = 1 << level;
            let two = 1 << (level + 1);

            if r % two == 0 && r + one <= p {
                self.send(r + one, value.clone())
            }
        }
        value
    }

    /// Implements a binomial tree reduce. All ranks return `None` except for
    /// the root.
    fn reduce<F>(&self, f: F, mut value: Vec<u8>) -> Option<Vec<u8>>
    where
        F: Fn(Vec<u8>, Vec<u8>) -> Vec<u8>,
    {
        let r = self.rank();
        let p = self.size();

        for level in (0..util::ceil_log2(p)).rev() {
            let one = 1 << level;
            let two = 1 << (level + 1);

            if r % two == 0 {
                value = f(value, self.recv())
            } else {
                self.send(r - one, value);
                return None;
            }
        }
        Some(value)
    }

    /// Implements an all-reduce (symmetric fold) operation over a commutative
    /// binary operator.
    fn all_reduce<F>(&self, f: F, value: Vec<u8>) -> Vec<u8>
    where
        F: Fn(Vec<u8>, Vec<u8>) -> Vec<u8>,
    {
        self.broadcast(self.reduce(f, value))
    }
}
