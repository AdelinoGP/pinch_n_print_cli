//! Independent literal-oracle coverage for Phase B automatic-value expansion.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use slicer_config::{
    assemble_registry, expand_automatic_values, resolve_scope_stack, ConfigSchemaRegistry,
    ConfigScope, ExpansionContext, ExpansionError, HostChannels, ModuleDeclaration, RegistryEntry,
    ResolutionError, ResolutionTarget, ScopeDelta, ScopedConfig,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::{ConfigValue, ResolvedConfig};

fn registry_with(entries: &[(&str, &str, Option<&str>)]) -> ConfigSchemaRegistry {
    let mut schema = ConfigSchema::default();
    for &(key, field_type, base_key) in entries {
        schema.entries.insert(
            key.to_owned(),
            ConfigFieldEntry {
                field_type: field_type.to_owned(),
                base_key: base_key.map(str::to_owned),
                ..ConfigFieldEntry::default()
            },
        );
    }

    assemble_registry(
        &[ModuleDeclaration {
            module_id: "dev.pinch.test.automatic-values".to_owned(),
            schema,
            ..ModuleDeclaration::default()
        }],
        &HostChannels::from_parts(Vec::new(), Vec::new(), Vec::new()),
    )
    .expect("automatic-value fixture registry must be valid")
    .registry
}

fn set_value(config: &mut ResolvedConfig, key: &str, value: ConfigValue) {
    if !config
        .apply_cli_key(key, &value)
        .expect("fixture value must match a typed config field")
    {
        config.extensions.insert(key.to_owned(), value);
    }
}

fn config_without_automatic_widths() -> ResolvedConfig {
    let mut config = ResolvedConfig {
        line_width: 1.0,
        ..ResolvedConfig::default()
    };
    set_value(
        &mut config,
        "support_line_width",
        ConfigValue::FloatOrPercent {
            value: 1.0,
            is_percent: false,
        },
    );
    config
}

fn assert_float(map: &std::collections::HashMap<String, ConfigValue>, key: &str, expected: f64) {
    match map.get(key) {
        Some(ConfigValue::Float(actual)) => assert_eq!(*actual, expected, "wrong {key}"),
        other => panic!("expected absolute float for {key}, got {other:?}"),
    }
}

fn assert_float_or_percent_absolute(
    map: &std::collections::HashMap<String, ConfigValue>,
    key: &str,
    expected: f64,
) {
    match map.get(key) {
        Some(ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        }) => assert_eq!(*value, expected, "wrong {key}"),
        other => panic!("expected absolute float-or-percent for {key}, got {other:?}"),
    }
}

