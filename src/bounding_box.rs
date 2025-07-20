use core::fmt;
use std::{io::{self, Write}, mem, ops::RangeInclusive};

use geo_types::Point;

use crate::{Coord, Coords, Relation, RelationBetweenShapes};

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

    pub fn from_slice_mut(data: &mut [f64]) -> &mut Self {
        Self::from_coords_mut(Coords::from_slice_mut(data))
    }

    pub fn from_coords(coords: &Coords) -> &Self {
        debug_assert_eq!(coords.len(), 2, "Bounding box must have 2 coordinates but instead got {}", coords.len());
        debug_assert!(coords[0].lng() <= coords[1].lng(), "Bounding box must have the left side before the right side");
        debug_assert!(coords[0].lat() <= coords[1].lat(), "Bounding box must have the bottom side before the top side");
        unsafe { mem::transmute(coords) }
    }

    pub fn from_coords_mut(coords: &mut Coords) -> &mut Self {
        debug_assert_eq!(coords.len(), 2, "Bounding box must have 2 coordinates but instead got {}", coords.len());
        debug_assert!(coords[0].lng() <= coords[1].lng(), "Bounding box must have the left side before the right side");
        debug_assert!(coords[0].lat() <= coords[1].lat(), "Bounding box must have the bottom side before the top side");
        unsafe { mem::transmute(coords) }
    }


    pub fn write_from_geometry(writer: &mut impl Write, mut points: impl Iterator<Item = Point<f64>>) -> Result<(), io::Error> {
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

impl RelationBetweenShapes<Coord> for BoundingBox {
    fn relation(&self, other: &Coord) -> Relation {
        if self.contains_coord(other) {
            Relation::Contains
        } else {
            Relation::Disjoint
        }
    }
}

impl RelationBetweenShapes<BoundingBox> for BoundingBox {
    fn relation(&self, other: &BoundingBox) -> Relation {
        let contains_bottom_left = self.contains_coord(other.bottom_left());
        let contains_top_right = self.contains_coord(other.top_right());
        let contained_bottom_left = other.contains_coord(self.bottom_left());
        let contained_top_right = other.contains_coord(self.top_right());
        if contains_bottom_left && contains_top_right {
            Relation::Contains
        } else if contained_bottom_left && contained_top_right {
            Relation::Contained
        } else if contained_bottom_left || contained_top_right || contains_bottom_left || contains_top_right {
            Relation::Intersects
        } else {
            Relation::Disjoint
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

    #[test]
    fn test_bounding_box_relation_to_coord() {
        let bb = BoundingBox::from_slice(&[0.0, 0.0, 10.0, 10.0]);
        assert_eq!(bb.relation(Coord::from_slice(&[2.0, 3.0])), Relation::Contains);
        assert_eq!(bb.relation(Coord::from_slice(&[0.0, 0.0])), Relation::Contains);
        assert_eq!(bb.relation(Coord::from_slice(&[10.0, 10.0])), Relation::Contains);
        assert_eq!(bb.relation(Coord::from_slice(&[11.0, 11.0])), Relation::Disjoint);
        assert_eq!(bb.relation(Coord::from_slice(&[-1.0, -1.0])), Relation::Disjoint);
    }

    #[test]
    fn test_bounding_box_relation_to_bounding_box() {
        let bb = BoundingBox::from_slice(&[0.0, 0.0, 10.0, 10.0]);
        assert_eq!(bb.relation(BoundingBox::from_slice(&[1.0, 1.0, 3.0, 3.0])), Relation::Contains);
        assert_eq!(bb.relation(BoundingBox::from_slice(&[-1.0, 0.0, 1.0, 2.0])), Relation::Intersects);
        assert_eq!(bb.relation(BoundingBox::from_slice(&[10.0, 0.0, 20.0, 10.0])), Relation::Intersects);
        assert_eq!(bb.relation(BoundingBox::from_slice(&[0.0, 0.0, 10.0, 10.0])), Relation::Contains);
        assert_eq!(bb.relation(BoundingBox::from_slice(&[-1.0, -1.0, 11.0, 11.0])), Relation::Contained);
        assert_eq!(bb.relation(BoundingBox::from_slice(&[11.0, 11.0, 12.0, 12.0])), Relation::Disjoint);
    }
}
