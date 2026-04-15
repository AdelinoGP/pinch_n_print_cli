//! Paint segmentation prepass module for ModularSlicer.
//!
//! Projects 3D painted facets into 2D per-layer regions using the object's
//! transform matrix. For each object with paint layers, each painted facet is
//! projected onto the XY plane via the 4x4 transform matrix, and the resulting
//! 2D polygon is emitted as a region entry for every participating layer.
//!
//! # Algorithm
//!
//! For each object with paint layers:
//! 1. Compute facet count from triangles.len()
//! 2. For each paint_layer (enumerate as paint_order):
//!    - Validate facet_values.len() == facet_count
//!    - For each facet with Some(value):
//!      - Project 3D triangle to 2D using transform_matrix
//!      - For each participating_layer_index, push the region

use slicer_sdk::prelude::*;

/// Paint segmentation prepass module.
///
/// Implements `PrepassModule::run_paint_segmentation` to project 3D painted
/// facets into 2D per-layer regions.
pub struct PaintSegmentation;

#[slicer_module]
impl PrepassModule for PaintSegmentation {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(PaintSegmentation)
    }

    fn run_paint_segmentation(
        &self,
        objects: &[PaintSegmentationObjectView],
        output: &mut PaintSegmentationOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        for object in objects {
            if object.paint_layers.is_empty() {
                continue;
            }

            let facet_count = object.triangles.len();

            for (paint_order, paint_layer) in object.paint_layers.iter().enumerate() {
                if paint_layer.facet_values.len() != facet_count {
                    return Err(ModuleError::fatal(
                        1,
                        format!(
                            "object '{}' paint layer {} has {} facet values but {} triangles",
                            object.object_id,
                            paint_order,
                            paint_layer.facet_values.len(),
                            facet_count,
                        ),
                    ));
                }

                for (facet_index, facet_value) in paint_layer.facet_values.iter().enumerate() {
                    let Some(value) = facet_value else {
                        continue;
                    };

                    let contour = project_facet(
                        &object.vertices,
                        &object.triangles[facet_index],
                        &object.transform_matrix,
                    );

                    for &layer_index in &object.participating_layer_indices {
                        output.push_paint_region(
                            layer_index,
                            paint_layer.semantic.clone(),
                            object.object_id.clone(),
                            value.clone(),
                            paint_order as u64,
                            contour.clone(),
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

/// Project a 3D triangle to 2D using the transform matrix (XY projection).
///
/// Uses the same projection formula as the host executor at
/// `crates/slicer-host/src/paint_segmentation.rs`.
fn project_facet(vertices: &[[f32; 3]], triangle: &[u32; 3], matrix: &[f64; 16]) -> Vec<[f64; 2]> {
    triangle
        .iter()
        .map(|&vertex_index| {
            let v = vertices[vertex_index as usize];
            transform_point(v[0], v[1], v[2], matrix)
        })
        .collect()
}

/// Transform a 3D point through a 4x4 column-major matrix, returning 2D (x, y).
///
/// Mirrors the host executor projection formula.
fn transform_point(x: f32, y: f32, z: f32, matrix: &[f64; 16]) -> [f64; 2] {
    let x = f64::from(x);
    let y = f64::from(y);
    let z = f64::from(z);
    let tx = matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12];
    let ty = matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13];
    let tw = matrix[3] * x + matrix[7] * y + matrix[11] * z + matrix[15];
    if tw != 0.0 && tw != 1.0 {
        [tx / tw, ty / tw]
    } else {
        [tx, ty]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_identity() {
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let [x, y] = transform_point(1.0, 2.0, 3.0, &identity);
        assert!((x - 1.0).abs() < 1e-10);
        assert!((y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn transform_translation() {
        let matrix = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 20.0, 0.0, 1.0,
        ];
        let [x, y] = transform_point(1.0, 2.0, 0.0, &matrix);
        assert!((x - 11.0).abs() < 1e-10);
        assert!((y - 22.0).abs() < 1e-10);
    }

    #[test]
    fn transform_perspective_divide() {
        // w = 2.0 for all points
        let matrix = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 2.0,
        ];
        let [x, y] = transform_point(4.0, 6.0, 0.0, &matrix);
        assert!((x - 2.0).abs() < 1e-10);
        assert!((y - 3.0).abs() < 1e-10);
    }
}