#[test]
fn phase_b_expands_hand_computed_percent_zero_and_minus_one_cases() {
    let registry = registry_with(&[
        ("nozzle_diameter", "float", None),
        (
            "support_line_width",
            "float_or_percent",
            Some("nozzle_diameter"),
        ),
        (
            "outer_wall_line_width",
            "float_or_percent",
            Some("nozzle_diameter"),
        ),
        ("outer_wall_speed", "float", None),
        (
            "overhang_1_4_speed",
            "float_or_percent",
            Some("outer_wall_speed"),
        ),
        (
            "overhang_2_4_speed",
            "float_or_percent",
            Some("outer_wall_speed"),
        ),
        (
            "overhang_3_4_speed",
            "float_or_percent",
            Some("outer_wall_speed"),
        ),
        (
            "overhang_4_4_speed",
            "float_or_percent",
            Some("outer_wall_speed"),
        ),
        (
            "top_surface_speed",
            "float_or_percent",
            Some("outer_wall_speed"),
        ),
        ("support_interface_top_layers", "int", None),
        ("support_interface_bottom_layers", "int", None),
        ("support_interface_spacing", "float", None),
        ("support_bottom_interface_spacing", "float", None),
        (
            "overhang_zero_speed",
            "float_or_percent",
            Some("outer_wall_speed"),
        ),
        ("chained_base", "float_or_percent", Some("outer_wall_speed")),
        ("chained_relative", "float_or_percent", Some("chained_base")),
    ]);
    let mut config = ResolvedConfig {
        outer_wall_speed: 60.0,
        ..ResolvedConfig::default()
    };
    for (key, percent) in [
        ("overhang_1_4_speed", 25.0),
        ("overhang_2_4_speed", 50.0),
        ("overhang_3_4_speed", 75.0),
        ("overhang_4_4_speed", 100.0),
        ("top_surface_speed", 50.0),
    ] {
        config.extensions.insert(
            key.to_owned(),
            ConfigValue::FloatOrPercent {
                value: percent,
                is_percent: true,
            },
        );
    }
    config.extensions.extend([
        (
            "outer_wall_line_width".to_owned(),
            ConfigValue::Percent(150.0),
        ),
        (
            "support_interface_top_layers".to_owned(),
            ConfigValue::Int(3),
        ),
        (
            "support_interface_bottom_layers".to_owned(),
            ConfigValue::Int(-1),
        ),
        (
            "support_interface_spacing".to_owned(),
            ConfigValue::Float(0.35),
        ),
        (
            "support_bottom_interface_spacing".to_owned(),
            ConfigValue::Float(-1.0),
        ),
        ("overhang_zero_speed".to_owned(), ConfigValue::Float(0.0)),
        ("chained_base".to_owned(), ConfigValue::Percent(50.0)),
        ("chained_relative".to_owned(), ConfigValue::Percent(50.0)),
    ]);

    expand_automatic_values(
        &registry,
        &mut config,
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
        None,
    )
    .expect("the literal fixture has every required base");

    let actual = config.to_config_map();
    // `line_width` is a typed `f64` field, so the auto expansion lands on the
    // f64 product of the canonical formula (1.125 x nozzle_diameter) rather
    // than an `f32`-round-tripped literal.
    assert_float(&actual, "line_width", 1.125_f64 * 0.4);
    assert_float(&actual, "support_line_width", 0.4);
    // 150% of the 0.4 nozzle is hand-computed 0.6; the 1e-12 window absorbs
    // only binary representation, not a wrong base or ratio.
    match actual.get("outer_wall_line_width") {
        Some(ConfigValue::Float(value)) => assert!(
            (value - 0.6).abs() <= 1e-12,
            "outer_wall_line_width must expand 150% of 0.4 to 0.6, got {value}"
        ),
        other => panic!("expected absolute float for outer_wall_line_width, got {other:?}"),
    }
    assert_float_or_percent_absolute(&actual, "overhang_1_4_speed", 15.0);
    assert_float_or_percent_absolute(&actual, "overhang_2_4_speed", 30.0);
    assert_float_or_percent_absolute(&actual, "overhang_3_4_speed", 45.0);
    assert_float_or_percent_absolute(&actual, "overhang_4_4_speed", 60.0);
    assert_float_or_percent_absolute(&actual, "top_surface_speed", 30.0);
    assert_eq!(
        actual.get("support_interface_bottom_layers"),
        Some(&ConfigValue::Int(3))
    );
    assert_float(&actual, "support_bottom_interface_spacing", 0.35);
    assert_float(&actual, "overhang_zero_speed", 0.0);
    assert_float(&actual, "chained_base", 30.0);
    assert_float(&actual, "chained_relative", 15.0);

    // Second row: explicit bottom zeros and a numeric overhang zero are
    // explicit values, not auto sentinels, and must pass through unchanged.
    let mut explicit = config_without_automatic_widths();
    explicit.extensions.extend([
        (
            "support_interface_top_layers".to_owned(),
            ConfigValue::Int(3),
        ),
        (
            "support_interface_bottom_layers".to_owned(),
            ConfigValue::Int(0),
        ),
        (
            "support_interface_spacing".to_owned(),
            ConfigValue::Float(0.35),
        ),
        (
            "support_bottom_interface_spacing".to_owned(),
            ConfigValue::Float(0.0),
        ),
        ("overhang_zero_speed".to_owned(), ConfigValue::Float(0.0)),
    ]);
    expand_automatic_values(
        &registry,
        &mut explicit,
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
        None,
    )
    .expect("explicit zeros need no auto base");
    let second = explicit.to_config_map();
    assert_eq!(
        second.get("support_interface_bottom_layers"),
        Some(&ConfigValue::Int(0))
    );
    assert_float(&second, "support_bottom_interface_spacing", 0.0);
    assert_float(&second, "overhang_zero_speed", 0.0);
}

/// Regression: canonical `nozzle_diameter` is `coFloats` (per-tool vector,
/// `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp`) and real project
/// files author it as `["0.4"]`. Typed ingestion retains that shape as
/// `List([Float(0.4)])` (see
/// `tolerant_scalar_list_shape_is_retained_and_warns`) and typed reads
/// resolve it through the `envelope` first-element leniency; base resolution
/// must do the same. Regression shape: `bridge_line_width = "100%"` over
/// `nozzle_diameter = ["0.4"]` (cube_cilindrical_modifier.3mf's project
/// config) failed with "unknown base key nozzle_diameter required by
/// bridge_line_width".
#[test]
fn canonical_vector_base_resolves_at_first_element() {
    let registry = registry_with(&[
        ("nozzle_diameter", "float", None),
        (
            "bridge_line_width",
            "float_or_percent",
            Some("nozzle_diameter"),
        ),
    ]);
    let mut config = ResolvedConfig::default();
    config.extensions.insert(
        "nozzle_diameter".to_owned(),
        ConfigValue::List(vec![ConfigValue::Float(0.4)]),
    );
    config
        .extensions
        .insert("bridge_line_width".to_owned(), ConfigValue::Percent(100.0));

    expand_automatic_values(
        &registry,
        &mut config,
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
        None,
    )
    .expect("a vector-shaped base resolves at its first element");

    let actual = config.to_config_map();
    assert_float(&actual, "bridge_line_width", 0.4);
}

