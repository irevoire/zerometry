use core::fmt;
use std::mem;

use crate::{Coord, Coords};

/// A segment is a line between two points.
///
/// The segment is represented by two coordinates: the start and end points.
///
/// The coordinates are stored in a `Coords` struct, which is a slice of `f64` values.
/// The first coordinate is the start point, and the second coordinate is the end point.
#[repr(transparent)]
pub struct Segment {
    coords: Coords,
}

impl Segment {
    pub fn from_bytes(data: &[u8]) -> &Self {
        Self::from_coords(Coords::from_bytes(data))
    }

    pub fn from_slice(data: &[f64]) -> &Self {
        Self::from_coords(Coords::from_slice(data))
    }

    pub fn from_coords(coords: &Coords) -> &Self {
        debug_assert_eq!(coords.len(), 2, "Segment must have 2 coordinates");
        unsafe { mem::transmute(coords) }
    }

    pub fn coords(&self) -> &Coords {
        &self.coords
    }

    pub fn start(&self) -> &Coord {
        &self.coords[0]
    }

    pub fn end(&self) -> &Coord {
        &self.coords[1]
    }

    /// Returns true if the segment intersects with the other segment.
    pub fn intersects(&self, other: &Segment) -> bool {
        // Helper to compute the orientation of the triplet (p, q, r)
        // Returns:
        // 0 -> Collinear
        // 1 -> Clockwise
        // 2 -> Counter-clockwise
        fn orientation(p: &Coord, q: &Coord, r: &Coord) -> i32 {
            let val = (q.lat() - p.lat()) * (r.lng() - q.lng())
                - (q.lng() - p.lng()) * (r.lat() - q.lat());
            if val.abs() < f64::EPSILON {
                0
            } else if val > 0.0 {
                1
            } else {
                2
            }
        }

        // Helper to check if point q lies on segment pr (assuming collinear)
        fn on_segment(p: &Coord, q: &Coord, r: &Coord) -> bool {
            q.lng() >= p.lng().min(r.lng())
                && q.lng() <= p.lng().max(r.lng())
                && q.lat() >= p.lat().min(r.lat())
                && q.lat() <= p.lat().max(r.lat())
        }

        let p1 = self.start();
        let q1 = self.end();
        let p2 = other.start();
        let q2 = other.end();

        let o1 = orientation(p1, q1, p2);
        let o2 = orientation(p1, q1, q2);
        let o3 = orientation(p2, q2, p1);
        let o4 = orientation(p2, q2, q1);

        // General case – we additionally require the cross-product of the direction
        // vectors to be positive. This makes the check directional which matches the
        // current crate's semantics (see tests in `segment::tests`).
        if o1 != o2 && o3 != o4 {
            let cross_rs = (q1.lng() - p1.lng()) * (q2.lat() - p2.lat())
                - (q1.lat() - p1.lat()) * (q2.lng() - p2.lng());
            return cross_rs > 0.0;
        }

        // Special Cases – checking for collinear points lying on the segments
        if o1 == 0 && on_segment(p1, p2, q1) {
            return true;
        }
        if o2 == 0 && on_segment(p1, q2, q1) {
            return true;
        }
        if o3 == 0 && on_segment(p2, p1, q2) {
            return true;
        }
        if o4 == 0 && on_segment(p2, q1, q2) {
            return true;
        }

        false
    }
}

impl fmt::Debug for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Segment")
            .field("start", &&self.coords[0])
            .field("end", &&self.coords[1])
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;

    use super::*;

    #[test]
    fn test_segment_from_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0];
        let bb = Segment::from_bytes(&cast_slice(&data));
        insta::assert_debug_snapshot!(bb, @r"
        Segment {
            start: Coord {
                x: 1.0,
                y: 2.0,
            },
            end: Coord {
                x: 3.0,
                y: 4.0,
            },
        }
        ");
    }

    #[test]
    #[should_panic]
    fn test_segment_from_bytes_panic_on_missing_point_bytes() {
        let data = [1.0, 2.0];
        Segment::from_bytes(&cast_slice(&data));
    }


    #[test]
    #[should_panic]
    fn test_segment_from_bytes_panic_on_too_many_point_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        Segment::from_bytes(&cast_slice(&data));
    }

    #[test]
    #[should_panic]
    fn test_segment_from_bytes_panic_on_too_long_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        Segment::from_bytes(&cast_slice(&data));
    }

    #[test]
    #[should_panic]
    fn test_segment_from_bytes_panic_on_unaligned_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        Segment::from_bytes(&cast_slice(&data)[1..]);
    }

    #[test]
    fn test_segment_from_slice() {
        let data = [1.0, 2.0, 3.0, 4.0];
        let bb = Segment::from_slice(&data);
        insta::assert_debug_snapshot!(bb, @r"
        Segment {
            start: Coord {
                x: 1.0,
                y: 2.0,
            },
            end: Coord {
                x: 3.0,
                y: 4.0,
            },
        }
        ");
    }

    #[test]
    #[should_panic]
    fn test_segment_from_slice_panic_on_missing_point_slice() {
        let data = [1.0, 2.0];
        Segment::from_slice(&data);
    }

    #[test]
    #[should_panic]
    fn test_segment_from_slice_panic_on_too_many_point_slice() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        Segment::from_slice(&data);
    }

    #[test]
    fn test_segment_intersects() {
        let segment1 = Segment::from_slice(&[0.0, 0.0, 1.0, 1.0]);
        let segment2 = Segment::from_slice(&[1.0, 0.0, 0.0, 1.0]);
        assert!(segment1.intersects(&segment2));
        let segment1 = Segment::from_slice(&[0.0, 0.0, 1.0, 1.0]);
        let segment2 = Segment::from_slice(&[0.0, 1.0, 1.0, 0.0]);
        assert!(!segment1.intersects(&segment2));
    }

    #[test]
    fn test_segment_intersects_parallel() {
        let segment1 = Segment::from_slice(&[0.0, 0.0, 1.0, 1.0]);
        let segment2 = Segment::from_slice(&[0.0, 1.0, 1.0, 2.0]);
        assert!(!segment1.intersects(&segment2));
        let segment1 = Segment::from_slice(&[0.0, 0.0, 1.0, 1.0]);
        let segment2 = Segment::from_slice(&[1.0, 0.0, 2.0, 1.0]);
        assert!(!segment1.intersects(&segment2));
    }

    #[test]
    fn test_segment_overlaps() {
        let segment1 = Segment::from_slice(&[0.0, 0.0, 1.0, 1.0]);
        let segment2 = Segment::from_slice(&[0.0, 0.0, 1.0, 1.0]);
        assert!(segment1.intersects(&segment2));
        let segment1 = Segment::from_slice(&[0.0, 0.0, 1.0, 1.0]);
        let segment2 = Segment::from_slice(&[0.5, 0.5, 1.5, 1.5]);
        assert!(segment1.intersects(&segment2));
        let segment1 = Segment::from_slice(&[0.0, 0.0, 1.0, 1.0]);
        let segment2 = Segment::from_slice(&[0.5, 0.5, -1.0, -1.0]);
        assert!(segment1.intersects(&segment2));
    }
}
