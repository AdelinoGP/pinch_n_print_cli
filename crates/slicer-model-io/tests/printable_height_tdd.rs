//! TDD tests for the build-volume height gate (wayfinder map ticket 26, P19).
//!
//! These tests prove that `validate_printable_height` rejects any `ObjectMesh`
//! whose world-space Z maximum is above the configured `printable_height`, the
//! condition canonical rejects in `Print::validate` (`Print.cpp`).
//!
//! Acceptance criteria:
//!   AC-1: object below the limit                 -> Ok(())
//!   AC-2: object above the limit                 -> Err(ExceedsPrintableHeight)
//!   AC-3: translation pushes a fitting object over the limit -> Err
//!   AC-4: object exactly at the limit            -> Ok(()) (strict `>` compare)
//!   AC-5: empty mesh                             -> Ok(()) (no geometry)
//!   AC-6: non-positive limit disables the check  -> Ok(())
//!   AC-7: error carries the object id, z_max and printable_height
//!   AC-8: error message contains "EXCEEDS_PRINTABLE_HEIGHT"
//!
//! Verification: `cargo test -p slicer-model-io --test printable_height_tdd`

#![allow(missing_docs)]

mod common;

use slicer_ir::{IndexedTriangleSet, ObjectMesh, Point3, Transform3d};
use slicer_model_io::loader::{validate_printable_height, ModelLoadError};

fn identity_matrix() -> [f64; 16] {
    let mut m = [0.0f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    m
}

fn translation_matrix(tz: f64) -> [f64; 16] {
    let mut m = identity_matrix();
    m[14] = tz;
    m
}

fn make_object(id: &str, z_top: f32, matrix: [f64; 16]) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 10.0,
                    y: 10.0,
                    z: z_top,
                },
            ],
            indices: vec![],
        },
        transform: Transform3d { matrix },
        ..common::object_mesh_base()
    }
}

// AC-1
#[test]
fn object_under_the_limit_is_valid() {
    let obj = make_object("short", 80.0, identity_matrix());
    assert!(
        validate_printable_height(&obj, 250.0).is_ok(),
        "an 80mm object must fit a 250mm build volume"
    );
}

// AC-2
#[test]
fn object_over_the_limit_is_rejected() {
    let obj = make_object("tall", 300.0, identity_matrix());
    let err = validate_printable_height(&obj, 250.0)
        .expect_err("a 300mm object must not fit a 250mm build volume");
    assert!(
        matches!(err, ModelLoadError::ExceedsPrintableHeight { .. }),
        "expected ExceedsPrintableHeight, got {err:?}"
    );
}

// AC-3: the check reads world space, not model space.
#[test]
fn translation_can_push_a_fitting_object_over_the_limit() {
    let obj = make_object("lifted", 200.0, identity_matrix());
    assert!(
        validate_printable_height(&obj, 250.0).is_ok(),
        "untranslated 200mm object fits"
    );

    let lifted = make_object("lifted", 200.0, translation_matrix(100.0));
    let err = validate_printable_height(&lifted, 250.0)
        .expect_err("the same object lifted 100mm reaches 300mm and must be rejected");
    match err {
        ModelLoadError::ExceedsPrintableHeight { z_max, .. } => {
            assert!(
                (z_max - 300.0).abs() < 1e-3,
                "z_max must be the world-space maximum, got {z_max}"
            );
        }
        other => panic!("expected ExceedsPrintableHeight, got {other:?}"),
    }
}

// AC-4: canonical compares with a strict `>`, so exactly-at-the-limit fits.
#[test]
fn object_exactly_at_the_limit_is_valid() {
    let obj = make_object("exact", 250.0, identity_matrix());
    assert!(
        validate_printable_height(&obj, 250.0).is_ok(),
        "an object exactly at the limit must fit"
    );
}

// AC-5
#[test]
fn empty_mesh_is_valid() {
    let obj = ObjectMesh {
        id: "empty".to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![],
            indices: vec![],
        },
        transform: Transform3d {
            matrix: identity_matrix(),
        },
        ..common::object_mesh_base()
    };
    assert!(
        validate_printable_height(&obj, 10.0).is_ok(),
        "an empty mesh has no geometry to reject"
    );
}

// AC-6
#[test]
fn non_positive_limit_disables_the_check() {
    let obj = make_object("tall", 1000.0, identity_matrix());
    assert!(
        validate_printable_height(&obj, 0.0).is_ok(),
        "a zero limit must not reject every object"
    );
    assert!(
        validate_printable_height(&obj, -5.0).is_ok(),
        "a negative limit must not reject every object"
    );
}

// AC-7
#[test]
fn error_carries_object_id_and_both_heights() {
    let obj = make_object("cube-a", 300.0, identity_matrix());
    let err = validate_printable_height(&obj, 250.0).expect_err("must reject");
    match err {
        ModelLoadError::ExceedsPrintableHeight {
            object_id,
            z_max,
            printable_height,
        } => {
            assert_eq!(object_id, "cube-a");
            assert!((z_max - 300.0).abs() < 1e-3, "z_max was {z_max}");
            assert!(
                (printable_height - 250.0).abs() < 1e-3,
                "printable_height was {printable_height}"
            );
        }
        other => panic!("expected ExceedsPrintableHeight, got {other:?}"),
    }
}

// AC-8
#[test]
fn error_message_carries_the_stable_code() {
    let obj = make_object("cube-a", 300.0, identity_matrix());
    let err = validate_printable_height(&obj, 250.0).expect_err("must reject");
    let msg = err.to_string();
    assert!(
        msg.contains("EXCEEDS_PRINTABLE_HEIGHT"),
        "message must carry the stable code, got: {msg}"
    );
    assert!(
        msg.contains("cube-a"),
        "message must name the object, got: {msg}"
    );
}