#[test]
fn tool_specific_base_wins_for_selected_tool() {
    let registry = registry_with(&[
        ("nozzle_diameter", "float", None),
        (
            "outer_wall_line_width",
            "float_or_percent",
            Some("nozzle_diameter"),
        ),
        (
            "support_line_width",
            "float_or_percent",
            Some("nozzle_diameter"),
        ),
    ]);
    let mut unselected = config_without_automatic_widths();
    unselected.line_width = 0.0;
    set_value(
        &mut unselected,
        "support_line_width",
        ConfigValue::FloatOrPercent {
            value: 0.0,
            is_percent: false,
        },
    );
    unselected
        .extensions
        .insert("nozzle_diameter".to_owned(), ConfigValue::Float(0.4));
    unselected.extensions.insert(
        "outer_wall_line_width".to_owned(),
        ConfigValue::Percent(150.0),
    );
    let mut selected = unselected.clone();
    let context = ExpansionContext {
        nozzle_diameter_mm: 0.4,
        tool_bases: BTreeMap::from([(1, BTreeMap::from([("nozzle_diameter".to_owned(), 0.6)]))]),
    };

    expand_automatic_values(&registry, &mut unselected, &context, None)
        .expect("merged base is valid");
    expand_automatic_values(&registry, &mut selected, &context, Some(1))
        .expect("selected tool base is valid");

    for (map, expected, label) in [
        (unselected.to_config_map(), 0.6, "global"),
        (selected.to_config_map(), 0.9, "tool 1"),
    ] {
        match map.get("outer_wall_line_width") {
            Some(ConfigValue::Float(value)) => assert!(
                (value - expected).abs() <= 1e-12,
                "{label}: outer_wall_line_width must expand 150% to {expected}, got {value}"
            ),
            other => {
                panic!("{label}: expected absolute float for outer_wall_line_width, got {other:?}")
            }
        }
    }
    // `line_width` is a typed `f64` field; both the unselected (1.125 x 0.4)
    // and selected (1.125 x 0.6) expansions land on their exact f64 products.
    assert_float(&unselected.to_config_map(), "line_width", 1.125_f64 * 0.4);
    assert_float(&selected.to_config_map(), "line_width", 1.125_f64 * 0.6);
    assert_float(&unselected.to_config_map(), "support_line_width", 0.4);
    assert_float(&selected.to_config_map(), "support_line_width", 0.6);
}

#[test]
fn unknown_base_key_is_rejected_without_partial_mutation() {
    let registry = registry_with(&[
        ("absent_base", "float", None),
        ("unknown_relative", "float_or_percent", Some("absent_base")),
    ]);
    let mut config = ResolvedConfig::default();
    config
        .extensions
        .insert("unknown_relative".to_owned(), ConfigValue::Percent(50.0));
    let original = config.clone();

    let error = expand_automatic_values(
        &registry,
        &mut config,
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
        None,
    )
    .expect_err("an absent merged base must reject the whole expansion");

    assert_eq!(
        error,
        ExpansionError::UnknownBaseKey {
            key: "unknown_relative".to_owned(),
            base_key: "absent_base".to_owned(),
        }
    );
    assert_eq!(
        config, original,
        "failure must not commit the staged line width"
    );
}

#[test]
fn missing_auto_base_is_rejected() {
    let registry = registry_with(&[
        ("support_interface_bottom_layers", "int", None),
        ("support_bottom_interface_spacing", "float", None),
    ]);
    for (key, base_key, value) in [
        (
            "support_interface_bottom_layers",
            "support_interface_top_layers",
            ConfigValue::Int(-1),
        ),
        (
            "support_bottom_interface_spacing",
            "support_interface_spacing",
            ConfigValue::Float(-1.0),
        ),
    ] {
        let mut config = config_without_automatic_widths();
        config.extensions.insert(key.to_owned(), value);
        let original = config.clone();

        let error =
            expand_automatic_values(&registry, &mut config, &ExpansionContext::default(), None)
                .expect_err("a bottom sentinel requires its corresponding top value");

        assert_eq!(
            error,
            ExpansionError::MissingAutoBase {
                key: key.to_owned(),
                base_key: base_key.to_owned(),
            }
        );
        assert_eq!(
            config, original,
            "missing auto base must be atomic for {key}"
        );
    }
}

#[test]
fn zero_nozzle_rejects_required_width_expansion_without_nan() {
    let registry = registry_with(&[]);
    for invalid_nozzle in [0.0, f64::NAN, f64::INFINITY] {
        let mut config = ResolvedConfig::default();
        let original = config.clone();

        let error = expand_automatic_values(
            &registry,
            &mut config,
            &ExpansionContext {
                nozzle_diameter_mm: invalid_nozzle,
                ..ExpansionContext::default()
            },
            None,
        )
        .expect_err("a required nozzle base must be positive and finite");

        match error {
            ExpansionError::NonPositiveBase {
                key,
                base_key,
                value,
            } => {
                assert_eq!(key, "line_width");
                assert_eq!(base_key, "nozzle_diameter");
                assert_eq!(value.to_bits(), invalid_nozzle.to_bits());
            }
            other => panic!("expected NonPositiveBase, got {other:?}"),
        }
        assert_eq!(config, original, "invalid nozzle must not mutate config");
        assert!(!config.line_width.is_nan(), "failure must not write NaN");
    }
}

