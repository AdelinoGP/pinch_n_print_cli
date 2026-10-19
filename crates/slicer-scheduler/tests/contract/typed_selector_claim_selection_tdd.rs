#![allow(missing_docs)]

//! Contract coverage for registry-typed selector routing at scheduler startup.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use slicer_config::{
    assemble_registry, ConfigIngestor, HostChannels, IngestionWarning, ModuleDeclaration,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::resolved_config::{ResolvedConfig, HOST_RUNTIME_KEYS};
use slicer_ir::{ConfigValue, SemVer};
use slicer_scheduler::execution_plan::{
    SPIRAL_VASE_CONFIG_KEY, SUPPORT_GENERATOR_CONFIG_KEY, WALL_GENERATOR_CONFIG_KEY,
};
use slicer_scheduler::{
    dedup_same_claim_modules_with_typed_wall_generator, LoadDiagnostic, LoadedModule,
    LoadedModuleBuilder,
};

const CLASSIC_PERIMETERS_MODULE_ID: &str = "com.core.classic-perimeters";
const ARACHNE_PERIMETERS_MODULE_ID: &str = "com.core.arachne-perimeters";
const PERIMETER_STAGE: &str = "Layer::Perimeters";
const PERIMETER_GENERATOR_CLAIM: &str = "perimeter-generator";

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
    let mut field_types = host_field_types();
    for manifest in &manifests {
        for (key, field_type) in &manifest.field_types {
            field_types
                .entry(key.clone())
                .or_insert_with(|| field_type.clone());
        }
    }

    let modules = manifests
        .into_iter()
        .map(|manifest| ModuleDeclaration {
            module_id: manifest.module_id,
            schema: ConfigSchema {
                entries: manifest
                    .field_types
                    .into_iter()
                    .map(|(key, field_type)| {
                        let field_type = field_types.get(&key).cloned().unwrap_or(field_type);
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

    assemble_registry(&modules, &HostChannels::from_live())
        .unwrap_or_else(|error| panic!("real manifest registry failed to assemble: {error:?}"))
        .registry
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn perimeter_module(id: &str) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(0, 1, 0),
        PERIMETER_STAGE,
        slicer_schema::TIER_LAYER,
        PathBuf::from("placeholder.wasm"),
    )
    .claims(vec![PERIMETER_GENERATOR_CLAIM.to_owned()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true)
    .placeholder_wasm(true)
    .build()
}

fn perimeter_modules() -> Vec<LoadedModule> {
    vec![
        perimeter_module(CLASSIC_PERIMETERS_MODULE_ID),
        perimeter_module(ARACHNE_PERIMETERS_MODULE_ID),
    ]
}

fn kept_ids(
    wall_generator: Option<&ConfigValue>,
    spiral_vase: bool,
    support_type: Option<&str>,
) -> Vec<String> {
    let mut modules = perimeter_modules();
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    dedup_same_claim_modules_with_typed_wall_generator(
        &mut modules,
        &mut diagnostics,
        wall_generator,
        spiral_vase,
        support_type,
    )
    .iter()
    .map(|module| module.id().to_owned())
    .collect()
}

#[test]
fn typed_wall_generator_selector_claim_selection() {
    let registry = real_registry();
    let authored = HashMap::from([
        (
            WALL_GENERATOR_CONFIG_KEY.to_owned(),
            ConfigValue::String("arachne".to_owned()),
        ),
        (SPIRAL_VASE_CONFIG_KEY.to_owned(), ConfigValue::Bool(true)),
        (
            SUPPORT_GENERATOR_CONFIG_KEY.to_owned(),
            ConfigValue::String("tree(auto)".to_owned()),
        ),
        (
            "support_family".to_owned(),
            ConfigValue::String("tree".to_owned()),
        ),
    ]);

    let mut ingestor = ConfigIngestor::tolerant(&registry);
    ingestor
        .ingest_flat(&authored)
        .expect("real registry values should be ingestible");
    let outcome = ingestor.finish();

    assert_eq!(
        outcome.selector_values.len(),
        1,
        "only wall_generator may enter the selector channel"
    );
    assert_eq!(
        outcome
            .selector_values
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([WALL_GENERATOR_CONFIG_KEY.to_owned()]),
        "selector channel must contain exactly the wall_generator key"
    );
    assert_eq!(
        outcome.selector_values.get(WALL_GENERATOR_CONFIG_KEY),
        Some(&ConfigValue::String("arachne".to_owned()))
    );

    let global = outcome.scoped.global().expect("global authored delta");
    assert_eq!(
        global.values.get(SPIRAL_VASE_CONFIG_KEY),
        Some(&ConfigValue::Bool(true)),
        "spiral_vase remains an ordinary typed global value"
    );
    assert_eq!(
        global.values.get(SUPPORT_GENERATOR_CONFIG_KEY),
        Some(&ConfigValue::String("tree(auto)".to_owned())),
        "support_type remains an ordinary global value"
    );
    assert!(!outcome.selector_values.contains_key(SPIRAL_VASE_CONFIG_KEY));
    assert!(!outcome
        .selector_values
        .contains_key(SUPPORT_GENERATOR_CONFIG_KEY));

    assert_eq!(
        global.values.get("support_family"),
        Some(&ConfigValue::String("tree".to_owned())),
        "registered support_family must be retained in the global delta"
    );
    assert!(!outcome.selector_values.contains_key("support_family"));
    let support_family = registry
        .entry("support_family")
        .expect("HOST_RUNTIME_KEYS registers support_family");
    assert_eq!(support_family.field_type, "string");
    assert!(!support_family.selector);
    assert!(!outcome.warnings.iter().any(|warning| {
        matches!(
            warning,
            IngestionWarning::UnrecognizedKey { wire_key, .. } if wire_key == "support_family"
        )
    }));

    let wall_generator = outcome.selector_values.get(WALL_GENERATOR_CONFIG_KEY);
    assert_eq!(
        kept_ids(wall_generator, false, Some("tree(auto)")),
        vec![ARACHNE_PERIMETERS_MODULE_ID.to_owned()],
        "the typed wall_generator selector must choose Arachne"
    );
    assert_eq!(
        kept_ids(wall_generator, false, Some("normal(auto)")),
        vec![ARACHNE_PERIMETERS_MODULE_ID.to_owned()],
        "support_type must not choose the perimeter generator"
    );
    assert_eq!(
        kept_ids(wall_generator, true, Some("tree(auto)")),
        vec![CLASSIC_PERIMETERS_MODULE_ID.to_owned()],
        "spiral_vase remains a separate canonical classic-perimeter override"
    );
}
