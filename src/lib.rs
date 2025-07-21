mod bounding_box;
mod coord;
mod coords;
mod segment;
mod zoint;
mod zolygon;
mod zulti_points;
mod zulti_polygon;

use std::mem;

pub use bounding_box::BoundingBox;
pub use coord::Coord;
pub(crate) use coord::{COORD_SIZE_IN_BYTES, COORD_SIZE_IN_FLOATS};
pub use coords::Coords;
use geo_types::Geometry;
pub use segment::Segment;
pub use zoint::Zoint;
pub use zolygon::Zolygon;
pub use zulti_points::ZultiPoints;
pub use zulti_polygon::ZultiPolygon;

#[derive(Debug, Clone, Copy)]
pub enum Zerometry<'a> {
    Point(Zoint<'a>),
    MultiPoints(ZultiPoints<'a>),
    Polygon(Zolygon<'a>),
    MultiPolygon(ZultiPolygon<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Relation {
    Contains,
    Contained,
    Intersects,
    Disjoint,
}

pub trait RelationBetweenShapes<Other: ?Sized> {
    fn relation(&self, other: &Other) -> Relation;
}

impl<'a> Zerometry<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, std::io::Error> {
        let tag = u64::from_ne_bytes(data[..mem::size_of::<u64>()].try_into().unwrap());
        let data = &data[mem::size_of::<u64>()..];
        match tag {
            0 => Ok(Zerometry::Point(Zoint::from_bytes(data))),
            1 => Ok(Zerometry::MultiPoints(ZultiPoints::from_bytes(data))),
            2 => Ok(Zerometry::Polygon(Zolygon::from_bytes(data))),
            3 => Ok(Zerometry::MultiPolygon(ZultiPolygon::from_bytes(data))),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid zerometry tag",
            )),
        }
    }

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
                ZultiPolygon::write_from_geometry(writer, multi_polygon)?;
            }
            _ => todo!(),
        }
        Ok(())
    }
}

impl<'a> RelationBetweenShapes<Zoint<'a>> for Zerometry<'a> {
    fn relation(&self, other: &Zoint) -> Relation {
        match self {
            Zerometry::Point(a) => a.relation(other),
            Zerometry::MultiPoints(a) => a.relation(other),
            Zerometry::Polygon(a) => a.relation(other),
            Zerometry::MultiPolygon(a) => a.relation(other),
        }
    }
}

impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for Zerometry<'a> {
    fn relation(&self, other: &ZultiPoints) -> Relation {
        match self {
            Zerometry::Point(a) => a.relation(other),
            Zerometry::MultiPoints(a) => a.relation(other),
            Zerometry::Polygon(a) => a.relation(other),
            Zerometry::MultiPolygon(a) => a.relation(other),
        }
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for Zerometry<'a> {
    fn relation(&self, other: &Zolygon) -> Relation {
        match self {
            Zerometry::Point(a) => a.relation(other),
            Zerometry::MultiPoints(a) => a.relation(other),
            Zerometry::Polygon(a) => a.relation(other),
            Zerometry::MultiPolygon(a) => a.relation(other),
        }
    }
}

impl<'a> RelationBetweenShapes<ZultiPolygon<'a>> for Zerometry<'a> {
    fn relation(&self, other: &ZultiPolygon) -> Relation {
        match self {
            Zerometry::Point(a) => a.relation(other),
            Zerometry::MultiPoints(a) => a.relation(other),
            Zerometry::Polygon(a) => a.relation(other),
            Zerometry::MultiPolygon(a) => a.relation(other),
        }
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for Zerometry<'a> {
    fn relation(&self, other: &Zerometry) -> Relation {
        match other {
            Zerometry::Point(a) => self.relation(a),
            Zerometry::MultiPoints(a) => self.relation(a),
            Zerometry::Polygon(a) => self.relation(a),
            Zerometry::MultiPolygon(a) => self.relation(a),
        }
    }
}
