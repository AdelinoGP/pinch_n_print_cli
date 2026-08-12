#![allow(missing_docs)]

use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, GlobalLayer, Point2, Polygon, SemVer, SliceIR, SlicedRegion, StageId,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};
use tree_support::TreeSupport;

use crate::common::{
    parity_invariants::{assert_parity_structural, ParityTolerance},
    wasm_cache,
};

fn support_slice() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "obj-0".to_string(),
            region_id: 0,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 10_000, y: 0 },
                        Point2 {
                            x: 10_000,
                            y: 10_000,
                        },
                        Point2 { x: 0, y: 10_000 },
                    ],
                },
                holes: Vec::new(),
            }],
            effective_layer_height: 0.2,
            ..Default::default()
        }],
    }
}

fn wasm_live<'a>(module: &'a slicer_runtime::CompiledModule) -> CompiledModuleLive<'a> {
    let loaded = LoadedModuleBuilder::new(
        module.module_id().as_str(),
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::Support",
        String::new(),
        std::path::PathBuf::from("/dev/null"),
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
    let component = wasm_cache::compiled_component_at(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/core-modules/tree-support/tree-support.wasm"),
    );
    CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(component),
        module.claims(),
        Arc::clone(module.config_view()),
    )
}

#[test]
fn integrated_parity_tree_support() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        (
            "enable_support".to_string(),
            slicer_ir::ConfigValue::Bool(true),
        ),
        (
            "support_density".to_string(),
            slicer_ir::ConfigValue::Float(20.0),
        ),
        ("line_width".to_string(), slicer_ir::ConfigValue::Float(0.4)),
    ])));
    let wasm_module = CompiledModuleBuilder::new("com.core.tree-support")
        .claims(vec!["support-generator".to_string()])
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new("com.core.tree-support")
        .claims(vec!["support-generator".to_string()])
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
    .with_native_entry(TreeSupport::__slicer_native_entry());
    let wasm_bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let native_bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_slice(support_slice())
        .expect("set wasm slice");
    native_arena
        .set_slice(support_slice())
        .expect("set native slice");
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::Support".to_string();
    let wasm = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &wasm_live,
        crate::common::layer_input(&wasm_bb, &wasm_arena),
    )
    .expect("wasm dispatch")
    .expect("wasm commit");
    let native = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &native_live,
        crate::common::layer_input(&native_bb, &native_arena),
    )
    .expect("native dispatch")
    .expect("native commit");
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("tree support native/wasm parity");
}
