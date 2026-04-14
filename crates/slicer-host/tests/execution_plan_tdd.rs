#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::{
    build_execution_plan, build_wasm_instance_pool, CompiledModule, ConfigSchema,
    ExecutionModuleBinding, ExecutionPlanRequest, SortedStageModules, WasmArtifactMetadata,
};
use slicer_ir::{
    ConfigValue, ConfigView, GlobalLayer, RegionKey, RegionPlan, ResolvedConfig, SemVer,
};

#[test]
fn freezes_sorted_stage_buckets_runtime_bindings_and_shared_ir_ownership() {
    let global_layers = Arc::new(vec![GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: true,
    }]);
    let region_plans = Arc::new(HashMap::from([(
        RegionKey {
            global_layer_index: 0,
            object_id: String::from("cube"),
            region_id: 7,
        },
        RegionPlan {
            config: ResolvedConfig::default(),
            stage_modules: HashMap::new(),
        },
    )]));

    let prepass_module = bound_module(
        loaded_module(
            "com.example.mesh-analysis",
            "PrePass::MeshAnalysis",
            &["MeshIR.objects"],
            &["SurfaceClassificationIR.per_object"],
            true,
            "slicer:world-prepass@1.0.0",
        ),
        config_view(&[("feature.prepass", ConfigValue::Bool(true))]),
        4,
    );
    let layer_module = bound_module(
        loaded_module(
            "com.example.perimeters",
            "Layer::Perimeters",
            &["SliceIR.regions"],
            &["PerimeterIR.regions.walls"],
            true,
            "slicer:world-layer@1.0.0",
        ),
        config_view(&[("walls", ConfigValue::Int(3))]),
        4,
    );
    let finalization_module = bound_module(
        loaded_module(
            "com.example.finalizer",
            "PostPass::LayerFinalization",
            &["LayerCollectionIR.layers"],
            &["LayerCollectionIR.finalized"],
            false,
            "slicer:world-finalization@1.0.0",
        ),
        config_view(&[("emit-checkpoints", ConfigValue::Bool(false))]),
        4,
    );
    let postpass_module = bound_module(
        loaded_module(
            "com.example.text-post",
            "PostPass::TextPostProcess",
            &["GCodeIR.text"],
            &["GCodeIR.text"],
            false,
            "slicer:world-postpass@1.0.0",
        ),
        config_view(&[("footer", ConfigValue::String(String::from("done")))]),
        4,
    );

    let request = ExecutionPlanRequest {
        sorted_stages: vec![
            sorted_stage("PrePass::RegionMapping", &[]),
            sorted_stage("PrePass::MeshAnalysis", &["com.example.mesh-analysis"]),
            sorted_stage("Layer::Perimeters", &["com.example.perimeters"]),
            sorted_stage("PostPass::LayerFinalization", &["com.example.finalizer"]),
            sorted_stage("PostPass::GCodeEmit", &[]),
            sorted_stage("PostPass::TextPostProcess", &["com.example.text-post"]),
        ],
        module_bindings: vec![
            prepass_module.clone(),
            layer_module.clone(),
            finalization_module.clone(),
            postpass_module.clone(),
        ],
        global_layers: Arc::clone(&global_layers),
        region_plans: Arc::clone(&region_plans),
    };

    let plan = build_execution_plan(&request)
        .expect("execution plan builder should freeze validated order and bindings");

    assert_eq!(plan.prepass_stages.len(), 1);
    assert_eq!(plan.prepass_stages[0].stage_id, "PrePass::MeshAnalysis");
    assert_module(
        &plan.prepass_stages[0].modules[0],
        &prepass_module,
        &["MeshIR.objects"],
        &["SurfaceClassificationIR.per_object"],
    );

    assert_eq!(plan.per_layer_stages.len(), 1);
    assert_eq!(plan.per_layer_stages[0].stage_id, "Layer::Perimeters");
    assert_module(
        &plan.per_layer_stages[0].modules[0],
        &layer_module,
        &["SliceIR.regions"],
        &["PerimeterIR.regions.walls"],
    );

    let finalization_stage = plan
        .layer_finalization_stage
        .as_ref()
        .expect("layer finalization should be isolated into its own bucket");
    assert_eq!(finalization_stage.stage_id, "PostPass::LayerFinalization");
    assert_module(
        &finalization_stage.modules[0],
        &finalization_module,
        &["LayerCollectionIR.layers"],
        &["LayerCollectionIR.finalized"],
    );

    assert_eq!(plan.postpass_stages.len(), 1);
    assert_eq!(
        plan.postpass_stages[0].stage_id,
        "PostPass::TextPostProcess"
    );
    assert_module(
        &plan.postpass_stages[0].modules[0],
        &postpass_module,
        &["GCodeIR.text"],
        &["GCodeIR.text"],
    );

    assert!(Arc::ptr_eq(&plan.global_layers, &global_layers));
    assert!(Arc::ptr_eq(&plan.region_plans, &region_plans));
}

