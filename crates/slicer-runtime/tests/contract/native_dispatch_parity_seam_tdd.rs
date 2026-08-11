#![allow(missing_docs)]

use std::sync::Arc;

use sdk_layer_infill_guest::SdkLayerInfillModule;
use slicer_ir::{
    ConfigView, ExPolygon, GlobalLayer, LayerStageCommit, Point2, Polygon, SemVer, SliceIR,
    SlicedRegion, StageId,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::wasm_cache;

fn non_empty_slice() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 5,
        z: 1.0,
        regions: vec![SlicedRegion {
            object_id: "parity-object".to_string(),
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
                    ],
                },
                holes: Vec::new(),
            }],
            ..Default::default()
        }],
    }
}

fn module_id() -> slicer_ir::ModuleId {
    "com.test.sdk-layer-infill-parity".to_string()
}

fn wasm_live<'a>(
    module: &'a slicer_runtime::CompiledModule,
) -> (CompiledModuleLive<'a>, Arc<slicer_runtime::WasmComponent>) {
    let loaded = LoadedModuleBuilder::new(
        module.module_id().as_str(),
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Layer::Infill",
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
        major: 2,
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
    let component = wasm_cache::compiled_guest("sdk-layer-infill-guest");
    let live = CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(Arc::clone(&component)),
        module.claims(),
        Arc::clone(module.config_view()),
    );
    (live, component)
}

fn commit_shape(commit: &LayerStageCommit) -> Vec<(usize, Vec<(usize, slicer_ir::ExtrusionRole)>)> {
    let LayerStageCommit::Infill(infill) = commit else {
        panic!("expected infill commit")
    };
    infill
        .regions
        .iter()
        .map(|region| {
            (
                region.sparse_infill.len(),
                region
                    .sparse_infill
                    .iter()
                    .map(|path| (path.points.len(), path.role.clone()))
                    .collect(),
            )
        })
        .collect()
}

fn run_pair() -> (LayerStageCommit, LayerStageCommit) {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let wasm_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::new(ConfigView::new()))
        .build();
    let native_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::new(ConfigView::new()))
        .build();
    let (wasm_live, _component) = wasm_live(&wasm_module);
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(SdkLayerInfillModule::__slicer_native_entry());
    let bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_slice(non_empty_slice())
        .expect("set wasm slice");
    native_arena
        .set_slice(non_empty_slice())
        .expect("set native slice");
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        ..Default::default()
    };
    let stage: StageId = "Layer::Infill".to_string();
    let mut wasm_input = crate::common::layer_input(&bb, &wasm_arena);
    let mut native_input = crate::common::layer_input(&bb, &native_arena);
    wasm_input.paint_regions = Some(());
    native_input.paint_regions = Some(());
    let wasm = LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &wasm_live, wasm_input)
        .expect("wasm dispatch")
        .expect("wasm commit");
    let native =
        LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &native_live, native_input)
            .expect("native dispatch")
            .expect("native commit");
    (wasm, native)
}

fn run_native_only() -> LayerStageCommit {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let native_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::new(ConfigView::new()))
        .build();
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(SdkLayerInfillModule::__slicer_native_entry());
    let bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut arena = LayerArena::new();
    arena
        .set_slice(non_empty_slice())
        .expect("set native slice");
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        ..Default::default()
    };
    let stage: StageId = "Layer::Infill".to_string();
    let mut input = crate::common::layer_input(&bb, &arena);
    input.paint_regions = Some(());
    LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &native_live, input)
        .expect("native dispatch")
        .expect("native commit")
}

#[test]
fn native_dispatch_matches_wasm_structurally() {
    let (wasm, native) = run_pair();
    assert_eq!(commit_shape(&wasm), commit_shape(&native));
}

#[test]
fn native_dispatch_without_component() {
    assert!(matches!(run_native_only(), LayerStageCommit::Infill(_)));
}
