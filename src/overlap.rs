use core::ops::Bound;
use core::ops::RangeBounds;

/// Extension trait to determine whether two range bounds objects overlap. Two
/// ranges that line up end-to-end do not overlap, regardless of whether the
/// endpoints that touch are included or exluded.
pub trait Overlap<T>: RangeBounds<T> {
    fn overlaps<S: Overlap<T>>(&self, s: &S) -> bool;
}

impl<R, T> Overlap<T> for R
where
    R: RangeBounds<T>,
    T: Ord,
{
    fn overlaps<S: Overlap<T>>(&self, s: &S) -> bool {
        use Bound::*;

        let lower = match (self.start_bound(), s.start_bound()) {
            (Unbounded, Unbounded) => Unbounded,
            (Included(l0), Unbounded) => Included(l0),
            (Unbounded, Included(l1)) => Included(l1),
            (Excluded(l0), Unbounded) => Excluded(l0),
            (Unbounded, Excluded(l1)) => Excluded(l1),
            (Included(l0), Included(l1)) => Included(l0.max(l1)),
            (Included(l0), Excluded(l1)) => Excluded(l0.max(l1)),
            (Excluded(l0), Included(l1)) => Excluded(l0.max(l1)),
            (Excluded(l0), Excluded(l1)) => Excluded(l0.max(l1)),
        };

        let upper = match (self.end_bound(), s.end_bound()) {
            (Unbounded, Unbounded) => Unbounded,
            (Included(r0), Unbounded) => Included(r0),
            (Unbounded, Included(r1)) => Included(r1),
            (Excluded(r0), Unbounded) => Excluded(r0),
            (Unbounded, Excluded(r1)) => Excluded(r1),
            (Included(r0), Included(r1)) => Included(r0.min(r1)),
            (Included(r0), Excluded(r1)) => Excluded(r0.min(r1)),
            (Excluded(r0), Included(r1)) => Excluded(r0.min(r1)),
            (Excluded(r0), Excluded(r1)) => Excluded(r0.min(r1)),
        };

        match (lower, upper) {
            (Unbounded, _) => true,
            (_, Unbounded) => true,
            (Included(l), Included(r)) => l < r,
            (Included(l), Excluded(r)) => l < r,
            (Excluded(l), Included(r)) => l < r,
            (Excluded(l), Excluded(r)) => l < r,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Overlap;

    #[test]
    fn overlapping_ranges_works() {
        assert!((0..2).overlaps(&(1..3)));
        assert!((..).overlaps(&(..2)));
        assert!(!(..=2).overlaps(&(2..)));
        assert!(!(0..2).overlaps(&(2..3)));
        assert!(!(..=2).overlaps(&(3..)));
        assert!(!(4..).overlaps(&(..2)));
    }
}
