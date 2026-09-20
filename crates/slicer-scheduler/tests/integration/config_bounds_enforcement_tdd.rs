//! Tests for numeric `[min, max]` bounds enforcement on CLI-sourced config
//! values. The bounds are declared in module manifest `[config.schema]` tables,
//! merged into a `ConfigBoundsIndex` at host startup, and checked at the same
//! point variant `TypeMismatch` is rejected (before `apply_cli_key` writes the
//! value into the macro-generated `ResolvedConfig` field).

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use slicer_config::{ExpansionContext, ResolutionError, ResolutionTarget};
use slicer_ir::{ConfigValue, ResolvedConfig, SemVer};
use slicer_scheduler::config_resolution::resolve_config;
use slicer_scheduler::{
    load_module_from_paths, BoundsDeclaration, ConfigBoundsIndex, ConfigFieldEntry,
    ConfigResolutionError, ConfigSchema, LoadedModule, LoadedModuleBuilder,
};

fn resolve_global_config(
    source: &HashMap<String, ConfigValue>,
    bounds: &ConfigBoundsIndex,
) -> Result<ResolvedConfig, ConfigResolutionError> {
    let mut source = source.clone();
    source
        .entry("nozzle_diameter".to_string())
        .or_insert(ConfigValue::Float(0.4));

    match resolve_config(
        &source,
        bounds,
        &ResolutionTarget::default(),
        &ExpansionContext::default(),
    ) {
        Ok(config) => Ok(config),
        Err(ResolutionError::Application(error)) => Err(error),
        Err(error) => panic!("bounds fixture failed unified scope resolution: {error}"),
    }
}

/// Field type the bounds fixture declares for `key`, mirroring the
/// production registry (host `cli` rows for typed `ResolvedConfig` fields,
/// module manifest `[config.schema]` entries for the rest).
fn fixture_field_type(key: &str) -> &'static str {
    match key {
        "wall_count" => "int",
        "wall_widths" => "float-list",
        "min_width_top_surface" => "float_or_percent",
        _ => "float",
    }
}

/// `[config.schema]` entries for a bounds fixture module: `key` with the
/// declared `[min, max]`, plus `nozzle_diameter` (a plain float). The nozzle
/// key is production-registered by the perimeter modules but absent from an
/// empty-module registry, so a fixture that injects `nozzle_diameter` into
/// the source (see `resolve_global_config`) must register it here too —
/// under warn-then-drop ingestion an unregistered key never reaches Phase-B
/// expansion of `line_width`.
fn fixture_entries(
    key: &str,
    min: Option<f64>,
    max: Option<f64>,
) -> BTreeMap<String, ConfigFieldEntry> {
    let mut entries = BTreeMap::new();
    entries.insert(
        key.to_string(),
        ConfigFieldEntry {
            field_type: fixture_field_type(key).to_string(),
            min,
            max,
            ..Default::default()
        },
    );
    entries.insert(
        "nozzle_diameter".to_string(),
        ConfigFieldEntry {
            field_type: "float".to_string(),
            ..Default::default()
        },
    );
    entries
}

fn fixture_module(module_id: &str, entries: BTreeMap<String, ConfigFieldEntry>) -> LoadedModule {
    LoadedModuleBuilder::new(
        module_id.to_string(),
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Layer::Perimeter",
        "legacy",
        PathBuf::from("fixtures/bounds-fixture.wasm"),
    )
    .config_schema(ConfigSchema { entries })
    .build()
}

/// A fixture module that registers only `nozzle_diameter` (no bounds), used
/// by tests that need an empty bounds index whose registry still carries the
/// injected nozzle diameter through warn-then-drop ingestion.
fn nozzle_only_fixture(module_id: &str) -> LoadedModule {
    let mut entries = BTreeMap::new();
    entries.insert(
        "nozzle_diameter".to_string(),
        ConfigFieldEntry {
            field_type: "float".to_string(),
            ..Default::default()
        },
    );
    fixture_module(module_id, entries)
}

/// One-module bounds index whose schema declares `key` with the given
/// `[min, max]` and registers `nozzle_diameter`, so warn-then-drop ingestion
/// retains the fixture keys and the bounds assertions below actually run.
fn single_module_bounds(key: &str, min: Option<f64>, max: Option<f64>) -> ConfigBoundsIndex {
    ConfigBoundsIndex::from_modules([&fixture_module(
        "test.module",
        fixture_entries(key, min, max),
    )])
}

