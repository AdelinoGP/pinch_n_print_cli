#![allow(missing_docs)]

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
    let expanded = offset(&[input], 0.4, OffsetJoinType::Miter);

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
fn result_order_is_deterministic_for_same_input() {
    let a = square(0.0, 0.0, 4.0, 4.0);
    let b = square(3.0, 0.0, 7.0, 4.0);

    let first = union(&[a.clone()], &[b.clone()]);
    let second = union(&[a], &[b]);

    assert_eq!(shape_signature(&first), shape_signature(&second));
}
