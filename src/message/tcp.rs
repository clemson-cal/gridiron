//! Provides a message-passing communicator based on TCP sockets.
//! 
//! TCP is a connection-oriented protocol, which means that a connection must
//! be established between the sending and receiving ends of the socket in
//! order to read from or write to a stream.

use super::comm::Communicator;
use super::util;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

type Sender = mpsc::Sender<(usize, Vec<u8>)>;
type Receiver = mpsc::Receiver<(usize, Vec<u8>)>;

pub struct TcpCommunicator {
    rank: usize,
    num_peers: usize,
    listener: TcpListener,
    send_sink: Option<mpsc::Sender<(usize, Vec<u8>)>>,
    send_thread: Option<thread::JoinHandle<()>>,
}

impl TcpCommunicator {
    pub fn new(rank: usize, peers: Vec<SocketAddr>) -> Self {
        let listener = TcpListener::bind(peers[rank]).unwrap();
        let num_peers = peers.len();
        let (send_sink, recv_sink): (Sender, Receiver) = mpsc::channel();
        let send_thread = thread::spawn(move || {
            for (rank, message) in recv_sink {
                let mut stream = TcpStream::connect(peers[rank]).unwrap();
                stream.write_all(&message.len().to_le_bytes()).unwrap();
                stream.write_all(&message).unwrap();
            }
        });
        Self {
            rank,
            num_peers,
            listener,
            send_sink: Some(send_sink),
            send_thread: Some(send_thread),
        }
    }
}

impl Communicator for TcpCommunicator {
    fn rank(&self) -> usize {
        self.rank
    }

    fn size(&self) -> usize {
        self.num_peers
    }

    fn send(&self, rank: usize, message: Vec<u8>) {
        self.send_sink
            .as_ref()
            .unwrap()
            .send((rank, message))
            .unwrap()
    }

    fn recv(&self) -> Vec<u8> {
        let (mut stream, _) = self.listener.accept().unwrap();
        let size = util::read_usize(&mut stream);
        util::read_bytes_vec(&mut stream, size)
    }

    fn next_time_stamp(&mut self) {
        unimplemented!("this Communicator impl is deprecated")
    }
}

impl Drop for TcpCommunicator {
    fn drop(&mut self) {
        self.send_sink.take().unwrap();
        self.send_thread.take().unwrap().join().unwrap();
    }
}
