#![allow(missing_docs)]

use slicer_core::polygon_ops::clip_polylines;
use slicer_core::{difference, intersection, offset, union, xor, OffsetJoinType};
use slicer_ir::{ExPolygon, Point2, Polygon};

fn square(min_x_mm: f32, min_y_mm: f32, max_x_mm: f32, max_y_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x_mm, min_y_mm),
                Point2::from_mm(max_x_mm, min_y_mm),
                Point2::from_mm(max_x_mm, max_y_mm),
                Point2::from_mm(min_x_mm, max_y_mm),
            ],
        },
        holes: Vec::new(),
    }
}

fn shape_signature(polys: &[ExPolygon]) -> Vec<Vec<(i64, i64)>> {
    polys
        .iter()
        .map(|poly| poly.contour.points.iter().map(|p| (p.x, p.y)).collect())
        .collect()
}

#[test]
fn boolean_ops_produce_expected_presence_for_overlapping_squares() {
    let a = square(0.0, 0.0, 10.0, 10.0);
    let b = square(5.0, 0.0, 15.0, 10.0);

    let union_result = union(&[a.clone()], &[b.clone()]);
    let intersection_result = intersection(&[a.clone()], &[b.clone()]);
    let difference_result = difference(&[a.clone()], &[b.clone()]);
    let xor_result = xor(&[a], &[b]);

    assert!(!union_result.is_empty(), "union should return geometry");
    assert!(
        !intersection_result.is_empty(),
        "intersection should return geometry"
    );
    assert!(
        !difference_result.is_empty(),
        "difference should return geometry"
    );
    assert!(!xor_result.is_empty(), "xor should return geometry");
}

#[test]
fn offset_outward_expands_bounds() {
    let input = square(0.0, 0.0, 10.0, 10.0);
    let expanded = offset(&[input], 0.4, OffsetJoinType::Miter, 0.0);

    assert!(!expanded.is_empty(), "offset should return geometry");

    let first = &expanded[0].contour.points;
    let min_x = first.iter().map(|p| p.x).min().unwrap_or(0);
    let min_y = first.iter().map(|p| p.y).min().unwrap_or(0);
    let max_x = first.iter().map(|p| p.x).max().unwrap_or(0);
    let max_y = first.iter().map(|p| p.y).max().unwrap_or(0);

    assert!(min_x < Point2::from_mm(0.0, 0.0).x);
    assert!(min_y < Point2::from_mm(0.0, 0.0).y);
    assert!(max_x > Point2::from_mm(10.0, 0.0).x);
    assert!(max_y > Point2::from_mm(0.0, 10.0).y);
}

#[test]
fn degenerate_polygon_is_ignored() {
    let degenerate = ExPolygon {
        contour: Polygon {
            points: vec![Point2::from_mm(1.0, 1.0), Point2::from_mm(1.0, 1.0)],
        },
        holes: Vec::new(),
    };
    let normal = square(0.0, 0.0, 1.0, 1.0);

    let result = union(&[degenerate], &[normal]);

    assert_eq!(result.len(), 1, "degenerate geometry should be ignored");
}

#[test]
fn offset_arc_tolerance_reduces_vertex_count() {
    // Square contour offset by +1.0 with Round join, comparing arc_tolerance 0.0 vs 0.5.
    // arc_tolerance 0.5 should produce strictly fewer vertices on the rounded corners.
    let sq = square(0.0, 0.0, 10.0, 10.0);
    let fine = offset(&[sq.clone()], 1.0, OffsetJoinType::Round, 0.0);
    let coarse = offset(&[sq], 1.0, OffsetJoinType::Round, 0.5);
    let fine_count: usize = fine.iter().map(|p| p.contour.points.len()).sum();
    let coarse_count: usize = coarse.iter().map(|p| p.contour.points.len()).sum();
    assert!(
        coarse_count < fine_count,
        "coarse={} not less than fine={}",
        coarse_count,
        fine_count
    );
}

// ---------------------------------------------------------------------------
// clip_polylines (packet 129)
// ---------------------------------------------------------------------------

/// Boundary-endpoint tolerance in integer units (1 unit = 100 nm).
const EDGE_TOL: i64 = 2;

