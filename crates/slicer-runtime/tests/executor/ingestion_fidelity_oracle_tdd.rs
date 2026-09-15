//! Independent authored-value oracle for the cube fixture.
//!
//! The expected side is decoded directly from the 3MF JSON and the registry.
//! The delivered side is observed only from live scheduler plans.  Keeping
//! those paths separate is intentional: packet 03 must make the principal
//! assertion green without changing this expectation or its ownership join.

#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use slicer_config::{
    assemble_registry, ConfigSchemaRegistry, HostChannels, ModuleDeclaration, RegistryEntry,
};
use slicer_ir::{ConfigValue, MeshIR};
use slicer_runtime::{load_modules_from_roots, CompiledModuleStatic, LoadedModule};

#[derive(Clone)]
struct AuthoredValue {
    json: serde_json::Value,
    raw: Option<String>,
}

impl AuthoredValue {
    fn from_json(json: serde_json::Value) -> Self {
        let raw = match &json {
            serde_json::Value::String(value) => Some(value.clone()),
            serde_json::Value::Bool(value) => Some(value.to_string()),
            serde_json::Value::Number(value) => Some(value.to_string()),
            serde_json::Value::Array(_)
            | serde_json::Value::Object(_)
            | serde_json::Value::Null => None,
        };
        Self { json, raw }
    }

    fn string(raw: &str) -> Self {
        Self::from_json(serde_json::Value::String(raw.to_owned()))
    }

    fn is_authored_string(&self) -> bool {
        matches!(self.json, serde_json::Value::String(_))
    }
}

struct OracleInputs {
    authored: BTreeMap<String, AuthoredValue>,
    production_source: HashMap<String, ConfigValue>,
    mesh: Arc<MeshIR>,
    modules: Vec<LoadedModule>,
    registry: ConfigSchemaRegistry,
    ownership: BTreeMap<String, Vec<String>>,
    object_scope_keys: BTreeSet<String>,
    support_family_modules: BTreeSet<String>,
}

struct LivePlans {
    arachne: slicer_runtime::run::PrepassContext,
    classic: slicer_runtime::run::PrepassContext,
}

#[derive(Clone)]
struct Observation {
    key: String,
    module_id: String,
    authored: String,
    expected: ConfigValue,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn fixture_path() -> PathBuf {
    workspace_root().join("resources").join("cube_4color.3mf")
}

fn core_modules_path() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn projected_declarations(modules: &[LoadedModule]) -> Vec<ModuleDeclaration> {
    modules
        .iter()
        .map(|module| ModuleDeclaration {
            module_id: module.id().to_owned(),
            schema: module.config_schema().clone(),
            claim_exclusive_group: None,
        })
        .collect()
}

fn read_authored_values(path: &Path) -> BTreeMap<String, AuthoredValue> {
    let file = File::open(path)
        .unwrap_or_else(|error| panic!("fixture missing: {} ({error})", path.display()));
    let mut archive = zip::ZipArchive::new(file).unwrap_or_else(|error| {
        panic!(
            "fixture is not a readable 3MF: {} ({error})",
            path.display()
        )
    });
    let names: Vec<String> = (0..archive.len())
        .map(|index| {
            archive
                .name_for_index(index)
                .unwrap_or_else(|| {
                    panic!(
                        "fixture member name missing: {} index={index}",
                        path.display()
                    )
                })
                .to_owned()
        })
        .collect();

    if let Some(layer_range) = names.iter().find(|name| {
        name.rsplit('/')
            .next()
            .is_some_and(|base| base.eq_ignore_ascii_case("layer_config_ranges.xml"))
    }) {
        panic!(
            "fixture must not contain layer-range part: {} member {layer_range}",
            path.display()
        );
    }

    let member = "Metadata/project_settings.config";
    assert!(
        names.iter().any(|name| name == member),
        "fixture missing: {} member {member}",
        path.display()
    );
    let mut text = String::new();
    archive
        .by_name(member)
        .unwrap_or_else(|error| {
            panic!(
                "fixture member unreadable: {} member {member} ({error})",
                path.display()
            )
        })
        .read_to_string(&mut text)
        .unwrap_or_else(|error| {
            panic!(
                "fixture member unreadable: {} member {member} ({error})",
                path.display()
            )
        });
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!(
            "fixture member has malformed JSON: {} member {member} ({error})",
            path.display()
        )
    });
    let object = value.as_object().unwrap_or_else(|| {
        panic!(
            "fixture member must be a JSON object: {} member {member}",
            path.display()
        )
    });
    assert!(
        !object.is_empty(),
        "fixture member has no authored values: {member}"
    );

    object
        .iter()
        .map(|(key, value)| (key.clone(), AuthoredValue::from_json(value.clone())))
        .collect()
}