// ── Live-registry negative-sentinel census (packet 10, Step 2 slice C) ─────
//
// Derivation rule: scan the live host declaration channels plus every real
// core-module manifest declaration for a numeric negative default or lower
// bound. A default is inspected as the same comma-joined wire string the
// declaration carries, so a negative element inside an array default is a
// candidate exactly like a negative scalar; non-numeric elements are not.
// Assemble the registry from those exact declarations, and fail loudly
// when a derived key has no explicit owner rule. The owner rule is narrow on
// purpose: packet 04 exclusively owns the two config-only `-1` mirrors, so any
// other negative-capable declaration is an unowned Phase C candidate until a
// rule names it. No complete key roster is maintained here.

/// One declaration whose numeric default (a scalar, or one numeric element of
/// an array default) or numeric lower bound is negative.
#[derive(Debug)]
struct NegativeDeclaration {
    key: String,
    provenance: String,
    negative_default: Option<f64>,
    negative_min: Option<f64>,
}

/// The explicit narrow owner rules for negative-capable declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NegativeSentinelOwner {
    /// `support_interface_bottom_layers = -1` mirrors `support_interface_top_layers`.
    BottomLayers,
    /// `support_bottom_interface_spacing = -1` mirrors `support_interface_spacing`.
    BottomSpacing,
}

impl NegativeSentinelOwner {
    fn key(self) -> &'static str {
        match self {
            Self::BottomLayers => "support_interface_bottom_layers",
            Self::BottomSpacing => "support_bottom_interface_spacing",
        }
    }

    fn sentinel(self) -> ConfigValue {
        match self {
            Self::BottomLayers => ConfigValue::Int(-1),
            Self::BottomSpacing => ConfigValue::Float(-1.0),
        }
    }
}

/// The owner rule. `None` means the key is an unowned Phase C candidate.
fn classify_phase_c_negative_candidate(key: &str) -> Option<NegativeSentinelOwner> {
    match key {
        "support_interface_bottom_layers" => Some(NegativeSentinelOwner::BottomLayers),
        "support_bottom_interface_spacing" => Some(NegativeSentinelOwner::BottomSpacing),
        _ => None,
    }
}

/// Every derived candidate whose key the narrow owner rule does not classify,
/// i.e. an unowned Phase-C candidate.
fn unowned_phase_c_candidates(candidates: &[NegativeDeclaration]) -> Vec<&NegativeDeclaration> {
    candidates
        .iter()
        .filter(|candidate| classify_phase_c_negative_candidate(&candidate.key).is_none())
        .collect()
}

/// The census's ownership gate: panic on the first derived negative-capable
/// candidate the narrow owner rule does not classify, and return the set of
/// classified owners otherwise. The live census and the falsifying negative
/// control both pass through this function, so the control exercises the very
/// failure the census enforces.
fn enforce_negative_sentinel_ownership(
    candidates: &[NegativeDeclaration],
) -> BTreeSet<&'static str> {
    if let Some(candidate) = unowned_phase_c_candidates(candidates).first() {
        panic!(
            "unowned negative-capable Phase-C candidate `{}` declared by {} (negative default {:?}, negative lower bound {:?}); classify its owner before this packet can claim the census is complete",
            candidate.key, candidate.provenance, candidate.negative_default, candidate.negative_min
        );
    }
    candidates
        .iter()
        .filter_map(|candidate| classify_phase_c_negative_candidate(&candidate.key))
        .map(NegativeSentinelOwner::key)
        .collect()
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn real_manifest_paths() -> Vec<PathBuf> {
    let modules_dir = workspace_root().join("modules/core-modules");
    let mut paths = fs::read_dir(&modules_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", modules_dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("cannot read module directory entry: {error}"))
                .path()
        })
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "real core-module manifest census found no module directories"
    );
    paths
}

fn toml_default_to_wire(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => text.clone(),
        toml::Value::Array(items) => items
            .iter()
            .map(|item| match item {
                toml::Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(","),
        other => other.to_string(),
    }
}

fn toml_value_as_f64(value: &toml::Value) -> Option<f64> {
    value
        .as_float()
        .or_else(|| value.as_integer().map(|int| int as f64))
}

fn parse_manifest_field(key: &str, value: &toml::Value) -> ConfigFieldEntry {
    if let Some(field_type) = value.as_str() {
        return ConfigFieldEntry {
            field_type: field_type.to_owned(),
            ..ConfigFieldEntry::default()
        };
    }

    let table = value
        .as_table()
        .unwrap_or_else(|| panic!("config.schema.{key} must be a string or a table"));
    ConfigFieldEntry {
        field_type: table
            .get("type")
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("config.schema.{key}.type is required"))
            .to_owned(),
        default: table.get("default").map(toml_default_to_wire),
        min: table.get("min").and_then(toml_value_as_f64),
        max: table.get("max").and_then(toml_value_as_f64),
        base_key: table
            .get("base_key")
            .and_then(toml::Value::as_str)
            .map(str::to_owned),
        values: table
            .get("values")
            .and_then(toml::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            }),
        selector: table
            .get("selector")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false),
        denied_scopes: table
            .get("denied_scopes")
            .and_then(toml::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        omit_from_config_block: !table
            .get("config_block")
            .and_then(toml::Value::as_bool)
            .unwrap_or(true),
        ..ConfigFieldEntry::default()
    }
}

