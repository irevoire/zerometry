use core::fmt;
use std::{mem, ops::RangeInclusive};

use crate::{Coord, Coords};

/// Bounding box of a Zerometry.
///
/// The bounding box is a rectangle that contains the Zerometry.
/// It is represented by two coordinates: the bottom-left and top-right corners.
///
/// The coordinates are stored in a `Coords` struct, which is a slice of `f64` values.
/// The first coordinate is the bottom-left corner, and the second coordinate is the top-right corner.
#[repr(transparent)]
pub struct BoundingBox {
    coords: Coords,
}

impl BoundingBox {
    pub fn from_bytes(data: &[u8]) -> &Self {
        Self::from_coords(Coords::from_bytes(data))
    }

    pub fn from_slice(data: &[f64]) -> &Self {
        Self::from_coords(Coords::from_slice(data))
    }

    pub fn from_coords(coords: &Coords) -> &Self {
        debug_assert_eq!(coords.len(), 2, "Bounding box must have 2 coordinates");
        debug_assert!(coords[0].lng() <= coords[1].lng(), "Bounding box must have the left side before the right side");
        debug_assert!(coords[0].lat() <= coords[1].lat(), "Bounding box must have the bottom side before the top side");
        unsafe { mem::transmute(coords) }
    }

    pub fn coords(&self) -> &Coords {
        &self.coords
    }

    pub fn bottom_left(&self) -> &Coord {
        &self.coords[0]
    }

    pub fn top_right(&self) -> &Coord {
        &self.coords[1]
    }

    pub fn bottom(&self) -> f64 {
        self.bottom_left().lat()
    }
    pub fn top(&self) -> f64 {
        self.top_right().lat()
    }
    pub fn left(&self) -> f64 {
        self.bottom_left().lng()
    }
    pub fn right(&self) -> f64 {
        self.top_right().lng()
    }

    pub fn horizontal_range(&self) -> RangeInclusive<f64> {
        self.left()..=self.right()
    }
    pub fn vertical_range(&self) -> RangeInclusive<f64> {
        self.bottom()..=self.top()
    }

    pub fn contains_coord(&self, coord: &Coord) -> bool {
        self.vertical_range().contains(&coord.lat()) && self.horizontal_range().contains(&coord.lng())
    }
}

impl fmt::Debug for BoundingBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoundingBox")
            .field("bottom_left", &&self.coords[0])
            .field("top_right", &&self.coords[1])
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;

    use super::*;

    #[test]
    fn test_bounding_box_from_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0];
        let bb = BoundingBox::from_bytes(&cast_slice(&data));
        insta::assert_debug_snapshot!(bb, @r"
        BoundingBox {
            bottom_left: Coord {
                x: 1.0,
                y: 2.0,
            },
            top_right: Coord {
                x: 3.0,
                y: 4.0,
            },
        }
        ");
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_bytes_panic_on_missing_point_bytes() {
        let data = [1.0, 2.0];
        BoundingBox::from_bytes(&cast_slice(&data));
    }


    #[test]
    #[should_panic]
    fn test_bounding_box_from_bytes_panic_on_too_many_point_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        BoundingBox::from_bytes(&cast_slice(&data));
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_bytes_panic_on_too_long_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        BoundingBox::from_bytes(&cast_slice(&data));
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_bytes_panic_on_unaligned_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        BoundingBox::from_bytes(&cast_slice(&data)[1..]);
    }

    #[test]
    fn test_bounding_box_from_slice() {
        let data = [1.0, 2.0, 3.0, 4.0];
        let bb = BoundingBox::from_slice(&data);
        insta::assert_debug_snapshot!(bb, @r"
        BoundingBox {
            bottom_left: Coord {
                x: 1.0,
                y: 2.0,
            },
            top_right: Coord {
                x: 3.0,
                y: 4.0,
            },
        }
        ");
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_slice_panic_on_missing_point_slice() {
        let data = [1.0, 2.0];
        BoundingBox::from_slice(&data);
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_slice_panic_on_too_many_point_slice() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        BoundingBox::from_slice(&data);
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_slice_panic_on_unordered_points() {
        let data = [1.0, 4.0, 3.0, 2.0];
        BoundingBox::from_slice(&data);
    }

    #[test]
    fn test_bounding_box_contains_coord() {
        let bb = BoundingBox::from_slice(&[1.0, 2.0, 3.0, 4.0]);
        assert!(bb.contains_coord(&Coord::from_slice(&[2.0, 3.0])));
        assert!(!bb.contains_coord(&Coord::from_slice(&[0.0, 0.0])));
    }
}
