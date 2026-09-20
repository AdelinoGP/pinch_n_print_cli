//! Resolved-config view must not drop keys: the CONFIG_BLOCK emitted into the
//! g-code is a projection of the assembled config registry — nothing dropped,
//! nothing invented — even when the caller's config file carries undeclared
//! keys (which must not leak into the block).
//!
//! Step 6a (no-drop e2e in retained mode, AC-5): the full-registry synthesized
//! population must ingest through the real `run_slice` path with zero
//! `UnrecognizedKey` warnings surfaced on `SliceOutcome.ingestion_warnings`,
//! every owning module's bound view must receive its declared key, and a
//! negative control (one withheld declaration) must make the same oracle
//! report exactly that key (docs/22 §2.4, §2.6, §4).

#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use crate::common::slicer_cache::{
    fixture_stl, module_dir_paths, run_pnp_cli_uncached, ModuleDirKind,
};
use slicer_config::{
    assemble_registry, ConfigIngestor, ConfigSchemaRegistry, HostChannels, IngestionWarning,
    ModuleDeclaration, RegistryEntry,
};
use slicer_runtime::execution_plan::parse_cli_config_source;
use slicer_runtime::{
    load_modules_from_roots, prepare_prepass_context, run_slice, LoadedModule, SliceRunOptions,
};
use slicer_scheduler::execution_plan::bind_module_config_view;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/slicer-runtime
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn core_modules_dir() -> PathBuf {
    repo_root().join("modules/core-modules")
}

/// Load the live core modules and assemble the registry from their manifests
/// plus the live host channels — the same derivation `run_slice` performs
/// internally (integrated modules declare no config keys, so the manifest-only
/// assembly matches the runtime registry).
fn live_registry() -> (
    Vec<LoadedModule>,
    Vec<ModuleDeclaration>,
    ConfigSchemaRegistry,
) {
    let modules: Vec<LoadedModule> =
        load_modules_from_roots(std::slice::from_ref(&core_modules_dir()))
            .unwrap_or_else(|error| panic!("load core module schemas failed: {error:?}"))
            .modules;
    let declarations: Vec<ModuleDeclaration> = modules
        .iter()
        .map(|module| ModuleDeclaration {
            module_id: module.id().to_owned(),
            schema: module.config_schema().clone(),
            claim_exclusive_group: None,
        })
        .collect();
    let registry: ConfigSchemaRegistry =
        assemble_registry(&declarations, &HostChannels::from_live())
            .unwrap_or_else(|error| panic!("assemble registry from live schemas failed: {error}"))
            .registry;
    (modules, declarations, registry)
}

/// Derive the flat wire value for one registry entry: the first of these that
/// exists — the declared default rendered to wire form; the first declared
/// enum value; the declared `min`; the declared `max`; or a type-neutral value
/// (`false`, `0`, `""`). AC-5 derivation rule; never a hand-listed roster.
fn wire_value_for(entry: &RegistryEntry) -> String {
    if let Some(default) = &entry.default {
        return default.clone();
    }
    if let Some(first_enum) = entry.values.as_ref().and_then(|values| values.first()) {
        return first_enum.clone();
    }
    if let Some(min) = entry.min {
        return format!("{min}");
    }
    if let Some(max) = entry.max {
        return format!("{max}");
    }
    match entry.field_type.as_str() {
        "bool" => "false",
        "int" | "float" | "percent" | "float_or_percent" => "0",
        _ => "",
    }
    .to_string()
}

/// Render a wire value as the JSON shape `parse_cli_config_source` consumes,
/// guided by the declared field type so numbers stay numbers and enum /
/// string / percent values stay strings.
fn wire_to_json(field_type: &str, wire: &str) -> serde_json::Value {
    match field_type {
        "bool" => serde_json::Value::Bool(wire == "true" || wire == "1"),
        "int" => wire
            .trim()
            .parse::<i64>()
            .map(|value| serde_json::Value::Number(value.into()))
            .unwrap_or_else(|_| serde_json::Value::String(wire.to_owned())),
        "float" => numeric_or_string(wire),
        "float_or_percent" | "percent" => {
            if wire.ends_with('%') {
                serde_json::Value::String(wire.to_owned())
            } else {
                numeric_or_string(wire)
            }
        }
        // List field types are wired as JSON arrays: `ConfigIngestor`'s
        // `coerce_list` accepts `ConfigValue::List` only (a comma-joined
        // string is retained untyped), and the typed `ResolvedConfig`
        // extractors (`extract_float_list` / `extract_int_list` /
        // `extract_string_list`) reject scalar values.
        "float-list" | "int-list" | "string-list" => list_to_json(field_type, wire),
        _ => serde_json::Value::String(wire.to_owned()),
    }
}

