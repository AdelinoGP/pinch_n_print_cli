// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/ExtrusionLine.cpp
// (`ExtrusionLine::simplify`) and src/libslic3r/Arachne/utils/ExtrusionLine.cpp
// (`calculateExtrusionAreaDeviationError`).
//
// This file is an LLM-generated Rust port, adapted for the Pinch 'n Print
// architecture. It implements the canonical distance-gated single-pass
// simplification with `calculateExtrusionAreaDeviationError` as an extra guard
// on the near-colinear fast path only (packet 146, N13).
// -----------------------------------------------------------------------------
//! Packet 146 (N13): distance-gated polyline simplification for
//! `ExtrusionLine`s, replacing the iterative multi-pass area-only sweep
//! (packet 113a) with the canonical single linear pass from
//! `ExtrusionLine.cpp:56-243`.
//!
//! Three-tier removal per junction (evaluated in order):
//! 1. **Ultra-short bypass**: segments shorter than ~5 µm (0.005mm) are
//!    always removed.
//! 2. **Near-colinear fast path**: if `height_2 ≤ 0.001` AND the point is
//!    truly colinear (`distance_to_infinite ≤ 0.001`) AND
//!    `calculateExtrusionAreaDeviationError ≤ maximum_extrusion_area_deviation`
//!    → remove. The area-deviation guard is only on this branch.
//! 3. **Primary gate** (`smallest_line_segment_squared` +
//!    `allowed_error_distance_squared`): if the current segment is shorter
//!    than `smallest_line_segment_squared` AND
//!    `height_2 ≤ allowed_error_distance_squared` → remove.
//!
//! A junction that fails all removal tests is pushed onto the output and
//! resets the area accumulator. Both endpoints are always retained.
//!
//! There is no area-only fallback. Canonical `ExtrusionLine::simplify` has no
//! such branch, and the one PnP carried (the packet-113a sweep, kept alive for
//! zero distance gates) inverted the meaning of zero gates: it simplified most
//! aggressively exactly when the caller had asked for no simplification. Zero
//! gates now leave every junction in place.
//!
//! Each retained junction keeps its original `ExtrusionJunction` value
//! (width, flow_factor, overhang_quartile, perimeter_index) untouched — no
//! averaging or interpolation of width across a dropped run.

use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

/// Width-difference threshold (mm) below which
/// [`calculate_extrusion_area_deviation_error`] takes canonical's
/// equal-width branch instead of the weighted-average-area branch.
///
/// Canonical tests `width_diff > 1` in scaled integer units. Under its standard
/// scaling factor one unit is one nanometre, so the millimetre equivalent is
/// `1e-6`.
///
/// **Known divergence:** canonical's scaling factor is not a constant — it
/// carries a standard value and a ten-times-coarser large-printer value, so this
/// branch point (and the `maximum_extrusion_area_deviation` threshold that gates
/// the same guard) shift by 10x on large-printer profiles. PnP has no
/// large-printer scaling concept anywhere in its coordinate system, so both
/// values are pinned to canonical's standard-printer behaviour rather than
/// introducing one here to match a fork quirk.
const WIDTH_DIFF_EPSILON_MM: f64 = 1e-6;

/// Runs distance-gated simplification on every line's junction polyline.
///
/// `smallest_line_segment_squared` (mm²) is the squared distance gate from
/// `meshfix_maximum_resolution`: segments shorter than this AND within
/// `allowed_error_distance_squared` of the chord are removed.
///
/// `allowed_error_distance_squared` (mm²) is the squared error distance gate
/// from `meshfix_maximum_deviation`.
///
/// `maximum_extrusion_area_deviation` (mm²) is the area deviation threshold
/// for the near-colinear fast-path guard.
pub fn simplify_toolpaths(
    lines: Vec<ExtrusionLine>,
    smallest_line_segment_squared: f64,
    allowed_error_distance_squared: f64,
    maximum_extrusion_area_deviation: f64,
) -> Vec<ExtrusionLine> {
    lines
        .into_iter()
        .map(|line| {
            simplify_line(
                line,
                smallest_line_segment_squared,
                allowed_error_distance_squared,
                maximum_extrusion_area_deviation,
            )
        })
        .collect()
}

