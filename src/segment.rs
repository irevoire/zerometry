use core::fmt;

use crate::{Coord, Coords};

/// A segment is a line between two points.
///
/// The segment is represented by two coordinates: the start and end points.
///
/// The coordinates are stored in a `Coords` struct, which is a slice of `f64` values.
/// The first coordinate is the start point, and the second coordinate is the end point.
#[derive(Clone, Copy)]
pub struct Segment<'a> {
    start: &'a Coord,
    end: &'a Coord,
}

impl<'a> Segment<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Self {
        Self::from_coords(Coords::from_bytes(data))
    }

    pub fn from_slice(data: &'a [f64]) -> Self {
        Self::from_coords(Coords::from_slice(data))
    }

    pub fn from_coords(coords: &'a Coords) -> Self {
        debug_assert_eq!(coords.len(), 2, "Segment must have 2 coordinates");
        Self {
            start: &coords[0],
            end: &coords[1],
        }
    }

    pub fn from_coord_pair(start: &'a Coord, end: &'a Coord) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> &'a Coord {
        self.start
    }

    pub fn end(&self) -> &'a Coord {
        self.end
    }

    /// Returns true if the segment intersects with the other segment.
    pub fn intersects(&self, other: &Segment) -> bool {
        geo::intersects::Intersects::intersects(&geo_types::Line::new(self.start.to_geo(), self.end.to_geo()), &geo_types::Line::new(other.start.to_geo(), other.end.to_geo()))
    }
}

impl<'a> fmt::Debug for Segment<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Segment")
            .field("start", &self.start)
            .field("end", &self.end)
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
        assert!(segment1.intersects(&segment2));
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

    #[test]
    fn bug_missing_intersection() {
        // ray: Segment { start: Coord { x: -6.436337296790293, y: 49.63676497357687 }, end: Coord { x: 6.0197316417968105, y: 49.63676497357687 } }
        // segment: Segment { start: Coord { x: 1.188509553443464, y: 49.47027919866874 }, end: Coord { x: 3.6300086390995316, y: 50.610463312569514 } }
        let ray = Segment::from_slice(&[-6.436337296790293, 49.63676497357687, 6.0197316417968105, 49.63676497357687]);
        let segment = Segment::from_slice(&[1.188509553443464, 49.47027919866874, 3.6300086390995316, 50.610463312569514]);
        assert!(segment.intersects(&ray));
    }
}
