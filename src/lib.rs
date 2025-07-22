mod bounding_box;
mod coord;
mod coords;
mod segment;
#[cfg(test)]
mod test;
mod zoint;
mod zolygon;
mod zulti_points;
mod zulti_polygon;

use std::mem;

pub use bounding_box::BoundingBox;
pub use coord::Coord;
pub(crate) use coord::{COORD_SIZE_IN_BYTES, COORD_SIZE_IN_FLOATS};
pub use coords::Coords;
use geo_types::{Geometry, MultiPolygon, Polygon};
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

    fn contains(&self, other: &Other) -> bool {
        self.relation(other) == Relation::Contains
    }

    fn contained(&self, other: &Other) -> bool {
        self.relation(other) == Relation::Contained
    }

    fn intersects(&self, other: &Other) -> bool {
        self.relation(other) == Relation::Intersects
    }

    fn disjoint(&self, other: &Other) -> bool {
        self.relation(other) == Relation::Disjoint
    }
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

    pub fn to_geo(&self) -> geo_types::Geometry<f64> {
        match self {
            Zerometry::Point(a) => Geometry::Point(a.to_geo()),
            Zerometry::MultiPoints(a) => Geometry::MultiPoint(a.to_geo()),
            Zerometry::Polygon(a) => Geometry::Polygon(a.to_geo()),
            Zerometry::MultiPolygon(a) => Geometry::MultiPolygon(a.to_geo()),
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

impl<'a> From<ZultiPolygon<'a>> for Zerometry<'a> {
    fn from(polygon: ZultiPolygon<'a>) -> Self {
        Zerometry::MultiPolygon(polygon)
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

impl<'a> RelationBetweenShapes<Geometry<f64>> for Zerometry<'a> {
    fn relation(&self, other: &Geometry<f64>) -> Relation {
        let mut buffer = Vec::new();
        Zerometry::write_from_geometry(&mut buffer, other).unwrap();
        let other = Zerometry::from_bytes(&buffer).unwrap();
        self.relation(&other)
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for Geometry<f64> {
    fn relation(&self, other: &Zerometry<'a>) -> Relation {
        let mut buffer = Vec::new();
        Zerometry::write_from_geometry(&mut buffer, self).unwrap();
        let this = Zerometry::from_bytes(&buffer).unwrap();
        this.relation(other)
    }
}

impl<'a> RelationBetweenShapes<Polygon<f64>> for Zerometry<'a> {
    fn relation(&self, other: &Polygon<f64>) -> Relation {
        let mut buffer = Vec::new();
        Zerometry::write_from_geometry(&mut buffer, &Geometry::Polygon(other.clone())).unwrap();
        let other = Zerometry::from_bytes(&buffer).unwrap();
        self.relation(&other)
    }
}

impl<'a> RelationBetweenShapes<MultiPolygon<f64>> for Zerometry<'a> {
    fn relation(&self, other: &MultiPolygon<f64>) -> Relation {
        let mut buffer = Vec::new();
        Zerometry::write_from_geometry(&mut buffer, &Geometry::MultiPolygon(other.clone()))
            .unwrap();
        let other = Zerometry::from_bytes(&buffer).unwrap();
        self.relation(&other)
    }
}