fn simplify_line(
    line: ExtrusionLine,
    smallest_line_segment_squared: f64,
    allowed_error_distance_squared: f64,
    maximum_extrusion_area_deviation: f64,
) -> ExtrusionLine {
    let ExtrusionLine {
        junctions,
        inset_idx,
        is_odd,
        is_closed,
    } = line;

    let n = junctions.len();
    if n <= 2 {
        return ExtrusionLine {
            junctions,
            inset_idx,
            is_odd,
            is_closed,
        };
    }

    // Canonical has no fallback branch: `ExtrusionLine::simplify` always runs
    // the single distance-gated pass. Zero gates are not a mode switch — they
    // simply make the gates unsatisfiable, so every junction is retained, which
    // is the correct reading of "don't simplify".
    let simplified = simplify_distance_gated(
        &junctions,
        is_closed,
        smallest_line_segment_squared,
        allowed_error_distance_squared,
        maximum_extrusion_area_deviation,
    );
    ExtrusionLine {
        junctions: simplified,
        inset_idx,
        is_odd,
        is_closed,
    }
}

/// Canonical single linear pass with distance gates (ExtrusionLine.cpp:56-243).
///
/// Tracks `previous` and `previous_previous` as `ExtrusionJunction` value
/// copies (ExtrusionLine.cpp:75,79) and ports the tier-3 special case
/// (ExtrusionLine.cpp:166-220): when the next vertex is far away
/// (`next_length2 > 4 * smallest_line_segment_squared`), the intersection of
/// the infinite lines through `(previous_previous → previous)` and
/// `(current → next)` is computed and, unless the intersection is too far from
/// `previous`, the previously-pushed junction is popped and replaced by the
/// intersection carrying `current`'s width and `perimeter_index` verbatim.
///
/// Height at the tier-2 and tier-3 gate sites uses canonical's Shoelace
/// formula `height_2 = area_removed_so_far² / base_length_2`, where
/// `area_removed_so_far` is the running `accumulated_area_removed` plus
/// `negative_area_closing` — the latter recomputed each iteration against the
/// current short-cutting segment, not hoisted.
fn simplify_distance_gated(
    junctions: &[ExtrusionJunction],
    is_closed: bool,
    smallest_line_segment_squared: f64,
    allowed_error_distance_squared: f64,
    maximum_extrusion_area_deviation: f64,
) -> Vec<ExtrusionJunction> {
    let n = junctions.len();

    // Minimum-size guard (ExtrusionLine.cpp:63-65): open lines need at least
    // 3 junctions to have a simplifiable interior; closed lines need at least
    // 4, since the implicit closing edge consumes one more vertex.
    let min_path_size = if is_closed { 3 } else { 2 };
    if n <= min_path_size {
        return junctions.to_vec();
    }

    // Always retain the first junction.
    let mut result: Vec<ExtrusionJunction> = Vec::with_capacity(n);
    result.push(junctions[0].clone());

    // Track previous and previous_previous as value copies (not indices).
    let mut previous_previous = junctions[0].clone();
    let mut previous = junctions[0].clone();

    // Canonical accumulates the cut-off region with the Shoelace formula as a
    // *fan* of blades from an origin to each removed segment, so every term is
    // origin-relative (`p.x * q.y - p.y * q.x`) rather than a translation-
    // invariant triangle area. The fan sum's origin-dependence cancels only in
    // the closed combination `accumulated_area_removed + negative_area_closing`,
    // so the two must share one origin and `negative_area_closing` must be
    // recomputed against the current `next` on every iteration.
    //
    // **Deliberate numeric deviation.** Canonical works in scaled `coord_t`
    // and accumulates in `int64_t`, where origin-relative products are exact.
    // PnP's coordinates are `f32` millimetres evaluated in `f64`, so taking the
    // global origin would form products on the order of the plate offset and
    // then difference them, losing several digits to cancellation exactly where
    // canonical loses none. Translating the origin to `junctions[0]` keeps the
    // operands on the order of the part rather than the plate. The combination
    // is origin-independent in exact arithmetic, so this changes no result
    // canonical would compute -- it only removes error canonical never had.
    let ox = junctions[0].p.x as f64;
    let oy = junctions[0].p.y as f64;
    let shoelace = |p: &ExtrusionJunction, q: &ExtrusionJunction| -> f64 {
        let px = p.p.x as f64 - ox;
        let py = p.p.y as f64 - oy;
        let qx = q.p.x as f64 - ox;
        let qy = q.p.y as f64 - oy;
        px * qy - py * qx
    };

    // Seeded with the blade from the origin to junctions[0] → junctions[1]
    // (canonical's `initial`), not zero. Reset to the current iteration's
    // removed area on retain/replace.
    let mut accumulated_area_removed = shoelace(&junctions[0], &junctions[1]);

    let mut curr = 1usize;
    while curr < n - 1 {
        let current = junctions[curr].clone();
        let next = junctions[curr + 1].clone();

        // Canonical computes both area terms and accumulates once per
        // iteration, before any removal test. `negative_area_closing` closes
        // the fan against the *current* short-cutting segment, so it depends on
        // `next` and cannot be hoisted out of the loop.
        let removed_area_next = shoelace(&current, &next);
        let negative_area_closing = shoelace(&next, &previous);
        accumulated_area_removed += removed_area_next;

        let next_length2 = {
            let dx = (next.p.x - previous.p.x) as f64;
            let dy = (next.p.y - previous.p.y) as f64;
            dx * dx + dy * dy
        };

        // Tier 3 special case: the next vertex is far away, so the previous
        // vertex might be a feature we need to keep or relocate
        // (ExtrusionLine.cpp:166-220).
        if next_length2 > 4.0 * smallest_line_segment_squared {
            if let Some((ix, iy)) =
                line_intersection_infinite(&previous_previous, &previous, &current, &next)
            {
                // Reject path: if the intersection is too far from `previous`,
                // preserve `previous` and advance (current becomes the new
                // previous, retained).
                if dist_greater(
                    (ix, iy),
                    (previous.p.x as f64, previous.p.y as f64),
                    smallest_line_segment_squared,
                ) {
                    result.push(current.clone());
                    previous_previous = previous.clone();
                    previous = current.clone();
                    accumulated_area_removed = removed_area_next;
                    curr += 1;
                    continue;
                }

                // Replacement path: pop the previously-pushed junction,
                // restore previous = previous_previous, push the intersection
                // carrying `current`'s width and perimeter_index verbatim,
                // re-advance both cursors.
                result.pop();
                let intersection = ExtrusionJunction {
                    p: Point3WithWidth {
                        x: ix as f32,
                        y: iy as f32,
                        z: current.p.z,
                        width: current.p.width,
                        flow_factor: current.p.flow_factor,
                        overhang_quartile: current.p.overhang_quartile,
                        dist_to_top_mm: current.p.dist_to_top_mm,
                    },
                    perimeter_index: current.perimeter_index,
                };
                result.push(intersection.clone());
                previous = intersection;
                accumulated_area_removed = removed_area_next;
                curr += 1;
                continue;
            }
        }

        // Height via the canonical Shoelace formula: closing the fan gives the
        // cut-off area, and `h² = L² / b²` recovers the representative
        // triangle's height without recomputing previously removed vertices.
        let area_removed_so_far = accumulated_area_removed + negative_area_closing;
        let base_length2 = {
            let dx = (next.p.x - previous.p.x) as f64;
            let dy = (next.p.y - previous.p.y) as f64;
            dx * dx + dy * dy
        };

        // Two segments doubling back with no area between them: canonical
        // removes the junction rather than dividing by zero.
        if base_length2 == 0.0 {
            curr += 1;
            continue;
        }

        let height_2 = (area_removed_so_far * area_removed_so_far) / base_length2;

        // Segment length squared (previous → current).
        let seg_dx = (current.p.x - previous.p.x) as f64;
        let seg_dy = (current.p.y - previous.p.y) as f64;
        let seg_len_sq = seg_dx * seg_dx + seg_dy * seg_dy;

        // Tier 1: Ultra-short bypass (ExtrusionLine.cpp ~5µm).
        let ultra_short_threshold = 0.000025; // 0.005mm squared = 2.5e-5 mm²
        if seg_len_sq < ultra_short_threshold {
            // Remove: ultra-short segment.
            curr += 1;
            continue;
        }

        // Tier 2: Near-colinear fast path with area deviation guard.
        // Thresholds match OrcaSlicer ExtrusionLine.cpp's µm-scale constants
        // converted to mm²: 0.001² = 1e-6 mm² for height and inline distance.
        let near_colinear_height = 1e-6; // (0.001mm)² = 1µm²
        let near_colinear_inline = 1e-6; // (0.001mm)² = 1µm²
        let inline_dist = point_to_infinite_line_distance_squared(&previous, &next, &current);
        if height_2 <= near_colinear_height
            && inline_dist <= near_colinear_inline
            && maximum_extrusion_area_deviation > 0.0
        {
            let area_dev = calculate_extrusion_area_deviation_error(&previous, &current, &next);
            if area_dev <= maximum_extrusion_area_deviation {
                // Remove: near-colinear with acceptable area deviation.
                curr += 1;
                continue;
            }
        }

        // Tier 3: Primary distance gate.
        if seg_len_sq < smallest_line_segment_squared && height_2 <= allowed_error_distance_squared
        {
            // Remove: short segment within error tolerance.
            curr += 1;
            continue;
        }

        // Retain this junction.
        result.push(current.clone());
        previous_previous = previous.clone();
        previous = current.clone();
        accumulated_area_removed = removed_area_next;
        curr += 1;
    }

    // Always retain the last junction.
    result.push(junctions[n - 1].clone());
    result
}