/// Render a list wire value (comma-joined tokens — the `HostWireField`
/// rendering, e.g. `bed_shape`'s `0,0,250,0,250,250,0,250`) as a JSON array.
/// A token that cannot be parsed by the element type falls back to the
/// type-neutral single-element list (`[0]` / `[""]`), so every exact registry
/// key still receives one value (AC-5 derivation rule).
fn list_to_json(field_type: &str, wire: &str) -> serde_json::Value {
    let items: Vec<serde_json::Value> = wire
        .split(',')
        .filter_map(|token| match field_type {
            "float-list" => token.trim().parse::<f64>().ok().and_then(|value| {
                if value.is_finite() && value.fract() == 0.0 && value.abs() <= i64::MAX as f64 {
                    Some(serde_json::Value::Number((value as i64).into()))
                } else {
                    serde_json::Number::from_f64(value).map(serde_json::Value::Number)
                }
            }),
            "int-list" => token
                .trim()
                .parse::<i64>()
                .ok()
                .map(|value| serde_json::Value::Number(value.into())),
            _ => Some(serde_json::Value::String(token.trim().to_owned())),
        })
        .collect();
    if items.is_empty() {
        return serde_json::Value::Array(vec![match field_type {
            "string-list" => serde_json::Value::String(String::new()),
            _ => serde_json::Value::Number(serde_json::Number::from(0)),
        }]);
    }
    serde_json::Value::Array(items)
}

/// Render a numeric wire value as a JSON number when possible, else a string.
fn numeric_or_string(wire: &str) -> serde_json::Value {
    match wire.trim().parse::<f64>() {
        Ok(value)
            if value.is_finite() && value.fract() == 0.0 && value.abs() <= i64::MAX as f64 =>
        {
            serde_json::Value::Number((value as i64).into())
        }
        Ok(value) => serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(wire.to_owned())),
        Err(_) => serde_json::Value::String(wire.to_owned()),
    }
}

/// Synthesize the flat config population: one value per exact registry key,
/// derived by registry iteration (AC-5). Wildcard pattern entries (`prefix:*`)
/// are patterns, not exact keys, so they are skipped — the ingestor recognizes
/// exact authored keys through them.
fn synthesize_population(registry: &ConfigSchemaRegistry) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for key in registry.keys() {
        if key.ends_with(":*") {
            continue;
        }
        let entry = registry
            .entry(key)
            .expect("registry keys must resolve to entries");
        let wire = wire_value_for(entry);
        object.insert(key.to_owned(), wire_to_json(&entry.field_type, &wire));
    }
    serde_json::Value::Object(object)
}

