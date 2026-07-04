// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp
// (`WallToolPaths::removeSmallLines`, helper `shorterThan<T>`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture. **Deviation**: OrcaSlicer
// derives `min_width` per-line (the minimum junction width along that line)
// and additionally branches on "is top/bottom layer" for the length
// divisor. Packet 112 Track B's spec fixes the signature to a single
// caller-supplied `min_width: f64` (no per-line derivation, no layer-type
// branch) — see this packet's `packet.spec.md` for the simplified contract.
// -----------------------------------------------------------------------------
//! Packet 112 (Track B, T-227): drops degenerate odd, non-closed
//! `ExtrusionLine`s (bead-parity transition slivers) whose XY polyline
//! length falls below `min_length_factor * min_width` (millimeters, matching
//! `Point3WithWidth`'s coordinate unit).
//!
//! # Invariants (checked before the length computation)
//!
//! - Closed lines are never removed — this covers both the primary
//!   (`inset_idx == 0`) outer-wall contour and any other closed loop
//!   (even-inset regular walls, small closed odd-inset fill loops).
//! - Even (`is_odd == false`) lines are never removed, closed or not.
//!
//! Only lines with `is_odd == true && is_closed == false` are ever eligible
//! for removal, matching OrcaSlicer's `removeSmallLines` eligibility gate.

use slicer_ir::ExtrusionLine;

/// Removes odd, non-closed `ExtrusionLine`s shorter than
/// `min_length_factor * min_width`.
///
/// `min_width` is a caller-supplied nominal width (millimeters); this is
/// a simplified variant of OrcaSlicer's per-line-derived `min_width` (see
/// module doc-comment deviation note).
pub fn remove_small_lines(
    lines: Vec<ExtrusionLine>,
    min_length_factor: f64,
    min_width: f64,
) -> Vec<ExtrusionLine> {
    let threshold = min_length_factor * min_width;
    lines
        .into_iter()
        .filter(|line| !should_remove(line, threshold))
        .collect()
}

/// Preserve conditions are checked first, before any length computation:
/// closed lines (regardless of inset parity — this covers both the primary
/// `inset_idx == 0` contour and closed even-inset lines) and even lines are
/// never eligible for removal.
fn should_remove(line: &ExtrusionLine, threshold: f64) -> bool {
    if line.is_closed || !line.is_odd {
        return false;
    }

    polyline_length_xy(line) < threshold
}

/// XY-only polyline length (Arachne toolpaths are a per-layer 2D construct;
/// matches the XY-only convention used by `ExtrusionPath3D::is_closed`).
fn polyline_length_xy(line: &ExtrusionLine) -> f64 {
    line.junctions
        .windows(2)
        .map(|w| {
            let a = w[0].p;
            let b = w[1].p;
            let dx = (b.x - a.x) as f64;
            let dy = (b.y - a.y) as f64;
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}
