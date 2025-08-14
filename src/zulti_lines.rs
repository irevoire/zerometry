use std::{fmt, io, mem};

use bytemuck::cast_slice;
use geo_types::{MultiLineString, Point};

use crate::{
    BoundingBox, Relation, RelationBetweenShapes, Zerometry, Zoint, Zolygon, ZultiPoints,
    ZultiPolygons, bounding_box::BOUNDING_BOX_SIZE_IN_BYTES, zine::Zine,
};

#[derive(Clone, Copy)]
pub struct ZultiLines<'a> {
    bounding_box: &'a BoundingBox,
    // In the binary format we store the number of offsets here
    // If it's 0, it means that the multi lines is empty
    // If it's odd it means we also inserted one extra offset at the end for padding that should not ends up in the slice
    offsets: &'a [u32],
    bytes: &'a [u8],
}

impl<'a> ZultiLines<'a> {
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
        // 2.3 If we have an even number of offsets, there is one u32 of padding at the end that we must skip before retrieving coords of the lines
        let data = if offsets_count % 2 == 0 {
            debug_assert_eq!(data[0..mem::size_of::<u32>()], [0, 0, 0, 0]);
            &data[mem::size_of::<u32>()..]
        } else {
            data
        };
        // 3. Finally retrieve the lines
        let bytes = data;

        Self {
            bounding_box,
            offsets,
            bytes,
        }
    }

    pub fn write_from_geometry(
        writer: &mut Vec<u8>,
        geometry: &MultiLineString<f64>,
    ) -> Result<(), io::Error> {
        BoundingBox::write_from_geometry(
            writer,
            geometry
                .0
                .iter()
                .flat_map(|line| line.0.iter())
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
            // If we have an even number of lines, we must add an extra offset at the end for padding
            writer.extend(0_u32.to_ne_bytes());
        }
        let start = writer.len();
        let mut offsets = Vec::new();
        for line in geometry.iter() {
            offsets.push(writer.len() as u32 - start as u32);
            Zine::write_from_geometry(writer, line)?;
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

    pub fn get(&self, index: usize) -> Option<Zine<'a>> {
        let offset = *self.offsets.get(index)?;
        let next_offset = *self
            .offsets
            .get(index + 1)
            .unwrap_or(&(self.bytes.len() as u32));
        let bytes = &self.bytes[offset as usize..next_offset as usize];
        Some(Zine::from_bytes(bytes))
    }

    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn lines(&'a self) -> impl Iterator<Item = Zine<'a>> {
        (0..self.len()).map(move |index| self.get(index).unwrap())
    }

    pub fn to_geo(&self) -> geo_types::MultiLineString<f64> {
        geo_types::MultiLineString::new(self.lines().map(|zine| zine.to_geo()).collect())
    }
}

impl<'a> fmt::Debug for ZultiLines<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct ZinesDebug<'b, 'a>(&'b ZultiLines<'a>);

        impl<'b, 'a> fmt::Debug for ZinesDebug<'b, 'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.0.lines()).finish()
            }
        }

        f.debug_struct("ZultiLines")
            .field("bounding_box", &self.bounding_box())
            .field("zines", &ZinesDebug(self))
            .finish()
    }
}

