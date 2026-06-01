//! TDD regression tests for TASK-157/TASK-158: NON_UNIFORM_SCALE_UNSUPPORTED.
//!
//! These tests prove that `validate_non_uniform_scale` rejects any `ObjectMesh`
//! whose transform applies different scale factors along X, Y, and Z.
//!
//! The slicer pipeline cannot correctly slice geometry that has been stretched
//! along one or two axes without compensating â€” so non-uniform scale is a fatal
//! load-time error.
//!
//! Acceptance criteria:
//!   AC-1: uniform scale       â†’ Ok(())
//!   AC-2: non-uniform Xâ‰ Y    â†’ Err(NonUniformScaleUnsupported)
//!   AC-3: non-uniform Yâ‰ Z    â†’ Err(NonUniformScaleUnsupported)
//!   AC-4: all axes different  â†’ Err(NonUniformScaleUnsupported)
//!   AC-5: identity transform  â†’ Ok(())
//!   AC-6: zero matrix         â†’ Ok(()) (treated as identity)
//!   AC-N1: error message contains "NON_UNIFORM_SCALE_UNSUPPORTED"
//!
//! Verification: `cargo test -p slicer-runtime --test non_uniform_scale_tdd -- --nocapture`

#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{IndexedTriangleSet, ObjectConfig, ObjectMesh, Point3, Transform3d};
use slicer_model_io::loader::{validate_non_uniform_scale, ModelLoadError};

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

/// Build a scale matrix with independent X/Y/Z scale factors.
/// Column-major 4Ã—4: diagonal entries are (sx, sy, sz, 1).
fn scale_matrix(sx: f64, sy: f64, sz: f64) -> [f64; 16] {
    let mut m = [0.0f64; 16];
    m[0] = sx; // col 0, row 0
    m[5] = sy; // col 1, row 1
    m[10] = sz; // col 2, row 2
    m[15] = 1.0;
    m
}

