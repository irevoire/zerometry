use core::fmt;
use std::{
    io::{self, Write},
    mem,
    ops::RangeInclusive,
};

use geo_types::Point;

use crate::{
    COORD_SIZE_IN_BYTES, Coord, Coords, InputRelation, OutputRelation, RelationBetweenShapes,
};

pub(crate) const BOUNDING_BOX_SIZE_IN_BYTES: usize = COORD_SIZE_IN_BYTES * 2;

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
    /// # Safety
    /// The data must be generated from the [`Self::write_from_geometry`] method and be aligned on 64 bits
    #[inline]
    pub unsafe fn from_bytes(data: &[u8]) -> &Self {
        unsafe { Self::from_coords(Coords::from_bytes(data)) }
    }

    /// Create the bounding box from a slice of `f64`
    ///
    /// # Safety
    /// The data must be generated from the [`Self::write_from_geometry`] method and be aligned on 64 bits
    #[inline]
    pub unsafe fn from_slice(data: &[f64]) -> &Self {
        unsafe { Self::from_coords(Coords::from_slice(data)) }
    }

    /// # Safety
    /// The data must be generated from the [`Self::write_from_geometry`] method and be aligned on 64 bits
    #[inline]
    pub unsafe fn from_slice_mut(data: &mut [f64]) -> &mut Self {
        unsafe { Self::from_coords_mut(Coords::from_slice_mut(data)) }
    }

    /// # Safety
    /// The Coords must:
    /// - Contains at least two elements
    /// - Be aligned on 64 bits
    /// - Have it's first coordinate smaller than the second one
    #[inline]
    pub unsafe fn from_coords(coords: &Coords) -> &Self {
        debug_assert_eq!(
            coords.len(),
            2,
            "Bounding box must have 2 coordinates but instead got {}",
            coords.len()
        );
        debug_assert!(
            coords[0].lng() <= coords[1].lng(),
            "Bounding box must have the left side before the right side"
        );
        debug_assert!(
            coords[0].lat() <= coords[1].lat(),
            "Bounding box must have the bottom side before the top side"
        );
        unsafe { mem::transmute(coords) }
    }

    /// # Safety
    /// The Coords must:
    /// - Contains at least two elements
    /// - Be aligned on 64 bits
    /// - Have it's first coordinate smaller than the second one
    #[inline]
    pub unsafe fn from_coords_mut(coords: &mut Coords) -> &mut Self {
        debug_assert_eq!(
            coords.len(),
            2,
            "Bounding box must have 2 coordinates but instead got {}",
            coords.len()
        );
        debug_assert!(
            coords[0].lng() <= coords[1].lng(),
            "Bounding box must have the left side before the right side"
        );
        debug_assert!(
            coords[0].lat() <= coords[1].lat(),
            "Bounding box must have the bottom side before the top side"
        );
        unsafe { mem::transmute(coords) }
    }

    /// Write the bounding boxe that contains all the specified points to a buffer.
    pub fn write_from_geometry(
        writer: &mut impl Write,
        mut points: impl Iterator<Item = Point<f64>>,
    ) -> Result<(), io::Error> {
        // if there is no points we ends up with an empty bouding box in 0,0 and on points in the polygon
        let first_point = points.next().unwrap_or_default();
        let mut top = first_point.y();
        let mut bottom = first_point.y();
        let mut left = first_point.x();
        let mut right = first_point.x();

        for point in points {
            if point.y() > top {
                top = point.y();
            }
            if point.y() < bottom {
                bottom = point.y();
            }
            if point.x() < left {
                left = point.x();
            }
            if point.x() > right {
                right = point.x();
            }
        }

        // 1. Write the bounding box
        //   It's bottom left first
        writer.write_all(&left.to_ne_bytes())?;
        writer.write_all(&bottom.to_ne_bytes())?;
        //   Then the top right
        writer.write_all(&right.to_ne_bytes())?;
        writer.write_all(&top.to_ne_bytes())?;
        Ok(())
    }

    /// Return the internal coords
    #[inline]
    pub fn coords(&self) -> &Coords {
        &self.coords
    }

    /// Return the bottom left coord
    #[inline]
    pub fn bottom_left(&self) -> &Coord {
        &self.coords[0]
    }

    /// Return the top right coord
    #[inline]
    pub fn top_right(&self) -> &Coord {
        &self.coords[1]
    }

    /// Return the bottom latitude
    #[inline]
    pub fn bottom(&self) -> f64 {
        self.bottom_left().lat()
    }
    /// Return the top latitude
    #[inline]
    pub fn top(&self) -> f64 {
        self.top_right().lat()
    }
    /// Return the left longitude
    #[inline]
    pub fn left(&self) -> f64 {
        self.bottom_left().lng()
    }
    /// Return the right longitude
    #[inline]
    pub fn right(&self) -> f64 {
        self.top_right().lng()
    }

    /// Return the longitude range contained in the bounding box
    #[inline]
    pub fn horizontal_range(&self) -> RangeInclusive<f64> {
        self.left()..=self.right()
    }
    /// Return the latitude range contained in the bounding box
    #[inline]
    pub fn vertical_range(&self) -> RangeInclusive<f64> {
        self.bottom()..=self.top()
    }

    /// Return `true` if the coord is contained in the bounding box
    #[inline]
    pub fn contains_coord(&self, coord: &Coord) -> bool {
        self.vertical_range().contains(&coord.lat())
            && self.horizontal_range().contains(&coord.lng())
    }

    /// Convert the bounding box to a [`geo_types::Rect`].
    /// Note: Converting that back to a [`crate::Zerometry`] will produce a [`crate::Zolygon`]
    #[inline]
    pub fn to_geo(&self) -> geo_types::Rect<f64> {
        geo_types::Rect::new(self.bottom_left().to_geo(), self.top_right().to_geo())
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

impl RelationBetweenShapes<Coord> for BoundingBox {
    #[inline]
    fn relation(&self, other: &Coord, relation: InputRelation) -> OutputRelation {
        if self.contains_coord(other) {
            relation.to_false().make_strict_contains_if_set()
        } else {
            relation.to_false().make_disjoint_if_set()
        }
    }
}

impl RelationBetweenShapes<BoundingBox> for BoundingBox {
    #[inline]
    fn relation(&self, other: &BoundingBox, relation: InputRelation) -> OutputRelation {
        let relation = relation.to_false();

        let self_vertical_range = self.vertical_range();
        let self_horizontal_range = self.horizontal_range();
        let other_vertical_range = other.vertical_range();
        let other_horizontal_range = other.horizontal_range();

        match (
            self_vertical_range.contains(other_vertical_range.start()),
            self_vertical_range.contains(other_vertical_range.end()),
            self_horizontal_range.contains(other_horizontal_range.start()),
            self_horizontal_range.contains(other_horizontal_range.end()),
        ) {
            (true, true, true, true) => relation.make_strict_contains_if_set(),
            (false, false, false, false) => {
                match (
                    other_vertical_range.contains(self_vertical_range.start()),
                    other_vertical_range.contains(self_vertical_range.end()),
                    other_horizontal_range.contains(self_horizontal_range.start()),
                    other_horizontal_range.contains(self_horizontal_range.end()),
                ) {
                    (true, true, true, true) => relation.make_strict_contained_if_set(),
                    (false, false, false, false) => relation.make_disjoint_if_set(),
                    _ => relation.make_intersect_if_set(),
                }
            }
            _ => relation.make_intersect_if_set(),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;

    use super::*;

    #[test]
    fn test_bounding_box_from_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0];
        let bb = unsafe { BoundingBox::from_bytes(cast_slice(&data)) };
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
        unsafe { BoundingBox::from_bytes(cast_slice(&data)) };
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_bytes_panic_on_too_many_point_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        unsafe { BoundingBox::from_bytes(cast_slice(&data)) };
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_bytes_panic_on_too_long_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        unsafe { BoundingBox::from_bytes(cast_slice(&data)) };
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_bytes_panic_on_unaligned_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        unsafe { BoundingBox::from_bytes(&cast_slice(&data)[1..]) };
    }

    #[test]
    fn test_bounding_box_from_slice() {
        let data = [1.0, 2.0, 3.0, 4.0];
        let bb = unsafe { BoundingBox::from_slice(&data) };
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
        unsafe { BoundingBox::from_slice(&data) };
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_slice_panic_on_too_many_point_slice() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        unsafe { BoundingBox::from_slice(&data) };
    }

    #[test]
    #[should_panic]
    fn test_bounding_box_from_slice_panic_on_unordered_points() {
        let data = [1.0, 4.0, 3.0, 2.0];
        unsafe { BoundingBox::from_slice(&data) };
    }

    #[test]
    fn test_bounding_box_contains_coord() {
        let bb = unsafe { BoundingBox::from_slice(&[1.0, 2.0, 3.0, 4.0]) };
        assert!(bb.contains_coord(unsafe { Coord::from_slice(&[2.0, 3.0]) }));
        assert!(!bb.contains_coord(unsafe { Coord::from_slice(&[0.0, 0.0]) }));
    }

    #[test]
    fn test_bounding_box_relation_to_coord() {
        let bb = unsafe { BoundingBox::from_slice(&[0.0, 0.0, 10.0, 10.0]) };
        assert!(bb.contains(unsafe { Coord::from_slice(&[2.0, 3.0]) }));
        assert!(bb.contains(unsafe { Coord::from_slice(&[0.0, 0.0]) }));
        assert!(bb.contains(unsafe { Coord::from_slice(&[10.0, 10.0]) }));
        assert!(bb.disjoint(unsafe { Coord::from_slice(&[11.0, 11.0]) }));
        assert!(bb.disjoint(unsafe { Coord::from_slice(&[-1.0, -1.0]) }));
    }

    #[test]
    fn test_bounding_box_relation_to_bounding_box() {
        let bb = unsafe { BoundingBox::from_slice(&[0.0, 0.0, 10.0, 10.0]) };
        assert!(bb.contains(unsafe { BoundingBox::from_slice(&[1.0, 1.0, 3.0, 3.0]) }));
        assert!(bb.intersects(unsafe { BoundingBox::from_slice(&[-1.0, 0.0, 1.0, 2.0]) }));
        assert!(bb.intersects(unsafe { BoundingBox::from_slice(&[10.0, 0.0, 20.0, 10.0]) }));
        assert!(bb.contains(unsafe { BoundingBox::from_slice(&[0.0, 0.0, 10.0, 10.0]) }));
        assert!(bb.contained(unsafe { BoundingBox::from_slice(&[-1.0, -1.0, 11.0, 11.0]) }));
        assert!(bb.disjoint(unsafe { BoundingBox::from_slice(&[11.0, 11.0, 12.0, 12.0]) }));
    }
}
