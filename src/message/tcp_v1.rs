//! Provides a message-passing communicator based on TCP sockets.
//!
//! TCP is a connection-oriented protocol, which means that a connection must
//! be established between the sending and receiving ends of the socket in
//! order to read from or write to a stream.

use super::comm::Communicator;
use super::util;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_nanos(100);
type Sender = mpsc::Sender<(SocketAddr, Vec<u8>, usize)>;
type Receiver = mpsc::Receiver<(SocketAddr, Vec<u8>, usize)>;

/// Maintains a cache of ingoing and outgoing TCP connections.
///
/// This object facilitates non-blocking sends and blocking receives from any
/// peer. Communicating with a remote peer only opens a new connection on the
/// on the first call; subsequent communications with that peer reuse the
/// cached connection. It also facilitates receiving a message from any of the
/// open connections. When no message can be read from one of the cached
/// connections, it will try to accept an incoming connection on a short
/// timeout.
pub struct ConnectionPool {
    listener: TcpListener,
    streams: Vec<TcpStream>,
    message_sender: Option<Sender>,
    message_thread: Option<thread::JoinHandle<()>>,
}

impl ConnectionPool {
    fn poll(stream: &mut TcpStream) -> Option<(Vec<u8>, usize)> {
        util::read_usize_non_blocking(stream).map(|len| {
            let tag = util::read_usize(stream);
            (util::read_bytes_vec(stream, len), tag)
        })
    }

    /// Creates a `ConnectionPool` from a `TcpListener`. The listener is
    /// placed in a non-blocking accept mode, so the pre-existing blocking
    /// mode is overwritten.
    pub fn from_listener(listener: TcpListener) -> Self {
        let (message_sender, message_recv): (Sender, Receiver) = mpsc::channel();
        let message_thread = thread::spawn(move || {
            let mut streams = HashMap::new();
            for (address, message, tag) in message_recv {
                let stream = streams
                    .entry(address)
                    .or_insert_with(|| TcpStream::connect(address).unwrap());
                stream.write_all(&message.len().to_le_bytes()).unwrap();
                stream.write_all(&tag.to_le_bytes()).unwrap();
                stream.write_all(&message).unwrap();
            }
        });
        listener.set_nonblocking(true).unwrap();
        Self {
            listener,
            streams: Vec::new(),
            message_sender: Some(message_sender),
            message_thread: Some(message_thread),
        }
    }

    /// Initiates a blocking receive from any peer.
    pub fn recv(&mut self) -> (Vec<u8>, usize) {
        loop {
            for stream in &mut self.streams {
                if let Some(message) = Self::poll(stream) {
                    return message;
                }
            }
            if let Ok((stream, _)) = self.listener.accept() {
                stream.set_read_timeout(Some(TIMEOUT)).unwrap();
                self.streams.push(stream)
            }
        }
    }

    /// Initiates a non-blocking send to a particular peer.
    pub fn send(&mut self, peer: SocketAddr, message: Vec<u8>, tag: usize) {
        self.message_sender
            .as_ref()
            .unwrap()
            .send((peer, message, tag))
            .unwrap()
    }
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        self.message_sender.take().unwrap();
        self.message_thread.take().unwrap().join().unwrap();
    }
}

pub struct TcpCommunicator {
    rank: usize,
    peers: Vec<SocketAddr>,
    connections: RefCell<ConnectionPool>,
    undelivered: RefCell<Vec<(Vec<u8>, usize)>>,
    time_stamp: usize,
}

impl TcpCommunicator {
    pub fn new(rank: usize, peers: Vec<SocketAddr>) -> Self {
        let listener = TcpListener::bind(peers[rank]).unwrap();
        let connections = RefCell::new(ConnectionPool::from_listener(listener));
        Self {
            rank,
            peers,
            connections,
            undelivered: RefCell::new(Vec::new()),
            time_stamp: 0,
        }
    }
}

impl Communicator for TcpCommunicator {
    fn rank(&self) -> usize {
        self.rank
    }

    fn size(&self) -> usize {
        self.peers.len()
    }

    fn send(&self, rank: usize, message: Vec<u8>) {
        self.connections
            .borrow_mut()
            .send(self.peers[rank], message, self.time_stamp)
    }

    fn recv(&self) -> Vec<u8> {
        let mut connections = self.connections.borrow_mut();
        let mut undelivered = self.undelivered.borrow_mut();
        match undelivered
            .iter()
            .position(|(_, tag)| tag == &self.time_stamp)
        {
            Some(index) => undelivered.remove(index).0,
            None => loop {
                let (message, tag) = connections.recv();
                if tag != self.time_stamp {
                    undelivered.push((message, tag))
                } else {
                    return message;
                }
            },
        }
    }

    fn next_time_stamp(&mut self) {
        self.time_stamp += 1;
    }
}