mod bounding_box;
mod coord;
mod coords;
mod segment;
mod zoint;
mod zolygon;
mod zulti_points;
mod zulti_polygon;

pub use bounding_box::BoundingBox;
pub use coord::Coord;
pub(crate) use coord::{COORD_SIZE_IN_BYTES, COORD_SIZE_IN_FLOATS};
pub use coords::Coords;
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
