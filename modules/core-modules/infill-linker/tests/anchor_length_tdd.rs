#![allow(missing_docs)]

use std::collections::HashMap;

use infill_linker::connect::{chain_or_connect_infill, connect_infill, AnchorParams};
use infill_linker::graph::BoundaryInfillGraph;
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth,
    Polygon,
};

fn square(size_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(size_mm, 0.0),
                Point2::from_mm(size_mm, size_mm),
                Point2::from_mm(0.0, size_mm),
            ],
        },
        holes: vec![],
    }
}

fn point(x_mm: f32, y_mm: f32) -> Point3WithWidth {
    Point3WithWidth {
        x: x_mm,
        y: y_mm,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
        overhang_distance_mm: None,
    }
}

fn segment_with_speed(start: (f32, f32), end: (f32, f32), speed_factor: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![point(start.0, start.1), point(end.0, end.1)],
        role: ExtrusionRole::SparseInfill,
        speed_factor,
    }
}

fn segment(start: (f32, f32), end: (f32, f32)) -> ExtrusionPath3D {
    segment_with_speed(start, end, 1.0)
}

fn anchors(anchor_length_mm: f32, anchor_length_max_mm: f32) -> AnchorParams {
    AnchorParams {
        anchor_length_mm,
        anchor_length_max_mm,
    }
}

fn has_vertex(path: &ExtrusionPath3D, x_mm: f32, y_mm: f32) -> bool {
    path.points
        .iter()
        .any(|point| (point.x - x_mm).abs() < 1e-3 && (point.y - y_mm).abs() < 1e-3)
}

fn same_xy(point: &Point3WithWidth, expected: (f32, f32)) -> bool {
    (point.x - expected.0).abs() < 1e-3 && (point.y - expected.1).abs() < 1e-3
}

fn has_non_input_boundary_point(path: &ExtrusionPath3D, input_endpoints: &[(f32, f32)]) -> bool {
    path.points.iter().any(|point| {
        let on_boundary = point.x.abs() < 1e-3
            || (point.x - 10.0).abs() < 1e-3
            || point.y.abs() < 1e-3
            || (point.y - 10.0).abs() < 1e-3;
        on_boundary
            && !input_endpoints
                .iter()
                .any(|expected| same_xy(point, *expected))
    })
}

fn distance(a: &Point3WithWidth, b: &Point3WithWidth) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn contour_runs(path: &ExtrusionPath3D, input_endpoints: &[(f32, f32)]) -> Vec<(f32, (f32, f32))> {
    let is_input = |point: &Point3WithWidth| {
        input_endpoints
            .iter()
            .any(|expected| same_xy(point, *expected))
    };
    let mut runs = Vec::new();

    for (start_index, point) in path.points.iter().enumerate() {
        if !is_input(point) {
            continue;
        }
        for direction in [-1_i32, 1_i32] {
            let next_index = start_index as i32 + direction;
            if next_index < 0
                || next_index >= path.points.len() as i32
                || is_input(&path.points[next_index as usize])
            {
                continue;
            }

            let mut index = next_index;
            let mut previous = point;
            let mut length = 0.0;
            let mut terminal = (previous.x, previous.y);
            while index >= 0
                && index < path.points.len() as i32
                && !is_input(&path.points[index as usize])
            {
                let current = &path.points[index as usize];
                length += distance(previous, current);
                terminal = (current.x, current.y);
                previous = current;
                index += direction;
            }
            if length > 1e-3 {
                runs.push((length, terminal));
            }
        }
    }

    runs
}

fn wide_pair() -> Vec<ExtrusionPath3D> {
    vec![
        segment_with_speed((1.0, 0.0), (1.0, 10.0), 0.7),
        segment_with_speed((9.0, 0.0), (9.0, 10.0), 0.7),
    ]
}

fn wide_pair_endpoints() -> Vec<(f32, f32)> {
    vec![(1.0, 0.0), (1.0, 10.0), (9.0, 0.0), (9.0, 10.0)]
}

#[test]
fn whole_arc_under_anchor_length_max_merges_into_one_polyline() {
    let graph = BoundaryInfillGraph::new(&[square(10.0)]);
    let output = connect_infill(
        vec![
            segment_with_speed((9.5, 0.0), (9.5, 10.0), 0.7),
            segment_with_speed((10.0, 0.5), (0.0, 0.5), 0.7),
        ],
        &graph,
        anchors(2.0, 5.0),
    );

    assert_eq!(output.len(), 1);
    assert!((output[0].speed_factor - 0.7).abs() < f32::EPSILON);
    assert!(has_vertex(&output[0], 9.5, 0.0));
    assert!(has_vertex(&output[0], 10.0, 0.0));
    assert!(has_vertex(&output[0], 10.0, 0.5));

    // connect_infill flattens active slots in index order. Incompatible paths
    // at slots 0 and 2 make the lower-slot survivor observable between them.
    let slot_output = connect_infill(
        vec![
            segment_with_speed((0.0, 0.5), (0.0, 9.5), 0.5),
            segment_with_speed((9.5, 0.0), (9.5, 10.0), 0.7),
            segment_with_speed((0.5, 10.0), (9.5, 10.0), 0.9),
            segment_with_speed((10.0, 0.5), (0.0, 0.5), 0.7),
        ],
        &graph,
        anchors(2.0, 5.0),
    );
    assert_eq!(slot_output.len(), 3);
    assert!((slot_output[1].speed_factor - 0.7).abs() < f32::EPSILON);
    assert!(has_vertex(&slot_output[1], 9.5, 0.0));
    assert!((slot_output[2].speed_factor - 0.9).abs() < f32::EPSILON);
}