fn assert_module(
    compiled: &CompiledModule,
    expected: &ExecutionModuleBinding,
    expected_reads: &[&str],
    expected_writes: &[&str],
) {
    assert_eq!(compiled.module_id, expected.module.id);
    assert_eq!(compiled.ir_read_mask.paths, strings(expected_reads));
    assert_eq!(compiled.ir_write_mask.paths, strings(expected_writes));
    assert!(Arc::ptr_eq(
        &compiled.instance_pool,
        &expected.instance_pool
    ));
    assert!(Arc::ptr_eq(&compiled.config_view, &expected.config_view));
}

fn bound_module(
    module: slicer_host::LoadedModule,
    config_view: ConfigView,
    host_parallelism: usize,
) -> ExecutionModuleBinding {
    let instance_pool = Arc::new(
        build_wasm_instance_pool(
            &module,
            host_parallelism,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture module should build a pool"),
    );

    ExecutionModuleBinding {
        module,
        instance_pool,
        config_view: Arc::new(config_view),
        wasm_component: None,
    }
}

fn sorted_stage(stage_id: &str, module_ids: &[&str]) -> SortedStageModules {
    SortedStageModules {
        stage_id: String::from(stage_id),
        module_ids: strings(module_ids),
    }
}

fn config_view(entries: &[(&str, ConfigValue)]) -> ConfigView {
    ConfigView {
        fields: entries
            .iter()
            .map(|(key, value)| (String::from(*key), value.clone()))
            .collect(),
    }
}

fn loaded_module(
    id: &str,
    stage: &str,
    ir_reads: &[&str],
    ir_writes: &[&str],
    layer_parallel_safe: bool,
    wit_world: &str,
) -> slicer_host::LoadedModule {
    slicer_host::LoadedModule {
        id: String::from(id),
        version: semver(1, 0, 0),
        stage: String::from(stage),
        wit_world: String::from(wit_world),
        ir_reads: strings(ir_reads),
        ir_writes: strings(ir_writes),
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: ConfigSchema::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe,
        wasm_path: PathBuf::from(format!("fixtures/{id}.wasm")),
        placeholder_wasm: false,
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| String::from(*value)).collect()
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

// ── Layer index budget tests ──────────────────────────────────────────

#[test]
fn layer_index_at_budget_boundary_is_rejected() {
    use slicer_host::{ExecutionPlanError, MAX_LAYER_INDEX};

    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: MAX_LAYER_INDEX, // exactly at boundary
            z: 20_000.0,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
    };

    let err = build_execution_plan(&request)
        .expect_err("layer index at budget boundary should be rejected");
    match err {
        ExecutionPlanError::LayerIndexBudgetExceeded { layer_index, budget } => {
            assert_eq!(layer_index, MAX_LAYER_INDEX);
            assert_eq!(budget, MAX_LAYER_INDEX);
        }
        other => panic!("expected LayerIndexBudgetExceeded, got {other:?}"),
    }
}

#[test]
fn layer_index_just_below_budget_is_accepted() {
    use slicer_host::MAX_LAYER_INDEX;

    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: MAX_LAYER_INDEX - 1,
            z: 19_999.8,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
    };

    build_execution_plan(&request).expect("layer index just below budget should be accepted");
}

#[test]
fn layer_index_zero_is_accepted() {
    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
    };

    build_execution_plan(&request).expect("layer index 0 should be accepted");
}

