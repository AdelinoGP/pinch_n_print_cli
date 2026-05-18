//! TDD acceptance tests for the host-side config resolver.
//!
//! These tests cover the four acceptance criteria pinned by packet
//! 35a_resolved-config-propagation Step 2.

use std::collections::HashMap;

use slicer_host::{
    resolve_global_config, resolve_per_object_configs, ConfigBoundsIndex, ConfigResolutionError,
};
use slicer_ir::ConfigValue;

/// AC-1: A known field (top_shell_layers) is applied; unlisted fields keep
/// their defaults; extensions must be empty.
#[test]
fn resolver_maps_top_shell_layers() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert("top_shell_layers".to_string(), ConfigValue::Int(4));

    let bounds = ConfigBoundsIndex::empty();
    let resolved = resolve_global_config(&source, &bounds).expect("resolution should succeed");

    assert_eq!(resolved.top_shell_layers, 4, "top_shell_layers should be 4");
    assert_eq!(
        resolved.bottom_shell_layers, 3,
        "bottom_shell_layers should keep default (3)"
    );
    assert!(
        resolved.extensions.is_empty(),
        "extensions must be empty when no unknown keys are present"
    );
}

/// AC-2: An unknown key is routed to extensions; a known key is still applied.
#[test]
fn resolver_unknown_key_routes_to_extensions() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert("top_shell_layers".to_string(), ConfigValue::Int(2));
    source.insert(
        "experimental_xyz".to_string(),
        ConfigValue::String("on".to_string()),
    );

    let bounds = ConfigBoundsIndex::empty();
    let resolved = resolve_global_config(&source, &bounds).expect("resolution should succeed");

    assert_eq!(resolved.top_shell_layers, 2);
    assert_eq!(
        resolved.extensions.get("experimental_xyz"),
        Some(&ConfigValue::String("on".to_string())),
        "unknown key should land in extensions"
    );
}

/// AC-3: Per-object overrides are applied independently; non-overridden objects
/// inherit the global value.
#[test]
fn resolver_per_object_overrides_global() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert("top_shell_layers".to_string(), ConfigValue::Int(3));
    source.insert(
        "object_config:obj-A:top_shell_layers".to_string(),
        ConfigValue::Int(5),
    );

    let bounds = ConfigBoundsIndex::empty();
    let global = resolve_global_config(&source, &bounds).expect("global resolution should succeed");
    assert_eq!(global.top_shell_layers, 3);

    let per_object = resolve_per_object_configs(&global, &source, &["obj-A", "obj-B"], &bounds)
        .expect("per-object resolution should succeed");

    // BTreeMap ordering: obj-A < obj-B alphabetically.
    let obj_a = per_object.get("obj-A").expect("obj-A must be present");
    let obj_b = per_object.get("obj-B").expect("obj-B must be present");

    assert_eq!(
        obj_a.top_shell_layers, 5,
        "obj-A override should be applied"
    );
    assert_eq!(
        obj_b.top_shell_layers, 3,
        "obj-B should inherit global value"
    );

    // Verify deterministic BTreeMap ordering.
    let keys: Vec<&String> = per_object.keys().collect();
    assert_eq!(keys, vec!["obj-A", "obj-B"]);
}

/// AC-4: Supplying a String value for an Int field returns a TypeMismatch error.
#[test]
fn resolver_rejects_string_for_top_shell_layers() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "top_shell_layers".to_string(),
        ConfigValue::String("four".to_string()),
    );

    let bounds = ConfigBoundsIndex::empty();
    let err = resolve_global_config(&source, &bounds).expect_err("should fail on type mismatch");

    match err {
        ConfigResolutionError::TypeMismatch {
            key,
            expected,
            actual,
        } => {
            assert_eq!(key, "top_shell_layers");
            assert_eq!(expected, "Int");
            assert!(
                actual.contains("String"),
                "actual variant should mention 'String', got: {actual}"
            );
        }
        other => panic!("expected TypeMismatch, got {other:?}"),
    }
}
