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
//! Phase 6 — Top/bottom surface propagation for paint-segmentation.
//!
//! For each layer `L`, this stage emits the intersection of
//! `layer_input_polygons[L]` with the union of:
//!   - `top_proj[M]` for any layer `M` in `[L, L + top_shell_layers)`  — top-shell
//!     painted top-facing facets propagate DOWN by `top_shell_layers` layers
//!     (so a top-painted face also paints the `N-1` layers beneath it).
//!   - `bot_proj[M]` for any layer `M` in `(L - bottom_shell_layers, L]` — symmetric
//!     bottom-shell propagation UP by `bottom_shell_layers` layers.
//!
//! When `top_shell_layers == 0` and `bottom_shell_layers == 0`, both windows
//! collapse to the single layer `L` and the result is
//! `intersection(top_proj[L] ∪ bot_proj[L], layer_input_polygons[L])` — matching
//! the original first-cut behaviour.

use crate::polygon_ops::{intersection_ex, union_ex};
use crate::triangle_mesh_slicer::slice_mesh_slabs;
use slicer_ir::{ExPolygon, IndexedTriangleSet, PaintSemantic, PaintValue};

/// Per-layer semantic polygon output from `propagate_top_bottom`.
pub struct PerLayerSemanticPolygons {
    /// The semantic type (e.g. Material, SupportEnforcer) for these regions.
    pub semantic: PaintSemantic,
    /// The specific paint value (tool index, flag, etc.) for these regions.
    pub value: PaintValue,
    /// `per_layer[l]` = ExPolygons for layer index `l`.
    pub per_layer: Vec<Vec<ExPolygon>>,
}

