use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::wit_host::HostExecutionContext;
use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer,
    Transform3d,
};

pub fn ctx_with_mesh(module_id: &str, mesh: Arc<MeshIR>) -> HostExecutionContext {
    HostExecutionContext::new(module_id.to_string(), 0.0, 0.0, None, Some(mesh))
}

pub fn point3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

pub fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

pub fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: point3(0.0, 0.0, 0.0),
        max: point3(200.0, 200.0, 200.0),
    }
}

pub fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

pub fn translation_transform(tx: f64, ty: f64, tz: f64) -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, tz, 1.0,
        ],
    }
}

pub fn mesh_fixture(objects: Vec<ObjectMesh>) -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(),
        objects,
        build_volume: build_volume(),
    })
}

pub fn flat_plate_object(id: &str, local_z: f32, transform: Transform3d) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![
                point3(0.0, 0.0, local_z),
                point3(10.0, 0.0, local_z),
                point3(0.0, 10.0, local_z),
                point3(10.0, 10.0, local_z),
            ],
            indices: vec![0, 1, 2, 1, 3, 2],
        },
        transform,
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
    }
}

/// Reserved for future non-axis-aligned surface normal tests.
#[allow(dead_code)]
pub fn sloped_triangle_object(id: &str, transform: Transform3d) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![point3(0.0, 0.0, 0.0), point3(10.0, 0.0, 0.0), point3(0.0, 10.0, 10.0)],
            indices: vec![0, 1, 2],
        },
        transform,
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
    }
}

pub fn assert_close(actual: f32, expected: f32, label: &str) {
    assert!(
        (actual - expected).abs() < 1.0e-4,
        "{label} expected {expected}, got {actual}"
    );
}

pub fn assert_unit_length(x: f32, y: f32, z: f32, label: &str) {
    let magnitude = (x * x + y * y + z * z).sqrt();
    assert_close(magnitude, 1.0, label);
}

pub fn assert_perpendicular(
    x: f32,
    y: f32,
    z: f32,
    edge1: [f32; 3],
    edge2: [f32; 3],
    label: &str,
) {
    let dot1 = x * edge1[0] + y * edge1[1] + z * edge1[2];
    let dot2 = x * edge2[0] + y * edge2[1] + z * edge2[2];
    assert_close(dot1, 0.0, &format!("{label} dot edge1"));
    assert_close(dot2, 0.0, &format!("{label} dot edge2"));
}