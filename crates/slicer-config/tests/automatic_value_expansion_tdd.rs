//! Independent literal-oracle coverage for Phase B automatic-value expansion.

use std::collections::BTreeMap;

use slicer_config::{
    assemble_registry, expand_automatic_values, ConfigSchemaRegistry, ExpansionContext,
    ExpansionError, HostChannels, ModuleDeclaration,
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
