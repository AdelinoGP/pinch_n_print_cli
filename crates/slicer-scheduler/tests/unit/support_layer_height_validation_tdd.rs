//! TDD coverage for `validate_support_layer_heights`.
//!
//! Rule under test (see `config_resolution::validate_support_layer_heights`):
//! per-object `support_layer_height_mm` of `0.0` means "use the object's
//! effective layer height" (historical default); any non-zero value MUST be
//! at least the object's effective layer height (the printer cannot extrude
//! a support layer thinner than the model layer).
//!
//! Five cases:
//!   1. default (0.0) accepted
//!   2. equal to object layer height accepted
//!   3. coarser than object layer height accepted
//!   4. finer than object layer height rejected with SupportLayerHeightTooFine
//!   5. multi-object with mixed layer heights — fails on first offender

#![allow(missing_docs)]

use std::collections::BTreeMap;

use slicer_ir::ResolvedConfig;
use slicer_scheduler::{validate_support_layer_heights, ConfigResolutionError};

fn cfg(layer_height: f64, support_layer_height_mm: f32) -> ResolvedConfig {
    ResolvedConfig {
        layer_height,
        support_layer_height_mm,
        ..ResolvedConfig::default()
    }
}

#[test]
fn case_1_default_zero_support_layer_height_is_accepted() {
    let mut map = BTreeMap::new();
    map.insert("obj-a".to_string(), cfg(0.2, 0.0));

    validate_support_layer_heights(&map)
        .expect("default 0.0 support_layer_height_mm must pass validation");
}

#[test]
fn case_2_equal_to_object_layer_height_is_accepted() {
    let mut map = BTreeMap::new();
    map.insert("obj-b".to_string(), cfg(0.2, 0.2));

    validate_support_layer_heights(&map)
        .expect("support layer height equal to object layer height must pass");
}

#[test]
fn case_3_coarser_than_object_layer_height_is_accepted() {
    let mut map = BTreeMap::new();
    map.insert("obj-c".to_string(), cfg(0.2, 0.4));

    validate_support_layer_heights(&map)
        .expect("coarser support layer height (0.4mm vs 0.2mm) must pass");
}

#[test]
fn case_4_finer_than_object_layer_height_is_rejected() {
    let mut map = BTreeMap::new();
    map.insert("obj-d".to_string(), cfg(0.2, 0.1));

    let err = validate_support_layer_heights(&map)
        .expect_err("finer support layer height (0.1mm vs 0.2mm) must fail");
    match err {
        ConfigResolutionError::SupportLayerHeightTooFine {
            object_id,
            support_layer_height_mm,
            effective_layer_height_mm,
        } => {
            assert_eq!(object_id, "obj-d");
            assert!((support_layer_height_mm - 0.1).abs() < 1e-6);
            assert!((effective_layer_height_mm - 0.2).abs() < 1e-6);
        }
        other => panic!("expected SupportLayerHeightTooFine, got {other:?}"),
    }
}

#[test]
fn case_5_multi_object_mixed_layer_heights_fails_on_offender() {
    let mut map = BTreeMap::new();
    // BTreeMap iterates in key order, so the validation walks alphabetically.
    // Object 'a' (compatible) must not trip the gate; the offender is 'b'.
    map.insert("obj-a-ok".to_string(), cfg(0.1, 0.2));
    map.insert("obj-b-bad".to_string(), cfg(0.3, 0.1));
    map.insert("obj-c-ok".to_string(), cfg(0.2, 0.0));

    let err = validate_support_layer_heights(&map)
        .expect_err("multi-object scan must reject the finer-than-layer offender");
    match err {
        ConfigResolutionError::SupportLayerHeightTooFine {
            object_id,
            support_layer_height_mm,
            effective_layer_height_mm,
        } => {
            assert_eq!(object_id, "obj-b-bad");
            assert!((support_layer_height_mm - 0.1).abs() < 1e-6);
            assert!((effective_layer_height_mm - 0.3).abs() < 1e-6);
        }
        other => panic!("expected SupportLayerHeightTooFine, got {other:?}"),
    }
}
