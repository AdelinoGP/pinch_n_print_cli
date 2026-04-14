//! Typed `config-view` boundary semantics for the WIT host bridge.
//!
//! Mirrors the in-process typed accessors on `slicer_ir::ConfigView`
//! (docs/05_module_sdk.md §SDK config) so that a module observes identical
//! numeric semantics whether it reads config in-process or over the WASM
//! component-model boundary. These tests lock in subnormal-normalization
//! parity between `slicer-ir::ConfigView::get_float` and
//! `wit_host::normalize_subnormal_boundary`, plus the pass-through behavior
//! of finite/non-finite values.

#![allow(missing_docs)]

use slicer_host::wit_host::{config_value_to_storage, normalize_subnormal_boundary};

#[test]
fn boundary_normalizes_subnormal_to_zero() {
    let subnormal = f64::from_bits(1);
    assert!(subnormal.is_subnormal());
    assert_eq!(normalize_subnormal_boundary(subnormal), 0.0);
    assert_eq!(normalize_subnormal_boundary(-subnormal), 0.0);
}

#[test]
fn boundary_passes_through_normal_values() {
    for v in [0.0_f64, 1.0, -1.5, 1e-200, 1e200, f64::MIN_POSITIVE] {
        assert_eq!(normalize_subnormal_boundary(v), v);
    }
}

#[test]
fn boundary_preserves_nan_and_infinity() {
    assert!(normalize_subnormal_boundary(f64::NAN).is_nan());
    assert_eq!(normalize_subnormal_boundary(f64::INFINITY), f64::INFINITY);
    assert_eq!(
        normalize_subnormal_boundary(f64::NEG_INFINITY),
        f64::NEG_INFINITY
    );
}

#[test]
fn config_value_to_storage_round_trips_basic_types() {
    use slicer_ir::ConfigValue;
    use slicer_host::wit_host::ConfigValueStorage;

    assert!(matches!(
        config_value_to_storage(&ConfigValue::Bool(true)),
        ConfigValueStorage::Bool(true)
    ));
    assert!(matches!(
        config_value_to_storage(&ConfigValue::Int(7)),
        ConfigValueStorage::Int(7)
    ));
    assert!(matches!(
        config_value_to_storage(&ConfigValue::Float(0.25)),
        ConfigValueStorage::Float(v) if v == 0.25
    ));
    assert!(matches!(
        config_value_to_storage(&ConfigValue::String("x".into())),
        ConfigValueStorage::Str(s) if s == "x"
    ));
}

#[test]
fn config_value_to_storage_homogeneous_float_list_collapses() {
    use slicer_ir::ConfigValue;
    use slicer_host::wit_host::ConfigValueStorage;

    let v = ConfigValue::List(vec![ConfigValue::Float(1.0), ConfigValue::Float(2.0)]);
    match config_value_to_storage(&v) {
        ConfigValueStorage::FloatList(xs) => assert_eq!(xs, vec![1.0, 2.0]),
        other => panic!("expected FloatList, got {other:?}"),
    }
}
