#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, ConfigId, ConfigValue, ConfigView, GlobalLayer, RegionKey, RegionPlan,
    ResolvedConfig, SemVer,
};
use slicer_scheduler::{
    build_execution_plan, CompiledModuleStatic, ConfigFieldEntry, ExecutionModuleBinding,
    ExecutionPlanRequest, LoadDiagnostic, LoadedModuleBuilder, SortedStageModules,
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
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config: ConfigId::default(),
            stage_modules: HashMap::new(),
            paint_overrides: BTreeMap::new(),
        },
    )]));

    let prepass_module = bound_module(
        loaded_module(
            "com.example.mesh-analysis",
            "PrePass::MeshAnalysis",
            &["MeshIR.objects"],
            &["SurfaceClassificationIR.per_object"],
            true,
            slicer_schema::WORLD_PREPASS,
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
            slicer_schema::WORLD_LAYER,
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
            slicer_schema::WORLD_FINALIZATION,
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
            slicer_schema::WORLD_POSTPASS,
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

    let plan = build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect("execution plan builder should freeze validated order and bindings");

    assert_eq!(plan.prepass_stages.len(), 1);
    assert_eq!(plan.prepass_stages[0].stage_id, "PrePass::MeshAnalysis");
    assert_module(
        &plan.prepass_stages[0].modules[0],
        &prepass_module,
        &["MeshIR.objects"],
        &["SurfaceClassificationIR.per_object"],
    );

    // `build_execution_plan` auto-injects an empty `Layer::PaintRegionAnnotation`
    // stage so the host annotator runs before downstream stages need
    // segment_annotations (packet-64). Skip past it when locating the user-facing
    // stage under test.
    let perimeters_stage = plan
        .per_layer_stages
        .iter()
        .find(|s| s.stage_id == "Layer::Perimeters")
        .expect("plan must contain Layer::Perimeters");
    assert_module(
        &perimeters_stage.modules[0],
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
    compiled: &CompiledModuleStatic,
    expected: &ExecutionModuleBinding,
    expected_reads: &[&str],
    expected_writes: &[&str],
) {
    assert_eq!(compiled.module_id(), expected.module.id());
    assert_eq!(compiled.ir_read_mask().paths, strings(expected_reads));
    assert_eq!(compiled.ir_write_mask().paths, strings(expected_writes));
    assert!(Arc::ptr_eq(compiled.config_view(), &expected.config_view));
}

fn bound_module(
    module: slicer_scheduler::LoadedModule,
    config_view: ConfigView,
    _host_parallelism: usize,
) -> ExecutionModuleBinding {
    // Ensure the module's declared `[config.schema]` covers every key the
    // fixture-built `ConfigView` exposes, so the plan-build declared-read
    // guardrail (`ExecutionPlanError::UndeclaredConfigKey`) doesn't reject
    // the synthetic fixture. This mirrors the production contract where
    // live views come from `bind_module_config_view(module, source)`.
    let mut schema = module.config_schema().clone();
    for key in config_view.keys() {
        schema.entries.insert(
            key.clone(),
            ConfigFieldEntry {
                field_type: "bool".to_string(),
                ..Default::default()
            },
        );
    }
    let module = LoadedModuleBuilder::new(
        module.id(),
        module.version(),
        module.stage(),
        module.wit_world(),
        module.wasm_path().to_path_buf(),
    )
    .ir_reads(module.ir_reads().to_vec())
    .ir_writes(module.ir_writes().to_vec())
    .min_host_version(module.min_host_version())
    .layer_parallel_safe(module.layer_parallel_safe())
    .config_schema(schema)
    .build();

    ExecutionModuleBinding {
        module,
        config_view: Arc::new(config_view),
    }
}

fn sorted_stage(stage_id: &str, module_ids: &[&str]) -> SortedStageModules {
    SortedStageModules {
        stage_id: String::from(stage_id),
        module_ids: strings(module_ids),
    }
}

fn config_view(entries: &[(&str, ConfigValue)]) -> ConfigView {
    ConfigView::from_map(
        entries
            .iter()
            .map(|(key, value)| (String::from(*key), value.clone()))
            .collect(),
    )
}

fn loaded_module(
    id: &str,
    stage: &str,
    ir_reads: &[&str],
    ir_writes: &[&str],
    layer_parallel_safe: bool,
    wit_world: &str,
) -> slicer_scheduler::LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        wit_world,
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .ir_reads(strings(ir_reads))
    .ir_writes(strings(ir_writes))
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(layer_parallel_safe)
    .build()
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

// â”€â”€ Layer index budget tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn layer_index_at_budget_boundary_is_rejected() {
    use slicer_scheduler::{ExecutionPlanError, MAX_LAYER_INDEX};

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

    let err = build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect_err("layer index at budget boundary should be rejected");
    match err {
        ExecutionPlanError::LayerIndexBudgetExceeded {
            layer_index,
            budget,
        } => {
            assert_eq!(layer_index, MAX_LAYER_INDEX);
            assert_eq!(budget, MAX_LAYER_INDEX);
        }
        other => panic!("expected LayerIndexBudgetExceeded, got {other:?}"),
    }
}

#[test]
fn layer_index_just_below_budget_is_accepted() {
    use slicer_scheduler::MAX_LAYER_INDEX;

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

    build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect("layer index just below budget should be accepted");
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

    build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect("layer index 0 should be accepted");
}

#[test]
fn error_display_includes_layer_budget_remediation() {
    use slicer_scheduler::ExecutionPlanError;

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

// â”€â”€ Region map cap tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn region_map_exceeding_cap_is_rejected() {
    use slicer_scheduler::{ExecutionPlanError, DEFAULT_REGION_MAP_CAP};

    let mut entries = HashMap::new();
    for i in 0..=DEFAULT_REGION_MAP_CAP {
        entries.insert(
            RegionKey {
                global_layer_index: (i / 10) as u32,
                object_id: format!("obj-{}", i % 3),
                region_id: i as u64,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config: ConfigId::default(),
                stage_modules: HashMap::new(),
                paint_overrides: BTreeMap::new(),
            },
        );
    }

    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(entries),
    };

    let err = build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
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
    use slicer_scheduler::DEFAULT_REGION_MAP_CAP;

    let mut entries = HashMap::new();
    for i in 0..DEFAULT_REGION_MAP_CAP {
        entries.insert(
            RegionKey {
                global_layer_index: (i / 10) as u32,
                object_id: format!("obj-{}", i % 3),
                region_id: i as u64,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config: ConfigId::default(),
                stage_modules: HashMap::new(),
                paint_overrides: BTreeMap::new(),
            },
        );
    }

    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(entries),
    };

    build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect("region map at exactly the cap should be accepted");
}

