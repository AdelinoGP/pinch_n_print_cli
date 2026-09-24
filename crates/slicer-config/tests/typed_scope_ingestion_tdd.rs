//! Behavior-first tests for registry-typed, scope-aware config ingestion.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use slicer_config::{
    assemble_registry, ConfigIngestionError, ConfigIngestor, ConfigSchemaRegistry, ConfigScope,
    HostChannels, IngestionWarning, ModuleDeclaration,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::resolved_config::{ResolvedConfig, HOST_RUNTIME_KEYS};
use slicer_ir::ConfigValue;

struct ParsedManifest {
    module_id: String,
    field_types: BTreeMap<String, String>,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest_paths() -> Vec<PathBuf> {
    let modules_dir = workspace_root().join("modules/core-modules");
    let mut paths = fs::read_dir(&modules_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", modules_dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("cannot read module directory entry: {error}"))
                .path()
        })
        .filter(|path| path.is_dir())
        .map(|module_dir| {
            let stem = module_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_else(|| panic!("module directory is not valid UTF-8: {module_dir:?}"));
            let manifest = module_dir.join(format!("{stem}.toml"));
            assert!(
                manifest.is_file(),
                "module directory {module_dir:?} has no same-stem manifest"
            );
            manifest
        })
        .collect::<Vec<_>>();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "real core-module manifest census found no same-stem manifests"
    );
    paths
}

fn read_manifest(path: &Path) -> toml::Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read manifest {}: {error}", path.display()));
    toml::from_str(&text)
        .unwrap_or_else(|error| panic!("cannot parse manifest {}: {error}", path.display()))
}

fn manifest_module_id<'a>(document: &'a toml::Value, path: &Path) -> &'a str {
    document
        .get("module")
        .and_then(toml::Value::as_table)
        .and_then(|module| module.get("id"))
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("manifest {} has no string module.id", path.display()))
}

fn manifest_schema<'a>(
    document: &'a toml::Value,
    path: &Path,
) -> &'a toml::map::Map<String, toml::Value> {
    document
        .get("config")
        .and_then(toml::Value::as_table)
        .and_then(|config| config.get("schema"))
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("manifest {} has no [config.schema] table", path.display()))
}