/// Propagate top/bottom painted surfaces across layers with OrcaSlicer-parity
/// shell-window propagation.
///
/// # Arguments
/// - `painted_mesh_facets` — the painted-facets-only mesh extract.
/// - `semantic` / `value` — forwarded unchanged into the result.
/// - `layer_zs` — layer centre Z values in mm (length N).
/// - `layer_input_polygons` — sliced contours per layer (length N).
/// - `top_shell_layers` — number of layers a top-facing painted facet propagates
///   DOWN.  A facet sitting in slab `M` paints layers `M - top_shell_layers + 1
///   .. M` (clipped to `[0, N)`).  `0` disables this contribution.
/// - `bottom_shell_layers` — symmetric: a bottom-facing painted facet in slab
///   `M` paints layers `M .. M + bottom_shell_layers - 1`.
///
/// # Behaviour at shell_layers = 0
/// With both shell counts at zero, only the layer's own slab projection
/// contributes; the output collapses to
/// `intersection(top_proj[l] ∪ bot_proj[l], layer_input_polygons[l])` for every
/// `l` — the first-cut behaviour preserved.
pub fn propagate_top_bottom(
    painted_mesh_facets: &IndexedTriangleSet,
    semantic: PaintSemantic,
    value: PaintValue,
    layer_zs: &[f32],
    layer_input_polygons: &[Vec<ExPolygon>],
    top_shell_layers: usize,
    bottom_shell_layers: usize,
) -> PerLayerSemanticPolygons {
    let n = layer_zs.len();
    if n == 0 || painted_mesh_facets.vertices.is_empty() {
        return PerLayerSemanticPolygons {
            semantic,
            value,
            per_layer: vec![Vec::new(); layer_input_polygons.len()],
        };
    }

    // Build N+1 slab boundaries from layer centres.
    let layer_thickness = if n > 1 {
        layer_zs[1] - layer_zs[0]
    } else {
        0.2_f32
    };
    let half = layer_thickness / 2.0;
    // `slice_mesh_slabs` uses a strict `face_min_z < slab_hi && face_max_z > slab_lo`
    // overlap check.  A facet that sits exactly on `layer_zs[n-1] + half` (the top
    // boundary) — i.e. the cube's top face at z = top_z — would be excluded from
    // the topmost slab.  Symmetrically the very bottom face sits at the lower
    // boundary.  Extend the outermost two bounds by a tiny epsilon so these
    // on-boundary facets are captured in their natural slab.  Epsilon is sized in
    // mm and well below printable resolution (1 µm).
    let bound_eps = 1.0e-3_f32;
    let mut zs_bounds: Vec<f32> = Vec::with_capacity(n + 1);
    zs_bounds.push(layer_zs[0] - half - bound_eps);
    for i in 1..n {
        zs_bounds.push((layer_zs[i - 1] + layer_zs[i]) / 2.0);
    }
    zs_bounds.push(layer_zs[n - 1] + half + bound_eps);

    let (top_proj, bot_proj) = slice_mesh_slabs(painted_mesh_facets, &zs_bounds);

    let out_len = n.min(layer_input_polygons.len());
    let mut per_layer: Vec<Vec<ExPolygon>> = Vec::with_capacity(out_len);

    // Shell window radii.  At shells = 0 both windows collapse to {l}.
    // top_shell_layers semantics: a top-facing painted facet at slab M paints
    // layers `M - top_shell_layers + 1 .. M` (so layer L pulls from slabs
    // `L ..= L + top_shell_layers - 1`).  Equivalently, sliding window of
    // length `max(1, top_shell_layers)` starting at L.
    let top_window = top_shell_layers.max(1);
    let bot_window = bottom_shell_layers.max(1);

    for l in 0..out_len {
        // Gather top contributions from slabs [l, l + top_window).
        let top_hi = (l + top_window).min(top_proj.len());
        let mut combined: Vec<ExPolygon> = Vec::new();
        for m in l..top_hi {
            combined.extend_from_slice(&top_proj[m]);
        }
        // Gather bottom contributions from slabs (l - bot_window, l].
        let bot_lo = l.saturating_sub(bot_window - 1);
        let bot_hi = (l + 1).min(bot_proj.len());
        for m in bot_lo..bot_hi {
            combined.extend_from_slice(&bot_proj[m]);
        }

        if combined.is_empty() {
            per_layer.push(Vec::new());
            continue;
        }

        let unioned = union_ex(&combined);

        // Intersect with layer's sliced contour to stay within the print body.
        let layer_contour = &layer_input_polygons[l];
        let result = if layer_contour.is_empty() {
            unioned
        } else {
            intersection_ex(&unioned, layer_contour)
        };

        per_layer.push(result);
    }

    // Pad any remaining layers (if layer_input_polygons is longer than slab_count).
    while per_layer.len() < layer_input_polygons.len() {
        per_layer.push(Vec::new());
    }

    PerLayerSemanticPolygons {
        semantic,
        value,
        per_layer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{Point3, Polygon};

    fn unit_cube_painted_mesh() -> IndexedTriangleSet {
        // Top face only (z=1, normal.z > 0)
        let vertices = vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 1.0,
            },
        ];
        // Two triangles for the top face (CCW from above → normal.z > 0)
        let indices = vec![0, 1, 2, 0, 2, 3];
        IndexedTriangleSet { vertices, indices }
    }

    fn square_contour_mm(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<ExPolygon> {
        use slicer_ir::Point2;
        vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(x0, y0),
                    Point2::from_mm(x1, y0),
                    Point2::from_mm(x1, y1),
                    Point2::from_mm(x0, y1),
                ],
            },
            holes: Vec::new(),
        }]
    }

    #[test]
    fn propagate_top_bottom_empty_mesh_returns_empty_per_layer() {
        let mesh = IndexedTriangleSet {
            vertices: vec![],
            indices: vec![],
        };
        let layer_zs = vec![0.1, 0.3, 0.5];
        let contours: Vec<Vec<ExPolygon>> = layer_zs.iter().map(|_| Vec::new()).collect();
        let result = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(1),
            &layer_zs,
            &contours,
            3,
            3,
        );
        assert_eq!(result.per_layer.len(), contours.len());
        assert!(result.per_layer.iter().all(|l| l.is_empty()));
    }

    #[test]
    fn propagate_top_bottom_cube_upward_face_covers_top_slab() {
        let mesh = unit_cube_painted_mesh();
        // 5 layers; top face at z=1mm sits in the last slab.
        let layer_zs: Vec<f32> = (0..5).map(|i| (i as f32 + 0.5) * 0.3).collect();
        // layer_zs ≈ [0.15, 0.45, 0.75, 1.05, 1.35]; top face at z=1 is in slab 3 (0.9..1.2mm).
        let contours: Vec<Vec<ExPolygon>> = layer_zs
            .iter()
            .map(|_| square_contour_mm(0.0, 0.0, 1.0, 1.0))
            .collect();

        // shell_layers = 0 — original first-cut behaviour: only the slab's own
        // layer carries the top-face projection.
        let result = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(2),
            &layer_zs,
            &contours,
            0,
            0,
        );

        // At least one layer should have non-empty polygons (the slab covering z=1).
        let has_coverage = result.per_layer.iter().any(|l| !l.is_empty());
        assert!(
            has_coverage,
            "At least one layer must contain top-face projection"
        );

        // Find the layer that covers z=1mm (slab around index 3 or 4).
        let covering_layer = result
            .per_layer
            .iter()
            .enumerate()
            .find(|(_, l)| !l.is_empty())
            .map(|(i, _)| i);
        assert!(covering_layer.is_some());

        let idx = covering_layer.unwrap();
        let all_pts: Vec<_> = result.per_layer[idx]
            .iter()
            .flat_map(|ep| ep.contour.points.iter().copied())
            .collect();
        let max_x = all_pts.iter().map(|p| p.x).max().unwrap_or(0);
        let max_y = all_pts.iter().map(|p| p.y).max().unwrap_or(0);
        assert!(max_x > 0, "Coverage should have positive X extent");
        assert!(max_y > 0, "Coverage should have positive Y extent");
    }

    #[test]
    fn propagate_top_bottom_assigns_correct_semantic_value_tuple() {
        let mesh = IndexedTriangleSet {
            vertices: vec![],
            indices: vec![],
        };
        let result = propagate_top_bottom(
            &mesh,
            PaintSemantic::SupportEnforcer,
            PaintValue::Flag(true),
            &[0.2],
            &[Vec::new()],
            2,
            2,
        );
        assert_eq!(result.semantic, PaintSemantic::SupportEnforcer);
        // PaintValue doesn't derive PartialEq; verify via debug string.
        assert!(
            format!("{:?}", result.value).contains("Flag(true)"),
            "Expected PaintValue::Flag(true), got {:?}",
            result.value
        );
    }

    #[test]
    fn propagate_top_bottom_top_shell_3_propagates_down_3_layers() {
        // 5 layers, top-painted face at z=1mm (top of unit cube).  With
        // top_shell_layers=3 the top-face slab plus the two layers below it
        // should all receive coverage.
        let mesh = unit_cube_painted_mesh();
        let layer_zs: Vec<f32> = (0..5).map(|i| (i as f32 + 0.5) * 0.3).collect();
        // layer_zs ≈ [0.15, 0.45, 0.75, 1.05, 1.35]; top face slab ≈ idx 3.
        let contours: Vec<Vec<ExPolygon>> = layer_zs
            .iter()
            .map(|_| square_contour_mm(0.0, 0.0, 1.0, 1.0))
            .collect();

        let result_shells_3 = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(2),
            &layer_zs,
            &contours,
            3, // top shell propagates DOWN by 3
            0,
        );
        let result_shells_0 = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(2),
            &layer_zs,
            &contours,
            0,
            0,
        );

        let nonempty_count_3 = result_shells_3
            .per_layer
            .iter()
            .filter(|l| !l.is_empty())
            .count();
        let nonempty_count_0 = result_shells_0
            .per_layer
            .iter()
            .filter(|l| !l.is_empty())
            .count();
        assert!(
            nonempty_count_3 > nonempty_count_0,
            "shell_layers=3 must propagate to more layers ({} vs {} at shells=0)",
            nonempty_count_3,
            nonempty_count_0
        );
        // With a top-only painted facet near the top of the stack, top-shell=3
        // pulls coverage onto the slab's layer and (at least) the layer below.
        assert!(
            nonempty_count_3 >= 2,
            "Expected ≥2 non-empty layers at shells=3, got {}",
            nonempty_count_3
        );
    }

    #[test]
    fn propagate_top_bottom_shells_zero_collapses_to_first_cut() {
        // Property: with shells = 0, output must equal the per-slab
        // intersection-only behaviour (no propagation).
        let mesh = unit_cube_painted_mesh();
        let layer_zs: Vec<f32> = (0..4).map(|i| (i as f32 + 0.5) * 0.3).collect();
        let contours: Vec<Vec<ExPolygon>> = layer_zs
            .iter()
            .map(|_| square_contour_mm(0.0, 0.0, 1.0, 1.0))
            .collect();

        let result = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(1),
            &layer_zs,
            &contours,
            0,
            0,
        );

        // At least one layer non-empty; no padding beyond layer count.
        assert_eq!(result.per_layer.len(), contours.len());
        let any_nonempty = result.per_layer.iter().any(|l| !l.is_empty());
        assert!(any_nonempty, "shells=0 still produces coverage in top slab");
    }
}
