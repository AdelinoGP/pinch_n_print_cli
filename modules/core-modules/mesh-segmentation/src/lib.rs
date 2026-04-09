//! Mesh segmentation prepass module for ModularSlicer.
//!
//! Normalizes sub-facet paint strokes into whole-triangle assignments so that
//! downstream stages consume a uniformly tagged mesh where each triangle has
//! exactly one paint value per semantic.
//!
//! # Algorithm (MVP — centroid assignment)
//!
//! For each object with paint strokes:
//! 1. Clone the original vertices, triangles, and facet values.
//! 2. For each paint layer that has strokes:
//!    - For each stroke, compute the centroid of each stroke triangle.
//!    - Find which mesh facet the centroid falls inside (barycentric point-in-triangle test).
//!    - Assign that facet the stroke's paint value.
//! 3. Mark strokes as cleared and push the modification to output.

use slicer_sdk::prelude::*;

/// Mesh segmentation prepass module.
///
/// Implements `PrepassModule::run_mesh_segmentation` to normalize sub-facet paint
/// strokes into whole-triangle facet value assignments.
pub struct MeshSegmentation;

impl PrepassModule for MeshSegmentation {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(MeshSegmentation)
    }

    fn run_mesh_segmentation(
        &self,
        objects: &[MeshObjectView],
        output: &mut MeshSegmentationOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        for object in objects {
            if !object_has_strokes(object) {
                continue;
            }

            let modification = process_object(object)?;
            output
                .push_modification(modification)
                .map_err(|e| ModuleError::fatal(1, e))?;
        }

        Ok(())
    }
}

/// Check whether any paint layer on the object has non-empty strokes.
fn object_has_strokes(object: &MeshObjectView) -> bool {
    object
        .paint_layers
        .iter()
        .any(|layer| !layer.strokes.is_empty())
}

/// Process a single object: resolve strokes into facet values.
fn process_object(object: &MeshObjectView) -> Result<ObjectMeshModification, ModuleError> {
    let new_vertices = object.vertices.clone();
    let new_triangles = object.triangles.clone();

    // Clone facet values from each paint layer
    let mut updated_facet_values: Vec<Vec<Option<PaintValueView>>> = object
        .paint_layers
        .iter()
        .map(|layer| layer.facet_values.clone())
        .collect();

    // For each paint layer, resolve strokes into facet values
    for (layer_idx, paint_layer) in object.paint_layers.iter().enumerate() {
        for stroke in &paint_layer.strokes {
            for stroke_tri in &stroke.triangles {
                let centroid = triangle_centroid(stroke_tri);

                if let Some(facet_idx) =
                    locate_facet_by_centroid(&object.vertices, &object.triangles, &centroid)
                {
                    updated_facet_values[layer_idx][facet_idx] = Some(stroke.value.clone());
                }
            }
        }
    }

    Ok(ObjectMeshModification {
        object_id: object.object_id.clone(),
        new_vertices,
        new_triangles,
        updated_facet_values,
        strokes_cleared: true,
    })
}

/// Compute the centroid of a triangle given as three `[f32; 3]` vertices.
fn triangle_centroid(tri: &[[f32; 3]; 3]) -> [f32; 3] {
    [
        (tri[0][0] + tri[1][0] + tri[2][0]) / 3.0,
        (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0,
        (tri[0][2] + tri[1][2] + tri[2][2]) / 3.0,
    ]
}

/// Find which mesh facet (by index) contains the given point using barycentric coordinates.
///
/// Returns `None` if no facet contains the point.
fn locate_facet_by_centroid(
    vertices: &[[f32; 3]],
    triangles: &[[u32; 3]],
    point: &[f32; 3],
) -> Option<usize> {
    for (facet_idx, tri) in triangles.iter().enumerate() {
        let a = &vertices[tri[0] as usize];
        let b = &vertices[tri[1] as usize];
        let c = &vertices[tri[2] as usize];

        if point_in_triangle(point, a, b, c) {
            return Some(facet_idx);
        }
    }
    None
}

/// Barycentric coordinate point-in-triangle test (2.5D — projects onto the dominant plane).
///
/// Uses the same approach as the host executor: compute barycentric coordinates
/// and check that all are non-negative (within epsilon).
fn point_in_triangle(p: &[f32; 3], a: &[f32; 3], b: &[f32; 3], c: &[f32; 3]) -> bool {
    const EPSILON: f32 = 1.0e-6;

    let v0 = sub3(b, a);
    let v1 = sub3(c, a);
    let v2 = sub3(p, a);

    let d00 = dot3(&v0, &v0);
    let d01 = dot3(&v0, &v1);
    let d11 = dot3(&v1, &v1);
    let d20 = dot3(&v2, &v0);
    let d21 = dot3(&v2, &v1);

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() <= EPSILON {
        return false;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    u >= -EPSILON && v >= -EPSILON && w >= -EPSILON
}

fn sub3(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot3(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centroid_computation() {
        let tri = [[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [0.0, 3.0, 0.0]];
        let c = triangle_centroid(&tri);
        assert!((c[0] - 1.0).abs() < 1e-6);
        assert!((c[1] - 1.0).abs() < 1e-6);
        assert!((c[2] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn point_in_triangle_inside() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        let c = [0.0, 1.0, 0.0];
        assert!(point_in_triangle(&[0.2, 0.2, 0.0], &a, &b, &c));
    }

    #[test]
    fn point_in_triangle_outside() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        let c = [0.0, 1.0, 0.0];
        assert!(!point_in_triangle(&[5.0, 5.0, 0.0], &a, &b, &c));
    }

    #[test]
    fn point_in_triangle_on_edge() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        let c = [0.0, 1.0, 0.0];
        // Point on edge AB at midpoint
        assert!(point_in_triangle(&[0.5, 0.0, 0.0], &a, &b, &c));
    }
}
