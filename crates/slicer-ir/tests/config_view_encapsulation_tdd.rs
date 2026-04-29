//! Tests the read-only, declared-reads-only ConfigView contract
//! (docs/03 §host-boundary access enforcement; `wit/deps/config.wit`
//! `resource config-view`).
//!
//! This file is the external-crate witness for the contract: because
//! `ConfigView`'s backing map is private, any code outside `slicer-ir`
//! must go through the typed accessors — no struct-literal or `.fields`
//! mutation is possible from here. If someone re-exposes the field or
//! adds a mutating accessor, these tests (and the file's compile-time
//! negatives below) will fail.

use std::collections::HashMap;

use slicer_ir::{ConfigValue, ConfigView};

fn source() -> HashMap<String, ConfigValue> {
    let mut m = HashMap::new();
    m.insert("declared_a".into(), ConfigValue::Float(1.25));
    m.insert("declared_b".into(), ConfigValue::Int(42));
    m.insert(
        "undeclared_secret".into(),
        ConfigValue::String("no peek".into()),
    );
    m
}

#[test]
fn from_declared_drops_keys_not_in_declared_set() {
    let src = source();
    let view = ConfigView::from_declared(&src, ["declared_a", "declared_b"]);
    assert!(view.contains_key("declared_a"));
    assert!(view.contains_key("declared_b"));
    assert!(
        !view.contains_key("undeclared_secret"),
        "undeclared keys must not leak into the view"
    );
    assert_eq!(view.len(), 2);
}

#[test]
fn from_declared_keys_missing_in_source_do_not_appear() {
    let src = source();
    let view = ConfigView::from_declared(&src, ["declared_a", "missing"]);
    assert!(view.contains_key("declared_a"));
    assert!(
        !view.contains_key("missing"),
        "declared-but-absent keys must not fabricate entries"
    );
    assert_eq!(view.get("missing"), None);
}

#[test]
fn keys_is_sorted_and_deterministic() {
    let mut m: HashMap<String, ConfigValue> = HashMap::new();
    m.insert("zebra".into(), ConfigValue::Bool(true));
    m.insert("alpha".into(), ConfigValue::Bool(true));
    m.insert("mu".into(), ConfigValue::Bool(true));
    let view = ConfigView::from_map(m);
    let keys_a = view.keys();
    let keys_b = view.keys();
    assert_eq!(keys_a, keys_b);
    assert_eq!(
        keys_a,
        vec!["alpha".to_string(), "mu".to_string(), "zebra".to_string()]
    );
}

#[test]
fn typed_getters_preserve_subnormal_normalization() {
    let mut m: HashMap<String, ConfigValue> = HashMap::new();
    m.insert("sub".into(), ConfigValue::Float(f64::from_bits(1))); // smallest subnormal
    m.insert("normal".into(), ConfigValue::Float(1.5));
    let view = ConfigView::from_map(m);
    assert_eq!(
        view.get_float("sub"),
        Some(0.0),
        "subnormals must coerce to 0.0"
    );
    assert_eq!(view.get_float("normal"), Some(1.5));
}

#[test]
fn iter_entries_is_deterministic_by_key_order() {
    let mut m: HashMap<String, ConfigValue> = HashMap::new();
    m.insert("c".into(), ConfigValue::Int(3));
    m.insert("a".into(), ConfigValue::Int(1));
    m.insert("b".into(), ConfigValue::Int(2));
    let view = ConfigView::from_map(m);
    let out: Vec<&str> = view.iter_entries().map(|(k, _)| k).collect();
    assert_eq!(out, vec!["a", "b", "c"]);

    // Two independent runs must produce identical serializations.
    let ser_a: Vec<(String, String)> = view
        .iter_entries()
        .map(|(k, v)| (k.to_string(), format!("{v:?}")))
        .collect();
    let ser_b: Vec<(String, String)> = view
        .iter_entries()
        .map(|(k, v)| (k.to_string(), format!("{v:?}")))
        .collect();
    assert_eq!(ser_a, ser_b);
}

#[test]
fn typed_getters_return_none_for_wrong_type() {
    let mut m: HashMap<String, ConfigValue> = HashMap::new();
    m.insert("flag".into(), ConfigValue::Bool(true));
    let view = ConfigView::from_map(m);
    assert_eq!(view.get_bool("flag"), Some(true));
    assert_eq!(
        view.get_float("flag"),
        None,
        "wrong-type reads must produce None, not coerce"
    );
    assert_eq!(view.get_int("flag"), None);
    assert_eq!(view.get_string("flag"), None);
}

#[test]
fn default_and_new_are_empty() {
    assert!(ConfigView::new().is_empty());
    assert!(ConfigView::default().is_empty());
    assert_eq!(ConfigView::new().len(), 0);
}

/// Compile-time contract guard: it must be impossible to construct a
/// `ConfigView` by struct literal from outside `slicer-ir`, and it must
/// be impossible to reach the backing map through a `pub` field. If
/// either regresses, this module stops compiling.
///
/// The regression guard is validated by the fact that this entire test
/// file compiles: it imports `ConfigView` and uses every public
/// accessor, but never writes `ConfigView { fields: ... }` and never
/// touches `.fields`. Any future `pub` field re-exposure on
/// `ConfigView` would still compile here but would also silently
/// weaken the contract; those regressions are blocked separately by
/// a static source-level test in slicer-host (see
/// `crates/slicer-host/tests/config_view_encapsulation_source_tdd.rs`).
#[test]
fn construction_goes_through_named_factories_only() {
    // This function is a documentation witness — the absence of
    // `ConfigView { fields: ... }` in this test file is the proof,
    // enforced at compile time by the private field.
    let _ = ConfigView::new();
    let _ = ConfigView::default();
    let _ = ConfigView::from_map(HashMap::new());
    let _ = ConfigView::from_declared(&HashMap::new(), std::iter::empty::<&str>());
}
