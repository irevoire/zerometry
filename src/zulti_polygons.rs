use std::{fmt, io, mem};

use bytemuck::cast_slice;
use geo_types::{MultiPolygon, Point};

use crate::{
    BoundingBox, InputRelation, OutputRelation, RelationBetweenShapes, Zerometry, Zoint,
    Zollection, Zolygon, ZultiLines, ZultiPoints, bounding_box::BOUNDING_BOX_SIZE_IN_BYTES,
    zine::Zine,
};

#[derive(Clone, Copy)]
pub struct ZultiPolygons<'a> {
    bounding_box: &'a BoundingBox,
    // In the binary format we store the number of offsets here
    // If it's 0, it means that the polygon is empty
    // If it's odd it means we also inserted one extra offset at the end for padding that should not ends up in the slice
    offsets: &'a [u32],
    bytes: &'a [u8],
}

impl<'a> ZultiPolygons<'a> {
    pub fn new(bounding_box: &'a BoundingBox, offsets: &'a [u32], bytes: &'a [u8]) -> Self {
        Self {
            bounding_box,
            offsets,
            bytes,
        }
    }

    pub fn from_bytes(data: &'a [u8]) -> Self {
        // 1. Retrieve the bounding box
        let bounding_box = BoundingBox::from_bytes(&data[..BOUNDING_BOX_SIZE_IN_BYTES]);
        let data = &data[BOUNDING_BOX_SIZE_IN_BYTES..];

        // 2. Then retrieve the offsets
        // 2.1 Start by getting the number of offsets to retrieve
        let offsets_count = u32::from_ne_bytes(data[..mem::size_of::<u32>()].try_into().unwrap());
        let data = &data[mem::size_of::<u32>()..];
        // 2.2 Then retrieve the offsets
        let size_of_offsets = offsets_count as usize * mem::size_of::<u32>();
        let offsets = &data[..size_of_offsets];
        let offsets: &[u32] = cast_slice(offsets);
        let data = &data[size_of_offsets..];
        // 2.3 If we have an even number of offsets, there is one u32 of padding at the end that we must skip before retrieving coords of the polygons
        let data = if offsets_count % 2 == 0 {
            debug_assert_eq!(data[0..mem::size_of::<u32>()], [0, 0, 0, 0]);
            &data[mem::size_of::<u32>()..]
        } else {
            data
        };
        // 3. Finally retrieve the polygons
        let bytes = data;

        Self {
            bounding_box,
            offsets,
            bytes,
        }
    }

    pub fn write_from_geometry(
        writer: &mut Vec<u8>,
        geometry: &MultiPolygon<f64>,
    ) -> Result<(), io::Error> {
        BoundingBox::write_from_geometry(
            writer,
            geometry
                .iter()
                .flat_map(|polygon| polygon.exterior().0.iter())
                .map(|coord| Point::from((coord.x, coord.y))),
        )?;
        // Write the number of offsets to expect
        writer.extend((geometry.0.len() as u32).to_ne_bytes());
        let offsets_addr = writer.len();
        // We must leave an empty space to write the offsets later
        writer.extend(std::iter::repeat_n(
            0,
            geometry.0.len() * mem::size_of::<u32>(),
        ));
        if geometry.0.len() % 2 == 0 {
            // If we have an even number of polygons, we must add an extra offset at the end for padding
            writer.extend(0_u32.to_ne_bytes());
        }
        let start = writer.len();
        let mut offsets = Vec::new();
        for polygon in geometry.iter() {
            offsets.push(writer.len() as u32 - start as u32);
            Zolygon::write_from_geometry(writer, polygon)?;
        }

        for (i, offset) in offsets.iter().enumerate() {
            let offset_addr = offsets_addr + i * mem::size_of::<u32>();
            writer[offset_addr..offset_addr + mem::size_of::<u32>()]
                .copy_from_slice(&offset.to_ne_bytes());
        }
        Ok(())
    }

