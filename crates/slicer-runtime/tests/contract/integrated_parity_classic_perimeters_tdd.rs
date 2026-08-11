#![allow(missing_docs)]

//! AC-3 (ADR-0056): one `WasmRuntimeDispatcher`, two `CompiledModuleLive`
//! values for `com.core.classic-perimeters` (native entry vs real wasm
//! component), driven on a byte-identical `LayerStageInput`, must both return
//! `Ok(Some(LayerStageCommit::Perimeters))` and pass the Step-3 structural
//! parity comparator. Parity is structural, tolerance-based — never
//! byte-equality, never relaxed.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, GlobalLayer, LayerStageCommit, Point2, Polygon, SemVer,
    SliceIR, SlicedRegion, StageId,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::parity_invariants::{assert_parity_structural, ParityTolerance};
use crate::common::wasm_cache;

/// Coordinates use the seam's scale: 1 unit = 100 nm, so a 20 mm square is
/// 200_000 units. The 8 mm centered square hole (80_000 units) yields a
/// region with >= 2 nesting levels (outer contour + hole perimeter).
fn holed_slice() -> SliceIR {
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
                        Point2 { x: 200_000, y: 0 },
                        Point2 {
                            x: 200_000,
                            y: 200_000,
                        },
                        Point2 { x: 0, y: 200_000 },
                    ],
                },
                holes: vec![Polygon {
                    points: vec![
                        Point2 {
                            x: 60_000,
                            y: 60_000,
                        },
                        Point2 {
                            x: 140_000,
                            y: 60_000,
                        },
                        Point2 {
                            x: 140_000,
                            y: 140_000,
                        },
                        Point2 {
                            x: 60_000,
                            y: 140_000,
                        },
                    ],
                }],
            }],
            ..Default::default()
        }],
    }
}

fn module_id() -> slicer_ir::ModuleId {
    "com.core.classic-perimeters".to_string()
}

fn wasm_live<'a>(
    module: &'a slicer_runtime::CompiledModule,
) -> (CompiledModuleLive<'a>, Arc<slicer_runtime::WasmComponent>) {
    let loaded = LoadedModuleBuilder::new(
        module.module_id().as_str(),
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::Perimeters",
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
    // CARGO_MANIFEST_DIR is crates/slicer-runtime; two levels up is repo root.
    let wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("modules/core-modules/classic-perimeters/classic-perimeters.wasm");
    assert!(
        wasm_path.exists(),
        "real classic-perimeters guest is missing: {} (run `cargo xtask build-guests`)",
        wasm_path.display()
    );
    let component = wasm_cache::compiled_component_at(&wasm_path);
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
fn integrated_parity_classic_perimeters_native_matches_wasm() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let config = Arc::new(ConfigView::from_map(HashMap::from([(
        "line_width".to_owned(),
        ConfigValue::Float(0.4),
    )])));
    let wasm_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::clone(&config))
        .build();
    let (wasm_live, _component) = wasm_live(&wasm_module);
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(ClassicPerimeters::__slicer_native_entry());
    let bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let slice = holed_slice();
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena.set_slice(slice.clone()).expect("set wasm slice");
    native_arena.set_slice(slice).expect("set native slice");
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        ..Default::default()
    };
    let stage: StageId = "Layer::Perimeters".to_string();
    let wasm_input = crate::common::layer_input(&bb, &wasm_arena);
    let native_input = crate::common::layer_input(&bb, &native_arena);
    let wasm_commit: LayerStageCommit =
        LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &wasm_live, wasm_input)
            .expect("wasm dispatch")
            .expect("wasm commit");
    let native_commit: LayerStageCommit =
        LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &native_live, native_input)
            .expect("native dispatch")
            .expect("native commit");
    assert_parity_structural(
        &native_commit,
        &wasm_commit,
        ParityTolerance::default(),
        0.4,
    )
    .expect("AC-3 structural parity native vs wasm for classic-perimeters");
}
