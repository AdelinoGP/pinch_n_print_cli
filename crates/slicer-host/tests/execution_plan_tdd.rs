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
