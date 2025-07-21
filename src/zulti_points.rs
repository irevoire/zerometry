use core::fmt;
use std::io::{self, Write};

use geo_types::MultiPoint;

use crate::{
    BoundingBox, Coords, Relation, RelationBetweenShapes, Zoint, Zolygon, COORD_SIZE_IN_BYTES,
};

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

    pub fn from_bytes(data: &'a [u8]) -> Self {
        let bounding_box = BoundingBox::from_bytes(&data[0..COORD_SIZE_IN_BYTES * 2]);
        let coords = Coords::from_bytes(&data[COORD_SIZE_IN_BYTES * 2..]);
        Self::new(bounding_box, coords)
    }

    pub fn write_from_geometry(
        writer: &mut impl Write,
        geometry: &MultiPoint<f64>,
    ) -> Result<(), io::Error> {
        BoundingBox::write_from_geometry(writer, geometry.iter().copied())?;
        for point in geometry.iter() {
            writer.write_all(&point.x().to_ne_bytes())?;
            writer.write_all(&point.y().to_ne_bytes())?;
        }
        Ok(())
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

impl<'a> RelationBetweenShapes<Zolygon<'a>> for ZultiPoints<'a> {
    fn relation(&self, other: &Zolygon<'a>) -> Relation {
        match other.relation(self) {
            Relation::Contains => {
                debug_assert!(true, "A point cannot contain a polygon");
                Relation::Contained
            }
            Relation::Contained => Relation::Contains,
            r => r,
        }
    }
}

impl<'a> PartialEq<MultiPoint<f64>> for ZultiPoints<'a> {
    fn eq(&self, other: &MultiPoint<f64>) -> bool {
        self.coords
            .iter()
            .zip(other.iter())
            .all(|(a, b)| a.lng() == b.x() && a.lat() == b.y())
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;
    use geo_types::Point;
    use insta::assert_compact_debug_snapshot;

    use super::*;

    #[test]
    fn test_zulti_points_binary_format() {
        let mut buffer = Vec::new();
        ZultiPoints::write_from_geometry(
            &mut buffer,
            &MultiPoint::from(vec![Point::new(1.0, 2.0), Point::new(3.0, 4.0)]),
        )
        .unwrap();
        let input: &[f64] = cast_slice(&buffer);
        assert_compact_debug_snapshot!(input, @"[1.0, 2.0, 3.0, 4.0, 1.0, 2.0, 3.0, 4.0]");
        let zulti_points = ZultiPoints::from_bytes(&buffer);
        assert_compact_debug_snapshot!(zulti_points.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 1.0, y: 2.0 }, top_right: Coord { x: 3.0, y: 4.0 } }");
        assert_compact_debug_snapshot!(zulti_points.coords(), @"[Coord { x: 1.0, y: 2.0 }, Coord { x: 3.0, y: 4.0 }]");
    }

    // Prop test ensuring we can round trip from a multi-point to a zulti-points and back to a multi-point
    proptest::proptest! {
        #[test]
        fn test_zulti_points_round_trip(points: Vec<(f64, f64)>) {
            let multi_point = MultiPoint::from(points);
            let mut buffer = Vec::new();
            ZultiPoints::write_from_geometry(&mut buffer, &multi_point).unwrap();
            let zulti_points = ZultiPoints::from_bytes(&buffer);
            assert_eq!(zulti_points, multi_point);
        }
    }
}
