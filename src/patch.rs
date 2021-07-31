use crate::index_space::IndexSpace;
use crate::rect_map::Rectangle;
use std::cmp::Ordering::*;

/// Identifies the part of the mesh where patch data resides. An
/// `n`-dimensional cartesian array has `n` of these parameters, one per axis.
/// `Cell` regions are the spaces between `Node` points. For example in a 3D
/// array, the tuple `(Cell, Cell, Cell)` refers to 3D cell volumes, `(Node,
/// Cell, Cell)` refers to the 2D, `i`-directed cell faces, and `(Node, Node,
/// Cell)` refers to the 1D `k`-directed cell edges. (`Node`, `Node`, `Node`)
/// are the point-like vertices of the dual mesh.
///
/// Different mesh locations have different sampling policies. For example,
/// sampling data in 3D finite volumes at a coarser granularity level involves
/// averaging over 8 smaller volumes, whereas down-sampling the data residing
/// on 2D faces involves averaging over 4 smaller faces and down-sampling
/// edge-like data averages over 2 smaller edges. Data can only be sampled up
/// and down on its cell-like axes. For example, up-sampling a 3D array of
/// faces only splits each "window" into four smaller ones; it does not add
/// new "panes" between the existing ones.
///
/// A patch's index space is the same regardless of the mesh location: it
/// always refers to the patch's index extent on the primary grid. However the
/// array size is one larger on the node-like axes.
///
/// The flux correction on a patch P at level n procedes by identifying all
/// patches which overlap P at a higher granularity, and sampling those
/// patches at level n wherever they intersect P.
pub enum MeshLocation {
    Cell,
    Node,
}

/// A patch is a mapping from a rectangular subset of a high-resolution index
/// space (HRIS), to associated field values. The mapping is backed by an
/// array of data, which is in general at a coarser level of granularity than
/// the HRIS; each cell in the backing array stands for (2^level)^rank cells
/// in the HRIS. The HRIS is at level 0.
///
/// The patch can be sampled at different granularity levels. If the sampling
/// level is finer than the patch granularity, then sub-cell sampling is
/// employed, either with piecewise constant or multi-linear interpolation. If
/// the sampling level is coarser than the patch granularity, then the result
/// is obtained by averaging over the region within the patch covered by the
/// coarse cell.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Patch {
    /// The granularity level of this patch. Level 0 is the highest resolution.
    level: u32,

    /// The region of index space covered by this patch. The indexes are with
    /// respect to the ticks at this patch's granularity level.
    rect: Rectangle<i64>,

    /// The number of fields stored at each zone.
    num_fields: usize,

    /// The backing array of data on this patch.
    data: Vec<f64>,
}

impl Patch {
    /// Creates a new empty patch.
    pub fn new() -> Self {
        Self {
            level: 0,
            rect: (0..0, 0..0),
            num_fields: 0,
            data: Vec::new(),
        }
    }

    /// Generates a patch of zeros over the given index space.
    pub fn zeros<I: Into<IndexSpace>>(level: u32, num_fields: usize, space: I) -> Self {
        let space: IndexSpace = space.into();
        let data = vec![0.0; space.len() * num_fields];

        Self {
            rect: space.into(),
            level,
            num_fields,
            data,
        }
    }

    /// Generates a patch at a given level, covering the given space, with
    /// values defined from a closure.
    pub fn from_scalar_function<I, F>(level: u32, space: I, f: F) -> Self
    where
        I: Into<IndexSpace>,
        F: Fn((i64, i64)) -> f64,
    {
        Self::from_vector_function(level, space, |i| [f(i)])
    }

    /// Generates a patch at a given level, covering the given space, with
    /// values defined from a closure which returns a fixed-length array. The
    /// number of fields in the patch is inferred from the size of the fixed
    /// length array returned by the closure.
    pub fn from_vector_function<I, F, const NUM_FIELDS: usize>(level: u32, space: I, f: F) -> Self
    where
        I: Into<IndexSpace>,
        F: Fn((i64, i64)) -> [f64; NUM_FIELDS],
    {
        Self::from_slice_function(level, space, NUM_FIELDS, |i, s| s.clone_from_slice(&f(i)))
    }

