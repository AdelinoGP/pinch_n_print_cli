//! Behavior-first tests for four-channel registry assembly and validation.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use slicer_config::{
    assemble_registry, HostChannels, ModuleDeclaration, RegistryLoadError, RegistryWarning,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::resolved_config::{
    HostConfigKey, HostKeyMeta, HostRuntimeKey, ResolvedConfig, HOST_RUNTIME_KEYS,
};

fn field(field_type: &str) -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: field_type.to_owned(),
        ..Default::default()
    }
}

fn field_with_default(field_type: &str, default: &str) -> ConfigFieldEntry {
    let mut entry = field(field_type);
    entry.default = Some(default.to_owned());
    entry
}

fn module(
    module_id: &str,
    fields: Vec<(&str, ConfigFieldEntry)>,
    claim_exclusive_group: Option<&str>,
) -> ModuleDeclaration {
    ModuleDeclaration {
        module_id: module_id.to_owned(),
        schema: ConfigSchema {
            entries: fields
                .into_iter()
                .map(|(key, entry)| (key.to_owned(), entry))
                .collect(),
        },
        claim_exclusive_group: claim_exclusive_group.map(str::to_owned),
    }
}

fn empty_host() -> HostChannels {
    HostChannels::from_parts(Vec::new(), Vec::new(), Vec::new())
}

// These are the seven manifests changed by packet 1's three declaration repairs.
// This targeted list is intentionally not a roster of registry keys; the census
// test derives the complete registry key set independently.
const REPAIRED_MANIFEST_NAMES: [&str; 7] = [
    "arachne-perimeters",
    "classic-perimeters",
    "gyroid-infill",
    "lightning-infill",
    "rectilinear-infill",
    "wave-overhangs",
    "traditional-support",
];

struct RepairedManifest {
    owner: String,
    module_id: String,
    schema: ConfigSchema,
}

fn repaired_manifest_paths() -> Vec<(String, PathBuf)> {
    let modules_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("modules/core-modules");
    REPAIRED_MANIFEST_NAMES
        .iter()
        .map(|name| {
            let owner = (*name).to_owned();
            let path = modules_dir.join(name).join(format!("{name}.toml"));
            assert!(
                path.is_file(),
                "repaired manifest {owner} is missing at {}",
                path.display()
            );
            (owner, path)
        })
        .collect()
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
        .unwrap_or_else(|| panic!("manifest {} has no string type for {key}", path.display()))
}

fn manifest_field_entry(value: &toml::Value, key: &str, path: &Path) -> ConfigFieldEntry {
    let mut entry = ConfigFieldEntry {
        field_type: manifest_field_type(value, key, path),
        ..Default::default()
    };
    let Some(field) = value.as_table() else {
        return entry;
    };

    entry.base_key = field.get("base_key").map(|base| {
        base.as_str()
            .unwrap_or_else(|| {
                panic!(
                    "manifest {} has non-string base_key for {key}",
                    path.display()
                )
            })
            .to_owned()
    });
    entry.values = field.get("values").map(|values| {
        values
            .as_array()
            .unwrap_or_else(|| panic!("manifest {} has non-array values for {key}", path.display()))
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .unwrap_or_else(|| {
                        panic!(
                            "manifest {} has non-string enum value for {key}",
                            path.display()
                        )
                    })
                    .to_owned()
            })
            .collect()
    });
    entry
}

