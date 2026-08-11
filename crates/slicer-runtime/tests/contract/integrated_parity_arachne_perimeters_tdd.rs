#![allow(missing_docs)]

//! AC-4 + AC-6 runtime half (ADR-0056): one `WasmRuntimeDispatcher`, two
//! `CompiledModuleLive` values for `com.core.arachne-perimeters` (native
//! entry vs real wasm component), driven on a byte-identical
//! `LayerStageInput`, must both return
//! `Ok(Some(LayerStageCommit::Perimeters))` and pass the Step-3 structural
//! parity comparator — including the bead-count sequence and the
//! 2.0 × optimal-width bound (ADR-0042 D4, implemented by the comparator).
//! AC-6 runtime half: the native commit must be a non-empty wall set, which
//! is impossible unless the native arm of
//! `slicer_sdk::host::generate_arachne_walls` reached
//! `slicer_core::arachne::pipeline::run_arachne_pipeline`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use arachne_perimeters::ArachnePerimeters;
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

/// Coordinates use the seam's scale: 1 unit = 100 nm. A tapering
/// quadrilateral — 20 mm wide (200_000 units) at y=0 narrowing to 7 mm
/// (70_000 units) at y=200_000 — so arachne emits >= 2 beads in the wide
/// part and 1 in the narrow part (>= 1 bead transition along the loop).
fn taper_slice() -> SliceIR {
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
                            x: 135_000,
                            y: 200_000,
                        },
                        Point2 {
                            x: 65_000,
                            y: 200_000,
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
    "com.core.arachne-perimeters".to_string()
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
        .join("modules/core-modules/arachne-perimeters/arachne-perimeters.wasm");
    assert!(
        wasm_path.exists(),
        "real arachne-perimeters guest is missing: {} (run `cargo xtask build-guests`)",
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
fn integrated_parity_arachne_perimeters_native_matches_wasm() {
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
    .with_native_entry(ArachnePerimeters::__slicer_native_entry());
    let bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let slice = taper_slice();
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
    // AC-6 runtime half: the native path emits a NON-EMPTY wall set — at
    // least one loop with at least one point — proving the native arm of
    // `generate_arachne_walls` reached
    // `slicer_core::arachne::pipeline::run_arachne_pipeline`.
    match &native_commit {
        LayerStageCommit::Perimeters(ir) => {
            let non_empty_loop = ir
                .regions
                .iter()
                .flat_map(|r| &r.walls)
                .any(|w| !w.path.points.is_empty());
            assert!(
                non_empty_loop,
                "AC-6: native arachne commit must contain >= 1 loop with >= 1 point"
            );
        }
        other => panic!(
            "AC-4: native commit must be LayerStageCommit::Perimeters, got {:?}",
            other.stage_id()
        ),
    }
    assert_parity_structural(
        &native_commit,
        &wasm_commit,
        ParityTolerance::default(),
        0.4,
    )
    .expect("AC-4 structural parity native vs wasm for arachne-perimeters");
}