#[test]
fn error_display_includes_layer_budget_remediation() {
    use slicer_host::ExecutionPlanError;

    let err = ExecutionPlanError::LayerIndexBudgetExceeded {
        layer_index: 200_000,
        budget: 100_000,
    };
    let msg = err.to_string();
    assert!(msg.contains("200000"), "should include actual index: {msg}");
    assert!(msg.contains("100000"), "should include budget: {msg}");
    assert!(
        msg.contains("reduce") || msg.contains("increase"),
        "should include remediation hint: {msg}"
    );
}

// ── Region map cap tests ──────────────────────────────────────────────

#[test]
fn region_map_exceeding_cap_is_rejected() {
    use slicer_host::{ExecutionPlanError, DEFAULT_REGION_MAP_CAP};

    let mut entries = HashMap::new();
    for i in 0..=DEFAULT_REGION_MAP_CAP {
        entries.insert(
            RegionKey {
                global_layer_index: (i / 10) as u32,
                object_id: format!("obj-{}", i % 3),
                region_id: i as u64,
            },
            RegionPlan {
                config: ResolvedConfig::default(),
                stage_modules: HashMap::new(),
            },
        );
    }

    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(entries),
    };

    let err = build_execution_plan(&request)
        .expect_err("region map exceeding cap should be rejected");
    match err {
        ExecutionPlanError::RegionMapCapExceeded { entry_count, cap } => {
            assert!(entry_count > DEFAULT_REGION_MAP_CAP);
            assert_eq!(cap, DEFAULT_REGION_MAP_CAP);
        }
        other => panic!("expected RegionMapCapExceeded, got {other:?}"),
    }
}

#[test]
fn region_map_at_cap_is_accepted() {
    use slicer_host::DEFAULT_REGION_MAP_CAP;

    let mut entries = HashMap::new();
    for i in 0..DEFAULT_REGION_MAP_CAP {
        entries.insert(
            RegionKey {
                global_layer_index: (i / 10) as u32,
                object_id: format!("obj-{}", i % 3),
                region_id: i as u64,
            },
            RegionPlan {
                config: ResolvedConfig::default(),
                stage_modules: HashMap::new(),
            },
        );
    }

    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(entries),
    };

    build_execution_plan(&request).expect("region map at exactly the cap should be accepted");
}

#[test]
fn error_display_includes_region_map_remediation() {
    use slicer_host::ExecutionPlanError;

    let err = ExecutionPlanError::RegionMapCapExceeded {
        entry_count: 2000,
        cap: 1000,
    };
    let msg = err.to_string();
    assert!(msg.contains("2000"), "should include entry count: {msg}");
    assert!(msg.contains("1000"), "should include cap: {msg}");
    assert!(
        msg.contains("reduce") || msg.contains("raise") || msg.contains("split"),
        "should include remediation hint: {msg}"
    );
}

// ── Deterministic plan construction ───────────────────────────────────

#[test]
fn plan_construction_is_deterministic_across_repeated_calls() {
    let mk_request = || {
        let module = bound_module(
            loaded_module(
                "com.test.infill",
                "Layer::Infill",
                &[],
                &[],
                true,
                "slicer:world-layer@1.0.0",
            ),
            ConfigView { fields: HashMap::new() },
            4,
        );
        ExecutionPlanRequest {
            sorted_stages: vec![sorted_stage("Layer::Infill", &["com.test.infill"])],
            module_bindings: vec![module],
            global_layers: Arc::new(vec![
                GlobalLayer { index: 0, z: 0.2, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false },
                GlobalLayer { index: 1, z: 0.4, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false },
            ]),
            region_plans: Arc::new(HashMap::new()),
        }
    };

    let plan_a = build_execution_plan(&mk_request()).unwrap();
    let plan_b = build_execution_plan(&mk_request()).unwrap();

    assert_eq!(plan_a.per_layer_stages.len(), plan_b.per_layer_stages.len());
    assert_eq!(
        plan_a.per_layer_stages[0].modules[0].module_id,
        plan_b.per_layer_stages[0].modules[0].module_id,
    );
    assert_eq!(plan_a.global_layers.len(), plan_b.global_layers.len());
}

// ── Resource-bound enforcement / bounded-failure contracts ────────────