fn oracle_inputs() -> OracleInputs {
    let path = fixture_path();
    assert!(
        path.exists(),
        "fixture missing: {} — restore resources/",
        path.display()
    );
    let authored = read_authored_values(&path);
    assert!(
        !authored.is_empty(),
        "independently decoded authored map must be non-empty"
    );

    let production_source = slicer_model_io::read_3mf_project_settings(&path)
        .unwrap_or_else(|| panic!("production project settings missing: {}", path.display()));
    assert!(
        !production_source.is_empty(),
        "production-decoded source map must be non-empty: {}",
        path.display()
    );
    assert_eq!(
        production_source.get("wall_generator"),
        Some(&ConfigValue::String("arachne".to_owned())),
        "fixture selector must be production-decoded as arachne"
    );

    let mesh = Arc::new(
        slicer_model_io::load_model(&path)
            .unwrap_or_else(|error| panic!("load_model({}) failed: {error}", path.display())),
    );
    let object_scope_keys = mesh
        .objects
        .iter()
        .flat_map(|object| object.config.data.keys().cloned())
        .collect::<BTreeSet<_>>();

    let report = load_modules_from_roots(std::slice::from_ref(&core_modules_path()))
        .unwrap_or_else(|error| panic!("load core module schemas failed: {error:?}"));
    assert!(
        !report.modules.is_empty(),
        "loaded module schemas must be non-empty"
    );
    let modules = report.modules;
    let declarations = projected_declarations(&modules);
    let registry = assemble_registry(&declarations, &HostChannels::from_live())
        .unwrap_or_else(|error| panic!("assemble registry from live schemas failed: {error}"))
        .registry;
    assert!(
        !registry.is_empty(),
        "registry derived from live schemas must be non-empty"
    );

    let mut ownership: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut ordered_modules = modules.iter().collect::<Vec<_>>();
    ordered_modules.sort_by(|left, right| left.id().cmp(right.id()));
    for module in ordered_modules {
        for key in module.config_keys() {
            if registry.entry(&key).is_none() {
                panic!("registry key disappeared during ownership projection: {key}");
            }
            ownership
                .entry(key)
                .or_default()
                .push(module.id().to_owned());
        }
    }
    assert!(
        !ownership.is_empty(),
        "registry-derived ownership must be non-empty"
    );

    let support_family_modules: BTreeSet<String> = modules
        .iter()
        .filter(|module| {
            module
                .claims()
                .iter()
                .any(|claim| claim.starts_with("support-family:"))
        })
        .map(|module| module.id().to_owned())
        .collect();
    assert!(
        !support_family_modules.is_empty(),
        "loaded schemas must expose support-family modules"
    );

    OracleInputs {
        authored,
        production_source,
        mesh,
        modules,
        registry,
        ownership,
        object_scope_keys,
        support_family_modules,
    }
}

fn live_plans(inputs: &OracleInputs) -> LivePlans {
    let arachne_source = inputs.production_source.clone();
    let mut classic_source = inputs.production_source.clone();
    classic_source.insert(
        "wall_generator".to_owned(),
        ConfigValue::String("classic".to_owned()),
    );
    let differing_keys: BTreeSet<&str> = arachne_source
        .keys()
        .chain(classic_source.keys())
        .filter(|key| arachne_source.get(*key) != classic_source.get(*key))
        .map(String::as_str)
        .collect();
    assert_eq!(differing_keys, BTreeSet::from(["wall_generator"]));

    let module_dirs = vec![core_modules_path()];
    let arachne = slicer_runtime::run::prepare_prepass_context(
        Arc::clone(&inputs.mesh),
        arachne_source,
        &module_dirs,
        true,
        false,
    )
    .unwrap_or_else(|error| panic!("arachne live plan construction failed: {error}"));
    let classic = slicer_runtime::run::prepare_prepass_context(
        Arc::clone(&inputs.mesh),
        classic_source,
        &module_dirs,
        true,
        false,
    )
    .unwrap_or_else(|error| panic!("classic live plan construction failed: {error}"));
    assert!(
        !module_ids(&arachne).is_empty(),
        "arachne live plan must contain modules"
    );
    assert!(
        !module_ids(&classic).is_empty(),
        "classic live plan must contain modules"
    );
    LivePlans { arachne, classic }
}

