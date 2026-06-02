#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::mesh_segmentation::execute_mesh_segmentation;
use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer, Transform3d,
};

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn p3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: p3(0.0, 0.0, 0.0),
        max: p3(200.0, 200.0, 200.0),
    }
}

fn simple_mesh() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "simple".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![p3(0.0, 0.0, 0.0), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0)],
                indices: vec![0, 1, 2],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: build_volume(),
    }
}

#[test]
fn passthrough_when_no_strokes() {
    let mesh = simple_mesh();
    let original_id = mesh.objects[0].id.clone();

    let result = execute_mesh_segmentation(Arc::new(mesh)).expect("must succeed");

    assert_eq!(result.objects[0].id, original_id);
    assert_eq!(result.objects[0].mesh.indices.len(), 3);
}

#[test]
fn preserves_determinism() {
    let mesh = simple_mesh();

    let a = execute_mesh_segmentation(Arc::new(mesh.clone())).unwrap();
    let b = execute_mesh_segmentation(Arc::new(mesh)).unwrap();

    assert_eq!(a.objects[0].mesh.indices, b.objects[0].mesh.indices);
    assert_eq!(a.objects[0].mesh.vertices, b.objects[0].mesh.vertices);
}
