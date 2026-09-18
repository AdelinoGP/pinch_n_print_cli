//! TDD acceptance tests for the host-side config resolver.
//!
//! These tests cover the four acceptance criteria pinned by packet
//! 35a_resolved-config-propagation Step 2.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use slicer_config::{ConfigScope, ExpansionContext, ResolutionError, ResolutionTarget};
use slicer_ir::{resolved_config::ResolvedFloatOrPercent, ConfigValue, SemVer};
use slicer_scheduler::{
    config_resolution::{ingest_resolution_config, resolve_config},
    ConfigBoundsIndex, ConfigFieldEntry, ConfigResolutionError, ConfigSchema, LoadedModuleBuilder,
};

fn resolve(
    source: &HashMap<String, ConfigValue>,
    bounds: &ConfigBoundsIndex,
    target: &ResolutionTarget,
) -> Result<slicer_ir::ResolvedConfig, ResolutionError> {
    resolve_config(
        source,
        bounds,
        target,
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
    )
}

/// Build a one-module `ConfigBoundsIndex` whose schema declares
/// `experimental_percent` as a `float_or_percent` field with a parsed
/// `"50%"` default — the same shape `read_config_schema` produces for a real
/// `[config.schema.*]` manifest entry.
fn percent_schema_bounds() -> ConfigBoundsIndex {
    let mut entries = BTreeMap::new();
    entries.insert(
        "experimental_percent".to_string(),
        ConfigFieldEntry {
            field_type: "float_or_percent".to_string(),
            default: Some("50%".to_string()),
            parsed_default: Some(ConfigValue::FloatOrPercent {
                value: 50.0,
                is_percent: true,
            }),
            ..Default::default()
        },
    );
    let module = LoadedModuleBuilder::new(
        "percent-fixture",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Layer::Perimeter",
        "legacy",
        PathBuf::from("fixtures/percent-fixture.wasm"),
    )
    .config_schema(ConfigSchema { entries })
    .build();
    ConfigBoundsIndex::from_modules([&module])
}

/// AC-1: A known field (top_shell_layers) is applied; unlisted fields keep
/// their defaults; extensions must be empty.
#[test]
fn resolver_maps_top_shell_layers() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert("top_shell_layers".to_string(), ConfigValue::Int(4));

    let bounds = ConfigBoundsIndex::empty();
    let resolved =
        resolve(&source, &bounds, &ResolutionTarget::default()).expect("resolution should succeed");

    assert_eq!(resolved.top_shell_layers, 4, "top_shell_layers should be 4");
    assert_eq!(
        resolved.bottom_shell_layers, 3,
        "bottom_shell_layers should keep default (3)"
    );
    assert!(
        !resolved.extensions.contains_key("top_shell_layers"),
        "a declared field must not spill into extensions"
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
    let resolved =
        resolve(&source, &bounds, &ResolutionTarget::default()).expect("resolution should succeed");

    assert_eq!(resolved.top_shell_layers, 2);
    assert_eq!(
        resolved.extensions.get("experimental_xyz"),
        Some(&ConfigValue::String("on".to_string())),
        "unknown key should land in extensions"
    );
}

/// A percent default parsed from a `[config.schema.*]` manifest entry must
/// retain its percent variant while crossing config resolution: with no
/// profile value present, the parsed schema default lands in `extensions`
/// via the live `ConfigBoundsIndex::from_modules` → `resolve_global_config`
/// path (packet 185 / AC-6, TASK-303).
#[test]
fn percent_round_trip() {
    let source: HashMap<String, ConfigValue> = HashMap::new();
    let bounds = percent_schema_bounds();
    let resolved =
        resolve(&source, &bounds, &ResolutionTarget::default()).expect("resolution should succeed");

    assert_eq!(
        resolved.extensions.get("experimental_percent"),
        Some(&ConfigValue::FloatOrPercent {
            value: 50.0,
            is_percent: true,
        }),
        "parsed percent schema default must reach extensions uncoerced"
    );
}

/// A profile-supplied percent value still overrides the threaded schema
/// default on the same key.
#[test]
fn percent_profile_value_overrides_schema_default() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "experimental_percent".to_string(),
        ConfigValue::Percent(75.0),
    );

    let bounds = percent_schema_bounds();
    let resolved =
        resolve(&source, &bounds, &ResolutionTarget::default()).expect("resolution should succeed");

    assert_eq!(
        resolved.extensions.get("experimental_percent"),
        Some(&ConfigValue::FloatOrPercent {
            value: 75.0,
            is_percent: true,
        }),
        "profile value must win over the parsed schema default"
    );
}

/// Part C: `tool_config:<idx>:<key>` overrides resolve into a per-tool overlay
/// keyed by tool index, on top of the global base; tools without an override
/// are absent (callers fall back to the global value).
#[test]
fn resolver_per_tool_overrides_global() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert("retract_length".to_string(), ConfigValue::Float(2.0));
    // Tool 1 overrides retract_length; tool 0 has no override.
    source.insert(
        "tool_config:1:retract_length".to_string(),
        ConfigValue::Float(5.5),
    );

    let bounds = ConfigBoundsIndex::empty();
    let global =
        resolve(&source, &bounds, &ResolutionTarget::default()).expect("global resolution");
    assert_eq!(global.retract_length, 2.0);

    let tool_1 = resolve(
        &source,
        &bounds,
        &ResolutionTarget {
            tool_index: Some(1),
            ..ResolutionTarget::default()
        },
    )
    .expect("per-tool resolution");

    assert_eq!(tool_1.retract_length, 5.5);
    let scoped = ingest_resolution_config(&source, &bounds).expect("typed ingestion");
    assert!(
        scoped.delta(&ConfigScope::Tool(1)).is_some()
            && scoped.delta(&ConfigScope::Tool(0)).is_none(),
        "tool 0 has no tool_config override, so it must be absent (falls back to global)"
    );
}

