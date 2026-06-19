// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Point.hpp, src/libslic3r/Geometry.hpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Typed 2-D geometry primitives and ray/closest-point algorithms.
//!
//! These are promoted from the `arachne-perimeters` module so that any guest or
//! host module can share the same well-typed API without copy-pasting math.

use slicer_ir::{ExPolygon, Point2};

/// A 2-D floating-point vector (direction or offset).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
}

/// A 2-D ray: a starting point and a unit-length direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// Ray origin in scaled integer units.
    pub origin: Point2,
    /// Ray direction (should be unit-length for meaningful `distance` results).
    pub direction: Vec2,
}

/// Result of a closest-point query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosestPoint {
    /// The closest point on the segment/polygon boundary.
    pub point: Point2,
    /// Squared distance from the query point to `point`, in scaled units².
    pub distance_sq: f64,
}

/// Result of a ray-intersection query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    /// The intersection point on the polygon boundary.
    pub point: Point2,
    /// Parametric distance along the ray (`t` in `origin + t * direction`).
    pub distance: f64,
}

// ── Internal ray type (mirrors the old private `Ray` in arachne-perimeters) ──

struct InternalRay {
    ox: f64,
    oy: f64,
    dx: f64,
    dy: f64,
}

fn ray_segment_intersect(ray: &InternalRay, ax: f64, ay: f64, bx: f64, by: f64) -> Option<f64> {
    let sx = bx - ax;
    let sy = by - ay;

    let denom = ray.dx * sy - ray.dy * sx;
    if denom.abs() < 1e-10 {
        return None; // Parallel
    }

    let t = ((ax - ray.ox) * sy - (ay - ray.oy) * sx) / denom;
    let u = ((ax - ray.ox) * ray.dy - (ay - ray.oy) * ray.dx) / denom;

    if t >= 0.0 && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns the squared distance from `p` to the closest point on segment `[a, b]`.
pub fn point_to_segment_distance_squared(p: Point2, a: Point2, b: Point2) -> f64 {
    let cp = closest_point_on_segment(p, a, b);
    cp.distance_sq
}

/// Returns the closest point on segment `[a, b]` to `p`, together with the
/// squared distance.
pub fn closest_point_on_segment(p: Point2, a: Point2, b: Point2) -> ClosestPoint {
    let dx = (b.x - a.x) as f64;
    let dy = (b.y - a.y) as f64;
    let len_sq = dx * dx + dy * dy;

    let (proj_x, proj_y) = if len_sq == 0.0 {
        (a.x as f64, a.y as f64)
    } else {
        let t = (((p.x - a.x) as f64 * dx + (p.y - a.y) as f64 * dy) / len_sq).clamp(0.0, 1.0);
        (a.x as f64 + t * dx, a.y as f64 + t * dy)
    };

    let dpx = p.x as f64 - proj_x;
    let dpy = p.y as f64 - proj_y;
    let distance_sq = dpx * dpx + dpy * dpy;

    ClosestPoint {
        point: Point2 {
            x: proj_x.round() as i64,
            y: proj_y.round() as i64,
        },
        distance_sq,
    }
}

/// Returns the closest point on any contour edge of `polygons` to `p`.
///
/// Returns `None` if `polygons` is empty or contains no edges.
pub fn closest_point_on_polygons(p: Point2, polygons: &[ExPolygon]) -> Option<ClosestPoint> {
    let mut best: Option<ClosestPoint> = None;

    for poly in polygons {
        let pts = &poly.contour.points;
        let n = pts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let cp = closest_point_on_segment(p, pts[i], pts[j]);
            if best.map_or(true, |b| cp.distance_sq < b.distance_sq) {
                best = Some(cp);
            }
        }
    }

    best
}

/// Cast `ray` against every contour edge in `polygons` and return the nearest
/// intersection beyond `t = 1.0` (to skip the boundary closest to the origin).
///
/// Returns `None` when no intersection is found (replaces the old `0.0` sentinel).
pub fn ray_to_polygons(ray: &Ray, polygons: &[ExPolygon]) -> Option<RayHit> {
    let mut min_t = f64::MAX;

    let internal = InternalRay {
        ox: ray.origin.x as f64,
        oy: ray.origin.y as f64,
        dx: ray.direction.x,
        dy: ray.direction.y,
    };

    for poly in polygons {
        let pts = &poly.contour.points;
        let n = pts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            if let Some(t) = ray_segment_intersect(
                &internal,
                pts[i].x as f64,
                pts[i].y as f64,
                pts[j].x as f64,
                pts[j].y as f64,
            ) {
                if t > 1.0 && t < min_t {
                    // t > 1.0 to skip the near boundary we came from
                    min_t = t;
                }
            }
        }
    }

    if min_t == f64::MAX {
        None
    } else {
        let hit_x = (internal.ox + min_t * internal.dx).round() as i64;
        let hit_y = (internal.oy + min_t * internal.dy).round() as i64;
        Some(RayHit {
            point: Point2 { x: hit_x, y: hit_y },
            distance: min_t,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::Point2;

    #[test]
    fn closest_point_on_segment_midpoint() {
        let a = Point2 { x: 0, y: 0 };
        let b = Point2 { x: 10_000, y: 0 };
        let p = Point2 { x: 5_000, y: 5_000 };
        let cp = closest_point_on_segment(p, a, b);
        assert!(
            (cp.distance_sq - 25_000_000.0_f64).abs() < 2.0,
            "distance_sq should be 5000^2 = 25_000_000"
        );
        assert!((cp.point.x - 5_000).abs() <= 1, "nearest X should be ~5000");
        assert!(cp.point.y.abs() <= 1, "nearest Y should be ~0");
    }

    #[test]
    fn closest_point_on_segment_degenerate() {
        let a = Point2 { x: 3_000, y: 4_000 };
        let b = Point2 { x: 3_000, y: 4_000 };
        let p = Point2 { x: 0, y: 0 };
        let cp = closest_point_on_segment(p, a, b);
        // distance = sqrt(3000^2 + 4000^2) = 5000; distance_sq = 25_000_000
        assert!((cp.distance_sq - 25_000_000.0_f64).abs() < 2.0);
    }

    #[test]
    fn point_to_segment_distance_squared_matches() {
        let a = Point2 { x: 0, y: 0 };
        let b = Point2 { x: 10_000, y: 0 };
        let p = Point2 { x: 5_000, y: 5_000 };
        let dsq = point_to_segment_distance_squared(p, a, b);
        assert!((dsq - 25_000_000.0_f64).abs() < 2.0);
    }

    #[test]
    fn closest_point_on_polygons_none_on_empty() {
        let result = closest_point_on_polygons(Point2 { x: 0, y: 0 }, &[]);
        assert!(result.is_none());
    }
}
