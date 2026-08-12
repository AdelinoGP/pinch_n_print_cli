#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use part_cooling::PartCooling;
use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, SemVer, StageId,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, FinalizationStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};

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
        .join("../../modules/core-modules/part-cooling/part-cooling.wasm");
    CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(wasm_cache::compiled_component_at(&path)),
        module.claims(),
        Arc::clone(module.config_view()),
    )
}

fn layer(index: u32, role: ExtrusionRole) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: index,
        z: 0.2 + index as f32 * 0.2,
        // exhaustive: parity comparison pins every field explicitly
        ordered_entities: vec![PrintEntity {
            entity_id: index as u64 + 1,
            path: ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: 0.2 + index as f32 * 0.2,
                        width: 0.4,
                        ..Default::default()
                    },
                    Point3WithWidth {
                        x: 10.0,
                        y: 0.0,
                        z: 0.2 + index as f32 * 0.2,
                        width: 0.4,
                        ..Default::default()
                    },
                ],
                role: role.clone(),
                speed_factor: 1.0,
            },
            role,
            tool_index: 0,
            region_key: RegionKey {
                global_layer_index: index,
                object_id: "parity-object".into(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            topo_order: 0,
        }],
        ..Default::default()
    }
}

#[test]
fn integrated_parity_part_cooling() {
    let config = Arc::new(ConfigView::from_map(
        [
            ("fan_speed_max".into(), ConfigValue::Int(200)),
            ("disable_fan_first_layers".into(), ConfigValue::Int(0)),
            ("enable_overhang_fan".into(), ConfigValue::Bool(true)),
            ("overhang_fan_speed".into(), ConfigValue::Int(100)),
        ]
        .into_iter()
        .collect(),
    ));
    let wasm_module = CompiledModuleBuilder::new("com.core.part-cooling")
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new("com.core.part-cooling")
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
    .with_native_entry(PartCooling::__slicer_native_entry());
    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 0);
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));
    let stage: StageId = "PostPass::LayerFinalization".into();
    let mut native_layers = vec![
        layer(0, ExtrusionRole::OuterWall),
        layer(1, ExtrusionRole::BridgeInfill),
    ];
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
        .expect("part cooling native/wasm parity");
}