fn compiled_modules(context: &slicer_runtime::run::PrepassContext) -> Vec<&CompiledModuleStatic> {
    let mut modules = Vec::new();
    for stage in &context.plan.prepass_stages {
        modules.extend(stage.modules.iter());
    }
    for stage in &context.plan.per_layer_stages {
        modules.extend(stage.modules.iter());
    }
    if let Some(stage) = context.plan.layer_finalization_stage.as_ref() {
        modules.extend(stage.modules.iter());
    }
    for stage in &context.plan.postpass_stages {
        modules.extend(stage.modules.iter());
    }
    modules
}

fn module_ids(context: &slicer_runtime::run::PrepassContext) -> BTreeSet<String> {
    compiled_modules(context)
        .into_iter()
        .map(|module| module.module_id().to_string())
        .collect()
}

fn classic_perimeter_owner(inputs: &OracleInputs, plans: &LivePlans) -> String {
    let arachne_ids = module_ids(&plans.arachne);
    let classic_ids = module_ids(&plans.classic);
    let candidates: Vec<String> = inputs
        .modules
        .iter()
        .filter(|module| {
            module
                .claims()
                .iter()
                .any(|claim| claim == "perimeter-generator")
        })
        .map(|module| module.id().to_owned())
        .filter(|id| classic_ids.contains(id) && !arachne_ids.contains(id))
        .collect();
    assert_eq!(candidates, vec!["com.core.classic-perimeters".to_owned()]);
    candidates[0].clone()
}

fn derive_expected_value(raw: &str, entry: &RegistryEntry) -> Result<ConfigValue, String> {
    if let Some(values) = entry.values.as_ref() {
        if !values.iter().any(|value| value == raw) {
            return Err(format!("enum value outside registry domain: {raw:?}"));
        }
        if entry.field_type == "enum" || entry.field_type == "string" {
            return Ok(ConfigValue::String(raw.to_owned()));
        }
    }

    match entry.field_type.as_str() {
        "bool" => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" => Ok(ConfigValue::Bool(true)),
            "false" | "0" => Ok(ConfigValue::Bool(false)),
            _ => Err(format!("invalid bool: {raw:?}")),
        },
        "int" => raw
            .parse::<i64>()
            .map(ConfigValue::Int)
            .map_err(|_| format!("invalid int: {raw:?}")),
        "float" => raw
            .parse::<f64>()
            .map(|value| ConfigValue::Float(if value.is_subnormal() { 0.0 } else { value }))
            .map_err(|_| format!("invalid float: {raw:?}")),
        "string" => Ok(ConfigValue::String(raw.to_owned())),
        "enum" => Err(format!("enum declaration has no values: {raw:?}")),
        "percent" => raw
            .strip_suffix('%')
            .unwrap_or(raw)
            .trim()
            .parse::<f64>()
            .map(ConfigValue::Percent)
            .map_err(|_| format!("invalid percent: {raw:?}")),
        "float_or_percent" | "float-or-percent" => {
            let (number, is_percent) = raw
                .strip_suffix('%')
                .map_or((raw, false), |value| (value.trim(), true));
            number
                .parse::<f64>()
                .map(|value| ConfigValue::FloatOrPercent {
                    value: if value.is_subnormal() { 0.0 } else { value },
                    is_percent,
                })
                .map_err(|_| format!("invalid float-or-percent: {raw:?}"))
        }
        other => Err(format!("unsupported registry field type: {other:?}")),
    }
}

fn automatically_excluded(raw: &str, entry: &RegistryEntry) -> bool {
    if entry.field_type == "percent" {
        return true;
    }
    if entry.field_type == "float_or_percent" && raw.trim_end().ends_with('%') {
        return true;
    }
    if entry.base_key.is_some() && raw.parse::<f64>().is_ok_and(|value| value == 0.0) {
        return true;
    }
    entry.field_type == "int" && raw.parse::<i64>().is_ok_and(|value| value == -1)
}

