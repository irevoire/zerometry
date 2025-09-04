#![doc = include_str!("../README.md")]

mod bounding_box;
mod coord;
mod coords;
mod relation;
mod segment;
#[cfg(test)]
mod test;
mod zine;
mod zoint;
mod zollection;
mod zolygon;
mod zulti_lines;
mod zulti_points;
mod zulti_polygons;

use std::mem;

pub use bounding_box::BoundingBox;
pub use coord::Coord;
pub(crate) use coord::{COORD_SIZE_IN_BYTES, COORD_SIZE_IN_FLOATS};
pub use coords::Coords;
use geo::LineString;
use geo_types::{Geometry, MultiPolygon, Polygon};
pub use relation::{InputRelation, OutputRelation, RelationBetweenShapes};
pub use segment::Segment;
pub use zine::Zine;
pub use zoint::Zoint;
pub use zollection::Zollection;
pub use zolygon::Zolygon;
pub use zulti_lines::ZultiLines;
pub use zulti_points::ZultiPoints;
pub use zulti_polygons::ZultiPolygons;

/// Main structure of this crate, this is the equivalent of a [`geo_types::Geometry`] but serialized.
#[derive(Debug, Clone, Copy)]
pub enum Zerometry<'a> {
    Point(Zoint<'a>),
    MultiPoints(ZultiPoints<'a>),
    Line(Zine<'a>),
    MultiLines(ZultiLines<'a>),
    Polygon(Zolygon<'a>),
    MultiPolygon(ZultiPolygons<'a>),
    Collection(Zollection<'a>),
}

impl<'a> Zerometry<'a> {
    /// Create a `Zerometry` from a slice of bytes.
    /// See [`Self::write_from_geometry`] to create the slice of bytes.
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, std::io::Error> {
        let tag = u64::from_ne_bytes(data[..mem::size_of::<u64>()].try_into().unwrap());
        let data = &data[mem::size_of::<u64>()..];
        match tag {
            0 => Ok(Zerometry::Point(Zoint::from_bytes(data))),
            1 => Ok(Zerometry::MultiPoints(ZultiPoints::from_bytes(data))),
            2 => Ok(Zerometry::Polygon(Zolygon::from_bytes(data))),
            3 => Ok(Zerometry::MultiPolygon(ZultiPolygons::from_bytes(data))),
            // They're located after because it would be a db-breaking to edit the already existing tags
            4 => Ok(Zerometry::Line(Zine::from_bytes(data))),
            5 => Ok(Zerometry::MultiLines(ZultiLines::from_bytes(data))),
            6 => Ok(Zerometry::Collection(Zollection::from_bytes(data))),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid zerometry tag",
            )),
        }
    }

    /// Convert the specified [`geo_types::Geometry`] to a valid [`Zerometry`] slice of bytes in the input buffer.
    /// This is a destructive operation, the original geometry cannot be recreated as-is from the outputted zerometry:
    /// - The Line, Triangle and Rectangle gets converted respectively to Zine and Zolygon
    /// - The collections are flattened to a collection containing one multipoints, one multipolygons and one multilines.
    pub fn write_from_geometry(
        writer: &mut Vec<u8>,
        geometry: &Geometry<f64>,
    ) -> Result<(), std::io::Error> {
        // to stay aligned on 64 bits we must add the tag as a u64
        match geometry {
            Geometry::Point(point) => {
                writer.extend_from_slice(&0_u64.to_ne_bytes());
                Zoint::write_from_geometry(writer, point)?;
            }
            Geometry::MultiPoint(multi_point) => {
                writer.extend_from_slice(&1_u64.to_ne_bytes());
                ZultiPoints::write_from_geometry(writer, multi_point)?;
            }
            Geometry::Polygon(polygon) => {
                writer.extend_from_slice(&2_u64.to_ne_bytes());
                Zolygon::write_from_geometry(writer, polygon)?;
            }
            Geometry::MultiPolygon(multi_polygon) => {
                writer.extend_from_slice(&3_u64.to_ne_bytes());
                ZultiPolygons::write_from_geometry(writer, multi_polygon)?;
            }
            Geometry::LineString(line_string) => {
                writer.extend_from_slice(&4_u64.to_ne_bytes());
                Zine::write_from_geometry(writer, line_string)?;
            }
            Geometry::MultiLineString(multi_line_string) => {
                writer.extend_from_slice(&5_u64.to_ne_bytes());
                ZultiLines::write_from_geometry(writer, multi_line_string)?;
            }
            Geometry::GeometryCollection(collection) => {
                writer.extend_from_slice(&6_u64.to_ne_bytes());
                Zollection::write_from_geometry(writer, collection)?;
            }
            // Should never happens since we're working with geogson in meilisearch
            Geometry::Line(line) => {
                let line = LineString::new(vec![line.start, line.end]);
                Self::write_from_geometry(writer, &line.into())?;
            }
            Geometry::Rect(rect) => {
                Self::write_from_geometry(writer, &rect.to_polygon().into())?;
            }
            Geometry::Triangle(triangle) => {
                Self::write_from_geometry(writer, &triangle.to_polygon().into())?;
            }
        }
        Ok(())
    }

    pub fn to_point(&self) -> Option<Zoint> {
        match self {
            Zerometry::Point(a) => Some(*a),
            _ => None,
        }
    }

    pub fn to_multi_points(&self) -> Option<ZultiPoints> {
        match self {
            Zerometry::MultiPoints(a) => Some(*a),
            _ => None,
        }
    }

    pub fn to_line(&self) -> Option<Zine> {
        match self {
            Zerometry::Line(a) => Some(*a),
            _ => None,
        }
    }

    pub fn to_zulti_lines(&self) -> Option<ZultiLines> {
        match self {
            Zerometry::MultiLines(a) => Some(*a),
            _ => None,
        }
    }

    pub fn to_polygon(&self) -> Option<Zolygon> {
        match self {
            Zerometry::Polygon(a) => Some(*a),
            _ => None,
        }
    }

    pub fn to_multi_polygon(&self) -> Option<ZultiPolygons> {
        match self {
            Zerometry::MultiPolygon(a) => Some(*a),
            _ => None,
        }
    }

    pub fn to_geo(&self) -> geo_types::Geometry<f64> {
        match self {
            Zerometry::Point(a) => Geometry::Point(a.to_geo()),
            Zerometry::MultiPoints(a) => Geometry::MultiPoint(a.to_geo()),
            Zerometry::Line(a) => Geometry::LineString(a.to_geo()),
            Zerometry::MultiLines(a) => Geometry::MultiLineString(a.to_geo()),
            Zerometry::Polygon(a) => Geometry::Polygon(a.to_geo()),
            Zerometry::MultiPolygon(a) => Geometry::MultiPolygon(a.to_geo()),
            Zerometry::Collection(zollection) => Geometry::GeometryCollection(zollection.to_geo()),
        }
    }
}

impl<'a> From<Zoint<'a>> for Zerometry<'a> {
    fn from(point: Zoint<'a>) -> Self {
        Zerometry::Point(point)
    }
}

impl<'a> From<ZultiPoints<'a>> for Zerometry<'a> {
    fn from(points: ZultiPoints<'a>) -> Self {
        Zerometry::MultiPoints(points)
    }
}

impl<'a> From<Zolygon<'a>> for Zerometry<'a> {
    fn from(polygon: Zolygon<'a>) -> Self {
        Zerometry::Polygon(polygon)
    }
}

impl<'a> From<ZultiPolygons<'a>> for Zerometry<'a> {
    fn from(polygon: ZultiPolygons<'a>) -> Self {
        Zerometry::MultiPolygon(polygon)
    }
}

