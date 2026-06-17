// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
/// Painted line with semantic value and spatial cell membership.
use crate::algos::paint_segmentation::triangle_intersect::Line;
use slicer_ir::{PaintSemantic, PaintValue};

/// A line segment carrying paint-semantic data and spatial-cell membership.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintedLine {
    /// The 2D line segment.
    pub line: Line,
    /// The paint semantic family this line belongs to.
    pub semantic: PaintSemantic,
    /// The paint value associated with this line.
    pub value: PaintValue,
    /// Spatial grid cell indices this line overlaps.
    pub cell_indices: Vec<usize>,
    /// Index of the contour this painted line was projected onto.
    pub contour_idx: usize,
    /// Index of the edge within the contour this painted line covers.
    pub line_idx: usize,
    /// The projected segment on the contour edge (parameterised in contour-edge space).
    pub projected_line: Line,
}

/// Visitor that collects `PaintedLine` records.
#[derive(Debug, Default)]
pub struct PaintedLineVisitor {
    /// Collected painted lines.
    pub lines: Vec<PaintedLine>,
}

impl PaintedLineVisitor {
    /// Create a new empty visitor.
    pub fn new() -> Self {
        Self::default()
    }
    /// Append a painted line to the visitor.
    pub fn push(&mut self, line: PaintedLine) {
        self.lines.push(line);
    }
}
