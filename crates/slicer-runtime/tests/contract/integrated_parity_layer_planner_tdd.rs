#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use layer_planner_default::DefaultLayerPlanner;
use slicer_core::PrepassStageOutput;
use slicer_ir::{ConfigValue, ConfigView, SemVer, StageId};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LoadedModuleBuilder, PrepassStageRunner,
    WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::{
    flat_plate_object, identity_transform, mesh_fixture,
    parity_invariants::{assert_layer_plan_parity_structural, ParityTolerance},
    prepass_input, wasm_cache,
};

fn wasm_live<'a>(module: &'a slicer_runtime::CompiledModule) -> CompiledModuleLive<'a> {
    let loaded = LoadedModuleBuilder::new(
        module.module_id().as_str(),
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "PrePass::LayerPlanning",
        String::new(),
        PathBuf::from("/dev/null"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 5,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build instance pool"),
    );
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/layer-planner-default/layer-planner-default.wasm");
    assert!(
        path.exists(),
        "layer-planner-default guest is missing: {}",
        path.display()
    );
    CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(wasm_cache::compiled_component_at(&path)),
        module.claims(),
        Arc::clone(module.config_view()),
    )
}

#[test]
fn integrated_parity_layer_planner() {
    let config = Arc::new(ConfigView::from_map(
        [
            ("layer_height".into(), ConfigValue::Float(0.2)),
            ("first_layer_height".into(), ConfigValue::Float(0.2)),
            ("object_height:obj-1".into(), ConfigValue::Float(2.0)),
        ]
        .into_iter()
        .collect(),
    ));
    let wasm_module = CompiledModuleBuilder::new("com.core.layer-planner-default")
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new("com.core.layer-planner-default")
        .config_view(config)
        .build();
    let wasm_live = wasm_live(&wasm_module);
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(DefaultLayerPlanner::__slicer_native_entry());

    let blackboard = Blackboard::new(
        mesh_fixture(vec![flat_plate_object("obj-1", 0.0, identity_transform())]),
        0,
    );
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));
    let stage = StageId::from("PrePass::LayerPlanning");
    let native = PrepassStageRunner::run_stage(
        &dispatcher,
        &stage,
        &native_live,
        prepass_input(&blackboard),
    )
    .expect("native dispatch");
    let wasm =
        PrepassStageRunner::run_stage(&dispatcher, &stage, &wasm_live, prepass_input(&blackboard))
            .expect("wasm dispatch");
    assert!(matches!(native, PrepassStageOutput::LayerPlan(_)));
    assert!(matches!(wasm, PrepassStageOutput::LayerPlan(_)));
    assert_layer_plan_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect("layer planner native/wasm parity");
}
