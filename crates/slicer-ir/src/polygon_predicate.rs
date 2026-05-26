//! Polygon predicates shared across module crates.
//!
//! Lives in `slicer-ir` (rather than `slicer-helpers`) so WASM module
//! crates can depend on it without dragging `slicer-helpers`'s heavy
//! native-only dependencies (ruststep, etc.) into a wasm32 build.

use crate::slice_ir::{ExPolygon, Point2, Polygon};

/// Convert a `Point2` (integer units, 1 unit = 100 nm) to floating-point
/// millimetres. Mirrors the helper that lived in
/// `top-surface-ironing/src/lib.rs` so callers don't have to thread the
/// scale factor by hand.
#[inline]
fn units_to_mm(units: i64) -> f64 {
    units as f64 / 10_000.0
}

/// Winding-number point-in-polygon test with millimetre boundary tolerance.
///
/// Returns `true` when the point `(px_mm, py_mm)` lies inside (winding
/// non-zero) OR within `eps_mm` of any contour edge. Stronger than the
/// classic even-odd test for two reasons:
///
/// 1. **Boundary tolerance.** Points within `eps_mm` of an edge count as
///    inside. Even-odd's strict inequality flips on the wrong side of
///    float epsilon when a stroke endpoint lands exactly on a polygon
///    edge — the case that the top-surface-ironing clipper hits whenever
///    a zigzag stroke's terminator coincides with the polygon contour.
///
/// 2. **f64 arithmetic.** All cross products and projections are in `f64`
///    so polygons with sub-0.1 mm features survive the test correctly.
///
/// Holes are NOT subtracted — callers union or difference at the slice
/// level before invoking. This mirrors the ironing helper's contour-only
/// semantics: an `ExPolygon` with holes treats the OUTER contour as a
/// filled region and ignores holes during point classification.
///
/// `eps_mm` is a small positive tolerance in millimetres. Pass `0.0` for
/// strict containment (winding-number only); pass `0.001` (one slicer
/// unit) for the documented "edge-points are inside" semantics.
pub fn point_in_polygon_winding(poly: &ExPolygon, px_mm: f64, py_mm: f64, eps_mm: f64) -> bool {
    point_in_contour_winding(&poly.contour, px_mm, py_mm, eps_mm)
}

/// Same predicate against a raw `Polygon` (single closed contour). Useful
/// when the caller already extracted a ring from an `ExPolygon`.
pub fn point_in_contour_winding(ring: &Polygon, px_mm: f64, py_mm: f64, eps_mm: f64) -> bool {
    let pts = &ring.points;
    let n = pts.len();
    if n < 3 {
        return false;
    }

    // First pass: boundary check. Any edge within eps_mm of the query
    // point qualifies as "inside" via tolerance.
    let eps2 = eps_mm * eps_mm;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        if dist2_point_to_segment_mm(a, b, px_mm, py_mm) <= eps2 {
            return true;
        }
    }

    // Second pass: winding number. Each edge contributes +1 if it
    // crosses the +X horizontal ray upward, -1 if it crosses downward.
    // Non-zero winding => inside.
    let mut wn: i32 = 0;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let ay = units_to_mm(a.y);
        let by = units_to_mm(b.y);
        if ay <= py_mm {
            if by > py_mm && is_left(a, b, px_mm, py_mm) > 0.0 {
                wn += 1;
            }
        } else if by <= py_mm && is_left(a, b, px_mm, py_mm) < 0.0 {
            wn -= 1;
        }
    }
    wn != 0
}

/// 2× the signed triangle area (a, b, p). Positive when p is left of a→b.
#[inline]
fn is_left(a: Point2, b: Point2, px_mm: f64, py_mm: f64) -> f64 {
    let ax = units_to_mm(a.x);
    let ay = units_to_mm(a.y);
    let bx = units_to_mm(b.x);
    let by = units_to_mm(b.y);
    (bx - ax) * (py_mm - ay) - (px_mm - ax) * (by - ay)
}

