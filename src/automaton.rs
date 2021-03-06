//! Provides an interface ([`Automaton`]) for task-based parallelism.
//!
//! This module also provides a handful of sample multi-threaded and
//! distributed execution strategies which might be useful in production. It
//! models computations consisting of an ensemble of recurring,
//! semi-indepdendent tasks, which must exchange messages with a subset of
//! their peers to complete each stage of the computation. The tasks may exist
//! in a shared memory setting, with the parallel part of the task execution
//! being delegated to a pool of worker threads, or they may also be
//! distributed over a group of processes which exchange messages via the
//! [`Communicator`] trait. The `Communicator` sends and receives messages as
//! pure bytes (`Vec<u8>`). For this reason, distributed executors must take
//! an instance of [`Coder`] which can encode to decode from `(Automaton::Key,
//! Automaton::Message)`.
//!
//! The task group must be flat, not hierarchical. That means tasks cannot
//! spawn new asynchronous tasks into the executor while doing their work.
//! That type of flexibility is offered by other task parallel frameworks like
//! `taskflow` and also by Rayon's scheduler. The computation modeled here is
//! appropriate for grid-based physics problems, where a group of tasks is
//! advanced in discrete stages. This should not preclude time subcycling: if
//! certain tasks are updated at a higher cadence than others, the work on the
//! time-coarse tasks can be skipped, even though the executor formally
//! processes the entire task group at each fine stage.

use crate::coder::{Coder, NullCoder};
use crate::message::{Communicator, NullCommunicator};
use core::hash::Hash;
use std::collections::hash_map::{Entry, HashMap};

/// Returned by [`Automaton::receive`] to indicate whether a task is eligible
/// to be evaluated.
pub enum Status {
    Eligible,
    Ineligible,
}

impl Status {
    pub fn eligible_if(condition: bool) -> Self {
        if condition {
            Self::Eligible
        } else {
            Self::Ineligible
        }
    }
    pub fn is_eligible(&self) -> bool {
        match self {
            Self::Eligible => true,
            Self::Ineligible => false,
        }
    }
}

/// An agent in a group of compute tasks that can communicate with its peers,
/// and yields a computationally intensive data product.
///
/// The data product can be another `Automaton` to enable folding of parallel
/// executions. The model uses message passing rather than memory sharing:
/// tasks own their data, and transfer ownership of the message content to the
/// recipient. This strategy adheres to the principle of sharing memory by
/// passing messages, rather than passing messages by sharing memory. A task's
/// `value` method consumes `self`, allowing any internal memory buffers it
/// uses for computation to be transferred to the `Automaton::Value` instance
/// (which may be `Self`). Heap Heap usage in the `value` method (which is
/// generally run on a worker thread by the executor) can thus be avoided
/// entirely.
pub trait Automaton {
    /// The type of the key to uniquely identify this automaton within a
    /// group. Executors will generally require this type to be `Hash + Eq`,
    /// and also `Send` if the executor is multi-threaded.
    type Key;

    /// The type of a message to be passed between the automata. Each stage of
    /// computation requires the receipt of zero or one messages from the
    /// other automata in the group in order to yield a value.
    type Message;

    /// The type of the value yielded by this automaton. Generation of the
    /// yielded value is expected to be CPU-intensive, and may be carried on a
    /// worker thread at the discretion of the executor. For the computation
    /// to proceed requires the initial data on this task, and the messages it
    /// recieved from its peers,
    type Value;

    /// Return the key to uniquely identify this automaton within the group.
    fn key(&self) -> Self::Key;

    /// Return a list of messages to be sent to peers.
    fn messages(&self) -> Vec<(Self::Key, Self::Message)>;

    /// This method must be implemented to receive and store a message from
    /// another task. The receiving task should take ownership of the message
    /// and keep it until a call to `Self::value` is made by the executor.
    /// This method returns a `Status` enum (`Eligible` or `Ineligible`)
    /// indicating if it has now received all of its incoming messages and is
    /// ready to compute a value. This method will be invoked once by the
    /// executor for each incoming message.
    fn receive(&mut self, message: Self::Message) -> Status;

