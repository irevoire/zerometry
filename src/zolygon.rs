use core::fmt;
use std::io::{self, Write};

use geo_types::{Geometry, Polygon};

use crate::{
    BoundingBox, COORD_SIZE_IN_BYTES, COORD_SIZE_IN_FLOATS, Coord, Coords, Relation,
    RelationBetweenShapes, Segment, Zerometry, Zoint, ZultiPoints, ZultiPolygon,
};

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
        Self {
            bounding_box,
            coords,
        }
    }

    pub fn from_bytes(data: &'a [u8]) -> Self {
        debug_assert!(
            data.len() % COORD_SIZE_IN_FLOATS == 0,
            "Data length must be a multiple of {COORD_SIZE_IN_FLOATS}"
        );
        debug_assert!(
            data.len() >= COORD_SIZE_IN_FLOATS * 2,
            "Data length must be at least 2 coordinates to hold the bounding box"
        );
        debug_assert!(
            data.as_ptr() as usize % COORD_SIZE_IN_FLOATS == 0,
            "Data must be aligned to {COORD_SIZE_IN_FLOATS}"
        );
        let bounding_box = BoundingBox::from_bytes(&data[0..COORD_SIZE_IN_BYTES * 2]);
        let coords = Coords::from_bytes(&data[COORD_SIZE_IN_BYTES * 2..]);
        Self::new(bounding_box, coords)
    }

    pub fn write_from_geometry(
        writer: &mut impl Write,
        geometry: &Polygon<f64>,
    ) -> Result<(), io::Error> {
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
        self.coords.consecutive_pairs().map(Segment::from_slice)
    }

    pub fn is_empty(&self) -> bool {
        self.coords.len() == 0
    }

    pub fn to_geo(&self) -> geo_types::Polygon<f64> {
        geo_types::Polygon::new(
            self.coords
                .iter()
                .map(|coord| geo_types::Point::new(coord.lng(), coord.lat()))
                .collect(),
            Vec::new(),
        )
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

impl<'a> RelationBetweenShapes<Coord> for Zolygon<'a> {
    fn relation(&self, other: &Coord) -> Relation {
        if self.is_empty() {
            return Relation::Disjoint;
        }

        // If the point is outside of the bounding box, we can early return it's definitely not IN the polygon
        if !self.bounding_box.contains_coord(other) {
            return Relation::Disjoint;
        }

        // To find if a point is in a polygon we draw a ray from outside of the polygon to the point
        // and count the number of times the ray intersects with the polygon. In it's even it means
        // the point is outside of the polygon, otherwise it's inside.
        let end = other;
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
            Relation::Contains
        }
    }
}

impl<'a> RelationBetweenShapes<Zoint<'a>> for Zolygon<'a> {
    fn relation(&self, other: &Zoint<'a>) -> Relation {
        self.relation(other.coord())
    }
}

