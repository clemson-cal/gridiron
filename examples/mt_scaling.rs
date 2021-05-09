// #![feature(test)]

// use clap::{AppSettings, Clap};
// use gridiron::thread_pool::ThreadPool;
// use rayon::prelude::*;

// #[derive(Debug, Clap)]
// #[clap(version = "1.0", author = "J. Zrake <jzrake@clemson.edu>")]
// #[clap(setting = AppSettings::ColoredHelp)]
// struct Opts {
//     #[clap(short = 't', long, default_value = "1")]
//     num_threads: usize,

//     #[clap(short = 'n', long, default_value = "1000")]
//     num_jobs: usize,

//     #[clap(short = 'w', long, default_value = "100000")]
//     work_per_job: u64,
// }

// fn do_work(work: u64) -> f64 {

//     // Work load 1:
//     // std::thread::sleep(std::time::Duration::from_micros(work));
//     // 0.0

//     // Work load 2:
//     // (0..work).map(|n| n as f64).sum()

//     // Work load 3:
//     let mut x: [f64; 4] = [0.0, 0.0, 0.0, 0.0];
//     let mut n = 0;

//     while n < work {
//         n += 1;
//         x[0] += 0.0;
//         x[1] += x[0].sin();
//         x[2] += x[1].cos();
//         x[3] += x[2].ln();
//     }
//     x[3]
// }

// fn main() {
//     let opts = Opts::parse();
//     println!("{:?}", opts);

//     let duration = {
//         let pool = ThreadPool::new(opts.num_threads);
//         let work = opts.work_per_job;
//         let start = std::time::Instant::now();

//         for _ in 0..opts.num_jobs {
//             pool.spawn(move || {
//                 std::hint::black_box(do_work(work));
//             });
//         }
//         drop(pool);
//         start.elapsed().as_secs_f64()
//     };
//     println!();
//     println!("gridiron::ThreadPool");
//     println!("total ................. {}s", duration);
//     println!(
//         "cpu-s ................. {}",
//         duration * opts.num_threads as f64
//     );
//     println!(
//         "cpu-ns / job / work ... {}",
//         duration * opts.num_threads as f64 / opts.num_jobs as f64 / opts.work_per_job as f64 * 1e9
//     );

//     let duration = {
//         let pool = rayon::ThreadPoolBuilder::new()
//             .num_threads(opts.num_threads)
//             .build()
//             .unwrap();
//         let work = opts.work_per_job;
//         let start = std::time::Instant::now();
//         pool.scope(|scope| {
//             for _ in 0..opts.num_jobs {
//                 scope.spawn(|_| {
//                     std::hint::black_box(do_work(work));
//                 });
//             }
//         });
//         start.elapsed().as_secs_f64()
//     };
//     println!();
//     println!("rayon::ThreadPool");
//     println!("total ................. {}s", duration);
//     println!(
//         "cpu-s ................. {}",
//         duration * opts.num_threads as f64
//     );
//     println!(
//         "cpu-ns / job / work ... {}",
//         duration * opts.num_threads as f64 / opts.num_jobs as f64 / opts.work_per_job as f64 * 1e9
//     );

//     let duration = {
//         let pool = rayon::ThreadPoolBuilder::new()
//             .num_threads(opts.num_threads)
//             .build()
//             .unwrap();
//         let work = opts.work_per_job;
//         let data: Vec<_> = (0..opts.num_jobs).collect();
//         let start = std::time::Instant::now();
//         pool.install(|| {
//             data.par_iter().for_each(|_| {
//                 std::hint::black_box(do_work(work));
//             });
//         });
//         start.elapsed().as_secs_f64()
//     };
//     println!();
//     println!("rayon::par_iter");
//     println!("total ................. {}s", duration);
//     println!(
//         "cpu-s ................. {}",
//         duration * opts.num_threads as f64
//     );
//     println!(
//         "cpu-ns / job / work ... {}",
//         duration * opts.num_threads as f64 / opts.num_jobs as f64 / opts.work_per_job as f64 * 1e9
//     );
// }
fn main() {

}
