mod bounding_box;
mod coord;
mod coords;
mod zulti_points;
mod zoint;
mod segment;
mod zolygon;

pub use bounding_box::BoundingBox;
pub use zoint::Zoint;
pub use zulti_points::ZultiPoints;
pub use segment::Segment;
pub use coord::Coord;
pub(crate) use coord::{COORD_SIZE_IN_BYTES, COORD_SIZE_IN_FLOATS};
pub use coords::Coords;
pub use zolygon::Zolygon;

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