    /// Generates a patch at a given level, covering the given space, with
    /// values defined from a closure which operates on mutable slices.
    pub fn from_slice_function<I, F>(level: u32, space: I, num_fields: usize, f: F) -> Self
    where
        I: Into<IndexSpace>,
        F: Fn((i64, i64), &mut [f64]),
    {
        let space: IndexSpace = space.into();
        let mut data = vec![0.0; space.len() * num_fields];

        for (index, slice) in space.iter().zip(data.chunks_exact_mut(num_fields)) {
            f(index, slice)
        }
        Self {
            level,
            data,
            rect: space.into(),
            num_fields,
        }
    }

    pub fn extract_from(source: &Patch, selection: IndexSpace) -> Self {
        Self::from_slice_function(
            source.level,
            selection,
            source.num_fields,
            |index, slice| {
                if source.index_space().contains(index) {
                    slice.clone_from_slice(source.get_slice(index))
                }
            },
        )
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn num_fields(&self) -> usize {
        self.num_fields
    }

    pub fn data(&self) -> &Vec<f64> {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [f64] {
        &mut self.data
    }

    pub fn iter_data_mut(&mut self) -> impl Iterator<Item = &mut [f64]> {
        self.data.chunks_exact_mut(self.num_fields)
    }

    pub fn select(&self, subspace: IndexSpace) -> impl Iterator<Item = &'_ [f64]> {
        subspace
            .memory_region_in(&self.index_space())
            .iter_slice(&self.data, self.num_fields)
    }

    pub fn select_mut(&mut self, subspace: IndexSpace) -> impl Iterator<Item = &'_ mut [f64]> {
        subspace
            .memory_region_in(&self.index_space())
            .iter_slice_mut(&mut self.data, self.num_fields)
    }

    /// Returns this patch's rectangle.
    pub fn local_rect(&self) -> &Rectangle<i64> {
        &self.rect
    }

    pub fn index_space(&self) -> IndexSpace {
        IndexSpace::from(self.rect.clone())
    }

    /// Returns the index space at the high-resolution level below this patch.
    pub fn high_resolution_space(&self) -> IndexSpace {
        self.index_space().refine_by(1 << self.level)
    }

    /// Convenience method to convert the high resolution index space to a
    /// rectangle.
    pub fn high_resolution_rect(&self) -> Rectangle<i64> {
        self.index_space().refine_by(1 << self.level).into()
    }

    /// Samples the field at the given level and index. The index measures
    /// ticks at the target sampling level, not the HRIS.
    pub fn sample(&self, level: u32, index: (i64, i64), field: usize) -> f64 {
        match level.cmp(&self.level) {
            Equal => {
                self.validate_index(index, field);

                let (i0, j0) = self.index_space().start();
                let i = (index.0 - i0) as usize;
                let j = (index.1 - j0) as usize;

                let (_m, n) = self.index_space().dim();
                self.data[(i * n + j) * self.num_fields + field]
            }
            Less => self.sample(level + 1, (index.0 / 2, index.1 / 2), field),
            Greater => {
                let y00 = self.sample(level - 1, (index.0 * 2, index.1 * 2), field);
                let y01 = self.sample(level - 1, (index.0 * 2, index.1 * 2 + 1), field);
                let y10 = self.sample(level - 1, (index.0 * 2 + 1, index.1 * 2), field);
                let y11 = self.sample(level - 1, (index.0 * 2 + 1, index.1 * 2 + 1), field);
                0.25 * (y00 + y01 + y10 + y11)
            }
        }
    }

