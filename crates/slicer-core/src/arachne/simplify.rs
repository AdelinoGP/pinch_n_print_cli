// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp
// (`WallToolPaths::simplifyToolPaths` / `Arachne::ExtrusionLine::simplify`)
// and src/libslic3r/Arachne/utils/ExtrusionLine.cpp
// (`calculateExtrusionAreaDeviationError`).
//
// This file is an LLM-generated Rust port, adapted for the Pinch 'n Print
// architecture. It implements the Visvalingam-Whyatt-like area-based sweep
// gated by `calculateExtrusionAreaDeviationError` described in the OrcaSlicer
// source, closing the D-112-SIMPLIFY-DP deviation.
// -----------------------------------------------------------------------------
//! Packet 113a (Step 1): Visvalingam-Whyatt polyline simplification for
//! `ExtrusionLine`s, run per-line on the junction polyline in XY (millimeters,
//! matching `Point3WithWidth`'s coordinate unit — see
//! `crates/slicer-ir/src/slice_ir.rs::Point3WithWidth` doc comments).
//!
//! Each retained junction keeps its original `ExtrusionJunction` value
//! (width, flow_factor, overhang_quartile, perimeter_index) untouched — no
//! averaging or interpolation of width across a dropped run. A line is never
//! reduced below 2 junctions (both endpoints are always retained).

use slicer_ir::{ExtrusionJunction, ExtrusionLine};

/// Runs Visvalingam-Whyatt simplification on every line's junction polyline.
///
/// `visvalingam_area_threshold` is the maximum width-weighted area deviation
/// (mm²) a dropped junction may introduce; larger values simplify more
/// aggressively.
pub fn simplify_toolpaths(
    lines: Vec<ExtrusionLine>,
    visvalingam_area_threshold: f64,
) -> Vec<ExtrusionLine> {
    lines
        .into_iter()
        .map(|line| simplify_line(line, visvalingam_area_threshold))
        .collect()
}

fn simplify_line(line: ExtrusionLine, visvalingam_area_threshold: f64) -> ExtrusionLine {
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

    let mut keep: Vec<bool> = vec![true; n];

    // Repeatedly scan left-to-right and drop the first interior junction whose
    // removal does not exceed the width-weighted area threshold. This matches
    // OrcaSlicer's single-pass Visvalingam-like sweep over each
    // `ExtrusionLine`.
    loop {
        let mut removed = false;
        let mut i = 1;
        while i < n - 1 {
            if !keep[i] {
                i += 1;
                continue;
            }

            // Find the previous/next still-kept neighbours of `i`.
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

/// Width-weighted area deviation of the middle junction `b` relative to the
/// chord `a`–`c`.
///
/// Formula (OrcaSlicer `ExtrusionLine.cpp:248`):
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
        // Degenerate chord: fall back to a point-to-point width-weighted area.
        let dx = bx - ax;
        let dy = by - ay;
        return 0.5 * b.p.width as f64 * (dx * dx + dy * dy).sqrt();
    }

    0.5 * b.p.width as f64 * cross.abs() / chord_len
}
