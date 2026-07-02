//! TDD test for packet 108 AC-4: sharp-corner angle-threshold seam candidacy.
//!
//! A square contour with 4 right-angle corners, tessellated with 30 redundant
//! collinear points per edge (124 points total), must produce exactly 4 seam
//! candidates at a 30.0 degree threshold — one per corner, not one per point.
//!
//! Also covers the sharpest-vertex fallback regression: a contour with no
//! vertex clearing the angle threshold (e.g. an MMU bisector-fragment
//! perimeter) must still yield exactly one candidate rather than an empty
//! result, which is fatal downstream in `com.core.seam-placer`.

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

/// Build a regular convex `n`-gon (radius `radius_mm`, centered at origin),
/// traversed CCW. Each vertex's turn angle equals the polygon's exterior
/// angle, `360.0 / n` degrees.
fn regular_ngon(radius_mm: f32, n: usize) -> (Polygon, Vec<(f32, f32)>) {
    let mut vertices = Vec::with_capacity(n);
    for i in 0..n {
        let theta = (i as f32) * std::f32::consts::TAU / (n as f32);
        vertices.push((radius_mm * theta.cos(), radius_mm * theta.sin()));
    }
    let points = vertices
        .iter()
        .map(|&(x, y)| Point2 {
            x: mm_to_units(x),
            y: mm_to_units(y),
        })
        .collect();
    (Polygon { points }, vertices)
}

/// Regression test: a contour whose maximum turn angle is below the 30°
/// threshold (a regular 24-gon approximating a circle has a uniform 15°
/// exterior/turn angle at every vertex) must not degrade to zero candidates.
/// Packet 108 originally gated candidacy on the threshold alone, so
/// low-curvature MMU bisector-fragment perimeters produced zero seam
/// candidates, fatally erroring `com.core.seam-placer`. The sharpest-vertex
/// fallback guarantees exactly one candidate, positioned at a polygon vertex.
#[test]
fn below_threshold_contour_falls_back_to_single_sharpest_vertex() {
    const N: usize = 24;
    let (contour, vertices) = regular_ngon(10.0, N);
    assert_eq!(contour.points.len(), N, "sanity: 24-gon has 24 points");

    // Sanity: exterior/turn angle of a regular 24-gon is 15°, below the 30°
    // threshold used elsewhere in this file (and by the perimeter-generation
    // module default).
    let exterior_angle_deg = 360.0 / (N as f32);
    assert!(
        exterior_angle_deg < 30.0,
        "sanity: 24-gon exterior angle {exterior_angle_deg} deg must be below the 30 deg threshold"
    );

    let candidates = generate_sharp_corner_seam_candidates(&contour, 0.0, 30.0);

    assert_eq!(
        candidates.len(),
        1,
        "expected exactly 1 fallback candidate for a below-threshold contour, got {}: positions={:?}",
        candidates.len(),
        candidates
            .iter()
            .map(|c| (c.position.x, c.position.y))
            .collect::<Vec<_>>()
    );

    const TOL_MM: f32 = 0.01;
    let candidate = &candidates[0];
    let at_a_vertex = vertices.iter().any(|&(vx, vy)| {
        (candidate.position.x - vx).abs() <= TOL_MM && (candidate.position.y - vy).abs() <= TOL_MM
    });
    assert!(
        at_a_vertex,
        "expected the fallback candidate at ({}, {}) to coincide with one of the polygon's vertices={:?}",
        candidate.position.x, candidate.position.y, vertices
    );
}
