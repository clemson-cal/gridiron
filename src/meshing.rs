//! Functions for filling guard zone regions and creating adjacency lists.
//!
//! Adjacency lists are used to establish the flow of data in parallel
//! executions based on message-passing.

use crate::adjacency_list::AdjacencyList;
use crate::index_space::IndexSpace;
use crate::patch::Patch;
use crate::rect_map::{Rectangle, RectangleMap};

/// A trait for a container that can respond to queries for a patch overlying
/// a point.
pub trait PatchQuery {
    /// Return a patch containing the given point, if one exists.
    fn patch_containing_point(&self, point: (i64, i64)) -> Option<&Patch>;
}

impl PatchQuery for Vec<Patch> {
    fn patch_containing_point(&self, point: (i64, i64)) -> Option<&Patch> {
        self.iter()
            .find(|p| p.high_resolution_space().contains(point))
    }
}

impl PatchQuery for RectangleMap<i64, Patch> {
    fn patch_containing_point(&self, point: (i64, i64)) -> Option<&Patch> {
        self.query_point(point).next().map(|(_, p)| p)
    }
}

/// Fills guard zone values in a mutable patch by sampling data from other
/// patches in `PatchQuery` object. Indexes contained in the
/// `valid_index_space` are not touched.
///
/// __WARNING__: this function is currently implemented only for patches at
/// uniform refinement level.
///
/// __WARNING__: this function currently neglects the patch corners. The
/// corners are needed for MHD and viscous fluxes.
pub fn extend_patch_mut<P, G>(
    patch: &mut Patch,
    valid_index_space: &IndexSpace,
    boundary_value: G,
    neighbors: &P,
) where
    P: PatchQuery,
    G: Fn((i64, i64), &mut [f64]),
{
    let (i0, j0) = valid_index_space.start();
    let (i1, j1) = valid_index_space.end();
    let (x0, y0) = patch.index_space().start();
    let (x1, y1) = patch.index_space().end();

    let li = IndexSpace::new(x0..i0, j0..j1);
    let lj = IndexSpace::new(i0..i1, y0..j0);
    let ri = IndexSpace::new(i1..x1, j0..j1);
    let rj = IndexSpace::new(i0..i1, j1..y1);

    for index in li.iter().chain(lj.iter()).chain(ri.iter()).chain(rj.iter()) {
        let slice = patch.get_slice_mut(index);
        if let Some(neigh) = neighbors.patch_containing_point(index) {
            slice.clone_from_slice(neigh.get_slice(index))
        } else {
            boundary_value(index, slice)
        }
    }
}

/// A trait for a container that can yield an adjacency list (the container
/// items can form a topology). The intended use case is for a `RectangleMap`
/// of patches, where adjacency means that two patches overlap when one is
/// extended. More specifically, a graph edge pointing from patch `A` to patch
/// `B` means that `A` is _upstream_ of `B`: guard zones from `A` are required
/// to extend `B`. In parallel executions, messages are passed in the
/// direction of the arrows, from `A` to `B` in this case.
pub trait GraphTopology {
    /// The type of key used to identify vertices
    type Key;

    /// An additional type parameter given to `Self::adjacency_list`. In
    /// contect, this is probably the number of guard zones, which in general
    /// will influence which other patches are neighbors.
    type Parameter;

    /// Return an adjacency list derived from this container.
    fn adjacency_list(&self, parameter: Self::Parameter) -> AdjacencyList<Self::Key>;
}

impl GraphTopology for RectangleMap<i64, Patch> {
    type Key = (Rectangle<i64>, u32);

    type Parameter = i64;

    fn adjacency_list(&self, num_guard: Self::Parameter) -> AdjacencyList<Self::Key> {
        let mut edges = AdjacencyList::new();

        for (b, q) in self.iter() {
            for (a, p) in self.query_rect(q.index_space().extend_all(num_guard)) {
                if a != b {
                    let a = (IndexSpace::from(a).into(), p.level());
                    let b = (IndexSpace::from(b).into(), q.level());
                    edges.insert(a, b)
                }
            }
        }
        edges
    }
}

/// Computes the integer square root, `floor(sqrt(n))`, of an unsigned integer
/// `n`. Based on [Newton's method][1].
///
/// [1]: https://en.wikipedia.org/wiki/Integer_square_root
pub fn integer_square_root(n: usize) -> usize {
    let mut x0 = n >> 1;

    if x0 == 0 {
        n
    } else {
        let mut x1 = (x0 + n / x0) >> 1;

        while x1 < x0 {
            x0 = x1;
            x1 = (x0 + n / x0) >> 1;
        }
        x0
    }
}

/// Find the prime factors of an unsigned integer. Based on Pollard’s Rho
/// algorithm.
pub fn prime_factors(mut n: usize) -> Vec<usize> {
    let mut result = Vec::new();

    while n % 2 == 0 {
        result.push(2);
        n /= 2
    }
    let mut i = 3;

    while i <= integer_square_root(n) {
        while n % i == 0 {
            result.push(i);
            n /= i
        }
        i += 2
    }

    if n > 2 {
        result.push(n)
    }
    result
}

/// Factors a target number of total blocks (`count`) (say 200) into
/// rectangular dimensions, (`[20, 10]` for `num_dims=2` or `[10, 10, 2]` for
/// `num_dims=3`). In context, `count` will be the number of tasks in a
/// calculation, and `num_dims` is the rank of the arrays. This function is
/// like `MPI_Dims_create`.
pub fn block_dims(count: usize, num_dims: usize) -> Vec<usize> {
    let factors = prime_factors(count);
    (0..num_dims)
        .map(|dim| {
            if factors.is_empty() {
                1
            } else {
                factors[dim..]
                    .chunks(num_dims)
                    .map(|chunk| chunk[0])
                    .product()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_square_root_works() {
        assert_eq!(integer_square_root(0), 0);
        assert_eq!(integer_square_root(1), 1);
        assert_eq!(integer_square_root(2), 1);
        assert_eq!(integer_square_root(4), 2);
        assert_eq!(integer_square_root(35), 5);
        assert_eq!(integer_square_root(36), 6);
    }

    #[test]
    fn prime_factors_works() {
        assert_eq!(prime_factors(1), vec![]);
        assert_eq!(prime_factors(2), vec![2]);
        assert_eq!(prime_factors(3), vec![3]);
        assert_eq!(prime_factors(4), vec![2, 2]);
        assert_eq!(prime_factors(5), vec![5]);
        assert_eq!(prime_factors(6), vec![2, 3]);
        assert_eq!(prime_factors(9), vec![3, 3]);
        assert_eq!(prime_factors(12), vec![2, 2, 3]);
        assert_eq!(prime_factors(100), vec![2, 2, 5, 5]);
    }

    #[test]
    fn block_dims_works() {
        assert_eq!(block_dims(1, 2), vec![1, 1]);
        assert_eq!(block_dims(1, 3), vec![1, 1, 1]);
        assert_eq!(block_dims(4, 2), vec![2, 2]);
        assert_eq!(block_dims(5, 2), vec![5, 1]);
        assert_eq!(block_dims(10, 2), vec![2, 5]);
        assert_eq!(block_dims(16, 2), vec![4, 4]);
        assert_eq!(block_dims(200, 2), vec![20, 10]);
        assert_eq!(block_dims(200, 3), vec![10, 10, 2]);
        assert_eq!(block_dims(1000, 3), vec![10, 10, 10]);
        assert_eq!(block_dims(2000, 3), vec![20, 10, 10]);
    }
}
