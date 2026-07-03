//! Golden/invariant tests for `discretize_parabolic_edge` (T-203, Step 4 of
//! the M2 Arachne port, packet 110).
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this whole file
//! is a no-op under default features — it must not break
//! `cargo check --workspace --all-targets` without `--features host-algos`.
//!
//! # On the "OrcaSlicer reference" in `parabolic_discretize_matches_orca`
//!
//! This environment cannot execute OrcaSlicer's C++ code, so a literal
//! OrcaSlicer-execution-captured golden is not obtainable here — this
//! mirrors the precedent already set in Steps 2-3 of this same packet
//! (`skt_graph_golden.rs`, `voronoi` tests), where goldens are recorded from
//! the Rust implementation's own deterministic output rather than an actual
//! OrcaSlicer C++ run. OrcaSlicer numeric parity is explicitly deferred to
//! P112/T-231 per `design.md`'s Risks section.
//!
//! Instead, this test builds the "OrcaSlicer-discretized polyline" stand-in
//! as an **independent, higher-resolution resampling of the same closed-form
//! parabola equation** (`y = x^2/(2d) + d/2` in the local directrix frame),
//! generated via a separate code path in this file — NOT by calling
//! `discretize_parabolic_edge` itself with a smaller step. It is a
//! same-formula dense-resampling stand-in, not a captured OrcaSlicer C++
//! execution trace.
//!
//! The Hausdorff-distance check against that stand-in is necessarily
//! reference-implementation-relative. The stronger, reference-independent
//! assertion is the on-parabola check: every point returned by
//! `discretize_parabolic_edge` must satisfy
//! `|dist(point, focus) - dist(point, directrix_line)| <= tolerance`, which
//! directly verifies the returned points lie ON the parabola — the real
//! correctness property, independent of any golden ambiguity.

#![cfg(feature = "host-algos")]

use slicer_core::skeletal_trapezoidation::discretize_parabolic_edge;
use slicer_ir::Point2;

/// Distance from a floating-point point to the infinite line through `a`/`b`.
fn dist_point_to_line(px: f64, py: f64, a: Point2, b: Point2) -> f64 {
    let ax = a.x as f64;
    let ay = a.y as f64;
    let bx = b.x as f64;
    let by = b.y as f64;
    let dx = bx - ax;
    let dy = by - ay;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-9 {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    // |cross(b-a, p-a)| / |b-a|
    ((dx * (py - ay) - dy * (px - ax)).abs()) / len
}

fn dist_point_to_point(px: f64, py: f64, q: Point2) -> f64 {
    ((px - q.x as f64).powi(2) + (py - q.y as f64).powi(2)).sqrt()
}

/// Independent dense resampling of the SAME closed-form parabola equation
/// used by `discretize_parabolic_edge` (see module doc comment: this is a
/// same-formula stand-in for an OrcaSlicer C++ golden, not a captured
/// OrcaSlicer trace). Built via its own from-scratch local-frame derivation
/// rather than by calling `discretize_parabolic_edge` with a smaller step,
/// so it does not share a bug with the function under test.
fn dense_reference_polyline(
    focus: Point2,
    line_a: Point2,
    line_b: Point2,
    sample_count: usize,
) -> Vec<(f64, f64)> {
    let ax = line_a.x as f64;
    let ay = line_a.y as f64;
    let bx = line_b.x as f64;
    let by = line_b.y as f64;
    let fx = focus.x as f64;
    let fy = focus.y as f64;

    let dir_x = bx - ax;
    let dir_y = by - ay;
    let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();
    let dir_ux = dir_x / dir_len;
    let dir_uy = dir_y / dir_len;

    let t = (fx - ax) * dir_ux + (fy - ay) * dir_uy;
    let pxx_x = ax + t * dir_ux;
    let pxx_y = ay + t * dir_uy;

    let to_focus_x = fx - pxx_x;
    let to_focus_y = fy - pxx_y;
    let d = (to_focus_x * to_focus_x + to_focus_y * to_focus_y).sqrt();
    let perp_ux = to_focus_x / d;
    let perp_uy = to_focus_y / d;

    let local_x_of = |px: f64, py: f64| (px - pxx_x) * dir_ux + (py - pxx_y) * dir_uy;
    let local_x_a = local_x_of(ax, ay);
    let local_x_b = local_x_of(bx, by);

    let mut points = Vec::with_capacity(sample_count + 1);
    for i in 0..=sample_count {
        let frac = i as f64 / sample_count as f64;
        let local_x = local_x_a + frac * (local_x_b - local_x_a);
        let local_y = local_x * local_x / (2.0 * d) + d / 2.0;
        let world_x = pxx_x + local_x * dir_ux + local_y * perp_ux;
        let world_y = pxx_y + local_x * dir_uy + local_y * perp_uy;
        points.push((world_x, world_y));
    }
    points
}

/// Distance from a point to a single line segment `[a, b]` (clamped
/// projection).
fn point_to_segment_distance(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    let t = if len_sq < 1e-12 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0)
    };
    let cx = ax + t * dx;
    let cy = ay + t * dy;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// Distance from a point to the nearest point on `polyline`, treating
