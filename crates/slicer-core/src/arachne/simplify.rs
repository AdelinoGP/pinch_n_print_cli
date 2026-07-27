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
//! 2. **Near-colinear fast path**: if `height_2 ≤ 2.5e-5 mm²` AND the point is
//!    truly colinear (`distance_to_infinite² ≤ 2.5e-5 mm²`, i.e. canonical's
//!    0.005mm branch point) AND
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
//! aggressively exactly when the caller had asked for no simplification.
//!
//! Note that only tier 3 reads the distance gates. Tier 1 compares against a
//! hardcoded 5µm and tier 2 against a hardcoded 5µm colinearity band, so zero
//! gates do not freeze the polyline outright — they disable the primary gate
//! while ultra-short and exactly-colinear junctions remain removable, exactly
//! as in canonical.
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

    // Canonical's closed-polygon walk assumes the representation where a closed
    // `ExtrusionLine`'s first and last junctions are the same point, and it ends
    // by copying the last position onto the first.
    //
    // PnP has two conventions in play. `stitch_extrusions` builds closed loops
    // that way, duplicating the start junction onto the end. But
    // `generate_toolpaths` emits the local-maximum hexagonal micro-loop as six
    // *distinct* junctions with `is_closed: true` and no duplicate. Applying the
    // closing copy to that representation overwrites a real vertex, collapsing
    // two of the six and shifting the loop's centroid.
    //
    // So the wrap-around is driven by the actual geometry rather than by the
    // flag alone: a closed line whose endpoints do not coincide is walked as an
    // open polyline, which keeps every one of its vertices.
    let has_duplicate_endpoint =
        junctions[0].p.x == junctions[n - 1].p.x && junctions[0].p.y == junctions[n - 1].p.y;
    let wrap_around = is_closed && has_duplicate_endpoint;

    // Always retain the first junction.
    let mut result: Vec<ExtrusionJunction> = Vec::with_capacity(n);
    result.push(junctions[0].clone());

    // Track previous and previous_previous as value copies (not indices). When
    // the first and last junctions coincide, the vertex "before" the start is
    // the one prior to the last, not the start itself.
    let mut previous_previous = if wrap_around {
        junctions[n - 2].clone()
    } else {
        junctions[0].clone()
    };
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

    // A closed polygon processes one vertex further than an open polyline: its
    // final vertex is the same point as its first, and canonical folds it back
    // into the already-emitted output rather than treating it as an endpoint.
    let end = if wrap_around { n } else { n - 1 };

    let mut curr = 1usize;
    while curr < end {
        // For the last vertex of a closed polygon, use the first junction of
        // the *new* polygon, since it may already have been relocated.
        let is_last = curr + 1 == n;
        let current = if is_last {
            result[0].clone()
        } else {
            junctions[curr].clone()
        };

        // Never simplify a closed polygon below 3 junctions. This also bounds
        // the wrap-around indexing below: it fires before `curr + 2 - n` could
        // exceed what has been emitted.
        if wrap_around && result.len() + (n - curr) <= 3 {
            result.push(current);
            previous_previous = previous.clone();
            previous = result[result.len() - 1].clone();
            curr += 1;
            continue;
        }

        // Spill over into the emitted output when `next` would run past the
        // end of a closed polygon.
        let spill_over = wrap_around && curr + 2 >= n && (curr + 2 - n) < result.len();
        let next = if spill_over {
            result[curr + 2 - n].clone()
        } else {
            junctions[curr + 1].clone()
        };

        // Canonical computes both area terms and accumulates once per
        // iteration, before any removal test. `negative_area_closing` closes
        // the fan against the *current* short-cutting segment, so it depends on
        // `next` and cannot be hoisted out of the loop.
        let removed_area_next = shoelace(&current, &next);
        let negative_area_closing = shoelace(&next, &previous);
        accumulated_area_removed += removed_area_next;

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
        //
        // Canonical gates this on `height_2 <= sqr(scaled(0.005))` and
        // `distance_to_infinite(current, previous, next) <= scaled(0.005)`.
        // Both constants are 0.005mm — 5µm, not 1µm. The height term is
        // already squared, giving 2.5e-5 mm²; the distance term is compared
        // un-squared upstream, so squaring both sides of that comparison gives
        // the same 2.5e-5 mm² against PnP's squared distance helper.
        //
        // PnP previously used 1e-6 for both, i.e. a 0.001mm branch point,
        // making this guard 25x stricter than canonical in squared terms.
        const NEAR_COLINEAR_MM2: f64 = 0.005 * 0.005; // 2.5e-5 mm²
        let inline_dist = point_to_infinite_line_distance_squared(&previous, &next, &current);
        if height_2 <= NEAR_COLINEAR_MM2 && inline_dist <= NEAR_COLINEAR_MM2 {
            let area_dev = calculate_extrusion_area_deviation_error(&previous, &current, &next);
            if area_dev <= maximum_extrusion_area_deviation {
                // Remove: near-colinear with acceptable area deviation.
                curr += 1;
                continue;
            }
        }

        // Tier 3: Primary distance gate. Canonical nests the far-next-vertex
        // special case *inside* this gate — it is not a standalone branch — so
        // a junction that fails this gate is never a candidate for relocation.
        if seg_len_sq < smallest_line_segment_squared && height_2 <= allowed_error_distance_squared
        {
            // Canonical measures `current -> next` here, not `previous -> next`.
            let next_length2 = {
                let dx = (next.p.x - current.p.x) as f64;
                let dy = (next.p.y - current.p.y) as f64;
                dx * dx + dy * dy
            };

            if next_length2 > 4.0 * smallest_line_segment_squared {
                // The next line is long: removing `current` outright could leave
                // a noticeable artifact, so try to relocate it to the
                // intersection of `previous_previous -> previous` and
                // `current -> next`, which keeps both edge directions.
                let relocated =
                    line_intersection_infinite(&previous_previous, &previous, &current, &next)
                        .filter(|&(ix, iy)| {
                            // Reject an intersection that is itself an artifact: too far
                            // off the `previous -> current` line, or too far from either
                            // endpoint to stand in for `current`.
                            point_to_infinite_line_distance_squared_xy(
                                (ix, iy),
                                &previous,
                                &current,
                            ) <= allowed_error_distance_squared
                                && !dist_greater(
                                    (ix, iy),
                                    (previous.p.x as f64, previous.p.y as f64),
                                    smallest_line_segment_squared,
                                )
                                && !dist_greater(
                                    (ix, iy),
                                    (current.p.x as f64, current.p.y as f64),
                                    smallest_line_segment_squared,
                                )
                        });

                if let Some((ix, iy)) = relocated {
                    // Replace: drop the previously-pushed junction and push the
                    // intersection, carrying `current`'s width and
                    // `perimeter_index` verbatim.
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
                    if !result.is_empty() {
                        result.pop();
                        previous = previous_previous.clone();
                    }
                    accumulated_area_removed = removed_area_next;
                    previous_previous = previous.clone();
                    previous = intersection.clone();
                    result.push(intersection);
                    curr += 1;
                    continue;
                }
                // No usable spot for it: fall through and retain `current`.
            } else {
                // Remove: short segment within error tolerance, and the next
                // line is not long enough to need the relocation treatment.
                curr += 1;
                continue;
            }
        }

        // Retain this junction.
        result.push(current.clone());
        previous_previous = previous.clone();
        previous = current.clone();
        accumulated_area_removed = removed_area_next;
        curr += 1;
    }

    if wrap_around {
        // The first and last points of a closed polygon must be the same. The
        // last point was processed in the loop above, so copy its position into
        // the first — position only, so the start junction keeps its own width
        // and per-vertex attributes.
        if let Some(back) = result.last().map(|j| j.p) {
            if let Some(front) = result.first_mut() {
                front.p = back;
            }
        }
    } else {
        // The ending junction always exists in the simplified path.
        result.push(junctions[n - 1].clone());
    }
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