fn parse_repaired_manifests() -> Vec<RepairedManifest> {
    let manifests = repaired_manifest_paths()
        .into_iter()
        .map(|(owner, path)| {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            let document: toml::Value = toml::from_str(&text)
                .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
            let module_id = document
                .get("module")
                .and_then(toml::Value::as_table)
                .and_then(|module| module.get("id"))
                .and_then(toml::Value::as_str)
                .unwrap_or_else(|| panic!("manifest {} has no module.id", path.display()))
                .to_owned();
            let schema = document
                .get("config")
                .and_then(toml::Value::as_table)
                .and_then(|config| config.get("schema"))
                .and_then(toml::Value::as_table)
                .unwrap_or_else(|| panic!("manifest {} has no config.schema", path.display()))
                .iter()
                .map(|(key, value)| (key.clone(), manifest_field_entry(value, key, &path)))
                .collect();
            RepairedManifest {
                owner,
                module_id,
                schema: ConfigSchema { entries: schema },
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(manifests.len(), REPAIRED_MANIFEST_NAMES.len());
    manifests
}

fn repaired_manifest<'a>(manifests: &'a [RepairedManifest], owner: &str) -> &'a RepairedManifest {
    manifests
        .iter()
        .find(|manifest| manifest.owner == owner)
        .unwrap_or_else(|| panic!("repaired manifest {owner} was not parsed"))
}

fn declared_owners(manifests: &[RepairedManifest], key: &str) -> BTreeSet<String> {
    let owners = manifests
        .iter()
        .filter(|manifest| manifest.schema.entries.contains_key(key))
        .map(|manifest| manifest.owner.clone())
        .collect::<BTreeSet<_>>();
    assert!(!owners.is_empty(), "no repaired manifest declares {key}");
    owners
}

fn repaired_field<'a>(manifest: &'a RepairedManifest, key: &str) -> &'a ConfigFieldEntry {
    manifest
        .schema
        .entries
        .get(key)
        .unwrap_or_else(|| panic!("{} does not declare {key}", manifest.owner))
}

fn parse_real_manifest(name: &str, keys: &[&str]) -> (ModuleDeclaration, toml::Value) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("modules/core-modules")
        .join(name)
        .join(format!("{name}.toml"));
    assert!(path.is_file(), "manifest {} is missing", path.display());
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let document: toml::Value = toml::from_str(&text)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
    let module_id = document
        .get("module")
        .and_then(toml::Value::as_table)
        .and_then(|module| module.get("id"))
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("manifest {} has no module.id", path.display()))
        .to_owned();
    let schema_table = document
        .get("config")
        .and_then(toml::Value::as_table)
        .and_then(|config| config.get("schema"))
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("manifest {} has no config.schema", path.display()));
    let schema = keys
        .iter()
        .map(|key| {
            let value = schema_table
                .get(*key)
                .unwrap_or_else(|| panic!("manifest {} does not declare {key}", path.display()));
            ((*key).to_owned(), manifest_field_entry(value, key, &path))
        })
        .collect();

    (
        ModuleDeclaration {
            module_id,
            schema: ConfigSchema { entries: schema },
            claim_exclusive_group: None,
        },
        document,
    )
}

fn manifest_numeric_default(document: &toml::Value, key: &str) -> Option<f64> {
    document
        .get("config")
        .and_then(toml::Value::as_table)
        .and_then(|config| config.get("schema"))
        .and_then(toml::Value::as_table)
        .and_then(|schema| schema.get(key))
        .and_then(toml::Value::as_table)
        .and_then(|field| field.get("default"))
        .and_then(toml::Value::as_float)
}