// points and line have nothing in common
impl<'a> RelationBetweenShapes<Zoint<'a>> for ZultiLines<'a> {
    fn relation(&self, _other: &Zoint) -> Relation {
        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<ZultiPoints<'a>> for ZultiLines<'a> {
    fn relation(&self, _other: &ZultiPoints) -> Relation {
        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<Zine<'a>> for ZultiLines<'a> {
    fn relation(&self, other: &Zine) -> Relation {
        if self.is_empty()
            || other.is_empty()
            || self.bounding_box().relation(other.bounding_box()) == Relation::Disjoint
        {
            return Relation::Disjoint;
        }

        for line in self.lines() {
            if line.relation(other) == Relation::Intersects {
                return Relation::Intersects;
            }
        }

        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<ZultiLines<'a>> for ZultiLines<'a> {
    fn relation(&self, other: &ZultiLines) -> Relation {
        if self.is_empty()
            || other.is_empty()
            || self.bounding_box().relation(other.bounding_box()) == Relation::Disjoint
        {
            return Relation::Disjoint;
        }

        for left in self.lines() {
            for right in other.lines() {
                if left.relation(&right) == Relation::Intersects {
                    return Relation::Intersects;
                }
            }
        }

        Relation::Disjoint
    }
}

impl<'a> RelationBetweenShapes<Zolygon<'a>> for ZultiLines<'a> {
    fn relation(&self, other: &Zolygon) -> Relation {
        if self.is_empty() || other.is_empty() {
            return Relation::Disjoint;
        }
        if self.bounding_box().relation(other.bounding_box()) == Relation::Disjoint {
            return Relation::Disjoint;
        }
        let mut relation = Relation::Disjoint;
        for line in self.lines() {
            match line.relation(other) {
                // Inretsects take precedence over contained
                Relation::Contained if relation != Relation::Intersects => {
                    relation = Relation::Contained
                }
                Relation::Intersects => relation = Relation::Intersects,
                Relation::Disjoint | Relation::Contained | Relation::Contains => {}
            }
        }
        relation
    }
}

impl<'a> RelationBetweenShapes<ZultiPolygons<'a>> for ZultiLines<'a> {
    fn relation(&self, other: &ZultiPolygons) -> Relation {
        if self.is_empty() || other.is_empty() {
            return Relation::Disjoint;
        }
        if self.bounding_box().relation(other.bounding_box()) == Relation::Disjoint {
            return Relation::Disjoint;
        }
        let mut relation = Relation::Disjoint;
        for line in self.lines() {
            match line.relation(other) {
                // Inretsects take precedence over contained
                Relation::Contained if relation != Relation::Intersects => {
                    relation = Relation::Contained
                }
                Relation::Intersects => relation = Relation::Intersects,
                Relation::Disjoint | Relation::Contained | Relation::Contains => {}
            }
        }
        relation
    }
}

impl<'a> RelationBetweenShapes<Zerometry<'a>> for ZultiLines<'a> {
    fn relation(&self, other: &Zerometry<'a>) -> Relation {
        match other.relation(self) {
            Relation::Contains => Relation::Contained,
            Relation::Contained => Relation::Contains,
            r => r,
        }
    }
}

impl PartialEq<MultiLineString> for ZultiLines<'_> {
    fn eq(&self, other: &MultiLineString) -> bool {
        self.lines()
            .zip(other.0.iter())
            .all(|(zine, line)| zine.eq(line))
    }
}

#[cfg(test)]
mod tests {
    use geo::LineString;
    use geo_types::MultiLineString;
    use insta::{assert_compact_debug_snapshot, assert_debug_snapshot, assert_snapshot};

    use super::*;

    #[test]
    fn test_write_from_geometry_with_even_number_of_elements() {
        let first_line = LineString::from(vec![
            Point::from((0.0, 0.0)),
            Point::from((10.0, 0.0)),
            Point::from((0.0, 10.0)),
        ]);
        let second_line = LineString::from(vec![
            Point::from((10.0, 10.0)),
            Point::from((20.0, 0.0)),
            Point::from((20.0, 10.0)),
        ]);
        let geometry = MultiLineString::new(vec![first_line.clone(), second_line.clone()]);

        let mut writer = Vec::new();

        ZultiLines::write_from_geometry(&mut writer, &geometry).unwrap();
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
        assert_compact_debug_snapshot!(expected_offsets, @"[0, 80]");
        current_offset += mem::size_of::<u32>() * expected_nb_offsets as usize;
        // Now there should be a one u32 of padding
        let padding = &writer[current_offset..current_offset + mem::size_of::<u32>()];
        assert_compact_debug_snapshot!(padding, @"[0, 0, 0, 0]");
        current_offset += mem::size_of::<u32>();
        // Now there should be the first zine at the offset 0
        let first_zine_bytes = &writer[current_offset + expected_offsets[0] as usize
            ..current_offset + expected_offsets[1] as usize];
        assert_compact_debug_snapshot!(first_zine_bytes, @"[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64]");
        let first_zine = Zine::from_bytes(first_zine_bytes);
        assert_compact_debug_snapshot!(first_zine, @"Zine { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }, points: [Zoint { lng: 0.0, lat: 0.0 }, Zoint { lng: 10.0, lat: 0.0 }, Zoint { lng: 0.0, lat: 10.0 }] }");
        assert_eq!(first_zine, first_line);
        let second_zine_bytes = &writer[current_offset + expected_offsets[1] as usize..];
        assert_compact_debug_snapshot!(second_zine_bytes, @"[0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 52, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 52, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 52, 64, 0, 0, 0, 0, 0, 0, 36, 64]");
        let second_zine = Zine::from_bytes(second_zine_bytes);
        assert_compact_debug_snapshot!(second_zine, @"Zine { bounding_box: BoundingBox { bottom_left: Coord { x: 10.0, y: 0.0 }, top_right: Coord { x: 20.0, y: 10.0 } }, points: [Zoint { lng: 10.0, lat: 10.0 }, Zoint { lng: 20.0, lat: 0.0 }, Zoint { lng: 20.0, lat: 10.0 }] }");
        assert_eq!(second_zine, second_line);

        // Try to parse the zulti lines
        let zulti_lines = ZultiLines::from_bytes(&writer);
        assert_snapshot!(zulti_lines.len(), @"2");
        assert_compact_debug_snapshot!(zulti_lines.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 20.0, y: 10.0 } }");
        assert_compact_debug_snapshot!(zulti_lines.offsets, @"[0, 80]");
        assert_compact_debug_snapshot!(zulti_lines.get(0).unwrap(), @"Zine { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }, points: [Zoint { lng: 0.0, lat: 0.0 }, Zoint { lng: 10.0, lat: 0.0 }, Zoint { lng: 0.0, lat: 10.0 }] }");
        assert_compact_debug_snapshot!(zulti_lines.get(1).unwrap(), @"Zine { bounding_box: BoundingBox { bottom_left: Coord { x: 10.0, y: 0.0 }, top_right: Coord { x: 20.0, y: 10.0 } }, points: [Zoint { lng: 10.0, lat: 10.0 }, Zoint { lng: 20.0, lat: 0.0 }, Zoint { lng: 20.0, lat: 10.0 }] }");
        assert_compact_debug_snapshot!(zulti_lines.get(2), @"None");
        assert_debug_snapshot!(zulti_lines, @r"
        ZultiLines {
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
            zines: [
                Zine {
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
                    points: [
                        Zoint {
                            lng: 0.0,
                            lat: 0.0,
                        },
                        Zoint {
                            lng: 10.0,
                            lat: 0.0,
                        },
                        Zoint {
                            lng: 0.0,
                            lat: 10.0,
                        },
                    ],
                },
                Zine {
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
                    points: [
                        Zoint {
                            lng: 10.0,
                            lat: 10.0,
                        },
                        Zoint {
                            lng: 20.0,
                            lat: 0.0,
                        },
                        Zoint {
                            lng: 20.0,
                            lat: 10.0,
                        },
                    ],
                },
            ],
        }
        ");
    }

    #[test]
    fn test_write_from_geometry_with_odd_number_of_elements() {
        let first_line = LineString::from(vec![
            Point::from((0.0, 0.0)),
            Point::from((10.0, 0.0)),
            Point::from((0.0, 10.0)),
        ]);
        let geometry = MultiLineString::new(vec![first_line.clone()]);

        let mut writer = Vec::new();

        ZultiLines::write_from_geometry(&mut writer, &geometry).unwrap();
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
        // Now there should be the first zine at the offset 0
        let first_zine_bytes = &writer[current_offset + expected_offsets[0] as usize..];
        assert_compact_debug_snapshot!(first_zine_bytes, @"[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64]");
        let first_zine = Zine::from_bytes(first_zine_bytes);
        assert_compact_debug_snapshot!(first_zine, @"Zine { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }, points: [Zoint { lng: 0.0, lat: 0.0 }, Zoint { lng: 10.0, lat: 0.0 }, Zoint { lng: 0.0, lat: 10.0 }] }");
        assert_eq!(first_zine, first_line);

        // Try to parse the zulti lines
        let zulti_polygon = ZultiLines::from_bytes(&writer);
        assert_snapshot!(zulti_polygon.len(), @"1");
        assert_compact_debug_snapshot!(zulti_polygon.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }");
        assert_compact_debug_snapshot!(zulti_polygon.offsets, @"[0]");
        assert_compact_debug_snapshot!(zulti_polygon.get(0).unwrap(), @"Zine { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 10.0, y: 10.0 } }, points: [Zoint { lng: 0.0, lat: 0.0 }, Zoint { lng: 10.0, lat: 0.0 }, Zoint { lng: 0.0, lat: 10.0 }] }");
        assert_compact_debug_snapshot!(zulti_polygon.get(1), @"None");
        assert_debug_snapshot!(zulti_polygon, @r"
        ZultiLines {
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
            zines: [
                Zine {
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
                    points: [
                        Zoint {
                            lng: 0.0,
                            lat: 0.0,
                        },
                        Zoint {
                            lng: 10.0,
                            lat: 0.0,
                        },
                        Zoint {
                            lng: 0.0,
                            lat: 10.0,
                        },
                    ],
                },
            ],
        }
        ");
    }

    #[test]
    fn test_write_from_geometry_with_no_elements() {
        let geometry = MultiLineString::new(vec![]);

        let mut writer = Vec::new();

        ZultiLines::write_from_geometry(&mut writer, &geometry).unwrap();
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

        // Try to parse the zulti lines
        let zulti_lines = ZultiLines::from_bytes(&writer);
        assert_snapshot!(zulti_lines.len(), @"0");
        assert_compact_debug_snapshot!(zulti_lines.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 0.0, y: 0.0 } }");
        assert_compact_debug_snapshot!(zulti_lines.offsets, @"[]");
        assert_compact_debug_snapshot!(zulti_lines.get(0), @"None");
        assert_debug_snapshot!(zulti_lines, @r"
        ZultiLines {
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
            zines: [],
        }
        ");
    }
}
