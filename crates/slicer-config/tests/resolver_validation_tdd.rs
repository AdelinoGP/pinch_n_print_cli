//! Public-boundary regressions: ingestion must not depend on scheduler adapters.

use std::collections::{BTreeMap, HashMap};

use slicer_config::ResolutionError;
use slicer_config::{
    assemble_registry, resolve_scope_stack, ConfigIngestor, ConfigSchemaRegistry, ConfigScope,
    ExpansionContext, HostChannels, ModuleDeclaration, ResolutionTarget,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::{ConfigResolutionError, ConfigValue};

fn registry(entries: BTreeMap<String, ConfigFieldEntry>) -> ConfigSchemaRegistry {
    assemble_registry(
        &[ModuleDeclaration {
            module_id: "dev.pinch.test.resolver-validation".to_owned(),
            schema: ConfigSchema { entries },
            ..ModuleDeclaration::default()
        }],
        &HostChannels::from_live(),
    )
    .expect("fixture registry must assemble")
    .registry
}

fn expansion() -> ExpansionContext {
    ExpansionContext {
        nozzle_diameter_mm: 0.4,
        ..ExpansionContext::default()
    }
}

#[test]
fn live_wall_speed_schema_admits_auto_zero_and_retains_numeric_bounds() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/classic-perimeters/classic-perimeters.toml");
    let manifest: toml::Value = std::fs::read_to_string(manifest_path)
        .unwrap()
        .parse()
        .unwrap();
    let entries = ["outer_wall_speed", "inner_wall_speed", "gap_infill_speed"]
        .into_iter()
        .map(|key| {
            let field = &manifest["config"]["schema"][key];
            (
                key.to_owned(),
                ConfigFieldEntry {
                    field_type: field["type"].as_str().unwrap().to_owned(),
                    min: Some(field["min"].as_float().unwrap()),
                    max: Some(field["max"].as_float().unwrap()),
                    ..ConfigFieldEntry::default()
                },
            )
        })
        .collect();
    let registry = registry(entries);
    for key in ["outer_wall_speed", "inner_wall_speed", "gap_infill_speed"] {
        for value in [0.0, 30.0, -1.0, 301.0] {
            let mut ingestor = ConfigIngestor::new(&registry);
            ingestor
                .ingest_flat(&HashMap::from([(
                    key.to_owned(),
                    ConfigValue::Float(value),
                )]))
                .unwrap();
            let result = resolve_scope_stack(
                &registry,
                &ingestor.finish().scoped,
                &ResolutionTarget::default(),
                &expansion(),
            );
            if !(0.0..=300.0).contains(&value) {
                assert!(
                    matches!(
                        result,
                        Err(ResolutionError::Application(
                            ConfigResolutionError::OutOfRange { .. }
                        ))
                    ),
                    "{key}={value}: {result:?}"
                );
            } else {
                let resolved = result.expect("zero AUTO and explicit speeds must resolve");
                assert_eq!(
                    resolved.to_config_map().get(key),
                    Some(&ConfigValue::Float(value))
                );
            }
        }
    }
}

#[test]
fn ingested_aliases_resolve_at_public_boundary_and_preserve_scope_precedence() {
    let registry = registry(BTreeMap::new());
    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_delta(
            ConfigScope::Global,
            &HashMap::from([
                (
                    "initial_layer_line_width".to_owned(),
                    ConfigValue::Float(0.8),
                ),
                (
                    "support_threshold_angle".to_owned(),
                    ConfigValue::Float(50.0),
                ),
            ]),
        )
        .unwrap();
    ingestor
        .ingest_delta(
            ConfigScope::Object("obj".to_owned()),
            &HashMap::from([
                ("first_layer_line_width".to_owned(), ConfigValue::Float(0.5)),
                (
                    "support_overhang_angle".to_owned(),
                    ConfigValue::Float(30.0),
                ),
            ]),
        )
        .unwrap();
    let resolved = resolve_scope_stack(
        &registry,
        &ingestor.finish().scoped,
        &ResolutionTarget {
            object_id: "obj".to_owned(),
            ..ResolutionTarget::default()
        },
        &expansion(),
    )
    .expect("ingested legacy aliases must resolve without scheduler preparation");
    assert_eq!(resolved.initial_layer_line_width.value, 0.5);
    assert!(!resolved.initial_layer_line_width.is_percent);
    assert_eq!(resolved.support_threshold_angle, 30.0);
    assert!(!resolved.extensions.contains_key("first_layer_line_width"));
    assert!(!resolved.extensions.contains_key("support_overhang_angle"));
}

#[test]
fn legacy_registry_default_does_not_seed_a_stale_second_typed_identity() {
    let registry = registry(BTreeMap::from([(
        "support_overhang_angle".to_owned(),
        ConfigFieldEntry {
            field_type: "float".to_owned(),
            default: Some("30".to_owned()),
            ..Default::default()
        },
    )]));
    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_delta(
            ConfigScope::Object("obj".to_owned()),
            &HashMap::from([(
                "support_threshold_angle".to_owned(),
                ConfigValue::Float(47.0),
            )]),
        )
        .unwrap();
    let resolved = resolve_scope_stack(
        &registry,
        &ingestor.finish().scoped,
        &ResolutionTarget {
            object_id: "obj".to_owned(),
            ..Default::default()
        },
        &expansion(),
    )
    .unwrap();
    assert_eq!(resolved.support_threshold_angle, 47.0);
    assert!(!resolved.extensions.contains_key("support_overhang_angle"));
    assert!(!resolved
        .to_config_map()
        .contains_key("support_overhang_angle"));
}

