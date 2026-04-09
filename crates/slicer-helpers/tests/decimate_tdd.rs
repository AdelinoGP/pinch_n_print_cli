//! TDD red tests for mesh decimation (TASK-057).
//!
//! These tests compile but fail on the `todo!()` stub in `decimate::decimate()`.
//! Each test constructs a MeshIR with appropriate geometry and asserts the expected
//! decimation outcome.

use slicer_helpers::{decimate, DecimateConfig, DecimateError};
use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer,
    Transform3d,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Identity 4x4 matrix in column-major order.
fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, // col 0
            0.0, 1.0, 0.0, 0.0, // col 1
            0.0, 0.0, 1.0, 0.0, // col 2
            0.0, 0.0, 0.0, 1.0, // col 3
        ],
    }
}

/// Wrap an IndexedTriangleSet in a single-object MeshIR.
fn single_object_mesh(its: IndexedTriangleSet) -> MeshIR {
    MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![ObjectMesh {
            id: "test-object".to_string(),
            mesh: its,
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: -200.0,
                y: -200.0,
                z: -200.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    }
}

/// Build a UV sphere with approximately `target_tris` triangles.
/// Uses lat/lon subdivision. Coordinates in mm (f32).
fn uv_sphere(radius_mm: f32, lat_segments: usize, lon_segments: usize) -> IndexedTriangleSet {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for lat in 0..=lat_segments {
        let theta = std::f32::consts::PI * (lat as f32) / (lat_segments as f32);
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=lon_segments {
            let phi = 2.0 * std::f32::consts::PI * (lon as f32) / (lon_segments as f32);
            let x = radius_mm * sin_theta * phi.cos();
            let y = radius_mm * sin_theta * phi.sin();
            let z = radius_mm * cos_theta;
            vertices.push(Point3 { x, y, z });
        }
    }

    // Generate indices
    for lat in 0..lat_segments {
        for lon in 0..lon_segments {
            let first = (lat * (lon_segments + 1) + lon) as u32;
            let second = first + (lon_segments + 1) as u32;

            // First triangle of quad
            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            // Second triangle of quad
            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    IndexedTriangleSet { vertices, indices }
}

/// Build a sphere with ~2000 triangles (32 lat × 32 lon = 2048 tris).
fn sphere_2000() -> IndexedTriangleSet {
    uv_sphere(50.0, 32, 32)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn decimate_by_ratio() {
    let its = sphere_2000();
    let mesh = single_object_mesh(its);
    let original_count = mesh.objects[0].mesh.indices.len() / 3;
    assert!(original_count >= 1800, "sphere should have ~2000 tris, got {original_count}");

    let config = DecimateConfig {
        target_ratio: Some(0.5),
        ..DecimateConfig::default()
    };

    let result = decimate(mesh, config).expect("decimate should succeed");
    let half = original_count / 2;
    assert!(
        result.final_triangle_count <= half + half / 10,
        "expected ≤{} tris (50% + 10% tolerance), got {}",
        half + half / 10,
        result.final_triangle_count
    );
    assert_eq!(result.original_triangle_count, original_count);
}

#[test]
fn decimate_by_count() {
    let its = sphere_2000();
    let mesh = single_object_mesh(its);
    let original_count = mesh.objects[0].mesh.indices.len() / 3;
    assert!(original_count >= 1800);

    let config = DecimateConfig {
        target_count: Some(400),
        max_error: f32::MAX,
        ..DecimateConfig::default()
    };

    let result = decimate(mesh, config).expect("decimate should succeed");
    assert!(
        result.final_triangle_count <= 400,
        "expected ≤400 tris, got {}",
        result.final_triangle_count
    );
    assert_eq!(result.original_triangle_count, original_count);
}

#[test]
fn decimate_respects_error_budget() {
    let its = sphere_2000();
    let mesh = single_object_mesh(its);

    let config = DecimateConfig {
        target_ratio: Some(0.5),
        max_error: 0.001,
        ..DecimateConfig::default()
    };

    let result = decimate(mesh, config).expect("decimate should succeed");
    assert!(
        result.achieved_error <= 0.001,
        "achieved_error {} should be ≤ 0.001",
        result.achieved_error
    );
}

#[test]
fn decimate_stops_early() {
    let its = sphere_2000();
    let mesh = single_object_mesh(its);
    let original_count = mesh.objects[0].mesh.indices.len() / 3;

    // Very aggressive target (1% of original) but very tight error budget.
    // Decimation should stop early, keeping more triangles than the 1% target.
    let target = (original_count as f32 * 0.01) as usize;
    let config = DecimateConfig {
        target_ratio: Some(0.01),
        max_error: 0.001,
        ..DecimateConfig::default()
    };

    let result = decimate(mesh, config).expect("decimate should succeed");
    // Stopped early: final count should be more than the aggressive target
    assert!(
        result.final_triangle_count > target,
        "expected early stop (>{} tris), got {}",
        target,
        result.final_triangle_count
    );
}

#[test]
fn decimate_empty_mesh_error() {
    let mesh = MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
    };

    let config = DecimateConfig {
        target_ratio: Some(0.5),
        ..DecimateConfig::default()
    };

    let result = decimate(mesh, config);
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), DecimateError::EmptyMesh),
        "expected EmptyMesh error"
    );
}

#[test]
fn decimate_conflict_config_error() {
    let its = sphere_2000();
    let mesh = single_object_mesh(its);

    // Both target_count and target_ratio set — should be rejected.
    let config = DecimateConfig {
        target_count: Some(400),
        target_ratio: Some(0.5),
        ..DecimateConfig::default()
    };

    let result = decimate(mesh, config);
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), DecimateError::InvalidConfig(_)),
        "expected InvalidConfig error"
    );
}