    pub fn bounding_box(&self) -> &'a BoundingBox {
        self.bounding_box
    }

    pub fn get(&self, index: usize) -> Option<Zolygon<'a>> {
        let offset = *self.offsets.get(index)?;
        let next_offset = *self
            .offsets
            .get(index + 1)
            .unwrap_or(&(self.bytes.len() as u32));
        let bytes = &self.bytes[offset as usize..next_offset as usize];
        Some(Zolygon::from_bytes(bytes))
    }

    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn polygons(&'a self) -> impl Iterator<Item = Zolygon<'a>> {
        (0..self.len()).map(move |index| self.get(index).unwrap())
    }

    pub fn to_geo(&self) -> geo_types::MultiPolygon<f64> {
        geo_types::MultiPolygon::new(self.polygons().map(|zolygon| zolygon.to_geo()).collect())
    }
}

impl<'a> fmt::Debug for ZultiPolygons<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct ZolygonsDebug<'b, 'a>(&'b ZultiPolygons<'a>);

        impl<'b, 'a> fmt::Debug for ZolygonsDebug<'b, 'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.0.polygons()).finish()
            }
        }

        f.debug_struct("ZultiPolygons")
            .field("bounding_box", &self.bounding_box())
            .field("zolygons", &ZolygonsDebug(self))
            .finish()
    }
}

impl<'a> RelationBetweenShapes<Zoint<'a>> for ZultiPolygons<'a> {
    fn relation(&self, other: &Zoint, relation: InputRelation) -> OutputRelation {
        if self.is_empty() || !self.bounding_box().contains_coord(other.coord()) {
            return relation.to_false().make_disjoint_if_set();
        }
        for zolygon in self.polygons() {
            if zolygon.strict_contains(other) {
                return relation.to_false().make_strict_contains_if_set();
            }
        }
        relation.to_false().make_disjoint_if_set()
    }
}

impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for ZultiPolygons<'a> {
    fn relation(&self, other: &ZultiPoints, relation: InputRelation) -> OutputRelation {
        let mut output = relation.to_false();

        if self.is_empty() || other.is_empty() || self.bounding_box().disjoint(other.bounding_box())
        {
            return relation.to_false().make_disjoint_if_set();
        }

        let mut contains = 0;
        for zolygon in self.polygons() {
            for point in other.coords().iter() {
                if zolygon.contains(point) {
                    output = output.make_contains_if_set();
                    contains += 1;
                    if !relation.strict_contains || relation.early_exit {
                        return output;
                    }
                }
            }
        }

        if contains == other.len() {
            output.make_strict_contains_if_set()
        } else if output.any_relation() {
            output
        } else {
            output.make_disjoint_if_set()
        }
    }
}

impl<'a> RelationBetweenShapes<Zine<'a>> for ZultiPolygons<'a> {
    fn relation(&self, other: &Zine, relation: InputRelation) -> OutputRelation {
        other
            .relation(self, relation.swap_contains_relation())
            .swap_contains_relation()
    }
}

impl<'a> RelationBetweenShapes<ZultiLines<'a>> for ZultiPolygons<'a> {
    fn relation(&self, other: &ZultiLines, relation: InputRelation) -> OutputRelation {
        other
            .relation(self, relation.swap_contains_relation())
            .swap_contains_relation()
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for ZultiPolygons<'a> {
    fn relation(&self, other: &Zolygon, relation: InputRelation) -> OutputRelation {
        if self.is_empty() || other.is_empty() || self.bounding_box().disjoint(other.bounding_box())
        {
            return relation.to_false().make_disjoint_if_set();
        }
        let mut output = relation.to_false();

        let mut contained = 0;

        for zolygon in self.polygons() {
            let r = zolygon.relation(other, relation.strip_disjoint().strip_strict_contained());
            output |= r;

            if r.contained.unwrap_or_default() {
                contained += 1;
            }

            if output.any_relation() && relation.early_exit {
                return output;
            }
        }

        if self.len() == contained {
            output = output.make_strict_contained_if_set();
        }

        if output.any_relation() {
            output
        } else {
            output.make_disjoint_if_set()
        }
    }
}

