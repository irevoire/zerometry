use crate::{Coord, Relation, RelationBetweenShapes, Zerometry, Zoint};

#[test]
fn test_mono_multipolygon_contains_points() {
    // This test highlights a bug I found on cellulite where a point was considered contained in an h3 cell even though it clearly is not.
    let buffer = [6.0197316417968105, 49.63676497357687];
    let point = Zoint::new(&Coord::from_slice(&buffer));
    println!("point: {}", print_geojson(&point.into()));

    let wrong_multipolygon = geo_types::MultiPolygon::new(vec![geo_types::Polygon::new(
        geo_types::LineString::from(vec![
            geo_types::Point::new(-6.436337296790293, 55.37739041554851),
            geo_types::Point::new(-4.889760342933786, 51.22372845966178),
            geo_types::Point::new(1.188509553443464, 49.47027919866874),
            geo_types::Point::new(3.6300086390995316, 50.610463312569514),
            geo_types::Point::new(6.259687055981991, 51.96477015603749),
            geo_types::Point::new(5.52364654929031, 55.70676846515227),
            geo_types::Point::new(-0.9315871635106105, 57.689497374592854),
            geo_types::Point::new(-6.436337296790293, 55.37739041554851),
        ]),
        Vec::new(),
    )]);
    let mut wrong_buffer = Vec::new();
    Zerometry::write_from_geometry(
        &mut wrong_buffer,
        &geo_types::Geometry::MultiPolygon(wrong_multipolygon.clone()),
    )
    .unwrap();
    let wrong_multipolygon = Zerometry::from_bytes(&wrong_buffer).unwrap();
    let wrong = wrong_multipolygon.relation(&point);
    assert_eq!(wrong, Relation::Disjoint);

    let right_multipolygon = geo_types::MultiPolygon::new(vec![geo_types::Polygon::new(
        geo_types::LineString::from(vec![
            geo_types::Point::new(7.509948481928903, 43.78660935394501),
            geo_types::Point::new(12.677317810359865, 46.406957457982266),
            geo_types::Point::new(12.345747364400054, 50.55427508726938),
            geo_types::Point::new(6.259687055981991, 51.96477015603749),
            geo_types::Point::new(3.630008639099554, 50.610463312569486),
            geo_types::Point::new(1.188509553443464, 49.47027919866874),
            geo_types::Point::new(2.026568965384611, 45.18424868970644),
            geo_types::Point::new(7.509948481928903, 43.78660935394501),
        ]),
        Vec::new(),
    )]);

    let mut right_buffer = Vec::new();
    Zerometry::write_from_geometry(
        &mut right_buffer,
        &geo_types::Geometry::MultiPolygon(right_multipolygon.clone()),
    )
    .unwrap();
    let right_multipolygon = Zerometry::from_bytes(&right_buffer).unwrap();
    println!("right_multipolygon: {}", print_geojson(&right_multipolygon));

    let right = right_multipolygon.relation(&point);
    assert_eq!(right, Relation::Contains);
}

fn print_geojson(geometry: &Zerometry) -> String {
    geojson::GeoJson::Geometry(geojson::Geometry::new(geojson::Value::from(
        &geometry.to_geo(),
    )))
    .to_string_pretty()
    .unwrap()
}
