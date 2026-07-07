// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp
// (`WallToolPaths::separateOutInnerContour`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Packet 146 (N11): separates zero-width "inner contour" marker lines from
//! printable toolpaths. Lines with zero extrusion width at their first
//! junction are contour-only markers that define the boundary between wall
//! and infill regions (the "inner contour"). These are extracted for infill
//! boundary bookkeeping; printable paths (first junction width > 0) remain
//! in the toolpaths vector.

use slicer_ir::ExtrusionLine;

/// Separates inner-contour marker lines from printable toolpaths.
///
/// Lines whose first junction has zero extrusion width are inner-contour
/// markers (wall/infill boundary bookkeeping). They are returned in the
/// second vector; all other lines remain in the first.
///
/// Matches `WallToolPaths::separateOutInnerContour` (line 685): the
/// zero-width check inspects only the *first* junction of each line.
pub fn separate_out_inner_contour(
    lines: Vec<ExtrusionLine>,
) -> (Vec<ExtrusionLine>, Vec<ExtrusionLine>) {
    let mut toolpaths = Vec::with_capacity(lines.len());
    let mut inner_contour = Vec::new();

    for line in lines {
        let is_contour_marker = line.junctions.first().map_or(false, |j| j.p.width == 0.0);

        if is_contour_marker {
            inner_contour.push(line);
        } else {
            toolpaths.push(line);
        }
    }

    (toolpaths, inner_contour)
}

/// Filters out `ExtrusionLine`s with empty `junctions` vectors.
///
/// Matches `WallToolPaths::removeEmptyToolPaths` (line 689): a cleanup step
/// after `separateOutInnerContour` may have drained some insets.
pub fn remove_empty_toolpaths(lines: Vec<ExtrusionLine>) -> Vec<ExtrusionLine> {
    lines
        .into_iter()
        .filter(|l| !l.junctions.is_empty())
        .collect()
}
