use core::fmt;
use std::io::{Write, self};

use geo_types::Polygon;

use crate::{BoundingBox, Coord, Coords, Relation, RelationBetweenShapes, Segment, Zoint, COORD_SIZE_IN_BYTES, COORD_SIZE_IN_FLOATS};

/// A polygon is a closed shape defined by a list of coordinates.
///
/// The polygon is represented by a bounding box and a list of coordinates.
///
/// The coordinates are stored in a `Coords` struct, which is a slice of `f64` values.
/// The first and last coordinates must be the same.
/// Don't support holes.
#[derive(Clone, Copy)]
pub struct Zolygon<'a> {
    bounding_box: &'a BoundingBox,
    coords: &'a Coords,
}

impl<'a> Zolygon<'a> {
    pub fn new(bounding_box: &'a BoundingBox, coords: &'a Coords) -> Self {
        Self { bounding_box, coords }
    }

    pub fn from_bytes(data: &'a [u8]) -> Self {
        debug_assert!(data.len() % COORD_SIZE_IN_FLOATS == 0, "Data length must be a multiple of {}", COORD_SIZE_IN_FLOATS);
        debug_assert!(data.len() >= COORD_SIZE_IN_FLOATS * 2, "Data length must be at least 2 coordinates to hold the bounding box");
        debug_assert!(data.as_ptr() as usize % COORD_SIZE_IN_FLOATS == 0, "Data must be aligned to {}", COORD_SIZE_IN_FLOATS);
        let bounding_box = BoundingBox::from_bytes(&data[0..COORD_SIZE_IN_BYTES * 2]);
        let coords = Coords::from_bytes(&data[COORD_SIZE_IN_BYTES * 2..]);
        Self::new(bounding_box, coords)
    }

    pub fn write_from_geometry(writer: &mut impl Write, geometry: &Polygon<f64>) -> Result<(), io::Error> {
        BoundingBox::write_from_geometry(writer, geometry.exterior().points())?;

        for point in geometry.exterior().points() {
            writer.write_all(&point.x().to_ne_bytes())?;
            writer.write_all(&point.y().to_ne_bytes())?;
        }

        Ok(())   
    }

    pub fn bounding_box(&self) -> &'a BoundingBox {
        self.bounding_box
    }

    pub fn coords(&self) -> &'a Coords {
        self.coords
    }

    pub fn segments(&self) -> impl Iterator<Item = Segment<'a>> {
        self.coords.consecutive_pairs().map(|chunk| Segment::from_slice(chunk))
    }
}

impl<'a> fmt::Debug for Zolygon<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Zolygon")
            .field("bounding_box", &self.bounding_box)
            .field("coords", &self.coords)
            .finish()
    }
}

impl<'a> RelationBetweenShapes<Zoint<'a>> for Zolygon<'a> {
    fn relation(&self, other: &Zoint<'a>) -> Relation {
        // If the point is outside of the bounding box, we can early return it's definitely not IN the polygon
        if !self.bounding_box.contains_coord(other.coord()) {
            return Relation::Disjoint;
        }

        // To find if a point is in a polygon we draw a ray from outside of the polygon to the point
        // and count the number of times the ray intersects with the polygon. In it's even it means
        // the point is outside of the polygon, otherwise it's inside.
        let end = other.coord();
        let mut buffer = [0.0; COORD_SIZE_IN_FLOATS];
        let start = Coord::from_slice_mut(&mut buffer);
        *start.lng_mut() = self.bounding_box.left();
        *start.lat_mut() = end.lat();
        let ray = Segment::from_coord_pair(start, end);

        let mut intersections = 0;
        for segment in self.segments() {
            // TODO: Since the ray is horizontal we could optimize this by checking only the lng
            if segment.intersects(&ray) {
                intersections += 1;
            }
        }

        if intersections % 2 == 0 {
            Relation::Disjoint
        } else {
            Relation::Intersects
        }
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;
    use geo_types::LineString;
    use insta::assert_debug_snapshot;

    use super::*;

    #[test]
    fn test_zolygon_binary_format() {
        // 2 coordinates for the bounding box and 3 coordinates for the polygon
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(&mut buffer, &Polygon::new(LineString::new(vec![
            geo_types::Coord {x: -10.0, y: 0.0},
            geo_types::Coord {x: 10.0, y: -10.0},
            geo_types::Coord {x: 10.0, y: 10.0},
            geo_types::Coord {x: 0.0, y: 10.0},
            // Here we forgot to close the polygon but it should be done automatically by the geometry library
            // A polygon MUST be closed
        ]), Vec::new())).unwrap();
        let input: &[f64] = cast_slice(&buffer);
        assert_debug_snapshot!(input, @r"
        [
            -10.0,
            -10.0,
            10.0,
            10.0,
            -10.0,
            0.0,
            10.0,
            -10.0,
            10.0,
            10.0,
            0.0,
            10.0,
            -10.0,
            0.0,
        ]
        ");
        let zolygon = Zolygon::from_bytes(&buffer);
        insta::assert_debug_snapshot!(zolygon.bounding_box(), @r"
        BoundingBox {
            bottom_left: Coord {
                x: -10.0,
                y: -10.0,
            },
            top_right: Coord {
                x: 10.0,
                y: 10.0,
            },
        }
        ");
        insta::assert_debug_snapshot!(zolygon.coords(), @r"
        [
            Coord {
                x: -10.0,
                y: 0.0,
            },
            Coord {
                x: 10.0,
                y: -10.0,
            },
            Coord {
                x: 10.0,
                y: 10.0,
            },
            Coord {
                x: 0.0,
                y: 10.0,
            },
            Coord {
                x: -10.0,
                y: 0.0,
            },
        ]
        ");
    }


    #[test]
    fn test_zolygon_empty_binary_format() {
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(&mut buffer, &Polygon::new(LineString::new(vec![]), Vec::new())).unwrap();
        let input: &[f64] = cast_slice(&buffer);
        assert_debug_snapshot!(input, @r"
        [
            0.0,
            0.0,
            0.0,
            0.0,
        ]
        ");
        let zolygon = Zolygon::from_bytes(&buffer);
        insta::assert_debug_snapshot!(zolygon.bounding_box(), @r"
        BoundingBox {
            bottom_left: Coord {
                x: 0.0,
                y: 0.0,
            },
            top_right: Coord {
                x: 0.0,
                y: 0.0,
            },
        }
        ");
        insta::assert_debug_snapshot!(zolygon.coords(), @"[]");
    }

    /*
    #[test]
    fn test_zolygon_relation_to_zoint() {
        // 2 coordinates for the bounding box and 3 coordinates for the polygon
        let mut buffer = Vec::new();
        let polygon = Zolygon::write_from_geometry(&mut buffer, &Polygon::new(LineString::new(vec![
            geo_types::Coord {x: 0.0, y: 0.0},
            geo_types::Coord {x: 10.0, y: 0.0},
            geo_types::Coord {x: 10.0, y: 10.0},
            geo_types::Coord {x: 0.0, y: 10.0},
        ]), Vec::new())).unwrap();
        let point = Zoint::new(&Coord::new(5.0, 5.0));
        assert_eq!(polygon.relation(&point), Relation::Intersects);
    }
    */
}