impl<'a> RelationBetweenShapes<ZultiPolygons<'a>> for ZultiPolygons<'a> {
    fn relation(&self, other: &ZultiPolygons, relation: InputRelation) -> OutputRelation {
        if self.is_empty() || other.is_empty() || self.bounding_box().disjoint(other.bounding_box())
        {
            return relation.to_false().make_disjoint_if_set();
        }
        let mut output = relation.to_false();
        let mut contains = 0;
        let mut contained = 0;

        for left in self.polygons() {
            for right in other.polygons() {
                let r = left.relation(&right, relation.strip_strict().strip_disjoint());
                output |= r;
                contains += r.contains.unwrap_or_default() as usize;
                contained += r.contained.unwrap_or_default() as usize;

                if output.any_relation() && relation.early_exit {
                    return output;
                }
            }
        }

        if contains == other.len() {
            output = output.make_strict_contains_if_set();
        }
        if contained == self.len() {
            output = output.make_strict_contained_if_set();
        }

        if output.any_relation() {
            output
        } else {
            output.make_disjoint_if_set()
        }
    }
}

impl<'a> RelationBetweenShapes<Zollection<'a>> for ZultiPolygons<'a> {
    fn relation(&self, other: &Zollection<'a>, relation: InputRelation) -> OutputRelation {
        other
            .relation(self, relation.swap_contains_relation())
            .swap_contains_relation()
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for ZultiPolygons<'a> {
    fn relation(&self, other: &Zerometry, relation: InputRelation) -> OutputRelation {
        other
            .relation(self, relation.swap_contains_relation())
            .swap_contains_relation()
    }
}

impl PartialEq<MultiPolygon> for ZultiPolygons<'_> {
    fn eq(&self, other: &MultiPolygon) -> bool {
        self.polygons()
            .zip(other.0.iter())
            .all(|(zolygon, polygon)| zolygon.eq(polygon))
    }
}

#[cfg(test)]
mod tests {
    use geo::polygon;
    use geo_types::{LineString, Polygon};
    use insta::{assert_compact_debug_snapshot, assert_debug_snapshot, assert_snapshot};

    use super::*;

