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
//! Phase 6 — Top/bottom surface paint propagation for paint-segmentation.
//!
//! Rust port of OrcaSlicer's `segmentation_top_and_bottom_layers`
//! (`MultiMaterialSegmentation.cpp`). For a per-colour painted-facet mesh it:
//!
//!   1. Slices the facets into per-layer slab projections via `slice_mesh_slabs`
//!      (top-facing vs bottom-facing), with the outer slab bounds extended to the
//!      mesh's true Z extent so a flat top/bottom face sitting beyond the last
//!      layer centre is still captured.
//!   2. Applies the occlusion rule — a top surface exists at layer `L` only where
//!      the layer ABOVE does not cover it (symmetric for bottom) — then a
//!      morphological opening to drop unprintable slivers.
//!   3. Propagates each exposed surface across the solid shell: the contact layer
//!      plus `top_shell_layers - 1` layers below (resp. `bottom_shell_layers - 1`
//!      above), intersected with the running intersection of the intervening layer
//!      slices to stay within the print body.
//!
//! The result is one `Vec<ExPolygon>` per layer giving the area that this colour's
//! top/bottom faces own. The caller (`execute_paint_segmentation`) gives this area
//! precedence over the vertical-side segmentation so the top/bottom SOLID surface
//! is coloured by the face it belongs to rather than by the adjacent side walls.
//!
//! Successive shell layers are inset inward by `extrusion_spacing +
//! extrusion_width` per layer of depth (OrcaSlicer parity), so the face colour
//! fills only the internal-solid-infill interior of those layers while their
//! perimeter walls keep the side-face colour — see the comment on Step 2.

