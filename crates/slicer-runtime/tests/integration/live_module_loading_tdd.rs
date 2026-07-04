//! TASK: live-path module loading wiring.
//!
//! Verifies that `load_live_modules_for_plan` + `build_live_execution_plan`
//! (the helpers used by `main.rs`) together deliver real module bindings
//! with declared-read-filtered `Arc<ConfigView>` values to the compiled
//! modules on the production entry path (docs/03 Â§host-boundary
//! enforcement; docs/04 Â§Fixed Stage Order; docs/02 Â§pre-filtered config).

#![allow(missing_docs)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use slicer_ir::{ConfigValue, RegionKey, RegionPlan};
use slicer_model_io::load_model;
use slicer_runtime::{
    build_live_execution_plan, load_live_modules_for_plan, parse_cli_config_source,
    ExecutionPlanError, LoadDiagnostic, STAGE_ORDER,
};
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn write_module(root: &Path, stem: &str, manifest: &str) {
    fs::write(root.join(format!("{stem}.toml")), manifest).expect("write manifest");
    fs::write(root.join(format!("{stem}.wasm")), b"placeholder wasm").expect("write wasm");
}

fn manifest(
    id: &str,
    stage: &str,
    wit_world: &str,
    config_keys: &[(&str, &str)],
    requires_modules: &[&str],
    ir_reads: &[&str],
    ir_writes: &[&str],
) -> String {
    let mut schema = String::new();
    for (key, ty) in config_keys {
        schema.push_str(&format!(
            "\n  [config.schema.{key}]\n  type = \"{ty}\"\n  default = 0.0\n"
        ));
    }
    let requires = requires_modules
        .iter()
        .map(|m| format!("\"{m}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let reads = ir_reads
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let writes = ir_writes
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"
[module]
id = "{id}"
version = "1.0.0"
display-name = "Fixture"
description = "fixture"
author = "test"
license = "MIT"
homepage = "https://example.invalid/{id}"
wit-world = "{wit_world}"

[stage]
id = "{stage}"

[ir-access]
reads = [{reads}]
writes = [{writes}]

[claims]
holds = []
requires = []

[compatibility]
incompatible-with = []
requires = [{requires}]
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "2.0.0"

[config.schema]
{schema}

[config.overridable-per-region]
keys = []

[config.overridable-per-layer]
keys = []

[hints]
layer-parallel-safe = false
"#
    )
}

fn infill_manifest(id: &str, requires: &[&str]) -> String {
    manifest(
        id,
        "Layer::Infill",
        "slicer:world-layer@1.0.0",
        &[("density", "float"), ("pattern", "enum")],
        requires,
        &["SliceIR.regions.infill_areas"],
        &["InfillIR.regions.sparse_infill"],
    )
}

fn prepass_manifest(id: &str) -> String {
    manifest(
        id,
        "PrePass::MeshAnalysis",
        "slicer:world-prepass@1.0.0",
        &[("threshold", "float")],
        &[],
        &[],
        &["SurfaceClassificationIR"],
    )
}

#[test]
fn load_live_modules_for_plan_discovers_manifests_from_module_dir() {
    let dir = TempDir::new().unwrap();
    write_module(dir.path(), "a", &infill_manifest("com.example.a", &[]));
    write_module(dir.path(), "b", &prepass_manifest("com.example.b"));

    let out = load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 2)
        .expect("load");
    let ids: Vec<String> = out
        .bindings
        .iter()
        .map(|b| b.module.id().to_string())
        .collect();
    assert!(ids.contains(&"com.example.a".to_string()));
    assert!(ids.contains(&"com.example.b".to_string()));
}

