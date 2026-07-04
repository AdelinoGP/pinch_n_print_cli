// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp
// (`WallToolPaths::simplifyToolPaths` / `Arachne::ExtrusionLine::simplify`).
//
// This file is an LLM-generated Rust port, adapted for the Pinch 'n Print
// architecture. **Intentional deviation**: OrcaSlicer's `simplify` is *not*
// classic recursive Douglas-Peucker — it is a single left-to-right
// Visvalingam-Whyatt-like sweep gated by an additional cross-section-area
// deviation check (`calculateExtrusionAreaDeviationError`). Packet 112's
// Track B spec explicitly directs this file to implement classic recursive
// Douglas-Peucker instead (a simpler, well-understood algorithm satisfying
// the same *behavioral* invariant — junction count strictly drops, width is
// preserved at every retained junction). Do NOT read this file as an
// OrcaSlicer-parity implementation of `simplifyToolPaths`; it is a
// deliberately simplified stand-in. See `docs/DEVIATION_LOG.md` conventions
// for how packet closure should record this gap if OrcaSlicer numeric parity
// is ever required here.
// -----------------------------------------------------------------------------
//! Packet 112 (Track B, T-226): Douglas-Peucker polyline simplification for
//! `ExtrusionLine`s, run per-line on the junction polyline in XY (millimeters,
//! matching `Point3WithWidth`'s coordinate unit — see
//! `crates/slicer-ir/src/slice_ir.rs::Point3WithWidth` doc comments).
//!
//! Each retained junction keeps its original `ExtrusionJunction` value
//! (width, flow_factor, overhang_quartile, perimeter_index) untouched — no
//! averaging or interpolation of width across a dropped run. A line is never
//! reduced below 2 junctions (both endpoints are always retained).

use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

/// Runs Douglas-Peucker simplification on every line's junction polyline.
///
/// `dp_epsilon` is the maximum perpendicular deviation (millimeters) a
/// dropped junction may have introduced; larger values simplify more
/// aggressively.
pub fn simplify_toolpaths(lines: Vec<ExtrusionLine>, dp_epsilon: f64) -> Vec<ExtrusionLine> {
    lines
        .into_iter()
        .map(|line| simplify_line(line, dp_epsilon))
        .collect()
}

fn simplify_line(line: ExtrusionLine, dp_epsilon: f64) -> ExtrusionLine {
    let ExtrusionLine {
        junctions,
        inset_idx,
        is_odd,
        is_closed,
    } = line;

    let n = junctions.len();
    if n <= 2 {
        // Never reduce below 2 junctions; nothing to simplify either way.
        return ExtrusionLine {
            junctions,
            inset_idx,
            is_odd,
            is_closed,
        };
    }

    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    dp_mark(&junctions, 0, n - 1, dp_epsilon * dp_epsilon, &mut keep);

    let simplified: Vec<ExtrusionJunction> = junctions
        .into_iter()
        .zip(keep)
        .filter_map(|(j, k)| k.then_some(j))
        .collect();

    ExtrusionLine {
        junctions: simplified,
        inset_idx,
        is_odd,
        is_closed,
    }
}

/// Recursive Douglas-Peucker marking pass over the index range `[lo, hi]`
/// (inclusive). Marks `keep[idx] = true` for the junction with the largest
/// perpendicular deviation from the `lo`-`hi` chord, if that deviation
/// exceeds `epsilon_sq`, then recurses into both halves.
fn dp_mark(
    junctions: &[ExtrusionJunction],
    lo: usize,
    hi: usize,
    epsilon_sq: f64,
    keep: &mut [bool],
) {
    if hi <= lo + 1 {
        return;
    }

    let start = junctions[lo].p;
    let end = junctions[hi].p;

    let mut max_d2 = 0.0f64;
    let mut split_idx = lo;
    for (i, junction) in junctions.iter().enumerate().take(hi).skip(lo + 1) {
        let d2 = perpendicular_dist_sq(junction.p, start, end);
        if d2 > max_d2 {
            max_d2 = d2;
            split_idx = i;
        }
    }

    if max_d2 > epsilon_sq {
        keep[split_idx] = true;
        dp_mark(junctions, lo, split_idx, epsilon_sq, keep);
        dp_mark(junctions, split_idx, hi, epsilon_sq, keep);
    }
}

/// Squared perpendicular XY distance from `p` to the line through `a`-`b`.
/// Falls back to squared point-to-point distance when `a == b` (degenerate
/// chord).
fn perpendicular_dist_sq(p: Point3WithWidth, a: Point3WithWidth, b: Point3WithWidth) -> f64 {
    let (ax, ay) = (a.x as f64, a.y as f64);
    let (bx, by) = (b.x as f64, b.y as f64);
    let (px, py) = (p.x as f64, p.y as f64);

    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-18 {
        let ddx = px - ax;
        let ddy = py - ay;
        return ddx * ddx + ddy * ddy;
    }

    let cross = dx * (py - ay) - dy * (px - ax);
    (cross * cross) / len_sq
}
