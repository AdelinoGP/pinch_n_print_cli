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
/// Phase 1 preprocess — extracts per-layer paint data from mesh objects.
use slicer_ir::{IndexedTriangleSet, PaintLayer, PaintSemantic, PaintValue, Point3};

/// Preprocessed paint data for one triangle.
#[derive(Debug, Clone)]
pub struct TrianglePaint {
    /// World-space vertices of the triangle.
    pub vertices: [Point3; 3],
    /// The paint semantic family.
    pub semantic: PaintSemantic,
    /// The paint value.
    pub value: PaintValue,
}

/// Extract TrianglePaint entries from a PaintLayer + mesh.
pub fn extract_paint_layer_data(
    paint_layer: &PaintLayer,
    mesh: &IndexedTriangleSet,
    transform: &[f64; 16],
) -> Vec<TrianglePaint> {
    let facet_count = mesh.indices.len() / 3;
    let mut result = Vec::new();

    for (facet_idx, facet_value) in paint_layer.facet_values.iter().enumerate() {
        if facet_idx >= facet_count {
            break;
        }
        let Some(value) = facet_value else { continue };

        let base = facet_idx * 3;
        let vertices = [
            crate::transform_point3(transform, mesh.vertices[mesh.indices[base] as usize]),
            crate::transform_point3(transform, mesh.vertices[mesh.indices[base + 1] as usize]),
            crate::transform_point3(transform, mesh.vertices[mesh.indices[base + 2] as usize]),
        ];

        result.push(TrianglePaint {
            vertices,
            semantic: paint_layer.semantic.clone(),
            value: value.clone(),
        });
    }
    result
}

/// Extract TrianglePaint entries from strokes.
pub fn extract_stroke_data(
    strokes: &[slicer_ir::PaintStroke],
    transform: &[f64; 16],
) -> Vec<TrianglePaint> {
    let mut result = Vec::new();
    for stroke in strokes {
        for tri in &stroke.triangles {
            result.push(TrianglePaint {
                vertices: [
                    crate::transform_point3(transform, tri[0]),
                    crate::transform_point3(transform, tri[1]),
                    crate::transform_point3(transform, tri[2]),
                ],
                semantic: stroke.semantic.clone(),
                value: stroke.value.clone(),
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::PaintStroke;

    fn identity() -> [f64; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    #[test]
    fn extract_from_facet_values() {
        let mesh = IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 10.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 5.0,
                    y: 10.0,
                    z: 0.0,
                },
            ],
            indices: vec![0, 1, 2],
        };
        let layer = PaintLayer {
            semantic: PaintSemantic::Material,
            facet_values: vec![Some(PaintValue::ToolIndex(1))],
            strokes: Vec::new(),
        };
        let result = extract_paint_layer_data(&layer, &mesh, &identity());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].semantic, PaintSemantic::Material);
    }

    #[test]
    fn extract_stroke_data_basic() {
        let strokes = vec![PaintStroke {
            triangles: vec![[
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.5,
                    y: 1.0,
                    z: 0.0,
                },
            ]],
            semantic: PaintSemantic::FuzzySkin,
            value: PaintValue::Flag(true),
        }];
        let result = extract_stroke_data(&strokes, &identity());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].semantic, PaintSemantic::FuzzySkin);
    }
}