fn parse_real_module_declarations() -> Vec<ModuleDeclaration> {
    let mut declarations = Vec::new();
    for module_dir in real_manifest_paths() {
        let stem = module_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| panic!("module directory is not valid UTF-8: {module_dir:?}"));
        let manifest_path = module_dir.join(format!("{stem}.toml"));
        let text = fs::read_to_string(&manifest_path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", manifest_path.display()));
        let document: toml::Value = toml::from_str(&text)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", manifest_path.display()));
        let module_id = document
            .get("module")
            .and_then(toml::Value::as_table)
            .and_then(|module| module.get("id"))
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| {
                panic!(
                    "manifest {} has no string module.id",
                    manifest_path.display()
                )
            })
            .to_owned();
        let schema = document
            .get("config")
            .and_then(toml::Value::as_table)
            .and_then(|config| config.get("schema"))
            .and_then(toml::Value::as_table)
            .unwrap_or_else(|| {
                panic!(
                    "manifest {} has no [config.schema] table",
                    manifest_path.display()
                )
            });
        let entries = schema
            .iter()
            .map(|(key, value)| (key.clone(), parse_manifest_field(key, value)))
            .collect();
        declarations.push(ModuleDeclaration {
            module_id,
            schema: ConfigSchema { entries },
            ..ModuleDeclaration::default()
        });
    }
    assert!(
        !declarations.is_empty(),
        "real core-module manifest parse produced no module declarations"
    );
    declarations
}

fn is_negative(value: f64) -> bool {
    value.is_finite() && value < 0.0
}

/// The negative numeric value carried by a declaration's default wire string.
///
/// Production renders array defaults comma-joined (`toml_default_to_wire` in
/// `slicer-scheduler`'s manifest reader; `HostWireField::wire_default` for
/// `Vec<f64>`/`Vec<String>`), so a scalar and an array element share one wire
/// shape and are inspected the same way: every comma-separated element is
/// parsed, and the first negative numeric element is the candidate. A
/// non-numeric element (`enum`/string-list text, `"100%"` magnitudes) is not
/// a numeric scalar and cannot be a negative default.
fn negative_default_number(wire: &str) -> Option<f64> {
    wire.split(',')
        .filter_map(|element| element.trim().parse::<f64>().ok())
        .find(|value| is_negative(*value))
}

fn push_negative_candidate(
    candidates: &mut Vec<NegativeDeclaration>,
    key: &str,
    provenance: &str,
    default: Option<&str>,
    min: Option<f64>,
) {
    let negative_default = default.and_then(negative_default_number);
    let negative_min = min.filter(|value| is_negative(*value));
    if negative_default.is_some() || negative_min.is_some() {
        candidates.push(NegativeDeclaration {
            key: key.to_owned(),
            provenance: provenance.to_owned(),
            negative_default,
            negative_min,
        });
    }
}

/// Whether a reconciled [`RegistryEntry`] carries a negative numeric default
/// (scalar or one element of an array default) or a negative lower bound.
/// Shared by the per-declaration registry check, the registry-wide derivation,
/// and the falsifying negative control, so all three take the same path.
fn registry_entry_is_negative(entry: &RegistryEntry) -> bool {
    entry
        .default
        .as_deref()
        .and_then(negative_default_number)
        .is_some()
        || entry.min.is_some_and(is_negative)
}

/// Every reconciled registry key whose default or lower bound is negative,
/// derived straight off the assembled registry.
fn registry_negative_keys(registry: &ConfigSchemaRegistry) -> BTreeSet<String> {
    registry
        .keys()
        .filter(|key| registry.entry(key).is_some_and(registry_entry_is_negative))
        .map(str::to_owned)
        .collect()
}

fn host_negative_declarations(host: &HostChannels) -> Vec<NegativeDeclaration> {
    let mut candidates = Vec::new();
    for row in &host.host_keys {
        push_negative_candidate(
            &mut candidates,
            row.key,
            "HostChannels::from_live (ResolvedConfig)",
            row.default.as_deref(),
            row.meta.min,
        );
    }
    for row in &host.speed_keys {
        push_negative_candidate(
            &mut candidates,
            row.key,
            "HostChannels::from_live (feedrate)",
            row.default.as_deref(),
            row.meta.min,
        );
    }
    for row in &host.runtime_keys {
        push_negative_candidate(
            &mut candidates,
            row.key,
            "HostChannels::from_live (host runtime)",
            row.default,
            row.meta.min,
        );
    }
    candidates
}