fn polyline(points_mm: &[(f32, f32)]) -> Vec<Point2> {
    points_mm
        .iter()
        .map(|&(x, y)| Point2::from_mm(x, y))
        .collect()
}

fn square_with_hole(min_mm: f32, max_mm: f32, hole_min_mm: f32, hole_max_mm: f32) -> ExPolygon {
    let mut exp = square(min_mm, min_mm, max_mm, max_mm);
    // Hole wound CW (opposite of the CCW contour).
    exp.holes.push(Polygon {
        points: vec![
            Point2::from_mm(hole_min_mm, hole_min_mm),
            Point2::from_mm(hole_min_mm, hole_max_mm),
            Point2::from_mm(hole_max_mm, hole_max_mm),
            Point2::from_mm(hole_max_mm, hole_min_mm),
        ],
    });
    exp
}

fn near(p: Point2, q: Point2, tol: i64) -> bool {
    (p.x - q.x).abs() <= tol && (p.y - q.y).abs() <= tol
}

fn polyline_len(poly: &[Point2]) -> f64 {
    poly.windows(2)
        .map(|w| {
            let dx = (w[1].x - w[0].x) as f64;
            let dy = (w[1].y - w[0].y) as f64;
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

/// AC-1: polyline strictly inside → returned whole, unsplit, order preserved.
#[test]
fn clip_polylines_line_fully_inside_returned_whole() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(2.0, 2.0), (5.0, 3.0), (8.0, 8.0)]);

    let result = clip_polylines(&[input.clone()], &clip);

    assert_eq!(result.len(), 1, "expected exactly one polyline");
    assert_eq!(
        result[0], input,
        "inside polyline must be returned verbatim"
    );
}

/// AC-2: single boundary crossing → one polyline covering only the inside
/// portion; crossing endpoint on the boundary within ±2 units.
#[test]
fn clip_polylines_line_crossing_once_split() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(5.0, 5.0), (15.0, 5.0)]);

    let result = clip_polylines(&[input], &clip);

    assert_eq!(result.len(), 1, "expected exactly one polyline");
    let out = &result[0];
    assert!(out.len() >= 2);
    let inside_pt = Point2::from_mm(5.0, 5.0);
    let boundary_pt = Point2::from_mm(10.0, 5.0);
    let (first, last) = (out[0], *out.last().unwrap());
    let matches = (near(first, inside_pt, EDGE_TOL) && near(last, boundary_pt, EDGE_TOL))
        || (near(first, boundary_pt, EDGE_TOL) && near(last, inside_pt, EDGE_TOL));
    assert!(
        matches,
        "endpoints {:?}..{:?} must be the inside point and the boundary crossing",
        first, last
    );
    // Only the inside portion: nothing beyond x = 10 mm (+tol).
    for p in out {
        assert!(
            p.x <= boundary_pt.x + EDGE_TOL,
            "point {:?} outside clip",
            p
        );
    }
}

/// AC-3: enter-exit-enter → exactly 2 disjoint inside sub-polylines.
#[test]
fn clip_polylines_line_crossing_twice_two_segments() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    // Starts inside, exits through the top edge, re-enters: W shape.
    let input = polyline(&[(2.0, 5.0), (4.0, 15.0), (6.0, 5.0)]);

    let result = clip_polylines(&[input], &clip);

    assert_eq!(result.len(), 2, "expected two inside sub-polylines");
    let top = Point2::from_mm(0.0, 10.0).y;
    for poly in &result {
        assert!(poly.len() >= 2);
        for p in poly {
            assert!(p.y <= top + EDGE_TOL, "point {:?} above clip top", p);
        }
    }
}

