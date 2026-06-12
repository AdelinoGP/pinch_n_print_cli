//! Phase 6 — Top/bottom surface propagation for paint-segmentation.
//!
//! SIMPLIFIED FIRST CUT: For each layer, emits the intersection of
//! `top_proj[l] ∪ bottom_proj[l]` with `layer_input_polygons[l]`.
//! Full shell-propagation (top_shell_layers / bottom_shell_layers) is a
//! follow-up; tracked in follow_up notes at the bottom of this file.

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

/// Propagate top/bottom painted surfaces across layers.
///
/// # Arguments
/// - `painted_mesh_facets` — the painted-facets-only mesh extract.
/// - `semantic` / `value` — forwarded unchanged into the result.
/// - `layer_zs` — layer centre Z values in mm (length N).
/// - `layer_input_polygons` — sliced contours per layer (length N).
/// - `top_shell_layers` / `bottom_shell_layers` — shell thickness (future use).
///
/// # Simplified first cut
/// For each layer l:
///   result.per_layer[l] = intersection(top_proj[l] ∪ bottom_proj[l], layer_input_polygons[l])
///
/// Full shell propagation (expanding top/bottom influence by `top_shell_layers`) is deferred;
/// see follow-up notes at file bottom.
pub fn propagate_top_bottom(
    painted_mesh_facets: &IndexedTriangleSet,
    semantic: PaintSemantic,
    value: PaintValue,
    layer_zs: &[f32],
    layer_input_polygons: &[Vec<ExPolygon>],
    _top_shell_layers: usize,
    _bottom_shell_layers: usize,
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
    let mut zs_bounds: Vec<f32> = Vec::with_capacity(n + 1);
    zs_bounds.push(layer_zs[0] - half);
    for i in 1..n {
        zs_bounds.push((layer_zs[i - 1] + layer_zs[i]) / 2.0);
    }
    zs_bounds.push(layer_zs[n - 1] + half);

    let (top_proj, bot_proj) = slice_mesh_slabs(painted_mesh_facets, &zs_bounds);

    let slab_count = top_proj.len(); // == n (== zs_bounds.len()-1)
    let out_len = n.min(layer_input_polygons.len());
    let mut per_layer: Vec<Vec<ExPolygon>> = Vec::with_capacity(out_len);

    for l in 0..out_len {
        let top_l = top_proj.get(l).map(|v| v.as_slice()).unwrap_or(&[]);
        let bot_l = bot_proj.get(l).map(|v| v.as_slice()).unwrap_or(&[]);

        if top_l.is_empty() && bot_l.is_empty() {
            per_layer.push(Vec::new());
            continue;
        }

        // Union top and bottom projections for this layer.
        let mut combined: Vec<ExPolygon> = Vec::new();
        combined.extend_from_slice(top_l);
        combined.extend_from_slice(bot_l);
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

    let _ = slab_count; // suppress lint

    PerLayerSemanticPolygons {
        semantic,
        value,
        per_layer,
    }
}

// follow_up: Full shell propagation:
//   - top:    for shell in 0..top_shell_layers, intersect top_proj[l+shell+1] with
//             layer_input_polygons[l+shell+1] and accumulate, then difference back residue.
//   - bottom: symmetric with bot_proj[l-shell-1].
//   This requires iterating over adjacent slabs and is omitted in the simplified first cut.

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

        let result = propagate_top_bottom(
            &mesh,
            PaintSemantic::Material,
            PaintValue::ToolIndex(2),
            &layer_zs,
            &contours,
            3,
            3,
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
}
