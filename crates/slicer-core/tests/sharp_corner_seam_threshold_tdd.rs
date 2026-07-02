//! TDD test for packet 108 AC-4: sharp-corner angle-threshold seam candidacy.
//!
//! A square contour with 4 right-angle corners, tessellated with 30 redundant
//! collinear points per edge (124 points total), must produce exactly 4 seam
//! candidates at a 30.0 degree threshold — one per corner, not one per point.

use slicer_core::perimeter_utils::generate_sharp_corner_seam_candidates;
use slicer_ir::{mm_to_units, Point2, Polygon};

/// Build a square contour (side length `side_mm`, lower-left corner at origin)
/// with `n_extra` redundant collinear points inserted evenly along each edge,
/// in addition to the 4 corners.
fn square_with_redundant_points(side_mm: f32, n_extra: usize) -> (Polygon, [(f32, f32); 4]) {
    let corners = [
        (0.0_f32, 0.0_f32),
        (side_mm, 0.0),
        (side_mm, side_mm),
        (0.0, side_mm),
    ];

    let mut points = Vec::new();
    for i in 0..4 {
        let (x0, y0) = corners[i];
        let (x1, y1) = corners[(i + 1) % 4];
        points.push(Point2 {
            x: mm_to_units(x0),
            y: mm_to_units(y0),
        });
        for k in 1..=n_extra {
            let t = k as f32 / (n_extra as f32 + 1.0);
            let x = x0 + (x1 - x0) * t;
            let y = y0 + (y1 - y0) * t;
            points.push(Point2 {
                x: mm_to_units(x),
                y: mm_to_units(y),
            });
        }
    }

    (Polygon { points }, corners)
}

#[test]
fn square_with_redundant_points_yields_only_corner_candidates() {
    let (contour, corners) = square_with_redundant_points(10.0, 30);
    assert_eq!(
        contour.points.len(),
        4 * (1 + 30),
        "sanity: 124 total points"
    );

    let candidates = generate_sharp_corner_seam_candidates(&contour, 0.0, 30.0);

    assert_eq!(
        candidates.len(),
        4,
        "expected exactly 4 seam candidates (one per corner), got {}: positions={:?}",
        candidates.len(),
        candidates
            .iter()
            .map(|c| (c.position.x, c.position.y))
            .collect::<Vec<_>>()
    );

    const TOL_MM: f32 = 0.01;
    for (cx, cy) in corners {
        let found = candidates
            .iter()
            .any(|c| (c.position.x - cx).abs() <= TOL_MM && (c.position.y - cy).abs() <= TOL_MM);
        assert!(
            found,
            "expected a seam candidate near corner ({cx}, {cy}) within {TOL_MM} mm, candidates={:?}",
            candidates
                .iter()
                .map(|c| (c.position.x, c.position.y))
                .collect::<Vec<_>>()
        );
    }
}