#[test]
fn layer_index_u32_max_is_rejected_with_budget_error() {
    use slicer_host::{ExecutionPlanError, MAX_LAYER_INDEX};

    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: u32::MAX,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
    };

    match build_execution_plan(&request).expect_err("u32::MAX must be rejected") {
        ExecutionPlanError::LayerIndexBudgetExceeded { layer_index, budget } => {
            assert_eq!(layer_index, u32::MAX);
            assert_eq!(budget, MAX_LAYER_INDEX);
        }
        other => panic!("expected LayerIndexBudgetExceeded, got {other:?}"),
    }
}

#[test]
fn layer_budget_check_preempts_module_binding_errors() {
    // Resource-bound failures must fire before coupling/binding failures so the
    // operator gets the actionable budget diagnostic per docs/12 §Resource Bounds.
    use slicer_host::{ExecutionPlanError, MAX_LAYER_INDEX};

    let request = ExecutionPlanRequest {
        // Reference an unbound module — would normally surface MissingModuleBinding.
        sorted_stages: vec![sorted_stage("Layer::Infill", &["com.test.absent"])],
        module_bindings: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: MAX_LAYER_INDEX,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
    };

    match build_execution_plan(&request).expect_err("budget should preempt binding error") {
        ExecutionPlanError::LayerIndexBudgetExceeded { .. } => {}
        other => panic!("expected LayerIndexBudgetExceeded to preempt, got {other:?}"),
    }
}

#[test]
fn layer_budget_reports_first_offending_layer_deterministically() {
    use slicer_host::{ExecutionPlanError, MAX_LAYER_INDEX};

    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(vec![
            GlobalLayer { index: MAX_LAYER_INDEX,     z: 0.0, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false },
            GlobalLayer { index: MAX_LAYER_INDEX + 1, z: 0.2, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false },
            GlobalLayer { index: u32::MAX,           z: 0.4, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false },
        ]),
        region_plans: Arc::new(HashMap::new()),
    };

    for _ in 0..5 {
        match build_execution_plan(&request).expect_err("must reject") {
            ExecutionPlanError::LayerIndexBudgetExceeded { layer_index, .. } => {
                assert_eq!(
                    layer_index, MAX_LAYER_INDEX,
                    "must report first offending layer in vector order, deterministically"
                );
            }
            other => panic!("expected LayerIndexBudgetExceeded, got {other:?}"),
        }
    }
}

#[test]
fn region_map_cap_reports_exact_computed_entry_count() {
    use slicer_host::{ExecutionPlanError, DEFAULT_REGION_MAP_CAP};

    let mut entries = HashMap::new();
    let overflow = DEFAULT_REGION_MAP_CAP + 7;
    for i in 0..overflow {
        entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: format!("obj-{i}"),
                region_id: i as u64,
            },
            RegionPlan {
                config: ResolvedConfig::default(),
                stage_modules: HashMap::new(),
            },
        );
    }
    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(entries),
    };

    match build_execution_plan(&request).expect_err("must reject overflow") {
        ExecutionPlanError::RegionMapCapExceeded { entry_count, cap } => {
            assert_eq!(entry_count, overflow);
            assert_eq!(cap, DEFAULT_REGION_MAP_CAP);
        }
        other => panic!("expected RegionMapCapExceeded, got {other:?}"),
    }
}

#[test]
fn duplicate_module_binding_rejected_with_stable_diagnostic() {
    use slicer_host::ExecutionPlanError;

    let mk_binding = || bound_module(
        loaded_module(
            "com.test.dup",
            "Layer::Infill",
            &[],
            &[],
            true,
            "slicer:world-layer@1.0.0",
        ),
        ConfigView { fields: HashMap::new() },
        2,
    );
    let request = ExecutionPlanRequest {
        sorted_stages: vec![sorted_stage("Layer::Infill", &["com.test.dup"])],
        module_bindings: vec![mk_binding(), mk_binding()],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    };

    for _ in 0..3 {
        match build_execution_plan(&request).expect_err("duplicate binding must be rejected") {
            ExecutionPlanError::DuplicateModuleBinding { module_id } => {
                assert_eq!(module_id.as_str(), "com.test.dup");
            }
            other => panic!("expected DuplicateModuleBinding, got {other:?}"),
        }
    }
}
