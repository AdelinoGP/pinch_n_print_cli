#![allow(missing_docs)]
//! AC-5: keep_largest_contour_only retains only the ExPolygon with the
//! greatest contour area; on ties, the lower-indexed polygon is kept.

use slicer_core::polygon_ops::keep_largest_contour_only;
use slicer_ir::{ExPolygon, Point2, Polygon};

/// Build a square ExPolygon with side `side_mm` anchored at (0,0).
fn square_ex(side_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(side_mm, 0.0),
                Point2::from_mm(side_mm, side_mm),
                Point2::from_mm(0.0, side_mm),
            ],
        },
        holes: Vec::new(),
    }
}

/// Shoelace signed area in mm² (units are 100 nm = 10^-4 mm; area units^2 = 10^-8 mm^2).
fn contour_area_mm2(ep: &ExPolygon) -> f64 {
    let pts = &ep.contour.points;
    let mut area: i64 = 0;
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
    }
    // area is in unit² * 2; 1 unit = 1e-4 mm, so 1 unit² = 1e-8 mm²
    (area.abs() as f64) * 0.5 / (10_000.0_f64 * 10_000.0_f64)
}

#[test]
fn keep_largest_contour_only_retains_max_area_polygon() {
    // Areas: 4mm², 9mm², 1mm² → winner is index 1 (9mm²)
    let mut polys = vec![
        square_ex(2.0), // area = 4 mm²
        square_ex(3.0), // area = 9 mm²
        square_ex(1.0), // area = 1 mm²
    ];

    keep_largest_contour_only(&mut polys);

    assert_eq!(polys.len(), 1, "must retain exactly one polygon");
    let area = contour_area_mm2(&polys[0]);
    assert!(
        (area - 9.0).abs() < 0.01,
        "retained polygon area expected ≈9.0 mm², got {area}"
    );
}

#[test]
fn keep_largest_contour_only_tie_breaks_to_lower_index() {
    // Two equal-area 3mm² squares → lower index (0) must be retained.
    // We distinguish them by using different starting coordinates.
    let poly_a = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(3.0, 0.0),
                Point2::from_mm(3.0, 3.0),
                Point2::from_mm(0.0, 3.0),
            ],
        },
        holes: Vec::new(),
    };
    let poly_b = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(13.0, 10.0),
                Point2::from_mm(13.0, 13.0),
                Point2::from_mm(10.0, 13.0),
            ],
        },
        holes: Vec::new(),
    };

    // Verify equal areas before proceeding
    let area_a = contour_area_mm2(&poly_a);
    let area_b = contour_area_mm2(&poly_b);
    assert!(
        (area_a - area_b).abs() < 1e-9,
        "test precondition: both polygons must have equal area, got {area_a} vs {area_b}"
    );

    // Record the contour start point of poly_a (index 0) for identity check
    let first_pt_a = poly_a.contour.points[0];

    let mut polys = vec![poly_a, poly_b];
    keep_largest_contour_only(&mut polys);

    assert_eq!(polys.len(), 1, "must retain exactly one polygon");
    assert_eq!(
        polys[0].contour.points[0], first_pt_a,
        "on tie, lower-indexed polygon (index 0) must be kept"
    );
}

#[test]
fn keep_largest_contour_only_empty_input_is_noop() {
    let mut polys: Vec<ExPolygon> = Vec::new();
    keep_largest_contour_only(&mut polys);
    assert!(polys.is_empty());
}

#[test]
fn keep_largest_contour_only_single_polygon_is_retained() {
    let mut polys = vec![square_ex(5.0)];
    keep_largest_contour_only(&mut polys);
    assert_eq!(polys.len(), 1);
}
