//! 2D line-distance utility for the overhang classifier.
//!
//! [`LinesDistancer2D`] accepts a set of 2D line segments in mm-space (f32)
//! and answers:
//!
//! - [`LinesDistancer2D::nearest_distance`] — minimum unsigned distance from a
//!   query point to any segment.
//! - [`LinesDistancer2D::signed_distance`] — signed distance where **positive**
//!   means the point is inside at least one polygon (supported) and **negative**
//!   means outside (overhanging).
//!
//! ## Winding-number convention
//!
//! The inside test uses the standard winding-number algorithm.  A non-zero
//! winding number means "inside".  This handles both CCW and CW polygon
//! orientations: CCW gives winding +1 for interior points, CW gives -1.
//! Either non-zero value is treated as "inside" (supported).

const AABB_EPSILON: f32 = 1.0e-4; // mm — small guard for the bbox prefilter

/// Per-segment bounding box used to skip segments that cannot contribute to the
/// nearest distance for a given query point.
#[derive(Debug, Clone, Copy)]
struct SegmentAabb {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl SegmentAabb {
    fn from_segment(a: [f32; 2], b: [f32; 2]) -> Self {
        Self {
            min_x: a[0].min(b[0]) - AABB_EPSILON,
            min_y: a[1].min(b[1]) - AABB_EPSILON,
            max_x: a[0].max(b[0]) + AABB_EPSILON,
            max_y: a[1].max(b[1]) + AABB_EPSILON,
        }
    }

    /// Returns true when a circle of radius `r` centred at `p` could intersect
    /// the segment's AABB — i.e. the AABB expanded by `r` contains `p`.
    fn within_radius(&self, p: [f32; 2], r: f32) -> bool {
        p[0] >= self.min_x - r
            && p[0] <= self.max_x + r
            && p[1] >= self.min_y - r
            && p[1] <= self.max_y + r
    }
}

/// Acceleration structure for 2D line-segment distance queries.
///
/// Segments are stored together with per-segment AABBs for a fast bbox
/// prefilter.  All coordinates are in millimetres (f32).
#[derive(Debug, Clone)]
pub struct LinesDistancer2D {
    segments: Vec<([f32; 2], [f32; 2])>,
    aabbs: Vec<SegmentAabb>,
}

impl LinesDistancer2D {
    /// Builds a `LinesDistancer2D` from a slice of segments.
    ///
    /// Each segment is `(start, end)` where each point is `[x_mm, y_mm]`.
    pub fn new(segments: Vec<([f32; 2], [f32; 2])>) -> Self {
        let aabbs = segments
            .iter()
            .map(|&(a, b)| SegmentAabb::from_segment(a, b))
            .collect();
        Self { segments, aabbs }
    }

    /// Minimum unsigned distance from `p` to any stored segment.
    ///
    /// Returns `f32::INFINITY` when there are no segments.
    pub fn nearest_distance(&self, p: [f32; 2]) -> f32 {
        if self.segments.is_empty() {
            return f32::INFINITY;
        }

        let mut best = f32::INFINITY;

        for (i, &(a, b)) in self.segments.iter().enumerate() {
            // Coarse AABB prefilter: if the segment box expanded by `best` does
            // not reach `p`, skip.
            if !self.aabbs[i].within_radius(p, best) {
                continue;
            }

            let d = point_to_segment_distance(p, a, b);
            if d < best {
                best = d;
            }
        }

        best
    }

    /// Signed distance from `p` to the nearest segment.
    ///
    /// The sign is determined by a winding-number point-in-polygon test over
    /// all `polygons`:
    ///
    /// - **Positive** → `p` is inside at least one polygon (supported).
    /// - **Negative** → `p` is outside all polygons (overhanging).
    ///
    /// Both CCW and CW polygon winding orders are supported: any non-zero
    /// winding number is treated as "inside".
    pub fn signed_distance(&self, p: [f32; 2], polygons: &[Vec<[f32; 2]>]) -> f32 {
        let magnitude = self.nearest_distance(p);

        let inside = polygons.iter().any(|poly| winding_number(p, poly) != 0);

        if inside {
            magnitude
        } else {
            -magnitude
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Unsigned distance from point `p` to the line segment `(a, b)`.
fn point_to_segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let ap = [p[0] - a[0], p[1] - a[1]];

    let len_sq = ab[0] * ab[0] + ab[1] * ab[1];

    if len_sq <= f32::EPSILON * f32::EPSILON {
        // Degenerate segment — treat as point.
        return (ap[0] * ap[0] + ap[1] * ap[1]).sqrt();
    }

    // Project p onto the line, clamped to [0, 1].
    let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / len_sq).clamp(0.0, 1.0);

    let closest = [a[0] + t * ab[0], a[1] + t * ab[1]];
    let dx = p[0] - closest[0];
    let dy = p[1] - closest[1];
    (dx * dx + dy * dy).sqrt()
}

/// Winding number of point `p` with respect to polygon `poly`.
///
/// Uses the standard crossing-number accumulation.  Returns a non-zero value
/// for interior points regardless of whether the polygon is wound CW or CCW.
fn winding_number(p: [f32; 2], poly: &[[f32; 2]]) -> i32 {
    if poly.len() < 3 {
        return 0;
    }

    let n = poly.len();
    let mut wn = 0i32;

    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];