fn manifest_field_type(value: &toml::Value, key: &str, path: &Path) -> String {
    if let Some(field_type) = value.as_str() {
        return field_type.to_owned();
    }

    value
        .as_table()
        .and_then(|field| field.get("type"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| {
            panic!(
                "manifest {} has no string type for config.schema.{key}",
                path.display()
            )
        })
}

fn parse_real_manifests() -> Vec<ParsedManifest> {
    manifest_paths()
        .into_iter()
        .map(|path| {
            let document = read_manifest(&path);
            let module_id = manifest_module_id(&document, &path).to_owned();
            let field_types = manifest_schema(&document, &path)
                .iter()
                .map(|(key, value)| (key.clone(), manifest_field_type(value, key, &path)))
                .collect();
            ParsedManifest {
                module_id,
                field_types,
            }
        })
        .collect()
}

fn host_field_types() -> BTreeMap<String, String> {
    let mut field_types = BTreeMap::new();
    for row in ResolvedConfig::host_config_keys() {
        field_types
            .entry(row.key.to_owned())
            .or_insert_with(|| row.field_type.to_owned());
    }
    for &(key, _) in slicer_ir::feedrate::SPEED_KEYS {
        let field_type = if key == "internal_bridge_speed"
            || matches!(
                key,
                "overhang_1_4_speed"
                    | "overhang_2_4_speed"
                    | "overhang_3_4_speed"
                    | "overhang_4_4_speed"
            ) {
            "float_or_percent"
        } else {
            "float"
        };
        field_types
            .entry(key.to_owned())
            .or_insert_with(|| field_type.to_owned());
    }
    for row in HOST_RUNTIME_KEYS {
        field_types
            .entry(row.key.to_owned())
            .or_insert_with(|| row.field_type.to_owned());
    }
    field_types
}

fn real_registry() -> slicer_config::ConfigSchemaRegistry {
    let manifests = parse_real_manifests();
    let mut host_types = host_field_types();
    for manifest in &manifests {
        for (key, field_type) in &manifest.field_types {
            host_types
                .entry(key.clone())
                .or_insert_with(|| field_type.clone());
        }
    }

    let mut modules = manifests
        .into_iter()
        .map(|manifest| ModuleDeclaration {
            module_id: manifest.module_id,
            schema: ConfigSchema {
                entries: manifest
                    .field_types
                    .into_iter()
                    .map(|(key, field_type)| {
                        let field_type = host_types.get(&key).cloned().unwrap_or(field_type);
                        (
                            key,
                            ConfigFieldEntry {
                                field_type,
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
            },
            claim_exclusive_group: None,
        })
        .collect::<Vec<_>>();

    // The flat-wire scope fixture (`flat_wire_keys_decode_once_into_typed_scopes`)
    // authors Object/Modifier/PaintSemantic wire keys whose sub-keys
    // (`wall_loops`, `fuzzy_skin_point_dist`) no real core-module manifest
    // declares. Register them on a synthetic fixture module so warn-then-drop
    // ingestion retains them and the scoped deltas materialize; the test's
    // assertions are scope-shape assertions, not output blessings.
    modules.push(ModuleDeclaration {
        module_id: "scope-fixture".to_owned(),
        schema: ConfigSchema {
            entries: BTreeMap::from([
                (
                    "wall_loops".to_owned(),
                    ConfigFieldEntry {
                        field_type: "int".to_owned(),
                        ..Default::default()
                    },
                ),
                (
                    "fuzzy_skin_point_dist".to_owned(),
                    ConfigFieldEntry {
                        field_type: "float".to_owned(),
                        ..Default::default()
                    },
                ),
            ]),
        },
        claim_exclusive_group: None,
    });

    assemble_registry(&modules, &HostChannels::from_live())
        .unwrap_or_else(|error| panic!("real manifest registry failed to assemble: {error:?}"))
        .registry
}

/// Return every key the real registry declares with `field_type`.
///
/// The new non-finite guards below must exercise keys the assembled registry
/// really declares, so the declared keys are read from the registry itself
/// rather than hard-coded here.
fn declared_keys_with_type(registry: &ConfigSchemaRegistry, field_type: &str) -> Vec<String> {
    let mut keys = registry
        .keys()
        .filter(|key| {
            registry
                .entry(key)
                .is_some_and(|entry| entry.field_type == field_type)
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    keys.sort();
    assert!(
        !keys.is_empty(),
        "real registry declares no {field_type} key; the non-finite guard cannot be exercised"
    );
    keys
}

#[test]
fn flat_wire_keys_decode_once_into_typed_scopes() {
    let registry = real_registry();
    let flat = HashMap::from([
        (
            "layer_height".to_owned(),
            ConfigValue::String("0.2".to_owned()),
        ),
        (
            "object_height:obj-a".to_owned(),
            ConfigValue::String("0.3".to_owned()),
        ),
        (
            "object_config:obj-a:wall_loops".to_owned(),
            ConfigValue::String("3".to_owned()),
        ),
        (
            "paint_config:fuzzy_skin:fuzzy_skin_point_dist".to_owned(),
            ConfigValue::String("0.1".to_owned()),
        ),
        (
            "tool_config:1:retract_length".to_owned(),
            ConfigValue::String("2.0".to_owned()),
        ),
    ]);
    let modifier = HashMap::from([("wall_loops".to_owned(), ConfigValue::String("4".to_owned()))]);

    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_flat(&flat)
        .expect("flat wire keys should ingest");
    ingestor
        .ingest_delta(
            ConfigScope::Modifier {
                object_id: "obj-a".to_owned(),
                modifier_id: "mod-a".to_owned(),
            },
            &modifier,
        )
        .expect("modifier delta should ingest");
    let outcome = ingestor.finish();

    let scopes = outcome
        .scoped
        .deltas
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        scopes,
        BTreeSet::from([
            ConfigScope::Global,
            ConfigScope::Object("obj-a".to_owned()),
            ConfigScope::Modifier {
                object_id: "obj-a".to_owned(),
                modifier_id: "mod-a".to_owned(),
            },
            ConfigScope::PaintSemantic("fuzzy_skin".to_owned()),
            ConfigScope::Tool(1),
        ])
    );

    for delta in outcome.scoped.deltas.values() {
        assert!(delta.values.keys().all(|key| {
            !key.starts_with("object_config:")
                && !key.starts_with("paint_config:")
                && !key.starts_with("tool_config:")
        }));
    }
    assert_eq!(
        outcome
            .scoped
            .global()
            .and_then(|delta| delta.values.get("object_height:obj-a")),
        Some(&ConfigValue::Float(0.3))
    );
}

#[test]
fn registry_types_the_five_authored_value_oracle_divergences() {
    let registry = real_registry();
    let authored = HashMap::from([
        (
            "skirt_loops".to_owned(),
            ConfigValue::String("1".to_owned()),
        ),
        ("brim_width".to_owned(), ConfigValue::String("0".to_owned())),
        (
            "filter_out_gap_fill".to_owned(),
            ConfigValue::String("0".to_owned()),
        ),
        (
            "tree_support_wall_count".to_owned(),
            ConfigValue::String("0".to_owned()),
        ),
        (
            "support_interface_bottom_layers".to_owned(),
            ConfigValue::String("0".to_owned()),
        ),
    ]);

    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_flat(&authored)
        .expect("real registry declarations should type authored strings");
    let outcome = ingestor.finish();
    let values = &outcome.scoped.global().expect("global delta").values;

    assert_eq!(values.get("skirt_loops"), Some(&ConfigValue::Int(1)));
    assert_eq!(values.get("brim_width"), Some(&ConfigValue::Float(0.0)));
    assert_eq!(
        values.get("filter_out_gap_fill"),
        Some(&ConfigValue::Float(0.0))
    );
    assert_eq!(
        values.get("tree_support_wall_count"),
        Some(&ConfigValue::Int(0))
    );
    assert_eq!(
        values.get("support_interface_bottom_layers"),
        Some(&ConfigValue::Int(0))
    );
    assert!(!values
        .values()
        .any(|value| matches!(value, ConfigValue::Bool(_))));
}

#[test]
fn unknown_key_warns_with_near_miss_and_is_dropped() {
    let registry = real_registry();
    let authored = HashMap::from([(
        "skrit_loops".to_owned(),
        ConfigValue::String("1".to_owned()),
    )]);

    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_flat(&authored)
        .expect("unknown keys remain ingestible");
    let outcome = ingestor.finish();

    // Warn-then-drop: the near-miss key appears in no scope delta.
    for (scope, delta) in outcome.scoped.iter() {
        assert!(
            !delta.values.contains_key("skrit_loops"),
            "scope {scope:?} must not carry the dropped key"
        );
    }
    assert_eq!(
        outcome.warnings,
        vec![IngestionWarning::UnrecognizedKey {
            wire_key: "skrit_loops".to_owned(),
            key: "skrit_loops".to_owned(),
            suggestion: Some("skirt_loops".to_owned()),
        }]
    );
}

#[test]
fn malformed_scope_key_is_rejected() {
    let registry = real_registry();
    let authored = HashMap::from([(
        "object_config:obj-a".to_owned(),
        ConfigValue::String("1".to_owned()),
    )]);
    let mut ingestor = ConfigIngestor::new(&registry);

    let error = ingestor
        .ingest_flat(&authored)
        .expect_err("missing canonical sub-key must be rejected");
    assert!(matches!(
        error,
        ConfigIngestionError::MalformedScopeKey { wire_key, .. }
            if wire_key == "object_config:obj-a"
    ));
}

#[test]
fn declared_value_type_mismatch_is_rejected() {
    let registry = real_registry();
    let authored = HashMap::from([(
        "skirt_loops".to_owned(),
        ConfigValue::String("one".to_owned()),
    )]);
    let mut ingestor = ConfigIngestor::new(&registry);

    let error = ingestor
        .ingest_flat(&authored)
        .expect_err("non-numeric integer sidecar must be rejected");
    assert!(matches!(
        error,
        ConfigIngestionError::TypeMismatch {
            key,
            expected,
            authored,
        } if key == "skirt_loops" && expected == "int" && authored == "one"
    ));
}

#[test]
fn non_finite_native_float_for_declared_float_is_rejected() {
    let registry = real_registry();
    let key = declared_keys_with_type(&registry, "float")
        .into_iter()
        .next()
        .expect("registry declares at least one float key");
    let authored = HashMap::from([(key.clone(), ConfigValue::Float(f64::NAN))]);
    let mut ingestor = ConfigIngestor::new(&registry);

    let error = ingestor
        .ingest_flat(&authored)
        .expect_err("a non-finite native float must be rejected for a declared float key");
    assert!(matches!(
        error,
        ConfigIngestionError::TypeMismatch {
            key: reported_key,
            expected,
            ..
        } if reported_key == key && expected == "float"
    ));
}

#[test]
fn non_finite_native_float_element_for_declared_float_list_is_rejected() {
    let registry = real_registry();
    let key = declared_keys_with_type(&registry, "float-list")
        .into_iter()
        .next()
        .expect("registry declares at least one float-list key");
    let authored = HashMap::from([(
        key.clone(),
        ConfigValue::List(vec![
            ConfigValue::Float(1.0),
            ConfigValue::Float(f64::INFINITY),
        ]),
    )]);
    let mut ingestor = ConfigIngestor::new(&registry);

    let error = ingestor
        .ingest_flat(&authored)
        .expect_err("a non-finite native float element must reject the whole list");
    assert!(matches!(
        error,
        ConfigIngestionError::TypeMismatch {
            key: reported_key,
            expected,
            ..
        } if reported_key == key && expected == "float-list"
    ));
}

#[test]
fn declared_float_or_percent_accepts_percent_variant() {
    let registry = real_registry();
    let key = declared_keys_with_type(&registry, "float_or_percent")
        .into_iter()
        .next()
        .expect("registry declares at least one float_or_percent key");
    let authored = HashMap::from([(key.clone(), ConfigValue::Percent(75.0))]);
    let mut ingestor = ConfigIngestor::new(&registry);

    ingestor
        .ingest_flat(&authored)
        .expect("a percent is representable by a float_or_percent declaration");
    let outcome = ingestor.finish();

    assert_eq!(
        outcome
            .scoped
            .global()
            .and_then(|delta| delta.values.get(&key)),
        Some(&ConfigValue::FloatOrPercent {
            value: 75.0,
            is_percent: true,
        })
    );
}

#[test]
fn declared_percent_accepts_percent_authored_float_or_percent_variant() {
    let registry = real_registry();
    let key = declared_keys_with_type(&registry, "percent")
        .into_iter()
        .next()
        .expect("registry declares at least one percent key");
    let authored = HashMap::from([(
        key.clone(),
        ConfigValue::FloatOrPercent {
            value: 75.0,
            is_percent: true,
        },
    )]);
    let mut ingestor = ConfigIngestor::new(&registry);

    ingestor
        .ingest_flat(&authored)
        .expect("a percent-authored float_or_percent is representable by a percent declaration");
    let outcome = ingestor.finish();

    assert_eq!(
        outcome
            .scoped
            .global()
            .and_then(|delta| delta.values.get(&key)),
        Some(&ConfigValue::Percent(75.0))
    );
}

#[test]
fn declared_float_accepts_canonical_percent_spelling_string() {
    // Canonical scalar wire (canonical `ConfigOptionFloat::deserialize`): a
    // numeric string parses with its optional trailing `%` ignored, yielding
    // the percent magnitude — the same spelling `ConfigView::get_float`'s
    // string branch accepts at the readers. Regression: a modifier delta
    // authored `sparse_infill_density = "40%"` (a real OrcaSlicer export's
    // percent wire) was rejected as `expected float, authored "40%"` by the
    // packet-08 registry-routed modifier ingestion.
    let registry = real_registry();
    let key = declared_keys_with_type(&registry, "float")
        .into_iter()
        .next()
        .expect("registry declares at least one float key");
    let values = HashMap::from([(key.clone(), ConfigValue::String("40%".to_owned()))]);
    let mut ingestor = ConfigIngestor::new(&registry);

    ingestor
        .ingest_delta(
            ConfigScope::Modifier {
                object_id: "obj-a".to_owned(),
                modifier_id: "mod-a".to_owned(),
            },
            &values,
        )
        .expect("a percent-spelled numeric string is canonical float wire");
    let outcome = ingestor.finish();

    let delta = outcome
        .scoped
        .deltas
        .get(&ConfigScope::Modifier {
            object_id: "obj-a".to_owned(),
            modifier_id: "mod-a".to_owned(),
        })
        .expect("modifier delta retained");
    assert_eq!(delta.values.get(&key), Some(&ConfigValue::Float(40.0)));
}

#[test]
fn strict_scalar_list_shape_is_rejected() {
    let registry = real_registry();
    let key = declared_keys_with_type(&registry, "int")
        .into_iter()
        .next()
        .expect("registry declares at least one scalar int key");
    let authored = HashMap::from([(
        key.clone(),
        ConfigValue::List(vec![ConfigValue::String("1".to_owned())]),
    )]);
    let mut ingestor = ConfigIngestor::new(&registry);

    let error = ingestor
        .ingest_flat(&authored)
        .expect_err("strict ingestion must reject a list for a scalar declaration");
    assert!(matches!(
        error,
        ConfigIngestionError::TypeMismatch {
            key: reported_key,
            expected,
            ..
        } if reported_key == key && expected == "int"
    ));
}

#[test]
fn tolerant_scalar_list_shape_is_retained_and_warns() {
    let registry = real_registry();
    let key = declared_keys_with_type(&registry, "int")
        .into_iter()
        .next()
        .expect("registry declares at least one scalar int key");
    let authored_value = ConfigValue::List(vec![ConfigValue::String("1".to_owned())]);
    let authored = HashMap::from([(key.clone(), authored_value.clone())]);
    let mut ingestor = ConfigIngestor::tolerant(&registry);

    ingestor
        .ingest_flat(&authored)
        .expect("tolerant ingestion should retain a representable scalar list");
    let outcome = ingestor.finish();

    assert_eq!(
        outcome
            .scoped
            .global()
            .and_then(|delta| delta.values.get(&key)),
        Some(&ConfigValue::List(vec![ConfigValue::Int(1)]))
    );
    assert_eq!(
        outcome.warnings,
        vec![IngestionWarning::UntypedValue {
            key,
            declared_type: "int".to_owned(),
            authored: "[String(\"1\")]".to_owned(),
        }]
    );
}

#[test]
fn tolerant_malformed_scope_key_is_still_rejected() {
    let registry = real_registry();
    let authored = HashMap::from([(
        "object_config:obj-a".to_owned(),
        ConfigValue::String("1".to_owned()),
    )]);
    let mut ingestor = ConfigIngestor::tolerant(&registry);

    let error = ingestor
        .ingest_flat(&authored)
        .expect_err("tolerant ingestion must not soften malformed scope keys");
    assert!(matches!(
        error,
        ConfigIngestionError::MalformedScopeKey { wire_key, .. }
            if wire_key == "object_config:obj-a"
    ));
}

#[test]
fn tolerant_non_finite_declared_float_is_still_rejected() {
    let registry = real_registry();
    let key = declared_keys_with_type(&registry, "float")
        .into_iter()
        .next()
        .expect("registry declares at least one float key");
    let authored = HashMap::from([(key.clone(), ConfigValue::Float(f64::NAN))]);
    let mut ingestor = ConfigIngestor::tolerant(&registry);

    let error = ingestor
        .ingest_flat(&authored)
        .expect_err("tolerant ingestion must not soften non-finite declared floats");
    assert!(matches!(
        error,
        ConfigIngestionError::TypeMismatch {
            key: reported_key,
            expected,
            ..
        } if reported_key == key && expected == "float"
    ));
}

/// Residual-red session: canonical bed-point wire (canonical
/// `ConfigOptionPoints::deserialize`) — "XxY" point strings decode into the
/// flat bed-pair model at strict ingestion, with no warning. Flat floats stay
/// pinned verbatim by `resolved_config_view_tdd`'s AC-12.
#[test]
fn float_list_bed_point_wire_deserializes_to_flat_pairs() {
    let registry = real_registry();
    let authored = HashMap::from([(
        "printable_area".to_owned(),
        ConfigValue::List(vec![
            ConfigValue::String("0x0".to_owned()),
            ConfigValue::String("250x0".to_owned()),
            ConfigValue::String("250x210".to_owned()),
            ConfigValue::String("0x210".to_owned()),
        ]),
    )]);

    let mut strict = ConfigIngestor::new(&registry);
    strict
        .ingest_flat(&authored)
        .expect("canonical ConfigOptionPoints wire must decode as flat pairs");
    let outcome = strict.finish();

    assert_eq!(
        outcome
            .scoped
            .global()
            .and_then(|delta| delta.values.get("printable_area")),
        Some(&ConfigValue::List(vec![
            ConfigValue::Float(0.0),
            ConfigValue::Float(0.0),
            ConfigValue::Float(250.0),
            ConfigValue::Float(0.0),
            ConfigValue::Float(250.0),
            ConfigValue::Float(210.0),
            ConfigValue::Float(0.0),
            ConfigValue::Float(210.0),
        ])),
        "XxY point strings must deserialize into the flat bed-pair model"
    );
    assert!(
        outcome.warnings.is_empty(),
        "clean coercion raises no warnings, got {:?}",
        outcome.warnings
    );
}

#[test]
fn tolerant_tri_state_string_is_retained_and_strict_mode_rejects_it() {
    let registry = real_registry();
    let key = declared_keys_with_type(&registry, "bool")
        .into_iter()
        .next()
        .expect("registry declares at least one bool key");
    let authored = HashMap::from([(key.clone(), ConfigValue::String("disabled".to_owned()))]);

    let mut strict = ConfigIngestor::new(&registry);
    let error = strict
        .ingest_flat(&authored)
        .expect_err("strict ingestion must reject the tri-state spelling");
    assert!(matches!(
        error,
        ConfigIngestionError::TypeMismatch {
            key: reported_key,
            expected,
            authored,
        } if reported_key == key && expected == "bool" && authored == "disabled"
    ));

    let mut tolerant = ConfigIngestor::tolerant(&registry);
    tolerant
        .ingest_flat(&authored)
        .expect("tolerant ingestion should retain the tri-state spelling");
    let outcome = tolerant.finish();

    assert_eq!(
        outcome
            .scoped
            .global()
            .and_then(|delta| delta.values.get(&key)),
        Some(&ConfigValue::String("disabled".to_owned()))
    );
    assert_eq!(
        outcome.warnings,
        vec![IngestionWarning::UntypedValue {
            key,
            declared_type: "bool".to_owned(),
            authored: "disabled".to_owned(),
        }]
    );
}