#[test]
fn load_live_modules_for_plan_emits_stages_in_canonical_stage_order() {
    let dir = TempDir::new().unwrap();
    // Deliberately write stages out of canonical order on disk so we can
    // assert STAGE_ORDER-sorting in the output.
    write_module(
        dir.path(),
        "infill",
        &infill_manifest("com.example.infill", &[]),
    );
    write_module(
        dir.path(),
        "prepass",
        &prepass_manifest("com.example.prepass"),
    );

    let out =
        load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 1).unwrap();

    let stages: Vec<&str> = out
        .sorted_stages
        .iter()
        .map(|s| s.stage_id.as_str())
        .collect();
    let prepass_idx = stages
        .iter()
        .position(|s| *s == "PrePass::MeshAnalysis")
        .unwrap();
    let layer_idx = stages.iter().position(|s| *s == "Layer::Infill").unwrap();
    assert!(
        prepass_idx < layer_idx,
        "prepass stages must precede layer stages per STAGE_ORDER"
    );
    // Each emitted stage is also present in the canonical STAGE_ORDER table.
    for s in &stages {
        assert!(STAGE_ORDER.iter().any(|known| known == s));
    }
}

#[test]
fn load_live_modules_for_plan_topologically_sorts_modules_within_a_stage() {
    let dir = TempDir::new().unwrap();
    // Module "b" requires "a"; topological order must place a before b.
    write_module(
        dir.path(),
        "b",
        &infill_manifest("com.example.b", &["com.example.a"]),
    );
    write_module(dir.path(), "a", &infill_manifest("com.example.a", &[]));

    let out =
        load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 1).unwrap();
    let stage = out
        .sorted_stages
        .iter()
        .find(|s| s.stage_id == "Layer::Infill")
        .unwrap();
    let a_idx = stage
        .module_ids
        .iter()
        .position(|m| m == "com.example.a")
        .unwrap();
    let b_idx = stage
        .module_ids
        .iter()
        .position(|m| m == "com.example.b")
        .unwrap();
    assert!(a_idx < b_idx, "dependency ordering must be deterministic");
}

#[test]
fn live_plan_assigns_declared_read_filtered_config_view_to_every_module() {
    let dir = TempDir::new().unwrap();
    write_module(dir.path(), "m", &infill_manifest("com.example.infill", &[]));

    let out =
        load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 1).unwrap();

    // Raw source carries declared keys AND unrelated noise; the bound view
    // must not expose anything undeclared to the compiled module.
    let mut source = HashMap::new();
    source.insert("density".to_string(), ConfigValue::Float(0.35));
    source.insert("pattern".to_string(), ConfigValue::String("gyroid".into()));
    source.insert("secret".to_string(), ConfigValue::String("leak".into()));

    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let plan = build_live_execution_plan(
        out.sorted_stages,
        out.bindings,
        &source,
        Arc::new(Vec::new()),
        Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
        &mut diagnostics,
    )
    .expect("plan must build on the live path");

    let infill_stage = plan
        .per_layer_stages
        .iter()
        .find(|s| s.stage_id == "Layer::Infill")
        .expect("plan must contain Layer::Infill");
    let module = &infill_stage.modules[0];
    let mut keys = module.config_view().keys();
    keys.sort();
    assert_eq!(keys, vec!["density".to_string(), "pattern".to_string()]);
    assert!(!module.config_view().contains_key("secret"));
    assert_eq!(module.config_view().get_float("density"), Some(0.35));
}

#[test]
fn live_plan_end_to_end_with_cli_config_json_respects_declared_reads() {
    let dir = TempDir::new().unwrap();
    write_module(dir.path(), "m", &infill_manifest("com.example.infill", &[]));

    let out =
        load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 1).unwrap();

    // Simulate a user `--config` JSON with one declared key and one leaked.
    let source = parse_cli_config_source(r#"{"density": 0.9, "extra": "nope"}"#).unwrap();
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let plan = build_live_execution_plan(
        out.sorted_stages,
        out.bindings,
        &source,
        Arc::new(Vec::new()),
        Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
        &mut diagnostics,
    )
    .unwrap();

    let infill_stage = plan
        .per_layer_stages
        .iter()
        .find(|s| s.stage_id == "Layer::Infill")
        .expect("plan must contain Layer::Infill");
    let module = &infill_stage.modules[0];
    assert!(module.config_view().contains_key("density"));
    assert!(!module.config_view().contains_key("extra"));
    assert_eq!(module.config_view().get_float("density"), Some(0.9));
}

