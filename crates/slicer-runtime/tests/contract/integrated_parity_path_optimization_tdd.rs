#![allow(missing_docs)]

use std::sync::Arc;

use path_optimization_default::PathOptimizationDefault;
use slicer_ir::{
    ConfigView, ExtrusionPath3D, ExtrusionRole, GlobalLayer, LayerStageCommit, LoopType, MeshIR,
    PerimeterIR, PerimeterRegion, Point3WithWidth, SemVer, StageId, WallBoundaryType, WallLoop,
    WidthProfile,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::{parity_invariants, wasm_cache};

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
        "Layer::PathOptimization",
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
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../modules/core-modules/path-optimization-default/path-optimization-default.wasm",
    );
    let component = wasm_cache::compiled_component_at(&path);
    let live = CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(Arc::clone(&component)),
        module.claims(),
        Arc::clone(module.config_view()),
    );
    (live, component)
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
    let region = |object_id: &str, offset: f32| PerimeterRegion {
        object_id: object_id.into(),
        region_id: 0,
        walls: vec![WallLoop {
            perimeter_index: 0,
            loop_type: LoopType::Outer,
            path: ExtrusionPath3D {
                points: points
                    .iter()
                    .map(|p| Point3WithWidth {
                        x: p.x + offset,
                        y: p.y,
                        ..*p
                    })
                    .collect(),
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            width_profile: WidthProfile {
                widths: points.iter().map(|point| point.width).collect(),
            },
            feature_flags: vec![Default::default(); points.len()],
            boundary_type: WallBoundaryType::ExteriorSurface,
        }],
        ..Default::default()
    };
    PerimeterIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 1,
        regions: vec![
            region("parity-object-a", 0.0),
            region("parity-object-b", 20.0),
        ],
    }
}

#[test]
fn integrated_parity_path_optimization() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let id = "com.core.path-optimization-default".to_string();
    let wasm_module = CompiledModuleBuilder::new(id.clone())
        .config_view(Arc::new(ConfigView::new()))
        .build();
    let native_module = CompiledModuleBuilder::new(id)
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
    .with_native_entry(PathOptimizationDefault::__slicer_native_entry());

    let wasm_bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let native_bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_perimeter(perimeter())
        .expect("set wasm perimeter");
    native_arena
        .set_perimeter(perimeter())
        .expect("set native perimeter");
    let wasm_input = crate::common::layer_input(&wasm_bb, &wasm_arena);
    let native_input = crate::common::layer_input(&native_bb, &native_arena);
    let layer = GlobalLayer {
        index: 1,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::PathOptimization".to_string();
    let wasm = LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &wasm_live, wasm_input)
        .expect("wasm dispatch")
        .expect("wasm commit");
    let native =
        LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &native_live, native_input)
            .expect("native dispatch")
            .expect("native commit");
    assert!(matches!(wasm, LayerStageCommit::PathOptimization(_)));
    assert!(matches!(native, LayerStageCommit::PathOptimization(_)));
    parity_invariants::assert_parity_structural(
        &native,
        &wasm,
        parity_invariants::ParityTolerance {
            coord_mm: 1e-3,
            ..Default::default()
        },
        0.4,
    )
    .expect("native and wasm path optimization parity");
}