    /// Samples all the fields in this patch at the given index and returns
    /// the result as a fixed-length array. The array size must be less than
    /// or equal to the number of fields.
    pub fn sample_array<const NUM_FIELDS: usize>(
        &self,
        level: u32,
        index: (i64, i64),
    ) -> [f64; NUM_FIELDS] {
        assert! {
            NUM_FIELDS <= self.num_fields,
            "attempt to sample {} fields from a patch with {} fields",
            NUM_FIELDS,
            self.num_fields
        };

        let mut result = [0.0; NUM_FIELDS];
        self.sample_slice(level, index, &mut result);
        result
    }

    /// Samples all the fields in this patch at the given index and writes the
    /// result into the given slice. The slice must be at least as large as
    /// the number of fields.
    pub fn sample_slice(&self, level: u32, index: (i64, i64), result: &mut [f64]) {
        for (field, r) in result.iter_mut().enumerate() {
            *r = self.sample(level, index, field)
        }
    }

    /// Returns a slice of all data fields at the given index. This method
    /// does not check if the index is logically in bounds, but will panic if
    /// a memory location would have been out of bounds.
    pub fn get_slice(&self, index: (i64, i64)) -> &[f64] {
        let s = self.index_space().row_major_offset(index);
        &self.data[s * self.num_fields..(s + 1) * self.num_fields]
    }

    pub fn get_slice_mut(&mut self, index: (i64, i64)) -> &mut [f64] {
        let s = self.index_space().row_major_offset(index);
        &mut self.data[s * self.num_fields..(s + 1) * self.num_fields]
    }

    /// Extracts a subset of this patch and return it. This method panics if
    /// the slice is out of bounds.
    pub fn extract<I: Into<IndexSpace>>(&self, subset: I) -> Self {
        let subset: IndexSpace = subset.into();

        assert! {
            self.index_space().contains_space(&subset),
            "the index space is out of bounds"
        }

        Self::from_slice_function(self.level, subset, self.num_fields, |index, slice| {
            slice.clone_from_slice(self.get_slice(index))
        })
    }

    pub fn map_index_mut<F>(&mut self, f: F)
    where
        F: Fn((i64, i64), &mut [f64]),
    {
        let num_fields = self.num_fields();
        let index_space = self.index_space();
        let memory_region = index_space.memory_region();

        index_space
            .iter()
            .zip(memory_region.iter_slice_mut(&mut self.data, num_fields))
            .for_each(|(index, slice)| f(index, slice))
    }

    /// Maps values from this patch into another one. The two patches must be
    /// on the same level and have the same number of fields, but they do not
    /// need to have the same index space. Only the elements at the
    /// overlapping part of the index spaces are mapped; the remaining part of
    /// the target patch is unchanged. This method panics if the target space
    /// does not overlap this one.
    pub fn map_into<F>(&self, target: &mut Self, f: F)
    where
        F: Fn(&[f64], &mut [f64]),
    {
        assert!(self.level == target.level);
        assert!(self.num_fields == target.num_fields);

        let overlap_space = self
            .index_space()
            .intersect(&target.index_space())
            .expect("patches do not overlap");
        let source_region = overlap_space.memory_region_in(&self.index_space());
        let target_region = overlap_space.memory_region_in(&target.index_space());

        source_region
            .iter_slice(&self.data, self.num_fields)
            .zip(target_region.iter_slice_mut(&mut target.data, self.num_fields))
            .for_each(|x| f(x.0, x.1))
    }

    /// Copies values from this patch into another one. The two patches must
    /// be on the same level and have the same number of fields, but they do
    /// not need to have the same index space. Only the elements at the
    /// overlapping part of the index spaces are mapped; the remaining part of
    /// the target patch is unchanged.
    pub fn copy_into(&self, target: &mut Self) {
        self.map_into(target, |src, dst| {
            for (s, d) in src.iter().zip(dst) {
                *d = *s;
            }
        })
    }