#[test]
fn live_plan_is_deterministic_across_repeated_loads() {
    let dir = TempDir::new().unwrap();
    write_module(
        dir.path(),
        "b",
        &infill_manifest("com.example.b", &["com.example.a"]),
    );
    write_module(dir.path(), "a", &infill_manifest("com.example.a", &[]));

    let source = HashMap::new();
    let roots = [PathBuf::from(dir.path())];

    let run = || {
        let out = load_live_modules_for_plan(&roots, 1).unwrap();
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        build_live_execution_plan(
            out.sorted_stages,
            out.bindings,
            &source,
            Arc::new(Vec::new()),
            Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
            &mut diagnostics,
        )
        .unwrap()
    };
    let a = run();
    let b = run();
    let ids_a: Vec<String> = a
        .per_layer_stages
        .iter()
        .flat_map(|s| s.modules.iter().map(|m| m.module_id().to_string()))
        .collect();
    let ids_b: Vec<String> = b
        .per_layer_stages
        .iter()
        .flat_map(|s| s.modules.iter().map(|m| m.module_id().to_string()))
        .collect();
    assert_eq!(ids_a, ids_b, "module ordering must be deterministic");
}

#[test]
fn live_plan_preserves_seeded_planner_object_height_keys_for_real_core_modules() {
    let repo = repo_root();
    let model = repo.join("resources/regression_wedge.stl");
    let core_modules = repo.join("modules/core-modules");

    let mesh_ir = load_model(&model).expect("load real Benchy fixture");
    let object = mesh_ir
        .objects
        .first()
        .expect("Benchy fixture should load one object");
    let (z_min, z_max) = object
        .world_z_extent
        .expect("Benchy fixture should expose cached world_z_extent");

    let mut source = HashMap::new();
    let object_height_key = format!("object_height:{}", object.id);
    source.insert(
        object_height_key.clone(),
        ConfigValue::Float((z_max - z_min) as f64),
    );

    let out = crate::common::wasm_cache::cached_live_modules(&[core_modules], 1);
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let plan = build_live_execution_plan(
        out.sorted_stages.clone(),
        out.bindings.clone(),
        &source,
        Arc::new(Vec::new()),
        Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
        &mut diagnostics,
    )
    .expect("build live execution plan");

    let planner_stage = plan
        .prepass_stages
        .iter()
        .find(|stage| stage.stage_id == "PrePass::LayerPlanning")
        .expect("planner stage present");
    let planner_module = planner_stage
        .modules
        .iter()
        .find(|module| module.module_id() == "com.core.layer-planner-default")
        .expect("default planner module present");

    assert!(
        planner_module
            .config_view()
            .contains_key(&object_height_key),
        "real planner binding must preserve seeded wildcard-expanded key '{}'",
        object_height_key,
    );
    assert_eq!(
        planner_module.config_view().get_float(&object_height_key),
        Some((z_max - z_min) as f64),
        "real planner binding must expose the seeded object height",
    );
}

#[test]
fn live_plan_build_still_rejects_handrolled_undeclared_view_via_guardrail() {
    use slicer_ir::ConfigView;
    use slicer_runtime::{build_execution_plan, ExecutionModuleBinding, ExecutionPlanRequest};

    let dir = TempDir::new().unwrap();
    write_module(dir.path(), "m", &infill_manifest("com.example.infill", &[]));
    let out =
        load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 1).unwrap();

    // Bypass `build_live_execution_plan` â€” hand-roll an
    // `ExecutionModuleBinding` with a leaky ConfigView and pass it to
    // `build_execution_plan` directly. The guardrail must fire.
    let binding_src = out.bindings.into_iter().next().unwrap();
    let mut leaky = HashMap::new();
    leaky.insert("density".to_string(), ConfigValue::Float(0.2));
    leaky.insert("leaked".to_string(), ConfigValue::Bool(true));
    let request = ExecutionPlanRequest {
        sorted_stages: out.sorted_stages,
        module_bindings: vec![ExecutionModuleBinding {
            module: binding_src.module,
            config_view: Arc::new(ConfigView::from_map(leaky)),
        }],
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    };
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    match build_execution_plan(&request, &mut diagnostics).unwrap_err() {
        ExecutionPlanError::UndeclaredConfigKey { key, .. } => assert_eq!(key, "leaked"),
        other => panic!("expected UndeclaredConfigKey, got {other:?}"),
    }
}