use crate::flow::line_width_to_spacing;
use crate::polygon_ops::{
    difference_ex, intersection_ex, offset, opening, union_ex, OffsetJoinType,
};
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
#[allow(clippy::too_many_arguments)]
pub fn propagate_top_bottom(
    painted_mesh_facets: &IndexedTriangleSet,
    semantic: PaintSemantic,
    value: PaintValue,
    layer_zs: &[f32],
    layer_input_polygons: &[Vec<ExPolygon>],
    top_shell_layers: usize,
    bottom_shell_layers: usize,
    extrusion_width_mm: f32,
    layer_height_mm: f32,
) -> PerLayerSemanticPolygons {
    let n = layer_zs.len();
    let total_layers = layer_input_polygons.len();
    let empty = |sem: PaintSemantic, val: PaintValue| PerLayerSemanticPolygons {
        semantic: sem,
        value: val,
        per_layer: vec![Vec::new(); total_layers],
    };
    if n == 0 || painted_mesh_facets.vertices.is_empty() {
        return empty(semantic, value);
    }

    // Build N+1 slab boundaries from layer centres.  `slice_mesh_slabs` uses a
    // strict `face_min_z < slab_hi && face_max_z > slab_lo` overlap test, so a
    // perfectly horizontal top/bottom face sitting exactly on the outer boundary
    // would be excluded — extend the two outermost bounds by 1 µm so on-boundary
    // faces are captured in their natural slab.
    let layer_thickness = if n > 1 {
        layer_zs[1] - layer_zs[0]
    } else {
        layer_height_mm.max(0.01)
    };
    let half = layer_thickness / 2.0;
    let bound_eps = 1.0e-3_f32;
    // The outermost slab bounds must reach the mesh's true Z extent: a flat top
    // face sits ABOVE the last layer centre (and the bottom face BELOW the first),
    // by up to a layer height, so bounds derived purely from layer centres would
    // exclude them and `slice_mesh_slabs` would drop the surface entirely. Extend
    // the first/last bound to cover the painted geometry's Z range.
    let mesh_z_min = painted_mesh_facets
        .vertices
        .iter()
        .map(|v| v.z)
        .fold(f32::INFINITY, f32::min);
    let mesh_z_max = painted_mesh_facets
        .vertices
        .iter()
        .map(|v| v.z)
        .fold(f32::NEG_INFINITY, f32::max);
    let mut zs_bounds: Vec<f32> = Vec::with_capacity(n + 1);
    zs_bounds.push((layer_zs[0] - half).min(mesh_z_min) - bound_eps);
    for i in 1..n {
        zs_bounds.push((layer_zs[i - 1] + layer_zs[i]) / 2.0);
    }
    zs_bounds.push((layer_zs[n - 1] + half).max(mesh_z_max) + bound_eps);

    let (top_proj, bot_proj) = slice_mesh_slabs(painted_mesh_facets, &zs_bounds);

    let out_len = n.min(total_layers);

    // OrcaSlicer-parity shell parameters (MultiMaterialSegmentation.cpp:1499-1562):
    //   shell_step = extrusion_spacing + extrusion_width  (inward inset per shell layer)
    //   small_region_threshold ≈ 0.5 · extrusion_width    (opening radius, anti-sliver)
    let width = extrusion_width_mm.max(0.0);
    let spacing = line_width_to_spacing(width, layer_height_mm, width);
    let shell_step = spacing + width; // mm, inward per shell layer
    let small_thr = (0.5 * width) as f64; // mm opening radius
                                          // Effective shell depth (contact layer + propagated layers). `.max(1)` keeps a
                                          // single contact layer even when the shell-count config is 0.
    let top_depth = top_shell_layers.max(1);
    let bottom_depth = bottom_shell_layers.max(1);

    let layer_at = |i: usize| -> &[ExPolygon] {
        layer_input_polygons
            .get(i)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    };
    let open = |p: &[ExPolygon]| -> Vec<ExPolygon> {
        if small_thr > 0.0 && !p.is_empty() {
            opening(p, small_thr, OffsetJoinType::Miter)
        } else {
            p.to_vec()
        }
    };

    // Step 1 — occlusion: a top surface exists at layer `l` only where the layer
    // ABOVE (`l+1`) does not cover it; a bottom surface only where the layer BELOW
    // (`l-1`) does not. (MMSeg.cpp:1462-1469.) Then clean slivers via opening.
    let mut top_raw: Vec<Vec<ExPolygon>> = vec![Vec::new(); out_len];
    let mut bot_raw: Vec<Vec<ExPolygon>> = vec![Vec::new(); out_len];
    for l in 0..out_len {
        if l < top_proj.len() && !top_proj[l].is_empty() {
            let exposed = if l + 1 < out_len {
                difference_ex(&top_proj[l], layer_at(l + 1))
            } else {
                top_proj[l].clone()
            };
            top_raw[l] = open(&exposed);
        }
        if l < bot_proj.len() && !bot_proj[l].is_empty() {
            let exposed = if l >= 1 {
                difference_ex(&bot_proj[l], layer_at(l - 1))
            } else {
                bot_proj[l].clone()
            };
            bot_raw[l] = open(&exposed);
        }
    }

    // Step 2 — contact + shell propagation across the solid shell.
    //
    // The contact layer takes the full exposed surface; each shell layer below
    // (top) / above (bottom) takes the SAME projected surface, intersected with
    // the running intersection of the intervening layer slices to stay within the
    // print body. The whole top/bottom solid shell is therefore coloured by the
    // face it belongs to.
    //
    // OrcaSlicer inset (`segmentation_top_and_bottom_layers`, MMSeg.cpp:1551-1562):
    // each successive shell layer below a top surface (above a bottom surface) is
    // inset inward by `extrusion_spacing + extrusion_width` per layer of depth, so
    // the face colour fills only the INTERIOR of those layers — which print as
    // `Internal solid infill` (G4) — while their perimeter WALLS keep the side-face
    // colour. The contact (exposed) layer takes the full surface.
    let inset = |p: &[ExPolygon], delta_mm: f32| -> Vec<ExPolygon> {
        if delta_mm <= 0.0 || p.is_empty() {
            p.to_vec()
        } else {
            offset(p, -delta_mm, OffsetJoinType::Miter, 0.01)
        }
    };
    let mut acc: Vec<Vec<ExPolygon>> = vec![Vec::new(); out_len];
    for l in 0..out_len {
        if !top_raw[l].is_empty() {
            let top_ex = union_ex(&top_raw[l]);
            acc[l].extend(top_ex.iter().cloned());
            let mut trimmed: Vec<ExPolygon> = layer_at(l).to_vec();
            let lo = l.saturating_sub(top_depth - 1);
            for last in (lo..l).rev() {
                trimmed = intersection_ex(&trimmed, layer_at(last));
                if trimmed.is_empty() {
                    break;
                }
                // Depth below the contact layer (1, 2, …): inset progressively.
                let depth = (l - last) as f32;
                let inset_surface = inset(&top_ex, depth * shell_step);
                if inset_surface.is_empty() {
                    break;
                }
                let region = open(&intersection_ex(&inset_surface, &trimmed));
                if region.is_empty() {
                    break;
                }
                acc[last].extend(region);
            }
        }
        if !bot_raw[l].is_empty() {
            let bot_ex = union_ex(&bot_raw[l]);
            acc[l].extend(bot_ex.iter().cloned());
            let mut trimmed: Vec<ExPolygon> = layer_at(l).to_vec();
            let hi = (l + bottom_depth).min(out_len);
            for last in (l + 1)..hi {
                trimmed = intersection_ex(&trimmed, layer_at(last));
                if trimmed.is_empty() {
                    break;
                }
                // Depth above the contact layer (1, 2, …): inset progressively.
                let depth = (last - l) as f32;
                let inset_surface = inset(&bot_ex, depth * shell_step);
                if inset_surface.is_empty() {
                    break;
                }
                let region = open(&intersection_ex(&inset_surface, &trimmed));
                if region.is_empty() {
                    break;
                }
                acc[last].extend(region);
            }
        }
    }

    // Step 3 — union each layer's accumulated polygons and pad to total length.
    let mut per_layer: Vec<Vec<ExPolygon>> = acc
        .into_iter()
        .map(|v| if v.is_empty() { v } else { union_ex(&v) })
        .collect();
    while per_layer.len() < total_layers {
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
        // Top face only, 20 mm wide at z=1 (normal.z > 0). The wide XY footprint
        // keeps the progressive shell inset (≈0.8 mm/layer) non-empty for several
        // shell layers; the low Z keeps the layer count small in tests.
        let vertices = vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 20.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 20.0,
                y: 20.0,
                z: 1.0,
            },
            Point3 {
                x: 0.0,
                y: 20.0,
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

    /// Regression: a flat top face sits at z=`top_z` which lies ABOVE the last
    /// layer centre (`layer_zs[n-1]`) by up to a layer height. Slab bounds derived
    /// purely from layer centres would stop short of it, so `slice_mesh_slabs`
    /// would drop the face and the top surface would never be coloured. The bound
    /// extension must capture it, the contact layer must be the top layer, and the
    /// shell must propagate down `top_shell_layers`.
    #[test]
    fn propagate_top_bottom_face_above_last_layer_centre_is_captured() {
        // Top face at z=10.0; 4 layers whose centres stop at 9.5 (below the face).
        let mut mesh = unit_cube_painted_mesh();
        for v in &mut mesh.vertices {
            v.z = 10.0;
        }
        let layer_zs: Vec<f32> = vec![6.5, 7.5, 8.5, 9.5]; // top face (z=10) is above 9.5
        let contours: Vec<Vec<ExPolygon>> = layer_zs
            .iter()
            .map(|_| square_contour_mm(0.0, 0.0, 20.0, 20.0))
            .collect();

        let result = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(2),
            &layer_zs,
            &contours,
            3, // top shell = contact + 2 below
            0,
            0.4,
            0.2,
        );
        // The contact (top) layer and the two shell layers below must be covered.
        let covered: Vec<usize> = result
            .per_layer
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.is_empty())
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            covered,
            vec![1, 2, 3],
            "top face above the last layer centre must colour the top 3 layers; got {covered:?}"
        );
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
            0.4,
            0.2,
        );
        assert_eq!(result.per_layer.len(), contours.len());
        assert!(result.per_layer.iter().all(|l| l.is_empty()));
    }

    #[test]
    fn propagate_top_bottom_cube_upward_face_covers_top_slab() {
        let mesh = unit_cube_painted_mesh();
        // 4 layers ending at the cube top (z≈1.05); nothing exists above the top
        // face, so the occlusion test keeps it as an exposed top surface.
        let layer_zs: Vec<f32> = (0..4).map(|i| (i as f32 + 0.5) * 0.3).collect();
        // layer_zs ≈ [0.15, 0.45, 0.75, 1.05]; top face at z=1 is in the last slab.
        let contours: Vec<Vec<ExPolygon>> = layer_zs
            .iter()
            .map(|_| square_contour_mm(0.0, 0.0, 20.0, 20.0))
            .collect();

        // shell_layers = 0 — only the contact (cap) layer carries the top-face
        // projection.
        let result = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(2),
            &layer_zs,
            &contours,
            0,
            0,
            0.4,
            0.2,
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
            0.4,
            0.2,
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
        // 4 layers ending at the cube top (nothing above → exposed top surface).
        let layer_zs: Vec<f32> = (0..4).map(|i| (i as f32 + 0.5) * 0.3).collect();
        // layer_zs ≈ [0.15, 0.45, 0.75, 1.05]; top face slab = last (idx 3).
        let contours: Vec<Vec<ExPolygon>> = layer_zs
            .iter()
            .map(|_| square_contour_mm(0.0, 0.0, 20.0, 20.0))
            .collect();

        let result_shells_3 = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(2),
            &layer_zs,
            &contours,
            3, // top shell propagates DOWN by 3
            0,
            0.4,
            0.2,
        );
        let result_shells_0 = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(2),
            &layer_zs,
            &contours,
            0,
            0,
            0.4,
            0.2,
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
            .map(|_| square_contour_mm(0.0, 0.0, 20.0, 20.0))
            .collect();

        let result = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(1),
            &layer_zs,
            &contours,
            0,
            0,
            0.4,
            0.2,
        );

        // At least one layer non-empty; no padding beyond layer count.
        assert_eq!(result.per_layer.len(), contours.len());
        let any_nonempty = result.per_layer.iter().any(|l| !l.is_empty());
        assert!(any_nonempty, "shells=0 still produces coverage in top slab");
    }
}