#[test]
fn run_slice_config_block_is_registry_projection() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let output = tmp.path().join("wedge.gcode");
    let config_path = tmp.path().join("resolved_config.json");
    // `my_undeclared_probe_key` is deliberately not part of any registry
    // declaration; it must not leak into the CONFIG_BLOCK projection.
    std::fs::write(
        &config_path,
        r#"{"gcode_flavor": "klipper", "my_undeclared_probe_key": 1}"#,
    )
    .expect("write temp config");
    let run = run_pnp_cli_uncached(
        &fixture_stl(),
        &module_dir_paths(&ModuleDirKind::CoreModules),
        &output,
        Some(&config_path),
    );
    assert!(
        run.status.success(),
        "pnp_cli must succeed with an undeclared config key. Stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let gcode = std::fs::read_to_string(&output).expect("default wedge g-code output");

    // ── CONFIG_BLOCK key set ────────────────────────────────────────────────
    // Parse loop copied from the canary (`slice_end_to_end_tdd.rs`).
    let config_start = gcode
        .find("; CONFIG_BLOCK_START")
        .expect("CONFIG_BLOCK_START must be present");
    let config_end = gcode
        .find("; CONFIG_BLOCK_END")
        .expect("CONFIG_BLOCK_END must be present");
    assert!(
        config_start < config_end,
        "CONFIG_BLOCK_START must precede CONFIG_BLOCK_END"
    );
    // `object_height:<uuid>` entries are per-object bookkeeping injected by the
    // caller, not part of the config surface, and their UUID varies per run.
    let mut keys: Vec<&str> = gcode[config_start..config_end]
        .lines()
        .filter_map(|line| line.strip_prefix("; "))
        .filter_map(|rest| rest.split_once(" = "))
        .map(|(key, _)| key)
        .filter(|key| !key.contains(':'))
        .collect();
    keys.sort_unstable();
    keys.dedup();
    let block_lines: Vec<&str> = gcode[config_start..config_end].lines().collect();

    // ── Registry assembly ───────────────────────────────────────────────────
    // Copied from the canary: the registry reconciles host + module
    // declarations, so the block must be a projection of it.
    let modules: Vec<LoadedModule> =
        load_modules_from_roots(std::slice::from_ref(&core_modules_dir()))
            .unwrap_or_else(|error| panic!("load core module schemas failed: {error:?}"))
            .modules;
    let declarations: Vec<ModuleDeclaration> = modules
        .iter()
        .map(|module| ModuleDeclaration {
            module_id: module.id().to_owned(),
            schema: module.config_schema().clone(),
            claim_exclusive_group: None,
        })
        .collect();
    let registry: ConfigSchemaRegistry =
        assemble_registry(&declarations, &HostChannels::from_live())
            .unwrap_or_else(|error| panic!("assemble registry from live schemas failed: {error}"))
            .registry;

    // ── Derived pick: a key only a module manifest can have declared ────────
    // Host-declared `plain`-typed keys carry no registry default and are
    // emitted explicitly; they form the padding set below. Any remaining key
    // with a declared default was projected from a module manifest, so the
    // pick is DERIVED from the registry — never a hardcoded key name.
    let padding: [&str; 5] = [
        "filament_diameter",
        "filament_colour",
        "extruder_colour",
        "printer_model",
        "gcode_flavor",
    ];
    let picked: Option<String> = registry
        .keys()
        .filter(|key| !padding.contains(key))
        .find_map(|key| {
            let entry: &RegistryEntry = registry
                .entry(key)
                .expect("registry keys must resolve to entries");
            (!entry.omit_from_config_block && entry.default.is_some()).then(|| key.to_string())
        });
    let picked = picked.expect(
        "at least one module-manifest key with a declared default must be \
         projected into the CONFIG_BLOCK",
    );
    let default = registry
        .entry(&picked)
        .expect("picked key must resolve to a registry entry")
        .default
        .as_ref()
        .expect("picked key must declare a default");
    let expected_line = format!("; {picked} = {default}");
    let occurrences = block_lines
        .iter()
        .filter(|line| **line == expected_line.as_str())
        .count();
    assert_eq!(
        occurrences, 1,
        "module-declared key `{picked}` must be emitted exactly once as `{expected_line}`"
    );

    // ── Host key resolved from the caller's config, present exactly once ────
    let flavor_lines = block_lines
        .iter()
        .filter(|line| **line == "; gcode_flavor = klipper")
        .count();
    assert_eq!(
        flavor_lines, 1,
        "`; gcode_flavor = klipper` must be emitted exactly once"
    );

    // ── Banned keys are not part of the registry projection ─────────────────
    for banned in [
        "mmu_segmented_region_start",
        "mmu_segmented_region_end",
        "mmu_segmented_region_count",
        "thumbnail_path",
        "my_undeclared_probe_key",
    ] {
        assert!(
            !keys.contains(&banned),
            "CONFIG_BLOCK must not contain `; {banned} = ...`; the block is a \
             projection of the assembled registry"
        );
    }
}

#[test]
fn cube_and_full_registry_config_have_zero_unrecognized_keys() {
    // Fixture absence is a hard failure, not a skip (AC-5).
    let fixture = repo_root().join("resources/cube_4color.3mf");
    assert!(
        fixture.exists(),
        "fixture missing: {} — restore resources/cube_4color.3mf",
        fixture.display()
    );
    let core = core_modules_dir();
    assert!(core.exists(), "core-modules directory missing: {core:?}");

    let (modules, _declarations, registry) = live_registry();
    assert!(!registry.is_empty(), "live registry must not be empty");

    // Synthesized flat config: one value per exact registry key.
    let population = synthesize_population(&registry);
    let json = population.to_string();
    let tmp = tempfile::tempdir().expect("tempdir");
    let config_path = tmp.path().join("full_registry.json");
    std::fs::write(&config_path, &json).expect("write synthesized population config");

    // The real run_slice path ingests and resolves both inputs.
    let mesh = Arc::new(
        slicer_model_io::load_model(&fixture)
            .unwrap_or_else(|e| panic!("load_model({fixture:?}) failed: {e}")),
    );
    let opts = SliceRunOptions {
        mesh: Arc::clone(&mesh),
        model_label: fixture.to_string_lossy().into_owned(),
        config_path: Some(config_path.clone()),
        module_dirs: vec![core.clone()],
        no_default_module_paths: true,
        ..Default::default()
    };
    let outcome = run_slice(opts)
        .unwrap_or_else(|e| panic!("run_slice with synthesized full-registry config failed: {e}"));

    let unrecognized: Vec<&IngestionWarning> = outcome
        .ingestion_warnings
        .iter()
        .filter(|warning| matches!(warning, IngestionWarning::UnrecognizedKey { .. }))
        .collect();
    assert!(
        unrecognized.is_empty(),
        "registry-derived population must ingest with zero UnrecognizedKey \
         warnings, got {unrecognized:?}"
    );

    // Every owning module's view receives its declared key: resolve through
    // the shared public prefix (prepare_prepass_context resolves exactly the
    // default resolved config run_slice hands its prepass) and bind each
    // live module's view from it.
    let config_source = parse_cli_config_source(&json)
        .unwrap_or_else(|e| panic!("synthesized population must parse as CLI config: {e}"));
    let prepass = prepare_prepass_context(Arc::clone(&mesh), config_source, &[core], true, false)
        .unwrap_or_else(|e| {
            panic!("prepare_prepass_context on synthesized population failed: {e}")
        });
    let resolved_source = prepass.default_resolved_config.to_config_map();
    for module in &modules {
        let view = bind_module_config_view(module, &prepass.default_resolved_config);
        for declared in module.config_schema().entries.keys() {
            if let Some(prefix) = declared.strip_suffix(":*") {
                // Wildcard pattern: every matching source key must reach the view.
                for src_key in resolved_source.keys() {
                    if src_key
                        .strip_prefix(prefix)
                        .is_some_and(|rest| rest.starts_with(':'))
                    {
                        assert!(
                            view.get(src_key).is_some(),
                            "module {} declares {declared}; its bound view must carry \
                             the matching source key {src_key}",
                            module.id()
                        );
                    }
                }
            } else {
                assert!(
                    view.get(declared).is_some(),
                    "module {} declares {declared}; its bound view must receive it",
                    module.id()
                );
            }
        }
    }
}

