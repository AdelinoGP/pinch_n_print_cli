#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, SemVer, StageId, ToolChange,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, FinalizationStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};
use wipe_tower::WipeTower;

use crate::common::{
    finalization_input,
    parity_invariants::{assert_finalization_parity_structural, ParityTolerance},
    wasm_cache,
};

fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn wasm_live<'a>(module: &'a slicer_runtime::CompiledModule) -> CompiledModuleLive<'a> {
    let loaded = LoadedModuleBuilder::new(
        module.module_id().as_str(),
        semver(),
        "PostPass::LayerFinalization",
        slicer_schema::TIER_FINALIZATION,
        PathBuf::from("/dev/null"),
    )
    .min_host_version(semver())
    .min_ir_schema(semver())
    .max_ir_schema(SemVer {
        major: 5,
        minor: 0,
        patch: 0,
    })
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
        .join("../../modules/core-modules/wipe-tower/wipe-tower.wasm");
    CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(wasm_cache::compiled_component_at(&path)),
        module.claims(),
        Arc::clone(module.config_view()),
    )
}

fn layers() -> Vec<LayerCollectionIR> {
    let z = 0.2;
    vec![LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z,
        // exhaustive: parity comparison pins every field explicitly
        ordered_entities: vec![PrintEntity {
            entity_id: 1,
            path: ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: 10.0,
                        y: 10.0,
                        z,
                        width: 0.4,
                        ..Default::default()
                    },
                    Point3WithWidth {
                        x: 20.0,
                        y: 10.0,
                        z,
                        width: 0.4,
                        ..Default::default()
                    },
                    Point3WithWidth {
                        x: 10.0,
                        y: 10.0,
                        z,
                        width: 0.4,
                        ..Default::default()
                    },
                ],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            role: ExtrusionRole::OuterWall,
            tool_index: 0,
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: "parity-object".into(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            topo_order: 0,
        }],
        tool_changes: vec![ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        }],
        ..Default::default()
    }]
}

#[test]
fn integrated_parity_wipe_tower() {
    let config = Arc::new(ConfigView::from_map(
        [
            ("wipe_tower_enabled".into(), ConfigValue::Bool(true)),
            ("wipe_tower_x".into(), ConfigValue::Float(10.0)),
            ("wipe_tower_y".into(), ConfigValue::Float(10.0)),
            ("wipe_tower_width".into(), ConfigValue::Float(60.0)),
            ("wipe_tower_purge_volume".into(), ConfigValue::Float(10.0)),
            ("line_width".into(), ConfigValue::Float(0.4)),
            (
                "bed_shape".into(),
                ConfigValue::List(vec![
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(200.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(200.0),
                    ConfigValue::Float(200.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(200.0),
                ]),
            ),
        ]
        .into_iter()
        .collect(),
    ));
    let wasm_module = CompiledModuleBuilder::new("com.core.wipe-tower")
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new("com.core.wipe-tower")
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
    .with_native_entry(WipeTower::__slicer_native_entry());
    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 0);
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));
    let stage: StageId = "PostPass::LayerFinalization".into();
    let mut native_layers = layers();
    let mut wasm_layers = native_layers.clone();
    FinalizationStageRunner::run_stage(
        &dispatcher,
        &stage,
        &native_live,
        finalization_input(&blackboard),
        &mut native_layers,
    )
    .expect("native finalization dispatch");
    FinalizationStageRunner::run_stage(
        &dispatcher,
        &stage,
        &wasm_live,
        finalization_input(&blackboard),
        &mut wasm_layers,
    )
    .expect("wasm finalization dispatch");
    assert!(!native_layers.is_empty());
    assert!(!wasm_layers.is_empty());
    assert_finalization_parity_structural(&native_layers, &wasm_layers, ParityTolerance::default())
        .expect("wipe tower native/wasm parity");
}
