// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Geometry/VoronoiUtils.cpp
// (`VoronoiUtils::discretize_parabola`, VoronoiUtils.cpp:166-261)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Parabolic Voronoi edge discretization (T-203, Step 4 of the M2 Arachne
//! port, `docs/adr/0023-arachne-port-strategy.md`).
//!
//! A point-site/segment-site bisector edge in a segment Voronoi diagram is a
//! parabola: the locus of points equidistant from a focus point (the point
//! site) and a directrix line (the segment site's supporting line). This
//! module tessellates such an edge into a polyline for downstream consumers
//! (wall generation, P112's centrality/bead pass) that only handle straight
//! segments.
//!
//! Host-only: gated behind the `host-algos` feature, matching
//! [`crate::skeletal_trapezoidation::graph`].
//!
//! # Signature interpretation (deliberate scoping decision, packet 110)
//!
//! OrcaSlicer's real `discretize_parabola` takes six parameters: `focus`,
//! `start`, `end`, `line_a`, `line_b`, `transitioning_angle` — `start`/`end`
//! bound the arc along the parabola independently from the two points that
//! merely define the directrix line. This packet's contract
//! (`.ralph/specs/110_arachne-voronoi-skt-foundations/`) specifies a
//! 4-parameter signature with no separate arc-bound parameters. To resolve
//! that gap, `line_a`/`line_b` here serve **double duty**:
//!
//! 1. Together they define the directrix line (the infinite line through
//!    them) that the parabola is bisecting against `focus`.
//! 2. Their own positions, projected onto that directrix line, define the
//!    local-x (along-directrix) bounds of the arc to discretize — i.e.
//!    `start = line_a`, `end = line_b` in OrcaSlicer's terms, after
//!    projection.
//!
//! This is a legitimate simplification for a foundations-layer packet; P112's
//! real wire-up against actual VD edges may need a richer signature
//! extension (recovering independent `start`/`end` arc bounds) — that is out
//! of this packet's scope. `transitioning_angle` (OrcaSlicer's bead-transition
//! marking-point insertion) is likewise out of scope: this function performs
//! only the core geometric parabola tessellation.
//!
//! # Math
//!
//! Local frame: origin at `pxx`, the foot of the perpendicular from `focus`
//! onto the directrix line; local x-axis along the directrix direction
//! (unit vector from `line_a` toward `line_b`); local y-axis perpendicular
//! to the directrix, oriented so `focus` has positive local y. In this
//! frame, `focus` sits at local `(0, d)` where `d` is the perpendicular
//! distance from `focus` to the directrix, and the directrix is the line
//! local-y = 0.
//!
//! For a point at local `(x, y)`, `dist(point, focus) == dist(point,
//! directrix)` expands to `sqrt(x^2 + (y-d)^2) == |y|`, which solves to the
//! closed form used below:
//!
//! ```text
//! y = x^2 / (2*d) + d/2
//! ```
//!
//! (Verified directly from the equidistance definition above, not merely
//! transcribed.)

use slicer_ir::Point2;

/// Numerical tolerance (in `f64` scaled-integer units) below which a
/// direction vector or perpendicular distance is treated as zero.
const EPS: f64 = 1e-6;

