//! TDD red tests for mesh repair (TASK-056).
//!
//! These tests compile but fail on the `todo!()` stub in `repair::repair()`.
//! Each test constructs a MeshIR with a specific defect and asserts the expected
//! repair outcome.

use slicer_helpers::{repair, RepairResult, RepairWarning};
use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer, Transform3d,
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
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: -100.0,
                y: -100.0,
                z: -100.0,
            },
            max: Point3 {
                x: 100.0,
                y: 100.0,
                z: 100.0,
            },
        },
    }
}

/// Build a valid closed cube mesh (8 vertices, 12 triangles).
/// The cube spans from (0,0,0) to (10,10,10) mm.
fn valid_cube() -> IndexedTriangleSet {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }, // 0: left  bottom back
        Point3 {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        }, // 1: right bottom back
        Point3 {
            x: 10.0,
            y: 10.0,
            z: 0.0,
        }, // 2: right top    back
        Point3 {
            x: 0.0,
            y: 10.0,
            z: 0.0,
        }, // 3: left  top    back
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 10.0,
        }, // 4: left  bottom front
        Point3 {
            x: 10.0,
            y: 0.0,
            z: 10.0,
        }, // 5: right bottom front
        Point3 {
            x: 10.0,
            y: 10.0,
            z: 10.0,
        }, // 6: right top    front
        Point3 {
            x: 0.0,
            y: 10.0,
            z: 10.0,
        }, // 7: left  top    front
    ];

    // Outward-facing winding (CCW when viewed from outside).
    // Each face = 2 triangles.
    #[rustfmt::skip]
    let indices = vec![
        // Back face (z=0, normal -Z) — CCW from outside = CW from +Z
        0, 2, 1,
        0, 3, 2,
        // Front face (z=10, normal +Z) — CCW from outside
        4, 5, 6,
        4, 6, 7,
        // Bottom face (y=0, normal -Y)
        0, 1, 5,
        0, 5, 4,
        // Top face (y=10, normal +Y)
        3, 6, 2,
        3, 7, 6,
        // Left face (x=0, normal -X)
        0, 4, 7,
        0, 7, 3,
        // Right face (x=10, normal +X)
        1, 2, 6,
        1, 6, 5,
    ];

    IndexedTriangleSet { vertices, indices }
}

// ---------------------------------------------------------------------------
// Test 1: Degenerate triangle removal
// ---------------------------------------------------------------------------

/// Mesh with 3 zero-area (degenerate) triangles among valid ones.
/// After repair, `stats.degenerate_removed` must be 3.
#[test]
fn repair_removes_degenerate_triangles() {
    let mut its = valid_cube();

    // Add 3 degenerate triangles (collinear vertices → zero area).
    let base = its.vertices.len() as u32;
    // Degenerate 1: three collinear points along X.
    its.vertices.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 20.0,
    });
    its.vertices.push(Point3 {
        x: 5.0,
        y: 0.0,
        z: 20.0,
    });
    its.vertices.push(Point3 {
        x: 10.0,
        y: 0.0,
        z: 20.0,
    });
    its.indices.extend_from_slice(&[base, base + 1, base + 2]);

    // Degenerate 2: three identical points.
    let base2 = its.vertices.len() as u32;
    its.vertices.push(Point3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    });
    its.vertices.push(Point3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    });
    its.vertices.push(Point3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    });
    its.indices
        .extend_from_slice(&[base2, base2 + 1, base2 + 2]);

    // Degenerate 3: two vertices the same, third different but still collinear.
    let base3 = its.vertices.len() as u32;
    its.vertices.push(Point3 {
        x: 3.0,
        y: 3.0,
        z: 3.0,
    });
    its.vertices.push(Point3 {
        x: 3.0,
        y: 3.0,
        z: 3.0,
    });
    its.vertices.push(Point3 {
        x: 6.0,
        y: 6.0,
        z: 6.0,
    });
    its.indices
        .extend_from_slice(&[base3, base3 + 1, base3 + 2]);

    let mesh = single_object_mesh(its);
    let result: RepairResult = repair(mesh).expect("repair should not error");

    assert_eq!(
        result.stats.degenerate_removed, 3,
        "expected 3 degenerate triangles removed"
    );
    // The valid cube triangles (12) should remain.
    let tri_count = result.mesh.objects[0].mesh.indices.len() / 3;
    assert_eq!(tri_count, 12, "only valid cube triangles should remain");
}

// ---------------------------------------------------------------------------
// Test 2: Face orientation normalization
// ---------------------------------------------------------------------------