// â”€â”€ Component compilation attachment â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

use slicer_runtime::manifest::DiagnosticLevel;

/// Write a manifest + an explicit byte payload at the given path.
fn write_module_with_wasm(root: &Path, stem: &str, manifest: &str, wasm_bytes: &[u8]) {
    fs::write(root.join(format!("{stem}.toml")), manifest).expect("write manifest");
    fs::write(root.join(format!("{stem}.wasm")), wasm_bytes).expect("write wasm");
}

fn minimal_component_bytes() -> Vec<u8> {
    // Smallest valid component-model artifact. Must be >8 bytes so
    // manifest ingestion doesn't classify it as a placeholder binary.
    let bytes = wat::parse_str("(component (core module))").expect("wat parse");
    assert!(
        bytes.len() > 8,
        "component binary must exceed placeholder threshold (actual {} bytes)",
        bytes.len()
    );
    bytes
}

#[test]
fn valid_component_binary_is_compiled_and_attached_to_live_binding() {
    let dir = TempDir::new().unwrap();
    write_module_with_wasm(
        dir.path(),
        "m",
        &infill_manifest("com.example.infill", &[]),
        &minimal_component_bytes(),
    );
    let out =
        load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 1).unwrap();
    let binding = out
        .bindings
        .iter()
        .find(|b| b.module.id() == "com.example.infill")
        .unwrap();
    assert!(
        binding.wasm_component.is_some(),
        "valid component must be compiled and attached"
    );
    // No per-module warning diagnostic on the happy path.
    assert!(out
        .diagnostics
        .iter()
        .all(|d| d.path != binding.module.wasm_path()
            || !matches!(d.level, DiagnosticLevel::Warning)));
}

#[test]
fn placeholder_wasm_is_skipped_with_structured_warning_diagnostic() {
    let dir = TempDir::new().unwrap();
    // File size <=8 bytes is flagged by manifest ingestion as placeholder.
    write_module_with_wasm(
        dir.path(),
        "m",
        &infill_manifest("com.example.placeholder", &[]),
        b"x",
    );
    let out =
        load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 1).unwrap();
    let binding = out
        .bindings
        .iter()
        .find(|b| b.module.id() == "com.example.placeholder")
        .unwrap();
    assert!(binding.module.placeholder_wasm());
    assert!(
        binding.wasm_component.is_none(),
        "placeholder binary must not produce a compiled component"
    );
    // The loader-side skip diagnostic is distinguished by
    // `field = Some("wasm_path")`; manifest ingestion emits its own
    // placeholder warning with `field = None`.
    let warn = out
        .diagnostics
        .iter()
        .find(|d| {
            d.path == binding.module.wasm_path()
                && matches!(d.level, DiagnosticLevel::Warning)
                && d.field.as_deref() == Some("wasm_path")
        })
        .expect("placeholder skip must emit a structured loader diagnostic");
    assert!(warn.message.contains("placeholder"));
}

#[test]
fn non_component_bytes_are_skipped_with_compile_failure_diagnostic() {
    let dir = TempDir::new().unwrap();
    // Not a placeholder (>8 bytes) but not a valid component binary either.
    write_module_with_wasm(
        dir.path(),
        "m",
        &infill_manifest("com.example.broken", &[]),
        b"this is definitely not a wasm component binary",
    );
    let out =
        load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 1).unwrap();
    let binding = out
        .bindings
        .iter()
        .find(|b| b.module.id() == "com.example.broken")
        .unwrap();
    assert!(!binding.module.placeholder_wasm());
    assert!(
        binding.wasm_component.is_none(),
        "invalid component bytes must not crash the loader but also must not attach"
    );
    let warn = out
        .diagnostics
        .iter()
        .find(|d| {
            d.path == binding.module.wasm_path()
                && matches!(d.level, DiagnosticLevel::Warning)
                && d.message.contains("compile component")
        })
        .expect("invalid component must emit a compile-failure warning diagnostic");
    assert_eq!(warn.field.as_deref(), Some("wasm_path"));
}

