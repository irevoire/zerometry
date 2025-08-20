use std::str::FromStr;

use geo::{LineString, Polygon};
use insta::assert_compact_debug_snapshot;
use zerometry::{OutputRelation, RelationBetweenShapes, Zerometry, Zolygon};

const BREAU_ET_SALAGOSSE: &str = include_str!("assets/breau-et-salagosse.geojson");

#[test]
fn bug_breo_et_salagosse() {
    /*
    Original query:
    Coords: [
    COORD(3.5699973106384277 43.97279739379883),
    COORD(3.5925638675689697 44.001197814941406),
    COORD(3.6337668895721436 44.00157165527344),
    COORD(3.6359827518463135 43.98838806152344),
    COORD(3.611032247543335 43.983375549316406),
    COORD(3.5965945720672607 43.96871566772461),
    COORD(3.5699973106384277 43.97279739379883)]
     */
    let mut buffer = Vec::new();
    Zolygon::write_from_geometry(
        &mut buffer,
        &Polygon::new(
            LineString::new(vec![
                geo_types::Coord {
                    x: 3.5699973106384277,
                    y: 43.97279739379883,
                },
                geo_types::Coord {
                    x: 3.5925638675689697,
                    y: 44.001197814941406,
                },
                geo_types::Coord {
                    x: 3.6337668895721436,
                    y: 44.00157165527344,
                },
                geo_types::Coord {
                    x: 3.6359827518463135,
                    y: 43.98838806152344,
                },
                geo_types::Coord {
                    x: 3.611032247543335,
                    y: 43.983375549316406,
                },
                geo_types::Coord {
                    x: 3.5965945720672607,
                    y: 43.96871566772461,
                },
                geo_types::Coord {
                    x: 3.5699973106384277,
                    y: 43.97279739379883,
                },
            ]),
            Vec::new(),
        ),
    )
    .unwrap();
    let first = buffer.len();

    let breau = geojson::GeoJson::from_str(BREAU_ET_SALAGOSSE).unwrap();
    Zerometry::write_from_geometry(&mut buffer, &breau.try_into().unwrap()).unwrap();
    let second = buffer.len();
    let query = Zolygon::from_bytes(&buffer[..first]);
    let breau = Zerometry::from_bytes(&buffer[first..second]).unwrap();

    let query_bb = query.bounding_box();
    let breau_bb = breau.to_polygon().unwrap().bounding_box();
    assert_compact_debug_snapshot!(
        query_bb.all_relation(breau_bb),
        @"OutputRelation { contains: Some(false), strict_contains: Some(false), contained: Some(false), strict_contained: Some(false), intersect: Some(true), disjoint: Some(false) }"
    );
    assert_compact_debug_snapshot!(
        breau_bb.all_relation(query_bb),
        @"OutputRelation { contains: Some(false), strict_contains: Some(false), contained: Some(false), strict_contained: Some(false), intersect: Some(true), disjoint: Some(false) }"
    );

    assert_compact_debug_snapshot!(breau.all_relation(&query), @"OutputRelation { contains: Some(false), strict_contains: Some(false), contained: Some(false), strict_contained: Some(false), intersect: Some(true), disjoint: Some(false) }");
    assert_compact_debug_snapshot!(query.all_relation(&breau), @"OutputRelation { contains: Some(false), strict_contains: Some(false), contained: Some(false), strict_contained: Some(false), intersect: Some(true), disjoint: Some(false) }");
}