impl<'a> RelationBetweenShapes<Zoint<'a>> for Zerometry<'a> {
    fn relation(&self, other: &Zoint, relation: InputRelation) -> OutputRelation {
        match self {
            Zerometry::Point(a) => a.relation(other, relation),
            Zerometry::MultiPoints(a) => a.relation(other, relation),
            Zerometry::Line(a) => a.relation(other, relation),
            Zerometry::MultiLines(a) => a.relation(other, relation),
            Zerometry::Polygon(a) => a.relation(other, relation),
            Zerometry::MultiPolygon(a) => a.relation(other, relation),
            Zerometry::Collection(a) => a.relation(other, relation),
        }
    }
}

impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for Zerometry<'a> {
    fn relation(&self, other: &ZultiPoints, relation: InputRelation) -> OutputRelation {
        match self {
            Zerometry::Point(a) => a.relation(other, relation),
            Zerometry::MultiPoints(a) => a.relation(other, relation),
            Zerometry::Line(a) => a.relation(other, relation),
            Zerometry::MultiLines(a) => a.relation(other, relation),
            Zerometry::Polygon(a) => a.relation(other, relation),
            Zerometry::MultiPolygon(a) => a.relation(other, relation),
            Zerometry::Collection(a) => a.relation(other, relation),
        }
    }
}