fn make_object(matrix: [f64; 16]) -> ObjectMesh {
    ObjectMesh {
        id: "test-object".to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            ],
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
// AC-1: uniform scale â†’ Ok(())
// ---------------------------------------------------------------------------

/// Scale 2Ã— uniformly on all three axes is valid.
#[test]
fn uniform_scale_2x_is_valid() {
    let obj = make_object(scale_matrix(2.0, 2.0, 2.0));
    let result = validate_non_uniform_scale(&obj);
    println!("uniform_scale_2x: {result:?}");
    assert!(
        result.is_ok(),
        "uniform scale 2Ã— should be Ok, got: {result:?}"
    );
}

/// Scale 0.5Ã— uniformly on all three axes is valid.
#[test]
fn uniform_scale_half_is_valid() {
    let obj = make_object(scale_matrix(0.5, 0.5, 0.5));
    let result = validate_non_uniform_scale(&obj);
    println!("uniform_scale_half: {result:?}");
    assert!(
        result.is_ok(),
        "uniform scale 0.5Ã— should be Ok, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-2: non-uniform X â‰  Y â†’ Err(NonUniformScaleUnsupported)
// ---------------------------------------------------------------------------

/// Different scale on X vs Y is rejected.
#[test]
fn non_uniform_scale_x_ne_y_is_rejected() {
    let obj = make_object(scale_matrix(2.0, 1.0, 2.0));
    let result = validate_non_uniform_scale(&obj);
    println!("non_uniform_scale_x_ne_y: {result:?}");
    assert!(
        result.is_err(),
        "scale_x=2.0, scale_y=1.0, scale_z=2.0 should be Err, got Ok"
    );
    assert!(
        matches!(
            result,
            Err(ModelLoadError::NonUniformScaleUnsupported { .. })
        ),
        "error should be NonUniformScaleUnsupported, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-3: non-uniform Y â‰  Z â†’ Err(NonUniformScaleUnsupported)
// ---------------------------------------------------------------------------

/// Different scale on Y vs Z is rejected.
#[test]
fn non_uniform_scale_y_ne_z_is_rejected() {
    let obj = make_object(scale_matrix(1.0, 1.0, 3.0));
    let result = validate_non_uniform_scale(&obj);
    println!("non_uniform_scale_y_ne_z: {result:?}");
    assert!(
        result.is_err(),
        "scale_x=1.0, scale_y=1.0, scale_z=3.0 should be Err, got Ok"
    );
    assert!(
        matches!(
            result,
            Err(ModelLoadError::NonUniformScaleUnsupported { .. })
        ),
        "error should be NonUniformScaleUnsupported, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-4: all three axes different â†’ Err(NonUniformScaleUnsupported)
// ---------------------------------------------------------------------------

/// A transform that stretches X by 1.5Ã—, Y by 2.0Ã—, Z by 3.0Ã— is rejected.
#[test]
fn non_uniform_scale_all_axes_different_is_rejected() {
    let obj = make_object(scale_matrix(1.5, 2.0, 3.0));
    let result = validate_non_uniform_scale(&obj);
    println!("non_uniform_scale_all_different: {result:?}");
    assert!(
        result.is_err(),
        "non-uniform scale (1.5, 2.0, 3.0) should be Err, got Ok"
    );
    if let Err(ModelLoadError::NonUniformScaleUnsupported {
        scale_x,
        scale_y,
        scale_z,
    }) = &result
    {
        assert!(
            (scale_x - 1.5).abs() < 1e-5,
            "scale_x should be ~1.5, got {scale_x}"
        );
        assert!(
            (scale_y - 2.0).abs() < 1e-5,
            "scale_y should be ~2.0, got {scale_y}"
        );
        assert!(
            (scale_z - 3.0).abs() < 1e-5,
            "scale_z should be ~3.0, got {scale_z}"
        );
    } else {
        panic!("expected NonUniformScaleUnsupported, got: {result:?}");
    }
}

// ---------------------------------------------------------------------------
// AC-5: identity transform â†’ Ok(())
// ---------------------------------------------------------------------------

/// The identity transform has uniform scale 1.0 on all axes.
#[test]
fn identity_transform_is_valid() {
    let obj = make_object(identity_matrix());
    let result = validate_non_uniform_scale(&obj);
    println!("identity_transform: {result:?}");
    assert!(
        result.is_ok(),
        "identity transform should be Ok, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-6: zero matrix treated as identity â†’ Ok(())
// ---------------------------------------------------------------------------

/// A zero matrix is treated as identity (uniform scale 1.0) per the canonical
/// convention.
#[test]
fn zero_matrix_treated_as_identity_is_valid() {
    let obj = make_object([0.0f64; 16]);
    let result = validate_non_uniform_scale(&obj);
    println!("zero_matrix: {result:?}");
    assert!(
        result.is_ok(),
        "zero matrix (identity convention) should be Ok, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-N1: error Display contains "NON_UNIFORM_SCALE_UNSUPPORTED"
// ---------------------------------------------------------------------------

/// The Display representation of the error must contain the canonical error
/// code string so downstream diagnostics can key on it.
#[test]
fn error_display_contains_error_code() {
    let obj = make_object(scale_matrix(1.0, 2.0, 1.0));
    let err = validate_non_uniform_scale(&obj).unwrap_err();
    let msg = err.to_string();
    println!("error_display: {msg}");
    assert!(
        msg.contains("NON_UNIFORM_SCALE_UNSUPPORTED"),
        "Display string should contain 'NON_UNIFORM_SCALE_UNSUPPORTED', got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Extra: uniform-scale with translation is still valid
// ---------------------------------------------------------------------------

/// A combined scale+translate transform (uniform scale 2Ã—, translate 5mm Z)
/// must still be accepted since the scale is uniform.
#[test]
fn uniform_scale_with_z_translation_is_valid() {
    let mut m = scale_matrix(2.0, 2.0, 2.0);
    m[14] = 5.0; // Z translation in column-major layout
    let obj = make_object(m);
    let result = validate_non_uniform_scale(&obj);
    println!("uniform_scale_with_translation: {result:?}");
    assert!(
        result.is_ok(),
        "uniform scale 2Ã— with Z translation should be Ok, got: {result:?}"
    );
}
