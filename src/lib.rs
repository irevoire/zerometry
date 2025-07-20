mod bounding_box;
mod coord;
mod coords;
use core::fmt;

pub use bounding_box::BoundingBox;
pub use coord::Coord;
pub(crate) use coord::{COORD_SIZE_IN_BYTES, COORD_SIZE_IN_FLOATS};
pub use coords::Coords;

#[derive(Debug, Clone, Copy)]
pub enum Zerometry<'a> {
    Point(Zoint<'a>),
    MultiPoint(ZultiPoint<'a>),
}

#[derive(Clone, Copy)]
pub struct Zoint<'a> {
    coord: &'a Coord,
}

impl<'a> Zoint<'a> {
    pub fn new(coord: &'a Coord) -> Self {
        Self { coord }
    }

    pub fn coord(&self) -> &'a Coord {
        self.coord
    }

    pub fn to_geo(&self) -> geo_types::Point<f64> {
        geo_types::Point::new(self.coord.lng(), self.coord.lat())
    }
}

impl<'a> fmt::Debug for Zoint<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Zoint")
            .field("x", &self.coord.lng())
            .field("y", &self.coord.lat())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct ZultiPoint<'a> {
    bounding_box: &'a BoundingBox,
    coords: &'a Coords,
}

impl<'a> ZultiPoint<'a> {
    pub fn new(bounding_box: &'a BoundingBox, coords: &'a Coords) -> Self {
        Self {
            bounding_box,
            coords,
        }
    }

    pub fn bounding_box(&self) -> &'a BoundingBox {
        self.bounding_box
    }

    pub fn coords(&self) -> &'a Coords {
        self.coords
    }
}

impl<'a> fmt::Debug for ZultiPoint<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZultiPoint")
            .field("bounding_box", &self.bounding_box)
            .field(
                "points",
                &self
                    .coords
                    .iter()
                    .map(|c| Zoint::new(c))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}