/// Intersection of the infinite lines through `a`–`b` and `c`–`d`.
/// Returns `None` when the lines are (near-)parallel. Coordinates are mm (f64).
fn line_intersection_infinite(
    a: &ExtrusionJunction,
    b: &ExtrusionJunction,
    c: &ExtrusionJunction,
    d: &ExtrusionJunction,
) -> Option<(f64, f64)> {
    let ax = a.p.x as f64;
    let ay = a.p.y as f64;
    let bx = b.p.x as f64;
    let by = b.p.y as f64;
    let cx = c.p.x as f64;
    let cy = c.p.y as f64;
    let dx = d.p.x as f64;
    let dy = d.p.y as f64;

    let r_px = bx - ax;
    let r_py = by - ay;
    let s_px = dx - cx;
    let s_py = dy - cy;

    let denom = r_px * s_py - r_py * s_px;
    if denom.abs() < 1e-18 {
        return None;
    }

    let t = ((cx - ax) * s_py - (cy - ay) * s_px) / denom;
    Some((ax + t * r_px, ay + t * r_py))
}

/// Overflow-avoiding distance-greater predicate (ExtrusionLine.cpp:180-188).
///
/// Returns `true` when `p1` is farther from `p2` than `threshold_sq` (the
/// squared form): first a component-wise fast-reject (any coordinate magnitude
/// exceeds `threshold_sq`), then the precise squared-norm comparison
/// `(p1 − p2).squaredNorm() > threshold_sq²`.
fn dist_greater(p1: (f64, f64), p2: (f64, f64), threshold_sq: f64) -> bool {
    let dx = p1.0 - p2.0;
    let dy = p1.1 - p2.1;
    if dx > threshold_sq || dx < -threshold_sq || dy > threshold_sq || dy < -threshold_sq {
        return true;
    }
    dx * dx + dy * dy > threshold_sq * threshold_sq
}

