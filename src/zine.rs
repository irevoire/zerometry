use core::fmt;
use std::io::{self, Write};

use geo::{LineString, Point};

use crate::{
    BoundingBox, COORD_SIZE_IN_BYTES, Coords, InputRelation, OutputRelation, RelationBetweenShapes,
    Segment, Zerometry, Zoint, Zolygon, ZultiPoints, ZultiPolygons, zulti_lines::ZultiLines,
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
    fn relation(&self, _other: &Zoint<'a>, relation: InputRelation) -> OutputRelation {
        relation.to_false().make_disjoint_if_set()
    }
}

// A point cannot contains or intersect with anything
impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for Zine<'a> {
    fn relation(&self, _other: &ZultiPoints<'a>, relation: InputRelation) -> OutputRelation {
        relation.to_false().make_disjoint_if_set()
    }
}

impl<'a> RelationBetweenShapes<Zine<'a>> for Zine<'a> {
    fn relation(&self, other: &Zine<'a>, relation: InputRelation) -> OutputRelation {
        let relation = relation.to_false();
        if self.is_empty() || other.is_empty() || self.bounding_box().disjoint(other.bounding_box())
        {
            return relation.make_disjoint_if_set();
        }

        for lhs in self.segments() {
            for rhs in other.segments() {
                if lhs.intersects(&rhs) {
                    return relation.make_intersect_if_set();
                }
            }
        }

        relation.make_disjoint_if_set()
    }
}

impl<'a> RelationBetweenShapes<ZultiLines<'a>> for Zine<'a> {
    fn relation(&self, other: &ZultiLines<'a>, relation: InputRelation) -> OutputRelation {
        // no need to revert the contains/contained as this cannot happens with lines
        other.relation(self, relation)
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for Zine<'a> {
    fn relation(&self, other: &Zolygon<'a>, relation: InputRelation) -> OutputRelation {
        if self.is_empty() || other.is_empty() || self.bounding_box().disjoint(other.bounding_box())
        {
            return relation.to_false().make_disjoint_if_set();
        }

        // To know if a line and a polygon intersect we check if any of our segments intersect with the polygon.
        // That's O(n^2) but if you know a better algorithm please let me know.
        for segment in self.segments() {
            for other_segment in other.segments() {
                if segment.intersects(&other_segment) {
                    return relation.to_false().make_intersect_if_set();
                }
            }
        }

        // If we reached this point, the line and polygon don't intersect. To know if the line
        // is contained in the polygon we check if any of its points is contained in the polygon.
        // safe to unwrap because we checked that the polygon and line are not empty
        let any = self.coords().iter().next().unwrap();
        if other.contains(any) {
            return relation.to_false().make_strict_contained_if_set();
        }

        relation.to_false().make_disjoint_if_set()
    }
}