#[test]
fn repaired_conflicting_keys_agree_across_all_declarers() {
    let manifests = parse_repaired_manifests();
    let expected_bridge_owners = [
        "arachne-perimeters",
        "classic-perimeters",
        "gyroid-infill",
        "lightning-infill",
        "rectilinear-infill",
        "wave-overhangs",
    ]
    .into_iter()
    .map(String::from)
    .collect::<BTreeSet<_>>();
    let expected_initial_owners = [
        "arachne-perimeters",
        "classic-perimeters",
        "gyroid-infill",
        "lightning-infill",
        "rectilinear-infill",
    ]
    .into_iter()
    .map(String::from)
    .collect::<BTreeSet<_>>();
    let expected_support_owners = ["traditional-support"]
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let expected_support_values = [
        "default",
        "grid",
        "snug",
        "organic",
        "tree_slim",
        "tree_strong",
        "tree_hybrid",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();

    assert_eq!(
        declared_owners(&manifests, "bridge_line_width"),
        expected_bridge_owners
    );
    assert_eq!(
        declared_owners(&manifests, "initial_layer_line_width"),
        expected_initial_owners
    );
    assert_eq!(
        declared_owners(&manifests, "support_style"),
        expected_support_owners
    );
    assert!(
        !repaired_manifest(&manifests, "wave-overhangs")
            .schema
            .entries
            .contains_key("initial_layer_line_width"),
        "wave-overhangs is bridge-only"
    );

    for owner in &expected_bridge_owners {
        let entry = repaired_field(repaired_manifest(&manifests, owner), "bridge_line_width");
        assert_eq!(entry.field_type, "float_or_percent", "bridge owner {owner}");
        assert_eq!(
            entry.base_key.as_deref(),
            Some("nozzle_diameter"),
            "bridge owner {owner}"
        );
    }
    for owner in &expected_initial_owners {
        let entry = repaired_field(
            repaired_manifest(&manifests, owner),
            "initial_layer_line_width",
        );
        assert_eq!(
            entry.field_type, "float_or_percent",
            "initial owner {owner}"
        );
        assert_eq!(
            entry.base_key.as_deref(),
            Some("nozzle_diameter"),
            "initial owner {owner}"
        );
    }

    let support_style = repaired_field(
        repaired_manifest(&manifests, "traditional-support"),
        "support_style",
    );
    assert_eq!(support_style.field_type, "enum");
    assert_eq!(
        support_style.values.as_ref(),
        Some(&expected_support_values)
    );

    let modules = manifests
        .iter()
        .map(|manifest| ModuleDeclaration {
            module_id: manifest.module_id.clone(),
            schema: manifest.schema.clone(),
            claim_exclusive_group: None,
        })
        .collect::<Vec<_>>();
    let outcome = assemble_registry(&modules, &empty_host()).unwrap_or_else(|error| {
        panic!("repaired real manifests must assemble without a type warning: {error:?}")
    });

    let bridge = outcome
        .registry
        .entry("bridge_line_width")
        .expect("assembled bridge declaration");
    assert_eq!(bridge.field_type, "float_or_percent");
    assert_eq!(bridge.base_key.as_deref(), Some("nozzle_diameter"));

    let initial = outcome
        .registry
        .entry("initial_layer_line_width")
        .expect("assembled initial-layer declaration");
    assert_eq!(initial.field_type, "float_or_percent");
    assert_eq!(initial.base_key.as_deref(), Some("nozzle_diameter"));

    let assembled_support_style = outcome
        .registry
        .entry("support_style")
        .expect("assembled support-style declaration");
    assert_eq!(assembled_support_style.field_type, "enum");
    assert_eq!(
        assembled_support_style.values.as_ref(),
        Some(&expected_support_values)
    );
}

#[test]
fn phase_b_support_line_width_declares_typed_nozzle_base() {
    let (module, document) = parse_real_manifest(
        "tree-support-planner",
        &["support_line_width", "nozzle_diameter"],
    );
    assert_eq!(
        manifest_numeric_default(&document, "support_line_width"),
        Some(0.0)
    );

    let module_field = module
        .schema
        .entries
        .get("support_line_width")
        .expect("support-line-width module declaration");
    assert_eq!(module_field.field_type, "float_or_percent");
    assert_eq!(module_field.base_key.as_deref(), Some("nozzle_diameter"));

    let outcome = assemble_registry(&[module], &HostChannels::from_live())
        .expect("real tree-support manifest must assemble");
    let entry = outcome
        .registry
        .entry("support_line_width")
        .expect("assembled support-line-width declaration");
    assert_eq!(entry.field_type, "float_or_percent");
    assert_eq!(entry.base_key.as_deref(), Some("nozzle_diameter"));
    assert_eq!(
        entry
            .default
            .as_deref()
            .and_then(|default| default.parse::<f64>().ok()),
        Some(0.0)
    );
}

#[test]
fn phase_b_speed_percent_family_declares_typed_outer_wall_base() {
    const SPEED_KEYS: [&str; 4] = [
        "overhang_1_4_speed",
        "overhang_2_4_speed",
        "overhang_3_4_speed",
        "overhang_4_4_speed",
    ];
    let (module, document) = parse_real_manifest(
        "overhang-classifier-default",
        &[
            "outer_wall_speed",
            "overhang_1_4_speed",
            "overhang_2_4_speed",
            "overhang_3_4_speed",
            "overhang_4_4_speed",
        ],
    );
    let host = HostChannels::from_live();
    let outcome = assemble_registry(std::slice::from_ref(&module), &host)
        .expect("real overhang manifest must assemble");

    for key in SPEED_KEYS {
        let module_entry = module
            .schema
            .entries
            .get(key)
            .expect("overhang module declaration");
        assert_eq!(module_entry.field_type, "float_or_percent", "module {key}");
        assert_eq!(
            module_entry.base_key.as_deref(),
            Some("outer_wall_speed"),
            "module {key} base"
        );
        assert_eq!(
            manifest_numeric_default(&document, key),
            Some(0.0),
            "module {key} default"
        );

        let host_row = host
            .speed_keys
            .iter()
            .find(|row| row.key == key)
            .unwrap_or_else(|| panic!("host speed declaration {key}"));
        assert_eq!(
            host_row.meta.wire_type,
            Some("float_or_percent"),
            "host {key} wire type"
        );
        assert_eq!(
            host_row.meta.wire_type.unwrap_or(host_row.field_type),
            "float_or_percent",
            "effective host {key} wire type"
        );
        assert_eq!(
            host_row
                .default
                .as_deref()
                .and_then(|default| default.parse::<f64>().ok()),
            Some(0.0),
            "host {key} default"
        );

        let registry_entry = outcome
            .registry
            .entry(key)
            .expect("assembled overhang declaration");
        assert_eq!(
            registry_entry.field_type, "float_or_percent",
            "registry {key} type"
        );
        assert_eq!(
            registry_entry.base_key.as_deref(),
            Some("outer_wall_speed"),
            "registry {key} base"
        );
        assert_eq!(
            registry_entry
                .default
                .as_deref()
                .and_then(|default| default.parse::<f64>().ok()),
            Some(0.0),
            "registry {key} default"
        );
    }
}

fn host_key(key: &'static str, field_type: &'static str, default: &str) -> HostConfigKey {
    let mut row = ResolvedConfig::host_config_keys()
        .into_iter()
        .next()
        .expect("the live host channel has a seed row");
    row.key = key;
    row.field_type = field_type;
    row.default = Some(default.to_owned());
    row.meta = HostKeyMeta::NONE;
    row.denied_scopes = &[];
    row
}

fn runtime_key(
    key: &'static str,
    field_type: &'static str,
    default: Option<&'static str>,
) -> HostRuntimeKey {
    let mut row = HOST_RUNTIME_KEYS[0];
    row.key = key;
    row.field_type = field_type;
    row.default = default;
    row.meta = HostKeyMeta::NONE;
    row.selector = false;
    row.denied_scopes = &[];
    row
}

#[test]
fn default_precedence_host_then_alphabetical_first() {
    let alpha = module(
        "alpha-module",
        vec![("chosen", field_with_default("string", "alpha"))],
        None,
    );
    let beta = module(
        "beta-module",
        vec![("chosen", field_with_default("string", "beta"))],
        None,
    );

    let without_host = assemble_registry(&[beta.clone(), alpha.clone()], &empty_host())
        .expect("module-only defaults should reconcile");
    assert_eq!(
        without_host
            .registry
            .entry("chosen")
            .and_then(|entry| entry.default.as_deref()),
        Some("alpha")
    );

    let host = HostChannels::from_parts(
        vec![host_key("chosen", "string", "host")],
        Vec::new(),
        Vec::new(),
    );
    let with_host = assemble_registry(&[beta, alpha], &host).expect("host default should win");
    assert_eq!(
        with_host
            .registry
            .entry("chosen")
            .and_then(|entry| entry.default.as_deref()),
        Some("host")
    );
}

#[test]
fn claim_exclusive_divergent_default_emits_warning_naming_both_declarers() {
    let alpha = module(
        "alpha-module",
        vec![("wall_mode", field_with_default("string", "alpha"))],
        Some("wall-generator"),
    );
    let beta = module(
        "beta-module",
        vec![("wall_mode", field_with_default("string", "beta"))],
        Some("wall-generator"),
    );

    let outcome = assemble_registry(&[beta, alpha], &empty_host()).expect("claims can reconcile");
    assert!(outcome.warnings.iter().any(|warning| {
        matches!(
            warning,
            RegistryWarning::ClaimExclusiveDivergence {
                key,
                first_module,
                first_value,
                second_module,
                second_value,
            } if key == "wall_mode"
                && first_module == "alpha-module"
                && first_value == "alpha"
                && second_module == "beta-module"
                && second_value == "beta"
        )
    }));
}

#[test]
fn claim_exclusive_divergent_bounds_emit_warning_naming_both_declarers_and_values() {
    let mut alpha_field = field("float");
    alpha_field.min = Some(0.0);
    alpha_field.max = Some(10.0);
    let mut beta_field = field("float");
    beta_field.min = Some(2.0);
    beta_field.max = Some(8.0);

    let outcome = assemble_registry(
        &[
            module(
                "beta-module",
                vec![("bounded", beta_field)],
                Some("wall-generator"),
            ),
            module(
                "alpha-module",
                vec![("bounded", alpha_field)],
                Some("wall-generator"),
            ),
        ],
        &empty_host(),
    )
    .expect("claim-exclusive bounds can reconcile");
    assert!(outcome.warnings.iter().any(|warning| {
        matches!(
            warning,
            RegistryWarning::ClaimExclusiveBoundsDivergence {
                key,
                first_module,
                first_min,
                first_max,
                second_module,
                second_min,
                second_max,
            } if key == "bounded"
                && first_module == "alpha-module"
                && *first_min == Some(0.0)
                && *first_max == Some(10.0)
                && second_module == "beta-module"
                && *second_min == Some(2.0)
                && *second_max == Some(8.0)
        )
    }));
}

#[test]
fn bounds_intersection_reported_not_applied_silently() {
    let mut alpha_field = field("float");
    alpha_field.min = Some(0.0);
    alpha_field.max = Some(10.0);
    let mut beta_field = field("float");
    beta_field.min = Some(2.0);
    beta_field.max = Some(8.0);

    let outcome = assemble_registry(
        &[
            module("beta-module", vec![("bounded", beta_field)], None),
            module("alpha-module", vec![("bounded", alpha_field)], None),
        ],
        &empty_host(),
    )
    .expect("compatible bounds should reconcile");
    let entry = outcome.registry.entry("bounded").expect("bounded entry");
    assert_eq!(entry.min, Some(2.0));
    assert_eq!(entry.max, Some(8.0));
    assert!(outcome.warnings.iter().any(|warning| {
        matches!(
            warning,
            RegistryWarning::BoundsIntersection {
                key,
                effective_min,
                effective_max,
                declarers,
            } if key == "bounded"
                && *effective_min == Some(2.0)
                && *effective_max == Some(8.0)
                && declarers == &["alpha-module".to_owned(), "beta-module".to_owned()]
        )
    }));
}

#[test]
fn denied_scopes_union_across_declarers() {
    let mut alpha_field = field("string");
    alpha_field.denied_scopes = vec!["tool".to_owned(), "object".to_owned()];
    let mut beta_field = field("string");
    beta_field.denied_scopes = vec!["paint_semantic".to_owned()];

    let outcome = assemble_registry(
        &[
            module("beta-module", vec![("scoped", beta_field)], None),
            module("alpha-module", vec![("scoped", alpha_field)], None),
        ],
        &empty_host(),
    )
    .expect("known denied scopes should reconcile");
    assert_eq!(
        outcome
            .registry
            .entry("scoped")
            .expect("scoped entry")
            .denied_scopes,
        vec!["object", "paint_semantic", "tool"]
    );
}

#[test]
fn enum_domains_agree_and_reconcile() {
    let mut alpha_field = field("enum");
    alpha_field.values = Some(vec!["default".to_owned(), "alternate".to_owned()]);
    let mut beta_field = field("enum");
    beta_field.values = Some(vec!["default".to_owned(), "alternate".to_owned()]);

    let outcome = assemble_registry(
        &[
            module("beta-module", vec![("mode", beta_field)], None),
            module("alpha-module", vec![("mode", alpha_field)], None),
        ],
        &empty_host(),
    )
    .expect("equal enum domains should reconcile");
    assert_eq!(
        outcome
            .registry
            .entry("mode")
            .and_then(|entry| entry.values.clone()),
        Some(vec!["default".to_owned(), "alternate".to_owned()])
    );
}

#[test]
fn ui_metadata_is_advisory_with_host_then_alphabetical_precedence() {
    let mut host = host_key("labelled", "string", "host");
    let mut host_meta = HostKeyMeta::NONE;
    host_meta.display = Some("Host label");
    host_meta.group = Some("Host group");
    host.meta = host_meta;

    let mut alpha_field = field("string");
    alpha_field.display = Some("Alpha label".to_owned());
    alpha_field.group = Some("Alpha group".to_owned());
    let mut beta_field = field("string");
    beta_field.display = Some("Beta label".to_owned());
    beta_field.group = Some("Beta group".to_owned());

    let outcome = assemble_registry(
        &[
            module("beta-module", vec![("labelled", beta_field)], None),
            module("alpha-module", vec![("labelled", alpha_field)], None),
        ],
        &HostChannels::from_parts(vec![host], Vec::new(), Vec::new()),
    )
    .expect("metadata does not conflict");
    let entry = outcome.registry.entry("labelled").expect("labelled entry");
    assert_eq!(entry.host_meta, Some(host_meta));
    assert_eq!(
        entry
            .module_meta
            .as_ref()
            .and_then(|metadata| metadata.display.as_deref()),
        Some("Alpha label")
    );
    assert_eq!(
        entry
            .module_meta
            .as_ref()
            .and_then(|metadata| metadata.group.as_deref()),
        Some("Alpha group")
    );
}

#[test]
fn module_default_for_host_owned_key_documented_with_warning() {
    let host = HostChannels::from_parts(
        vec![host_key("host_owned", "string", "host")],
        Vec::new(),
        Vec::new(),
    );
    let module = module(
        "module-default",
        vec![("host_owned", field_with_default("string", "module"))],
        None,
    );

    let outcome = assemble_registry(&[module], &host).expect("host-owned default is non-fatal");
    assert_eq!(
        outcome
            .registry
            .entry("host_owned")
            .and_then(|entry| entry.default.as_deref()),
        Some("host")
    );
    assert!(outcome.warnings.iter().any(|warning| {
        matches!(
            warning,
            RegistryWarning::ModuleDefaultForHostKey { key, module_id }
                if key == "host_owned" && module_id == "module-default"
        )
    }));
}

#[test]
fn absent_host_default_remains_absent_when_module_supplies_default() {
    let mut host_row = host_key("host_owned_without_default", "string", "unused");
    host_row.default = None;
    let host = HostChannels::from_parts(vec![host_row], Vec::new(), Vec::new());
    let module = module(
        "module-default",
        vec![(
            "host_owned_without_default",
            field_with_default("string", "module"),
        )],
        None,
    );

    let outcome = assemble_registry(&[module], &host).expect("host-owned default is non-fatal");
    let entry = outcome
        .registry
        .entry("host_owned_without_default")
        .expect("host-owned entry with absent default");
    assert_eq!(entry.default, None);
    assert!(outcome.warnings.iter().any(|warning| {
        matches!(
            warning,
            RegistryWarning::ModuleDefaultForHostKey { key, module_id }
                if key == "host_owned_without_default" && module_id == "module-default"
        )
    }));
}

#[test]
fn provenance_names_every_contributor() {
    let host = host_key("all_sources", "float", "host");
    let speed = host_key("all_sources", "float", "speed");
    let runtime = runtime_key("all_sources", "float", Some("runtime"));
    let modules = vec![
        module("beta-module", vec![("all_sources", field("float"))], None),
        module("alpha-module", vec![("all_sources", field("float"))], None),
    ];
    let channels = HostChannels::from_parts(vec![host], vec![speed], vec![runtime]);

    let outcome = assemble_registry(&modules, &channels).expect("all channels agree");
    assert_eq!(
        outcome
            .registry
            .entry("all_sources")
            .expect("all-source entry")
            .provenance,
        vec![
            "alpha-module".to_owned(),
            "beta-module".to_owned(),
            "host".to_owned(),
            "runtime".to_owned(),
            "speed".to_owned(),
        ]
    );
}

/// Regression for the CONFIG_BLOCK claim-drop leak: the registry spans every
/// discovered module (claim dedup drops a module from dispatch only, never
/// from the config schema), so `config_block_map` carries the claim-losing
/// module's keys. The emitted `CONFIG_BLOCK` contracts the LOADED set
/// (packet-06 AC-4): `config_block_map_for_modules` must project only keys
/// whose provenance names a live module or a host channel. Regression shape:
/// `wall_generator=classic` drops `arachne-perimeters` from dispatch, yet its
/// `initial_layer_min_bead_width` / `min_bead_count` / `max_bead_count` keys
/// leaked into the emitted block.
#[test]
fn config_block_map_for_modules_projects_only_live_module_and_host_keys() {
    let host = host_key("host_owned", "float", "0.5");
    let modules = vec![
        module(
            "alpha-module",
            vec![
                ("alpha_key", field_with_default("float", "1")),
                ("shared_key", field_with_default("float", "2")),
            ],
            None,
        ),
        module(
            "beta-module",
            vec![
                ("beta_key", field_with_default("float", "3")),
                ("shared_key", field_with_default("float", "2")),
            ],
            None,
        ),
    ];
    let channels = HostChannels::from_parts(vec![host], Vec::new(), Vec::new());
    let registry = assemble_registry(&modules, &channels)
        .expect("distinct keys with defaults are non-fatal")
        .registry;

    // The resolution-side effective map is seeded the way the default run
    // resolves it: non-typed registry defaults land in `extensions`.
    let mut resolved = ResolvedConfig::default();
    for key in ["alpha_key", "beta_key", "shared_key", "host_owned"] {
        resolved.extensions.insert(
            key.to_owned(),
            slicer_ir::ConfigValue::Float(
                registry
                    .entry(key)
                    .and_then(|entry| entry.default.as_deref())
                    .and_then(|value| value.parse::<f64>().ok())
                    .expect("fixture default parses"),
            ),
        );
    }

    let live: BTreeSet<String> = ["alpha-module".to_owned()].into_iter().collect();
    let projected = registry.config_block_map_for_modules(&resolved, &live);

    assert!(
        projected.contains_key("alpha_key"),
        "a live module's key must be projected"
    );
    assert!(
        projected.contains_key("shared_key"),
        "a key declared by a live module and a dropped one must be projected"
    );
    assert!(
        projected.contains_key("host_owned"),
        "a host-channel key must be projected regardless of the live module set"
    );
    assert!(
        !projected.contains_key("beta_key"),
        "a claim-dropped module's key must not leak into the emitted block"
    );

    // The unfiltered projection is unchanged: resolution-side consumers rely
    // on it spanning every declaration.
    let unfiltered = registry.config_block_map(&resolved);
    for key in ["alpha_key", "beta_key", "shared_key", "host_owned"] {
        assert!(
            unfiltered.contains_key(key),
            "{key} must survive the unfiltered projection"
        );
    }
}

#[test]
fn runtime_selector_flag_reaches_registry_entry_and_wall_generator_denials_are_retained() {
    let outcome = assemble_registry(&[], &HostChannels::from_live())
        .expect("the authored wall-generator selector is structurally valid");
    let entry = outcome
        .registry
        .entry("wall_generator")
        .expect("runtime selector entry");
    assert!(entry.selector);
    assert_eq!(entry.default.as_deref(), Some("classic"));
    assert_eq!(
        entry.denied_scopes,
        vec![
            "object",
            "layer_range",
            "modifier",
            "paint_semantic",
            "tool"
        ]
    );
}

#[test]
fn selector_validation_accepts_wall_generator_host_row() {
    let outcome = assemble_registry(&[], &HostChannels::from_live());
    assert!(
        outcome.is_ok(),
        "wall_generator's five denials must permit loading"
    );
    let registry = outcome.expect("validated wall-generator host row").registry;
    assert_eq!(
        registry.entry("wall_generator").map(|entry| entry.selector),
        Some(true)
    );
}

#[test]
fn base_key_is_retained_after_valid_assembly() {
    let mut width = field("float_or_percent");
    width.base_key = Some("nozzle_diameter".to_owned());
    let nozzle = field("float");
    let outcome = assemble_registry(
        &[module(
            "width-module",
            vec![("width", width), ("nozzle_diameter", nozzle)],
            None,
        )],
        &empty_host(),
    )
    .expect("valid base metadata should assemble");
    assert_eq!(
        outcome
            .registry
            .entry("width")
            .and_then(|entry| entry.base_key.as_deref()),
        Some("nozzle_diameter")
    );
}

#[test]
fn type_disagreement_is_a_load_error() {
    let error = assemble_registry(
        &[
            module("beta-module", vec![("conflict", field("string"))], None),
            module("alpha-module", vec![("conflict", field("bool"))], None),
        ],
        &empty_host(),
    )
    .expect_err("different field types must be rejected");
    match error {
        RegistryLoadError::TypeDisagreement { key, declarers } => {
            assert_eq!(key, "conflict");
            assert_eq!(
                declarers,
                vec!["alpha-module".to_owned(), "beta-module".to_owned()]
            );
        }
        other => panic!("unexpected load error: {other:?}"),
    }
}

#[test]
fn enum_domain_disagreement_is_a_load_error() {
    let mut alpha_field = field("enum");
    alpha_field.values = Some(vec!["a".to_owned(), "b".to_owned()]);
    let mut beta_field = field("enum");
    beta_field.values = Some(vec!["a".to_owned(), "c".to_owned()]);
    let error = assemble_registry(
        &[
            module("beta-module", vec![("enum_conflict", beta_field)], None),
            module("alpha-module", vec![("enum_conflict", alpha_field)], None),
        ],
        &empty_host(),
    )
    .expect_err("different enum domains must be rejected");
    match error {
        RegistryLoadError::EnumDomainDisagreement { key, declarers } => {
            assert_eq!(key, "enum_conflict");
            assert_eq!(
                declarers,
                vec!["alpha-module".to_owned(), "beta-module".to_owned()]
            );
        }
        other => panic!("unexpected load error: {other:?}"),
    }
}

#[test]
fn selector_marked_per_region_statable_key_is_a_load_error() {
    let mut selector = field("string");
    selector.selector = true;
    let error = assemble_registry(
        &[module(
            "selector-module",
            vec![("bad_selector", selector)],
            None,
        )],
        &empty_host(),
    )
    .expect_err("an empty selector denial policy leaves every region statable");
    assert_eq!(
        error,
        RegistryLoadError::SelectorStatablePerRegion {
            key: "bad_selector".to_owned()
        }
    );
}

#[test]
fn base_key_missing_is_a_load_error() {
    let mut dependent = field("float_or_percent");
    dependent.base_key = Some("missing_base".to_owned());
    let error = assemble_registry(
        &[module(
            "dependent-module",
            vec![("dependent", dependent)],
            None,
        )],
        &empty_host(),
    )
    .expect_err("an undeclared base key must be rejected");
    assert_eq!(
        error,
        RegistryLoadError::BaseKeyMissing {
            key: "dependent".to_owned(),
            base: "missing_base".to_owned(),
        }
    );
}

#[test]
fn base_key_not_percent_compatible_is_a_load_error() {
    let mut dependent = field("float_or_percent");
    dependent.base_key = Some("text_base".to_owned());
    let error = assemble_registry(
        &[module(
            "dependent-module",
            vec![("dependent", dependent), ("text_base", field("string"))],
            None,
        )],
        &empty_host(),
    )
    .expect_err("a string cannot supply a percent base");
    assert_eq!(
        error,
        RegistryLoadError::BaseKeyNotPercentCompatible {
            key: "dependent".to_owned(),
            base: "text_base".to_owned(),
        }
    );
}

#[test]
fn base_key_cycle_is_a_load_error() {
    let mut first = field("float_or_percent");
    first.base_key = Some("second".to_owned());
    let mut second = field("float_or_percent");
    second.base_key = Some("first".to_owned());
    let error = assemble_registry(
        &[module(
            "cycle-module",
            vec![("first", first), ("second", second)],
            None,
        )],
        &empty_host(),
    )
    .expect_err("cyclic bases must be rejected");
    assert_eq!(
        error,
        RegistryLoadError::BaseKeyCycle {
            key: "first".to_owned(),
            base: "second".to_owned(),
        }
    );
}

#[test]
fn unknown_denied_scope_is_a_load_error() {
    let mut field_with_unknown_scope = field("string");
    field_with_unknown_scope.denied_scopes = vec!["galaxy".to_owned()];
    let error = assemble_registry(
        &[module(
            "scope-module",
            vec![("scoped", field_with_unknown_scope)],
            None,
        )],
        &empty_host(),
    )
    .expect_err("unknown denied scopes must be rejected");
    assert_eq!(
        error,
        RegistryLoadError::UnknownDeniedScope {
            key: "scoped".to_owned(),
            scope: "galaxy".to_owned(),
        }
    );
}
