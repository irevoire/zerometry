use core::fmt;

use crate::{BoundingBox, Coords, Relation, RelationBetweenShapes, Zoint};


#[derive(Clone, Copy)]
pub struct ZultiPoints<'a> {
    bounding_box: &'a BoundingBox,
    coords: &'a Coords,
}

impl<'a> ZultiPoints<'a> {
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

impl<'a> fmt::Debug for ZultiPoints<'a> {
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

// A point cannot contains or intersect with anything
impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for ZultiPoints<'a> {
    fn relation(&self, _other: &ZultiPoints<'a>) -> Relation {
        Relation::Disjoint
    }
}

// A point cannot contains or intersect with anything
impl<'a> RelationBetweenShapes<Zoint<'a>> for ZultiPoints<'a> {
    fn relation(&self, _other: &Zoint<'a>) -> Relation {
        Relation::Disjoint
    }
}