/// Returns `true` when `p1` is farther from `p2` than `threshold_sq`, which is a
/// *squared* length (`smallest_line_segment_squared`) — so this is the plain
/// comparison `|p1 − p2|² > threshold_sq`.
///
/// Canonical (`ExtrusionLine::simplify`'s local `dist_greater` lambda) precedes
/// that with a component-wise fast-reject, `vec.x() > threshold ⇒ true`. That
/// shortcut is **deliberately not ported**, for two reasons:
///
/// 1. **Its implication only holds when `threshold >= 1`.** Canonical's
///    coordinates are scaled integers and its threshold is a squared length in
///    those units — an enormous number — so a single component exceeding it
///    guarantees the squared norm does too, and the branch is a pure overflow
///    guard that essentially never fires. PnP's coordinates are `f32`
///    millimetres, where `smallest_line_segment_squared` is `0.0025` (0.05mm
///    squared) — *less than one*. There the implication inverts: `dx = 0.01`mm
///    exceeds `0.0025` while `dx² = 1e-4` does not, so the shortcut rejects
///    points that are comfortably inside the threshold.
/// 2. **`f64` cannot overflow here anyway**, so the guard buys nothing.
///
/// PnP previously ported the shortcut literally *and* squared the threshold a
/// second time in the fallback (`> threshold_sq * threshold_sq`). Together those
/// made this predicate reject anything beyond ~2.5µm where canonical allows
/// 50µm — 20x too strict — which left the tier-3 relocation branch effectively
/// unreachable.
fn dist_greater(p1: (f64, f64), p2: (f64, f64), threshold_sq: f64) -> bool {
    let dx = p1.0 - p2.0;
    let dy = p1.1 - p2.1;
    dx * dx + dy * dy > threshold_sq
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

/// Squared distance from a bare `(x, y)` to the infinite line through `a` and
/// `b`. Used by the tier-3 relocation test, where the candidate point is a
/// computed intersection rather than an existing junction.
fn point_to_infinite_line_distance_squared_xy(
    p: (f64, f64),
    a: &ExtrusionJunction,
    b: &ExtrusionJunction,
) -> f64 {
    let (ax, ay) = (a.p.x as f64, a.p.y as f64);
    let (bx, by) = (b.p.x as f64, b.p.y as f64);

    let abx = bx - ax;
    let aby = by - ay;
    let apx = p.0 - ax;
    let apy = p.1 - ay;

    let ab_len_sq = abx * abx + aby * aby;
    if ab_len_sq < 1e-18 {
        // Degenerate: a and b coincide.
        return apx * apx + apy * apy;
    }

    let cross = abx * apy - aby * apx;
    (cross * cross) / ab_len_sq
}
