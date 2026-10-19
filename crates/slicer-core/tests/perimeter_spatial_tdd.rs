use slicer_core::perimeter_utils::{
    expolygon_to_path3d, point_in_any_polygon, signed_distance_to_boundary,
};
use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{ExPolygon, Point2, Point3WithWidth, Polygon};

struct SpatialFixture {
    boundary: Vec<ExPolygon>,
    overhang_bands: Vec<QuartileBand>,
    bridge_areas: Vec<ExPolygon>,
    path_contour: Polygon,
}

struct Observed {
    distances: Vec<f32>,
    distance_ordinals: Vec<Option<usize>>,
    quartiles: Vec<Option<u8>>,
    bridges: Vec<bool>,
    planar_paths: Vec<Vec<Point3WithWidth>>,
    nonplanar_paths: Vec<Vec<Point3WithWidth>>,
}

fn point_mm(x: f32, y: f32) -> Point2 {
    Point2::from_mm(x, y)
}

fn polygon_mm(points: &[(f32, f32)]) -> Polygon {
    Polygon {
        points: points.iter().map(|&(x, y)| point_mm(x, y)).collect(),
    }
}

fn expolygon_mm(contour: &[(f32, f32)], holes: &[&[(f32, f32)]]) -> ExPolygon {
    ExPolygon {
        contour: polygon_mm(contour),
        holes: holes.iter().map(|hole| polygon_mm(hole)).collect(),
    }
}

fn quartile_band(quartile: u8, polygons: &[&[(f32, f32)]]) -> QuartileBand {
    QuartileBand {
        quartile,
        polygons: polygons
            .iter()
            .map(|ring| expolygon_mm(ring, &[]))
            .collect(),
    }
}

