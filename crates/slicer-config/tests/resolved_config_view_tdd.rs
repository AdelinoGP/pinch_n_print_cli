//! AC-2, AC-4, AC-6, AC-8, AC-12, and AC-N1 for the resolved-config view
//! packet.
//!
//! The expected key sets below are derived from the assembled registry and from
//! `ResolvedConfig::typed_field_keys()` by iteration, never from a
//! hand-maintained literal key list, so a registry change fails these tests
//! loudly instead of being masked by a copied roster.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use slicer_config::{
    assemble_registry, resolve_scope_stack, ConfigIngestor, ConfigSchemaRegistry, ConfigScope,
    ExpansionContext, HostChannels, IngestionWarning, ModuleDeclaration, ResolutionError,
    ResolutionTarget, ScopeDelta, ScopedConfig,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::resolved_config::ResolvedConfig;
use slicer_ir::{ConfigResolutionError, ConfigValue};

// ── Fixtures ───────────────────────────────────────────────────────────────

fn field(field_type: &str) -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: field_type.to_owned(),
        ..ConfigFieldEntry::default()
    }
}

fn float_field(default: &str, min: Option<f64>, max: Option<f64>) -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: "float".to_owned(),
        default: Some(default.to_owned()),
        min,
        max,
        ..ConfigFieldEntry::default()
    }
}

/// A selector row must deny every per-region scope, or registry assembly
/// rejects it.
fn selector_field(default: &str) -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: "string".to_owned(),
        default: Some(default.to_owned()),
        selector: true,
        denied_scopes: [
            "object",
            "layer_range",
            "modifier",
            "paint_semantic",
            "tool",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        ..ConfigFieldEntry::default()
    }
}

/// A registry exercising all four seed-set membership rules:
///
/// - `seeded_float`: exact, defaulted, not a selector, not a typed field → seeded;
/// - `seeded_percent`: exact, defaulted, `float_or_percent` with a base → seeded
///   and covered by Phase-B expansion;
/// - `seeded_no_default`: exact but defaultless → not seeded;
/// - `seeded_selector`: exact, defaulted, selector → not seeded;
/// - `prefix:*`: a wildcard with a default → not seeded.
///
/// `line_width` and `nozzle_diameter` are declared so the typed-skip rule and
/// the percent base have real registry fixtures.
fn fixture_registry() -> ConfigSchemaRegistry {
    assemble_registry(
        &[ModuleDeclaration {
            module_id: "dev.pinch.test.resolved-config-view".to_owned(),
            schema: ConfigSchema {
                entries: BTreeMap::from([
                    (
                        "seeded_float".to_owned(),
                        float_field("1.5", Some(0.0), Some(10.0)),
                    ),
                    (
                        "seeded_percent".to_owned(),
                        ConfigFieldEntry {
                            field_type: "float_or_percent".to_owned(),
                            default: Some("50%".to_owned()),
                            base_key: Some("nozzle_diameter".to_owned()),
                            min: Some(0.0),
                            ..ConfigFieldEntry::default()
                        },
                    ),
                    ("seeded_no_default".to_owned(), field("string")),
                    ("seeded_selector".to_owned(), selector_field("classic")),
                    ("prefix:*".to_owned(), float_field("2.0", None, None)),
                    (
                        "nozzle_diameter".to_owned(),
                        float_field("0.4", Some(0.0), Some(2.0)),
                    ),
                    ("line_width".to_owned(), field("float")),
                ]),
            },
            ..ModuleDeclaration::default()
        }],
        &HostChannels::from_live(),
    )
    .expect("fixture registry must assemble")
    .registry
}

