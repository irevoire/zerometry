use core::fmt;

use crate::{Coord, Relation, RelationBetweenShapes, ZultiPoints};


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
            .field("lng", &self.coord.lng())
            .field("lat", &self.coord.lat())
            .finish()
    }
}


// A point cannot contains or intersect with anything
impl<'a> RelationBetweenShapes<Zoint<'a>> for Zoint<'a> {
    fn relation(&self, _other: &Zoint<'a>) -> Relation {
        Relation::Disjoint
    }
}

// A point cannot contains or intersect with anything
impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for Zoint<'a> {
    fn relation(&self, _other: &ZultiPoints<'a>) -> Relation {
        Relation::Disjoint
    }
}
