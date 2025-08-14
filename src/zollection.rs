use std::{io, mem};

use geo::{GeometryCollection, MultiLineString, MultiPoint, MultiPolygon, Point};

use crate::{
    BoundingBox, ZultiLines, ZultiPoints, ZultiPolygons, bounding_box::BOUNDING_BOX_SIZE_IN_BYTES,
};

/// This type is used to merge both the feature collection and the geometry collection.
// Since we don't care to return how was structured the initial structure we can just
// flatten everything to be a list of points, a list of lines and a list of polygons.
#[derive(Debug, Clone, Copy)]
pub struct Zollection<'a> {
    bounding_box: &'a BoundingBox,

    // After the bounding box we write exactly two u32
    // The first u32 gives the offset to apply to find the end of the multi points
    // and the beginning of the lines.
    // The second u32 gives the offset to find the end of the lines and the beginning
    // of the polygons.
    // Note: Since we're only using two u32 we're still aligned on 64 bits and don't need any padding

    //
    points: ZultiPoints<'a>,
    lines: ZultiLines<'a>,
    polygons: ZultiPolygons<'a>,
}

impl<'a> Zollection<'a> {
    pub fn new(
        bounding_box: &'a BoundingBox,
        points: ZultiPoints<'a>,
        lines: ZultiLines<'a>,
        polygons: ZultiPolygons<'a>,
    ) -> Self {
        Self {
            bounding_box,
            points,
            lines,
            polygons,
        }
    }

    pub fn from_bytes(data: &'a [u8]) -> Self {
        // 1. Retrieve the bounding box
        let bounding_box = BoundingBox::from_bytes(&data[..BOUNDING_BOX_SIZE_IN_BYTES]);
        let data = &data[BOUNDING_BOX_SIZE_IN_BYTES..];

        // 2. Then retrieve the offsets
        let lines_offset = u32::from_ne_bytes(data[..mem::size_of::<u32>()].try_into().unwrap());
        let data = &data[mem::size_of::<u32>()..];
        let polygons_offset = u32::from_ne_bytes(data[..mem::size_of::<u32>()].try_into().unwrap());
        let data = &data[mem::size_of::<u32>()..];

        let lines_offset = lines_offset as usize;
        let polygons_offset = polygons_offset as usize;

        // 3. Retrieve the internal structures
        let points = &data[..lines_offset];
        let points = ZultiPoints::from_bytes(points);

        let lines = &data[lines_offset..polygons_offset];
        let lines = ZultiLines::from_bytes(lines);

        let polygons = &data[polygons_offset..];
        let polygons = ZultiPolygons::from_bytes(polygons);

        Self {
            bounding_box,
            points,
            lines,
            polygons,
        }
    }

    pub fn write_from_geometry(
        writer: &mut Vec<u8>,
        geometry: &GeometryCollection<f64>,
    ) -> Result<(), io::Error> {
        let (points, lines, polygons) = flatten_geometry_collection(geometry);

        BoundingBox::write_from_geometry(
            writer,
            points
                .iter()
                .copied()
                .chain(
                    lines
                        .0
                        .iter()
                        .flat_map(|line| line.0.iter())
                        .map(|coord| Point::from((coord.x, coord.y))),
                )
                .chain(
                    polygons
                        .iter()
                        .flat_map(|polygon| polygon.exterior().0.iter())
                        .map(|coord| Point::from((coord.x, coord.y))),
                ),
        )?;

        let offsets_pos = writer.len();
        // We'll update the offsets after writing the structures
        writer.extend_from_slice(&0_u32.to_ne_bytes());
        writer.extend_from_slice(&0_u32.to_ne_bytes());

        let base_pos = writer.len();

        // Write the points and update the offset
        ZultiPoints::write_from_geometry(writer, &points)?;
        let line_offset = ((writer.len() - base_pos) as u32).to_ne_bytes();
        writer[offsets_pos..offsets_pos + mem::size_of::<u32>()].copy_from_slice(&line_offset);

        // Write the lines and update the offset
        ZultiLines::write_from_geometry(writer, &lines)?;
        let polygon_offset = ((writer.len() - base_pos) as u32).to_ne_bytes();
        writer[offsets_pos + mem::size_of::<u32>()..offsets_pos + mem::size_of::<u32>() * 2]
            .copy_from_slice(&polygon_offset);

        // Write the polygons and nothing to update anymore
        ZultiPolygons::write_from_geometry(writer, &polygons)?;

        Ok(())
    }

