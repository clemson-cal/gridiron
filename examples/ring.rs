use gridiron::message::comm::Communicator;
use gridiron::message::tcp::TcpCommunicator;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Range;
use std::thread;

fn peer(rank: usize) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000 + rank as u16)
}

fn main() {
    let ranks: Range<usize> = 0..8;
    let peers: Vec<_> = ranks.clone().map(|rank| peer(rank)).collect();
    let comms: Vec<_> = ranks
        .clone()
        .map(|rank| TcpCommunicator::new(rank, peers.clone()))
        .collect();
    let procs: Vec<_> = comms
        .into_iter()
        .map(|comm| {
            thread::spawn(move || {
                let dest = (comm.rank() + 1) % comm.size();
                let message = format!("hello from {}", comm.rank());
                comm.send(dest, message.into_bytes());

                let received = comm.recv();
                println! {
                    "{} received '{}'",
                    comm.rank(),
                    String::from_utf8(received).unwrap()
                };
            })
        })
        .collect();

    for process in procs {
        process.join().unwrap()
    }
}