fn module_negative_declarations(modules: &[ModuleDeclaration]) -> Vec<NegativeDeclaration> {
    let mut candidates = Vec::new();
    for module in modules {
        for (key, field) in &module.schema.entries {
            push_negative_candidate(
                &mut candidates,
                key,
                &module.module_id,
                field.default.as_deref(),
                field.min,
            );
        }
    }
    candidates
}

/// Build a control `ModuleDeclaration` from a manifest-shaped schema TOML
/// document through the same parse path real manifests take
/// ([`parse_manifest_field`]), so the control is discovered and classified
/// exactly like a live declaration rather than injected after the fact.
fn control_declaration(schema_toml: &str) -> ModuleDeclaration {
    let document: toml::Value = toml::from_str(schema_toml)
        .unwrap_or_else(|error| panic!("control schema TOML must parse: {error}"));
    let table = document
        .as_table()
        .expect("control schema document must be a table");
    let entries = table
        .iter()
        .map(|(key, value)| (key.clone(), parse_manifest_field(key, value)))
        .collect();
    ModuleDeclaration {
        module_id: "dev.pinch.test.negative-control".to_owned(),
        schema: ConfigSchema { entries },
        ..ModuleDeclaration::default()
    }
}

/// The message text of a caught panic payload (a `String` or `&'static str`).
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .unwrap_or_else(|| "<non-string panic payload>".to_owned())
}

