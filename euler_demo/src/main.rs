pub mod hydro;
pub mod solvers;

use crate::hydro::euler2d::Primitive;
use crate::solvers::euler2d_pcm::{Mesh, PatchUpdate};
use clap::{AppSettings, Clap};
use gridiron::automaton::{self, Automaton};
use gridiron::coder::Coder;
use gridiron::index_space::range2d;
use gridiron::meshing::{self, GraphTopology};
use gridiron::message::{comm::Communicator, null::NullCommunicator, tcp};
use gridiron::patch::Patch;
use gridiron::rect_map::{Rectangle, RectangleMap};
use gridiron::thread_pool;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Range;
use std::thread;

#[derive(Debug, Clone, Clap)]
#[clap(version = "1.0", author = "J. Zrake <jzrake@clemson.edu>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short = 't', long, default_value = "1")]
    num_threads: usize,

    #[clap(
        short = 's',
        long,
        default_value = "serial",
        about = "serial|stupid|rayon|tcp|mpi"
    )]
    strategy: String,

    #[clap(short = 'm', long)]
    multiple_send_threads: bool,

    #[clap(short = 'n', long, default_value = "1000")]
    grid_resolution: usize,

    #[clap(short = 'b', long, default_value = "100")]
    block_size: usize,

    #[clap(short = 'f', long, default_value = "1")]
    fold: usize,

    #[clap(long, default_value = "0.1")]
    tfinal: f64,
}

/// The initial model
struct Model {}

impl Model {
    fn primitive_at(&self, position: (f64, f64)) -> Primitive {
        let (x, y) = position;
        let r = (x * x + y * y).sqrt();

        if r < 0.24 {
            Primitive::new(1.0, 0.0, 0.0, 1.0)
        } else {
            Primitive::new(0.1, 0.0, 0.0, 0.125)
        }
    }
}

/// The simulation solution state
#[derive(serde::Serialize)]
struct State {
    time: f64,
    iteration: u64,
    primitive: Vec<Patch>,
}

impl State {
    fn new(mesh: &Mesh, bs: usize) -> Self {
        let model = Model {};
        let initial_data = |i| model.primitive_at(mesh.cell_center(i)).as_array();
        let primitive = mesh_rectangles(bs, mesh)
            .map(|rect| Patch::from_vector_function(0, rect, initial_data))
            .collect();

        Self {
            iteration: 0,
            time: 0.0,
            primitive,
        }
    }
}

struct CborCoder<A> {
    phantom: std::marker::PhantomData<A>,
}

impl<A> CborCoder<A> {
    fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData::<A> {},
        }
    }
}

impl<A, K, M> Coder for CborCoder<A>
where
    A: Automaton<Key = K, Message = M>,
    K: serde::Serialize + serde::Deserialize<'static>,
    M: serde::Serialize + serde::Deserialize<'static>,
{
    type Type = (K, M);

    fn encode(&self, inst: Self::Type) -> Vec<u8> {
        let mut buffer = Vec::new();
        ciborium::ser::into_writer(&inst, &mut buffer).unwrap();
        buffer
    }

    fn decode(&self, data: Vec<u8>) -> Self::Type {
        ciborium::de::from_reader(data.as_slice()).unwrap()
    }
}

fn mesh_rectangles(bs: usize, mesh: &Mesh) -> impl Iterator<Item = Rectangle<i64>> {
    let bs = bs as i64;
    let ni = mesh.size.0 as i64 / bs;
    let nj = mesh.size.1 as i64 / bs;

    range2d(0..ni, 0..nj)
        .into_iter()
        .map(move |(i, j)| (i * bs..(i + 1) * bs, j * bs..(j + 1) * bs))
}

#[allow(unused)]
fn work_assignment(
    bs: usize,
    mesh: &Mesh,
    comm: &impl Communicator,
) -> HashMap<Rectangle<i64>, usize> {
    // let block_dims = meshing::block_dims(comm.size(), 2);
    // let bi = block_dims[0];
    // let bj = block_dims[1];

    use gridiron::rect_map::RectangleMap;
    let edges_i: Vec<_> = std::iter::once(0)
        .chain(meshing::partition(100, 8).into_iter().scan(0, |a, b| {
            *a += b;
            Some(*a)
        }))
        .collect();
    let ranges_i = edges_i.windows(2).map(|s| s[0] as i64..s[1] as i64);

    let edges_j: Vec<_> = std::iter::once(0)
        .chain(meshing::partition(100, 8).into_iter().scan(0, |a, b| {
            *a += b;
            Some(*a)
        }))
        .collect();
    let ranges_j = edges_j.windows(2).map(|s| s[0] as i64..s[1] as i64);

    let subgrids: RectangleMap<_, _> = ranges_i
            .map(move |di| ranges_j.clone().map(move |dj| (di.clone(), dj)))
            .flatten()
            .map(|rect| (rect, 0))
            .collect();

    let ni = mesh.size.0 / bs;
    let nj = mesh.size.1 / bs;
    let blocks_per_peer = (ni * nj) / comm.size();

    mesh_rectangles(bs, mesh)
        .enumerate()
        .map(|(n, rect)| (rect, n / blocks_per_peer))
        .collect()
}

enum Execution {
    Serial,
    Stupid(thread_pool::ThreadPool),
    Rayon(rayon::ThreadPool),
    Distributed,
}

