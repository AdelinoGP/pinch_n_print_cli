#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, FacetPaintData, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, PaintLayer,
    PaintSemantic, PaintStroke, PaintValue, Point3, SemVer, Transform3d,
};
use slicer_runtime::{execute_mesh_segmentation, DegenerateStrokeReason, MeshSegmentationError};

#[test]
fn mesh_segmentation_passthrough_preserves_a_mesh_with_no_subfacet_strokes() {
    let mesh = Arc::new(mesh_without_strokes());

    let normalized = execute_mesh_segmentation(Arc::clone(&mesh))
        .expect("meshes without strokes are a Tier-1 no-op per docs/01_system_architecture.md");

    assert!(
        Arc::ptr_eq(&normalized, &mesh),
        "no-op normalization should preserve the original Arc for the blackboard handoff"
    );
    assert_eq!(*normalized, *mesh);
}

#[test]
fn mesh_segmentation_splits_one_triangle_and_assigns_whole_triangle_paint_deterministically() {
    let mesh = Arc::new(mesh_with_single_split_stroke());

    let normalized = execute_mesh_segmentation(mesh)
        .expect("a stroke crossing one facet should normalize before later semantic analysis");

    let object = &normalized.objects[0];
    let paint_layer = &object
        .paint_data
        .as_ref()
        .expect("fixture should carry paint data")
        .layers[0];

    // TASK-028 and docs/10 require deterministic normalization before PaintSegmentation.
    // The upstream OrcaSlicer mesh-boolean pipeline is the closest sequencing reference.
    assert_eq!(object.mesh.indices.len() / 3, 2);
    assert_eq!(paint_layer.strokes, Vec::<PaintStroke>::new());
    assert_eq!(
        paint_layer.facet_values,
        vec![Some(PaintValue::ToolIndex(1)), None]
    );
}

#[test]
fn mesh_segmentation_rejects_zero_area_strokes_with_a_stable_error() {
    let mesh = Arc::new(mesh_with_zero_area_stroke());

    assert_eq!(
        execute_mesh_segmentation(mesh),
        Err(MeshSegmentationError::DegenerateStroke {
            object_id: String::from("qa-triangle"),
            layer_index: 0,
            stroke_index: 0,
            reason: DegenerateStrokeReason::ZeroAreaStrokeTriangle,
        })
    );
}

#[test]
fn mesh_segmentation_is_idempotent_and_clears_subfacet_strokes_for_blackboard_commit() {
    let mesh = Arc::new(mesh_with_single_split_stroke());

    let normalized_once = execute_mesh_segmentation(mesh)
        .expect("first normalization should produce a downstream-safe MeshIR");
    let normalized_twice = execute_mesh_segmentation(Arc::clone(&normalized_once))
        .expect("running the executor again should be a stable no-op once strokes are resolved");

    assert_eq!(*normalized_twice, *normalized_once);
    assert!(Arc::ptr_eq(&normalized_twice, &normalized_once));

    let paint_layer = &normalized_twice.objects[0]
        .paint_data
        .as_ref()
        .expect("normalized mesh should still carry paint data")
        .layers[0];
    assert!(paint_layer.strokes.is_empty());
    assert_eq!(
        paint_layer.facet_values.len(),
        normalized_twice.objects[0].mesh.indices.len() / 3
    );
}

fn mesh_without_strokes() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("qa-triangle"),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    point3(0.0, 0.0, 0.0),
                    point3(2.0, 0.0, 0.0),
                    point3(1.0, 1.0, 0.0),
                ],
                indices: vec![0, 1, 2],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: Some(FacetPaintData {
                layers: vec![PaintLayer {
                    semantic: PaintSemantic::Material,
                    facet_values: vec![None],
                    strokes: Vec::new(),
                }],
            }),
            world_z_extent: None,
        }],
        build_volume: build_volume(),
    }
}

fn mesh_with_single_split_stroke() -> MeshIR {
    let mut mesh = mesh_without_strokes();
    mesh.objects[0].paint_data = Some(FacetPaintData {
        layers: vec![PaintLayer {
            semantic: PaintSemantic::Material,
            facet_values: vec![None],
            strokes: vec![PaintStroke {
                triangles: vec![[
                    point3(0.0, 0.0, 0.0),
                    point3(1.0, 0.0, 0.0),
                    point3(1.0, 1.0, 0.0),
                ]],
                semantic: PaintSemantic::Material,
                value: PaintValue::ToolIndex(1),
            }],
        }],
    });
    mesh
}

fn mesh_with_zero_area_stroke() -> MeshIR {
    let mut mesh = mesh_without_strokes();
    mesh.objects[0].paint_data = Some(FacetPaintData {
        layers: vec![PaintLayer {
            semantic: PaintSemantic::Material,
            facet_values: vec![None],
            strokes: vec![PaintStroke {
                triangles: vec![[
                    point3(0.5, 0.5, 0.0),
                    point3(0.5, 0.5, 0.0),
                    point3(0.5, 0.5, 0.0),
                ]],
                semantic: PaintSemantic::Material,
                value: PaintValue::ToolIndex(1),
            }],
        }],
    });
    mesh
}

fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: point3(0.0, 0.0, 0.0),
        max: point3(200.0, 200.0, 200.0),
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn point3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}