#[test]
fn error_display_includes_region_map_remediation() {
    use slicer_scheduler::ExecutionPlanError;

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

// â”€â”€ Deterministic plan construction â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
                slicer_schema::WORLD_LAYER,
            ),
            ConfigView::from_map(HashMap::new()),
            4,
        );
        ExecutionPlanRequest {
            sorted_stages: vec![sorted_stage("Layer::Infill", &["com.test.infill"])],
            module_bindings: vec![module],
            global_layers: Arc::new(vec![
                GlobalLayer {
                    index: 0,
                    z: 0.2,
                    active_regions: Vec::new(),
                    has_nonplanar: false,
                    is_sync_layer: false,
                },
                GlobalLayer {
                    index: 1,
                    z: 0.4,
                    active_regions: Vec::new(),
                    has_nonplanar: false,
                    is_sync_layer: false,
                },
            ]),
            region_plans: Arc::new(HashMap::new()),
        }
    };

    let plan_a = build_execution_plan(&mk_request(), &mut Vec::<LoadDiagnostic>::new()).unwrap();
    let plan_b = build_execution_plan(&mk_request(), &mut Vec::<LoadDiagnostic>::new()).unwrap();

    assert_eq!(plan_a.per_layer_stages.len(), plan_b.per_layer_stages.len());
    let infill_a = plan_a
        .per_layer_stages
        .iter()
        .find(|s| s.stage_id == "Layer::Infill")
        .expect("plan_a must contain Layer::Infill");
    let infill_b = plan_b
        .per_layer_stages
        .iter()
        .find(|s| s.stage_id == "Layer::Infill")
        .expect("plan_b must contain Layer::Infill");
    assert_eq!(
        infill_a.modules[0].module_id(),
        infill_b.modules[0].module_id(),
    );
    assert_eq!(plan_a.global_layers.len(), plan_b.global_layers.len());
}

// â”€â”€ Resource-bound enforcement / bounded-failure contracts â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn layer_index_u32_max_is_rejected_with_budget_error() {
    use slicer_scheduler::{ExecutionPlanError, MAX_LAYER_INDEX};

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

    match build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect_err("u32::MAX must be rejected")
    {
        ExecutionPlanError::LayerIndexBudgetExceeded {
            layer_index,
            budget,
        } => {
            assert_eq!(layer_index, u32::MAX);
            assert_eq!(budget, MAX_LAYER_INDEX);
        }
        other => panic!("expected LayerIndexBudgetExceeded, got {other:?}"),
    }
}