fn delta(entries: impl IntoIterator<Item = (&'static str, ConfigValue)>) -> ScopeDelta {
    ScopeDelta {
        values: entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    }
}

fn scoped(deltas: impl IntoIterator<Item = (ConfigScope, ScopeDelta)>) -> ScopedConfig {
    ScopedConfig {
        deltas: deltas.into_iter().collect(),
    }
}

fn expansion() -> ExpansionContext {
    ExpansionContext {
        nozzle_diameter_mm: 0.4,
        ..ExpansionContext::default()
    }
}

// ── AC-2 ───────────────────────────────────────────────────────────────────

/// AC-2: an extension key declared as `float` with bounds accepts authored
/// values in global/object/paint/tool scopes; the effective value is the
/// registry-typed `ConfigValue::Float`; an explicit value equal to the default
/// is still a real override over a non-default lower scope; a wrong variant is
/// rejected as `TypeMismatch`.
#[test]
fn extensions_are_typed_bounded_and_presence_preserving() {
    let registry = fixture_registry();

    let resolved = resolve_scope_stack(
        &registry,
        &scoped([
            (
                ConfigScope::Global,
                delta([("seeded_float", ConfigValue::Float(2.5))]),
            ),
            (
                ConfigScope::Object("obj-a".to_owned()),
                delta([("seeded_float", ConfigValue::Float(4.0))]),
            ),
            (
                ConfigScope::PaintSemantic("material".to_owned()),
                delta([("seeded_float", ConfigValue::Float(3.0))]),
            ),
            (
                ConfigScope::Tool(1),
                delta([("seeded_float", ConfigValue::Float(6.0))]),
            ),
        ]),
        &ResolutionTarget {
            object_id: "obj-a".to_owned(),
            paint_semantics: vec!["material".to_owned()],
            tool_index: Some(1),
            ..ResolutionTarget::default()
        },
        &expansion(),
    )
    .expect("in-bounds typed extensions must resolve");

    assert_eq!(
        resolved.extensions.get("seeded_float"),
        Some(&ConfigValue::Float(6.0)),
        "the selected tool is the highest-precedence scope"
    );

    // Presence preservation: the object scope authors the registry default
    // (1.5) over a non-default global scope (2.0). Value comparison must not
    // treat it as "no override".
    let authored_default = resolve_scope_stack(
        &registry,
        &scoped([
            (
                ConfigScope::Global,
                delta([("seeded_float", ConfigValue::Float(2.0))]),
            ),
            (
                ConfigScope::Object("obj-a".to_owned()),
                delta([("seeded_float", ConfigValue::Float(1.5))]),
            ),
        ]),
        &ResolutionTarget {
            object_id: "obj-a".to_owned(),
            ..ResolutionTarget::default()
        },
        &expansion(),
    )
    .expect("an authored default must resolve");
    assert_eq!(
        authored_default.extensions.get("seeded_float"),
        Some(&ConfigValue::Float(1.5)),
        "presence in the higher scope, not value comparison, decides"
    );

    // A wrong variant for the declared type is an atomic rejection.
    let error = resolve_scope_stack(
        &registry,
        &scoped([(
            ConfigScope::Global,
            delta([("seeded_float", ConfigValue::String("nope".to_owned()))]),
        )]),
        &ResolutionTarget::default(),
        &expansion(),
    )
    .expect_err("a wrong variant must reject");
    match error {
        ResolutionError::Application(ConfigResolutionError::TypeMismatch {
            key,
            expected,
            actual,
        }) => {
            assert_eq!(key, "seeded_float");
            assert_eq!(expected, "Float");
            assert_eq!(actual, "String");
        }
        other => panic!("expected Application(TypeMismatch), got {other:?}"),
    }
}

// ── AC-4 ───────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `modules/core-modules/*/*.toml` schema entry, parsed through the same
/// TOML shape the scheduler's manifest parser consumes. Duplicated here because
/// `slicer-config` cannot depend on `slicer-scheduler` (the dependency runs the
/// other way), and a projection that dropped a field would silently diverge
/// from production assembly.
fn real_manifest_entries() -> Vec<(String, BTreeMap<String, ConfigFieldEntry>)> {
    let modules_dir = workspace_root().join("modules/core-modules");
    let mut paths = std::fs::read_dir(&modules_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", modules_dir.display()))
        .map(|entry| entry.expect("module directory entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "real core-module manifest scan found no module directories"
    );

    let mut parsed = Vec::new();
    for path in paths {
        let stem = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("module directory name must be UTF-8");
        let manifest_path = path.join(format!("{stem}.toml"));
        let text = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", manifest_path.display()));
        let document: toml::Value = toml::from_str(&text)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", manifest_path.display()));
        let module_id = document
            .get("module")
            .and_then(toml::Value::as_table)
            .and_then(|module| module.get("id"))
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("manifest {} has no module.id", manifest_path.display()))
            .to_owned();
        let schema = document
            .get("config")
            .and_then(toml::Value::as_table)
            .and_then(|config| config.get("schema"))
            .and_then(toml::Value::as_table)
            .unwrap_or_else(|| {
                panic!(
                    "manifest {} has no [config.schema]",
                    manifest_path.display()
                )
            });
        let entries = schema
            .iter()
            .map(|(key, value)| (key.clone(), parse_manifest_field(key, value)))
            .collect();
        parsed.push((module_id, entries));
    }
    parsed
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
    let field_type = table
        .get("type")
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("config.schema.{key}.type is required"));
    let config_block = table
        .get("config_block")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    ConfigFieldEntry {
        field_type: field_type.to_owned(),
        default: table.get("default").map(manifest_default_to_wire),
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
        omit_from_config_block: !config_block,
        ..ConfigFieldEntry::default()
    }
}

fn manifest_default_to_wire(value: &toml::Value) -> String {
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

/// The real registry: live host channels plus every core-module manifest.
fn real_registry() -> ConfigSchemaRegistry {
    let modules = real_manifest_entries()
        .into_iter()
        .map(|(module_id, entries)| ModuleDeclaration {
            module_id,
            schema: ConfigSchema { entries },
            ..ModuleDeclaration::default()
        })
        .collect::<Vec<_>>();

    assemble_registry(&modules, &HostChannels::from_live())
        .unwrap_or_else(|error| panic!("real registry must assemble: {error:?}"))
        .registry
}

/// AC-4: `config_block_map` projects the effective config onto the registry.
///
/// The expected key set is computed by registry iteration: registry entries
/// without `omit_from_config_block` plus typed fields with no registry entry,
/// restricted to keys that have an effective value. Retained unknown extension
/// keys are never emitted, and the four explicit exclusions stay absent.
#[test]
fn config_block_map_is_registry_driven() {
    let registry = real_registry();

    // Non-default global config: the mmu keys are set, `thumbnail_path` is a
    // runtime-row extension, and one undeclared key is retained.
    let mut resolved = ResolvedConfig {
        mmu_segmented_region_max_width: 1.5,
        mmu_segmented_region_interlocking_depth: 0.5,
        ..ResolvedConfig::default()
    };
    resolved.extensions.insert(
        "thumbnail_path".to_owned(),
        ConfigValue::String("/tmp/plate.png".to_owned()),
    );
    resolved
        .extensions
        .insert("undeclared_extension".to_owned(), ConfigValue::Float(9.0));

    let map = registry.config_block_map(&resolved);
    let effective = resolved.to_config_map();
    let typed = ResolvedConfig::typed_field_keys();

    let mut expected = BTreeMap::new();
    for key in registry.keys() {
        let entry = registry.entry(key).expect("key came from the registry");
        if entry.omit_from_config_block {
            continue;
        }
        if let Some(value) = effective.get(key) {
            expected.insert(key.to_owned(), value.clone());
        }
    }
    for key in typed {
        if registry.entry(key).is_some() {
            continue;
        }
        if let Some(value) = effective.get(*key) {
            expected.insert((*key).to_owned(), value.clone());
        }
    }
    assert_eq!(
        map, expected,
        "config_block_map must equal the registry-driven projection"
    );

    // Concrete, independently stated properties so a bug in the generic
    // projection above cannot pass both checks.
    for key in [
        "mmu_segmented_region_max_width",
        "mmu_segmented_region_interlocking_depth",
        "mmu_segmented_region_interlocking_beam",
        "thumbnail_path",
    ] {
        assert!(
            effective.contains_key(key),
            "{key} must carry an effective value the projection could have emitted"
        );
        assert!(
            registry
                .entry(key)
                .is_some_and(|entry| entry.omit_from_config_block),
            "{key} must be a registry entry with omit_from_config_block"
        );
        assert!(
            !map.contains_key(key),
            "{key} must be absent through config_block = false"
        );
    }

    let omitted = registry
        .keys()
        .filter(|key| {
            registry
                .entry(key)
                .is_some_and(|entry| entry.omit_from_config_block)
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        omitted,
        [
            "mmu_segmented_region_interlocking_beam",
            "mmu_segmented_region_interlocking_depth",
            "mmu_segmented_region_max_width",
            "thumbnail_path",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>(),
        "exactly the four declared keys carry config_block = false"
    );

    assert!(
        effective.contains_key("undeclared_extension"),
        "the retained unknown key must be present in the effective map"
    );
    assert!(
        !map.contains_key("undeclared_extension"),
        "a retained unknown extension key must never be emitted"
    );

    assert!(
        registry.entry("infill_type").is_none(),
        "fixture assumption: infill_type has no registry entry"
    );
    assert_eq!(
        map.get("infill_type"),
        effective.get("infill_type"),
        "a typed field with no registry entry must be projected at its effective value"
    );
    assert!(
        map.contains_key("infill_type"),
        "infill_type must be emitted through typed_field_keys()"
    );
    assert_eq!(
        map.get("support_type"),
        effective.get("support_type"),
        "a typed value with a registry entry must be projected from to_config_map()"
    );
}

// ── AC-8 ───────────────────────────────────────────────────────────────────

/// AC-8: any resolution of the fixture registry carries every exact registry
/// key that has a default. Percent defaults pass through Phase B; wildcards,
/// defaultless entries, selectors, and typed fields seed nothing.
#[test]
fn every_resolved_config_carries_every_exact_registry_key() {
    let registry = fixture_registry();
    let resolved = resolve_scope_stack(
        &registry,
        &ScopedConfig::default(),
        &ResolutionTarget::default(),
        &expansion(),
    )
    .expect("an empty scope stack must still resolve through seeding");

    let map = resolved.to_config_map();
    let typed = ResolvedConfig::typed_field_keys();

    let mut seeded = 0;
    for key in registry.keys() {
        let entry = registry.entry(key).expect("key came from the registry");
        if key.ends_with(":*") {
            assert!(
                !resolved.extensions.contains_key(key),
                "wildcard {key} must not seed a concrete extension"
            );
            continue;
        }
        if entry.selector {
            assert!(
                !resolved.extensions.contains_key(key),
                "selector {key} must not be seeded"
            );
            continue;
        }
        if typed.contains(&key) {
            continue;
        }
        let Some(default) = entry.default.as_deref() else {
            assert!(
                !map.contains_key(key),
                "{key} has no default and must not be seeded"
            );
            continue;
        };
        seeded += 1;
        assert!(
            map.contains_key(key),
            "{key} has registry default {default:?} and must appear in the effective map"
        );
    }
    assert!(seeded > 0, "fixture must exercise the seed set");

    // The seeded percent default passed through Phase B: 50% of the 0.4 nozzle.
    assert_eq!(
        map.get("seeded_percent"),
        Some(&ConfigValue::FloatOrPercent {
            value: 0.2,
            is_percent: false,
        }),
        "a seeded percent default with a base must be expanded, not left relative"
    );

    // Seeded values are registry-typed.
    assert_eq!(
        resolved.extensions.get("seeded_float"),
        Some(&ConfigValue::Float(1.5))
    );
    assert_eq!(
        resolved.extensions.get("nozzle_diameter"),
        Some(&ConfigValue::Float(0.4))
    );

    // Typed fields carry their own defaults through `ResolvedConfig::default()`;
    // `line_width` is never shadowed by a seeded extension and its auto-zero
    // expands through the seeded nozzle base.
    assert!(
        !resolved.extensions.contains_key("line_width"),
        "a typed field must not be seeded into extensions"
    );
    assert_eq!(
        map.get("line_width"),
        Some(&ConfigValue::Float(f64::from(0.45_f32))),
        "the typed line_width auto-zero expands against the seeded nozzle"
    );

    // Exempt documented shadow: the typed support_line_width auto-zero expands
    // and `expand_automatic_values` mirrors the normalized literal into
    // extensions, shadowing the typed field in `to_config_map`.
    assert_eq!(
        resolved.extensions.get("support_line_width"),
        Some(&ConfigValue::Float(0.4)),
        "the documented support_line_width expansion shadow must be exercised"
    );

    // Apart from that shadow, no extension may be a typed field or a selector.
    for key in resolved.extensions.keys() {
        if key == "support_line_width" {
            continue;
        }
        assert!(
            !typed.contains(&key.as_str()),
            "extension {key} shadows a declare_resolved_config! field"
        );
        assert!(
            !registry.entry(key).is_some_and(|entry| entry.selector),
            "selector {key} must not be seeded into extensions"
        );
    }

    // The fixture assumptions that make the wildcard rule observable.
    assert!(
        registry
            .entry("prefix:*")
            .is_some_and(|entry| entry.default.is_some()),
        "fixture assumption: prefix:* carries a default"
    );
    assert!(
        !map.keys().any(|key| key.ends_with(":*")),
        "no wildcard key may reach the effective map"
    );
    assert!(
        !resolved.extensions.contains_key("seeded_no_default"),
        "a defaultless entry must not be seeded"
    );
}

// ── AC-N1 ──────────────────────────────────────────────────────────────────

/// AC-N1: an extension value outside its declared `min`/`max` rejects the whole
/// resolution through `ResolutionError::Application(OutOfRange)` naming the key
/// and authored value; no `ResolvedConfig` is yielded.
#[test]
fn out_of_bounds_extension_is_rejected_atomically() {
    let registry = fixture_registry();

    let error = resolve_scope_stack(
        &registry,
        &scoped([
            (
                ConfigScope::Global,
                delta([("seeded_float", ConfigValue::Float(4.0))]),
            ),
            (
                ConfigScope::Object("obj-a".to_owned()),
                delta([("seeded_float", ConfigValue::Float(11.0))]),
            ),
        ]),
        &ResolutionTarget {
            object_id: "obj-a".to_owned(),
            ..ResolutionTarget::default()
        },
        &expansion(),
    )
    .expect_err("an out-of-range extension must reject the whole resolution");

    match error {
        ResolutionError::Application(ConfigResolutionError::OutOfRange {
            key,
            value,
            min,
            max,
            index,
        }) => {
            assert_eq!(key, "seeded_float");
            assert_eq!(value, 11.0);
            assert_eq!(min, Some(0.0));
            assert_eq!(max, Some(10.0));
            assert_eq!(index, None);
        }
        other => panic!("expected Application(OutOfRange), got {other:?}"),
    }
}

// ── AC-6 ───────────────────────────────────────────────────────────────────

/// AC-6: against a registry that declares `skirt_loops` (the live
/// skirt-brim manifest), an authored flat `skrit_loops = "1"` warns exactly
/// once with the near-miss suggestion and is dropped from every scope delta
/// and from the resolved config. The declared near-miss key is authored in the
/// same flat ingestion so the drop is observed against a live delta rather
/// than over an empty one.
#[test]
fn unknown_key_warns_once_and_is_dropped_from_deltas_and_resolution() {
    let registry = real_registry();
    assert!(
        registry.entry("skirt_loops").is_some(),
        "fixture assumption: the live registry declares skirt_loops \
         (modules/core-modules/skirt-brim/skirt-brim.toml)"
    );

    let mut authored = HashMap::new();
    authored.insert(
        "skrit_loops".to_owned(),
        ConfigValue::String("1".to_owned()),
    );
    authored.insert(
        "skirt_loops".to_owned(),
        ConfigValue::String("2".to_owned()),
    );

    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_flat(&authored)
        .expect("flat authored config must decode");
    let outcome = ingestor.finish();

    // Exactly one warning: the misspelling only.
    assert_eq!(
        outcome.warnings.len(),
        1,
        "exactly one UnrecognizedKey is expected, got {:?}",
        outcome.warnings
    );
    match &outcome.warnings[0] {
        IngestionWarning::UnrecognizedKey {
            wire_key,
            key,
            suggestion,
        } => {
            assert_eq!(wire_key, "skrit_loops");
            assert_eq!(key, "skrit_loops");
            assert_eq!(suggestion.as_deref(), Some("skirt_loops"));
        }
        other => panic!("expected UnrecognizedKey for skrit_loops, got {other:?}"),
    }

    // The dropped key appears in no scope delta; the declared near-miss retains
    // its coerced value in the global delta.
    for (scope, delta) in outcome.scoped.iter() {
        assert!(
            !delta.values.contains_key("skrit_loops"),
            "scope {scope:?} must not carry the dropped key"
        );
    }
    let global = outcome
        .scoped
        .global()
        .expect("the declared near-miss key must land in the global delta");
    assert_eq!(
        global.values.get("skirt_loops"),
        Some(&ConfigValue::Int(2)),
        "the retained declared key keeps its coerced value"
    );

    // Resolution of the ingestion outcome yields no trace of the dropped key.
    let resolved = resolve_scope_stack(
        &registry,
        &outcome.scoped,
        &ResolutionTarget::default(),
        &expansion(),
    )
    .expect("resolution with a dropped unknown key must succeed");
    assert!(
        !resolved.extensions.contains_key("skrit_loops"),
        "the dropped key must not reach extensions"
    );
    assert!(
        !resolved.to_config_map().contains_key("skrit_loops"),
        "the dropped key must not reach the effective config map"
    );
    assert_eq!(
        resolved.extensions.get("skirt_loops"),
        Some(&ConfigValue::Int(2)),
        "the declared near-miss key stays part of the effective config"
    );
}

// ── AC-12 ──────────────────────────────────────────────────────────────────

/// AC-12: every registered host-consumed key survives the warn-to-drop flip —
/// authored values ingest with zero `UnrecognizedKey` warnings, land in the
/// global scope delta, and reach the resolved `to_config_map()` at their
/// authored value. The roster is the packet-06 Step-2a registered set
/// (`HOST_RUNTIME_KEYS`, defaults all `None`); each key is checked to be a
/// live registry entry before ingestion, so a de-registration fails the test
/// instead of silently skipping the key.
#[test]
fn host_consumed_keys_survive_the_drop() {
    let registry = real_registry();
    let authored: HashMap<String, ConfigValue> = [
        (
            "gcode_flavor".to_owned(),
            ConfigValue::String("klipper".to_owned()),
        ),
        (
            "printer_model".to_owned(),
            ConfigValue::String("fakesaurus".to_owned()),
        ),
        (
            "filament_colour".to_owned(),
            ConfigValue::List(vec![ConfigValue::String("red".to_owned())]),
        ),
        (
            "extruder_colour".to_owned(),
            ConfigValue::List(vec![ConfigValue::String("navy".to_owned())]),
        ),
        (
            "filament_cost".to_owned(),
            ConfigValue::List(vec![ConfigValue::String("31.5".to_owned())]),
        ),
        (
            "printable_area".to_owned(),
            ConfigValue::List(vec![
                ConfigValue::Float(250.0),
                ConfigValue::Float(0.0),
                ConfigValue::Float(250.0),
                ConfigValue::Float(210.0),
            ]),
        ),
        (
            "support_type".to_owned(),
            ConfigValue::String("tree(auto)".to_owned()),
        ),
        (
            "support_family".to_owned(),
            ConfigValue::String("tree".to_owned()),
        ),
        (
            "thumbnails".to_owned(),
            ConfigValue::String("300x300".to_owned()),
        ),
        (
            "machine_max_acceleration_retracting".to_owned(),
            ConfigValue::List(vec![ConfigValue::Float(1000.0)]),
        ),
        ("extruder".to_owned(), ConfigValue::Int(0)),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        authored.len(),
        11,
        "the AC-12 roster must cover all eleven registered host-consumed keys"
    );
    for key in authored.keys() {
        assert!(
            registry.entry(key).is_some(),
            "fixture assumption: {key} is a registered host key"
        );
    }

    let mut ingestor = ConfigIngestor::new(&registry);
    ingestor
        .ingest_flat(&authored)
        .expect("registered host-consumed values must decode");
    let outcome = ingestor.finish();

    // Every registered host key ingests without an UnrecognizedKey.
    assert!(
        outcome.warnings.is_empty(),
        "no warning may be raised for registered host keys, got {:?}",
        outcome.warnings
    );

    // All eleven values land in the global scope delta and in the resolved
    // `to_config_map()`, at their authored value.
    let global = outcome
        .scoped
        .global()
        .expect("host-consumed keys are global-scope");
    let resolved = resolve_scope_stack(
        &registry,
        &outcome.scoped,
        &ResolutionTarget::default(),
        &expansion(),
    )
    .expect("resolution over the authored host keys must succeed");
    let effective = resolved.to_config_map();
    for (key, value) in &authored {
        assert_eq!(
            global.values.get(key),
            Some(value),
            "{key} must survive ingestion into the global scope delta"
        );
        assert_eq!(
            effective.get(key),
            Some(value),
            "{key} must survive resolution into to_config_map()"
        );
    }
}