fn adversarial_fixture() -> SpatialFixture {
    let computed_endpoint = (0.1_f32 + 0.2_f32) * 10.0_f32;
    let boundary = vec![
        expolygon_mm(
            &[
                (-10.0, -10.0),
                (10.0, -10.0),
                (10.0, computed_endpoint),
                (-10.0, 10.0),
            ],
            &[&[(-2.0, -2.0), (2.0, -2.0), (2.0, 2.0), (-2.0, 2.0)]],
        ),
        expolygon_mm(
            &[(20.0, -10.0), (40.0, -10.0), (40.0, 10.0), (20.0, 10.0)],
            &[],
        ),
        expolygon_mm(&[(50.0, 50.0), (50.0, 50.0)], &[]),
        expolygon_mm(&[], &[]),
    ];
    let overhang_bands = vec![
        quartile_band(2, &[&[(-1.5, -1.5), (1.5, -1.5), (1.5, 1.5), (-1.5, 1.5)]]),
        quartile_band(4, &[&[(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]]),
        quartile_band(
            1,
            &[&[(19.0, -1.0), (21.0, -1.0), (21.0, 1.0), (19.0, 1.0)]],
        ),
    ];
    let mut bridge_areas = vec![expolygon_mm(
        &[(-3.0, -3.0), (3.0, -3.0), (3.0, 3.0), (-3.0, 3.0)],
        &[],
    )];
    for origin in [100.0, 110.0, 120.0, 130.0, 140.0, 150.0] {
        bridge_areas.push(expolygon_mm(
            &[
                (origin, -1.0),
                (origin + 2.0, -1.0),
                (origin + 2.0, 1.0),
                (origin, 1.0),
            ],
            &[],
        ));
    }
    let path_contour = polygon_mm(&[
        (-4.0, -4.0),
        (4.0, -4.0),
        (4.0, 4.0),
        (-4.0, 4.0),
        (computed_endpoint, 4.0),
    ]);

    SpatialFixture {
        boundary,
        overhang_bands,
        bridge_areas,
        path_contour,
    }
}

fn query_points() -> Vec<(f32, f32)> {
    let computed_endpoint = (0.1_f32 + 0.2_f32) * 10.0_f32;
    vec![
        (-10.0, 0.0),
        (0.0, 0.0),
        (2.0, 2.0),
        (-20.0, -20.0),
        (30.0, 0.0),
        (f32::from_bits(0x8000_0000), 0.0),
        (computed_endpoint, 4.0),
    ]
}

fn bridge_points() -> Vec<Point2> {
    vec![
        point_mm(0.0, 0.0),
        point_mm(3.0, 0.0),
        point_mm(-3.0, 0.0),
        point_mm(-20.0, -20.0),
    ]
}

fn path_widths() -> [f32; 3] {
    [0.0, 0.4, 1.25]
}

fn collect_legacy(fixture: &SpatialFixture) -> Observed {
    let distances = query_points()
        .into_iter()
        .map(|(x, y)| signed_distance_to_boundary(x, y, &fixture.boundary))
        .collect();
    let quartiles = query_points()
        .into_iter()
        .map(|(x, y)| {
            fixture
                .overhang_bands
                .iter()
                .filter(|band| {
                    band.polygons.iter().any(|polygon| {
                        slicer_ir::point_in_polygon_winding(
                            polygon,
                            f64::from(x),
                            f64::from(y),
                            0.0,
                        )
                    })
                })
                .map(|band| band.quartile)
                .max()
        })
        .collect();
    let bridges = bridge_points()
        .into_iter()
        .map(|point| point_in_any_polygon(&point, &fixture.bridge_areas))
        .collect();
    let planar_paths = path_widths()
        .into_iter()
        .map(|width| {
            expolygon_to_path3d(
                &fixture.path_contour,
                0.2,
                width,
                &fixture.overhang_bands,
                &fixture.boundary,
            )
        })
        .collect();
    let nonplanar_paths = path_widths()
        .into_iter()
        .map(|width| expolygon_to_path3d(&fixture.path_contour, 0.2, width, &[], &fixture.boundary))
        .collect();

    Observed {
        distances,
        distance_ordinals: Vec::new(),
        quartiles,
        bridges,
        planar_paths,
        nonplanar_paths,
    }
}

fn collect_indexed(fixture: &SpatialFixture) -> Observed {
    let context = slicer_core::perimeter_spatial::PerimeterSpatialContext::new(
        &fixture.boundary,
        &fixture.overhang_bands,
        &fixture.bridge_areas,
    );
    let distances = query_points()
        .into_iter()
        .map(|(x, y)| context.signed_distance_to_boundary(x, y))
        .collect();
    let quartiles = query_points()
        .into_iter()
        .map(|(x, y)| context.overhang_quartile(x, y))
        .collect();
    let bridges = bridge_points()
        .into_iter()
        .map(|point| context.is_bridge(&point))
        .collect();
    let planar_paths = path_widths()
        .into_iter()
        .map(|width| {
            slicer_core::perimeter_spatial::expolygon_to_path3d_indexed(
                &fixture.path_contour,
                0.2,
                width,
                &context,
                slicer_core::perimeter_spatial::PathAnnotationMode::Planar,
            )
        })
        .collect();
    let nonplanar_paths = path_widths()
        .into_iter()
        .map(|width| {
            slicer_core::perimeter_spatial::expolygon_to_path3d_indexed(
                &fixture.path_contour,
                0.2,
                width,
                &context,
                slicer_core::perimeter_spatial::PathAnnotationMode::NonPlanarNoQuartile,
            )
        })
        .collect();

    Observed {
        distances,
        distance_ordinals: Vec::new(),
        quartiles,
        bridges,
        planar_paths,
        nonplanar_paths,
    }
}

fn collect_with_distance_selections(
    fixture: &SpatialFixture,
    accelerated: bool,
) -> (
    Observed,
    slicer_core::perimeter_spatial::diagnostics::QueryCounters,
    Vec<Option<usize>>,
) {
    slicer_core::perimeter_spatial::diagnostics::with_counters_and_distance_selections(
        accelerated,
        false,
        || collect_indexed(fixture),
    )
}

fn assert_equal_distance_tie_uses_first_hole_edge(
    fixture: &SpatialFixture,
    legacy_ordinals: &[Option<usize>],
) {
    let hole = &fixture.boundary[0].holes[0];
    assert!(hole.points.len() >= 3, "tie fixture hole needs two edges");
    let first = hole.points[0].to_mm();
    let second = hole.points[1].to_mm();
    let third = hole.points[2].to_mm();
    let first_edge_distance =
        signed_distance_to_boundary(0.0, 0.0, &[expolygon_mm(&[first, second], &[])]);
    let second_edge_distance =
        signed_distance_to_boundary(0.0, 0.0, &[expolygon_mm(&[second, third], &[])]);
    assert_eq!(
        first_edge_distance.to_bits(),
        second_edge_distance.to_bits(),
        "the first two hole edges must have equal distance bits"
    );
    let first_hole_ordinal = fixture.boundary[0].contour.points.len();
    assert_eq!(
        legacy_ordinals[1],
        Some(first_hole_ordinal),
        "equal-distance hole candidates select the first edge in source order"
    );
    assert_ne!(
        legacy_ordinals[1],
        Some(first_hole_ordinal + 1),
        "the tie assertion must reject selecting the later equal-distance edge"
    );
}

fn assert_option_f32_bits(expected: Option<f32>, actual: Option<f32>, label: &str) {
    match (expected, actual) {
        (Some(expected), Some(actual)) => {
            assert_eq!(
                expected.to_bits(),
                actual.to_bits(),
                "{label}: f32 bits differ"
            );
        }
        (None, None) => {}
        (expected, actual) => {
            panic!("{label}: Option presence differs: {expected:?} != {actual:?}");
        }
    }
}

fn assert_point_bits(expected: &Point3WithWidth, actual: &Point3WithWidth, label: &str) {
    // `expolygon_to_path3d` returns only `Point3WithWidth`; feature flags live
    // on the enclosing `WallLoop`, not on this path record. The field-by-field
    // bitwise comparison below therefore covers every annotation this helper
    // can produce without inventing a flag comparison at the wrong layer.
    assert_eq!(expected.x.to_bits(), actual.x.to_bits(), "{label}: x");
    assert_eq!(expected.y.to_bits(), actual.y.to_bits(), "{label}: y");
    assert_eq!(expected.z.to_bits(), actual.z.to_bits(), "{label}: z");
    assert_eq!(
        expected.width.to_bits(),
        actual.width.to_bits(),
        "{label}: width"
    );
    assert_eq!(
        expected.flow_factor.to_bits(),
        actual.flow_factor.to_bits(),
        "{label}: flow factor"
    );
    assert_eq!(
        expected.overhang_quartile, actual.overhang_quartile,
        "{label}: quartile"
    );
    assert_eq!(
        expected.dist_to_top_mm.to_bits(),
        actual.dist_to_top_mm.to_bits(),
        "{label}: distance to top"
    );
    assert_option_f32_bits(
        expected.overhang_distance_mm,
        actual.overhang_distance_mm,
        label,
    );
}

fn assert_paths_bits(
    expected: &[Vec<Point3WithWidth>],
    actual: &[Vec<Point3WithWidth>],
    label: &str,
) {
    assert_eq!(expected.len(), actual.len(), "{label}: width count");
    for (width_index, (expected_path, actual_path)) in expected.iter().zip(actual).enumerate() {
        assert!(!expected_path.is_empty(), "{label}: oracle path is empty");
        assert_eq!(
            expected_path.len(),
            actual_path.len(),
            "{label}[{width_index}]: path length"
        );
        for (point_index, (expected_point, actual_point)) in
            expected_path.iter().zip(actual_path).enumerate()
        {
            assert_point_bits(
                expected_point,
                actual_point,
                &format!("{label}[{width_index}][{point_index}]"),
            );
        }
        let expected_first = &expected_path[0];
        let actual_last = actual_path.last().expect("actual path length checked");
        assert_point_bits(expected_first, actual_last, "closure output");
    }
}

fn assert_observed_equal(expected: &Observed, actual: &Observed) {
    assert_eq!(expected.distances.len(), actual.distances.len());
    for (index, (&expected_distance, &actual_distance)) in
        expected.distances.iter().zip(&actual.distances).enumerate()
    {
        assert_eq!(
            expected_distance.to_bits(),
            actual_distance.to_bits(),
            "distance[{index}] bits"
        );
    }
    assert_eq!(
        expected.distance_ordinals, actual.distance_ordinals,
        "distance source ordinals"
    );
    assert_eq!(expected.quartiles, actual.quartiles, "quartile selections");
    assert_eq!(
        expected.bridges, actual.bridges,
        "bridge strict-boundary results"
    );
    assert_paths_bits(&expected.planar_paths, &actual.planar_paths, "planar");
    assert_paths_bits(
        &expected.nonplanar_paths,
        &actual.nonplanar_paths,
        "nonplanar",
    );
}

#[test]
fn exact_queries_match_legacy() {
    let fixture = adversarial_fixture();
    let mut expected = collect_legacy(&fixture);
    let (mut legacy, legacy_counters, legacy_ordinals) =
        collect_with_distance_selections(&fixture, false);
    expected.distance_ordinals = legacy_ordinals.clone();
    legacy.distance_ordinals = legacy_ordinals.clone();
    assert_observed_equal(&expected, &legacy);

    let (mut actual, accelerated_counters) =
        slicer_core::perimeter_spatial::diagnostics::with_counters(true, false, || {
            collect_indexed(&fixture)
        });
    let (_, _, indexed_ordinals) = collect_with_distance_selections(&fixture, true);
    actual.distance_ordinals = indexed_ordinals.clone();

    assert_observed_equal(&expected, &actual);
    assert_eq!(
        indexed_ordinals, legacy_ordinals,
        "indexed and legacy source ordinals must match for every distance query"
    );
    assert_equal_distance_tie_uses_first_hole_edge(&fixture, &legacy_ordinals);
    assert!(
        accelerated_counters.indexed_queries > 0,
        "accelerated parity fixture did not exercise an indexed query"
    );
    assert_eq!(
        accelerated_counters.legacy_queries, 0,
        "accelerated parity fixture unexpectedly used a legacy query"
    );
    assert_eq!(
        accelerated_counters.fallback_queries, 0,
        "accelerated parity fixture unexpectedly fell back"
    );
    assert!(
        legacy_counters.legacy_queries > 0,
        "legacy parity fixture did not exercise a legacy query"
    );
    assert_eq!(
        legacy_counters.indexed_queries, 0,
        "legacy parity fixture unexpectedly used an indexed query"
    );
    assert_eq!(
        legacy_counters.fallback_queries, 0,
        "pure legacy mode must not count fallbacks"
    );
    assert_eq!(
        actual.quartiles[1],
        Some(4),
        "overlapping bands use maximum quartile"
    );
    assert_eq!(actual.bridges[0], true, "strictly interior bridge point");
    assert_eq!(
        actual.bridges[1], false,
        "bridge edge is not strictly inside"
    );
    assert_eq!(actual.distances[0].to_bits(), (-0.0_f32).to_bits());
}

#[test]
fn accelerated_mode_exercised_not_vacuous() {
    let fixture = adversarial_fixture();
    let expected = collect_legacy(&fixture);
    let (accelerated, accelerated_counters) =
        slicer_core::perimeter_spatial::diagnostics::with_counters(true, false, || {
            collect_indexed(&fixture)
        });
    let (ordinary, ordinary_counters) =
        slicer_core::perimeter_spatial::diagnostics::with_counters(false, false, || {
            collect_indexed(&fixture)
        });

    assert_observed_equal(&expected, &accelerated);
    assert_observed_equal(&expected, &ordinary);
    assert_observed_equal(&accelerated, &ordinary);
    assert!(
        accelerated_counters.indexed_queries > 0,
        "accelerated mode did not exercise the indexed path"
    );
    assert_eq!(
        accelerated_counters.legacy_queries, 0,
        "accelerated parity fixture unexpectedly used legacy queries"
    );
    assert_eq!(
        accelerated_counters.fallback_queries, 0,
        "accelerated parity fixture unexpectedly used a fallback"
    );
    assert!(
        ordinary_counters.legacy_queries > 0,
        "ordinary mode did not exercise the legacy path"
    );
    assert_eq!(
        ordinary_counters.indexed_queries, 0,
        "ordinary mode unexpectedly used indexed queries"
    );
    assert_eq!(
        ordinary_counters.fallback_queries, 0,
        "pure legacy mode must not count fallbacks"
    );
}

#[test]
fn fallback_paths_are_nonvacuous() {
    let fixture = adversarial_fixture();

    let (nonfinite_distances, nonfinite_counters) =
        slicer_core::perimeter_spatial::diagnostics::with_counters(true, false, || {
            let context = slicer_core::perimeter_spatial::PerimeterSpatialContext::new(
                &fixture.boundary,
                &fixture.overhang_bands,
                &fixture.bridge_areas,
            );
            [
                context.signed_distance_to_boundary(f32::NAN, 0.0),
                context.signed_distance_to_boundary(f32::INFINITY, 0.0),
                context.signed_distance_to_boundary(f32::NEG_INFINITY, 0.0),
            ]
        });
    let expected_nonfinite = [
        signed_distance_to_boundary(f32::NAN, 0.0, &fixture.boundary),
        signed_distance_to_boundary(f32::INFINITY, 0.0, &fixture.boundary),
        signed_distance_to_boundary(f32::NEG_INFINITY, 0.0, &fixture.boundary),
    ];
    for (index, (actual, expected)) in nonfinite_distances
        .iter()
        .zip(expected_nonfinite)
        .enumerate()
    {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "nonfinite distance fallback[{index}]"
        );
    }
    assert_eq!(nonfinite_counters.fallback_queries, 3);
    assert_eq!(nonfinite_counters.legacy_queries, 3);
    assert_eq!(nonfinite_counters.indexed_queries, 0);

    let (empty_distance, empty_counters) =
        slicer_core::perimeter_spatial::diagnostics::with_counters(true, false, || {
            let context =
                slicer_core::perimeter_spatial::PerimeterSpatialContext::new(&[], &[], &[]);
            context.signed_distance_to_boundary(0.0, 0.0)
        });
    assert_eq!(
        empty_distance.to_bits(),
        0.0_f32.to_bits(),
        "empty boundary uses the empty legacy result"
    );
    assert_eq!(empty_counters.fallback_queries, 1);
    assert_eq!(empty_counters.legacy_queries, 1);
    assert_eq!(empty_counters.indexed_queries, 0);

    let edge_less_boundary = vec![expolygon_mm(&[], &[])];
    let edge_less_fixture = SpatialFixture {
        boundary: edge_less_boundary,
        overhang_bands: Vec::new(),
        bridge_areas: Vec::new(),
        path_contour: fixture.path_contour,
    };
    let (observed, counters) =
        slicer_core::perimeter_spatial::diagnostics::with_counters(true, false, || {
            let context = slicer_core::perimeter_spatial::PerimeterSpatialContext::new(
                &edge_less_fixture.boundary,
                &edge_less_fixture.overhang_bands,
                &edge_less_fixture.bridge_areas,
            );
            let finite_distance = context.signed_distance_to_boundary(0.0, 0.0);
            let nan_distance = context.signed_distance_to_boundary(f32::NAN, 0.0);
            let infinity_distance = context.signed_distance_to_boundary(f32::INFINITY, 0.0);
            let path = slicer_core::perimeter_spatial::expolygon_to_path3d_indexed(
                &edge_less_fixture.path_contour,
                0.2,
                0.4,
                &context,
                slicer_core::perimeter_spatial::PathAnnotationMode::Planar,
            );
            let bridge = context.is_bridge(&point_mm(0.0, 0.0));
            (
                finite_distance,
                nan_distance,
                infinity_distance,
                path,
                bridge,
            )
        });
    let expected_finite = signed_distance_to_boundary(0.0, 0.0, &edge_less_fixture.boundary);
    let expected_nan = signed_distance_to_boundary(f32::NAN, 0.0, &edge_less_fixture.boundary);
    let expected_infinity =
        signed_distance_to_boundary(f32::INFINITY, 0.0, &edge_less_fixture.boundary);
    assert_eq!(
        observed.0.to_bits(),
        expected_finite.to_bits(),
        "finite fallback"
    );
    assert_eq!(
        observed.1.to_bits(),
        expected_nan.to_bits(),
        "nonfinite fallback"
    );
    assert_eq!(
        observed.2.to_bits(),
        expected_infinity.to_bits(),
        "infinite fallback"
    );
    let expected_path = expolygon_to_path3d(
        &edge_less_fixture.path_contour,
        0.2,
        0.4,
        &[],
        &edge_less_fixture.boundary,
    );
    assert_paths_bits(&[expected_path], &[observed.3], "edge-less fallback path");
    assert!(!observed.4, "empty bridge set must remain a false result");
    assert!(
        counters.fallback_queries > 0,
        "fallback counter stayed zero"
    );
    assert!(
        counters.legacy_queries > 0,
        "fallback did not run legacy evaluation"
    );
    assert_eq!(
        counters.indexed_queries, 0,
        "structural-small fallback unexpectedly used indexed pruning"
    );

    let overflow_bridge_areas = vec![
        ExPolygon {
            contour: Polygon {
                points: vec![Point2 {
                    x: i64::MIN + 1,
                    y: 0,
                }],
            },
            holes: Vec::new(),
        },
        ExPolygon {
            contour: Polygon {
                points: vec![Point2 {
                    x: i64::MAX - 1,
                    y: 0,
                }],
            },
            holes: Vec::new(),
        },
        ExPolygon {
            contour: Polygon {
                points: vec![Point2 { x: -2, y: 0 }],
            },
            holes: Vec::new(),
        },
        ExPolygon {
            contour: Polygon {
                points: vec![Point2 { x: -1, y: 0 }],
            },
            holes: Vec::new(),
        },
        ExPolygon {
            contour: Polygon {
                points: vec![Point2 { x: 1, y: 0 }],
            },
            holes: Vec::new(),
        },
        ExPolygon {
            contour: Polygon {
                points: vec![Point2 { x: 2, y: 0 }],
            },
            holes: Vec::new(),
        },
        ExPolygon {
            contour: Polygon {
                points: vec![Point2 { x: 3, y: 0 }],
            },
            holes: Vec::new(),
        },
    ];
    let (overflow_result, overflow_counters) =
        slicer_core::perimeter_spatial::diagnostics::with_counters(true, false, || {
            let context = slicer_core::perimeter_spatial::PerimeterSpatialContext::new(
                &[],
                &[],
                &overflow_bridge_areas,
            );
            context.is_bridge(&Point2 { x: 0, y: 0 })
        });
    assert!(!overflow_result, "degenerate overflow polygons are outside");
    assert_eq!(overflow_counters.fallback_queries, 1);
    assert_eq!(overflow_counters.legacy_queries, 1);
    assert_eq!(overflow_counters.indexed_queries, 0);

    let (small_results, small_counters) =
        slicer_core::perimeter_spatial::diagnostics::with_counters(true, false, || {
            let context = slicer_core::perimeter_spatial::PerimeterSpatialContext::new(
                &[expolygon_mm(
                    &[(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)],
                    &[],
                )],
                &[],
                &[],
            );
            (
                context.signed_distance_to_boundary(0.0, 0.0),
                context.overhang_quartile(0.0, 0.0),
                context.is_bridge(&point_mm(0.0, 0.0)),
            )
        });
    let small_boundary = [expolygon_mm(
        &[(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)],
        &[],
    )];
    let expected_small_distance = signed_distance_to_boundary(0.0, 0.0, &small_boundary);
    assert_eq!(
        small_results.0.to_bits(),
        expected_small_distance.to_bits(),
        "structural-small distance fallback"
    );
    assert_eq!(small_results.1, None, "structural-small quartile fallback");
    assert!(!small_results.2, "structural-small bridge fallback");
    assert_eq!(small_counters.fallback_queries, 3);
    assert_eq!(small_counters.legacy_queries, 3);
    assert_eq!(small_counters.indexed_queries, 0);

    let (fault_results, fault_counters) =
        slicer_core::perimeter_spatial::diagnostics::with_counters(true, true, || {
            let context = slicer_core::perimeter_spatial::PerimeterSpatialContext::new(
                &fixture.boundary,
                &fixture.overhang_bands,
                &fixture.bridge_areas,
            );
            (
                context.signed_distance_to_boundary(0.0, 0.0),
                context.overhang_quartile(0.0, 0.0),
                context.is_bridge(&point_mm(0.0, 0.0)),
            )
        });
    let expected_fault_distance = signed_distance_to_boundary(0.0, 0.0, &fixture.boundary);
    let expected_fault_quartile = fixture
        .overhang_bands
        .iter()
        .filter(|band| {
            band.polygons
                .iter()
                .any(|polygon| slicer_ir::point_in_polygon_winding(polygon, 0.0, 0.0, 0.0))
        })
        .map(|band| band.quartile)
        .max();
    let expected_fault_bridge = point_in_any_polygon(&point_mm(0.0, 0.0), &fixture.bridge_areas);
    assert_eq!(
        fault_results.0.to_bits(),
        expected_fault_distance.to_bits(),
        "fault-injected distance fallback"
    );
    assert_eq!(
        fault_results.1, expected_fault_quartile,
        "fault-injected quartile fallback"
    );
    assert_eq!(
        fault_results.2, expected_fault_bridge,
        "fault-injected bridge fallback"
    );
    assert_eq!(fault_counters.fallback_queries, 3);
    assert_eq!(fault_counters.legacy_queries, 3);
    assert_eq!(fault_counters.indexed_queries, 0);
}