/// Width-weighted extrusion-area deviation introduced by removing the middle
/// junction `b`, i.e. by replacing the two segments `a`–`b` and `b`–`c` with the
/// single segment `a`–`c` carrying their length-weighted average width.
///
/// Port of canonical `ExtrusionLine::calculateExtrusionAreaDeviationError`
/// (`ExtrusionLine.cpp`). Returned in mm².
///
/// **This replaced a non-canonical formula.** The previous implementation
/// computed `0.5 * width_at_b * |cross(AB, AC)| / |AC|` — a width-weighted
/// triangle height — and attributed it to canonical. That formula does not
/// appear anywhere in `ExtrusionLine.cpp`: it measures how far `b` sits off the
/// chord, not how much extruded area moves when `b` is dropped, and it ignores
/// the widths at `a` and `c` entirely. Canonical's quantity is a genuine area
/// difference and depends on all three widths. Do not "simplify" it back.
///
/// Two fidelity notes against canonical, both deliberate:
///
/// 1. **Arithmetic domain.** Canonical works in scaled integer `coord_t` and
///    accumulates in `int64_t`, so its two divisions truncate. PnP's junction
///    coordinates and widths are `f32` millimetres (`Point3WithWidth`), so this
///    port evaluates in `f64` and does not truncate. Reproducing the truncation
///    would mean reproducing canonical's scaled-integer coordinate space inside
///    this one function, which would be a larger and more fragile divergence
///    than the rounding it removes.
/// 2. **The small-width-difference branch threshold.** Canonical tests
///    `width_diff > 1`, where `1` is one scaled unit — one nanometre under its
///    standard scaling factor. Expressed in millimetres that is `1e-6`, which is
///    what [`WIDTH_DIFF_EPSILON_MM`] carries. Note this makes canonical's own
///    branch point scaling-factor dependent; PnP pins the standard-printer
///    value (see [`WIDTH_DIFF_EPSILON_MM`]).
fn calculate_extrusion_area_deviation_error(
    a: &ExtrusionJunction,
    b: &ExtrusionJunction,
    c: &ExtrusionJunction,
) -> f64 {
    let (ax, ay) = (a.p.x as f64, a.p.y as f64);
    let (bx, by) = (b.p.x as f64, b.p.y as f64);
    let (cx, cy) = (c.p.x as f64, c.p.y as f64);

    let (aw, bw, cw) = (a.p.width as f64, b.p.width as f64, c.p.width as f64);

    let ab_length = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
    let bc_length = ((cx - bx).powi(2) + (cy - by).powi(2)).sqrt();

    let width_diff = (bw - aw).abs().max((cw - bw).abs());

    if width_diff > WIDTH_DIFF_EPSILON_MM {
        let ab_weight = (aw + bw) / 2.0;
        let bc_weight = (bw + cw) / 2.0;

        let total_length = ab_length + bc_length;
        if total_length < 1e-18 {
            // Degenerate: a, b and c coincide, so removing b moves no area.
            // Canonical divides by this sum unguarded (both lengths are zero
            // only for coincident junctions, which its callers filter earlier).
            return 0.0;
        }

        let weighted_average_width = (ab_length * ab_weight + bc_length * bc_weight) / total_length;
        let ac_length = ((cx - ax).powi(2) + (cy - ay).powi(2)).sqrt();

        ((ab_weight * ab_length + bc_weight * bc_length) - (weighted_average_width * ac_length))
            .abs()
    } else {
        // Widths are effectively equal: charge the width difference against the
        // shorter of the two segments, matching canonical's else-branch.
        if ab_length > bc_length {
            width_diff * bc_length
        } else {
            width_diff * ab_length
        }
    }
}