impl<'a> RelationBetweenShapes<Zine<'a>> for Zerometry<'a> {
    fn relation(&self, other: &Zine, relation: InputRelation) -> OutputRelation {
        match self {
            Zerometry::Point(a) => a.relation(other, relation),
            Zerometry::MultiPoints(a) => a.relation(other, relation),
            Zerometry::MultiLines(a) => a.relation(other, relation),
            Zerometry::Line(a) => a.relation(other, relation),
            Zerometry::Polygon(a) => a.relation(other, relation),
            Zerometry::MultiPolygon(a) => a.relation(other, relation),
            Zerometry::Collection(a) => a.relation(other, relation),
        }
    }
}

impl<'a> RelationBetweenShapes<ZultiLines<'a>> for Zerometry<'a> {
    fn relation(&self, other: &ZultiLines, relation: InputRelation) -> OutputRelation {
        match self {
            Zerometry::Point(a) => a.relation(other, relation),
            Zerometry::MultiPoints(a) => a.relation(other, relation),
            Zerometry::MultiLines(a) => a.relation(other, relation),
            Zerometry::Line(a) => a.relation(other, relation),
            Zerometry::Polygon(a) => a.relation(other, relation),
            Zerometry::MultiPolygon(a) => a.relation(other, relation),
            Zerometry::Collection(a) => a.relation(other, relation),
        }
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for Zerometry<'a> {
    fn relation(&self, other: &Zolygon, relation: InputRelation) -> OutputRelation {
        match self {
            Zerometry::Point(a) => a.relation(other, relation),
            Zerometry::MultiPoints(a) => a.relation(other, relation),
            Zerometry::MultiLines(a) => a.relation(other, relation),
            Zerometry::Line(a) => a.relation(other, relation),
            Zerometry::Polygon(a) => a.relation(other, relation),
            Zerometry::MultiPolygon(a) => a.relation(other, relation),
            Zerometry::Collection(a) => a.relation(other, relation),
        }
    }
}

impl<'a> RelationBetweenShapes<ZultiPolygons<'a>> for Zerometry<'a> {
    fn relation(&self, other: &ZultiPolygons, relation: InputRelation) -> OutputRelation {
        match self {
            Zerometry::Point(a) => a.relation(other, relation),
            Zerometry::MultiPoints(a) => a.relation(other, relation),
            Zerometry::MultiLines(a) => a.relation(other, relation),
            Zerometry::Line(a) => a.relation(other, relation),
            Zerometry::Polygon(a) => a.relation(other, relation),
            Zerometry::MultiPolygon(a) => a.relation(other, relation),
            Zerometry::Collection(a) => a.relation(other, relation),
        }
    }
}

impl<'a> RelationBetweenShapes<Zollection<'a>> for Zerometry<'a> {
    fn relation(&self, other: &Zollection, relation: InputRelation) -> OutputRelation {
        match self {
            Zerometry::Point(a) => a.relation(other, relation),
            Zerometry::MultiPoints(a) => a.relation(other, relation),
            Zerometry::MultiLines(a) => a.relation(other, relation),
            Zerometry::Line(a) => a.relation(other, relation),
            Zerometry::Polygon(a) => a.relation(other, relation),
            Zerometry::MultiPolygon(a) => a.relation(other, relation),
            Zerometry::Collection(a) => a.relation(other, relation),
        }
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for Zerometry<'a> {
    fn relation(&self, other: &Zerometry, relation: InputRelation) -> OutputRelation {
        match other {
            Zerometry::Point(a) => self.relation(a, relation),
            Zerometry::MultiPoints(a) => self.relation(a, relation),
            Zerometry::Line(a) => a.relation(other, relation),
            Zerometry::MultiLines(a) => self.relation(a, relation),
            Zerometry::Polygon(a) => self.relation(a, relation),
            Zerometry::MultiPolygon(a) => self.relation(a, relation),
            Zerometry::Collection(a) => self.relation(a, relation),
        }
    }
}

