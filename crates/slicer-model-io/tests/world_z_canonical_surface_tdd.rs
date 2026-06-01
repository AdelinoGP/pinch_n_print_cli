//! TDD regression tests for TASK-158: world-space Z canonical surface.
//!
//! These tests prove that `object_world_z_extent` is the sole canonical path
//! for reading an object's world-space Z range.  They lock in three invariants:
//!
//!   1. Identity transform   â†’ world Z == vertex Z (no accidental offset).
//!   2. Z translation        â†’ world Z = vertex Z + translate_z (transform is applied).
//!   3. Non-trivial mesh     â†’ world Z covers the full vertex span after transform.
//!
//! If any code path silently returns object-local Z (skipping the transform),
//! the translated-object tests will fail with the wrong numeric bounds.
//!
//! Acceptance criterion: `cargo test -p slicer-runtime --test world_z_canonical_surface_tdd`

#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{IndexedTriangleSet, ObjectConfig, ObjectMesh, Point3, Transform3d};
use slicer_model_io::loader::object_world_z_extent;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn identity_matrix() -> [f64; 16] {
    let mut m = [0.0f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    m
}

fn translation_matrix(tx: f64, ty: f64, tz: f64) -> [f64; 16] {
    // Column-major 4Ã—4: translation in column 3.
    let mut m = identity_matrix();
    m[12] = tx;
    m[13] = ty;
    m[14] = tz;
    m
}

fn make_object(vertices: Vec<Point3>, matrix: [f64; 16]) -> ObjectMesh {
    ObjectMesh {
        id: "test-object".to_string(),
        mesh: IndexedTriangleSet {
            vertices,
            indices: vec![],
        },
        transform: Transform3d { matrix },
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: vec![],
        paint_data: None,
        world_z_extent: None,
    }
}

// ---------------------------------------------------------------------------
// AC-1: identity transform â€” world Z equals vertex Z
// ---------------------------------------------------------------------------

/// A mesh whose vertices span Z 0..20 with an identity transform must report
/// world Z [0.0, 20.0].  Any code path that ignores the transform would still
/// pass here, so this test anchors the baseline.
#[test]
fn identity_transform_world_z_equals_vertex_z() {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 10.0,
            y: 10.0,
            z: 20.0,
        },
    ];
    let obj = make_object(vertices, identity_matrix());
    let (z_min, z_max) = object_world_z_extent(&obj)
        .expect("identity object with two vertices must yield an extent");

    assert!(
        (z_min - 0.0).abs() < 1e-5,
        "identity: z_min should be 0.0, got {z_min}"
    );
    assert!(
        (z_max - 20.0).abs() < 1e-5,
        "identity: z_max should be 20.0, got {z_max}"
    );
}

// ---------------------------------------------------------------------------
// AC-2: Z translation â€” world Z includes the translation offset
// ---------------------------------------------------------------------------

/// Translating an object +15 mm along Z must shift the world-space extent.
/// A code path that returns local vertex Z (0..5) instead of world Z (15..20)
/// would produce wrong bounds and fail this assertion.
#[test]
fn z_translation_shifts_world_z_extent() {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 5.0,
        },
    ];
    let obj = make_object(vertices, translation_matrix(0.0, 0.0, 15.0));
    let (z_min, z_max) =
        object_world_z_extent(&obj).expect("translated object must yield an extent");

    assert!(
        (z_min - 15.0).abs() < 1e-5,
        "translated: world z_min should be 15.0, not local 0.0 â€” got {z_min}"
    );
    assert!(
        (z_max - 20.0).abs() < 1e-5,
        "translated: world z_max should be 20.0, not local 5.0 â€” got {z_max}"
    );
}

/// Negative Z translation must produce negative world-space Z.
/// This confirms the function is not clamping to local-space values.
#[test]
fn negative_z_translation_gives_negative_world_z() {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 10.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 20.0,
        },
    ];
    // Shift down by 25 mm â†’ world Z spans [-15, -5].
    let obj = make_object(vertices, translation_matrix(0.0, 0.0, -25.0));
    let (z_min, z_max) =
        object_world_z_extent(&obj).expect("downward-translated object must yield an extent");

    assert!(
        (z_min - (-15.0)).abs() < 1e-5,
        "world z_min should be -15.0, got {z_min}"
    );
    assert!(
        (z_max - (-5.0)).abs() < 1e-5,
        "world z_max should be -5.0, got {z_max}"
    );
}

// ---------------------------------------------------------------------------
// AC-3: world Z covers the full vertex span (multi-vertex mesh)
// ---------------------------------------------------------------------------

/// A mesh with many vertices at different Z values must have its world-space
/// extent equal to the full span [min_vertex_z, max_vertex_z] when the
/// transform is identity.
#[test]
fn full_vertex_span_covered_by_world_z_extent() {
    let vertices: Vec<Point3> = (0..=10)
        .map(|i| Point3 {
            x: i as f32,
            y: 0.0,
            z: i as f32 * 4.8,
        })
        .collect();
    let expected_z_min = 0.0_f32;
    let expected_z_max = 10.0 * 4.8_f32;

    let obj = make_object(vertices, identity_matrix());
    let (z_min, z_max) =
        object_world_z_extent(&obj).expect("multi-vertex object must yield an extent");

    assert!(
        (z_min - expected_z_min).abs() < 1e-4,
        "multi-vertex z_min should be {expected_z_min}, got {z_min}"
    );
    assert!(
        (z_max - expected_z_max).abs() < 1e-4,
        "multi-vertex z_max should be {expected_z_max}, got {z_max}"
    );
}

// ---------------------------------------------------------------------------
// AC-4: zero-matrix transform is treated as identity (canonical convention)
// ---------------------------------------------------------------------------

/// A zero-matrix (all elements zero) must be treated as identity per the
/// canonical convention.  Fixtures that leave `Transform3d::matrix` uninitialised
/// all-zero must still get correct world-space Z.
#[test]
fn zero_matrix_treated_as_identity_canonical_surface() {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 11.0,
        },
    ];
    let obj = make_object(vertices, [0.0f64; 16]);
    let (z_min, z_max) = object_world_z_extent(&obj)
        .expect("zero-matrix object must yield extent using identity convention");

    assert!(
        (z_min - 3.0).abs() < 1e-5,
        "zero-matrix z_min should be 3.0 (identity), got {z_min}"
    );
    assert!(
        (z_max - 11.0).abs() < 1e-5,
        "zero-matrix z_max should be 11.0 (identity), got {z_max}"
    );
}

// ---------------------------------------------------------------------------
// AC-5: XY translation does not alter world Z
// ---------------------------------------------------------------------------

/// A purely lateral (XY) translation must leave world-space Z unchanged
/// relative to the vertex coordinates.
#[test]
fn xy_translation_does_not_change_world_z() {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 8.0,
        },
    ];
    let obj = make_object(vertices, translation_matrix(100.0, 50.0, 0.0));
    let (z_min, z_max) =
        object_world_z_extent(&obj).expect("XY-translated object must yield an extent");

    assert!(
        (z_min - 2.0).abs() < 1e-5,
        "XY-translate: world z_min should still be 2.0, got {z_min}"
    );
    assert!(
        (z_max - 8.0).abs() < 1e-5,
        "XY-translate: world z_max should still be 8.0, got {z_max}"
    );
}
