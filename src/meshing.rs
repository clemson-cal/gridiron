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
