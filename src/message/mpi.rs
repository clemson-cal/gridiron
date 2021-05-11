use super::comm;
use mpi::point_to_point::{Destination, Source};
use mpi::topology::{Communicator, SystemCommunicator};
use mpi::collective::CommunicatorCollectives;
use std::sync::mpsc;
use std::thread;

type Sender = mpsc::Sender<(usize, Vec<u8>)>;
type Receiver = mpsc::Receiver<(usize, Vec<u8>)>;

pub struct MpiCommunicator {
    comm: SystemCommunicator,
    send_sink: Option<mpsc::Sender<(usize, Vec<u8>)>>,
    send_thread: Option<thread::JoinHandle<()>>,
}

impl MpiCommunicator {
    pub fn new(comm: SystemCommunicator) -> Self {
        let (send_sink, recv_sink): (Sender, Receiver) = mpsc::channel();
        let send_thread = thread::spawn(move || {
            for (rank, message) in recv_sink {
                comm.process_at_rank(rank as i32).send(&message[..])
            }
        });
        Self {
            comm: comm,
            send_sink: Some(send_sink),
            send_thread: Some(send_thread),
        }
    }

    pub fn barrier(&self) {
        self.comm.barrier()
    }
}

impl comm::Communicator for MpiCommunicator {
    fn rank(&self) -> usize {
        self.comm.rank() as usize
    }
    fn size(&self) -> usize {
        self.comm.size() as usize
    }
    fn send(&self, rank: usize, message: Vec<u8>) {
        self.send_sink
            .as_ref()
            .unwrap()
            .send((rank, message))
            .unwrap()
    }
    fn recv(&self) -> Vec<u8> {
        self.comm.any_process().receive_vec().0
    }
}

impl Drop for MpiCommunicator {
    fn drop(&mut self) {
        self.send_sink.take().unwrap();
        self.send_thread.take().unwrap().join().unwrap();
    }
}