        if a[1] <= p[1] {
            if b[1] > p[1] {
                // Upward crossing.
                if cross_2d(sub2(b, a), sub2(p, a)) > 0.0 {
                    wn += 1;
                }
            }
        } else if b[1] <= p[1] {
            // Downward crossing.
            if cross_2d(sub2(b, a), sub2(p, a)) < 0.0 {
                wn -= 1;
            }
        }
    }

    wn
}

#[inline]
fn sub2(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

#[inline]
fn cross_2d(u: [f32; 2], v: [f32; 2]) -> f32 {
    u[0] * v[1] - u[1] * v[0]
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::LinesDistancer2D;

    // Helper: square polygon, CCW: (0,0)→(10,0)→(10,10)→(0,10)
    fn ccw_square() -> Vec<[f32; 2]> {
        vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
    }

    // Helper: same square wound CW: (0,0)→(0,10)→(10,10)→(10,0)
    fn cw_square() -> Vec<[f32; 2]> {
        vec![[0.0, 0.0], [0.0, 10.0], [10.0, 10.0], [10.0, 0.0]]
    }

    /// Square's boundary segments, used as the line set for signed_distance tests.
    fn square_segments() -> Vec<([f32; 2], [f32; 2])> {
        vec![
            ([0.0, 0.0], [10.0, 0.0]),
            ([10.0, 0.0], [10.0, 10.0]),
            ([10.0, 10.0], [0.0, 10.0]),
            ([0.0, 10.0], [0.0, 0.0]),
        ]
    }

    // 1. Empty distancer returns INFINITY for nearest_distance.
    #[test]
    fn zero_segments() {
        let d = LinesDistancer2D::new(vec![]);
        assert!(
            d.nearest_distance([0.0, 0.0]).is_infinite(),
            "expected INFINITY for empty segment set"
        );
    }

    // 2. Single horizontal segment; point directly above midpoint.
    #[test]
    fn single_segment_nearest() {
        let d = LinesDistancer2D::new(vec![([0.0, 0.0], [10.0, 0.0])]);
        let dist = d.nearest_distance([5.0, 5.0]);
        assert!((dist - 5.0).abs() < 1e-5, "expected 5.0, got {dist}");
    }

    // 3. Query point coincides with a segment endpoint → distance = 0.
    #[test]
    fn point_on_endpoint() {
        let d = LinesDistancer2D::new(vec![([0.0, 0.0], [10.0, 0.0])]);
        let dist = d.nearest_distance([0.0, 0.0]);
        assert!(dist < 1e-5, "expected 0.0, got {dist}");
    }

    // 4. CCW square, interior point → positive (supported).
    #[test]
    fn inside_square_positive_sign() {
        let d = LinesDistancer2D::new(square_segments());
        let sd = d.signed_distance([5.0, 5.0], &[ccw_square()]);
        assert!(
            sd > 0.0,
            "interior point of CCW square should be positive (supported), got {sd}"
        );
    }

    // 5. CCW square, exterior point → negative (overhanging).
    #[test]
    fn outside_square_negative_sign() {
        let d = LinesDistancer2D::new(square_segments());
        let sd = d.signed_distance([15.0, 5.0], &[ccw_square()]);
        assert!(
            sd < 0.0,
            "exterior point of CCW square should be negative (overhanging), got {sd}"
        );
    }

    // 6. CW polygon — winding_number returns non-zero for interior points
    //    regardless of orientation, so the sign convention is preserved.
    //    Both CCW and CW are accepted as "inside" (non-zero winding number).
    #[test]
    fn cw_polygon_sign_flipped() {
        let d = LinesDistancer2D::new(square_segments());

        // Interior point should still be "inside" for CW polygon.
        let sd_inside = d.signed_distance([5.0, 5.0], &[cw_square()]);
        assert!(
            sd_inside > 0.0,
            "interior point of CW square should be positive (supported), got {sd_inside}"
        );

        // Exterior point should still be "outside".
        let sd_outside = d.signed_distance([15.0, 5.0], &[cw_square()]);
        assert!(
            sd_outside < 0.0,
            "exterior point of CW square should be negative (overhanging), got {sd_outside}"
        );
    }
}