#[test]
fn layer_budget_check_preempts_module_binding_errors() {
    // Resource-bound failures must fire before coupling/binding failures so the
    // operator gets the actionable budget diagnostic per docs/12 Â§Resource Bounds.
    use slicer_scheduler::{ExecutionPlanError, MAX_LAYER_INDEX};

    let request = ExecutionPlanRequest {
        // Reference an unbound module â€” would normally surface MissingModuleBinding.
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

    match build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect_err("budget should preempt binding error")
    {
        ExecutionPlanError::LayerIndexBudgetExceeded { .. } => {}
        other => panic!("expected LayerIndexBudgetExceeded to preempt, got {other:?}"),
    }
}

#[test]
fn layer_budget_reports_first_offending_layer_deterministically() {
    use slicer_scheduler::{ExecutionPlanError, MAX_LAYER_INDEX};

    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(vec![
            GlobalLayer {
                index: MAX_LAYER_INDEX,
                z: 0.0,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: MAX_LAYER_INDEX + 1,
                z: 0.2,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: u32::MAX,
                z: 0.4,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ]),
        region_plans: Arc::new(HashMap::new()),
    };

    for _ in 0..5 {
        match build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
            .expect_err("must reject")
        {
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
    use slicer_scheduler::{ExecutionPlanError, DEFAULT_REGION_MAP_CAP};

    let mut entries = HashMap::new();
    let overflow = DEFAULT_REGION_MAP_CAP + 7;
    for i in 0..overflow {
        entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: format!("obj-{i}"),
                region_id: i as u64,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config: ConfigId::default(),
                stage_modules: HashMap::new(),
                paint_overrides: BTreeMap::new(),
            },
        );
    }
    let request = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: Vec::new(),
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(entries),
    };

    match build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect_err("must reject overflow")
    {
        ExecutionPlanError::RegionMapCapExceeded { entry_count, cap } => {
            assert_eq!(entry_count, overflow);
            assert_eq!(cap, DEFAULT_REGION_MAP_CAP);
        }
        other => panic!("expected RegionMapCapExceeded, got {other:?}"),
    }
}

#[test]
fn duplicate_module_binding_rejected_with_stable_diagnostic() {
    use slicer_scheduler::ExecutionPlanError;

    let mk_binding = || {
        bound_module(
            loaded_module(
                "com.test.dup",
                "Layer::Infill",
                &[],
                &[],
                true,
                slicer_schema::WORLD_LAYER,
            ),
            ConfigView::from_map(HashMap::new()),
            2,
        )
    };
    let request = ExecutionPlanRequest {
        sorted_stages: vec![sorted_stage("Layer::Infill", &["com.test.dup"])],
        module_bindings: vec![mk_binding(), mk_binding()],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    };

    for _ in 0..3 {
        match build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
            .expect_err("duplicate binding must be rejected")
        {
            ExecutionPlanError::DuplicateModuleBinding { module_id } => {
                assert_eq!(module_id.as_str(), "com.test.dup");
            }
            other => panic!("expected DuplicateModuleBinding, got {other:?}"),
        }
    }
}

// â”€â”€ Precomputed module-region index (TASK-131) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn resolve_active_regions_uses_precomputed_index() {
    // Build a plan with two modules and three regions spread across two layers.
    let global_layers = Arc::new(vec![
        GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![active_region("cube", 1), active_region("cube", 2)],
            has_nonplanar: false,
            is_sync_layer: false,
        },
        GlobalLayer {
            index: 1,
            z: 0.4,
            active_regions: vec![active_region("cube", 1)],
            has_nonplanar: false,
            is_sync_layer: false,
        },
    ]);

    // Three region plans across two layers and two objects.
    let region_plans = Arc::new(HashMap::from([
        (
            RegionKey {
                global_layer_index: 0,
                object_id: "cube".into(),
                region_id: 1,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config: ConfigId::default(),
                stage_modules: HashMap::new(),
                paint_overrides: BTreeMap::new(),
            },
        ),
        (
            RegionKey {
                global_layer_index: 0,
                object_id: "cube".into(),
                region_id: 2,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config: ConfigId::default(),
                stage_modules: HashMap::new(),
                paint_overrides: BTreeMap::new(),
            },
        ),
        (
            RegionKey {
                global_layer_index: 1,
                object_id: "cube".into(),
                region_id: 1,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config: ConfigId::default(),
                stage_modules: HashMap::new(),
                paint_overrides: BTreeMap::new(),
            },
        ),
    ]));

    let mod_a = bound_module(
        loaded_module(
            "mod.a",
            "Layer::Perimeters",
            &[],
            &[],
            true,
            slicer_schema::WORLD_LAYER,
        ),
        ConfigView::from_map(HashMap::new()),
        4,
    );
    let mod_b = bound_module(
        loaded_module(
            "mod.b",
            "Layer::Infill",
            &[],
            &[],
            true,
            slicer_schema::WORLD_LAYER,
        ),
        ConfigView::from_map(HashMap::new()),
        4,
    );

    let request = ExecutionPlanRequest {
        sorted_stages: vec![
            sorted_stage("Layer::Perimeters", &["mod.a"]),
            sorted_stage("Layer::Infill", &["mod.b"]),
        ],
        module_bindings: vec![mod_a, mod_b],
        global_layers: Arc::clone(&global_layers),
        region_plans: Arc::clone(&region_plans),
    };

    let plan = build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect("plan should build");
    let perimeters_stage = plan
        .per_layer_stages
        .iter()
        .find(|s| s.stage_id == "Layer::Perimeters")
        .expect("plan must contain Layer::Perimeters");

    // mod.a on layer 0 â†’ 2 regions
    let layer0 = &global_layers[0];
    let result = plan.resolve_active_regions(layer0, &perimeters_stage.modules[0]);
    let region_keys: Vec<_> = result
        .iter()
        .map(|r| (r.object_id.clone(), r.region_id))
        .collect();
    assert_eq!(
        region_keys,
        &[("cube".into(), 1), ("cube".into(), 2)],
        "mod.a on layer 0 must find 2 regions"
    );

    // mod.a on layer 1 â†’ 1 region
    let layer1 = &global_layers[1];
    let result = plan.resolve_active_regions(layer1, &perimeters_stage.modules[0]);
    let region_keys: Vec<_> = result
        .iter()
        .map(|r| (r.object_id.clone(), r.region_id))
        .collect();
    assert_eq!(
        region_keys,
        &[("cube".into(), 1)],
        "mod.a on layer 1 must find 1 region"
    );
}