/// Derive every negative-capable declaration from the live channels and real
/// manifests, assemble the live registry from those exact declarations, and
/// fail on any derived key the narrow owner rule does not classify.
#[test]
fn registry_negative_sentinel_census_has_no_unowned_phase_c_candidate() {
    let modules = parse_real_module_declarations();
    let host = HostChannels::from_live();
    let registry = assemble_registry(&modules, &host)
        .unwrap_or_else(|error| {
            panic!("live registry must assemble for the negative-sentinel census: {error:?}")
        })
        .registry;

    let mut candidates = module_negative_declarations(&modules);
    candidates.extend(host_negative_declarations(&host));
    candidates.sort_by(|first, second| {
        (&first.key, &first.provenance).cmp(&(&second.key, &second.provenance))
    });
    assert!(
        !candidates.is_empty(),
        "census derived no negative-capable declaration; the two packet-04 config-only sentinels are live, so the derivation is broken"
    );

    // Ownership gate, shared with the falsifying negative control at the end
    // of this test: any derived candidate the narrow owner rule does not
    // classify is an unowned Phase-C candidate and fails the census loudly.
    let classified = enforce_negative_sentinel_ownership(&candidates);

    for candidate in &candidates {
        let entry = registry.entry(&candidate.key).unwrap_or_else(|| {
            panic!(
                "negative-capable declaration `{}` from {} is absent from the assembled live registry",
                candidate.key, candidate.provenance
            )
        });
        assert!(
            registry_entry_is_negative(entry),
            "negative-capable declaration `{}` from {} (negative default {:?}, negative lower bound {:?}) is not negative in the reconciled registry (default {:?}, lower bound {:?})",
            candidate.key,
            candidate.provenance,
            candidate.negative_default,
            candidate.negative_min,
            entry.default,
            entry.min
        );
    }

    // Second derivation, straight off the assembled registry: any entry whose
    // reconciled default or reconciled lower bound is negative must also be
    // classified. This closes the gap where a merge path could produce a
    // negative value the per-declaration scan does not model.
    let registry_negatives = registry_negative_keys(&registry);
    assert!(
        !registry_negatives.is_empty(),
        "assembled live registry has no negative-capable entry; the two packet-04 sentinels are live, so the registry derivation is broken"
    );
    for key in &registry_negatives {
        assert!(
            classify_phase_c_negative_candidate(key).is_some(),
            "assembled live registry exposes unowned negative-capable entry `{key}`; classify its owner before this packet can claim the census is complete"
        );
    }

    // The two packet-04 owners keep their existing classifications; the census
    // must not silently reclassify them after the array-aware derivation.
    // (The production cross-check below re-asserts membership per owner.)

    // Reconciled-metadata bound, derived from the live assembled registry so
    // it agrees with `docs/config/host-keys.toml` (`>= 0`) and doc 15: the
    // automatic volumetric key's lower bound is `Some(0.0)`, and zero itself
    // stays admissible (0 is only "unavailable" when an automatic speed is
    // requested, which is the emitter's concern, not the declaration's).
    let volumetric = registry
        .entry("filament_max_volumetric_speed")
        .expect("the automatic volumetric key must reach the live registry");
    assert_eq!(
        volumetric.min,
        Some(0.0),
        "filament_max_volumetric_speed must reconcile to lower bound Some(0.0) matching host-keys.toml >= 0"
    );
    assert_eq!(
        volumetric
            .default
            .as_deref()
            .and_then(|wire| wire.parse::<f64>().ok()),
        Some(0.0),
        "filament_max_volumetric_speed default must stay the live zero default"
    );
    assert!(
        volumetric.min.is_some_and(|min| 0.0_f64 >= min),
        "zero must remain acceptable to the declared lower bound"
    );

    // Production cross-check: a classified owner must actually be claimed by
    // the Phase B automatic-value resolver. An unowned negative value is not an
    // automatic sentinel, so `expect_err` discriminates ownership from the
    // production path rather than restating the test's own classification.
    for owner in [
        NegativeSentinelOwner::BottomLayers,
        NegativeSentinelOwner::BottomSpacing,
    ] {
        assert!(
            classified.contains(owner.key()),
            "census lost packet-04-owned key {}",
            owner.key()
        );
        let mut config = config_without_automatic_widths();
        config
            .extensions
            .insert(owner.key().to_owned(), owner.sentinel());
        let error = expand_automatic_values(
            &registry,
            &mut config,
            &ExpansionContext::default(),
            None,
        )
        .expect_err(
            "a classified packet-04 negative sentinel must be claimed by the automatic-value resolver",
        );
        match error {
            ExpansionError::MissingAutoBase { key, .. } => assert_eq!(
                key,
                owner.key(),
                "the missing-base error must name the classified key"
            ),
            other => panic!(
                "expected MissingAutoBase for {}, got {other:?}",
                owner.key()
            ),
        }
    }

    // Negative controls for the ownership oracle (docs/22 §3): the classifier
    // must reject a negative-capable key with no rule, and the production
    // resolver must leave an unowned negative value untouched instead of
    // claiming it as an automatic sentinel.
    assert!(
        classify_phase_c_negative_candidate("synthetic_unowned_negative_sentinel").is_none(),
        "the ownership rule must not silently accept a key it does not classify"
    );

    let mut unowned = config_without_automatic_widths();
    unowned.extensions.insert(
        "synthetic_unowned_negative_sentinel".to_owned(),
        ConfigValue::Float(-1.0),
    );
    expand_automatic_values(&registry, &mut unowned, &ExpansionContext::default(), None)
        .expect("an unowned negative value is not an automatic sentinel");
    assert_eq!(
        unowned
            .extensions
            .get("synthetic_unowned_negative_sentinel"),
        Some(&ConfigValue::Float(-1.0)),
        "an unowned negative value must pass through the automatic-value resolver"
    );

    // Falsifying negative control through the same discovery and classification
    // path the live census takes (docs/22 §3): a manifest-shaped declaration
    // with an unknown key and a negative numeric element inside an array
    // default must be discovered by `module_negative_declarations` and rejected
    // by the shared ownership gate. If the array-aware derivation regresses to
    // scalar-only parsing, the control yields no candidate, the gate returns
    // without panicking, and this test fails — the control falsifies the very
    // gap it guards.
    let control = control_declaration(
        "[negative_control_array_default]\n\
         type = \"float-list\"\n\
         default = [0.4, -1.25]\n",
    );
    let control_candidates = module_negative_declarations(&[control]);
    let control_candidate = control_candidates
        .iter()
        .find(|candidate| candidate.key == "negative_control_array_default")
        .expect(
            "the array-aware derivation must detect a negative numeric element in an array default",
        );
    assert_eq!(
        control_candidate.negative_default,
        Some(-1.25),
        "the derived candidate must carry the control's negative array element"
    );
    let control_panic = catch_unwind(AssertUnwindSafe(|| {
        enforce_negative_sentinel_ownership(&control_candidates)
    }))
    .expect_err("the shared ownership gate must reject the unclassified control declaration");
    let control_message = panic_message(control_panic.as_ref());
    assert!(
        control_message.contains("negative_control_array_default"),
        "the rejection must name the unclassified control key, got {control_message:?}"
    );

    // The same discovery path must NOT treat a non-numeric array default as a
    // negative candidate: string-list text elements are not numeric scalars.
    let non_numeric = control_declaration(
        "[negative_control_string_list]\n\
         type = \"string-list\"\n\
         default = [\"red\", \"green\"]\n",
    );
    assert!(
        module_negative_declarations(&[non_numeric]).is_empty(),
        "a non-numeric array default must not become a negative-capable candidate"
    );
}