#[test]
fn component_attachment_is_deterministic_across_repeated_loads() {
    let dir = TempDir::new().unwrap();
    write_module_with_wasm(
        dir.path(),
        "m",
        &infill_manifest("com.example.infill", &[]),
        &minimal_component_bytes(),
    );
    let roots = [PathBuf::from(dir.path())];
    let run = || {
        let out = load_live_modules_for_plan(&roots, 1).unwrap();
        out.bindings
            .iter()
            .map(|b| (b.module.id().to_string(), b.wasm_component.is_some()))
            .collect::<Vec<_>>()
    };
    let a = run();
    let b = run();
    let c = run();
    assert_eq!(a, b, "attachment outcome must be stable across runs");
    assert_eq!(b, c);
    assert!(a.iter().all(|(_, attached)| *attached));
}

#[test]
fn mixed_valid_and_invalid_binaries_load_deterministically_side_by_side() {
    let dir = TempDir::new().unwrap();
    write_module_with_wasm(
        dir.path(),
        "ok",
        &infill_manifest("com.example.ok", &[]),
        &minimal_component_bytes(),
    );
    write_module_with_wasm(
        dir.path(),
        "bad",
        &infill_manifest("com.example.bad", &[]),
        b"garbage garbage garbage",
    );
    write_module_with_wasm(
        dir.path(),
        "ph",
        &infill_manifest("com.example.ph", &[]),
        b"x",
    );
    let out =
        load_live_modules_for_plan(std::slice::from_ref(&PathBuf::from(dir.path())), 1).unwrap();
    let by_id: std::collections::HashMap<&str, &slicer_runtime::LiveModuleBinding> =
        out.bindings.iter().map(|b| (b.module.id(), b)).collect();
    assert!(by_id["com.example.ok"].wasm_component.is_some());
    assert!(by_id["com.example.bad"].wasm_component.is_none());
    assert!(by_id["com.example.ph"].wasm_component.is_none());
    // One warning per skipped module, exactly.
    let skipped_warnings = out
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(d.level, DiagnosticLevel::Warning) && d.field.as_deref() == Some("wasm_path")
        })
        .count();
    assert_eq!(skipped_warnings, 2);
}

#[test]
fn main_production_entry_path_loads_real_modules_and_calls_live_helpers() {
    // Source-level regression guard: run.rs (the library entry point after
    // pnp-cli-unification) must use the live module loader (base
    // `load_live_modules_for_plan` or its config-aware
    // `load_live_modules_for_plan_with_config` variant — packet 112 Step 10
    // switched run.rs to the latter to resolve the perimeter-generator claim
    // collision via `wall_generator`) and `build_live_execution_plan`, and
    // must read the CLI's --config via `parse_cli_config_source`. If any of
    // these vanish, real module bindings no longer flow through
    // `bind_module_config_view` on the production entry path.
    let run_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/run.rs")).unwrap();
    assert!(
        run_src.contains("load_live_modules_for_plan"),
        "run.rs must call the live config-aware module loader (load_live_modules_for_plan or load_live_modules_for_plan_with_config)"
    );
    assert!(
        run_src.contains("build_live_execution_plan("),
        "run.rs must build the plan via build_live_execution_plan"
    );
    assert!(
        run_src.contains("parse_cli_config_source"),
        "run.rs must parse --config through parse_cli_config_source"
    );
    assert!(
        !run_src
            .contains("Vec::new(),\n                Vec::new(),\n                &config_source"),
        "run.rs must no longer pass empty bindings into build_live_execution_plan"
    );
}
