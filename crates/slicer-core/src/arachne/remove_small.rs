// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp
// (`WallToolPaths::removeSmallLines`, helper `shorterThan<T>`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Packet 146 (N12): drops degenerate odd, non-closed `ExtrusionLine`s whose
//! XY polyline length falls below a per-line threshold derived from the
//! minimum junction width along that line.
//!
//! Canonical behaviour (WallToolPaths.cpp:838-856):
//! - `min_width` per line = minimum junction width over the line's junctions.
//! - On top/bottom layers (`is_initial_layer || is_top_or_bottom_layer`):
//!   threshold = `min_width / 2`.
//! - On other layers: threshold = `min_width * min_length_factor`.
//!
//! # Invariants (checked before the length computation)
//!
//! - Closed lines are never removed — this covers both the primary
//!   (`inset_idx == 0`) outer-wall contour and any other closed loop.
//! - Even (`is_odd == false`) lines are never removed, closed or not.
//!
//! Only lines with `is_odd == true && is_closed == false` are ever eligible
//! for removal, matching OrcaSlicer's `removeSmallLines` eligibility gate.

use slicer_ir::ExtrusionLine;

/// Removes odd, non-closed `ExtrusionLine`s shorter than a per-line threshold.
///
/// The threshold is computed per line from the minimum junction width along
/// that line:
/// - Top/bottom layers (`is_initial_layer == true` or
///   `is_top_or_bottom_layer == true`): `min_junction_width / 2`
///   (conservative — prevents top/bottom gaps, matching WallToolPaths.cpp:848).
/// - Other layers: `min_junction_width * min_length_factor`.
///
/// `min_length_factor` is the configurable multiplier (typically 0.5,
/// matching `docs/15_config_keys_reference.md`).
pub fn remove_small_lines(
    lines: Vec<ExtrusionLine>,
    min_length_factor: f64,
    _min_width: f64,
    is_initial_layer: bool,
    is_top_or_bottom_layer: bool,
) -> Vec<ExtrusionLine> {
    lines
        .into_iter()
        .filter(|line| {
            !should_remove(
                line,
                min_length_factor,
                is_initial_layer,
                is_top_or_bottom_layer,
            )
        })
        .collect()
}

/// Preserve conditions are checked first, before any length computation:
/// closed lines and even lines are never eligible for removal.
fn should_remove(
    line: &ExtrusionLine,
    min_length_factor: f64,
    is_initial_layer: bool,
    is_top_or_bottom_layer: bool,
) -> bool {
    if line.is_closed || !line.is_odd {
        return false;
    }

    // Per-line min_width: minimum junction width over the line.
    // WallToolPaths.cpp:840-845 — iterate all junctions, take minimum width.
    let min_width = line
        .junctions
        .iter()
        .map(|j| j.p.width as f64)
        .fold(f64::INFINITY, f64::min);

    if !min_width.is_finite() || min_width <= 0.0 {
        // No junctions or all zero-width: treat as degenerate, remove.
        return true;
    }

    // WallToolPaths.cpp:848-854 — layer-type divisor.
    let threshold = if is_initial_layer || is_top_or_bottom_layer {
        min_width / 2.0
    } else {
        min_width * min_length_factor
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ExtrusionJunction, Point3WithWidth};

    fn short_odd_line(x_end: f32, width: f32) -> ExtrusionLine {
        let junction = |x: f32| ExtrusionJunction {
            p: Point3WithWidth {
                x,
                y: 0.0,
                z: 0.0,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            perimeter_index: 0,
        };
        ExtrusionLine {
            junctions: vec![junction(0.0), junction(x_end)],
            inset_idx: 0,
            is_odd: true,
            is_closed: false,
        }
    }

    /// AC-N1 lock: on a mid-stack layer (neither initial nor top/bottom),
    /// short odd unclosed lines must still be strictly dropped even when the
    /// per-line width is well above the lenient top/bottom divisor.
    #[test]
    fn non_top_layer_strict() {
        let lines = vec![short_odd_line(3.0, 0.4), short_odd_line(4.0, 0.4)];

        let surviving = remove_small_lines(
            lines, 20.0,  // min_length_factor → strict threshold = 8 mm
            0.4,   // nominal min_width (unused by the per-line threshold)
            false, // is_initial_layer
            false, // is_top_or_bottom_layer
        );

        assert!(
            surviving.is_empty(),
            "mid-stack strict removal must drop short odd unclosed lines"
        );
    }
}
