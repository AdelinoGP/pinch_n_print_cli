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

use slicer_ir::{ExtrusionJunction, ExtrusionLine};

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
/// Three-tier removal per interior junction:
/// 1. Ultra-short bypass (segment < 5µm = 0.005mm)
/// 2. Near-colinear fast path (height_2 ≤ 0.001 AND distance_to_infinite ≤
///    0.001 AND area_deviation ≤ maximum_extrusion_area_deviation)
/// 3. Primary gate (segment² < smallest_line_segment_squared AND
///    height_2 ≤ allowed_error_distance_squared)
fn simplify_distance_gated(
    junctions: &[ExtrusionJunction],
    smallest_line_segment_squared: f64,
    allowed_error_distance_squared: f64,
    maximum_extrusion_area_deviation: f64,
) -> Vec<ExtrusionJunction> {
    let n = junctions.len();
    if n <= 2 {
        return junctions.to_vec();
    }

    // Always retain the first junction.
    let mut result: Vec<ExtrusionJunction> = Vec::with_capacity(n);
    result.push(junctions[0].clone());

    // Track the last retained junction index for segment-length checks.
    let mut last_retained = 0;

    for i in 1..n - 1 {
        let prev = &junctions[last_retained];
        let curr = &junctions[i];
        let next = junctions.get(i + 1);

        let Some(next) = next else {
            // No next junction found — retain this one.
            result.push(curr.clone());
            last_retained = i;
            continue;
        };

        // Segment length squared (prev → curr).
        let seg_dx = (curr.p.x - prev.p.x) as f64;
        let seg_dy = (curr.p.y - prev.p.y) as f64;
        let seg_len_sq = seg_dx * seg_dx + seg_dy * seg_dy;

        // Perpendicular height squared from curr to chord (prev → next).
        let height_2 = point_line_distance_squared(prev, curr, next);

        // Tier 1: Ultra-short bypass (ExtrusionLine.cpp ~5µm).
        let ultra_short_threshold = 0.000025; // 0.005mm squared = 2.5e-5 mm²
        if seg_len_sq < ultra_short_threshold {
            // Remove: ultra-short segment.
            continue;
        }

        // Tier 2: Near-colinear fast path with area deviation guard.
        // Thresholds match OrcaSlicer ExtrusionLine.cpp's µm-scale constants
        // converted to mm²: 0.001² = 1e-6 mm² for height and inline distance.
        let near_colinear_height = 1e-6; // (0.001mm)² = 1µm²
        let near_colinear_inline = 1e-6; // (0.001mm)² = 1µm²
        let inline_dist = point_to_infinite_line_distance_squared(prev, next, curr);
        if height_2 <= near_colinear_height
            && inline_dist <= near_colinear_inline
            && maximum_extrusion_area_deviation > 0.0
        {
            let area_dev = calculate_extrusion_area_deviation_error(prev, curr, next);
            if area_dev <= maximum_extrusion_area_deviation {
                // Remove: near-colinear with acceptable area deviation.
                continue;
            }
        }

        // Tier 3: Primary distance gate.
        if seg_len_sq < smallest_line_segment_squared && height_2 <= allowed_error_distance_squared
        {
            // Remove: short segment within error tolerance.
            continue;
        }

        // Retain this junction.
        result.push(curr.clone());
        last_retained = i;
    }

    // Always retain the last junction.
    result.push(junctions[n - 1].clone());
    result
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