#[test]
fn no_drop_oracle_detects_a_withheld_declaration() {
    let (_, declarations, registry) = live_registry();
    let population = synthesize_population(&registry);
    let config_source = parse_cli_config_source(&population.to_string())
        .unwrap_or_else(|e| panic!("synthesized population must parse as CLI config: {e}"));

    // Registry-derived pick: an exact key whose ONLY declaration is one module
    // manifest and that no `prefix:*` wildcard pattern covers — withholding
    // that one declaration must make exactly this key unrecognized.
    let victim: String = registry
        .keys()
        .find(|key| {
            !key.ends_with(":*") && {
                let entry = registry.entry(key).expect("keys must resolve to entries");
                entry.provenance.len() == 1
                    && declarations.iter().any(|declaration| {
                        declaration.module_id == entry.provenance[0]
                            && declaration.schema.entries.contains_key(*key)
                    })
                    && !registry.keys().any(|pattern| {
                        pattern.strip_suffix(":*").is_some_and(|prefix| {
                            key.strip_prefix(prefix)
                                .is_some_and(|rest| rest.starts_with(':'))
                        })
                    })
            }
        })
        .expect("live registry must contain a module-solely-declared exact key")
        .to_string();

    // Withhold exactly the victim's declaration: drop the key from its sole
    // declaring module's schema clone and reassemble the registry.
    let mut withheld = declarations.clone();
    let sole_declaration = withheld
        .iter_mut()
        .find(|declaration| declaration.schema.entries.contains_key(&victim))
        .unwrap_or_else(|| panic!("victim {victim} must be declared by a module"));
    sole_declaration.schema.entries.remove(&victim);
    let reduced = assemble_registry(&withheld, &HostChannels::from_live())
        .unwrap_or_else(|e| panic!("assemble reduced registry failed: {e}"))
        .registry;
    assert!(
        !reduced.keys().any(|key| key == victim),
        "withheld declaration must remove {victim} from the registry"
    );

    // Same oracle run_slice uses (ConfigIngestor::tolerant, warn-and-drop):
    // the full population against the reduced registry must report exactly
    // the withheld key once — never vacuously (docs/22 §2.4, §2.6).
    let mut ingestor = ConfigIngestor::tolerant(&reduced);
    ingestor
        .ingest_flat(&config_source)
        .expect("full population must decode against the reduced registry");
    let warnings = ingestor.finish().warnings;
    let unrecognized: Vec<&IngestionWarning> = warnings
        .iter()
        .filter(|warning| matches!(warning, IngestionWarning::UnrecognizedKey { .. }))
        .collect();
    assert_eq!(
        unrecognized.len(),
        1,
        "withholding exactly one declaration must yield exactly one \
         UnrecognizedKey, got {warnings:?}"
    );
    match unrecognized[0] {
        IngestionWarning::UnrecognizedKey { wire_key, key, .. } => {
            assert_eq!(wire_key, &victim, "wire key must be the withheld key");
            assert_eq!(key, &victim, "canonical key must be the withheld key");
        }
        _ => unreachable!("filtered to UnrecognizedKey only"),
    }
}