/// Part C: a non-numeric tool index violates the canonical scoped-key shape.
#[test]
fn resolver_per_tool_rejects_non_numeric_index() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "tool_config:bogus:retract_length".to_string(),
        ConfigValue::Float(9.9),
    );
    let bounds = ConfigBoundsIndex::empty();
    // Design rule AC-N1 rejects malformed scope encodings instead of silently skipping them.
    let error = resolve(
        &source,
        &bounds,
        &ResolutionTarget {
            tool_index: Some(1),
            ..ResolutionTarget::default()
        },
    )
    .expect_err("a non-numeric tool index must be rejected");
    assert_eq!(
        error,
        ResolutionError::Application(ConfigResolutionError::TypeMismatch {
            key: "tool_config:bogus:retract_length".to_owned(),
            expected: "tool_config:<u32>:<key>",
            actual: "malformed scoped config key".to_owned(),
        })
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
    let global = resolve(&source, &bounds, &ResolutionTarget::default())
        .expect("global resolution should succeed");
    assert_eq!(global.top_shell_layers, 3);

    let per_object = ["obj-A", "obj-B"]
        .into_iter()
        .map(|object_id| {
            resolve(
                &source,
                &bounds,
                &ResolutionTarget {
                    object_id: object_id.to_owned(),
                    ..ResolutionTarget::default()
                },
            )
            .map(|config| (object_id.to_owned(), config))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
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
    let err = resolve(&source, &bounds, &ResolutionTarget::default())
        .expect_err("should fail on type mismatch");

    match err {
        ResolutionError::Application(ConfigResolutionError::TypeMismatch {
            key,
            expected,
            actual,
        }) => {
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

#[test]
fn legacy_first_layer_line_width_alias_resolves() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "first_layer_line_width".to_string(),
        ConfigValue::Float(0.4),
    );

    let bounds = ConfigBoundsIndex::empty();
    let resolved = resolve(&source, &bounds, &ResolutionTarget::default())
        .expect("legacy alias should resolve");

    assert_eq!(
        resolved.initial_layer_line_width,
        ResolvedFloatOrPercent {
            value: 0.4,
            is_percent: false,
        }
    );
    assert!(!resolved.extensions.contains_key("first_layer_line_width"));
}

/// F-2 regression: `support_threshold_angle` is a CLI-bound typed field, so it
/// must land on `ResolvedConfig::support_threshold_angle` and NOT in
/// `extensions`. Before packet 224 the support-analysis producer read this key
/// out of `extensions` only, where `resolve_*` never puts a CLI-bound key — so
/// every configured overhang angle was silently ignored.
#[test]
fn support_threshold_angle_lands_on_typed_field_not_extensions() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "support_threshold_angle".to_string(),
        ConfigValue::Float(55.0),
    );

    let bounds = ConfigBoundsIndex::empty();
    let resolved = resolve(&source, &bounds, &ResolutionTarget::default())
        .expect("canonical key should resolve");

    assert_eq!(resolved.support_threshold_angle, 55.0);
    assert!(!resolved.extensions.contains_key("support_threshold_angle"));
}

/// The pre-rename in-tree spelling stays accepted through `canonical_config_key`.
#[test]
fn legacy_support_overhang_angle_alias_resolves() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "support_overhang_angle".to_string(),
        ConfigValue::Float(55.0),
    );

    let bounds = ConfigBoundsIndex::empty();
    let resolved = resolve(&source, &bounds, &ResolutionTarget::default())
        .expect("legacy alias should resolve");

    assert_eq!(resolved.support_threshold_angle, 55.0);
    assert!(!resolved.extensions.contains_key("support_overhang_angle"));
}

/// Same conflict-guard precedent as `initial_layer_line_width`: a `HashMap`
/// source has no key ordering, so accepting both spellings would make the
/// resolved value depend on hash iteration order.
#[test]
fn both_support_threshold_angle_spellings_rejected() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "support_threshold_angle".to_string(),
        ConfigValue::Float(30.0),
    );
    source.insert(
        "support_overhang_angle".to_string(),
        ConfigValue::Float(55.0),
    );

    let bounds = ConfigBoundsIndex::empty();
    let err = resolve(&source, &bounds, &ResolutionTarget::default())
        .expect_err("both keys must be rejected");
    let message = err.to_string();

    assert!(
        message.contains("support_threshold_angle"),
        "error should name the canonical key: {message}"
    );
    assert!(
        message.contains("support_overhang_angle"),
        "error should name the legacy key: {message}"
    );
}

#[test]
fn both_keys_rejected() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "initial_layer_line_width".to_string(),
        ConfigValue::Float(0.4),
    );
    source.insert(
        "first_layer_line_width".to_string(),
        ConfigValue::Float(0.5),
    );

    let bounds = ConfigBoundsIndex::empty();
    let err = resolve(&source, &bounds, &ResolutionTarget::default())
        .expect_err("both keys must be rejected");
    let message = err.to_string();

    assert!(
        message.contains("initial_layer_line_width"),
        "error should name the canonical key: {message}"
    );
    assert!(
        message.contains("first_layer_line_width"),
        "error should name the legacy key: {message}"
    );
}