    /// Run the task. CPU-intensive work should be done in this method only.
    /// It is likely to be called on a worker thread, so it should also
    /// minimize creating or dropping memory buffers.
    fn value(self) -> Self::Value;

    /// This method may be implemented to hint the executor which worker
    /// thread it wants to run on. The executor is allowed to ignore the hint.
    fn worker_hint(&self) -> Option<usize> {
        None
    }

    /// This method may be implemented to indicate that this task is eligible
    /// immediately; it does not receive any messages.
    fn independent(&self) -> bool {
        false
    }
}

/// Execute a group of tasks in serial.
pub fn execute<I, A, K, V, M>(flow: I) -> impl Iterator<Item = V>
where
    I: IntoIterator<Item = A>,
    A: Automaton<Key = K, Value = V, Message = M>,
    K: Hash + Eq,
{
    let (eligible_sink, eligible_source) = make_channels();
    let mut comm = NullCommunicator {};
    let code = NullCoder::<(K, M)>::new();
    let work = |_: &K| 0;
    let sink = |a: A| eligible_sink.send(a).unwrap();
    coordinate(flow, &mut comm, &code, work, sink);
    eligible_source.into_iter().map(|peer: A| peer.value())
}

/// Executes a group of tasks in parallel on the Rayon thread pool.
///
/// As tasks are yielded from the input iterator (`flow`), their messages are
/// gathered and delivered to any pending tasks. Those tasks which become
/// eligible upon receiving a message are spawned onto a worker thread. This
/// function returns as soon as the input iterator is exhausted. The output
/// iterator will then yield results until all the tasks have completed in the
/// pool.
///
/// __Warning__: although it's possible to chain task group executions into
/// chunks, and collect them after several stages, something about Rayon's
/// thread pool requires at least two worker threads to be running for this to
/// work. That limitation doesn't apply to the `gridiron` thread pool.
#[cfg(feature = "rayon")]
pub fn execute_rayon<'a, I, A, K, V, M>(
    scope: &rayon::Scope<'a>,
    flow: I,
) -> impl Iterator<Item = V>
where
    I: IntoIterator<Item = A>,
    A: Send + Automaton<Key = K, Value = V, Message = M> + 'a,
    K: Hash + Eq,
    V: Send + 'a,
{
    let (eligible_sink, eligible_source) = make_channels();
    let mut comm = NullCommunicator {};
    let code = NullCoder::<(K, M)>::new();
    let work = |_: &K| 0;
    let sink = |a: A| {
        let eligible_sink = eligible_sink.clone();
        scope.spawn(move |_| {
            eligible_sink.send(a.value()).unwrap();
        })
    };
    coordinate(flow, &mut comm, &code, work, sink);
    eligible_source.into_iter()
}

/// Executes a group of tasks in parallel using `gridiron`'s thread pool.
///
/// As tasks are yielded from the input iterator (`flow`), their messages are
/// gathered and delivered to any pending tasks. Those tasks which become
/// eligible upon receiving a message are spawned onto a worker thread. This
/// function returns as soon as the input iterator is exhausted. The output
/// iterator will then yield results until all the tasks have completed in the
/// pool.
pub fn execute_thread_pool<I, A, K, V, M>(
    pool: &crate::thread_pool::ThreadPool,
    flow: I,
) -> impl Iterator<Item = V>
where
    I: IntoIterator<Item = A>,
    A: 'static + Send + Automaton<Key = K, Value = V, Message = M>,
    K: 'static + Hash + Eq,
    V: 'static + Send,
{
    let (eligible_sink, eligible_source) = make_channels();
    let mut comm = NullCommunicator {};
    let code = NullCoder::<(K, M)>::new();
    let work = |_: &K| 0;
    let sink = |a: A| {
        let eligible_sink = eligible_sink.clone();
        pool.spawn_on(a.worker_hint(), move || {
            eligible_sink.send(a.value()).unwrap();
        })
    };
    coordinate(flow, &mut comm, &code, work, sink);
    eligible_source.into_iter()
}

