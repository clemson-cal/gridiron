use std::cell;
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

#[cfg(feature = "crossbeam_channel")]
type JobSender = crossbeam_channel::Sender<Job>;

#[cfg(not(feature = "crossbeam_channel"))]
type JobSender = std::sync::mpsc::Sender<Job>;

struct Worker {
    handle: Option<thread::JoinHandle<()>>,
    sender: Option<JobSender>,
}

/// A minimal thread pool implementation with core affinity. No effort is made
/// to schedule jobs intelligently, it just goes round-robin. Jobs must be
/// `'static`.
pub struct ThreadPool {
    workers: Vec<Worker>,
    current_worker_id: cell::Cell<usize>,
}

impl ThreadPool {
    /// Creates a new thread pool with at most the given number of threads. If
    /// the system has fewer physical CPU cores than the requested number of
    /// threads, then the number of cores is unsed instead.
    pub fn new(num_threads: usize) -> Self {
        ThreadPool {
            workers: Self::make_workers(num_threads),
            current_worker_id: cell::Cell::new(0),
        }
    }

    /// Returns the number of worker threads in the pool.
    pub fn num_threads(&self) -> usize {
        self.workers.len()
    }

    /// Spawnd a new job into the pool. Job submissions go cyclically to the
    /// workers: if worker `n` gets this job, then worker `(n + 1) %
    /// num_workers` gets the next one.
    pub fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.spawn_on(None, job)
    }

    /// Spawns a job onto the worker thread with the given index, if it is
    /// `Some`. The current worker index is not incremented. If the worker
    /// index is `None`, then the job is run on the current worker index,
    /// which is then incremented.
    pub fn spawn_on<F>(&self, worker_id: Option<usize>, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let worker_id = if let Some(worker_id) = worker_id {
            worker_id
        } else {
            let worker_id = self.current_worker_id.get();
            self.current_worker_id
                .set((worker_id + 1) % self.num_threads());
            worker_id
        };
        self.workers[worker_id]
            .sender
            .as_ref()
            .unwrap()
            .send(Box::new(job))
            .unwrap();
    }
}

impl ThreadPool {
    #[cfg(feature = "crossbeam_channel")]
    fn make_channels() -> (crossbeam_channel::Sender<Job>, crossbeam_channel::Receiver<Job>) {
        crossbeam_channel::unbounded()
    }

    #[cfg(not(feature = "crossbeam_channel"))]
    fn make_channels() -> (std::sync::mpsc::Sender<Job>, std::sync::mpsc::Receiver<Job>) {
        std::sync::mpsc::channel()
    }

    #[cfg(feature = "core_affinity")]
    fn make_workers(num_threads: usize) -> Vec<Worker> {
        use core_affinity::{get_core_ids, set_for_current};
        get_core_ids()
            .unwrap()
            .into_iter()
            .take(num_threads)
            .map(|core_id| {
                let (sender, receiver) = Self::make_channels();
                let handle = thread::spawn(move || {
                    set_for_current(core_id);
                    for job in receiver {
                        job()
                    }
                });
                Worker {
                    handle: Some(handle),
                    sender: Some(sender),
                }
            })
            .collect()
    }

    #[cfg(not(feature = "core_affinity"))]
    fn make_workers(num_threads: usize) -> Vec<Worker> {
        (0..num_threads)
            .map(|_| {
                let (sender, receiver) = Self::make_channels();
                let handle = thread::spawn(move || {
                    for job in receiver {
                        job()
                    }
                });
                Worker {
                    handle: Some(handle),
                    sender: Some(sender),
                }
            })
            .collect()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