/// Squared perpendicular distance from point `p` to the line through `a` and
/// `b`. Returns mm².
fn point_line_distance_squared(
    a: &ExtrusionJunction,
    p: &ExtrusionJunction,
    b: &ExtrusionJunction,
) -> f64 {
    let (ax, ay) = (a.p.x as f64, a.p.y as f64);
    let (bx, by) = (b.p.x as f64, b.p.y as f64);
    let (px, py) = (p.p.x as f64, p.p.y as f64);

    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;

    let cross = abx * apy - aby * apx;
    let ab_len_sq = abx * abx + aby * aby;

    if ab_len_sq < 1e-18 {
        // Degenerate: a and b coincide.
        let dx = px - ax;
        let dy = py - ay;
        return dx * dx + dy * dy;
    }

    (cross * cross) / ab_len_sq
}

/// Squared distance from point `p` to the infinite line through `a` and `b`.
/// Same as `point_line_distance_squared` — the perpendicular distance is the
/// same for the infinite line and the segment (the "distance to infinite line"
/// in ExtrusionLine.cpp).
fn point_to_infinite_line_distance_squared(
    a: &ExtrusionJunction,
    b: &ExtrusionJunction,
    p: &ExtrusionJunction,
) -> f64 {
    point_line_distance_squared(a, p, b)
}