    pub fn map<F>(&self, f: F) -> Self
    where
        F: Fn(&[f64], &mut [f64]),
    {
        let mut data = vec![0.0; self.data.len()];
        self.data
            .chunks_exact(self.num_fields)
            .zip(data.chunks_exact_mut(self.num_fields))
            .for_each(|x| f(x.0, x.1));
        Self {
            level: self.level,
            rect: self.rect.clone(),
            num_fields: self.num_fields,
            data,
        }
    }

    fn validate_index(&self, index: (i64, i64), field: usize) {
        let space = self.index_space();

        assert! {
            field < self.num_fields,
            "field index {} out of range on patch with {} fields",
            field,
            self.num_fields
        };

        assert! {
            space.contains(index),
            "index ({} {}) out of range on patch ({}..{} {}..{})",
            index.0,
            index.1,
            space.start().0,
            space.end().0,
            space.start().1,
            space.end().1
        };
    }
}

impl Default for Patch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {

    use super::Patch;
    use crate::index_space::{range2d, IndexSpace};
    use crate::rect_map::{Rectangle, RectangleMap, RectangleRef};

    fn finest_patch<'a>(
        map: &'a RectangleMap<i64, &'a Patch>,
        index: (i64, i64),
    ) -> Option<&'a Patch> {
        map.query_point(index)
            .map(|(_, &p)| p)
            .min_by_key(|p| p.level)
    }

    fn extend_patch(
        map: &RectangleMap<i64, Patch>,
        rect: RectangleRef<i64>,
    ) -> (Rectangle<i64>, Patch) {
        let space: IndexSpace = rect.into();
        let extended = space.extend_all(2);
        let local_map: RectangleMap<_, _> = map.query_rect(extended.clone()).collect();
        let p = local_map.get(rect).unwrap();

        let sample = |index| {
            if p.index_space().contains(index) {
                p.sample(p.level, index, 0)
            } else if let Some(n) = finest_patch(&local_map, index) {
                n.sample(p.level, index, 0)
            } else {
                0.0
            }
        };
        (
            extended.clone().into(),
            Patch::from_scalar_function(p.level, extended, sample),
        )
    }

    #[test]
    fn patch_sampling_works() {
        let patch = Patch::from_scalar_function(1, (4..10, 4..10), |(i, j)| i as f64 + j as f64);
        assert_eq!(patch.sample(1, (5, 5), 0), 10.0);
        assert_eq!(patch.sample(1, (6, 8), 0), 14.0);
        assert_eq!(patch.sample(2, (2, 2), 0), 0.25 * (8.0 + 9.0 + 9.0 + 10.0));

        // Piecewise constant sampling
        assert_eq!(patch.sample(0, (8, 8), 0), 8.0);
        assert_eq!(patch.sample(0, (9, 8), 0), 8.0);
        assert_eq!(patch.sample(0, (8, 9), 0), 8.0);
        assert_eq!(patch.sample(0, (9, 9), 0), 8.0);
        assert_eq!(patch.sample(0, (10, 10), 0), 10.0);
    }

    #[test]
    fn can_extend_patch() {
        let mut quilt = RectangleMap::new();

        for (i, j) in range2d(0..4, 0..4).iter() {
            let rect = (i * 10..(i + 1) * 10, j * 10..(j + 1) * 10);
            let patch = Patch::from_scalar_function(0, rect, |ij| ij.0 as f64 + ij.1 as f64);
            quilt.insert(patch.high_resolution_space(), patch);
        }

        let extended_quilt: RectangleMap<i64, Patch> = quilt
            .keys()
            .map(|rect| extend_patch(&quilt, rect))
            .collect();

        let p12 = extended_quilt
            .get((&(10 - 2..20 + 2), &(20 - 2..30 + 2)))
            .unwrap();
        let p21 = extended_quilt
            .get((&(20 - 2..30 + 2), &(10 - 2..20 + 2)))
            .unwrap();

        assert_eq!(p12.sample(0, (20, 20), 0), p21.sample(0, (20, 20), 0));
    }
}