// We don't need to know if everything is contained, only one point is enough for us.
impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for Zolygon<'a> {
    fn relation(&self, other: &ZultiPoints<'a>) -> Relation {
        // If the bounding boxes are disjoint, the relation must be disjoint, we can early return.
        if self.bounding_box().relation(other.bounding_box()) == Relation::Disjoint {
            return Relation::Disjoint;
        }

        for coord in other.coords().iter() {
            if self.relation(coord) == Relation::Contains {
                return Relation::Contains;
            }
        }
        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for Zolygon<'a> {
    fn relation(&self, other: &Zolygon<'a>) -> Relation {
        #[allow(clippy::if_same_then_else)] // readability
        if self.is_empty() || other.is_empty() {
            return Relation::Disjoint;
        } else if self.bounding_box().relation(other.bounding_box()) == Relation::Disjoint {
            return Relation::Disjoint;
        }

        // To know if two polygons intersect we check if any of the segments of the first polygon intersect with the second polygon.
        // That's O(n^2) but if you know a better algorithm please let me know.
        for segment in self.segments() {
            for other_segment in other.segments() {
                if segment.intersects(&other_segment) {
                    return Relation::Intersects;
                }
            }
        }

        // If we reached this point, the polygons don't intersect. To know if one polygon
        // is contained in the other we check any of his points is contained in the other polygon.
        // safe to unwrap because we checked that the polygons are not empty
        let any = self.coords().iter().next().unwrap();
        if other.relation(any) == Relation::Contains {
            return Relation::Contained;
        }
        let any = other.coords().iter().next().unwrap();
        if self.relation(any) == Relation::Contains {
            return Relation::Contains;
        }

        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<ZultiPolygon<'a>> for Zolygon<'a> {
    fn relation(&self, other: &ZultiPolygon<'a>) -> Relation {
        match other.relation(self) {
            Relation::Contains => Relation::Contained,
            Relation::Contained => Relation::Contains,
            r => r,
        }
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for Zolygon<'a> {
    fn relation(&self, other: &Zerometry<'a>) -> Relation {
        match other.relation(self) {
            Relation::Contains => Relation::Contained,
            Relation::Contained => Relation::Contains,
            r => r,
        }
    }
}

impl<'a> RelationBetweenShapes<Polygon<f64>> for Zolygon<'a> {
    fn relation(&self, other: &Polygon<f64>) -> Relation {
        let mut buffer = Vec::new();
        Zerometry::write_from_geometry(&mut buffer, &Geometry::Polygon(other.clone())).unwrap();
        let other = Zerometry::from_bytes(&buffer).unwrap();
        self.relation(&other)
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for Polygon<f64> {
    fn relation(&self, other: &Zolygon<'a>) -> Relation {
        match other.relation(self) {
            Relation::Contains => Relation::Contained,
            Relation::Contained => Relation::Contains,
            r => r,
        }
    }
}

impl<'a> PartialEq<Polygon<f64>> for Zolygon<'a> {
    fn eq(&self, other: &Polygon<f64>) -> bool {
        if !other.interiors().is_empty() {
            return false;
        }
        self.coords
            .iter()
            .zip(other.exterior().points())
            .all(|(a, b)| a.lng() == b.x() && a.lat() == b.y())
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;
    use geo_types::{LineString, Point};
    use insta::assert_debug_snapshot;

    use super::*;

    #[test]
    fn test_zolygon_binary_format() {
        // 2 coordinates for the bounding box and 3 coordinates for the polygon
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: -10.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: -10.0 },
                    geo_types::Coord { x: 10.0, y: 10.0 },
                    geo_types::Coord { x: 0.0, y: 10.0 },
                    // Here we forgot to close the polygon but it should be done automatically by the geometry library
                    // A polygon MUST be closed
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
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
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(LineString::new(vec![]), Vec::new()),
        )
        .unwrap();
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

    #[test]
    fn test_zolygon_relation_to_zoint() {
        // 2 coordinates for the bounding box and 3 coordinates for the polygon
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 0.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 10.0 },
                    geo_types::Coord { x: 0.0, y: 10.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let zoint_inside_bytes = buffer.len();
        Zoint::write_from_geometry(&mut buffer, &Point::new(5.0, 5.0)).unwrap();
        let zoint_outside_bytes = buffer.len();
        Zoint::write_from_geometry(&mut buffer, &Point::new(15.0, 15.0)).unwrap();

        let zolygon = Zolygon::from_bytes(&buffer[..zoint_inside_bytes]);
        let point_inside = Zoint::from_bytes(&buffer[zoint_inside_bytes..zoint_outside_bytes]);
        let point_outside = Zoint::from_bytes(&buffer[zoint_outside_bytes..]);
        assert_eq!(zolygon.relation(&point_inside), Relation::Contains);
        assert_eq!(zolygon.relation(&point_outside), Relation::Disjoint);
    }

    #[test]
    fn test_zolygon_relation_to_zolygon_intersects_basic() {
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 0.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 10.0 },
                    geo_types::Coord { x: 0.0, y: 10.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let first = buffer.len();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 5.0, y: 5.0 },
                    geo_types::Coord { x: 15.0, y: 5.0 },
                    geo_types::Coord { x: 15.0, y: 15.0 },
                    geo_types::Coord { x: 5.0, y: 15.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let second = buffer.len();
        let first_zolygon = Zolygon::from_bytes(&buffer[..first]);
        let second_zolygon = Zolygon::from_bytes(&buffer[first..second]);
        assert_eq!(
            first_zolygon.relation(&second_zolygon),
            Relation::Intersects
        );
        assert_eq!(
            second_zolygon.relation(&first_zolygon),
            Relation::Intersects
        );
    }

    #[test]
    fn test_zolygon_relation_to_zolygon_intersects_diagonal() {
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 0.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 10.0 },
                    geo_types::Coord { x: 0.0, y: 10.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let first = buffer.len();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 5.0, y: 7.0 },
                    geo_types::Coord { x: 8.0, y: 10.0 },
                    geo_types::Coord { x: 5.0, y: 13.0 },
                    geo_types::Coord { x: 2.0, y: 10.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let second = buffer.len();
        let first_zolygon = Zolygon::from_bytes(&buffer[..first]);
        let second_zolygon = Zolygon::from_bytes(&buffer[first..second]);
        assert_eq!(
            first_zolygon.relation(&second_zolygon),
            Relation::Intersects
        );
        assert_eq!(
            second_zolygon.relation(&first_zolygon),
            Relation::Intersects
        );
    }

    #[test]
    fn test_zolygon_relation_to_zolygon_intersects_on_edge() {
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 0.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 10.0 },
                    geo_types::Coord { x: 0.0, y: 10.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let first = buffer.len();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 10.0, y: 0.0 },
                    geo_types::Coord { x: 15.0, y: 0.0 },
                    geo_types::Coord { x: 15.0, y: 10.0 },
                    geo_types::Coord { x: 10.0, y: 10.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let second = buffer.len();
        let first_zolygon = Zolygon::from_bytes(&buffer[..first]);
        let second_zolygon = Zolygon::from_bytes(&buffer[first..second]);
        assert_eq!(
            first_zolygon.relation(&second_zolygon),
            Relation::Intersects
        );
        assert_eq!(
            second_zolygon.relation(&first_zolygon),
            Relation::Intersects
        );
    }

    #[test]
    fn test_zolygon_relation_to_zolygon_contains() {
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 0.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 10.0 },
                    geo_types::Coord { x: 0.0, y: 10.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let first = buffer.len();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 1.0, y: 1.0 },
                    geo_types::Coord { x: 1.0, y: 9.0 },
                    geo_types::Coord { x: 9.0, y: 9.0 },
                    geo_types::Coord { x: 9.0, y: 1.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let second = buffer.len();
        let first_zolygon = Zolygon::from_bytes(&buffer[..first]);
        let second_zolygon = Zolygon::from_bytes(&buffer[first..second]);
        assert_eq!(first_zolygon.relation(&second_zolygon), Relation::Contains);
        assert_eq!(second_zolygon.relation(&first_zolygon), Relation::Contained);
    }

    // In this test the bounding boxes are disjoint and we can early exit.
    #[test]
    fn test_zolygon_relation_to_zolygon_disjoint_basic() {
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 0.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 10.0 },
                    geo_types::Coord { x: 0.0, y: 10.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let first = buffer.len();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 15.0, y: 15.0 },
                    geo_types::Coord { x: 15.0, y: 25.0 },
                    geo_types::Coord { x: 25.0, y: 25.0 },
                    geo_types::Coord { x: 25.0, y: 15.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let second = buffer.len();
        let first_zolygon = Zolygon::from_bytes(&buffer[..first]);
        let second_zolygon = Zolygon::from_bytes(&buffer[first..second]);
        assert_eq!(first_zolygon.relation(&second_zolygon), Relation::Disjoint);
        assert_eq!(second_zolygon.relation(&first_zolygon), Relation::Disjoint);
    }

    #[test]
    fn test_zolygon_relation_to_zolygon_disjoint_near() {
        let mut buffer = Vec::new();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 0.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 0.0 },
                    geo_types::Coord { x: 10.0, y: 10.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let first = buffer.len();
        Zolygon::write_from_geometry(
            &mut buffer,
            &Polygon::new(
                LineString::new(vec![
                    geo_types::Coord { x: 0.0, y: 1.0 },
                    geo_types::Coord { x: 10.0, y: 11.0 },
                    geo_types::Coord { x: 0.0, y: 11.0 },
                ]),
                Vec::new(),
            ),
        )
        .unwrap();
        let second = buffer.len();
        let first_zolygon = Zolygon::from_bytes(&buffer[..first]);
        let second_zolygon = Zolygon::from_bytes(&buffer[first..second]);
        assert_eq!(first_zolygon.relation(&second_zolygon), Relation::Disjoint);
        assert_eq!(second_zolygon.relation(&first_zolygon), Relation::Disjoint);
    }

    // Prop test ensuring we can round trip from a polygon to a zolygon and back to a polygon
    proptest::proptest! {
        #[test]
        fn test_zolygon_round_trip(points: Vec<(f64, f64)>) {
            let polygon = Polygon::new(LineString::new(points.iter().map(|(x, y)| geo_types::Coord { x: *x, y: *y }).collect()), Vec::new());
            let mut buffer = Vec::new();
            Zolygon::write_from_geometry(&mut buffer, &polygon).unwrap();
            let zolygon = Zolygon::from_bytes(&buffer);
            assert_eq!(zolygon, polygon);
        }
    }
}