    pub fn bounding_box(&self) -> &'a BoundingBox {
        self.bounding_box
    }

    pub fn len(&self) -> usize {
        self.points.len() + self.lines.len() + self.polygons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn points(&'a self) -> ZultiPoints<'a> {
        self.points
    }

    pub fn lines(&'a self) -> ZultiLines<'a> {
        self.lines
    }

    pub fn polygons(&'a self) -> ZultiPolygons<'a> {
        self.polygons
    }

    /// The geometry collection outputted is completely unrelated to the one inputted.
    /// It has been flattened and contains three parts:
    /// 1. The multi-points
    /// 2. The multi-lines
    /// 3. The multi-polygons
    pub fn to_geo(&self) -> geo_types::GeometryCollection<f64> {
        geo_types::GeometryCollection::from_iter([
            geo::Geometry::from(self.points.to_geo()),
            geo::Geometry::from(self.lines.to_geo()),
            geo::Geometry::from(self.polygons.to_geo()),
        ])
    }
}

fn flatten_geometry_collection(
    collections: &GeometryCollection,
) -> (MultiPoint, MultiLineString, MultiPolygon) {
    let mut points = MultiPoint::new(vec![]);
    let mut lines = MultiLineString::new(vec![]);
    let mut polygons = MultiPolygon::new(vec![]);

    for geometry in collections {
        match geometry {
            geo::Geometry::Point(point) => points.0.push(*point),
            geo::Geometry::MultiPoint(multi_point) => {
                points.0.extend_from_slice(&multi_point.0);
            }
            geo::Geometry::LineString(line_string) => lines.0.push(line_string.clone()),
            geo::Geometry::MultiLineString(multi_line_string) => {
                lines.0.extend_from_slice(&multi_line_string.0);
            }
            geo::Geometry::Polygon(polygon) => polygons.0.push(polygon.clone()),
            geo::Geometry::MultiPolygon(multi_polygon) => {
                polygons.0.extend_from_slice(&multi_polygon.0);
            }

            geo::Geometry::GeometryCollection(geometry_collection) => {
                let (mut pt, mut l, mut pg) = flatten_geometry_collection(geometry_collection);
                points.0.append(&mut pt.0);
                lines.0.append(&mut l.0);
                polygons.0.append(&mut pg.0);
            }

            // The following should never happens in the context of meilisearch since we
            // only handle geojson and they're not valid types
            geo::Geometry::Line(line) => lines.0.push(line.into()),
            geo::Geometry::Rect(rect) => polygons.0.push(rect.to_polygon()),
            geo::Geometry::Triangle(triangle) => polygons.0.push(triangle.to_polygon()),
        }
    }

    (points, lines, polygons)
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;
    use geo::{LineString, Polygon};
    use geo_types::MultiLineString;
    use insta::{assert_compact_debug_snapshot, assert_debug_snapshot, assert_snapshot};

    use super::*;

    fn create_lines(n: usize) -> MultiLineString {
        let first_line = LineString::from(vec![
            Point::from((n as f64, n as f64)),
            Point::from((n as f64 + 1.0, n as f64 + 1.0)),
            Point::from((n as f64 + 2.0, n as f64 + 1.0)),
        ]);
        let second_line = LineString::from(vec![
            Point::from((n as f64 + 3.0, n as f64 + 1.0)),
            Point::from((n as f64 + 4.0, n as f64 + 1.0)),
        ]);
        MultiLineString::new(vec![first_line.clone(), second_line.clone()])
    }

    fn create_polygons(n: usize) -> MultiPolygon {
        let first_polygon = Polygon::new(
            LineString::from(vec![
                Point::from((n as f64, n as f64)),
                Point::from((n as f64 + 1.0, n as f64 + 1.0)),
                Point::from((n as f64 + 2.0, n as f64 + 2.0)),
            ]),
            vec![],
        );
        let second_polygon = Polygon::new(
            LineString::from(vec![
                Point::from((n as f64 + 3.0, n as f64 + 3.0)),
                Point::from((n as f64 + 4.0, n as f64 + 4.0)),
                Point::from((n as f64 + 5.0, n as f64 + 5.0)),
            ]),
            vec![],
        );
        MultiPolygon::from(vec![first_polygon.clone(), second_polygon.clone()])
    }

    #[test]
    fn test_write_from_geometry_with_simple_collection() {
        let multi_points = MultiPoint::from(vec![Point::new(1.0, 2.0), Point::new(3.0, 4.0)]);
        let multi_lines = create_lines(0);
        let multi_polygons = create_polygons(0);
        let collection = GeometryCollection::new_from(vec![
            multi_points.clone().into(),
            multi_lines.clone().into(),
            multi_polygons.clone().into(),
        ]);

        let mut writer = Vec::new();

        Zollection::write_from_geometry(&mut writer, &collection).unwrap();
        // Debug everything at once just to make sure it never changes
        assert_debug_snapshot!(writer);
        let mut current_offset = 0;
        let expected_bounding_box: &[f64] =
            cast_slice(&writer[current_offset..BOUNDING_BOX_SIZE_IN_BYTES]);
        assert_compact_debug_snapshot!(expected_bounding_box, @"[0.0, 0.0, 5.0, 5.0]");
        current_offset += BOUNDING_BOX_SIZE_IN_BYTES;
        let lines_offset: u32 = u32::from_ne_bytes(
            writer[current_offset..current_offset + mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        assert_snapshot!(lines_offset, @"64");
        current_offset += mem::size_of::<u32>();
        let polygon_offset: u32 = u32::from_ne_bytes(
            writer[current_offset..current_offset + mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        assert_snapshot!(polygon_offset, @"256");
        current_offset += mem::size_of::<u32>();

        // Now there should be the first multi points from the offset 0 to the offset line
        let points_bytes = &writer[current_offset..current_offset + lines_offset as usize];
        let points_f64: &[f64] = cast_slice(points_bytes);
        assert_compact_debug_snapshot!(points_f64, @"[1.0, 2.0, 3.0, 4.0, 1.0, 2.0, 3.0, 4.0]");
        let points = ZultiPoints::from_bytes(points_bytes);
        assert_compact_debug_snapshot!(points, @"ZultiPoints { bounding_box: BoundingBox { bottom_left: Coord { x: 1.0, y: 2.0 }, top_right: Coord { x: 3.0, y: 4.0 } }, points: [Zoint { lng: 1.0, lat: 2.0 }, Zoint { lng: 3.0, lat: 4.0 }] }");
        assert_eq!(points, multi_points);

        // Now there should be the first multi lines at the offset line to the offset polygon
        let lines_bytes = &writer
            [current_offset + lines_offset as usize..current_offset + polygon_offset as usize];
        assert_compact_debug_snapshot!(lines_bytes, @"[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 64, 0, 0, 0, 0, 0, 0, 240, 63, 2, 0, 0, 0, 0, 0, 0, 0, 80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 8, 64, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 16, 64, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 8, 64, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 16, 64, 0, 0, 0, 0, 0, 0, 240, 63]");
        let lines = ZultiLines::from_bytes(lines_bytes);
        assert_compact_debug_snapshot!(lines, @"ZultiLines { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 4.0, y: 1.0 } }, zines: [Zine { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 2.0, y: 1.0 } }, points: [Zoint { lng: 0.0, lat: 0.0 }, Zoint { lng: 1.0, lat: 1.0 }, Zoint { lng: 2.0, lat: 1.0 }] }, Zine { bounding_box: BoundingBox { bottom_left: Coord { x: 3.0, y: 1.0 }, top_right: Coord { x: 4.0, y: 1.0 } }, points: [Zoint { lng: 3.0, lat: 1.0 }, Zoint { lng: 4.0, lat: 1.0 }] }] }");
        assert_eq!(lines, multi_lines);

        // Now there should be the first multi lines at the offset line to the offset polygon
        let polygons_bytes = &writer[current_offset + polygon_offset as usize..];
        assert_compact_debug_snapshot!(polygons_bytes, @"[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 64, 0, 0, 0, 0, 0, 0, 20, 64, 2, 0, 0, 0, 0, 0, 0, 0, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 64, 0, 0, 0, 0, 0, 0, 8, 64, 0, 0, 0, 0, 0, 0, 20, 64, 0, 0, 0, 0, 0, 0, 20, 64, 0, 0, 0, 0, 0, 0, 8, 64, 0, 0, 0, 0, 0, 0, 8, 64, 0, 0, 0, 0, 0, 0, 16, 64, 0, 0, 0, 0, 0, 0, 16, 64, 0, 0, 0, 0, 0, 0, 20, 64, 0, 0, 0, 0, 0, 0, 20, 64, 0, 0, 0, 0, 0, 0, 8, 64, 0, 0, 0, 0, 0, 0, 8, 64]");
        let polygons = ZultiPolygons::from_bytes(polygons_bytes);
        assert_compact_debug_snapshot!(polygons, @"ZultiPolygons { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 5.0, y: 5.0 } }, zolygons: [Zolygon { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 2.0, y: 2.0 } }, coords: [Coord { x: 0.0, y: 0.0 }, Coord { x: 1.0, y: 1.0 }, Coord { x: 2.0, y: 2.0 }, Coord { x: 0.0, y: 0.0 }] }, Zolygon { bounding_box: BoundingBox { bottom_left: Coord { x: 3.0, y: 3.0 }, top_right: Coord { x: 5.0, y: 5.0 } }, coords: [Coord { x: 3.0, y: 3.0 }, Coord { x: 4.0, y: 4.0 }, Coord { x: 5.0, y: 5.0 }, Coord { x: 3.0, y: 3.0 }] }] }");
        assert_eq!(polygons, multi_polygons);

        // Try to parse the whole collection
        let zollection = Zollection::from_bytes(&writer);
        assert_compact_debug_snapshot!(zollection.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 5.0, y: 5.0 } }");
        assert_debug_snapshot!(zollection, @r"
        Zollection {
            bounding_box: BoundingBox {
                bottom_left: Coord {
                    x: 0.0,
                    y: 0.0,
                },
                top_right: Coord {
                    x: 5.0,
                    y: 5.0,
                },
            },
            points: ZultiPoints {
                bounding_box: BoundingBox {
                    bottom_left: Coord {
                        x: 1.0,
                        y: 2.0,
                    },
                    top_right: Coord {
                        x: 3.0,
                        y: 4.0,
                    },
                },
                points: [
                    Zoint {
                        lng: 1.0,
                        lat: 2.0,
                    },
                    Zoint {
                        lng: 3.0,
                        lat: 4.0,
                    },
                ],
            },
            lines: ZultiLines {
                bounding_box: BoundingBox {
                    bottom_left: Coord {
                        x: 0.0,
                        y: 0.0,
                    },
                    top_right: Coord {
                        x: 4.0,
                        y: 1.0,
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
                                x: 2.0,
                                y: 1.0,
                            },
                        },
                        points: [
                            Zoint {
                                lng: 0.0,
                                lat: 0.0,
                            },
                            Zoint {
                                lng: 1.0,
                                lat: 1.0,
                            },
                            Zoint {
                                lng: 2.0,
                                lat: 1.0,
                            },
                        ],
                    },
                    Zine {
                        bounding_box: BoundingBox {
                            bottom_left: Coord {
                                x: 3.0,
                                y: 1.0,
                            },
                            top_right: Coord {
                                x: 4.0,
                                y: 1.0,
                            },
                        },
                        points: [
                            Zoint {
                                lng: 3.0,
                                lat: 1.0,
                            },
                            Zoint {
                                lng: 4.0,
                                lat: 1.0,
                            },
                        ],
                    },
                ],
            },
            polygons: ZultiPolygons {
                bounding_box: BoundingBox {
                    bottom_left: Coord {
                        x: 0.0,
                        y: 0.0,
                    },
                    top_right: Coord {
                        x: 5.0,
                        y: 5.0,
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
                                x: 2.0,
                                y: 2.0,
                            },
                        },
                        coords: [
                            Coord {
                                x: 0.0,
                                y: 0.0,
                            },
                            Coord {
                                x: 1.0,
                                y: 1.0,
                            },
                            Coord {
                                x: 2.0,
                                y: 2.0,
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
                                x: 3.0,
                                y: 3.0,
                            },
                            top_right: Coord {
                                x: 5.0,
                                y: 5.0,
                            },
                        },
                        coords: [
                            Coord {
                                x: 3.0,
                                y: 3.0,
                            },
                            Coord {
                                x: 4.0,
                                y: 4.0,
                            },
                            Coord {
                                x: 5.0,
                                y: 5.0,
                            },
                            Coord {
                                x: 3.0,
                                y: 3.0,
                            },
                        ],
                    },
                ],
            },
        }
        ");
    }

    #[test]
    fn empty_collection() {
        let collection = GeometryCollection::new_from(vec![]);

        let mut writer = Vec::new();

        Zollection::write_from_geometry(&mut writer, &collection).unwrap();
        // Debug everything at once just to make sure it never changes
        assert_debug_snapshot!(writer);
        let mut current_offset = 0;
        let expected_bounding_box: &[f64] =
            cast_slice(&writer[current_offset..BOUNDING_BOX_SIZE_IN_BYTES]);
        assert_compact_debug_snapshot!(expected_bounding_box, @"[0.0, 0.0, 0.0, 0.0]");
        current_offset += BOUNDING_BOX_SIZE_IN_BYTES;
        let lines_offset: u32 = u32::from_ne_bytes(
            writer[current_offset..current_offset + mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        assert_snapshot!(lines_offset, @"32");
        current_offset += mem::size_of::<u32>();
        let polygon_offset: u32 = u32::from_ne_bytes(
            writer[current_offset..current_offset + mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        assert_snapshot!(polygon_offset, @"72");
        current_offset += mem::size_of::<u32>();

        // Now there should be the first multi points from the offset 0 to the offset line
        let points_bytes = &writer[current_offset..current_offset + lines_offset as usize];
        let points_f64: &[f64] = cast_slice(points_bytes);
        assert_compact_debug_snapshot!(points_f64, @"[0.0, 0.0, 0.0, 0.0]");
        let points = ZultiPoints::from_bytes(points_bytes);
        assert_compact_debug_snapshot!(points, @"ZultiPoints { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 0.0, y: 0.0 } }, points: [] }");
        assert!(points.is_empty());

        // Now there should be the first multi lines at the offset line to the offset polygon
        let lines_bytes = &writer
            [current_offset + lines_offset as usize..current_offset + polygon_offset as usize];
        assert_compact_debug_snapshot!(lines_bytes, @"[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]");
        let lines = ZultiLines::from_bytes(lines_bytes);
        assert_compact_debug_snapshot!(lines, @"ZultiLines { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 0.0, y: 0.0 } }, zines: [] }");
        assert!(lines.is_empty());

        // Now there should be the first multi lines at the offset line to the offset polygon
        let polygons_bytes = &writer[current_offset + polygon_offset as usize..];
        assert_compact_debug_snapshot!(polygons_bytes, @"[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]");
        let polygons = ZultiPolygons::from_bytes(polygons_bytes);
        assert_compact_debug_snapshot!(polygons, @"ZultiPolygons { bounding_box: BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 0.0, y: 0.0 } }, zolygons: [] }");
        assert!(polygons.is_empty());

        // Try to parse the whole collection
        let zollection = Zollection::from_bytes(&writer);
        assert_compact_debug_snapshot!(zollection.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 0.0, y: 0.0 }, top_right: Coord { x: 0.0, y: 0.0 } }");
        assert_debug_snapshot!(zollection, @r"
        Zollection {
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
            points: ZultiPoints {
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
                points: [],
            },
            lines: ZultiLines {
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
            },
            polygons: ZultiPolygons {
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
            },
        }
        ");
        assert!(zollection.is_empty());
    }

    #[test]
    fn nested_stuff() {
        let collection = GeometryCollection::new_from(vec![geo::Geometry::GeometryCollection(
            GeometryCollection::new_from(vec![geo::Geometry::GeometryCollection(
                GeometryCollection::new_from(vec![geo::Geometry::GeometryCollection(
                    GeometryCollection::new_from(vec![
                        geo::Geometry::GeometryCollection(GeometryCollection::new_from(vec![
                            Point::new(1.0, 2.0).into(),
                        ])),
                        Point::new(3.0, 4.0).into(),
                    ]),
                )]),
            )]),
        )]);

        let mut writer = Vec::new();

        Zollection::write_from_geometry(&mut writer, &collection).unwrap();

        // Try to parse the whole collection
        let zollection = Zollection::from_bytes(&writer);
        assert_compact_debug_snapshot!(zollection.bounding_box(), @"BoundingBox { bottom_left: Coord { x: 1.0, y: 2.0 }, top_right: Coord { x: 3.0, y: 4.0 } }");
        assert_debug_snapshot!(zollection, @r"
        Zollection {
            bounding_box: BoundingBox {
                bottom_left: Coord {
                    x: 1.0,
                    y: 2.0,
                },
                top_right: Coord {
                    x: 3.0,
                    y: 4.0,
                },
            },
            points: ZultiPoints {
                bounding_box: BoundingBox {
                    bottom_left: Coord {
                        x: 1.0,
                        y: 2.0,
                    },
                    top_right: Coord {
                        x: 3.0,
                        y: 4.0,
                    },
                },
                points: [
                    Zoint {
                        lng: 1.0,
                        lat: 2.0,
                    },
                    Zoint {
                        lng: 3.0,
                        lat: 4.0,
                    },
                ],
            },
            lines: ZultiLines {
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
            },
            polygons: ZultiPolygons {
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
            },
        }
        ");
        assert!(!zollection.is_empty());
    }
}
