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
//! Each retained junction keeps its original `ExtrusionJunction` value
//! (width, flow_factor, overhang_quartile, perimeter_index) untouched — no
//! averaging or interpolation of width across a dropped run.

use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

/// Runs distance-gated simplification on every line's junction polyline.
///
/// `visvalingam_area_threshold` is the maximum width-weighted area deviation
/// (mm²) a dropped junction may introduce (legacy parameter, used as fallback
/// when distance gates are zero).
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
    visvalingam_area_threshold: f64,
    smallest_line_segment_squared: f64,
    allowed_error_distance_squared: f64,
    maximum_extrusion_area_deviation: f64,
) -> Vec<ExtrusionLine> {
    lines
        .into_iter()
        .map(|line| {
            simplify_line(
                line,
                visvalingam_area_threshold,
                smallest_line_segment_squared,
                allowed_error_distance_squared,
                maximum_extrusion_area_deviation,
            )
        })
        .collect()
}

fn simplify_line(
    line: ExtrusionLine,
    visvalingam_area_threshold: f64,
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

    // Use distance gates if both are positive; otherwise fall back to the
    // legacy area-only sweep.
    let use_distance_gates =
        smallest_line_segment_squared > 0.0 && allowed_error_distance_squared > 0.0;

    if use_distance_gates {
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
    } else {
        // Legacy fallback: iterative area-only sweep (packet 113a).
        let simplified = simplify_area_only(&junctions, visvalingam_area_threshold);
        ExtrusionLine {
            junctions: simplified,
            inset_idx,
            is_odd,
            is_closed,
        }
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
/// Height at the tier-2 and tier-3 gate sites uses the OrcaSlicer Shoelace
/// formula `height_2 = area_removed_so_far² / base_length_2`
/// (ExtrusionLine.cpp:151), where `area_removed_so_far` is the running
/// `accumulated_area_removed` plus the constant `negative_area_closing`.
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

    // Signed area of the closing edge junctions[0] → junctions[1]
    // (ExtrusionLine.cpp: negative_area_closing). Constant for the whole walk.
    let negative_area_closing = {
        let c0 = &junctions[0];
        let c1 = &junctions[1];
        (c0.p.x as f64) * (c1.p.y as f64) - (c1.p.x as f64) * (c0.p.y as f64)
    };

    // Running accumulator of removed triangle area (declared once outside the
    // loop). Reset to the current iteration's removed area on retain/replace.
    let mut accumulated_area_removed = 0.0;

    let mut curr = 1usize;
    while curr < n - 1 {
        let current = junctions[curr].clone();
        let next = junctions[curr + 1].clone();

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
                let removed_area_next = triangle_signed_area_x2(&previous, &current, &next);

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

        // Height via OrcaSlicer Shoelace formula (ExtrusionLine.cpp:151).
        let removed_area_next = triangle_signed_area_x2(&previous, &current, &next);
        let area_removed_so_far = accumulated_area_removed + negative_area_closing;
        let base_length2 = {
            let dx = (next.p.x - previous.p.x) as f64;
            let dy = (next.p.y - previous.p.y) as f64;
            dx * dx + dy * dy
        };
        let height_2 = (area_removed_so_far * area_removed_so_far) / base_length2.max(1e-18);

        // Segment length squared (previous → current).
        let seg_dx = (current.p.x - previous.p.x) as f64;
        let seg_dy = (current.p.y - previous.p.y) as f64;
        let seg_len_sq = seg_dx * seg_dx + seg_dy * seg_dy;

        // Tier 1: Ultra-short bypass (ExtrusionLine.cpp ~5µm).
        let ultra_short_threshold = 0.000025; // 0.005mm squared = 2.5e-5 mm²
        if seg_len_sq < ultra_short_threshold {
            // Remove: ultra-short segment.
            accumulated_area_removed += removed_area_next;
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
                accumulated_area_removed += removed_area_next;
                curr += 1;
                continue;
            }
        }

        // Tier 3: Primary distance gate.
        if seg_len_sq < smallest_line_segment_squared && height_2 <= allowed_error_distance_squared
        {
            // Remove: short segment within error tolerance.
            accumulated_area_removed += removed_area_next;
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

/// Twice the signed area of triangle `(a, b, c)` via the cross product
/// `(a − c) × (b − c)` — the OrcaSlicer `removed_area_next` term
/// (ExtrusionLine.cpp:151 area term). Used by the Shoelace height formula and
/// by `accumulated_area_removed`.
fn triangle_signed_area_x2(
    a: &ExtrusionJunction,
    b: &ExtrusionJunction,
    c: &ExtrusionJunction,
) -> f64 {
    let ax = a.p.x as f64;
    let ay = a.p.y as f64;
    let bx = b.p.x as f64;
    let by = b.p.y as f64;
    let cx = c.p.x as f64;
    let cy = c.p.y as f64;
    (ax - cx) * (by - cy) - (bx - cx) * (ay - cy)
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

/// Legacy iterative area-only sweep (packet 113a). Kept as fallback when
/// distance gates are zero.
fn simplify_area_only(
    junctions: &[ExtrusionJunction],
    visvalingam_area_threshold: f64,
) -> Vec<ExtrusionJunction> {
    let n = junctions.len();
    if n <= 2 {
        return junctions.to_vec();
    }

    let mut keep: Vec<bool> = vec![true; n];

    loop {
        let mut removed = false;
        let mut i = 1;
        while i < n - 1 {
            if !keep[i] {
                i += 1;
                continue;
            }

            let mut prev = i.saturating_sub(1);
            while prev > 0 && !keep[prev] {
                prev -= 1;
            }
            if !keep[prev] {
                prev = i;
            }
            let mut next = i + 1;
            while next < n && !keep[next] {
                next += 1;
            }
            if next >= n {
                break;
            }

            let deviation = calculate_extrusion_area_deviation_error(
                &junctions[prev],
                &junctions[i],
                &junctions[next],
            );
            if deviation <= visvalingam_area_threshold {
                keep[i] = false;
                removed = true;
            }
            i = next;
        }
        if !removed {
            break;
        }
    }

    junctions
        .iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(j, _)| j.clone())
        .collect()
}

/// Width-weighted area deviation of the middle junction `b` relative to the
/// chord `a`–`c`.
///
/// Formula (OrcaSlicer ExtrusionLine.cpp:248):
/// `0.5 * width_at_b * |cross(AB, AC)| / |AC|` — the magnitude of the
/// triangle area projected at `b`, weighted by `b`'s extrusion width, and
/// normalized by the chord length `a`–`c`. Returned in mm².
fn calculate_extrusion_area_deviation_error(
    a: &ExtrusionJunction,
    b: &ExtrusionJunction,
    c: &ExtrusionJunction,
) -> f64 {
    let (ax, ay) = (a.p.x as f64, a.p.y as f64);
    let (bx, by) = (b.p.x as f64, b.p.y as f64);
    let (cx, cy) = (c.p.x as f64, c.p.y as f64);

    let abx = bx - ax;
    let aby = by - ay;
    let acx = cx - ax;
    let acy = cy - ay;

    let cross = abx * acy - aby * acx;
    let chord_len = (acx * acx + acy * acy).sqrt();

    if chord_len < 1e-18 {
        let dx = bx - ax;
        let dy = by - ay;
        return 0.5 * b.p.width as f64 * (dx * dx + dy * dy).sqrt();
    }

    0.5 * b.p.width as f64 * cross.abs() / chord_len
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