/// Cube with one face's winding reversed. After repair, at least one face
/// should be reoriented.
#[test]
fn repair_normalizes_flipped_face() {
    let mut its = valid_cube();

    // Flip the first triangle's winding (back face, triangle 0).
    // Original: 0, 2, 1 → flipped: 0, 1, 2
    its.indices[0] = 0;
    its.indices[1] = 1;
    its.indices[2] = 2;

    let mesh = single_object_mesh(its);
    let result: RepairResult = repair(mesh).expect("repair should not error");

    assert!(
        result.stats.faces_reoriented >= 1,
        "expected at least 1 face reoriented, got {}",
        result.stats.faces_reoriented
    );
}

// ---------------------------------------------------------------------------
// Test 3: Open-edge closure
// ---------------------------------------------------------------------------

/// Cube with one face (2 triangles) removed, creating open edges.
/// After repair, those open edges should be closed.
#[test]
fn repair_closes_open_edge() {
    let mut its = valid_cube();

    // Remove the front face (triangles at indices 6..12 in the index buffer,
    // i.e. the 3rd pair of triangles: indices[6], indices[7], ..., indices[11]).
    // Front face triangles: 4,5,6 and 4,6,7
    // Remove by truncating those 6 index entries.
    // The front face is indices 6..12 (triangles 2 and 3).
    its.indices.drain(6..12);

    // Now we have 10 triangles with 4 open boundary edges.
    let mesh = single_object_mesh(its);
    let result: RepairResult = repair(mesh).expect("repair should not error");

    assert!(
        result.stats.open_edges_closed > 0,
        "expected open edges to be closed, got 0"
    );
}

// ---------------------------------------------------------------------------
// Test 4: No-op on clean mesh
// ---------------------------------------------------------------------------

/// A valid closed cube mesh should pass through repair with all stats at zero.
#[test]
fn repair_noop_on_clean_mesh() {
    let mesh = single_object_mesh(valid_cube());
    let result: RepairResult = repair(mesh).expect("repair should not error");

    assert_eq!(result.stats.degenerate_removed, 0);
    assert_eq!(result.stats.faces_reoriented, 0);
    assert_eq!(result.stats.open_edges_closed, 0);
    assert!(
        result.stats.warnings.is_empty(),
        "clean mesh should produce no warnings"
    );
    // Mesh should still have 12 triangles.
    let tri_count = result.mesh.objects[0].mesh.indices.len() / 3;
    assert_eq!(tri_count, 12);
}

// ---------------------------------------------------------------------------
// Test 5: Large cap loop warning
// ---------------------------------------------------------------------------

/// Mesh with an open boundary loop of 300 vertices. The repair should emit
/// `RepairWarning::LargeCapLoop` and skip capping that loop.
#[test]
fn repair_large_cap_loop_warning() {
    // Build a tube-like mesh: a ring of 300 vertices at z=0 and z=10,
    // connected by 300 quads (600 triangles), but with the top cap missing
    // — creating a 300-vertex open boundary loop.
    let n = 300u32;
    let mut vertices = Vec::with_capacity((n * 2) as usize);
    let mut indices = Vec::new();

    let radius = 10.0_f32;
    for i in 0..n {
        let angle = 2.0 * std::f32::consts::PI * (i as f32) / (n as f32);
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        // Bottom ring
        vertices.push(Point3 { x, y, z: 0.0 });
        // Top ring
        vertices.push(Point3 { x, y, z: 10.0 });
    }

    // Side quads (2 triangles each).
    for i in 0..n {
        let bot_curr = i * 2;
        let top_curr = i * 2 + 1;
        let bot_next = ((i + 1) % n) * 2;
        let top_next = ((i + 1) % n) * 2 + 1;

        // Triangle 1
        indices.extend_from_slice(&[bot_curr, bot_next, top_curr]);
        // Triangle 2
        indices.extend_from_slice(&[top_curr, bot_next, top_next]);
    }

    // Add a bottom cap (closed) so only the top loop is open.
    // Fan from vertex 0 (bottom ring).
    for i in 1..n - 1 {
        indices.extend_from_slice(&[0, (i + 1) * 2, i * 2]);
    }

    // Top cap is intentionally missing → 300-vertex open boundary.

    let its = IndexedTriangleSet { vertices, indices };
    let mesh = single_object_mesh(its);
    let result: RepairResult = repair(mesh).expect("repair should not error");

    let has_large_cap_warning = result.stats.warnings.iter().any(|w| {
        matches!(
            w,
            RepairWarning::LargeCapLoop {
                vertex_count
            } if *vertex_count >= 300
        )
    });
    assert!(
        has_large_cap_warning,
        "expected RepairWarning::LargeCapLoop for 300-vertex boundary, got: {:?}",
        result.stats.warnings
    );
}