/// AC-4: polyline straight through a hole → 2 sub-polylines, no returned
/// point strictly inside the hole.
#[test]
fn clip_polylines_line_through_hole_split_around_hole() {
    let clip = [square_with_hole(0.0, 10.0, 4.0, 6.0)];
    let input = polyline(&[(1.0, 5.0), (9.0, 5.0)]);

    let result = clip_polylines(&[input], &clip);

    assert_eq!(
        result.len(),
        2,
        "expected the polyline split around the hole"
    );
    let hole_min = Point2::from_mm(4.0, 4.0);
    let hole_max = Point2::from_mm(6.0, 6.0);
    for poly in &result {
        for p in poly {
            let strictly_inside_hole = p.x > hole_min.x + EDGE_TOL
                && p.x < hole_max.x - EDGE_TOL
                && p.y > hole_min.y + EDGE_TOL
                && p.y < hole_max.y - EDGE_TOL;
            assert!(!strictly_inside_hole, "point {:?} lies inside the hole", p);
        }
    }
}

/// AC-5: polyline collinear with a contour edge → on-edge span is inside.
#[test]
fn clip_polylines_line_along_edge_inside() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(2.0, 0.0), (8.0, 0.0)]);

    let result = clip_polylines(&[input], &clip);

    assert_eq!(result.len(), 1, "edge-coincident span must be returned");
    let out = &result[0];
    for p in out {
        assert!(
            p.y.abs() <= EDGE_TOL,
            "point {:?} not on the bottom edge",
            p
        );
    }
    let expected_len = (Point2::from_mm(8.0, 0.0).x - Point2::from_mm(2.0, 0.0).x) as f64;
    let len = polyline_len(out);
    assert!(
        (len - expected_len).abs() <= 2.0 * EDGE_TOL as f64,
        "edge span length {} != expected {}",
        len,
        expected_len
    );
}

/// AC-6: 3 inputs (inside, outside, crossing) → exactly 2 outputs.
#[test]
fn clip_polylines_multi_polyline_clip() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let inside = polyline(&[(2.0, 2.0), (8.0, 8.0)]);
    let outside = polyline(&[(20.0, 20.0), (30.0, 30.0)]);
    let crossing = polyline(&[(5.0, 5.0), (15.0, 5.0)]);

    let result = clip_polylines(&[inside.clone(), outside, crossing], &clip);

    assert_eq!(
        result.len(),
        2,
        "expected inside whole + crossing's inside part"
    );
    // Set membership, not index positions: one output is the inside polyline
    // verbatim (possibly reversed), the other ends at the boundary.
    let mut reversed_inside = inside.clone();
    reversed_inside.reverse();
    let whole_count = result
        .iter()
        .filter(|p| **p == inside || **p == reversed_inside)
        .count();
    assert_eq!(
        whole_count, 1,
        "exactly one output is the untouched inside polyline"
    );
    let boundary_pt = Point2::from_mm(10.0, 5.0);
    let clipped_count = result
        .iter()
        .filter(|p| {
            near(p[0], boundary_pt, EDGE_TOL) || near(*p.last().unwrap(), boundary_pt, EDGE_TOL)
        })
        .count();
    assert_eq!(
        clipped_count, 1,
        "exactly one output ends on the clip boundary"
    );
    // All returned points inside the clip.
    for poly in &result {
        for p in poly {
            assert!(
                p.x <= boundary_pt.x + EDGE_TOL,
                "point {:?} outside clip",
                p
            );
        }
    }
}

/// AC-N1: fully outside → dropped.
#[test]
fn clip_polylines_line_fully_outside_dropped() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(20.0, 20.0), (30.0, 25.0)]);

    let result = clip_polylines(&[input], &clip);

    assert!(result.is_empty(), "fully outside polyline must be dropped");
}

/// AC-N2: empty polylines / empty clip / both → empty Vec, no panic.
#[test]
fn clip_polylines_empty_input_returns_empty() {
    let clip = [square(0.0, 0.0, 10.0, 10.0)];
    let input = polyline(&[(2.0, 2.0), (8.0, 8.0)]);

    assert!(clip_polylines(&[], &clip).is_empty());
    assert!(clip_polylines(&[input.clone()], &[]).is_empty());
    assert!(clip_polylines(&[], &[]).is_empty());
}

#[test]
fn result_order_is_deterministic_for_same_input() {
    let a = square(0.0, 0.0, 4.0, 4.0);
    let b = square(3.0, 0.0, 7.0, 4.0);

    let first = union(&[a.clone()], &[b.clone()]);
    let second = union(&[a], &[b]);

    assert_eq!(shape_signature(&first), shape_signature(&second));
}