    #[test]
    fn test_write_from_geometry_with_even_number_of_elements() {
        let first_polygon = Polygon::new(
            LineString::from(vec![
                Point::from((0.0, 0.0)),
                Point::from((10.0, 0.0)),
                Point::from((0.0, 10.0)),
            ]),
            vec![],
        );
        let second_polygon = Polygon::new(
            LineString::from(vec![
                Point::from((10.0, 10.0)),
                Point::from((20.0, 0.0)),
                Point::from((20.0, 10.0)),
            ]),
            vec![],
        );
        let geometry = MultiPolygon::from(vec![first_polygon.clone(), second_polygon.clone()]);

        let mut writer = Vec::new();

        ZultiPolygons::write_from_geometry(&mut writer, &geometry).unwrap();
        // Debug everything at once just to make sure it never changes
        assert_debug_snapshot!(writer);
        let mut current_offset = 0;
        let expected_bounding_box: &[f64] =
            cast_slice(&writer[current_offset..BOUNDING_BOX_SIZE_IN_BYTES]);
        assert_compact_debug_snapshot!(expected_bounding_box, @"[0.0, 0.0, 20.0, 10.0]");
        current_offset += BOUNDING_BOX_SIZE_IN_BYTES;
        let expected_nb_offsets: u32 = u32::from_ne_bytes(
            writer[current_offset..current_offset + mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        assert_snapshot!(expected_nb_offsets, @"2");
        current_offset += mem::size_of::<u32>();
        // With 2 elements + the u32 to give us the number of elements we're one u32 off at the end. There should be padding
        let expected_offsets: &[u32] = cast_slice(
            &writer[current_offset
                ..current_offset + mem::size_of::<u32>() * expected_nb_offsets as usize],
        );
        assert_compact_debug_snapshot!(expected_offsets, @"[0, 96]");
        current_offset += mem::size_of::<u32>() * expected_nb_offsets as usize;
        // Now there should be a one u32 of padding
        let padding = &writer[current_offset..current_offset + mem::size_of::<u32>()];
        assert_compact_debug_snapshot!(padding, @"[0, 0, 0, 0]");
        current_offset += mem::size_of::<u32>();
        // Now there should be the first zolygon at the offset 0
        let first_zolygon_bytes = &writer[current_offset + expected_offsets[0] as usize
            ..current_offset + expected_offsets[1] as usize];
        assert_compact_debug_snapshot!(first_zolygon_bytes, @"[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]");
        let first_zolygon = Zolygon::from_bytes(first_zolygon_bytes);
        assert_compact_debug_snapshot!(first_zolygon, @"Zolygon { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }, coords: [Coord { x: 0.0, y: 0.0 }, Coord { x: 10.0, y: 0.0 }, Coord { x: 0.0, y: 10.0 }, Coord { x: 0.0, y: 0.0 }] }");
        assert_eq!(first_zolygon, first_polygon);
        let second_zolygon_bytes = &writer[current_offset + expected_offsets[1] as usize..];
        assert_compact_debug_snapshot!(second_zolygon_bytes, @"[0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 52, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 52, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 52, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64]");
        let second_zolygon = Zolygon::from_bytes(second_zolygon_bytes);
        assert_compact_debug_snapshot!(second_zolygon, @"Zolygon { bounding_box: BoundingBox { bottom_left: Coord { x: 10.0, y: 0.0 }, top_right: Coord { x: 20.0, y: 10.0 } }, coords: [Coord { x: 10.0, y: 10.0 }, Coord { x: 20.0, y: 0.0 }, Coord { x: 20.0, y: 10.0 }, Coord { x: 10.0, y: 10.0 }] }");
        assert_eq!(second_zolygon, second_polygon);

        // Try to parse the zulti polygon
        let zulti_polygon = ZultiPolygons::from_bytes(&writer);
        assert_snapshot!(zulti_polygon.len(), @"2");
        assert_compact_debug_snapshot!(zulti_polygon.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 20.0, y: 10.0 } }");
        assert_compact_debug_snapshot!(zulti_polygon.offsets, @"[0, 96]");
        assert_compact_debug_snapshot!(zulti_polygon.get(0).unwrap(), @"Zolygon { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }, coords: [Coord { x: 0.0, y: 0.0 }, Coord { x: 10.0, y: 0.0 }, Coord { x: 0.0, y: 10.0 }, Coord { x: 0.0, y: 0.0 }] }");
        assert_compact_debug_snapshot!(zulti_polygon.get(1).unwrap(), @"Zolygon { bounding_box: BoundingBox { bottom_left: Coord { x: 10.0, y: 0.0 }, top_right: Coord { x: 20.0, y: 10.0 } }, coords: [Coord { x: 10.0, y: 10.0 }, Coord { x: 20.0, y: 0.0 }, Coord { x: 20.0, y: 10.0 }, Coord { x: 10.0, y: 10.0 }] }");
        assert_compact_debug_snapshot!(zulti_polygon.get(2), @"None");
        assert_debug_snapshot!(zulti_polygon, @r"
        ZultiPolygons {
            bounding_box: BoundingBox {
                bottom_left: Coord {
                    x: 0.0,
                    y: 0.0,
                },
                top_right: Coord {
                    x: 20.0,
                    y: 10.0,
                },
            },
            zolygons: [
                Zolygon {
                    bounding_box: BoundingBox {
                        bottom_left: Coord {
                            x: 0.0,
                            y: 0.0,
                        },
                        top_right: Coord {
                            x: 10.0,
                            y: 10.0,
                        },
                    },
                    coords: [
                        Coord {
                            x: 0.0,
                            y: 0.0,
                        },
                        Coord {
                            x: 10.0,
                            y: 0.0,
                        },
                        Coord {
                            x: 0.0,
                            y: 10.0,
                        },
                        Coord {
                            x: 0.0,
                            y: 0.0,
                        },
                    ],
                },
                Zolygon {
                    bounding_box: BoundingBox {
                        bottom_left: Coord {
                            x: 10.0,
                            y: 0.0,
                        },
                        top_right: Coord {
                            x: 20.0,
                            y: 10.0,
                        },
                    },
                    coords: [
                        Coord {
                            x: 10.0,
                            y: 10.0,
                        },
                        Coord {
                            x: 20.0,
                            y: 0.0,
                        },
                        Coord {
                            x: 20.0,
                            y: 10.0,
                        },
                        Coord {
                            x: 10.0,
                            y: 10.0,
                        },
                    ],
                },
            ],
        }
        ");
    }

    #[test]
    fn test_write_from_geometry_with_odd_number_of_elements() {
        let first_polygon = Polygon::new(
            LineString::from(vec![
                Point::from((0.0, 0.0)),
                Point::from((10.0, 0.0)),
                Point::from((0.0, 10.0)),
            ]),
            vec![],
        );
        let geometry = MultiPolygon::from(vec![first_polygon.clone()]);

        let mut writer = Vec::new();

        ZultiPolygons::write_from_geometry(&mut writer, &geometry).unwrap();
        // Debug everything at once just to make sure it never changes
        assert_debug_snapshot!(writer);
        let mut current_offset = 0;
        let expected_bounding_box: &[f64] =
            cast_slice(&writer[current_offset..BOUNDING_BOX_SIZE_IN_BYTES]);
        assert_compact_debug_snapshot!(expected_bounding_box, @"[0.0, 0.0, 10.0, 10.0]");
        current_offset += BOUNDING_BOX_SIZE_IN_BYTES;
        let expected_nb_offsets: u32 = u32::from_ne_bytes(
            writer[current_offset..current_offset + mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        assert_snapshot!(expected_nb_offsets, @"1");
        current_offset += mem::size_of::<u32>();
        // With 2 elements + the u32 to give us the number of elements we're one u32 off at the end. There should be padding
        let expected_offsets: &[u32] = cast_slice(
            &writer[current_offset
                ..current_offset + mem::size_of::<u32>() * expected_nb_offsets as usize],
        );
        assert_compact_debug_snapshot!(expected_offsets, @"[0]");
        current_offset += mem::size_of::<u32>() * expected_nb_offsets as usize;
        // This time we should not have any padding
        // -
        // Now there should be the first zolygon at the offset 0
        let first_zolygon_bytes = &writer[current_offset + expected_offsets[0] as usize..];
        assert_compact_debug_snapshot!(first_zolygon_bytes, @"[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]");
        let first_zolygon = Zolygon::from_bytes(first_zolygon_bytes);
        assert_compact_debug_snapshot!(first_zolygon, @"Zolygon { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }, coords: [Coord { x: 0.0, y: 0.0 }, Coord { x: 10.0, y: 0.0 }, Coord { x: 0.0, y: 10.0 }, Coord { x: 0.0, y: 0.0 }] }");
        assert_eq!(first_zolygon, first_polygon);

        // Try to parse the zulti polygon
        let zulti_polygon = ZultiPolygons::from_bytes(&writer);
        assert_snapshot!(zulti_polygon.len(), @"1");
        assert_compact_debug_snapshot!(zulti_polygon.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }");
        assert_compact_debug_snapshot!(zulti_polygon.offsets, @"[0]");
        assert_compact_debug_snapshot!(zulti_polygon.get(0).unwrap(), @"Zolygon { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }, coords: [Coord { x: 0.0, y: 0.0 }, Coord { x: 10.0, y: 0.0 }, Coord { x: 0.0, y: 10.0 }, Coord { x: 0.0, y: 0.0 }] }");
        assert_compact_debug_snapshot!(zulti_polygon.get(1), @"None");
        assert_debug_snapshot!(zulti_polygon, @r"
        ZultiPolygons {
            bounding_box: BoundingBox {
                bottom_left: Coord {
                    x: 0.0,
                    y: 0.0,
                },
                top_right: Coord {
                    x: 10.0,
                    y: 10.0,
                },
            },
            zolygons: [
                Zolygon {
                    bounding_box: BoundingBox {
                        bottom_left: Coord {
                            x: 0.0,
                            y: 0.0,
                        },
                        top_right: Coord {
                            x: 10.0,
                            y: 10.0,
                        },
                    },
                    coords: [
                        Coord {
                            x: 0.0,
                            y: 0.0,
                        },
                        Coord {
                            x: 10.0,
                            y: 0.0,
                        },
                        Coord {
                            x: 0.0,
                            y: 10.0,
                        },
                        Coord {
                            x: 0.0,
                            y: 0.0,
                        },
                    ],
                },
            ],
        }
        ");
    }

    #[test]
    fn test_write_from_geometry_with_no_elements() {
        let geometry = MultiPolygon::new(vec![]);

        let mut writer = Vec::new();

        ZultiPolygons::write_from_geometry(&mut writer, &geometry).unwrap();
        // Debug everything at once just to make sure it never changes
        assert_debug_snapshot!(writer);
        let mut current_offset = 0;
        let expected_bounding_box: &[f64] =
            cast_slice(&writer[current_offset..BOUNDING_BOX_SIZE_IN_BYTES]);
        assert_compact_debug_snapshot!(expected_bounding_box, @"[0.0, 0.0, 0.0, 0.0]");
        current_offset += BOUNDING_BOX_SIZE_IN_BYTES;
        let expected_nb_offsets: u32 = u32::from_ne_bytes(
            writer[current_offset..current_offset + mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        assert_snapshot!(expected_nb_offsets, @"0");
        current_offset += mem::size_of::<u32>();
        // With 2 elements + the u32 to give us the number of elements we're one u32 off at the end. There should be padding
        let expected_offsets: &[u32] = cast_slice(
            &writer[current_offset
                ..current_offset + mem::size_of::<u32>() * expected_nb_offsets as usize],
        );
        assert_compact_debug_snapshot!(expected_offsets, @"[]");
        current_offset += mem::size_of::<u32>() * expected_nb_offsets as usize;
        // Now there should be a one u32 of padding
        let padding = &writer[current_offset..current_offset + mem::size_of::<u32>()];
        assert_compact_debug_snapshot!(padding, @"[0, 0, 0, 0]");

        // Try to parse the zulti polygon
        let zulti_polygon = ZultiPolygons::from_bytes(&writer);
        assert_snapshot!(zulti_polygon.len(), @"0");
        assert_compact_debug_snapshot!(zulti_polygon.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 0.0, y: 0.0 } }");
        assert_compact_debug_snapshot!(zulti_polygon.offsets, @"[]");
        assert_compact_debug_snapshot!(zulti_polygon.get(0), @"None");
        assert_debug_snapshot!(zulti_polygon, @r"
        ZultiPolygons {
            bounding_box: BoundingBox {
                bottom_left: Coord {
                    x: 0.0,
                    y: 0.0,
                },
                top_right: Coord {
                    x: 0.0,
                    y: 0.0,
                },
            },
            zolygons: [],
        }
        ");
    }

    #[test]
    fn test_multi_polygon_contains_polygon() {
        let polygon = polygon![
             (x: 0., y: 0.),
             (x: 1., y: 0.),
             (x: 1., y: 1.),
             (x: 0., y: 1.),
        ];
        let unrelated_polygon = polygon![
             (x: 100., y: 100.),
             (x: 150., y: 100.),
             (x: 150., y: 150.),
             (x: 100., y: 150.),
        ];
        let contained_polygon = polygon![
             (x: 0.4, y: 0.4),
             (x: 0.6, y: 0.4),
             (x: 0.6, y: 0.6),
             (x: 0.4, y: 0.6),
        ];
        let contained_contained_polygon = polygon![
             (x: 0.45, y: 0.45),
             (x: 0.55, y: 0.45),
             (x: 0.55, y: 0.55),
             (x: 0.45, y: 0.55),
        ];

        let multi_contains = MultiPolygon::new(vec![polygon.clone(), unrelated_polygon.clone()]);
        let multi_contains_and_contained =
            MultiPolygon::new(vec![polygon.clone(), contained_contained_polygon.clone()]);
        let multi_contains_twice =
            MultiPolygon::new(vec![polygon.clone(), contained_polygon.clone()]);
        let multi_contained_twice = MultiPolygon::new(vec![
            contained_polygon.clone(),
            contained_contained_polygon.clone(),
        ]);

        let mut buf = Vec::new();
        Zolygon::write_from_geometry(&mut buf, &polygon).unwrap();
        let zolygon = Zolygon::from_bytes(&buf);
        let mut buf = Vec::new();
        Zolygon::write_from_geometry(&mut buf, &contained_polygon).unwrap();
        let contained_zolygon = Zolygon::from_bytes(&buf);
        let mut buf = Vec::new();
        Zolygon::write_from_geometry(&mut buf, &unrelated_polygon).unwrap();
        let unrelated_zolygon = Zolygon::from_bytes(&buf);

        let mut buf = Vec::new();
        ZultiPolygons::write_from_geometry(&mut buf, &multi_contains).unwrap();
        let multi_contains = ZultiPolygons::from_bytes(&buf);
        let mut buf = Vec::new();
        ZultiPolygons::write_from_geometry(&mut buf, &multi_contains_and_contained).unwrap();
        let multi_contains_and_contained = ZultiPolygons::from_bytes(&buf);
        let mut buf = Vec::new();
        ZultiPolygons::write_from_geometry(&mut buf, &multi_contains_twice).unwrap();
        let multi_contains_twice = ZultiPolygons::from_bytes(&buf);
        let mut buf = Vec::new();
        ZultiPolygons::write_from_geometry(&mut buf, &multi_contained_twice).unwrap();
        let multi_contained_twice = ZultiPolygons::from_bytes(&buf);

        assert_compact_debug_snapshot!(multi_contains.all_relation(&contained_zolygon), @"OutputRelation { contains: Some(true), strict_contains: Some(true), contained: Some(false), strict_contained: Some(false), intersect: Some(false), disjoint: Some(false) }");
        assert_compact_debug_snapshot!(multi_contains_and_contained.all_relation(&contained_zolygon), @"OutputRelation { contains: Some(true), strict_contains: Some(true), contained: Some(true), strict_contained: Some(false), intersect: Some(false), disjoint: Some(false) }");
        assert_compact_debug_snapshot!(multi_contains_twice.all_relation(&contained_zolygon), @"OutputRelation { contains: Some(true), strict_contains: Some(true), contained: Some(false), strict_contained: Some(false), intersect: Some(true), disjoint: Some(false) }");

        assert_compact_debug_snapshot!(multi_contains_and_contained.all_relation(&unrelated_zolygon), @"OutputRelation { contains: Some(false), strict_contains: Some(false), contained: Some(false), strict_contained: Some(false), intersect: Some(false), disjoint: Some(true) }");
        assert_compact_debug_snapshot!(multi_contained_twice.all_relation(&zolygon), @"OutputRelation { contains: Some(false), strict_contains: Some(false), contained: Some(true), strict_contained: Some(true), intersect: Some(false), disjoint: Some(false) }");

        assert_compact_debug_snapshot!(multi_contained_twice.all_relation(&multi_contains), @"OutputRelation { contains: Some(false), strict_contains: Some(false), contained: Some(true), strict_contained: Some(true), intersect: Some(false), disjoint: Some(false) }");
        assert_compact_debug_snapshot!(multi_contains_and_contained.all_relation(&multi_contained_twice), @"OutputRelation { contains: Some(true), strict_contains: Some(true), contained: Some(true), strict_contained: Some(false), intersect: Some(true), disjoint: Some(false) }");
    }
}