fn assert_out_of_range(
    err: ConfigResolutionError,
    expected_key: &str,
    expected_value: f64,
    expected_index: Option<usize>,
) {
    match err {
        ConfigResolutionError::OutOfRange {
            key, value, index, ..
        } => {
            assert_eq!(key, expected_key, "key in OutOfRange mismatched");
            if expected_value.is_nan() {
                assert!(value.is_nan(), "expected NaN value, got {value}");
            } else {
                assert_eq!(value, expected_value, "value in OutOfRange mismatched");
            }
            assert_eq!(index, expected_index, "index in OutOfRange mismatched");
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
}

#[test]
fn rejects_value_below_min() {
    let bounds = single_module_bounds("layer_height", Some(0.05), Some(1.0));
    let mut source = HashMap::new();
    source.insert("layer_height".to_string(), ConfigValue::Float(-1.0));

    let err = resolve_global_config(&source, &bounds).expect_err("below-min must reject");
    assert_out_of_range(err, "layer_height", -1.0, None);
}

#[test]
fn rejects_value_above_max() {
    let bounds = single_module_bounds("layer_height", Some(0.05), Some(1.0));
    let mut source = HashMap::new();
    source.insert("layer_height".to_string(), ConfigValue::Float(99.0));

    let err = resolve_global_config(&source, &bounds).expect_err("above-max must reject");
    assert_out_of_range(err, "layer_height", 99.0, None);
}

#[test]
fn out_of_range_support_threshold_angle_is_rejected() {
    let bounds = single_module_bounds("support_threshold_angle", Some(0.0), Some(90.0));
    let mut source = HashMap::new();
    source.insert(
        "support_threshold_angle".to_string(),
        ConfigValue::Float(200.0),
    );

    let err = resolve_global_config(&source, &bounds)
        .expect_err("support threshold angle above max must reject");
    assert_out_of_range(err, "support_threshold_angle", 200.0, None);
}

#[test]
fn rejects_unknown_support_style_value() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/tree-support-planner/tree-support-planner.toml");
    let wasm = manifest.with_extension("wasm");
    let module = load_module_from_paths(&manifest, &wasm).expect("real support manifest must load");
    let bounds = ConfigBoundsIndex::from_modules([&module]);
    let mut source = HashMap::new();
    source.insert(
        "support_style".to_string(),
        ConfigValue::String("tree_unknown".to_string()),
    );

    let err = resolve_global_config(&source, &bounds)
        .expect_err("an undeclared support style must be rejected");
    match err {
        ConfigResolutionError::TypeMismatch { key, expected, .. } => {
            assert_eq!(key, "support_style");
            assert_eq!(expected, "one of the manifest-declared enum values");
        }
        other => panic!("expected enum TypeMismatch, got {other:?}"),
    }
}

#[test]
fn manifest_declared_bound_rejects_out_of_range_value() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../modules/core-modules/traditional-support-planner/traditional-support-planner.toml",
    );
    let wasm = manifest.with_extension("wasm");
    let module = load_module_from_paths(&manifest, &wasm).expect("real support manifest must load");
    let bounds = ConfigBoundsIndex::from_modules([&module]);

    let mut source = HashMap::new();
    source.insert(
        "support_threshold_angle".to_string(),
        ConfigValue::Float(200.0),
    );

    let err = resolve_global_config(&source, &bounds)
        .expect_err("manifest-declared support threshold bound must reject");
    assert_out_of_range(err, "support_threshold_angle", 200.0, None);
}

#[test]
fn rejects_max_bridge_length_below_min() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/tree-support-planner/tree-support-planner.toml");
    let wasm = manifest.with_extension("wasm");
    let module = load_module_from_paths(&manifest, &wasm).expect("real support manifest must load");
    let bounds = ConfigBoundsIndex::from_modules([&module]);

    let mut source = HashMap::new();
    source.insert("max_bridge_length".to_string(), ConfigValue::Float(-1.0));

    let err = resolve_global_config(&source, &bounds)
        .expect_err("max bridge length below minimum must reject");
    assert_out_of_range(err, "max_bridge_length", -1.0, None);
}

