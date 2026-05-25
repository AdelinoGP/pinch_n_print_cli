//! TDD red tests for mesh decimation (TASK-057).
//!
//! These tests compile but fail on the `todo!()` stub in `decimate::decimate()`.
//! Each test constructs a MeshIR with appropriate geometry and asserts the expected
//! decimation outcome.

use slicer_helpers::{decimate, DecimateConfigBuilder, DecimateError};
use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, Transform3d,
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
    let world_z_extent = {
        let mut z_min = f32::INFINITY;
        let mut z_max = f32::NEG_INFINITY;
        for v in &its.vertices {
            if v.z < z_min {
                z_min = v.z;
            }
            if v.z > z_max {
                z_max = v.z;
            }
        }
        if z_min.is_finite() && z_max.is_finite() && z_max > z_min {
            Some((z_min, z_max))
        } else {
            None
        }
    };
    MeshIR {
        objects: vec![ObjectMesh {
            id: "test-object".to_string(),
            mesh: its,
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent,
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
        ..Default::default()
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
    assert!(
        original_count >= 1800,
        "sphere should have ~2000 tris, got {original_count}"
    );

    let config = DecimateConfigBuilder::new()
        .target_ratio(0.5)
        .build()
        .expect("builder should validate");

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

    let config = DecimateConfigBuilder::new()
        .target_count(400)
        .max_error(f32::MAX)
        .build()
        .expect("builder should validate");

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

    let config = DecimateConfigBuilder::new()
        .target_ratio(0.5)
        .max_error(0.001)
        .build()
        .expect("builder should validate");

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
    let config = DecimateConfigBuilder::new()
        .target_ratio(0.01)
        .max_error(0.001)
        .build()
        .expect("builder should validate");

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
    let mesh = MeshIR::default();

    let config = DecimateConfigBuilder::new()
        .target_ratio(0.5)
        .build()
        .expect("builder should validate");

    let result = decimate(mesh, config);
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), DecimateError::EmptyMesh),
        "expected EmptyMesh error"
    );
}

#[test]
fn decimate_conflict_config_error() {
    // Both target_count and target_ratio set — rejected at builder.build().
    let result = DecimateConfigBuilder::new()
        .target_count(400)
        .target_ratio(0.5)
        .build();
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), DecimateError::InvalidConfig(_)),
        "expected InvalidConfig error"
    );
}

#[test]
fn decimate_normalizes_winding_after_simplify() {
    // docs/13_slicer_helpers_crate.md §Decimation Algorithm step 4 requires a
    // Phase 2 (orientation normalization) pass after `meshopt::simplify` so
    // that any winding inconsistencies introduced by edge collapse are
    // corrected. Without that pass, seeded inverted triangles survive
    // simplification and the output mesh contains "same-direction" interior
    // edges — both triangles around a manifold edge reference it in the same
    // order, which breaks downstream pipeline assumptions.
    //
    // Test construction:
    //   1. Build a clean UV sphere (consistent winding).
    //   2. Manually flip the winding on every 5th triangle.
    //   3. Decimate at ratio 0.5.
    //   4. Assert: no manifold edge in the output has same-direction adjacency.
    let mut its = sphere_2000();
    let tri_count = its.indices.len() / 3;
    for t in (0..tri_count).step_by(5) {
        its.indices.swap(t * 3 + 1, t * 3 + 2);
    }
    let mesh = single_object_mesh(its);

    let config = DecimateConfigBuilder::new()
        .target_ratio(0.5)
        .build()
        .expect("builder should validate");

    let result = decimate(mesh, config).expect("decimate should succeed");
    let out_its = &result.mesh.objects[0].mesh;

    // Count directed edges across all output triangles.
    let mut directed: HashMap<(u32, u32), u32> = HashMap::new();
    for t in 0..(out_its.indices.len() / 3) {
        let i0 = out_its.indices[t * 3];
        let i1 = out_its.indices[t * 3 + 1];
        let i2 = out_its.indices[t * 3 + 2];
        for (a, b) in [(i0, i1), (i1, i2), (i2, i0)] {
            *directed.entry((a, b)).or_insert(0) += 1;
        }
    }

    // For each canonical edge that's manifold (two-triangle adjacency, i.e.
    // forward + reverse == 2), the two triangles must reference it in
    // opposite directions: forward == 1 AND reverse == 1.
    let mut same_direction_collisions = 0usize;
    let mut manifold_edges_checked = 0usize;
    let mut keys: Vec<(u32, u32)> = directed.keys().copied().collect();
    keys.sort();
    for (a, b) in keys {
        if a >= b {
            continue; // canonicalize: visit each undirected edge once
        }
        let forward = directed.get(&(a, b)).copied().unwrap_or(0);
        let reverse = directed.get(&(b, a)).copied().unwrap_or(0);
        if forward + reverse == 2 {
            manifold_edges_checked += 1;
            if forward != 1 || reverse != 1 {
                same_direction_collisions += 1;
            }
        }
    }

    assert!(
        manifold_edges_checked > 100,
        "test setup invalid: expected the decimated sphere to have many \
         manifold edges, got {manifold_edges_checked}"
    );
    assert_eq!(
        same_direction_collisions, 0,
        "after Phase 2 orientation pass, no manifold edge should have \
         same-direction adjacency; found {same_direction_collisions} out of \
         {manifold_edges_checked} manifold edges"
    );
}
