//! TDD tests for assert_paths assertion helpers (TASK-049).

use slicer_ir::{ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth, Polygon};
use slicer_sdk::test_support::assert_paths::*;

// ── Helpers ──────────────────────────────────────────────────────────────

fn pt(x: f32, y: f32, z: f32, width: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

fn make_path(points: Vec<Point3WithWidth>) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points,
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

// ── assert_max_segment_length ────────────────────────────────────────────

#[test]
fn max_segment_length_passes_short_segments() {
    let path = make_path(vec![pt(0.0, 0.0, 0.0, 0.4), pt(1.0, 0.0, 0.0, 0.4)]);
    assert_max_segment_length(&[path], 2.0);
}

#[test]
fn max_segment_length_empty_paths() {
    assert_max_segment_length(&[], 1.0);
}

#[test]
fn max_segment_length_single_point_path() {
    let path = make_path(vec![pt(0.0, 0.0, 0.0, 0.4)]);
    assert_max_segment_length(&[path], 0.1);
}

#[test]
#[should_panic(expected = "segment")]
fn max_segment_length_fails_long_segment() {
    // segment length = sqrt(9+0+0) = 3.0 > 2.0
    let path = make_path(vec![pt(0.0, 0.0, 0.0, 0.4), pt(3.0, 0.0, 0.0, 0.4)]);
    assert_max_segment_length(&[path], 2.0);
}

#[test]
fn max_segment_length_uses_3d_distance() {
    // segment = sqrt(1+1+1) = 1.732
    let path = make_path(vec![pt(0.0, 0.0, 0.0, 0.4), pt(1.0, 1.0, 1.0, 0.4)]);
    assert_max_segment_length(&[path], 2.0);
}

#[test]
#[should_panic(expected = "segment")]
fn max_segment_length_3d_exceeds() {
    // segment = sqrt(1+1+1) = 1.732 > 1.5
    let path = make_path(vec![pt(0.0, 0.0, 0.0, 0.4), pt(1.0, 1.0, 1.0, 0.4)]);
    assert_max_segment_length(&[path], 1.5);
}

// ── assert_extrusion_width_range ─────────────────────────────────────────

#[test]
fn width_range_passes_in_range() {
    let path = make_path(vec![pt(0.0, 0.0, 0.0, 0.4), pt(1.0, 0.0, 0.0, 0.35)]);
    assert_extrusion_width_range(&[path], 0.3, 0.5);
}

#[test]
fn width_range_empty_paths() {
    assert_extrusion_width_range(&[], 0.3, 0.5);
}

#[test]
#[should_panic(expected = "width")]
fn width_range_fails_too_narrow() {
    let path = make_path(vec![pt(0.0, 0.0, 0.0, 0.2)]);
    assert_extrusion_width_range(&[path], 0.3, 0.5);
}

#[test]
#[should_panic(expected = "width")]
fn width_range_fails_too_wide() {
    let path = make_path(vec![pt(0.0, 0.0, 0.0, 0.6)]);
    assert_extrusion_width_range(&[path], 0.3, 0.5);
}

#[test]
fn width_range_exact_boundaries() {
    let path = make_path(vec![pt(0.0, 0.0, 0.0, 0.3), pt(1.0, 0.0, 0.0, 0.5)]);
    assert_extrusion_width_range(&[path], 0.3, 0.5);
}

// ── assert_paths_inside_polygon ──────────────────────────────────────────

#[test]
fn inside_polygon_passes_contained() {
    // 10mm x 10mm square at origin
    let polygon = Polygon {
        points: vec![
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(10.0, 0.0),
            Point2::from_mm(10.0, 10.0),
            Point2::from_mm(0.0, 10.0),
        ],
    };
    let path = make_path(vec![pt(5.0, 5.0, 0.0, 0.4), pt(6.0, 6.0, 0.0, 0.4)]);
    assert_paths_inside_polygon(&[path], &polygon);
}

#[test]
fn inside_polygon_empty_paths() {
    let polygon = Polygon {
        points: vec![
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(10.0, 0.0),
            Point2::from_mm(10.0, 10.0),
            Point2::from_mm(0.0, 10.0),
        ],
    };
    assert_paths_inside_polygon(&[], &polygon);
}

#[test]
#[should_panic(expected = "outside")]
fn inside_polygon_fails_outside() {
    let polygon = Polygon {
        points: vec![
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(10.0, 0.0),
            Point2::from_mm(10.0, 10.0),
            Point2::from_mm(0.0, 10.0),
        ],
    };
    let path = make_path(vec![pt(15.0, 15.0, 0.0, 0.4)]);
    assert_paths_inside_polygon(&[path], &polygon);
}

#[test]
fn inside_polygon_on_boundary() {
    // Points on boundary should be considered inside (inclusive)
    let polygon = Polygon {
        points: vec![
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(10.0, 0.0),
            Point2::from_mm(10.0, 10.0),
            Point2::from_mm(0.0, 10.0),
        ],
    };
    let path = make_path(vec![pt(0.0, 5.0, 0.0, 0.4), pt(10.0, 5.0, 0.0, 0.4)]);
    assert_paths_inside_polygon(&[path], &polygon);
}

// ── assert_no_path_intersections ─────────────────────────────────────────

#[test]
fn no_intersections_parallel_paths() {
    let path_a = make_path(vec![pt(0.0, 0.0, 0.0, 0.4), pt(10.0, 0.0, 0.0, 0.4)]);
    let path_b = make_path(vec![pt(0.0, 1.0, 0.0, 0.4), pt(10.0, 1.0, 0.0, 0.4)]);
    assert_no_path_intersections(&[path_a, path_b]);
}

#[test]
fn no_intersections_empty_paths() {
    assert_no_path_intersections(&[]);
}

#[test]
fn no_intersections_single_path() {
    let path = make_path(vec![
        pt(0.0, 0.0, 0.0, 0.4),
        pt(5.0, 5.0, 0.0, 0.4),
        pt(10.0, 0.0, 0.0, 0.4),
    ]);
    assert_no_path_intersections(&[path]);
}

#[test]
#[should_panic(expected = "intersect")]
fn no_intersections_fails_crossing() {
    // X-shaped intersection
    let path_a = make_path(vec![pt(0.0, 0.0, 0.0, 0.4), pt(10.0, 10.0, 0.0, 0.4)]);
    let path_b = make_path(vec![pt(0.0, 10.0, 0.0, 0.4), pt(10.0, 0.0, 0.0, 0.4)]);
    assert_no_path_intersections(&[path_a, path_b]);
}

#[test]
fn no_intersections_non_crossing_l_shape() {
    // Two paths that don't cross
    let path_a = make_path(vec![pt(0.0, 0.0, 0.0, 0.4), pt(5.0, 0.0, 0.0, 0.4)]);
    let path_b = make_path(vec![pt(5.0, 0.0, 0.0, 0.4), pt(5.0, 5.0, 0.0, 0.4)]);
    // Shared endpoint but no crossing
    assert_no_path_intersections(&[path_a, path_b]);
}