/// Discretizes a parabolic Voronoi edge into a polyline.
///
/// `focus` is the point-site generator; `line_a`/`line_b` jointly define the
/// directrix line (the segment-site generator's supporting line) **and**
/// double as the arc's local-x bounds — see the module-level doc comment for
/// why this packet's 4-parameter signature resolves that way. `max_segment_len`
/// bounds the spacing between consecutive output points, measured along the
/// local-x (directrix-projection) axis, matching OrcaSlicer's actual
/// behavior (the spacing bound is along the directrix projection, not true
/// curve arc length).
///
/// Returns at least one point in every case; never panics or produces `NaN`.
///
/// # Degenerate cases
///
/// - `line_a == line_b` (or within [`EPS`]): the directrix line is
///   undefined, so no parabola can be constructed. Falls back to
///   `vec![line_a, line_b]`.
/// - `focus` lies on (or within [`EPS`] of) the directrix line: `d ≈ 0`
///   would require dividing by zero in the closed form. Geometrically this
///   is the limit where the parabola collapses onto the directrix line
///   itself, so this falls back to `vec![line_a, line_b]` rather than
///   dividing by ~0.
/// - `max_segment_len <= 0.0`: cannot be used to derive a positive step
///   count. Falls back to a single subdivision (the two arc endpoints only,
///   still computed on the true parabola — not `line_a`/`line_b`
///   themselves, which lie on the directrix, not the parabola).
pub fn discretize_parabolic_edge(
    focus: Point2,
    line_a: Point2,
    line_b: Point2,
    max_segment_len: f64,
) -> Vec<Point2> {
    let ax = line_a.x as f64;
    let ay = line_a.y as f64;
    let bx = line_b.x as f64;
    let by = line_b.y as f64;
    let fx = focus.x as f64;
    let fy = focus.y as f64;

    let dir_x = bx - ax;
    let dir_y = by - ay;
    let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();

    if dir_len < EPS {
        // line_a == line_b: no directrix direction, hence no parabola.
        return vec![line_a, line_b];
    }

    let dir_ux = dir_x / dir_len;
    let dir_uy = dir_y / dir_len;

    // pxx: foot of the perpendicular from `focus` onto the directrix line.
    let t = (fx - ax) * dir_ux + (fy - ay) * dir_uy;
    let pxx_x = ax + t * dir_ux;
    let pxx_y = ay + t * dir_uy;

    let to_focus_x = fx - pxx_x;
    let to_focus_y = fy - pxx_y;
    let d = (to_focus_x * to_focus_x + to_focus_y * to_focus_y).sqrt();

    if d < EPS {
        // focus lies on the directrix line: the parabola collapses to the
        // line itself. Return the two given endpoints rather than dividing
        // by ~0.
        return vec![line_a, line_b];
    }

    let perp_ux = to_focus_x / d;
    let perp_uy = to_focus_y / d;

    // Local-x projection helper: signed arclength along the directrix
    // direction from `pxx`.
    let local_x_of = |px: f64, py: f64| -> f64 { (px - pxx_x) * dir_ux + (py - pxx_y) * dir_uy };

    let local_x_a = local_x_of(ax, ay);
    let local_x_b = local_x_of(bx, by);

    let to_world = |local_x: f64| -> Point2 {
        let local_y = local_x * local_x / (2.0 * d) + d / 2.0;
        let world_x = pxx_x + local_x * dir_ux + local_y * perp_ux;
        let world_y = pxx_y + local_x * dir_uy + local_y * perp_uy;
        Point2 {
            x: world_x.round() as i64,
            y: world_y.round() as i64,
        }
    };

    let arc_span = local_x_b - local_x_a;

    if arc_span.abs() < EPS {
        // line_a and line_b project to the same local-x: zero-length arc.
        return vec![to_world(local_x_a)];
    }

    let step_count: usize = if max_segment_len <= 0.0 {
        1
    } else {
        let raw = (arc_span.abs() / max_segment_len).round() as i64;
        raw.max(1) as usize
    };

    let mut points = Vec::with_capacity(step_count + 1);
    for i in 0..=step_count {
        let frac = i as f64 / step_count as f64;
        let local_x = local_x_a + frac * arc_span;
        points.push(to_world(local_x));
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_a_equals_line_b_is_degenerate_no_panic() {
        let focus = Point2 { x: 5000, y: 5000 };
        let a = Point2 { x: 1000, y: 0 };
        let result = discretize_parabolic_edge(focus, a, a, 2000.0);
        assert_eq!(result, vec![a, a]);
    }

    #[test]
    fn focus_on_directrix_is_degenerate_no_panic() {
        let a = Point2 { x: 0, y: 0 };
        let b = Point2 { x: 10000, y: 0 };
        let focus = Point2 { x: 5000, y: 0 }; // exactly on the a-b line
        let result = discretize_parabolic_edge(focus, a, b, 2000.0);
        assert_eq!(result, vec![a, b]);
        for p in &result {
            assert!(p.x.abs() < i64::MAX && p.y.abs() < i64::MAX);
        }
    }

    #[test]
    fn non_positive_max_segment_len_falls_back_to_two_points() {
        let focus = Point2 { x: 5000, y: 5000 };
        let a = Point2 { x: 0, y: 0 };
        let b = Point2 { x: 10000, y: 0 };
        let result = discretize_parabolic_edge(focus, a, b, 0.0);
        assert_eq!(result.len(), 2);
        let result_neg = discretize_parabolic_edge(focus, a, b, -100.0);
        assert_eq!(result_neg.len(), 2);
    }

    #[test]
    fn realistic_case_produces_multiple_points() {
        let focus = Point2 { x: 5000, y: 5000 };
        let a = Point2 { x: 0, y: 0 };
        let b = Point2 { x: 10000, y: 0 };
        let result = discretize_parabolic_edge(focus, a, b, 2000.0);
        assert!(
            result.len() >= 4,
            "expected several points, got {}",
            result.len()
        );
    }
}