/// Focused live check of the reconciled registry entry for the automatic
/// volumetric key: the assembled live registry carries the zero default and
/// the `Some(0.0)` lower bound that `docs/config/host-keys.toml` (`>= 0`) and
/// generated doc 15 state, zero stays admissible, and a negative maximum falls
/// below the declared bound that the resolution paths compare against.
#[test]
fn live_registry_reconciles_volumetric_default_and_lower_bound() {
    let modules = parse_real_module_declarations();
    let host = HostChannels::from_live();
    let registry = assemble_registry(&modules, &host)
        .unwrap_or_else(|error| panic!("live registry must assemble: {error:?}"))
        .registry;

    let entry = registry
        .entry("filament_max_volumetric_speed")
        .expect("the volumetric key must declare through HostChannels::from_live");

    assert_eq!(
        entry.field_type, "float",
        "the volumetric key is a scalar float declaration"
    );
    assert_eq!(
        entry
            .default
            .as_deref()
            .and_then(|wire| wire.parse::<f64>().ok()),
        Some(0.0),
        "the reconciled default must parse as zero"
    );
    assert_eq!(
        entry.min,
        Some(0.0),
        "the reconciled lower bound must be Some(0.0), matching host-keys.toml >= 0"
    );

    // Compare against `entry.min` exactly as the bounds paths do, so the
    // assertions exercise the reconciled bound rather than a restated zero.
    let min = entry.min.expect("the reconciled lower bound is required");
    assert!(
        0.0 >= min,
        "zero must remain admissible under the reconciled lower bound {min}"
    );
    assert!(
        -1.0 < min,
        "a negative volumetric maximum must fall below the reconciled lower bound {min}"
    );
}

#[test]
fn declared_volumetric_extension_resolves_defaults_and_tool_precedence() {
    let registry = assemble_registry(&[], &HostChannels::from_live())
        .expect("host registry must assemble")
        .registry;
    let expansion = ExpansionContext {
        nozzle_diameter_mm: 0.4,
        ..ExpansionContext::default()
    };
    let default_config = resolve_scope_stack(
        &registry,
        &ScopedConfig::default(),
        &ResolutionTarget::default(),
        &expansion,
    )
    .expect("registry defaults must resolve");
    assert_eq!(
        default_config
            .extensions
            .get("filament_max_volumetric_speed"),
        Some(&ConfigValue::Float(0.0)),
        "the declared default must be seeded in the existing extension carrier"
    );

    let scoped = ScopedConfig {
        deltas: BTreeMap::from([
            (
                ConfigScope::Global,
                ScopeDelta {
                    values: BTreeMap::from([(
                        "filament_max_volumetric_speed".to_owned(),
                        ConfigValue::Float(20.0),
                    )]),
                },
            ),
            (
                ConfigScope::Tool(1),
                ScopeDelta {
                    values: BTreeMap::from([(
                        "filament_max_volumetric_speed".to_owned(),
                        ConfigValue::List(vec![ConfigValue::Float(12.0)]),
                    )]),
                },
            ),
        ]),
        ..ScopedConfig::default()
    };
    for (tool_index, expected) in [(None, 20.0), (Some(1), 12.0), (Some(2), 20.0)] {
        let config = resolve_scope_stack(
            &registry,
            &scoped,
            &ResolutionTarget {
                tool_index,
                ..ResolutionTarget::default()
            },
            &expansion,
        )
        .expect("global/tool settings must resolve");
        assert_eq!(
            config
                .filament_max_volumetric_speed()
                .expect("numeric resolved limit"),
            expected,
            "tool {tool_index:?} must consume its resolved scalar or first filament envelope value"
        );
        assert!(config
            .to_config_map()
            .contains_key("filament_max_volumetric_speed"));
    }
}

#[test]
fn declared_volumetric_extension_rejects_invalid_values_and_object_scope() {
    let registry = assemble_registry(&[], &HostChannels::from_live())
        .expect("host registry must assemble")
        .registry;
    let expansion = ExpansionContext {
        nozzle_diameter_mm: 0.4,
        ..ExpansionContext::default()
    };
    for value in [
        ConfigValue::Float(-1.0),
        ConfigValue::Float(f64::NAN),
        ConfigValue::Float(f64::INFINITY),
        ConfigValue::Bool(true),
        ConfigValue::List(vec![]),
    ] {
        let scoped = ScopedConfig {
            deltas: BTreeMap::from([(
                ConfigScope::Global,
                ScopeDelta {
                    values: BTreeMap::from([("filament_max_volumetric_speed".to_owned(), value)]),
                },
            )]),
            ..ScopedConfig::default()
        };
        let error =
            resolve_scope_stack(&registry, &scoped, &ResolutionTarget::default(), &expansion)
                .expect_err("the declared extension must not bypass registry numeric validation");
        assert!(matches!(error, ResolutionError::Application(_)));
        assert!(error.to_string().contains("filament_max_volumetric_speed"));
    }
    let scoped = ScopedConfig {
        deltas: BTreeMap::from([(
            ConfigScope::Object("object-a".to_owned()),
            ScopeDelta {
                values: BTreeMap::from([(
                    "filament_max_volumetric_speed".to_owned(),
                    ConfigValue::Float(8.0),
                )]),
            },
        )]),
        ..ScopedConfig::default()
    };
    let error = resolve_scope_stack(
        &registry,
        &scoped,
        &ResolutionTarget {
            object_id: "object-a".to_owned(),
            ..ResolutionTarget::default()
        },
        &expansion,
    )
    .expect_err("a per-filament limit must remain inadmissible at object scope");
    assert_eq!(
        error,
        ResolutionError::ScopeDenied {
            key: "filament_max_volumetric_speed".to_owned(),
            scope: ConfigScope::Object("object-a".to_owned()),
        }
    );
}
