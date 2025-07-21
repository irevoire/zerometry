use core::fmt;
use std::io::{self, Write};

use geo_types::Point;

use crate::{Coord, Relation, RelationBetweenShapes, Zolygon, ZultiPoints};

#[derive(Clone, Copy)]
pub struct Zoint<'a> {
    coord: &'a Coord,
}

impl<'a> Zoint<'a> {
    pub fn new(coord: &'a Coord) -> Self {
        Self { coord }
    }

    pub fn from_bytes(data: &'a [u8]) -> Self {
        let coord = Coord::from_bytes(&data);
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

impl PartialEq<geo_types::Point<f64>> for Zoint<'_> {
    fn eq(&self, other: &geo_types::Point<f64>) -> bool {
        self.coord.lng() == other.x() && self.coord.lat() == other.y()
    }
}

// A point cannot contains or intersect with another point
impl<'a> RelationBetweenShapes<Zoint<'a>> for Zoint<'a> {
    fn relation(&self, _other: &Zoint<'a>) -> Relation {
        Relation::Disjoint
    }
}

// A point cannot contains or intersect with a multi point
impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for Zoint<'a> {
    fn relation(&self, _other: &ZultiPoints<'a>) -> Relation {
        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for Zoint<'a> {
    fn relation(&self, other: &Zolygon<'a>) -> Relation {
        if other.relation(self) == Relation::Contains {
            Relation::Contained
        } else {
            Relation::Disjoint
        }
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
        let zoint = Zoint::from_bytes(&buffer);
        assert_compact_debug_snapshot!(zoint.coord(), @"Coord { x: 1.0, y: 2.0 }");
    }

    // Prop test ensuring we can round trip from a point to a zoint and back to a point
    proptest::proptest! {
        #[test]
        fn test_zoint_round_trip(lng: f64, lat: f64) {
            let point = Point::new(lng, lat);
            let mut buffer = Vec::new();
            Zoint::write_from_geometry(&mut buffer, &point).unwrap();
            let zoint = Zoint::from_bytes(&buffer);
            assert_eq!(zoint, point);
        }
    }
}
