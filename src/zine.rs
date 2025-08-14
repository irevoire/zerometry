use core::fmt;
use std::io::{self, Write};

use geo::{LineString, Point};

use crate::{
    BoundingBox, COORD_SIZE_IN_BYTES, Coords, Relation, RelationBetweenShapes, Segment, Zerometry,
    Zoint, Zolygon, ZultiPoints, ZultiPolygon, zulti_lines::ZultiLines,
};

#[derive(Clone, Copy)]
pub struct Zine<'a> {
    bounding_box: &'a BoundingBox,
    coords: &'a Coords,
}

impl<'a> Zine<'a> {
    pub fn new(bounding_box: &'a BoundingBox, coords: &'a Coords) -> Self {
        Self {
            bounding_box,
            coords,
        }
    }

    pub fn from_bytes(data: &'a [u8]) -> Self {
        let bounding_box = BoundingBox::from_bytes(&data[0..COORD_SIZE_IN_BYTES * 2]);
        let coords = Coords::from_bytes(&data[COORD_SIZE_IN_BYTES * 2..]);
        Self::new(bounding_box, coords)
    }

    pub fn write_from_geometry(
        writer: &mut impl Write,
        geometry: &LineString<f64>,
    ) -> Result<(), io::Error> {
        BoundingBox::write_from_geometry(
            writer,
            geometry.0.iter().map(|coord| Point::new(coord.x, coord.y)),
        )?;
        for point in geometry.0.iter() {
            writer.write_all(&point.x.to_ne_bytes())?;
            writer.write_all(&point.y.to_ne_bytes())?;
        }
        Ok(())
    }

    pub fn bounding_box(&self) -> &'a BoundingBox {
        self.bounding_box
    }

    pub fn len(&self) -> usize {
        self.coords.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn coords(&self) -> &'a Coords {
        self.coords
    }

    pub fn segments(&self) -> impl Iterator<Item = Segment<'a>> {
        self.coords.consecutive_pairs().map(Segment::from_slice)
    }

    pub fn to_geo(self) -> geo_types::LineString<f64> {
        geo_types::LineString::new(
            self.coords
                .iter()
                .map(|coord| geo_types::Coord {
                    x: coord.lng(),
                    y: coord.lat(),
                })
                .collect(),
        )
    }
}

impl<'a> fmt::Debug for Zine<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Zine")
            .field("bounding_box", &self.bounding_box)
            .field(
                "points",
                &self.coords.iter().map(Zoint::new).collect::<Vec<_>>(),
            )
            .finish()
    }
}

// A point cannot contains or intersect with anything
impl<'a> RelationBetweenShapes<Zoint<'a>> for Zine<'a> {
    fn relation(&self, _other: &Zoint<'a>) -> Relation {
        Relation::Disjoint
    }
}

// A point cannot contains or intersect with anything
impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for Zine<'a> {
    fn relation(&self, _other: &ZultiPoints<'a>) -> Relation {
        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<Zine<'a>> for Zine<'a> {
    fn relation(&self, other: &Zine<'a>) -> Relation {
        if self.is_empty()
            || other.is_empty()
            || self.bounding_box().relation(other.bounding_box()) == Relation::Disjoint
        {
            return Relation::Disjoint;
        }

        for lhs in self.segments() {
            for rhs in other.segments() {
                if lhs.intersects(&rhs) {
                    return Relation::Intersects;
                }
            }
        }

        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<ZultiLines<'a>> for Zine<'a> {
    fn relation(&self, other: &ZultiLines<'a>) -> Relation {
        // no need to revert the contains/contained as this connot happens with lines
        other.relation(self)
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for Zine<'a> {
    fn relation(&self, other: &Zolygon<'a>) -> Relation {
        if self.is_empty()
            || other.is_empty()
            || self.bounding_box().relation(other.bounding_box()) == Relation::Disjoint
        {
            return Relation::Disjoint;
        }

        // To know if a line and a polygon intersect we check if any of our segments intersect with the polygon.
        // That's O(n^2) but if you know a better algorithm please let me know.
        for segment in self.segments() {
            for other_segment in other.segments() {
                if segment.intersects(&other_segment) {
                    return Relation::Intersects;
                }
            }
        }

        // If we reached this point, the line and polygon don't intersect. To know if the line
        // is contained in the polygon we check if any of its points is contained in the polygon.
        // safe to unwrap because we checked that the polygon and line are not empty
        let any = self.coords().iter().next().unwrap();
        if other.relation(any) == Relation::Contains {
            return Relation::Contained;
        }

        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<ZultiPolygon<'a>> for Zine<'a> {
    fn relation(&self, other: &ZultiPolygon<'a>) -> Relation {
        if self.is_empty()
            || other.is_empty()
            || self.bounding_box().relation(other.bounding_box()) == Relation::Disjoint
        {
            return Relation::Disjoint;
        }

        for polygon in other.polygons() {
            match self.relation(&polygon) {
                Relation::Intersects => return Relation::Intersects,
                Relation::Contains => return Relation::Contains,
                _ => (),
            }
        }

        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for Zine<'a> {
    fn relation(&self, other: &Zerometry<'a>) -> Relation {
        match other {
            Zerometry::Point(zoint) => self.relation(zoint),
            Zerometry::MultiPoints(zulti_points) => self.relation(zulti_points),
            Zerometry::Line(zine) => self.relation(zine),
            Zerometry::MultiLines(zulti_lines) => self.relation(zulti_lines),
            Zerometry::Polygon(zolygon) => self.relation(zolygon),
            Zerometry::MultiPolygon(zulti_polygon) => self.relation(zulti_polygon),
            Zerometry::Collection(zollection) => todo!(),
        }
    }
}

impl<'a> PartialEq<LineString<f64>> for Zine<'a> {
    fn eq(&self, other: &LineString<f64>) -> bool {
        self.coords
            .iter()
            .zip(other.0.iter())
            .all(|(a, b)| a.lng() == b.x && a.lat() == b.y)
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;
    use geo_types::Point;
    use insta::assert_compact_debug_snapshot;

    use super::*;

    #[test]
    fn test_zine_binary_format() {
        let mut buffer = Vec::new();
        Zine::write_from_geometry(
            &mut buffer,
            &LineString::from(vec![Point::new(1.0, 2.0), Point::new(3.0, 4.0)]),
        )
        .unwrap();
        let input: &[f64] = cast_slice(&buffer);
        assert_compact_debug_snapshot!(input, @"[1.0, 2.0, 3.0, 4.0, 1.0, 2.0, 3.0, 4.0]");
        let zulti_points = Zine::from_bytes(&buffer);
        assert_compact_debug_snapshot!(zulti_points.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 1.0, y: 2.0 }, top_right: Coord { x: 3.0, y: 4.0 } }");
        assert_compact_debug_snapshot!(zulti_points.coords(), @"[Coord { x: 1.0, y: 2.0 }, Coord { x: 3.0, y: 4.0 }]");
    }

    // Prop test ensuring we can round trip from a multi-point to a zulti-points and back to a multi-point
    proptest::proptest! {
        #[test]
        fn test_zine_points_round_trip(points: Vec<(f64, f64)>) {
            let multi_point = LineString::from(points);
            let mut buffer = Vec::new();
            Zine::write_from_geometry(&mut buffer, &multi_point).unwrap();
            let zulti_points = Zine::from_bytes(&buffer);
            assert_eq!(zulti_points, multi_point);
        }
    }
}