fn derive_population(
    authored: &BTreeMap<String, AuthoredValue>,
    registry: &ConfigSchemaRegistry,
    ownership: &BTreeMap<String, Vec<String>>,
    narrower_scope_keys: &BTreeSet<String>,
) -> Vec<Observation> {
    let mut population = Vec::new();
    for (key, authored_value) in authored {
        if !authored_value.is_authored_string() || narrower_scope_keys.contains(key) {
            continue;
        }
        let Some(raw) = authored_value.raw.as_deref() else {
            continue;
        };
        let Some(entry) = registry.entry(key) else {
            continue;
        };
        if automatically_excluded(raw, entry) {
            continue;
        }
        let Ok(expected) = derive_expected_value(raw, entry) else {
            continue;
        };
        let Some(owners) = ownership.get(key) else {
            continue;
        };
        for module_id in owners {
            population.push(Observation {
                key: key.clone(),
                module_id: module_id.clone(),
                authored: raw.to_owned(),
                expected: expected.clone(),
            });
        }
    }
    population.sort_by(|left, right| {
        left.key
            .cmp(&right.key)
            .then_with(|| left.module_id.cmp(&right.module_id))
    });
    population
}

fn observations_have(population: &[Observation], key: &str, raw: &str) -> bool {
    population
        .iter()
        .any(|observation| observation.key == key && observation.authored == raw)
}

fn values_match(expected: Option<&ConfigValue>, delivered: Option<&ConfigValue>) -> bool {
    matches!((expected, delivered), (Some(left), Some(right)) if left == right)
}

