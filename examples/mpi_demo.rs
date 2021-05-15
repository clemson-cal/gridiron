#[cfg(feature = "mpi")]
fn main() {
    use gridiron::mpi;
    unsafe {
        mpi::init();

        let size = mpi::comm_size();
        let rank = mpi::comm_rank();

        if size == 1 {
            println!("example must be run with >1 processes, e.g. with mpiexec -np 2");
        } else {
            let send_buf = vec![0, 1, 2, 3];
            let mut recv_buf = vec![0; 4];

            mpi::send(send_buf.as_ptr(), 4, (rank + 1) % size, 0);
            mpi::recv(recv_buf.as_mut_ptr(), 4, (rank + size - 1) % size, 0);

            for i in 0..size {
                if rank == i {
                    println!("rank {} received {:?}", rank, recv_buf);
                }
                mpi::barrier();
            }            
        }
        mpi::finalize();
    }
}

#[cfg(not(feature = "mpi"))]
fn main() {
    println!("mpi feature is disabled");
}