impl<'a> RelationBetweenShapes<Geometry<f64>> for Zerometry<'a> {
    fn relation(&self, other: &Geometry<f64>, relation: InputRelation) -> OutputRelation {
        let mut buffer = Vec::new();
        Zerometry::write_from_geometry(&mut buffer, other).unwrap();
        let other = Zerometry::from_bytes(&buffer).unwrap();
        self.relation(&other, relation)
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for Geometry<f64> {
    fn relation(&self, other: &Zerometry<'a>, relation: InputRelation) -> OutputRelation {
        let mut buffer = Vec::new();
        Zerometry::write_from_geometry(&mut buffer, self).unwrap();
        let this = Zerometry::from_bytes(&buffer).unwrap();
        this.relation(other, relation)
    }
}

impl<'a> RelationBetweenShapes<Polygon<f64>> for Zerometry<'a> {
    fn relation(&self, other: &Polygon<f64>, relation: InputRelation) -> OutputRelation {
        let mut buffer = Vec::new();
        Zerometry::write_from_geometry(&mut buffer, &Geometry::Polygon(other.clone())).unwrap();
        let other = Zerometry::from_bytes(&buffer).unwrap();
        self.relation(&other, relation)
    }
}

impl<'a> RelationBetweenShapes<MultiPolygon<f64>> for Zerometry<'a> {
    fn relation(&self, other: &MultiPolygon<f64>, relation: InputRelation) -> OutputRelation {
        let mut buffer = Vec::new();
        Zerometry::write_from_geometry(&mut buffer, &Geometry::MultiPolygon(other.clone()))
            .unwrap();
        let other = Zerometry::from_bytes(&buffer).unwrap();
        self.relation(&other, relation)
    }
}

impl PartialEq<Geometry> for Zerometry<'_> {
    fn eq(&self, other: &Geometry) -> bool {
        match (self, other) {
            (Zerometry::Point(zoint), Geometry::Point(point)) => zoint.eq(point),
            (Zerometry::MultiPoints(zulti_points), Geometry::MultiPoint(multi_point)) => {
                zulti_points.eq(multi_point)
            }
            (Zerometry::Line(zine), Geometry::LineString(line_string)) => zine.eq(line_string),
            (Zerometry::MultiLines(zulti_lines), Geometry::MultiLineString(multi_line_string)) => {
                zulti_lines.eq(multi_line_string)
            }
            (Zerometry::Polygon(zolygon), Geometry::Polygon(polygon)) => zolygon.eq(polygon),
            (Zerometry::MultiPolygon(zulti_polygon), Geometry::MultiPolygon(multi_polygon)) => {
                zulti_polygon.eq(multi_polygon)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod zerometry_test {
    use geo_types::geometry;

    use crate::Zerometry;

    #[test]
    fn naive_point_roundtrip() {
        let point = geometry::Geometry::Point(geometry::Point::new(45.0, 65.0));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &point).unwrap();
        let zoint = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zoint, point);
    }

    #[test]
    fn naive_multi_point_roundtrip() {
        let multi_point = geometry::Geometry::MultiPoint(geometry::MultiPoint::new(vec![]));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &multi_point).unwrap();
        let zulti_point = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zulti_point, multi_point);

        let multi_point = geometry::Geometry::MultiPoint(geometry::MultiPoint::new(vec![
            geometry::Point::new(45.0, 65.0),
            geometry::Point::new(46.0, 66.0),
            geometry::Point::new(44.0, 64.0),
            geometry::Point::new(45.0, 65.0),
        ]));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &multi_point).unwrap();
        let zulti_point = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zulti_point, multi_point);
    }

    #[test]
    fn naive_line_string_roundtrip() {
        let line_string = geometry::Geometry::LineString(geometry::LineString::new(vec![]));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &line_string).unwrap();
        let zine_string = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zine_string, line_string);

        let line_string = geometry::Geometry::LineString(geometry::LineString::new(vec![
            geometry::Coord { x: 45.0, y: 25.0 },
            geometry::Coord { x: 46.0, y: 24.0 },
            geometry::Coord { x: 45.0, y: 25.0 },
        ]));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &line_string).unwrap();
        let zine_string = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zine_string, line_string);
    }

    #[test]
    fn naive_multi_line_string_roundtrip() {
        let multi_line_string =
            geometry::Geometry::MultiLineString(geometry::MultiLineString::new(vec![]));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &multi_line_string).unwrap();
        let zulti_line_string = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zulti_line_string, multi_line_string);

        let multi_line_string =
            geometry::Geometry::MultiLineString(geometry::MultiLineString::new(vec![
                geometry::LineString::new(vec![
                    geometry::Coord { x: 45.0, y: 25.0 },
                    geometry::Coord { x: 46.0, y: 24.0 },
                    geometry::Coord { x: 45.0, y: 25.0 },
                ]),
                geometry::LineString::new(vec![]),
                geometry::LineString::new(vec![
                    geometry::Coord { x: 66.0, y: 46.0 },
                    geometry::Coord { x: 47.0, y: 34.0 },
                    geometry::Coord { x: 66.0, y: 26.0 },
                ]),
            ]));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &multi_line_string).unwrap();
        let zulti_line_string = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zulti_line_string, multi_line_string);

        let multi_line_string =
            geometry::Geometry::MultiLineString(geometry::MultiLineString::new(vec![
                geometry::LineString::new(vec![
                    geometry::Coord { x: 45.0, y: 25.0 },
                    geometry::Coord { x: 46.0, y: 24.0 },
                    geometry::Coord { x: 45.0, y: 25.0 },
                ]),
                geometry::LineString::new(vec![
                    geometry::Coord { x: 55.0, y: 25.0 },
                    geometry::Coord { x: 46.0, y: 34.0 },
                    geometry::Coord { x: 55.0, y: 25.0 },
                ]),
                geometry::LineString::new(vec![
                    geometry::Coord { x: 66.0, y: 46.0 },
                    geometry::Coord { x: 47.0, y: 34.0 },
                    geometry::Coord { x: 66.0, y: 26.0 },
                ]),
            ]));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &multi_line_string).unwrap();
        let zulti_line_string = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zulti_line_string, multi_line_string);
    }

    #[test]
    fn naive_polygon_roundtrip() {
        let polygon = geometry::Geometry::Polygon(geometry::Polygon::new(
            geometry::LineString::new(vec![]),
            vec![],
        ));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &polygon).unwrap();
        let zolygon = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zolygon, polygon);

        let polygon = geometry::Geometry::Polygon(geometry::Polygon::new(
            geometry::LineString::new(vec![
                geometry::Coord { x: 66.0, y: 46.0 },
                geometry::Coord { x: 47.0, y: 34.0 },
                geometry::Coord { x: 66.0, y: 26.0 },
            ]),
            vec![],
        ));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &polygon).unwrap();
        let zolygon = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zolygon, polygon);
    }

    #[test]
    fn naive_multi_polygon_roundtrip() {
        let multi_polygon = geometry::Geometry::MultiPolygon(geometry::MultiPolygon::new(vec![]));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &multi_polygon).unwrap();
        let zulti_polygon = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zulti_polygon, multi_polygon);

        let multi_polygon = geometry::Geometry::MultiPolygon(geometry::MultiPolygon::new(vec![
            geometry::Polygon::new(
                geometry::LineString::new(vec![
                    geometry::Coord { x: 66.0, y: 46.0 },
                    geometry::Coord { x: 47.0, y: 34.0 },
                    geometry::Coord { x: 66.0, y: 26.0 },
                ]),
                vec![],
            ),
            geometry::Polygon::new(
                geometry::LineString::new(vec![
                    geometry::Coord { x: 86.0, y: 48.0 },
                    geometry::Coord { x: 67.0, y: 36.0 },
                    geometry::Coord { x: 86.0, y: 28.0 },
                ]),
                vec![],
            ),
        ]));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &multi_polygon).unwrap();
        let zulti_polygon = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zulti_polygon, multi_polygon);
    }

    #[test]
    fn naive_geometry_collection_roundtrip() {
        /*
        let geometry_collection =
            geometry::Geometry::GeometryCollection(geometry::GeometryCollection::new_from(todo!()));
        let mut buf = Vec::new();
        Zerometry::write_from_geometry(&mut buf, &geometry_collection).unwrap();
        let zeometry_collection = Zerometry::from_bytes(&buf).unwrap();
        assert_eq!(zeometry_collection, geometry_collection);
        */
    }
}
