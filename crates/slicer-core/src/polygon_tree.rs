// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/ExPolygon.cpp (ClipperUtils)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Hole/contour containment tree for sliced polygon layers.
//!
//! Given a flat list of [`ExPolygon`]s (as produced by the mesh slicer),
//! [`build_polygon_tree`] assembles them into a **containment forest** where
//! nesting depth determines whether each node is a filled contour (even depth)
//! or a hole (odd depth) — mirroring the even-odd rule used by OrcaSlicer's
//! `make_expolygons`.
//!
//! # Coordinate units
//! 1 unit = 100 nm.  All arithmetic operates on raw integer coordinates;
//! no mm conversion is needed for the containment test.

use slicer_ir::{point_in_polygon_winding, ExPolygon};

/// A node in the containment forest produced by [`build_polygon_tree`].
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonTreeNode {
    /// Index into the original `polygons` slice passed to [`build_polygon_tree`].
    pub polygon_index: u32,
    /// `true` if this node is at an even nesting depth (filled contour),
    /// `false` if it is at an odd nesting depth (hole).
    pub is_contour: bool,
    /// Immediate children, ordered by **ascending `polygon_index`**.
    pub children: Vec<PolygonTreeNode>,
}

/// Build a containment forest from a flat list of [`ExPolygon`]s.
///
/// Each polygon's contour is tested against every other polygon's contour to
/// determine containment.  Each polygon is assigned to its **immediate parent**
/// — the smallest-area polygon that fully contains it — so multi-level nesting
/// attaches to the direct parent rather than an ancestor.
///
/// **`is_contour`** is determined by nesting depth parity:
/// - depth 0 (root) → `true`  (contour)
/// - depth 1        → `false` (hole)
/// - depth 2        → `true`  (island inside a hole)
/// - …alternating.
///
/// Roots and siblings within each parent are ordered by **ascending
/// `polygon_index`** for deterministic output.
pub fn build_polygon_tree(polygons: &[ExPolygon]) -> Vec<PolygonTreeNode> {
    let n = polygons.len();
    if n == 0 {
        return Vec::new();
    }

    // Pre-compute signed areas so we can find the smallest containing parent
    // without recomputing inside the inner loop.
    let areas: Vec<f64> = polygons
        .iter()
        .map(|p| contour_area_abs(&p.contour.points))
        .collect();

    // For each polygon i, find its immediate parent: the smallest-area polygon j
    // such that polygon i is contained in polygon j (and j ≠ i).
    //
    // "Contained" is tested by checking whether the *first vertex* of polygon i's
    // contour lies inside polygon j's contour (winding-number test, eps = 0.0).
    // This matches the OrcaSlicer single-point containment heuristic.
    let mut parent: Vec<Option<usize>> = vec![None; n];

    for i in 0..n {
        // First vertex of polygon i's contour (in mm for the winding test).
        let pts_i = &polygons[i].contour.points;
        if pts_i.is_empty() {
            continue;
        }
        let (px_mm, py_mm) = units_to_mm_pair(pts_i[0].x, pts_i[0].y);

        let mut best_parent: Option<usize> = None;
        let mut best_area = f64::INFINITY;

        for j in 0..n {
            if i == j {
                continue;
            }
            // Does polygon j contain polygon i?
            if point_in_polygon_winding(&polygons[j], px_mm, py_mm, 0.0) {
                let aj = areas[j];
                if aj < best_area {
                    best_area = aj;
                    best_parent = Some(j);
                }
            }
        }
        parent[i] = best_parent;
    }

    // Build a child-list for each node (roots have no parent).
    // Children within each parent are ordered by ascending polygon_index (CONTRACT).
    let mut children_of: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut roots: Vec<usize> = Vec::new();

    for i in 0..n {
        match parent[i] {
            Some(p) => children_of[p].push(i),
            None => roots.push(i),
        }
    }

    // Sort roots and all child lists by ascending polygon_index (which equals
    // the index into the original slice).
    roots.sort_unstable();
    for list in children_of.iter_mut() {
        list.sort_unstable();
    }

    // Recursively build the tree nodes.
    roots
        .into_iter()
        .map(|i| build_node(i, 0, &children_of))
        .collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Recursively construct a [`PolygonTreeNode`] for polygon `idx` at the given
/// nesting `depth`.
fn build_node(idx: usize, depth: u32, children_of: &[Vec<usize>]) -> PolygonTreeNode {
    let is_contour = depth.is_multiple_of(2);
    let children = children_of[idx]
        .iter()
        .map(|&child_idx| build_node(child_idx, depth + 1, children_of))
        .collect();
    PolygonTreeNode {
        polygon_index: idx as u32,
        is_contour,
        children,
    }
}

/// Absolute value of the shoelace area of a polygon contour.
///
/// Operates entirely in raw integer units (no mm conversion needed for
/// area-comparison purposes — relative ordering is preserved).
fn contour_area_abs(pts: &[slicer_ir::Point2]) -> f64 {
    if pts.len() < 3 {
        return 0.0;
    }
    let n = pts.len();
    let mut area: i128 = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += (pts[i].x as i128) * (pts[j].y as i128);
        area -= (pts[j].x as i128) * (pts[i].y as i128);
    }
    (area.unsigned_abs() as f64) * 0.5
}

/// Convert a pair of integer-unit coordinates to millimetres.
/// 1 unit = 100 nm = 10⁻⁴ mm  →  divide by 10_000.
#[inline]
fn units_to_mm_pair(x: i64, y: i64) -> (f64, f64) {
    (x as f64 / 10_000.0, y as f64 / 10_000.0)
}
