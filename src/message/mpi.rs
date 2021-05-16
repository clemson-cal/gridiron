#![cfg(feature = "mpi")]
use crate::message::comm;
use crate::mpi;
use std::sync::mpsc;
use std::thread;

type Sender = mpsc::Sender<(usize, i32, Vec<u8>)>;
type Receiver = mpsc::Receiver<(usize, i32, Vec<u8>)>;

pub struct MpiCommunicator {
    send_sink: Option<Sender>,
    send_thread: Option<thread::JoinHandle<()>>,
    time_stamp: i32,
}

impl MpiCommunicator {
    pub fn new() -> Self {
        let (send_sink, recv_sink): (Sender, Receiver) = mpsc::channel();
        let send_thread = thread::spawn(move || {
            for (rank, time_stamp, message) in recv_sink {
                unsafe {
                    mpi::send(
                        message.as_ptr(),
                        message.len() as i32,
                        rank as i32,
                        time_stamp as i32);
                }
            }
        });
        Self {
            send_sink: Some(send_sink),
            send_thread: Some(send_thread),
            time_stamp: 0,
        }
    }
}

impl Default for MpiCommunicator {
    fn default() -> Self {
        Self::new()
    }
}

impl comm::Communicator for MpiCommunicator {
    fn rank(&self) -> usize {
        unsafe {
            mpi::comm_rank() as usize
        }
    }

    fn size(&self) -> usize {
        unsafe {
            mpi::comm_size() as usize
        }
    }

    fn send(&self, rank: usize, message: Vec<u8>) {
        self.send_sink
            .as_ref()
            .unwrap()
            .send((rank, self.time_stamp, message))
            .unwrap()
    }

    fn recv(&self) -> Vec<u8> {
        unsafe {
            let status = mpi::probe_tag(self.time_stamp as i32);
            let mut buffer = vec![0; status.count as usize];
            mpi::recv(buffer.as_mut_ptr(), status.count, status.source, status.tag);
            buffer
        }
    }

    fn next_time_stamp(&mut self) {
        self.time_stamp += 1;
    }
}

impl Drop for MpiCommunicator {
    fn drop(&mut self) {
        self.send_sink.take().unwrap();
        self.send_thread.take().unwrap().join().unwrap();
    }
}
