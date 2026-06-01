//! TDD regression tests for TASK-157/TASK-158: WORLD_Z_BELOW_FLOOR.
//!
//! These tests prove that `validate_world_z_floor` rejects any `ObjectMesh`
//! whose world-space Z minimum is negative (below the print volume floor at Z=0).
//!
//! Acceptance criteria:
//!   AC-1: object at Z=0 or above           â†’ Ok(())
//!   AC-2: object translated -5mm in Z      â†’ Err(WorldZBelowFloor { z_min: -5.0 })
//!   AC-3: object translated +10mm in Z     â†’ Ok(())
//!   AC-4: object straddles Z=0 (min < 0)   â†’ Err(WorldZBelowFloor)
//!   AC-5: empty mesh                        â†’ Ok(()) (no geometry to check)
//!   AC-6: error message contains "WORLD_Z_BELOW_FLOOR"
//!   AC-7: identity object at Z â‰¥ 0         â†’ Ok(())
//!   AC-8: z_min field in error is accurate  â†’ z_min matches expected value
//!
//! Verification: `cargo test -p slicer-runtime --test world_z_below_floor_tdd -- --nocapture`

#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{IndexedTriangleSet, ObjectConfig, ObjectMesh, Point3, Transform3d};
use slicer_model_io::loader::{validate_world_z_floor, ModelLoadError};

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
        world_z_extent: None, // computed by caller if needed; validator uses live math
    }
}

// ---------------------------------------------------------------------------
// AC-1: object sitting on the floor (Z â‰¥ 0) â†’ Ok(())
// ---------------------------------------------------------------------------

/// An object whose vertices start at Z=0 with an identity transform is valid.
#[test]
fn object_at_z_zero_is_valid() {
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
    let result = validate_world_z_floor(&obj);
    println!("object_at_z_zero: {result:?}");
    assert!(
        result.is_ok(),
        "object at Z=0 should be Ok, got: {result:?}"
    );
}

