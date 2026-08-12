#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use seam_planner_default::SeamPlannerDefault;
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, LayerPlanIR, Point2, Polygon, SemVer, SliceIR,
    SlicedRegion, StageId, SurfaceClassificationIR,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LoadedModuleBuilder, PrepassStageRunner,
    WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::{
    flat_plate_object, identity_transform, mesh_fixture,
    parity_invariants::{assert_seam_parity_structural, ParityTolerance},
    prepass_input, wasm_cache,
};

fn slice() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "obj-1".into(),
            region_id: 0,
            effective_layer_height: 0.2,
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
        "PrePass::SeamPlanning",
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
        .join("../../modules/core-modules/seam-planner-default/seam-planner-default.wasm");
    assert!(
        path.exists(),
        "seam-planner-default guest is missing: {}",
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
fn integrated_parity_seam_planner() {
    let config = Arc::new(ConfigView::from_map(
        [("seam_mode".into(), ConfigValue::String("nearest".into()))]
            .into_iter()
            .collect(),
    ));
    let wasm_module = CompiledModuleBuilder::new("com.core.seam-planner-default")
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new("com.core.seam-planner-default")
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
    .with_native_entry(SeamPlannerDefault::__slicer_native_entry());

    let mut blackboard = Blackboard::new(
        mesh_fixture(vec![flat_plate_object("obj-1", 0.0, identity_transform())]),
        0,
    );
    blackboard
        .commit_layer_plan(Arc::new(LayerPlanIR {
            global_layers: vec![slicer_ir::GlobalLayer {
                index: 0,
                z: 0.2,
                ..Default::default()
            }],
            ..Default::default()
        }))
        .expect("commit layer plan");
    blackboard
        .commit_slice_ir(Arc::new(vec![slice()]))
        .expect("commit slice");
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .expect("commit surface classification");
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));
    let stage = StageId::from("PrePass::SeamPlanning");
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
    assert_seam_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect("seam planner native/wasm parity");
}
