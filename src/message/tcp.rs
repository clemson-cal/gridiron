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
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_nanos(100);
type SendS = mpsc::Sender<(SocketAddr, Vec<u8>, usize)>;
type SendR = mpsc::Receiver<(SocketAddr, Vec<u8>, usize)>;
type RecvS = mpsc::Sender<(Vec<u8>, usize)>;
type RecvR = mpsc::Receiver<(Vec<u8>, usize)>;

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
    alive: Arc<AtomicBool>,
    send_s: Option<SendS>,
    recv_r: Option<RecvR>,
    send_thread: Option<thread::JoinHandle<()>>,
    recv_thread: Option<thread::JoinHandle<()>>,
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
        let (send_s, send_r): (SendS, SendR) = mpsc::channel();
        let (recv_s, recv_r): (RecvS, RecvR) = mpsc::channel();
        let alive = Arc::new(AtomicBool::new(true));
        let keep_receiving = alive.clone();

        // This thread takes the receiving end of the message sender channel.
        let send_thread = thread::spawn(move || {
            let mut streams = HashMap::new();
            for (address, message, tag) in send_r {
                let stream = streams
                    .entry(address)
                    .or_insert_with(|| TcpStream::connect(address).unwrap());
                stream.write_all(&message.len().to_le_bytes()).unwrap();
                stream.write_all(&tag.to_le_bytes()).unwrap();
                stream.write_all(&message).unwrap();
            }
        });
        listener.set_nonblocking(true).unwrap();

        // This thread takes the sending end of the message receiving channel.
        let recv_thread = thread::spawn(move || {
            let mut streams = Vec::new();
            while keep_receiving.load(Ordering::Relaxed) {
                for stream in &mut streams {
                    if let Some((message, tag)) = Self::poll(stream) {
                        recv_s.send((message, tag)).unwrap();
                    }
                }
                if let Ok((stream, _)) = listener.accept() {
                    stream.set_read_timeout(Some(READ_TIMEOUT)).unwrap();
                    streams.push(stream)
                }
            }
        });

        Self {
            alive,
            send_s: Some(send_s),
            recv_r: Some(recv_r),
            send_thread: Some(send_thread),
            recv_thread: Some(recv_thread),
        }
    }

    /// Initiates a blocking receive from any peer.
    pub fn recv(&mut self) -> (Vec<u8>, usize) {
        self.recv_r.as_ref().unwrap().recv().unwrap()
    }

    /// Initiates a non-blocking send to a particular peer.
    pub fn send(&mut self, peer: SocketAddr, message: Vec<u8>, tag: usize) {
        self.send_s
            .as_ref()
            .unwrap()
            .send((peer, message, tag))
            .unwrap()
    }
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        self.alive.swap(false, Ordering::Relaxed);
        self.send_s.take().unwrap();
        self.send_thread.take().unwrap().join().unwrap();
        self.recv_thread.take().unwrap().join().unwrap();
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
