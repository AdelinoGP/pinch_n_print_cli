//! TASK: ConfigView read-only + declared-read pre-filtering wiring.
//!
//! Verifies the live-path ConfigView contract (docs/02 Â§Pre-filtered config;
//! docs/03 Â§host-boundary enforcement; docs/05 Â§module SDK):
//!
//! 1. Modules see only the keys they declared in `[config.schema]`.
//! 2. Undeclared keys are filtered out by `bind_module_config_view`.
//! 3. Typed accessors preserve subnormal normalization and basic semantics.
//! 4. Views wrapped in `Arc` cannot be mutated by consumers.
//! 5. Repeated binding with identical inputs produces identical views.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{ConfigValue, ConfigView, SemVer};
use slicer_runtime::{
    bind_module_config_view, ConfigFieldEntry, ConfigSchema, LoadedModule, LoadedModuleBuilder,
};

fn sem() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn module_with_config_keys(id: &str, keys: &[&str]) -> LoadedModule {
    let mut entries = BTreeMap::new();
    for k in keys {
        entries.insert(
            (*k).to_string(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );
    }
    LoadedModuleBuilder::new(
        id,
        sem(),
        "PrePass::MeshAnalysis",
        slicer_schema::WORLD_PREPASS,
        PathBuf::from("fixtures/mod.wasm"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(sem())
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .config_schema(ConfigSchema { entries })
    .build()
}

fn module_with_config_keys_stage_world(
    id: &str,
    keys: &[&str],
    stage: &str,
    wit_world: &str,
) -> LoadedModule {
    let mut entries = BTreeMap::new();
    for k in keys {
        entries.insert(
            (*k).to_string(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );
    }
    LoadedModuleBuilder::new(
        id,
        sem(),
        stage,
        wit_world,
        PathBuf::from("fixtures/mod.wasm"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(sem())
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .config_schema(ConfigSchema { entries })
    .build()
}

fn source() -> HashMap<String, ConfigValue> {
    let mut m = HashMap::new();
    m.insert("density".to_string(), ConfigValue::Float(0.25));
    m.insert(
        "pattern".to_string(),
        ConfigValue::String("gyroid".to_string()),
    );
    m.insert("fuzzy".to_string(), ConfigValue::Bool(true));
    m.insert(
        "secret".to_string(),
        ConfigValue::String("do-not-leak".to_string()),
    );
    m
}

#[test]
fn bind_module_config_view_exposes_only_declared_keys() {
    let module = module_with_config_keys("com.example.infill", &["density", "pattern"]);
    let view = bind_module_config_view(&module, &source());

    let mut keys = view.keys();
    keys.sort();
    assert_eq!(keys, vec!["density".to_string(), "pattern".to_string()]);
    assert!(view.contains_key("density"));
    assert!(view.contains_key("pattern"));
}

#[test]
fn bind_module_config_view_hides_undeclared_keys_entirely() {
    let module = module_with_config_keys("com.example.infill", &["density"]);
    let view = bind_module_config_view(&module, &source());

    assert!(!view.contains_key("fuzzy"));
    assert!(!view.contains_key("secret"));
    assert!(view.get("fuzzy").is_none());
    assert!(view.get_bool("fuzzy").is_none());
    assert!(view.get_string("secret").is_none());
    assert_eq!(view.len(), 1);
}

#[test]
fn bind_module_config_view_declared_but_missing_key_returns_none() {
    let module = module_with_config_keys("com.example.infill", &["density", "nonesuch"]);
    let view = bind_module_config_view(&module, &source());

    assert_eq!(view.get_float("density"), Some(0.25));
    assert!(view.get("nonesuch").is_none());
}

#[test]
fn typed_getters_preserve_semantics_and_subnormal_normalization() {
    let mut fields = HashMap::new();
    fields.insert("f".to_string(), ConfigValue::Float(1.5));
    fields.insert("i".to_string(), ConfigValue::Int(7));
    fields.insert("b".to_string(), ConfigValue::Bool(true));
    fields.insert("s".to_string(), ConfigValue::String("hello".to_string()));
    fields.insert(
        "subnormal".to_string(),
        ConfigValue::Float(f64::from_bits(1)),
    );
    let view = ConfigView::from_map(fields);

    assert_eq!(view.get_float("f"), Some(1.5));
    assert_eq!(view.get_int("i"), Some(7));
    assert_eq!(view.get_bool("b"), Some(true));
    assert_eq!(view.get_string("s"), Some("hello"));
    assert_eq!(view.get_float("subnormal"), Some(0.0));
    assert!(
        view.get_int("f").is_none(),
        "wrong-type read must yield None"
    );
    assert!(view.get_bool("i").is_none());
}

#[test]
fn invalid_numeric_values_do_not_panic_and_remain_stable() {
    let mut fields = HashMap::new();
    fields.insert("nan".to_string(), ConfigValue::Float(f64::NAN));
    fields.insert("inf".to_string(), ConfigValue::Float(f64::INFINITY));
    fields.insert("neg_inf".to_string(), ConfigValue::Float(f64::NEG_INFINITY));
    let view = ConfigView::from_map(fields);

    assert!(view.get_float("nan").unwrap().is_nan());
    assert_eq!(view.get_float("inf"), Some(f64::INFINITY));
    assert_eq!(view.get_float("neg_inf"), Some(f64::NEG_INFINITY));
}

#[test]
fn arc_wrapped_view_cannot_be_mutated_by_consumers() {
    let module = module_with_config_keys("com.example.infill", &["density"]);
    let view: Arc<ConfigView> = bind_module_config_view(&module, &source());

    // A consumer holding an `Arc<ConfigView>` cannot obtain a &mut to the
    // inner view â€” this is the documented read-only guarantee on the live
    // path. Clone a shared handle to simulate "another consumer" holding
    // the same view; Arc::get_mut must then refuse.
    let shared = Arc::clone(&view);
    let mut owned = view;
    assert!(
        Arc::get_mut(&mut owned).is_none(),
        "shared Arc<ConfigView> must refuse mutable access while any consumer holds a handle"
    );
    drop(shared);
    // With the shared handle dropped the unique owner can reclaim &mut,
    // but by this point no consumer thread is still reading â€” this is the
    // construction-site property, not the live-path contract.
    assert!(Arc::get_mut(&mut owned).is_some());
}

#[test]
fn repeated_binding_with_identical_inputs_produces_identical_views() {
    let module = module_with_config_keys("com.example.infill", &["density", "pattern"]);
    let src = source();
    let a = bind_module_config_view(&module, &src);
    let b = bind_module_config_view(&module, &src);
    let c = bind_module_config_view(&module, &src);
    assert_eq!(*a, *b);
    assert_eq!(*b, *c);
    assert_eq!(a.keys(), b.keys());
    assert_eq!(a.get_float("density"), b.get_float("density"));
}

#[test]
fn module_with_empty_config_schema_sees_empty_view() {
    let module = module_with_config_keys("com.example.bare", &[]);
    let view = bind_module_config_view(&module, &source());
    assert!(view.is_empty());
    assert_eq!(view.keys(), Vec::<String>::new());
}

#[test]
fn from_declared_is_order_independent_and_deduplicates() {
    let src = source();
    let a = ConfigView::from_declared(&src, ["density", "pattern"].iter().copied());
    let b = ConfigView::from_declared(&src, ["pattern", "density", "pattern"].iter().copied());
    assert_eq!(a, b);
}

// â”€â”€ Live plan-build wiring + guardrail â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

use slicer_ir::{RegionKey, RegionPlan};
use slicer_runtime::{
    build_execution_plan, build_wasm_instance_pool, ExecutionModuleBinding, ExecutionPlanError,
    ExecutionPlanRequest, LoadDiagnostic, SortedStageModules, WasmArtifactMetadata,
};

fn plan_request_for(module: &LoadedModule, config_view: Arc<ConfigView>) -> ExecutionPlanRequest {
    let _pool = Arc::new(
        build_wasm_instance_pool(
            module.id(),
            module.stage(),
            module.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build pool"),
    );
    ExecutionPlanRequest {
        sorted_stages: vec![SortedStageModules {
            stage_id: module.stage().to_string(),
            module_ids: vec![module.id().to_string()],
        }],
        module_bindings: vec![ExecutionModuleBinding {
            module: module.clone(),
            config_view,
        }],
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(std::collections::HashMap::<RegionKey, RegionPlan>::new()),
    }
}

#[test]
fn build_execution_plan_accepts_bound_configview_from_bind_module_config_view() {
    let module = module_with_config_keys("com.example.infill", &["density", "pattern"]);
    let view = bind_module_config_view(&module, &source());
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let plan = build_execution_plan(
        &plan_request_for(&module, Arc::clone(&view)),
        &mut diagnostics,
    )
    .expect("plan with properly bound ConfigView must succeed");

    // The bound ConfigView flows through the compiled module unchanged and
    // exposes only declared keys on the real plan/build path.
    let compiled = &plan.prepass_stages[0].modules[0];
    let keys = compiled.config_view().keys();
    assert_eq!(keys, vec!["density".to_string(), "pattern".to_string()]);
    assert!(!compiled.config_view().contains_key("secret"));
    assert!(!compiled.config_view().contains_key("fuzzy"));
}

#[test]
fn build_execution_plan_rejects_configview_with_undeclared_key() {
    let module = module_with_config_keys("com.example.infill", &["density"]);

    // Bypass `bind_module_config_view` on purpose, simulating a buggy caller
    // that leaks an undeclared key. The plan builder's guardrail must catch
    // it fatally with a structured error.
    let mut leaky = HashMap::new();
    leaky.insert("density".to_string(), ConfigValue::Float(0.3));
    leaky.insert("secret".to_string(), ConfigValue::String("leaked".into()));
    let view = Arc::new(ConfigView::from_map(leaky));

    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    match build_execution_plan(&plan_request_for(&module, view), &mut diagnostics).unwrap_err() {
        ExecutionPlanError::UndeclaredConfigKey { module_id, key } => {
            assert_eq!(module_id, "com.example.infill");
            assert_eq!(key, "secret");
        }
        other => panic!("expected UndeclaredConfigKey, got {other:?}"),
    }
}

#[test]
fn build_execution_plan_rejects_empty_schema_with_any_configview_keys() {
    let module = module_with_config_keys("com.example.bare", &[]);
    // Wrong-path caller: synthesises a view with a single key even though
    // the module declared no config schema at all.
    let mut any = HashMap::new();
    any.insert("sneaky".to_string(), ConfigValue::Int(1));
    let view = Arc::new(ConfigView::from_map(any));

    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let err = build_execution_plan(&plan_request_for(&module, view), &mut diagnostics).unwrap_err();
    assert!(
        matches!(
            err,
            ExecutionPlanError::UndeclaredConfigKey { ref key, .. } if key == "sneaky"
        ),
        "got {err:?}"
    );
}

#[test]
fn bind_module_config_view_output_passes_plan_build_guardrail() {
    // Property test: any ConfigView produced by `bind_module_config_view`
    // must always satisfy the plan-build guardrail, for any raw source.
    let module = module_with_config_keys("com.example.infill", &["density", "pattern"]);
    for src in [HashMap::new(), source()] {
        let view = bind_module_config_view(&module, &src);
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let plan = build_execution_plan(&plan_request_for(&module, view), &mut diagnostics)
            .expect("bind_module_config_view output must pass guardrail");
        assert_eq!(plan.prepass_stages.len(), 1);
    }
}

#[test]
fn main_production_entry_path_routes_through_build_live_execution_plan() {
    // Guards the live plan/build wiring: the slicer-runtime run.rs library
    // entry point must construct its ExecutionPlan via
    // `build_live_execution_plan` (the canonical helper that routes every
    // per-module ConfigView through `bind_module_config_view`), not by
    // assembling a raw `ExecutionPlan` or calling `build_execution_plan`
    // with hand-rolled bindings.
    //
    // Note: after the pnp-cli-unification refactor, the binary entry point
    // moved from the pre-rename `slicer-host/src/main.rs` into
    // `slicer-runtime/src/run.rs` (the `run_slice()` library function).
    // We guard `run.rs` here.
    let run_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/run.rs"))
        .expect("read run.rs");
    assert!(
        run_src.contains("build_live_execution_plan("),
        "run.rs must construct its ExecutionPlan via build_live_execution_plan"
    );
    assert!(
        run_src.contains("parse_cli_config_source"),
        "run.rs must parse --config through parse_cli_config_source so the live-path source is real, not a placeholder"
    );
}

// â”€â”€ build_live_execution_plan wiring â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

use slicer_runtime::{build_live_execution_plan, LiveModuleBinding};

fn live_binding(module: &LoadedModule) -> LiveModuleBinding {
    let pool = Arc::new(
        build_wasm_instance_pool(
            module.id(),
            module.stage(),
            module.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build pool"),
    );
    LiveModuleBinding {
        module: module.clone(),
        instance_pool: pool,
        wasm_component: None,
    }
}

#[test]
fn build_live_execution_plan_filters_every_module_config_view_through_bind_helper() {
    let m = module_with_config_keys("com.example.infill", &["density", "pattern"]);
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let plan = build_live_execution_plan(
        vec![SortedStageModules {
            stage_id: m.stage().to_string(),
            module_ids: vec![m.id().to_string()],
        }],
        vec![live_binding(&m)],
        &source(),
        Arc::new(Vec::new()),
        Arc::new(std::collections::HashMap::<RegionKey, RegionPlan>::new()),
        &mut diagnostics,
    )
    .expect("live plan must build");

    let compiled = &plan.prepass_stages[0].modules[0];
    assert_eq!(
        compiled.config_view().keys(),
        vec!["density".to_string(), "pattern".to_string()]
    );
    assert!(!compiled.config_view().contains_key("secret"));
    assert!(!compiled.config_view().contains_key("fuzzy"));
    assert_eq!(compiled.config_view().get_float("density"), Some(0.25));
}

#[test]
fn build_live_execution_plan_never_exposes_undeclared_keys_to_compiled_modules() {
    // Raw source carries many keys; each module in the plan must see only
    // the subset it declared.
    let m1 = module_with_config_keys("com.example.a", &["density"]);
    let m2 = module_with_config_keys_stage_world(
        "com.example.b",
        &["fuzzy"],
        "PrePass::LayerPlanning",
        slicer_schema::WORLD_PREPASS,
    );

    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let plan = build_live_execution_plan(
        vec![
            SortedStageModules {
                stage_id: m1.stage().to_string(),
                module_ids: vec![m1.id().to_string()],
            },
            SortedStageModules {
                stage_id: m2.stage().to_string(),
                module_ids: vec![m2.id().to_string()],
            },
        ],
        vec![live_binding(&m1), live_binding(&m2)],
        &source(),
        Arc::new(Vec::new()),
        Arc::new(std::collections::HashMap::<RegionKey, RegionPlan>::new()),
        &mut diagnostics,
    )
    .unwrap();

    // module "a" declared only `density`; it must not see `fuzzy` or other keys
    let a = &plan.prepass_stages[0].modules[0];
    assert_eq!(a.config_view().keys(), vec!["density".to_string()]);
    assert!(!a.config_view().contains_key("fuzzy"));
    assert!(!a.config_view().contains_key("secret"));
    // module "b" declared only `fuzzy`; it must not see `density`.
    let b = &plan.prepass_stages[1].modules[0];
    assert_eq!(b.config_view().keys(), vec!["fuzzy".to_string()]);
    assert!(!b.config_view().contains_key("density"));
}

// â”€â”€ parse_cli_config_source â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

use slicer_runtime::{parse_cli_config_source, ConfigSourceParseError};

#[test]
fn parse_cli_config_source_maps_json_types_to_config_values() {
    let json = r#"{
        "enabled": true,
        "count": 7,
        "density": 0.25,
        "pattern": "gyroid",
        "angles": [0.0, 45.0, 90.0]
    }"#;
    let parsed = parse_cli_config_source(json).unwrap();
    assert!(matches!(
        parsed.get("enabled"),
        Some(ConfigValue::Bool(true))
    ));
    assert!(matches!(parsed.get("count"), Some(ConfigValue::Int(7))));
    assert!(
        matches!(parsed.get("density"), Some(ConfigValue::Float(f)) if (f - 0.25).abs() < 1e-12)
    );
    assert!(matches!(parsed.get("pattern"), Some(ConfigValue::String(s)) if s == "gyroid"));
    assert!(matches!(parsed.get("angles"), Some(ConfigValue::List(items)) if items.len() == 3));
}

#[test]
fn parse_cli_config_source_normalises_subnormal_floats_to_zero() {
    let subnormal = f64::from_bits(1);
    let json = format!(r#"{{"x": {subnormal}}}"#);
    let parsed = parse_cli_config_source(&json).unwrap();
    assert_eq!(parsed.get("x"), Some(&ConfigValue::Float(0.0)));
}

#[test]
fn parse_cli_config_source_rejects_unsupported_values_and_non_objects() {
    assert!(matches!(
        parse_cli_config_source("not valid json").unwrap_err(),
        ConfigSourceParseError::InvalidJson { .. }
    ));
    assert!(matches!(
        parse_cli_config_source("[1, 2, 3]").unwrap_err(),
        ConfigSourceParseError::NotAnObject
    ));
    assert!(matches!(
        parse_cli_config_source(r#"{"x": null}"#).unwrap_err(),
        ConfigSourceParseError::UnsupportedValue { ref key } if key == "x"
    ));
    assert!(matches!(
        parse_cli_config_source(r#"{"x": {"nested": 1}}"#).unwrap_err(),
        ConfigSourceParseError::UnsupportedValue { ref key } if key == "x"
    ));
}

#[test]
fn parse_cli_config_source_output_feeds_bind_module_config_view_cleanly() {
    // End-to-end: parsed JSON â†’ bound ConfigView â†’ only declared keys visible.
    let json = r#"{"density": 0.42, "undeclared": "leak"}"#;
    let src = parse_cli_config_source(json).unwrap();
    let m = module_with_config_keys("com.example.infill", &["density"]);
    let view = bind_module_config_view(&m, &src);
    assert_eq!(view.keys(), vec!["density".to_string()]);
    assert_eq!(view.get_float("density"), Some(0.42));
    assert!(view.get("undeclared").is_none());
}