/// consecutive entries as connected line segments (not just nearest
/// vertex) — this is the geometrically meaningful notion for comparing two
/// polylines that approximate the same curve at different resolutions.
fn point_to_polyline_distance(px: f64, py: f64, polyline: &[(f64, f64)]) -> f64 {
    if polyline.len() < 2 {
        return polyline
            .iter()
            .map(|&(qx, qy)| ((px - qx).powi(2) + (py - qy).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min);
    }
    polyline
        .windows(2)
        .map(|w| point_to_segment_distance(px, py, w[0].0, w[0].1, w[1].0, w[1].1))
        .fold(f64::INFINITY, f64::min)
}

/// One-directional (discrete curve) Hausdorff distance: for each point in
/// `from`, the distance to the nearest point on the `to` polyline (as
/// connected segments, not just its vertices); take the max over `from`.
fn one_sided_hausdorff(from: &[(f64, f64)], to_polyline: &[(f64, f64)]) -> f64 {
    from.iter()
        .map(|&(px, py)| point_to_polyline_distance(px, py, to_polyline))
        .fold(0.0, f64::max)
}

fn full_hausdorff(from: &[(f64, f64)], to: &[(f64, f64)]) -> f64 {
    one_sided_hausdorff(from, to).max(one_sided_hausdorff(to, from))
}

/// AC-5: `discretize_parabolic_edge`'s output, for a realistic fixture, must
/// (a) lie within 0.005mm (50 slicer-units) Hausdorff distance of a dense
/// same-formula resampling stand-in, and (b) every returned point must
/// satisfy the true on-parabola equidistance property to within a tight
/// tolerance.
#[test]
fn parabolic_discretize_matches_orca() {
    // Directrix line through (0,0)-(10000,0); focus at (5000,5000).
    // d = 5000 units = 0.5mm; max_segment_len = 1000 units = 0.1mm.
    //
    // max_segment_len is chosen (not copied verbatim from the packet's
    // illustrative "~2000" suggestion) so the curve-approximation sagitta
    // comfortably clears the 50-unit AC-5 tolerance: for this constant-
    // curvature parabola (d=5000) the worst-case perpendicular deviation of
    // a chord from the true curve is `h^2 / (2*d)` where `h` is half the
    // chord's local-x span. At max_segment_len=1000 (h=500), that sagitta is
    // 500^2 / (2*5000) = 25 units — half the tolerance, leaving margin for
    // rounding. The packet's own example (max_segment_len=2000, same
    // focus/line) yields a ~100-unit sagitta near the vertex, which would
    // *not* clear AC-5's 50-unit bound — this is a deliberate deviation from
    // the packet's illustrative numbers, not an oversight; see
    // `follow_up` in the closure log for the recorded rationale.
    let line_a = Point2 { x: 0, y: 0 };
    let line_b = Point2 { x: 10000, y: 0 };
    let focus = Point2 { x: 5000, y: 5000 };
    let max_segment_len = 1000.0;

    let result = discretize_parabolic_edge(focus, line_a, line_b, max_segment_len);

    // Meaningful fixture: several intermediate points, not just 2 endpoints.
    assert!(
        result.len() >= 4,
        "expected at least 4 points for this fixture, got {}",
        result.len()
    );

    // --- Assertion 1: Hausdorff distance vs. dense same-formula resampling ---
    const HAUSDORFF_TOLERANCE_UNITS: f64 = 50.0; // 0.005mm

    let dense = dense_reference_polyline(focus, line_a, line_b, result.len() * 10);
    let result_f: Vec<(f64, f64)> = result.iter().map(|p| (p.x as f64, p.y as f64)).collect();

    let hausdorff = full_hausdorff(&result_f, &dense);
    assert!(
        hausdorff <= HAUSDORFF_TOLERANCE_UNITS,
        "Hausdorff distance {hausdorff} exceeds tolerance {HAUSDORFF_TOLERANCE_UNITS} \
         (result={result:?})"
    );

    // --- Assertion 2: every returned point lies ON the true parabola ---
    // dist(point, focus) == dist(point, directrix_line) for a true parabola
    // point; this is reference-implementation-independent.
    const ON_PARABOLA_TOLERANCE_UNITS: f64 = 2.0;

    for (i, p) in result.iter().enumerate() {
        let px = p.x as f64;
        let py = p.y as f64;
        let d_focus = dist_point_to_point(px, py, focus);
        let d_line = dist_point_to_line(px, py, line_a, line_b);
        let diff = (d_focus - d_line).abs();
        assert!(
            diff <= ON_PARABOLA_TOLERANCE_UNITS,
            "point {i} ({px}, {py}): |dist(focus)={d_focus} - dist(directrix)={d_line}| \
             = {diff} exceeds tolerance {ON_PARABOLA_TOLERANCE_UNITS}"
        );
    }

    // Endpoints should be close to the arc bounds' local-x projections
    // (line_a and line_b themselves lie ON the directrix, not the parabola,
    // so we don't compare directly against them — just sanity-check the
    // polyline spans roughly the expected x-range).
    let xs: Vec<i64> = result.iter().map(|p| p.x).collect();
    let min_x = *xs.iter().min().unwrap();
    let max_x = *xs.iter().max().unwrap();
    assert!(
        min_x >= line_a.x - 10 && max_x <= line_b.x + 10,
        "x-range out of expected bounds: [{min_x}, {max_x}]"
    );
}

/// Degenerate case: focus placed ON the line_a-line_b line (`d ≈ 0`). Must
/// not panic or produce NaN; falls back to the documented straight-line
/// collapse (`vec![line_a, line_b]`).
#[test]
fn degenerate_focus_on_directrix_returns_sane_fallback() {
    let line_a = Point2 { x: 0, y: 0 };
    let line_b = Point2 { x: 10000, y: 0 };
    let focus_on_line = Point2 { x: 4000, y: 0 };

    let result = discretize_parabolic_edge(focus_on_line, line_a, line_b, 2000.0);

    // i64 coordinates cannot be NaN by construction; the meaningful check is
    // that the function returned the documented sane fallback instead of
    // dividing by ~0.
    assert!(!result.is_empty(), "must return at least one point");
    assert_eq!(result, vec![line_a, line_b]);
}

/// Degenerate case: `line_a == line_b` (no directrix direction defined).
/// Must not panic or divide by zero.
#[test]
fn degenerate_line_a_equals_line_b_returns_sane_fallback() {
    let a = Point2 { x: 3000, y: 3000 };
    let focus = Point2 { x: 5000, y: 5000 };

    let result = discretize_parabolic_edge(focus, a, a, 2000.0);

    assert!(!result.is_empty(), "must return at least one point");
    assert_eq!(result, vec![a, a]);
}