#[test]
fn oracle_authored_values_reach_owning_module_config_views() {
    let inputs = oracle_inputs();
    let plans = live_plans(&inputs);
    let classic_owner = classic_perimeter_owner(&inputs, &plans);
    let population = derive_population(
        &inputs.authored,
        &inputs.registry,
        &inputs.ownership,
        &inputs.object_scope_keys,
    );
    assert!(
        !population.is_empty(),
        "oracle observation population must be non-empty"
    );

    let mut missing = Vec::new();
    let mut mismatches = Vec::new();
    for observation in &population {
        let selector = if observation.module_id == classic_owner {
            "classic"
        } else {
            "arachne"
        };
        let context = if selector == "classic" {
            &plans.classic
        } else {
            &plans.arachne
        };
        let Some(compiled) = compiled_modules(context)
            .into_iter()
            .find(|module| module.module_id() == &observation.module_id)
        else {
            missing.push(format!(
                "MISSING_MODULE key={} expected_module={} selector={selector}",
                observation.key, observation.module_id
            ));
            continue;
        };
        let delivered = compiled.config_view().get(&observation.key);
        if !values_match(Some(&observation.expected), delivered) {
            let delivered =
                delivered.map_or_else(|| "None".to_owned(), |value| format!("{value:?}"));
            mismatches.push(format!(
                "MISMATCH key={} module={} authored={:?} expected={:?} delivered={delivered}",
                observation.key, observation.module_id, observation.authored, observation.expected
            ));
        }
    }

    missing.sort();
    if !missing.is_empty() {
        panic!("{}", missing.join("\n"));
    }
    mismatches.sort();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn oracle_selector_matrix_exposes_each_perimeter_owner() {
    let inputs = oracle_inputs();
    let plans = live_plans(&inputs);
    let arachne_ids = module_ids(&plans.arachne);
    let classic_ids = module_ids(&plans.classic);

    assert!(arachne_ids.contains("com.core.arachne-perimeters"));
    assert!(!arachne_ids.contains("com.core.classic-perimeters"));
    assert!(classic_ids.contains("com.core.classic-perimeters"));
    assert!(!classic_ids.contains("com.core.arachne-perimeters"));
    for module_id in &inputs.support_family_modules {
        assert!(
            arachne_ids.contains(module_id),
            "support-family module absent from arachne plan: {module_id}"
        );
        assert!(
            classic_ids.contains(module_id),
            "support-family module absent from classic plan: {module_id}"
        );
    }
}

#[test]
fn oracle_population_derivation_controls() {
    let inputs = oracle_inputs();
    let population = derive_population(
        &inputs.authored,
        &inputs.registry,
        &inputs.ownership,
        &inputs.object_scope_keys,
    );
    assert!(
        !population.is_empty(),
        "oracle population must be non-empty"
    );

    assert!(
        inputs.registry.entry("spiral_mode").is_none(),
        "spiral_mode must remain undeclared in the registry"
    );
    assert!(!observations_have(&population, "spiral_mode", "0"));
    assert!(!observations_have(&population, "bridge_density", "100%"));

    let list_key = inputs
        .authored
        .iter()
        .find(|(_, value)| matches!(value.json, serde_json::Value::Array(_)))
        .map(|(key, _)| key.clone())
        .expect("fixture must contain a real non-scalar authored list");
    assert!(
        !population
            .iter()
            .any(|observation| observation.key == list_key),
        "real non-scalar list must stay outside the scalar oracle: {list_key}"
    );

    for (key, raw) in [
        ("line_width", "0.45"),
        ("skirt_height", "3"),
        ("enable_support", "0"),
        ("internal_bridge_angle", "0"),
        ("internal_bridge_flow", "1"),
    ] {
        assert!(
            observations_have(&population, key, raw),
            "eligible authored observation absent: {key}={raw:?}"
        );
    }
    for key in ["internal_bridge_angle", "internal_bridge_flow"] {
        let entry = inputs
            .registry
            .entry(key)
            .unwrap_or_else(|| panic!("registry missing default-coincident key: {key}"));
        let raw = inputs
            .authored
            .get(key)
            .and_then(|value| value.raw.as_deref())
            .unwrap_or_else(|| panic!("fixture missing default-coincident key: {key}"));
        let default_raw = entry.default.as_deref().unwrap_or_else(|| {
            panic!("registry missing default for default-coincident key: {key}")
        });
        assert_eq!(
            derive_expected_value(default_raw, entry).expect("typed registry default"),
            derive_expected_value(raw, entry).expect("typed authored default"),
            "authored value must coincide with the typed registry default: {key}"
        );
    }

    let mut synthetic_narrower_scope = inputs.object_scope_keys.clone();
    synthetic_narrower_scope.insert("line_width".to_owned());
    let narrowed_population = derive_population(
        &inputs.authored,
        &inputs.registry,
        &inputs.ownership,
        &synthetic_narrower_scope,
    );
    assert!(!observations_have(
        &narrowed_population,
        "line_width",
        "0.45"
    ));

    let mut synthetic_sentinel = inputs.authored.clone();
    synthetic_sentinel.insert("skirt_height".to_owned(), AuthoredValue::string("-1"));
    let sentinel_population = derive_population(
        &synthetic_sentinel,
        &inputs.registry,
        &inputs.ownership,
        &inputs.object_scope_keys,
    );
    assert!(!observations_have(
        &sentinel_population,
        "skirt_height",
        "-1"
    ));

    let base_key = inputs
        .registry
        .keys()
        .filter(|key| {
            inputs
                .registry
                .entry(key)
                .is_some_and(|entry| entry.base_key.is_some())
        })
        .find(|key| !inputs.object_scope_keys.contains(*key))
        .map(str::to_owned)
        .expect("registry must expose a synthetic base-key candidate");
    let mut synthetic_base_zero = inputs.authored.clone();
    synthetic_base_zero.insert(base_key.clone(), AuthoredValue::string("0"));
    let base_zero_population = derive_population(
        &synthetic_base_zero,
        &inputs.registry,
        &inputs.ownership,
        &inputs.object_scope_keys,
    );
    assert!(
        !base_zero_population
            .iter()
            .any(|observation| observation.key == base_key),
        "synthetic base-key zero must be excluded: {base_key}"
    );
}

// test-quality: negative control — deliberate known-unequal variant pairs pin the shared `values_match` comparator; this is not a pipeline oracle
#[test]
fn oracle_comparator_is_type_aware_negative_control() {
    let inputs = oracle_inputs();
    let float_entry = inputs
        .registry
        .keys()
        .find_map(|key| {
            inputs
                .registry
                .entry(key)
                .filter(|entry| entry.field_type == "float")
        })
        .expect("registry must expose a float declaration");
    let int_entry = inputs
        .registry
        .keys()
        .find_map(|key| {
            inputs
                .registry
                .entry(key)
                .filter(|entry| entry.field_type == "int")
        })
        .expect("registry must expose an int declaration");
    let bool_entry = inputs
        .registry
        .keys()
        .find_map(|key| {
            inputs
                .registry
                .entry(key)
                .filter(|entry| entry.field_type == "bool")
        })
        .expect("registry must expose a bool declaration");

    let expected_float = derive_expected_value("0", float_entry).expect("float derivation");
    let expected_int = derive_expected_value("1", int_entry).expect("int derivation");
    let expected_bool = derive_expected_value("0", bool_entry).expect("bool derivation");
    assert!(!values_match(
        Some(&expected_float),
        Some(&ConfigValue::Bool(false))
    ));
    assert!(!values_match(
        Some(&expected_int),
        Some(&ConfigValue::Bool(true))
    ));
    assert!(values_match(
        Some(&expected_float),
        Some(&ConfigValue::Float(0.0))
    ));
    assert!(values_match(
        Some(&expected_bool),
        Some(&ConfigValue::Bool(false))
    ));
}
