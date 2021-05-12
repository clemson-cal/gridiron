use clap::{AppSettings, Clap};
use gridiron::automaton::{self, Automaton};
use gridiron::coder::Coder;
use gridiron::hydro::euler2d::Primitive;
use gridiron::index_space::range2d;
use gridiron::meshing::GraphTopology;
use gridiron::message::comm::Communicator as GridIronCommunicator;
use gridiron::patch::Patch;
use gridiron::rect_map::{Rectangle, RectangleMap};
use gridiron::solvers::euler2d_pcm::{Mesh, PatchUpdate};
use std::collections::HashMap;

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
        let bs = bs as i64;
        let ni = mesh.size.0 as i64 / bs;
        let nj = mesh.size.1 as i64 / bs;
        let model = Model {};
        let initial_data = |i| model.primitive_at(mesh.cell_center(i)).as_array();
        let primitive = range2d(0..ni, 0..nj)
            .iter()
            .map(|(i, j)| (i * bs..(i + 1) * bs, j * bs..(j + 1) * bs))
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

fn work_assignment(
    bs: usize,
    mesh: &Mesh,
    comm: &impl GridIronCommunicator,
) -> HashMap<Rectangle<i64>, usize> {
    let bs = bs as i64;
    let ni = mesh.size.0 as i64 / bs;
    let nj = mesh.size.1 as i64 / bs;
    let blocks_per_peer = (ni * nj) as usize / comm.size();

    if (ni * nj) as usize % comm.size() != 0 {
        panic!("the number of peers must divide the number of tasks");
    }

    range2d(0..ni, 0..nj)
        .iter()
        .map(|(i, j)| (i * bs..(i + 1) * bs, j * bs..(j + 1) * bs))
        .enumerate()
        .map(|(n, rect)| (rect, n / blocks_per_peer))
        .collect()
}

#[derive(Debug, Clone, Clap)]
#[clap(version = "1.0", author = "J. Zrake <jzrake@clemson.edu>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short = 'n', long, default_value = "1000")]
    grid_resolution: usize,

    #[clap(short = 'b', long, default_value = "100")]
    block_size: usize,

    #[clap(short = 'f', long, default_value = "1")]
    fold: usize,

    #[clap(long, default_value = "0.1")]
    tfinal: f64,
}

fn main() {
    let threading = mpi::environment::Threading::Multiple;
    let (universe, _) = mpi::initialize_with_threading(threading).unwrap();
    let opts = Opts::parse();

    if opts.grid_resolution % opts.block_size != 0 {
        eprintln!("Error: block size must divide the grid resolution");
        return;
    }

    let mut comm = gridiron::message::mpi::MpiCommunicator::new(universe.world());
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

    if comm.rank() == 0 {
        println!("{:?}", opts);
    }
    println!("rank {} working on {} blocks", comm.rank(), task_list.len());

    while time < opts.tfinal {
        let start = std::time::Instant::now();

        for _ in 0..opts.fold {
            task_list = automaton::execute_dist(&comm, &code, &work, task_list).collect();
            iteration += 1;
            time += dt;
            comm.next_time_stamp();
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