impl<'a> RelationBetweenShapes<ZultiPolygons<'a>> for Zine<'a> {
    fn relation(&self, other: &ZultiPolygons<'a>, relation: InputRelation) -> OutputRelation {
        let mut output = relation.to_false();
        if self.is_empty() || other.is_empty() || self.bounding_box().disjoint(other.bounding_box())
        {
            return output.make_disjoint_if_set();
        }

        for polygon in other.polygons() {
            output |= self.relation(&polygon, relation.strip_disjoint());

            if output.any_relation() && relation.early_exit {
                return output;
            }
        }

        if output.any_relation() {
            output
        } else {
            output.make_disjoint_if_set()
        }
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for Zine<'a> {
    fn relation(&self, other: &Zerometry<'a>, relation: InputRelation) -> OutputRelation {
        match other {
            Zerometry::Point(zoint) => self.relation(zoint, relation),
            Zerometry::MultiPoints(zulti_points) => self.relation(zulti_points, relation),
            Zerometry::Line(zine) => self.relation(zine, relation),
            Zerometry::MultiLines(zulti_lines) => self.relation(zulti_lines, relation),
            Zerometry::Polygon(zolygon) => self.relation(zolygon, relation),
            Zerometry::MultiPolygon(zulti_polygon) => self.relation(zulti_polygon, relation),
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
    use geo::{MultiPolygon, coord, polygon};
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

    #[test]
    fn test_zine_in_polygon() {
        let line = LineString::new(vec![
            coord! { x: 0.4, y: 0.4},
            coord! { x: 0.6, y: 0.4},
            coord! { x: 0.6, y: 0.6},
            coord! { x: 0.4, y: 0.6},
        ]);
        let polygon = polygon![
             (x: 0., y: 0.),
             (x: 1., y: 0.),
             (x: 1., y: 1.),
             (x: 0., y: 1.),
        ];

        let mut buf = Vec::new();
        Zine::write_from_geometry(&mut buf, &line).unwrap();
        let zine = Zine::from_bytes(&buf);

        let mut buf = Vec::new();
        Zolygon::write_from_geometry(&mut buf, &polygon).unwrap();
        let zolygon = Zolygon::from_bytes(&buf);

        assert_compact_debug_snapshot!(zine.all_relation(&zolygon), @"OutputRelation { contains: Some(true), strict_contains: Some(true), contained: Some(false), strict_contained: Some(false), intersect: Some(false), disjoint: Some(false) }");
    }

    #[test]
    fn test_zine_and_multipolygon() {
        let line = LineString::new(vec![
            coord! { x: 0.4, y: 0.4},
            coord! { x: 0.6, y: 0.4},
            coord! { x: 0.6, y: 0.6},
            coord! { x: 0.4, y: 0.6},
        ]);
        let inside = polygon![
             (x: 0., y: 0.),
             (x: 1., y: 0.),
             (x: 1., y: 1.),
             (x: 0., y: 1.),
        ];
        let outside = polygon![
             (x: 5., y: 5.),
             (x: 6., y: 5.),
             (x: 6., y: 6.),
             (x: 5., y: 6.),
        ];
        let intersect = polygon![
             (x: 0.5, y: 0.5),
             (x: 0.6, y: 0.5),
             (x: 0.6, y: 0.6),
             (x: 0.5, y: 0.6),
        ];
        let multi_polygons_inside = MultiPolygon::new(vec![inside.clone()]);
        let multi_polygons_outside = MultiPolygon::new(vec![outside.clone()]);
        let multi_polygons_intersect = MultiPolygon::new(vec![intersect.clone()]);
        let multi_polygons_in_and_out = MultiPolygon::new(vec![inside.clone(), outside.clone()]);
        let multi_polygons_all =
            MultiPolygon::new(vec![inside.clone(), outside.clone(), intersect.clone()]);

        let mut buf = Vec::new();
        Zine::write_from_geometry(&mut buf, &line).unwrap();
        let zine = Zine::from_bytes(&buf);

        let mut buf = Vec::new();
        ZultiPolygons::write_from_geometry(&mut buf, &multi_polygons_inside).unwrap();
        let inside = ZultiPolygons::from_bytes(&buf);
        assert_compact_debug_snapshot!(zine.all_relation(&inside ), @"OutputRelation { contains: Some(true), strict_contains: Some(true), contained: Some(false), strict_contained: Some(false), intersect: Some(false), disjoint: Some(false) }");

        let mut buf = Vec::new();
        ZultiPolygons::write_from_geometry(&mut buf, &multi_polygons_outside).unwrap();
        let multi_polygons_outside = ZultiPolygons::from_bytes(&buf);
        assert_compact_debug_snapshot!(zine.all_relation(&multi_polygons_outside), @"OutputRelation { contains: Some(false), strict_contains: Some(false), contained: Some(false), strict_contained: Some(false), intersect: Some(false), disjoint: Some(true) }");

        let mut buf = Vec::new();
        ZultiPolygons::write_from_geometry(&mut buf, &multi_polygons_intersect).unwrap();
        let multi_polygons_intersect = ZultiPolygons::from_bytes(&buf);
        assert_compact_debug_snapshot!(zine.all_relation(&multi_polygons_intersect), @"OutputRelation { contains: Some(false), strict_contains: Some(false), contained: Some(false), strict_contained: Some(false), intersect: Some(true), disjoint: Some(false) }");

        let mut buf = Vec::new();
        ZultiPolygons::write_from_geometry(&mut buf, &multi_polygons_in_and_out).unwrap();
        let multi_polygons_in_and_out = ZultiPolygons::from_bytes(&buf);
        assert_compact_debug_snapshot!(zine.all_relation(&multi_polygons_in_and_out), @"OutputRelation { contains: Some(true), strict_contains: Some(true), contained: Some(false), strict_contained: Some(false), intersect: Some(false), disjoint: Some(false) }");

        let mut buf = Vec::new();
        ZultiPolygons::write_from_geometry(&mut buf, &multi_polygons_all).unwrap();
        let multi_polygons_all = ZultiPolygons::from_bytes(&buf);
        assert_compact_debug_snapshot!(zine.all_relation(&multi_polygons_all), @"OutputRelation { contains: Some(true), strict_contains: Some(true), contained: Some(false), strict_contained: Some(false), intersect: Some(true), disjoint: Some(false) }");
        assert_compact_debug_snapshot!(zine.any_relation(&multi_polygons_all), @"OutputRelation { contains: Some(true), strict_contains: Some(true), contained: Some(false), strict_contained: Some(false), intersect: Some(false), disjoint: Some(false) }");
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
