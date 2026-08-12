#![allow(missing_docs)]

use crate::common::{
    parity_invariants::{assert_parity_structural, ParityTolerance},
    wasm_cache,
};
use fuzzy_skin::FuzzySkinModule;
use slicer_ir::{
    ConfigView, ExtrusionPath3D, ExtrusionRole, GlobalLayer, LoopType, MeshIR, PerimeterIR,
    PerimeterRegion, Point3WithWidth, SemVer, StageId, WallBoundaryType, WallFeatureFlags,
    WallLoop, WidthProfile,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};
use std::sync::Arc;

fn wasm_live<'a>(module: &'a slicer_runtime::CompiledModule) -> CompiledModuleLive<'a> {
    let loaded = LoadedModuleBuilder::new(
        module.module_id().as_str(),
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::PerimetersPostProcess",
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
            .join("../../modules/core-modules/fuzzy-skin/fuzzy-skin.wasm"),
    );
    CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(component),
        module.claims(),
        Arc::clone(module.config_view()),
    )
}

fn perimeter() -> PerimeterIR {
    let points = vec![
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
        (0.0, 0.0),
    ]
    .into_iter()
    .map(|(x, y)| Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        ..Default::default()
    })
    .collect::<Vec<_>>();
    let flags = (0..points.len())
        .map(|_| WallFeatureFlags {
            fuzzy_skin: true,
            ..Default::default()
        })
        .collect::<Vec<_>>();
    PerimeterIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: "parity-object".into(),
            region_id: 0,
            walls: vec![WallLoop {
                perimeter_index: 0,
                loop_type: LoopType::Outer,
                path: ExtrusionPath3D {
                    points: points.clone(),
                    role: ExtrusionRole::OuterWall,
                    speed_factor: 1.0,
                },
                width_profile: WidthProfile {
                    widths: points.iter().map(|p| p.width).collect(),
                },
                feature_flags: flags,
                boundary_type: WallBoundaryType::ExteriorSurface,
            }],
            ..Default::default()
        }],
    }
}

#[test]
fn integrated_parity_fuzzy_skin() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        ("thickness".into(), slicer_ir::ConfigValue::Float(0.3)),
        ("point_distance".into(), slicer_ir::ConfigValue::Float(0.5)),
        ("apply_to_all".into(), slicer_ir::ConfigValue::Bool(true)),
    ])));
    let wasm_module = CompiledModuleBuilder::new("com.core.fuzzy-skin")
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new("com.core.fuzzy-skin")
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
    .with_native_entry(FuzzySkinModule::__slicer_native_entry());
    let bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_perimeter(perimeter())
        .expect("set wasm perimeter");
    native_arena
        .set_perimeter(perimeter())
        .expect("set native perimeter");
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::PerimetersPostProcess".into();
    let wasm = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &wasm_live,
        crate::common::layer_input(&bb, &wasm_arena),
    )
    .expect("wasm dispatch")
    .expect("wasm commit");
    let native = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &native_live,
        crate::common::layer_input(&bb, &native_arena),
    )
    .expect("native dispatch")
    .expect("native commit");
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("fuzzy skin native/wasm parity");
}
