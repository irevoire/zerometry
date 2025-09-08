use core::fmt;
use std::io::{self, Write};

use geo_types::Point;

use crate::{
    Coord, InputRelation, OutputRelation, RelationBetweenShapes, Zerometry, Zollection, Zolygon,
    ZultiPoints, ZultiPolygons, zine::Zine, zulti_lines::ZultiLines,
};

/// Equivalent of a [`geo_types::Point`].
#[derive(Clone, Copy)]
pub struct Zoint<'a> {
    coord: &'a Coord,
}

impl<'a> Zoint<'a> {
    pub fn new(coord: &'a Coord) -> Self {
        Self { coord }
    }

    /// # Safety
    /// The data must be generated from the [`Self::write_from_geometry`] method and be aligned on 64 bits
    #[inline]
    pub unsafe fn from_bytes(data: &'a [u8]) -> Self {
        let coord = unsafe { Coord::from_bytes(data) };
        Self::new(coord)
    }

    pub fn write_from_geometry(
        writer: &mut impl Write,
        geometry: &Point<f64>,
    ) -> Result<(), io::Error> {
        writer.write_all(&geometry.x().to_ne_bytes())?;
        writer.write_all(&geometry.y().to_ne_bytes())?;
        Ok(())
    }

    #[inline]
    pub fn coord(&self) -> &'a Coord {
        self.coord
    }

    #[inline]
    pub fn lat(&self) -> f64 {
        self.coord.lat()
    }
    #[inline]
    pub fn lng(&self) -> f64 {
        self.coord.lng()
    }

    #[inline]
    pub fn x(&self) -> f64 {
        self.coord.lng()
    }
    #[inline]
    pub fn y(&self) -> f64 {
        self.coord.lat()
    }

    #[inline]
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

impl PartialEq<geo_types::Point<f64>> for Zoint<'_> {
    fn eq(&self, other: &geo_types::Point<f64>) -> bool {
        self.coord.lng() == other.x() && self.coord.lat() == other.y()
    }
}

// A point cannot contains or intersect with another point
impl<'a> RelationBetweenShapes<Zoint<'a>> for Zoint<'a> {
    fn relation(&self, _other: &Zoint<'a>, relation: InputRelation) -> OutputRelation {
        relation.to_false().make_disjoint_if_set()
    }
}

// A point cannot contains or intersect with a multi point
impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for Zoint<'a> {
    fn relation(&self, _other: &ZultiPoints<'a>, relation: InputRelation) -> OutputRelation {
        relation.to_false().make_disjoint_if_set()
    }
}

// A point cannot contains or intersect with a line
impl<'a> RelationBetweenShapes<Zine<'a>> for Zoint<'a> {
    fn relation(&self, _other: &Zine<'a>, relation: InputRelation) -> OutputRelation {
        relation.to_false().make_disjoint_if_set()
    }
}

// A point cannot contains or intersect with a line
impl<'a> RelationBetweenShapes<ZultiLines<'a>> for Zoint<'a> {
    fn relation(&self, _other: &ZultiLines<'a>, relation: InputRelation) -> OutputRelation {
        relation.to_false().make_disjoint_if_set()
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for Zoint<'a> {
    fn relation(&self, other: &Zolygon<'a>, relation: InputRelation) -> OutputRelation {
        if other.strict_contains(self) {
            relation.to_false().make_strict_contained_if_set()
        } else {
            relation.to_false().make_disjoint_if_set()
        }
    }
}

impl<'a> RelationBetweenShapes<ZultiPolygons<'a>> for Zoint<'a> {
    fn relation(&self, other: &ZultiPolygons<'a>, relation: InputRelation) -> OutputRelation {
        other
            .relation(self, relation.swap_contains_relation())
            .swap_contains_relation()
    }
}

impl<'a> RelationBetweenShapes<Zollection<'a>> for Zoint<'a> {
    fn relation(&self, other: &Zollection<'a>, relation: InputRelation) -> OutputRelation {
        other
            .relation(self, relation.swap_contains_relation())
            .swap_contains_relation()
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for Zoint<'a> {
    fn relation(&self, other: &Zerometry<'a>, relation: InputRelation) -> OutputRelation {
        other
            .relation(self, relation.swap_contains_relation())
            .swap_contains_relation()
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;
    use insta::assert_compact_debug_snapshot;

    use super::*;

    #[test]
    fn test_zoint_binary_format() {
        let mut buffer = Vec::new();
        Zoint::write_from_geometry(&mut buffer, &Point::new(1.0, 2.0)).unwrap();
        let input: &[f64] = cast_slice(&buffer);
        assert_compact_debug_snapshot!(input, @"[1.0, 2.0]");
        let zoint = unsafe { Zoint::from_bytes(&buffer) };
        assert_compact_debug_snapshot!(zoint.coord(), @"Coord { x: 1.0, y: 2.0 }");
    }

    // Prop test ensuring we can round trip from a point to a zoint and back to a point
    proptest::proptest! {
        #[test]
        fn test_zoint_round_trip(lng: f64, lat: f64) {
            let point = Point::new(lng, lat);
            let mut buffer = Vec::new();
            Zoint::write_from_geometry(&mut buffer, &point).unwrap();
            let zoint = unsafe { Zoint::from_bytes(&buffer) };
            assert_eq!(zoint, point);
        }
    }
}