/// An object entirely above Z=0 is valid.
#[test]
fn object_above_floor_is_valid() {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 5.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 30.0,
        },
    ];
    let obj = make_object(vertices, identity_matrix());
    let result = validate_world_z_floor(&obj);
    println!("object_above_floor: {result:?}");
    assert!(
        result.is_ok(),
        "object entirely above floor should be Ok, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-2: object translated -5mm in Z â†’ Err(WorldZBelowFloor)
// ---------------------------------------------------------------------------

/// An object with translate(0, 0, -5mm) produces WORLD_Z_BELOW_FLOOR.
/// This directly exercises the scenario described in the spec: translate(0,0,-5mm).
#[test]
fn object_translated_5mm_below_floor_is_rejected() {
    // Vertices from Z=0 to Z=10 in local space, then translated -5mm in Z.
    // World Z: -5 to +5.  z_min = -5 â†’ below floor.
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 10.0,
        },
    ];
    let obj = make_object(vertices, translation_matrix(0.0, 0.0, -5.0));
    let result = validate_world_z_floor(&obj);
    println!("translate_z_neg5: {result:?}");
    assert!(
        result.is_err(),
        "translate(0,0,-5mm) should produce WorldZBelowFloor, got Ok"
    );
    assert!(
        matches!(result, Err(ModelLoadError::WorldZBelowFloor { .. })),
        "error should be WorldZBelowFloor, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-3: object translated +10mm in Z â†’ Ok(())
// ---------------------------------------------------------------------------

/// A positive Z translation keeps the object above the floor.
#[test]
fn object_translated_up_is_valid() {
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
    let obj = make_object(vertices, translation_matrix(0.0, 0.0, 10.0));
    let result = validate_world_z_floor(&obj);
    println!("translate_z_pos10: {result:?}");
    assert!(
        result.is_ok(),
        "upward translation should be Ok, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-4: object straddles Z=0 (local min below 0) â†’ Err(WorldZBelowFloor)
// ---------------------------------------------------------------------------

/// A mesh with vertices at Z=-1 to Z=5 (no transform) violates the floor.
#[test]
fn object_straddling_floor_is_rejected() {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 5.0,
        },
    ];
    let obj = make_object(vertices, identity_matrix());
    let result = validate_world_z_floor(&obj);
    println!("straddle_floor: {result:?}");
    assert!(
        result.is_err(),
        "object straddling Z=0 should be rejected, got Ok"
    );
    assert!(
        matches!(result, Err(ModelLoadError::WorldZBelowFloor { .. })),
        "error should be WorldZBelowFloor, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-5: empty mesh â†’ Ok(()) (no geometry to violate the floor)
// ---------------------------------------------------------------------------

/// A mesh with no vertices cannot have a world-space Z below the floor.
/// The validator treats this as Ok â€” later stages catch empty meshes separately.
#[test]
fn empty_mesh_is_valid() {
    let obj = make_object(vec![], identity_matrix());
    let result = validate_world_z_floor(&obj);
    println!("empty_mesh: {result:?}");
    assert!(
        result.is_ok(),
        "empty mesh should be Ok (no geometry to check), got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-6: error Display contains "WORLD_Z_BELOW_FLOOR"
// ---------------------------------------------------------------------------

/// The Display output of the error must contain the canonical code string
/// for downstream diagnostic tools to key on.
#[test]
fn error_display_contains_error_code() {
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
    let obj = make_object(vertices, translation_matrix(0.0, 0.0, -10.0));
    let err = validate_world_z_floor(&obj).unwrap_err();
    let msg = err.to_string();
    println!("error_display: {msg}");
    assert!(
        msg.contains("WORLD_Z_BELOW_FLOOR"),
        "Display should contain 'WORLD_Z_BELOW_FLOOR', got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// AC-7: identity object at Z â‰¥ 0 â†’ Ok(())
// ---------------------------------------------------------------------------

/// An object sitting right at Z=0 with vertices from 0 to positive values,
/// using an identity transform, must pass.
#[test]
fn identity_object_at_z_zero_passes() {
    let vertices = vec![
        Point3 {
            x: 5.0,
            y: 5.0,
            z: 0.0,
        },
        Point3 {
            x: 10.0,
            y: 10.0,
            z: 48.0,
        }, // Benchy-like height
    ];
    let obj = make_object(vertices, identity_matrix());
    let result = validate_world_z_floor(&obj);
    println!("identity_at_z_zero: {result:?}");
    assert!(
        result.is_ok(),
        "identity object at Z=0 should pass, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-8: z_min field in error is accurate
// ---------------------------------------------------------------------------

/// The `z_min` reported in the error must match the actual computed world-space
/// minimum Z so callers can show users an accurate diagnostic.
#[test]
fn error_z_min_field_is_accurate() {
    // Vertices at local Z 2.0..12.0, translated by -15mm â†’ world Z -13..âˆ’3.
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 12.0,
        },
    ];
    let obj = make_object(vertices, translation_matrix(0.0, 0.0, -15.0));
    let err = validate_world_z_floor(&obj).unwrap_err();
    if let ModelLoadError::WorldZBelowFloor { z_min } = err {
        println!("error_z_min_field: z_min={z_min}");
        assert!(
            (z_min - (-13.0_f32)).abs() < 1e-4,
            "z_min should be -13.0, got {z_min}"
        );
    } else {
        panic!("expected WorldZBelowFloor, got a different error");
    }
}

// ---------------------------------------------------------------------------
// Extra: large negative Z translation â†’ Err(WorldZBelowFloor)
// ---------------------------------------------------------------------------

/// Sanity check: an object deep below the floor is still caught.
#[test]
fn large_negative_z_translation_is_rejected() {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 100.0,
        },
    ];
    // translate -200mm â†’ world Z -200 .. -100
    let obj = make_object(vertices, translation_matrix(0.0, 0.0, -200.0));
    let result = validate_world_z_floor(&obj);
    println!("large_neg_z: {result:?}");
    assert!(
        result.is_err(),
        "large negative Z translation should be rejected, got Ok"
    );
}