#[test]
fn rejects_support_max_branches_per_layer_zero() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/tree-support-planner/tree-support-planner.toml");
    let wasm = manifest.with_extension("wasm");
    let module = load_module_from_paths(&manifest, &wasm).expect("real support manifest must load");
    let bounds = ConfigBoundsIndex::from_modules([&module]);

    let mut source = HashMap::new();
    source.insert(
        "support_max_branches_per_layer".to_string(),
        ConfigValue::Int(0),
    );

    let err = resolve_global_config(&source, &bounds)
        .expect_err("zero support branches per layer must reject");
    assert_out_of_range(err, "support_max_branches_per_layer", 0.0, None);
}

#[test]
fn rejects_negative_support_branch_merge_distance() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/tree-support-planner/tree-support-planner.toml");
    let wasm = manifest.with_extension("wasm");
    let module = load_module_from_paths(&manifest, &wasm).expect("real support manifest must load");
    let bounds = ConfigBoundsIndex::from_modules([&module]);

    let mut source = HashMap::new();
    source.insert(
        "support_branch_merge_distance_mm".to_string(),
        ConfigValue::Float(-0.5),
    );

    let err = resolve_global_config(&source, &bounds)
        .expect_err("negative support branch merge distance must reject");
    assert_out_of_range(err, "support_branch_merge_distance_mm", -0.5, None);
}

#[test]
fn accepts_boundary_min() {
    // `wall_count` is an Int-typed declared field so the value lands on the
    // matching `ResolvedConfig` slot via `apply_cli_key`.
    let bounds = single_module_bounds("wall_count", Some(2.0), Some(8.0));
    let mut source = HashMap::new();
    source.insert("wall_count".to_string(), ConfigValue::Int(2));

    let resolved = resolve_global_config(&source, &bounds).expect("min boundary must accept");
    assert_eq!(resolved.wall_count, 2);
}

#[test]
fn accepts_boundary_max() {
    let bounds = single_module_bounds("wall_count", Some(2.0), Some(8.0));
    let mut source = HashMap::new();
    source.insert("wall_count".to_string(), ConfigValue::Int(8));

    let resolved = resolve_global_config(&source, &bounds).expect("max boundary must accept");
    assert_eq!(resolved.wall_count, 8);
}

#[test]
fn accepts_when_only_min_declared() {
    let bounds = single_module_bounds("wall_count", Some(1.0), None);
    let mut source = HashMap::new();
    source.insert("wall_count".to_string(), ConfigValue::Int(1_000));

    resolve_global_config(&source, &bounds).expect("unbounded above: large value must accept");
}

#[test]
fn rejects_below_min_when_only_min_declared() {
    let bounds = single_module_bounds("wall_count", Some(1.0), None);
    let mut source = HashMap::new();
    source.insert("wall_count".to_string(), ConfigValue::Int(0));

    let err = resolve_global_config(&source, &bounds).expect_err("below min must reject");
    assert_out_of_range(err, "wall_count", 0.0, None);
}

#[test]
fn rejects_nan_for_numeric_field() {
    let bounds = single_module_bounds("layer_height", Some(0.05), Some(1.0));
    let mut source = HashMap::new();
    source.insert("layer_height".to_string(), ConfigValue::Float(f64::NAN));

    // Packet 03 moved non-finite declared-value rejection to the ingestion boundary.
    let err = resolve_global_config(&source, &bounds).expect_err("NaN must reject");
    match err {
        ConfigResolutionError::TypeMismatch {
            key,
            expected,
            actual,
        } => {
            assert_eq!(key, "layer_height");
            assert_eq!(expected, "Float");
            assert_eq!(actual, "Float(NaN)");
        }
        other => panic!("expected ingestion TypeMismatch, got {other:?}"),
    }
}

#[test]
fn rejects_infinity_for_numeric_field() {
    let bounds = single_module_bounds("layer_height", None, Some(1.0));
    let mut source = HashMap::new();
    source.insert(
        "layer_height".to_string(),
        ConfigValue::Float(f64::INFINITY),
    );

    let err = resolve_global_config(&source, &bounds).expect_err("infinity must reject");
    match err {
        ConfigResolutionError::TypeMismatch {
            key,
            expected,
            actual,
        } => {
            assert_eq!(key, "layer_height");
            assert_eq!(expected, "Float");
            assert_eq!(actual, "Float(inf)");
        }
        other => panic!("expected ingestion TypeMismatch, got {other:?}"),
    }
}