fn run(opts: Opts, mut comm: impl Communicator) {
    // step 1: find the global mesh size: opts.grid_resolution
    // step 2: find the total number of blocks: opts.num_blocks^2
    // step 3: find the number of peers: comm.size()
    // step 4: use meshing::block_dims to create a 2d array of peers
    // step 5: map the 2d array of peers indexes (i, j) to rectangles
    //         in the global index space
    // step 6: create a shallow copy of the mesh, a rectangle map of
    //         rectangles representing the grid blocks
    // step 7: compute the adjacency list from the shallow mesh
    // step 8:

    let code = CborCoder::<PatchUpdate>::new();
    let mesh = Mesh {
        area: (-1.0..1.0, -1.0..1.0),
        size: (opts.grid_resolution, opts.grid_resolution),
    };
    let work = work_assignment(opts.block_size, &mesh, &comm);
    let State {
        mut iteration,
        mut time,
        primitive,
    } = State::new(&mesh, opts.block_size);

    let primitive_map: RectangleMap<_, _> = primitive
        .into_iter()
        .map(|p| (p.high_resolution_rect(), p))
        .collect();
    let dt = mesh.cell_spacing().0 * 0.1;
    let edge_list = primitive_map.adjacency_list(1);
    let primitive: Vec<_> = primitive_map.into_iter().map(|(_, prim)| prim).collect();

    let mut task_list: Vec<_> = primitive
        .into_iter()
        .filter(|patch| work[&patch.high_resolution_rect()] == comm.rank())
        .map(|patch| PatchUpdate::new(patch, mesh.clone(), dt, None, &edge_list))
        .collect();

    if opts.grid_resolution.pow(2) % comm.size() != 0 {
        panic!("the number of peers must divide the number of tasks");
    }

    if opts.grid_resolution % opts.block_size != 0 {
        if comm.rank() == 0 {
            eprintln!("Error: block size must divide the grid resolution");
        }
        return;
    }

    if vec!["serial", "mpi"].contains(&opts.strategy.as_str()) && opts.num_threads != 1 {
        if comm.rank() == 0 {
            eprintln!("Error: strategy option requires --num-threads=1");
        }
        return;
    }

    let executor = match opts.strategy.as_str() {
        "serial" => Execution::Serial,
        "stupid" => Execution::Stupid(thread_pool::ThreadPool::new(opts.num_threads)),
        "rayon" => Execution::Rayon(
            rayon::ThreadPoolBuilder::new()
                .num_threads(opts.num_threads)
                .build()
                .unwrap(),
        ),
        "tcp" | "mpi" => Execution::Distributed,
        _ => {
            eprintln!("Error: --strategy options are [serial|stupid|rayon|tcp|mpi]");
            return;
        }
    };

    println!("rank {} working on {} blocks", comm.rank(), task_list.len());

    while time < opts.tfinal {
        let start = std::time::Instant::now();

        for _ in 0..opts.fold {
            task_list = match executor {
                Execution::Serial => automaton::execute(task_list).collect(),
                Execution::Stupid(ref pool) => {
                    automaton::execute_thread_pool(&pool, task_list).collect()
                }
                Execution::Rayon(ref pool) => pool
                    .scope(|scope| automaton::execute_rayon(scope, task_list))
                    .collect(),
                Execution::Distributed => {
                    automaton::execute_comm(&mut comm, &code, &work, None, task_list).collect()
                }
            };
            iteration += 1;
            time += dt;
        }
        let step_seconds = start.elapsed().as_secs_f64() / opts.fold as f64;
        let mzps = mesh.total_zones() as f64 / 1e6 / step_seconds;

        if comm.rank() == 0 {
            println! {
                "[{}] t={:.3} Mzps={:.2}",
                iteration,
                time,
                mzps,
            };
        }
    }

    let primitive = task_list
        .into_iter()
        .map(|block| block.primitive())
        .collect();

    let state = State {
        iteration,
        time,
        primitive,
    };

    let file = std::fs::File::create(format! {"state.{:04}.cbor", comm.rank()}).unwrap();
    let mut buffer = std::io::BufWriter::new(file);
    ciborium::ser::into_writer(&state, &mut buffer).unwrap();
}

fn peer(rank: usize) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7070 + rank as u16)
}

fn main_tcp(opts: Opts) {
    let ranks: Range<usize> = 0..opts.num_threads;
    let peers: Vec<_> = ranks.clone().map(|rank| peer(rank)).collect();
    let comms: Vec<_> = ranks
        .clone()
        .map(|rank| tcp::TcpCommunicator::new(rank, peers.clone()))
        .collect();
    let procs: Vec<_> = comms
        .into_iter()
        .map(|comm| {
            let opts = opts.clone();
            thread::spawn(|| run(opts, comm))
        })
        .collect();

    for process in procs {
        process.join().unwrap()
    }
}

#[cfg(feature = "mpi")]
fn main_mpi(opts: Opts) {
    unsafe {
        mpi::init();
    }
    let comm = message::mpi::MpiCommunicator::new();
    run(opts, comm);
    unsafe {
        mpi::finalize();
    }
}

#[cfg(not(feature = "mpi"))]
fn main_mpi(_opts: Opts) {
    println!("Error: compiled without MPI support");
}

fn main_mt(opts: Opts) {
    run(opts, NullCommunicator::new())
}

fn main() {
    let opts = Opts::parse();

    match opts.strategy.as_str() {
        "mpi" => main_mpi(opts),
        "tcp" => main_tcp(opts),
        _ => main_mt(opts),
    }
}
