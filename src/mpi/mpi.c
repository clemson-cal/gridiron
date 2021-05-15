#include <stddef.h>
#include <mpi.h>

struct Status {
    int count;
    int source;
    int tag;
};

int gridiron_mpi_init() {
    int level;
    MPI_Init_thread(NULL, NULL, MPI_THREAD_MULTIPLE, &level);
    return level == MPI_THREAD_MULTIPLE;
}

void gridiron_mpi_barrier() {
    MPI_Barrier(MPI_COMM_WORLD);
}

void gridiron_mpi_finalize() {
    MPI_Finalize();
}

int gridiron_mpi_comm_size() {
    int size;
    MPI_Comm_size(MPI_COMM_WORLD, &size);
    return size;
}

int gridiron_mpi_comm_rank() {
    int rank;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);
    return rank;
}

void gridiron_mpi_send(const void* buf, int count, int dest, int tag) {
    MPI_Send(buf, count, MPI_BYTE, dest, tag, MPI_COMM_WORLD);
}

void gridiron_mpi_recv(void* buf, int count, int source, int tag) {
    MPI_Recv(buf, count, MPI_BYTE, source, tag, MPI_COMM_WORLD, MPI_STATUS_IGNORE);
}

struct Status gridiron_mpi_probe_tag(int tag) {
    MPI_Status status;
    struct Status result;
    MPI_Probe(MPI_ANY_SOURCE, tag, MPI_COMM_WORLD, &status);
    MPI_Get_count(&status, MPI_BYTE, &result.count);
    result.source = status.MPI_SOURCE;
    result.tag = tag;
    return result;
}