#[test]
fn arc_over_anchor_length_max_leaves_two_polylines_each_with_a_stub() {
    let graph = BoundaryInfillGraph::new(&[square(10.0)]);
    let output = connect_infill(wide_pair(), &graph, anchors(2.0, 5.0));
    let endpoints = wide_pair_endpoints();

    assert_eq!(output.len(), 2);
    assert!(output.iter().all(|path| path.points.len() > 2));
    assert!(output
        .iter()
        .all(|path| has_non_input_boundary_point(path, &endpoints)));
}

#[test]
fn stub_is_exactly_anchor_length_via_a_lerped_partial_segment() {
    let graph = BoundaryInfillGraph::new(&[square(10.0)]);
    let output = connect_infill(wide_pair(), &graph, anchors(2.0, 5.0));
    let endpoints = wide_pair_endpoints();

    assert_eq!(output.len(), 2);
    for path in &output {
        let runs = contour_runs(path, &endpoints);
        assert!(
            !runs.is_empty(),
            "each over-max path must have a contour run"
        );
        for (length, terminal) in runs {
            assert!((length - 2.0).abs() < 1e-3, "stub length was {length} mm");
            assert!(![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
                .iter()
                .any(|vertex| (terminal.0 - vertex.0).abs() < 1e-3
                    && (terminal.1 - vertex.1).abs() < 1e-3));
        }
    }
}

#[test]
fn shorter_arc_claims_its_endpoints_before_a_longer_arc() {
    let graph = BoundaryInfillGraph::new(&[square(10.0)]);
    let output = connect_infill(
        vec![
            segment_with_speed((1.0, 0.0), (5.0, 5.0), 0.1),
            segment_with_speed((9.0, 0.0), (5.0, 6.0), 0.2),
            segment_with_speed((10.0, 1.0), (6.0, 5.0), 0.2),
        ],
        &graph,
        anchors(1.0, 9.0),
    );

    assert_eq!(output.len(), 2);
    let merged = output
        .iter()
        .find(|path| (path.speed_factor - 0.2).abs() < f32::EPSILON)
        .expect("B must survive the shorter B-C merge");
    assert!(has_vertex(merged, 10.0, 0.0));
    assert!(has_vertex(merged, 9.0, 0.0));
    assert!(has_vertex(merged, 10.0, 1.0));
    assert!(output
        .iter()
        .any(|path| (path.speed_factor - 0.1).abs() < f32::EPSILON));
}

#[test]
fn percent_anchor_resolves_against_flow_spacing_via_get_abs_value() {
    let percent_view = ConfigView::from_map(HashMap::from([(
        "infill_anchor".to_string(),
        ConfigValue::FloatOrPercent {
            value: 400.0,
            is_percent: true,
        },
    )]));
    let params = AnchorParams::from_config(Some(&percent_view), 0.3570796);
    assert!((params.anchor_length_mm - 1.4283185).abs() < 1e-6);

    let absolute_view = ConfigView::from_map(HashMap::from([(
        "infill_anchor".to_string(),
        ConfigValue::Float(8.0),
    )]));
    let absolute = AnchorParams::from_config(Some(&absolute_view), 0.3570796);
    let absolute_with_other_base = AnchorParams::from_config(Some(&absolute_view), 2.0);
    assert!((absolute.anchor_length_mm - 8.0).abs() < 1e-6);
    assert!((absolute_with_other_base.anchor_length_mm - 8.0).abs() < 1e-6);
}

#[test]
fn zero_anchor_max_dispatches_to_chain_only_never_connect() {
    let graph = BoundaryInfillGraph::new(&[square(10.0)]);
    let output = chain_or_connect_infill(
        vec![
            segment((8.0, 5.0), (10.0, 5.0)),
            segment((0.0, 5.0), (2.0, 5.0)),
            segment((4.0, 5.0), (6.0, 5.0)),
        ],
        &graph,
        anchors(2.0, 0.0),
    );

    assert_eq!(output.len(), 3);
    let travel: f32 = output
        .windows(2)
        .map(|paths| distance(paths[0].points.last().unwrap(), &paths[1].points[0]))
        .sum();
    assert!(
        travel < 5.0,
        "chain-only dispatch must still optimize travel"
    );
}

#[test]
fn zero_anchor_length_leaves_the_over_max_arc_with_no_stub_at_all() {
    let graph = BoundaryInfillGraph::new(&[square(10.0)]);
    let output = connect_infill(wide_pair(), &graph, anchors(0.0, 5.0));

    assert_eq!(output.len(), 2);
    assert!(output.iter().all(|path| path.points.len() == 2));
}

#[test]
fn stub_is_clamped_at_the_next_boundary_position_and_never_walks_over_it() {
    let graph = BoundaryInfillGraph::new(&[square(10.0)]);
    let output = connect_infill(
        vec![
            segment_with_speed((9.0, 0.0), (5.0, 5.0), 0.1),
            segment_with_speed((1.0, 0.0), (5.0, 6.0), 0.2),
            segment_with_speed((3.0, 0.0), (6.0, 6.0), 0.2),
        ],
        &graph,
        anchors(4.0, 1.0),
    );

    assert_eq!(output.len(), 3);
    let b = output
        .iter()
        .find(|path| has_vertex(path, 1.0, 0.0))
        .expect("B must remain identifiable after the over-max attempt");
    assert!(has_vertex(b, 3.0, 0.0));
    assert!(b
        .points
        .iter()
        .all(|point| !(point.y.abs() < 1e-3 && point.x > 3.0 + 1e-3)));
}
