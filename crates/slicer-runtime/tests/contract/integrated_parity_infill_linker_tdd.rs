#![allow(missing_docs)]

use std::sync::Arc;

use infill_linker::InfillLinker;
use slicer_ir::{
    ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer, InfillIR, InfillRegion,
    MeshIR, PerimeterIR, PerimeterRegion, Point2, Point3WithWidth, Polygon, SemVer, SliceIR,
    SlicedRegion, StageId,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::{
    parity_invariants::{assert_parity_structural, ParityTolerance},
    wasm_cache,
};

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    }
}

fn slice() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "parity-object".to_string(),
            region_id: 7,
            polygons: vec![square()],
            infill_areas: vec![square()],
            sparse_infill_area: vec![square()],
            effective_layer_height: 0.2,
            ..Default::default()
        }],
    }
}

fn segment(start: f32, end: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: start,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            Point3WithWidth {
                x: end,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

fn infill() -> InfillIR {
    InfillIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        regions: vec![InfillRegion {
            object_id: "parity-object".to_string(),
            region_id: 7,
            sparse_infill: vec![segment(1.0, 3.0), segment(3.0, 5.0), segment(5.0, 7.0)],
            ..Default::default()
        }],
    }
}

fn perimeter() -> PerimeterIR {
    PerimeterIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: "parity-object".to_string(),
            region_id: 7,
            infill_areas: vec![square()],
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
        "Layer::InfillPostProcess",
        String::new(),
        std::path::PathBuf::from("/dev/null"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 3,
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
            .join("../../modules/core-modules/infill-linker/infill-linker.wasm"),
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
fn integrated_parity_infill_linker() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        (
            "infill_overlap".to_string(),
            slicer_ir::ConfigValue::Float(0.45),
        ),
        ("line_width".to_string(), slicer_ir::ConfigValue::Float(0.4)),
        (
            "infill_density".to_string(),
            slicer_ir::ConfigValue::Float(0.2),
        ),
    ])));
    let wasm_module = CompiledModuleBuilder::new("com.core.infill-linker")
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new("com.core.infill-linker")
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
    .with_native_entry(InfillLinker::__slicer_native_entry());

    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    for arena in [&mut wasm_arena, &mut native_arena] {
        arena.set_slice(slice()).expect("set slice");
        arena.set_perimeter(perimeter()).expect("set perimeter");
        arena.set_infill(infill()).expect("set infill");
    }
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::InfillPostProcess".to_string();
    let wasm = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &wasm_live,
        crate::common::layer_input(&blackboard, &wasm_arena),
    )
    .expect("wasm dispatch")
    .expect("wasm commit");
    let native = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &native_live,
        crate::common::layer_input(&blackboard, &native_arena),
    )
    .expect("native dispatch")
    .expect("native commit");
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("infill-linker native/wasm parity");
}
