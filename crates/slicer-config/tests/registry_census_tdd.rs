//! Independent, channel-derived coverage for the configuration registry.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use slicer_config::{assemble_registry, HostChannels, ModuleDeclaration};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::resolved_config::{HostKeyMeta, ResolvedConfig, HOST_RUNTIME_KEYS};

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
    let manifests = manifest_paths()
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
        .collect::<Vec<_>>();
    assert!(
        !manifests.is_empty(),
        "real core-module manifest parse produced no module schemas"
    );
    manifests
}

/// Read the raw schema-table keys separately from the module declarations.
/// The table keys are the independent manifest authority for this census;
/// registry assembly is only responsible for reconciling the resulting keys.
fn real_manifest_pairs() -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for path in manifest_paths() {
        let document = read_manifest(&path);
        let module_id = manifest_module_id(&document, &path).to_owned();
        pairs.extend(
            manifest_schema(&document, &path)
                .keys()
                .cloned()
                .map(|key| (module_id.clone(), key)),
        );
    }
    assert!(
        !pairs.is_empty(),
        "real core-module manifest census produced no schema entries"
    );
    pairs
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

fn projected_module_declarations(manifests: &[ParsedManifest]) -> Vec<ModuleDeclaration> {
    let mut field_types = host_field_types();
    for manifest in manifests {
        for (key, field_type) in &manifest.field_types {
            field_types
                .entry(key.clone())
                .or_insert_with(|| field_type.clone());
        }
    }

    manifests
        .iter()
        .map(|manifest| ModuleDeclaration {
            module_id: manifest.module_id.clone(),
            schema: ConfigSchema {
                entries: manifest
                    .field_types
                    .keys()
                    .map(|key| {
                        let field_type = field_types
                            .get(key)
                            .unwrap_or_else(|| panic!("no census type for key {key}"));
                        (
                            key.clone(),
                            ConfigFieldEntry {
                                field_type: field_type.clone(),
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
            },
            claim_exclusive_group: None,
        })
        .collect()
}

fn channel_union(manifest_pairs: &[(String, String)]) -> BTreeSet<String> {
    let host_keys = ResolvedConfig::host_config_keys();
    assert!(!host_keys.is_empty(), "ResolvedConfig channel has no keys");
    assert!(
        !slicer_ir::feedrate::SPEED_KEYS.is_empty(),
        "feedrate channel has no keys"
    );
    assert!(
        !HOST_RUNTIME_KEYS.is_empty(),
        "host runtime channel has no keys"
    );

    let mut keys = host_keys
        .into_iter()
        .map(|row| row.key.to_owned())
        .collect::<BTreeSet<_>>();
    keys.extend(
        slicer_ir::feedrate::SPEED_KEYS
            .iter()
            .map(|(key, _)| (*key).to_owned()),
    );
    keys.extend(HOST_RUNTIME_KEYS.iter().map(|row| row.key.to_owned()));
    keys.extend(manifest_pairs.iter().map(|(_, key)| key.clone()));
    keys
}

fn assembled_real_registry() -> slicer_config::ConfigSchemaRegistry {
    let manifests = parse_real_manifests();
    let modules = projected_module_declarations(&manifests);
    assemble_registry(&modules, &HostChannels::from_live())
        .unwrap_or_else(|error| panic!("real channel census failed to assemble: {error:?}"))
        .registry
}

#[test]
fn host_runtime_rows_are_exact() {
    assert_eq!(HOST_RUNTIME_KEYS.len(), 14);

    let relative_e_distances = &HOST_RUNTIME_KEYS[0];
    assert_eq!(relative_e_distances.key, "use_relative_e_distances");
    assert_eq!(relative_e_distances.field_type, "bool");
    assert_eq!(relative_e_distances.scope, "printer");
    assert_eq!(relative_e_distances.default, Some("true"));
    assert_eq!(relative_e_distances.meta, HostKeyMeta::NONE);
    assert!(!relative_e_distances.selector);
    assert!(relative_e_distances.denied_scopes.is_empty());

    let thumbnail_path = &HOST_RUNTIME_KEYS[1];
    assert_eq!(thumbnail_path.key, "thumbnail_path");
    assert_eq!(thumbnail_path.field_type, "string");
    assert_eq!(thumbnail_path.scope, "printer");
    assert_eq!(thumbnail_path.default, Some(""));
    assert!(!thumbnail_path.selector);
    assert!(thumbnail_path.denied_scopes.is_empty());
    assert_eq!(thumbnail_path.meta.display, Some("Thumbnail path"));
    assert_eq!(
        thumbnail_path.meta.description,
        Some("File path the slicer writes its thumbnail plate into; empty disables thumbnails.")
    );
    assert_eq!(thumbnail_path.meta.group, Some("Output"));
    assert_eq!(thumbnail_path.meta.unit, None);
    assert_eq!(thumbnail_path.meta.min, None);
    assert_eq!(thumbnail_path.meta.max, None);
    assert!(thumbnail_path.meta.values.is_empty());
    assert_eq!(thumbnail_path.meta.wire_type, None);
    assert!(!thumbnail_path.meta.advanced);

    let wall_generator = &HOST_RUNTIME_KEYS[2];
    assert_eq!(wall_generator.key, "wall_generator");
    assert_eq!(wall_generator.field_type, "string");
    assert_eq!(wall_generator.scope, "print");
    assert_eq!(wall_generator.default, Some("classic"));
    assert_eq!(wall_generator.meta, HostKeyMeta::NONE);
    assert!(wall_generator.selector);
    assert_eq!(
        wall_generator.denied_scopes,
        [
            "object",
            "layer_range",
            "modifier",
            "paint_semantic",
            "tool"
        ]
        .as_slice()
    );
}

/// Packet 06 Step 2a registered eleven host-consumed keys after the three
/// pre-existing rows. Each carries `default: None` by design (synthesized,
/// defaulted at the consuming site, or seeded elsewhere), so this pin derives
/// its expectations from those authored rows rather than restating them as
/// captured output: the ordered key list is asserted first, then all eleven
/// rows are checked as typed declarations for the key that list names.
#[test]
fn host_runtime_step_2a_rows_are_registered_with_option_defaults() {
    let registered = HOST_RUNTIME_KEYS
        .iter()
        .map(|row| row.key)
        .collect::<Vec<_>>();
    assert_eq!(
        registered,
        [
            "use_relative_e_distances",
            "thumbnail_path",
            "wall_generator",
            "gcode_flavor",
            "printer_model",
            "filament_colour",
            "extruder_colour",
            "filament_cost",
            "printable_area",
            "support_type",
            "support_family",
            "thumbnails",
            "machine_max_acceleration_retracting",
            "extruder",
        ]
    );

    let expected = [
        ("gcode_flavor", "string", "printer"),
        ("printer_model", "string", "printer"),
        ("filament_colour", "string-list", "filament"),
        ("extruder_colour", "string-list", "printer"),
        ("filament_cost", "string-list", "filament"),
        ("printable_area", "float-list", "printer"),
        ("support_type", "string", "print"),
        ("support_family", "string", "print"),
        ("thumbnails", "string", "print"),
        (
            "machine_max_acceleration_retracting",
            "float-list",
            "printer",
        ),
        ("extruder", "int", "print"),
    ];
    for (offset, (key, field_type, scope)) in expected.iter().enumerate() {
        let row = &HOST_RUNTIME_KEYS[3 + offset];
        assert_eq!(row.key, *key);
        assert_eq!(row.field_type, *field_type, "runtime {key} type");
        assert_eq!(row.scope, *scope, "runtime {key} scope");
        assert_eq!(row.default, None, "runtime {key} must stay unseeded");
        assert_eq!(row.meta, HostKeyMeta::NONE, "runtime {key} meta");
        assert!(!row.selector, "runtime {key} is not a selector");
        assert!(
            row.denied_scopes.is_empty(),
            "runtime {key} has no denied scopes"
        );
    }
}

#[test]
fn registry_covers_every_declared_key_from_every_channel() {
    let registry = assembled_real_registry();
    assert!(
        !registry.is_empty(),
        "real channel census assembled no keys"
    );

    let host_keys = ResolvedConfig::host_config_keys();
    assert!(!host_keys.is_empty(), "ResolvedConfig channel has no keys");
    for row in host_keys {
        assert!(
            registry.entry(row.key).is_some(),
            "registry omitted ResolvedConfig key {}",
            row.key
        );
    }
    assert!(
        !slicer_ir::feedrate::SPEED_KEYS.is_empty(),
        "feedrate channel has no keys"
    );
    for &(key, _) in slicer_ir::feedrate::SPEED_KEYS {
        assert!(
            registry.entry(key).is_some(),
            "registry omitted feedrate key {key}"
        );
    }
    assert!(
        !HOST_RUNTIME_KEYS.is_empty(),
        "host runtime channel has no keys"
    );
    for row in HOST_RUNTIME_KEYS {
        assert!(
            registry.entry(row.key).is_some(),
            "registry omitted runtime key {}",
            row.key
        );
    }
    for (module_id, key) in real_manifest_pairs() {
        assert!(
            registry.entry(&key).is_some(),
            "registry omitted manifest key {key} declared by {module_id}"
        );
    }
}

#[test]
fn registry_keys_equal_channel_union() {
    let manifest_pairs = real_manifest_pairs();
    let expected = channel_union(&manifest_pairs);
    assert!(!expected.is_empty(), "channel union has no declared keys");

    let actual = assembled_real_registry()
        .keys()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn real_toml_manifest_census_rederives_expected_scope() {
    let pairs = real_manifest_pairs();
    let mut owners = BTreeMap::<String, BTreeSet<String>>::new();
    for (module_id, key) in pairs.iter().cloned() {
        owners.entry(key).or_default().insert(module_id);
    }

    let entry_count = pairs.len();
    let distinct_key_count = owners.len();
    let multi_declared_count = owners.values().filter(|owners| owners.len() > 1).count();
    let wildcard_keys = owners
        .keys()
        .filter(|key| key.ends_with(":*"))
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(entry_count, 270);
    assert_eq!(distinct_key_count, 179);
    assert_eq!(multi_declared_count, 54);
    assert_eq!(
        wildcard_keys,
        ["object_height:*".to_owned(), "layer_height:*".to_owned()]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
    assert!(wildcard_keys.iter().all(|key| owners
        .get(key)
        .is_some_and(|declaring_modules| declaring_modules.len() == 1)));

    println!(
        "real manifest census: entries={entry_count}, distinct_keys={distinct_key_count}, multi_declared={multi_declared_count}, single_declarer_wildcards={wildcard_keys:?}"
    );
}