#[test]
fn unknown_key_skips_bounds_check() {
    // Index declares bounds for `layer_height`, but the source carries a
    // different (undeclared) key. Under warn-then-drop the undeclared key is
    // dropped before bounds checking, so no bounds error may surface and the
    // key must not reach extensions.
    let bounds = single_module_bounds("layer_height", Some(0.05), Some(1.0));
    let mut source = HashMap::new();
    source.insert(
        "unrelated_extension".to_string(),
        ConfigValue::Float(-9999.0),
    );

    let resolved = resolve_config(
        &source,
        &bounds,
        &ResolutionTarget::default(),
        // The dropped key leaves no `nozzle_diameter` for Phase-B expansion of
        // `line_width`, so the expansion context supplies it (the canonical
        // resolved-config-view fixture does the same).
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
    )
    .expect("an undeclared key must not surface a bounds error");
    assert!(
        !resolved.extensions.contains_key("unrelated_extension"),
        "an undeclared key must be dropped, not kept in extensions"
    );
}

#[test]
fn no_bounds_declared_accepts_any_value() {
    // The bounds index is empty (no module declared min/max); no rejection
    // should occur on any numeric value. The fixture module only carries the
    // `nozzle_diameter` registration (no bounds) so the injected nozzle
    // diameter survives ingestion for Phase-B expansion.
    let bounds = ConfigBoundsIndex::from_modules([&nozzle_only_fixture("test.module")]);
    let mut source = HashMap::new();
    source.insert("layer_height".to_string(), ConfigValue::Float(-1.0));

    resolve_global_config(&source, &bounds).expect("empty bounds: any value must accept");
}

#[test]
fn int_value_coerced_to_f64_for_check() {
    // Int values are coerced to f64 for the bounds comparison.
    let bounds = single_module_bounds("wall_count", Some(0.0), Some(100.0));
    let mut source = HashMap::new();
    source.insert("wall_count".to_string(), ConfigValue::Int(2_000_000_000));

    let err = resolve_global_config(&source, &bounds).expect_err("Int above max must reject");
    assert_out_of_range(err, "wall_count", 2_000_000_000.0, None);
}

#[test]
fn list_element_out_of_range_reports_index() {
    // `float-list` fields validate every element against the same [min, max].
    // Wall_widths is not a declared CLI-bound field, but the bounds check
    // applies before apply_cli_key, so an unknown numeric-list key still
    // triggers the bounds check when present in the index.
    let bounds = single_module_bounds("wall_widths", Some(0.0), Some(1.0));
    let mut source = HashMap::new();
    source.insert(
        "wall_widths".to_string(),
        ConfigValue::List(vec![
            ConfigValue::Float(0.5),
            ConfigValue::Float(99.0),
            ConfigValue::Float(0.75),
        ]),
    );

    let err =
        resolve_global_config(&source, &bounds).expect_err("list element out of range must reject");
    assert_out_of_range(err, "wall_widths", 99.0, Some(1));
}

#[test]
fn intersection_strictest_min_max_wins() {
    // Module A declares [0, 10]; module B declares [5, 100]; effective
    // range is [5, 10] â€” strictest of each side.
    let bounds = ConfigBoundsIndex::from_modules([
        &fixture_module(
            "mod.a",
            fixture_entries("wall_count", Some(0.0), Some(10.0)),
        ),
        &fixture_module(
            "mod.b",
            fixture_entries("wall_count", Some(5.0), Some(100.0)),
        ),
    ]);

    let mut below = HashMap::new();
    below.insert("wall_count".to_string(), ConfigValue::Int(3));
    let err =
        resolve_global_config(&below, &bounds).expect_err("3 below strictest min (5) must reject");
    assert_out_of_range(err, "wall_count", 3.0, None);

    let mut inside = HashMap::new();
    inside.insert("wall_count".to_string(), ConfigValue::Int(7));
    let resolved = resolve_global_config(&inside, &bounds).expect("7 inside [5,10] must accept");
    assert_eq!(resolved.wall_count, 7);

    let mut above = HashMap::new();
    above.insert("wall_count".to_string(), ConfigValue::Int(50));
    let err = resolve_global_config(&above, &bounds)
        .expect_err("50 above strictest max (10) must reject");
    assert_out_of_range(err, "wall_count", 50.0, None);
}