#[test]
fn both_alias_spellings_in_one_scope_are_rejected_even_with_equal_values() {
    let registry = registry(BTreeMap::new());
    for (legacy, canonical, value) in [
        (
            "first_layer_line_width",
            "initial_layer_line_width",
            ConfigValue::Float(0.5),
        ),
        (
            "support_overhang_angle",
            "support_threshold_angle",
            ConfigValue::Float(30.0),
        ),
    ] {
        let mut ingestor = ConfigIngestor::new(&registry);
        ingestor
            .ingest_delta(
                ConfigScope::Global,
                &HashMap::from([
                    (legacy.to_owned(), value.clone()),
                    (canonical.to_owned(), value),
                ]),
            )
            .unwrap();
        let error = resolve_scope_stack(
            &registry,
            &ingestor.finish().scoped,
            &ResolutionTarget::default(),
            &expansion(),
        )
        .expect_err("both spellings must be rejected independently of their values");
        assert_eq!(
            error,
            ResolutionError::Application(ConfigResolutionError::TypeMismatch {
                key: format!("{canonical} and {legacy}"),
                expected: "one config key",
                actual: "both config keys supplied".to_owned(),
            })
        );
    }
}

#[test]
fn typed_layer_height_obeys_registry_maximum_before_application() {
    let registry = registry(BTreeMap::from([(
        "layer_height".to_owned(),
        ConfigFieldEntry {
            field_type: "float".to_owned(),
            min: Some(0.0),
            max: Some(1.0),
            ..ConfigFieldEntry::default()
        },
    )]));
    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_delta(
            ConfigScope::Global,
            &HashMap::from([("layer_height".to_owned(), ConfigValue::Float(2.0))]),
        )
        .unwrap();
    let error = resolve_scope_stack(
        &registry,
        &ingestor.finish().scoped,
        &ResolutionTarget::default(),
        &expansion(),
    )
    .expect_err("typed fields must not bypass registry bounds");
    assert_eq!(
        error,
        ResolutionError::Application(ConfigResolutionError::OutOfRange {
            key: "layer_height".to_owned(),
            value: 2.0,
            min: Some(0.0),
            max: Some(1.0),
            index: None,
        })
    );
}

#[test]
fn negative_wall_count_is_rejected_before_unsigned_cast() {
    let registry = registry(BTreeMap::from([(
        "wall_count".to_owned(),
        ConfigFieldEntry {
            field_type: "int".to_owned(),
            min: Some(0.0),
            ..ConfigFieldEntry::default()
        },
    )]));
    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_delta(
            ConfigScope::Global,
            &HashMap::from([("wall_count".to_owned(), ConfigValue::Int(-1))]),
        )
        .unwrap();
    let error = resolve_scope_stack(
        &registry,
        &ingestor.finish().scoped,
        &ResolutionTarget::default(),
        &expansion(),
    )
    .expect_err("negative wall_count must not wrap to u32::MAX");
    assert_eq!(
        error,
        ResolutionError::Application(ConfigResolutionError::OutOfRange {
            key: "wall_count".to_owned(),
            value: -1.0,
            min: Some(0.0),
            max: None,
            index: None,
        })
    );
}

#[test]
fn unsigned_typed_counts_reject_negative_and_overflow_without_registry_bounds() {
    let registry = registry(BTreeMap::from([(
        "wall_count".to_owned(),
        ConfigFieldEntry {
            field_type: "int".to_owned(),
            ..Default::default()
        },
    )]));
    for value in [-1, i64::from(u32::MAX) + 1] {
        let mut ingestor = ConfigIngestor::new(&registry);
        ingestor
            .ingest_delta(
                ConfigScope::Global,
                &HashMap::from([("wall_count".into(), ConfigValue::Int(value))]),
            )
            .unwrap();
        let error = resolve_scope_stack(
            &registry,
            &ingestor.finish().scoped,
            &ResolutionTarget::default(),
            &expansion(),
        )
        .expect_err("unsigned extraction must remain checked without schema bounds");
        assert_eq!(
            error,
            ResolutionError::Application(ConfigResolutionError::OutOfRange {
                key: "wall_count".into(),
                value: value as f64,
                min: Some(0.0),
                max: Some(f64::from(u32::MAX)),
                index: None,
            })
        );
    }
    for value in [0, u32::MAX] {
        let mut ingestor = ConfigIngestor::new(&registry);
        ingestor
            .ingest_delta(
                ConfigScope::Global,
                &HashMap::from([("wall_count".into(), ConfigValue::Int(i64::from(value)))]),
            )
            .unwrap();
        let resolved = resolve_scope_stack(
            &registry,
            &ingestor.finish().scoped,
            &ResolutionTarget::default(),
            &expansion(),
        )
        .expect("representable unsigned counts must remain accepted");
        assert_eq!(resolved.wall_count, value);
    }
}