/// Squared distance from `(px_mm, py_mm)` to the closed segment ab. f64.
fn dist2_point_to_segment_mm(a: Point2, b: Point2, px_mm: f64, py_mm: f64) -> f64 {
    let ax = units_to_mm(a.x);
    let ay = units_to_mm(a.y);
    let bx = units_to_mm(b.x);
    let by = units_to_mm(b.y);
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        // Degenerate edge: distance to vertex.
        let ex = px_mm - ax;
        let ey = py_mm - ay;
        return ex * ex + ey * ey;
    }
    // Project p onto ab, clamp to [0, 1].
    let t = (((px_mm - ax) * dx + (py_mm - ay) * dy) / len2).clamp(0.0, 1.0);
    let cx = ax + t * dx;
    let cy = ay + t * dy;
    let ex = px_mm - cx;
    let ey = py_mm - cy;
    ex * ex + ey * ey
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(side_mm: f64) -> ExPolygon {
        let half = (side_mm / 2.0 * 10_000.0) as i64;
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: -half, y: -half },
                    Point2 { x: half, y: -half },
                    Point2 { x: half, y: half },
                    Point2 { x: -half, y: half },
                ],
            },
            holes: vec![],
        }
    }

    #[test]
    fn interior_point_is_inside() {
        let poly = square(10.0);
        assert!(point_in_polygon_winding(&poly, 0.0, 0.0, 0.0));
        assert!(point_in_polygon_winding(&poly, 4.99, 4.99, 0.0));
    }

    #[test]
    fn far_exterior_point_is_outside() {
        let poly = square(10.0);
        assert!(!point_in_polygon_winding(&poly, 100.0, 0.0, 0.0));
        assert!(!point_in_polygon_winding(&poly, 5.001, 0.0, 0.0));
    }

    #[test]
    fn exact_edge_point_is_inside_with_default_eps() {
        let poly = square(10.0);
        // Exact +X edge midpoint
        assert!(
            point_in_polygon_winding(&poly, 5.0, 0.0, 0.001),
            "edge-midpoint must be inside with eps=0.001"
        );
        // Vertex
        assert!(
            point_in_polygon_winding(&poly, 5.0, 5.0, 0.001),
            "vertex must be inside with eps=0.001"
        );
    }

    #[test]
    fn near_edge_point_within_eps_is_inside() {
        let poly = square(10.0);
        // 0.0005 mm outside +X edge — within 0.001 eps tolerance.
        assert!(point_in_polygon_winding(&poly, 5.0005, 0.0, 0.001));
        // 0.002 mm outside — beyond 0.001 eps tolerance.
        assert!(!point_in_polygon_winding(&poly, 5.002, 0.0, 0.001));
    }

    #[test]
    fn degenerate_polygon_returns_false() {
        let too_few = ExPolygon {
            contour: Polygon {
                points: vec![Point2 { x: 0, y: 0 }, Point2 { x: 1, y: 0 }],
            },
            holes: vec![],
        };
        assert!(!point_in_polygon_winding(&too_few, 0.0, 0.0, 0.001));
    }

    #[test]
    fn winding_handles_concave_polygon() {
        // U-shape: outer 20×20 with a 8×16 notch cut from the +Y side.
        // Test point in the notch should be OUTSIDE despite being inside
        // the bounding box.
        let u = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: -100_000, y: -100_000 }, // -10, -10
                    Point2 { x: 100_000, y: -100_000 },  //  10, -10
                    Point2 { x: 100_000, y: 100_000 },   //  10,  10
                    Point2 { x: 40_000, y: 100_000 },    //   4,  10
                    Point2 { x: 40_000, y: 20_000 },     //   4,   2
                    Point2 { x: -40_000, y: 20_000 },    //  -4,   2
                    Point2 { x: -40_000, y: 100_000 },   //  -4,  10
                    Point2 { x: -100_000, y: 100_000 },  // -10,  10
                ],
            },
            holes: vec![],
        };
        // Inside U (lower section)
        assert!(point_in_polygon_winding(&u, 0.0, -5.0, 0.001));
        // Inside the notch — OUTSIDE the U
        assert!(!point_in_polygon_winding(&u, 0.0, 5.0, 0.001));
        // Inside the arm
        assert!(point_in_polygon_winding(&u, 7.0, 5.0, 0.001));
    }
}