#[test]
fn intersection_empty_range_rejects_all_values() {
    // Module A declares [0, 5]; module B declares [10, 100]; the
    // intersection is empty (5 < 10). Every value must be rejected.
    let bounds = ConfigBoundsIndex::from_declarations([
        BoundsDeclaration {
            key: "wall_count".to_string(),
            min: Some(0.0),
            max: Some(5.0),
            module_id: "mod.a".to_string(),
        },
        BoundsDeclaration {
            key: "wall_count".to_string(),
            min: Some(10.0),
            max: Some(100.0),
            module_id: "mod.b".to_string(),
        },
    ]);

    for v in [-5, 0, 3, 7, 50, 200] {
        let mut source = HashMap::new();
        source.insert("wall_count".to_string(), ConfigValue::Int(v));
        let err = resolve_global_config(&source, &bounds)
            .expect_err("every value must reject when intersection is empty");
        match err {
            ConfigResolutionError::OutOfRange { .. } => {}
            other => panic!("expected OutOfRange for value {v}, got {other:?}"),
        }
    }
}

#[test]
fn bool_value_skips_numeric_bounds_check() {
    // A Bool value supplied for a numerically-bounded key passes the bounds
    // check (which only applies to numeric variants); the downstream
    // `apply_cli_key` path raises `TypeMismatch` if the declared field is
    // numeric, otherwise routes to extensions.
    let bounds = single_module_bounds("layer_height", Some(0.05), Some(1.0));
    let mut source = HashMap::new();
    source.insert("layer_height".to_string(), ConfigValue::Bool(true));

    let err = resolve_global_config(&source, &bounds)
        .expect_err("Bool for Float field must surface TypeMismatch");
    match err {
        ConfigResolutionError::TypeMismatch { key, .. } => {
            assert_eq!(key, "layer_height");
        }
        other => panic!("expected TypeMismatch, got {other:?}"),
    }
}

#[test]
fn percent_bounds_rejects_value_below_min() {
    // A `percent`-typed field (packet 150) IS numeric per
    // `is_numeric_field_type`, so a declared min/max is enforced against
    // the raw percent number itself (the resolution base is unknown here).
    let bounds = single_module_bounds("min_width_top_surface", Some(0.0), None);
    let mut source = HashMap::new();
    source.insert(
        "min_width_top_surface".to_string(),
        ConfigValue::Percent(-5.0),
    );

    let err = resolve_global_config(&source, &bounds)
        .expect_err("percent value below declared min must reject");
    assert_out_of_range(err, "min_width_top_surface", -5.0, None);
}

#[test]
fn percent_bounds_accepts_value_within_range() {
    let bounds = single_module_bounds("min_width_top_surface", Some(0.0), None);
    let mut source = HashMap::new();
    source.insert(
        "min_width_top_surface".to_string(),
        ConfigValue::Percent(300.0),
    );

    resolve_global_config(&source, &bounds)
        .expect("percent value within declared bounds must accept");
}

#[test]
fn percent_bounds_rejects_float_or_percent_value_below_min() {
    // `float_or_percent` is likewise numeric; bounds are checked against
    // the raw literal `value` field regardless of `is_percent`.
    let bounds = single_module_bounds("min_width_top_surface", Some(0.0), None);
    let mut source = HashMap::new();
    source.insert(
        "min_width_top_surface".to_string(),
        ConfigValue::FloatOrPercent {
            value: -1.0,
            is_percent: true,
        },
    );

    let err = resolve_global_config(&source, &bounds)
        .expect_err("float_or_percent value below declared min must reject");
    assert_out_of_range(err, "min_width_top_surface", -1.0, None);
}

#[test]
fn percent_bounds_accepts_float_or_percent_value_within_range() {
    let bounds = single_module_bounds("min_width_top_surface", Some(0.0), None);
    let mut source = HashMap::new();
    source.insert(
        "min_width_top_surface".to_string(),
        ConfigValue::FloatOrPercent {
            value: 300.0,
            is_percent: true,
        },
    );

    resolve_global_config(&source, &bounds)
        .expect("float_or_percent value within declared bounds must accept");
}