#[test]
fn resolve_active_regions_returns_empty_when_module_has_no_regions() {
    // Module `mod.a` has no regions at any layer â†’ returns empty slice (not error).
    let global_layers = Arc::new(vec![GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: vec![], // no active regions
        has_nonplanar: false,
        is_sync_layer: false,
    }]);

    let region_plans = Arc::new(HashMap::new()); // no region plans

    let mod_a = bound_module(
        loaded_module(
            "mod.a",
            "Layer::Perimeters",
            &[],
            &[],
            true,
            slicer_schema::WORLD_LAYER,
        ),
        ConfigView::from_map(HashMap::new()),
        4,
    );

    let request = ExecutionPlanRequest {
        sorted_stages: vec![sorted_stage("Layer::Perimeters", &["mod.a"])],
        module_bindings: vec![mod_a],
        global_layers: Arc::clone(&global_layers),
        region_plans,
    };

    let plan = build_execution_plan(&request, &mut Vec::<LoadDiagnostic>::new())
        .expect("plan should build");
    let perimeters_stage = plan
        .per_layer_stages
        .iter()
        .find(|s| s.stage_id == "Layer::Perimeters")
        .expect("plan must contain Layer::Perimeters");

    let result = plan.resolve_active_regions(&global_layers[0], &perimeters_stage.modules[0]);
    assert!(
        result.is_empty(),
        "empty result for module with no regions must be an empty slice, not an error"
    );
}

fn active_region(object_id: &str, region_id: u64) -> ActiveRegion {
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id,
        resolved_config: ResolvedConfig::default(),
        effective_layer_height: 0.2,
        nonplanar_shell: None,
        is_catchup_layer: false,
        catchup_z_bottom: 0.0,
        tool_index: 0,
    }
}

// â”€â”€ PrePass::SeamPlanning stage order tests (TASK-159) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation() {
    use slicer_scheduler::STAGE_ORDER;

    // Find the indices of the three relevant stages in the canonical order.
    let seam_idx = STAGE_ORDER
        .iter()
        .position(|&s| s == "PrePass::SeamPlanning")
        .expect("STAGE_ORDER must contain PrePass::SeamPlanning");
    let layer_plan_idx = STAGE_ORDER
        .iter()
        .position(|&s| s == "PrePass::LayerPlanning")
        .expect("STAGE_ORDER must contain PrePass::LayerPlanning");
    let paint_seg_idx = STAGE_ORDER
        .iter()
        .position(|&s| s == "PrePass::PaintSegmentation")
        .expect("STAGE_ORDER must contain PrePass::PaintSegmentation");

    // PrePass::SeamPlanning must come AFTER PrePass::LayerPlanning
    assert!(
        seam_idx > layer_plan_idx,
        "PrePass::SeamPlanning (index {seam_idx}) must come after PrePass::LayerPlanning (index {layer_plan_idx})"
    );
    // PrePass::SeamPlanning must come BEFORE PrePass::PaintSegmentation
    assert!(
        seam_idx < paint_seg_idx,
        "PrePass::SeamPlanning (index {seam_idx}) must come before PrePass::PaintSegmentation (index {paint_seg_idx})"
    );
}