/// Executes a group of compute tasks using a distributed communicator, and an
/// optional pool of worker threads. If no pool is given, the executions are
/// done synchronously.
pub fn execute_comm<Comm, Code, Work, I, A, K, V, M>(
    comm: &mut Comm,
    code: &Code,
    work: &Work,
    pool: Option<&crate::thread_pool::ThreadPool>,
    flow: I,
) -> impl Iterator<Item = V>
where
    Comm: Communicator,
    Code: Coder<Type = (A::Key, A::Message)>,
    Work: Fn(&K) -> usize,
    I: IntoIterator<Item = A>,
    A: 'static + Send + Automaton<Key = K, Value = V, Message = M>,
    K: 'static + Hash + Eq,
    V: 'static + Send,
{
    let (eligible_sink, eligible_source) = make_channels();
    let sink = |a: A| match pool {
        Some(pool) => {
            let eligible_sink = eligible_sink.clone();
            pool.spawn_on(a.worker_hint(), move || {
                eligible_sink.send(a.value()).unwrap();
            })
        }
        None => eligible_sink.send(a.value()).unwrap(),
    };
    coordinate(flow, comm, code, work, sink);
    eligible_source.into_iter()
}

fn coordinate<Comm, Code, Work, Sink, I, A, K, V>(
    flow: I,
    comm: &mut Comm,
    code: &Code,
    work: Work,
    sink: Sink,
) where
    Comm: Communicator,
    Code: Coder<Type = (A::Key, A::Message)>,
    Work: Fn(&K) -> usize,
    Sink: Fn(A),
    I: IntoIterator<Item = A>,
    A: Automaton<Key = K, Value = V>,
    K: Hash + Eq,
{
    let mut seen: HashMap<K, A> = HashMap::new();
    let mut undelivered = HashMap::new();

    for mut a in flow {
        // For each of A's messages, either deliver it to the recipient peer,
        // if the peer has already been seen, or otherwise put it in the
        // undelivered box.
        //
        // If any of the recipient peers became eligible upon receiving a
        // message, then send those peers off to be executed.
        for (dest, data) in a.messages() {
            if work(&dest) == comm.rank() {
                match seen.entry(dest) {
                    Entry::Occupied(mut entry) => {
                        if let Status::Eligible = entry.get_mut().receive(data) {
                            sink(entry.remove())
                        }
                    }
                    Entry::Vacant(none) => {
                        undelivered
                            .entry(none.into_key())
                            .or_insert_with(Vec::new)
                            .push(data);
                    }
                }
            } else {
                comm.send(work(&dest), code.encode(&(dest, data)))
            }
        }

        // Deliver any messages addressed to A that had arrived previously. If
        // A is eligible after receiving its messages, then send it off to be
        // executed. Otherwise mark it as seen and process the next automaton.
        let eligible = undelivered
            .remove_entry(&a.key())
            .map_or(false, |(_, messages)| {
                messages.into_iter().any(|m| a.receive(m).is_eligible())
            });

        if eligible || a.independent() {
            sink(a)
        } else {
            seen.insert(a.key(), a);
        }
    }
    assert!(undelivered.is_empty());

    // Receive messages from peers until all tasks have been evaluated.
    while !seen.is_empty() {
        let (dest, data) = code.decode(&comm.recv());
        match seen.entry(dest) {
            Entry::Occupied(mut entry) => {
                if let Status::Eligible = entry.get_mut().receive(data) {
                    sink(entry.remove())
                }
            }
            Entry::Vacant(_) => {
                panic!(
                    "message received for a task that has not been seen or was already evaluated"
                )
            }
        }
    }
    comm.next_time_stamp();
}

#[cfg(feature = "crossbeam_channel")]
fn make_channels<T>() -> (crossbeam_channel::Sender<T>, crossbeam_channel::Receiver<T>) {
    crossbeam_channel::unbounded()
}

#[cfg(not(feature = "crossbeam_channel"))]
fn make_channels<T>() -> (std::sync::mpsc::Sender<T>, std::sync::mpsc::Receiver<T>) {
    std::sync::mpsc::channel()
}
