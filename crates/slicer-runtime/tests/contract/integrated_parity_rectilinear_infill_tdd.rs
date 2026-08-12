#![allow(missing_docs)]

use std::sync::Arc;

use rectilinear_infill::RectilinearInfill;
use slicer_ir::{
    ConfigView, ExPolygon, GlobalLayer, Point2, Polygon, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SemVer, SliceIR, SlicedRegion, StageId,
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

fn non_empty_slice() -> SliceIR {
    let polygon = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 20_000, y: 0 },
                Point2 {
                    x: 20_000,
                    y: 20_000,
                },
                Point2 { x: 0, y: 20_000 },
            ],
        },
        holes: Vec::new(),
    };
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
            polygons: vec![polygon.clone()],
            infill_areas: vec![polygon.clone()],
            sparse_infill_area: vec![polygon],
            ..Default::default()
        }],
    }
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
            .join("../../modules/core-modules/rectilinear-infill/rectilinear-infill.wasm"),
    );
    let live = CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(Arc::clone(&component)),
        module.claims(),
        Arc::clone(module.config_view()),
    );
    (live, component)
}

#[test]
fn integrated_parity_rectilinear_infill() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let claims = vec![
        "claim:sparse-fill".to_string(),
        "claim:top-fill".to_string(),
        "claim:bottom-fill".to_string(),
        "claim:bridge-fill".to_string(),
    ];
    let wasm_module = CompiledModuleBuilder::new("com.core.rectilinear-infill")
        .claims(claims.clone())
        .config_view(Arc::new(ConfigView::from_map(
            std::collections::HashMap::from([
                (
                    "infill_density".to_string(),
                    slicer_ir::ConfigValue::Float(0.2),
                ),
                ("line_width".to_string(), slicer_ir::ConfigValue::Float(0.4)),
            ]),
        )))
        .build();
    let native_module = CompiledModuleBuilder::new("com.core.rectilinear-infill")
        .claims(claims)
        .config_view(Arc::new(ConfigView::from_map(
            std::collections::HashMap::from([
                (
                    "infill_density".to_string(),
                    slicer_ir::ConfigValue::Float(0.2),
                ),
                ("line_width".to_string(), slicer_ir::ConfigValue::Float(0.4)),
            ]),
        )))
        .build();
    let (wasm_live, _component) = wasm_live(&wasm_module);
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(RectilinearInfill::__slicer_native_entry());
    let mut bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut region_map = RegionMapIR::default();
    let resolved = ResolvedConfig {
        sparse_fill_holder: "com.core.rectilinear-infill".to_string(),
        ..Default::default()
    };
    let config = region_map.intern_config(resolved);
    region_map.entries.insert(
        RegionKey {
            global_layer_index: 5,
            object_id: "parity-object".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config,
            ..Default::default()
        },
    );
    bb.commit_region_map(Arc::new(region_map))
        .expect("commit region map");
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
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("rectilinear native/wasm parity");
}
