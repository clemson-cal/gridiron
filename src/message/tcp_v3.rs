//! Provides a message-passing communicator based on TCP sockets.
//!
//! TCP is a connection-oriented protocol, which means that a connection must
//! be established between the sending and receiving ends of the socket in
//! order to read from or write to a stream.

use super::comm::Communicator;
use super::util;
use std::cell::RefCell;
use std::collections::hash_map::{Entry, HashMap};
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

type SendS = mpsc::Sender<(SocketAddr, Vec<u8>, usize)>;
type SendR = mpsc::Receiver<(SocketAddr, Vec<u8>, usize)>;
type RecvS = mpsc::Sender<(Vec<u8>, usize)>;
type RecvR = mpsc::Receiver<(Vec<u8>, usize)>;

#[derive(Clone, Copy)]
pub enum SendThreads {
    Single,
    OnePerSocket,
}

/// Maintains a cache of ingoing and outgoing TCP connections.
///
/// This object facilitates non-blocking sends and blocking receives from any
/// peer. Communicating with a remote peer only opens a new connection on the
/// on the first call; subsequent communications with that peer reuse the
/// cached connection. It also facilitates receiving a message from any of the
/// open connections. In this implementation, there is one thread per incoming
/// connection, and one thread per outgoing connection.
///
/// __Note__: running threads are shamefully stranded. The shutdown logic will
/// complicate the code a little, so I don't want to add it until (unless) the
/// scheme is optimized.
pub struct ConnectionPool {
    send_s: Option<SendS>,
    recv_r: Option<RecvR>,
}

impl ConnectionPool {
    /// Creates a `ConnectionPool` from a `TcpListener`. The listener is
    /// placed in a non-blocking accept mode, so the pre-existing blocking
    /// mode is overwritten.
    pub fn from_listener(listener: TcpListener, mode: SendThreads) -> Self {
        let (send_s, send_r): (SendS, SendR) = mpsc::channel();
        let (recv_s, recv_r): (RecvS, RecvR) = mpsc::channel();

        thread::spawn(move || {
            let mut streams = HashMap::new();
            let mut channels: HashMap<SocketAddr, RecvS> = HashMap::new();
            for (address, message, tag) in send_r {
                match mode {
                    SendThreads::Single => {
                        let mut stream = streams
                            .entry(address)
                            .or_insert_with(|| TcpStream::connect(address).unwrap());
                        Self::write_message(&mut stream, message, tag)
                    }
                    SendThreads::OnePerSocket => match channels.entry(address) {
                        Entry::Vacant(entry) => {
                            let (s, r) = mpsc::channel();
                            s.send((message, tag)).unwrap();
                            entry.insert(s);
                            thread::spawn(move || {
                                let mut stream = TcpStream::connect(address).unwrap();
                                for (message, tag) in r {
                                    Self::write_message(&mut stream, message, tag)
                                }
                            });
                        }
                        Entry::Occupied(entry) => {
                            entry.get().send((message, tag)).unwrap();
                        }
                    },
                }
            }
        });

        thread::spawn(move || {
            for mut stream in listener.incoming().map(Result::unwrap) {
                let recv_s = recv_s.clone();
                thread::spawn(move || loop {
                    recv_s.send(Self::read_message(&mut stream)).unwrap()
                });
            }
        });

        Self {
            send_s: Some(send_s),
            recv_r: Some(recv_r),
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

    fn write_message(stream: &mut TcpStream, message: Vec<u8>, tag: usize) {
        let len = message.len().to_le_bytes();
        let tag = tag.to_le_bytes();
        stream.write_all(&len).unwrap();
        stream.write_all(&tag).unwrap();
        stream.write_all(&message).unwrap();
    }

    fn read_message(stream: &mut TcpStream) -> (Vec<u8>, usize) {
        let len = util::read_usize(stream);
        let tag = util::read_usize(stream);
        (util::read_bytes_vec(stream, len), tag)
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
    pub fn new(rank: usize, peers: Vec<SocketAddr>, mode: SendThreads) -> Self {
        let listener = TcpListener::bind(peers[rank]).unwrap();
        let connections = RefCell::new(ConnectionPool::from_listener(listener, mode));
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
