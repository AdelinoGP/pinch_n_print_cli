//! TDD tests for the `rect_polygon` freestanding fixture helper.
//!
//! Mirrors `square_polygon` for the non-square ExPolygon shape needed by
//! rectilinear-infill's `make_narrow_rect`-style helpers (packet 78 gap).

use slicer_ir::mm_to_units;
use slicer_sdk::test_prelude::*;

#[test]
fn rect_polygon_has_four_corners() {
    let rect = rect_polygon(0.0, 0.0, 4.0, 6.0);
    assert_eq!(rect.contour.points.len(), 4);
}

#[test]
fn rect_polygon_x_range_uses_width() {
    let rect = rect_polygon(0.0, 0.0, 4.0, 6.0);
    let pts = &rect.contour.points;
    let min_x = pts.iter().map(|p| p.x).min().expect("non-empty");
    let max_x = pts.iter().map(|p| p.x).max().expect("non-empty");
    assert_eq!(min_x, mm_to_units(-2.0));
    assert_eq!(max_x, mm_to_units(2.0));
}

#[test]
fn rect_polygon_y_range_uses_height() {
    let rect = rect_polygon(0.0, 0.0, 4.0, 6.0);
    let pts = &rect.contour.points;
    let min_y = pts.iter().map(|p| p.y).min().expect("non-empty");
    let max_y = pts.iter().map(|p| p.y).max().expect("non-empty");
    assert_eq!(min_y, mm_to_units(-3.0));
    assert_eq!(max_y, mm_to_units(3.0));
}

#[test]
fn rect_polygon_is_ccw() {
    let rect = rect_polygon(0.0, 0.0, 4.0, 6.0);
    let pts = &rect.contour.points;
    // Shoelace signed area: > 0 means CCW. Use i128 to avoid overflow on i64
    // coordinates from the 100 nm unit system.
    let mut signed_area: i128 = 0;
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let xi = pts[i].x as i128;
        let yi = pts[i].y as i128;
        let xj = pts[j].x as i128;
        let yj = pts[j].y as i128;
        signed_area += xi * yj - xj * yi;
    }
    assert!(
        signed_area > 0,
        "expected CCW winding (signed area > 0), got {}",
        signed_area
    );
}

#[test]
fn rect_polygon_has_no_holes() {
    let rect = rect_polygon(0.0, 0.0, 4.0, 6.0);
    assert!(rect.holes.is_empty());
}

#[test]
fn rect_polygon_offset_center_translates_corners() {
    // Centered at (10, -5) with 4x6 dims: x-range [8,12], y-range [-8,-2].
    let rect = rect_polygon(10.0, -5.0, 4.0, 6.0);
    let pts = &rect.contour.points;
    let min_x = pts.iter().map(|p| p.x).min().expect("non-empty");
    let max_x = pts.iter().map(|p| p.x).max().expect("non-empty");
    let min_y = pts.iter().map(|p| p.y).min().expect("non-empty");
    let max_y = pts.iter().map(|p| p.y).max().expect("non-empty");
    assert_eq!(min_x, mm_to_units(8.0));
    assert_eq!(max_x, mm_to_units(12.0));
    assert_eq!(min_y, mm_to_units(-8.0));
    assert_eq!(max_y, mm_to_units(-2.0));
}
