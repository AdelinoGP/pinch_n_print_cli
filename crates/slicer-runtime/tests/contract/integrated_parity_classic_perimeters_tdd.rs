#![allow(missing_docs)]

//! AC-3 (ADR-0056): one `WasmRuntimeDispatcher`, two `CompiledModuleLive`
//! values for `com.core.classic-perimeters` (native entry vs real wasm
//! component), driven on a byte-identical `LayerStageInput`, must both return
//! `Ok(Some(LayerStageCommit::Perimeters))` and pass the Step-3 structural
//! parity comparator. Parity is structural, tolerance-based — never
//! byte-equality, never relaxed.

use std::path::PathBuf;
use std::sync::Arc;

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{
    ExPolygon, GlobalLayer, LayerStageCommit, Point2, Polygon, SemVer, SliceIR, SlicedRegion,
    StageId,
};
use slicer_runtime::{Blackboard, LayerArena, LayerStageRunner};

use crate::common::{
    classic_perimeters_baseline,
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_parity_structural, ParityTolerance},
};

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

#[test]
fn integrated_parity_classic_perimeters_native_matches_wasm() {
    // Bound-view shape (packet 06 5c-prime, design.md item 11): a bound view
    // always holds the module's contract-required reads at manifest defaults.
    let config = Arc::new(
        classic_perimeters_baseline()
            .int("wall_count", 3)
            .float("line_width", 0.4)
            .build(),
    );
    let mut bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
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
    let mut region_map = slicer_ir::RegionMapIR::default();
    let mut resolved = slicer_ir::ResolvedConfig {
        line_width: 0.4,
        wall_count: 3,
        ..Default::default()
    };
    for (key, value) in config.iter_entries() {
        if !resolved
            .apply_cli_key(key, value)
            .expect("resolve fixture config")
        {
            resolved.extensions.insert(key.into(), value.clone());
        }
    }
    let config_id = region_map.intern_config(resolved);
    for region in &wasm_arena.slice().expect("staged fixture slice").regions {
        region_map.entries.insert(
            slicer_ir::RegionKey {
                global_layer_index: layer.index,
                object_id: region.object_id.clone(),
                region_id: if region.variant_chain.is_empty() {
                    region.region_id
                } else {
                    region.region_id / 1_000_000
                },
                variant_chain: region.variant_chain.clone(),
            },
            slicer_ir::RegionPlan {
                config: config_id,
                ..Default::default()
            },
        );
    }
    bb.commit_region_map(Arc::new(region_map))
        .expect("commit fixture region map");
    let (native_commit, wasm_commit): (LayerStageCommit, LayerStageCommit) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: module_id(),
            wasm_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/classic-perimeters/classic-perimeters.wasm"),
            stage: stage.clone(),
            version: SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            min_ir_schema: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            max_ir_schema: SemVer {
                major: 2,
                minor: 0,
                patch: 0,
            },
            tier: String::new(),
            claims: Vec::new(),
            config: Arc::clone(&config),
            native_entry: ClassicPerimeters::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live, native_module, wasm_module| {
            let wasm_index = crate::common::selection_for_module(
                &stage,
                &layer,
                wasm_module,
                &bb,
                wasm_arena.slice(),
                None,
            )
            .expect("build wasm selection index");
            let native_index = crate::common::selection_for_module(
                &stage,
                &layer,
                native_module,
                &bb,
                native_arena.slice(),
                None,
            )
            .expect("build native selection index");
            let wasm = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                wasm_live,
                crate::common::layer_input(
                    &bb,
                    &wasm_arena,
                    wasm_index
                        .selection_for(layer.index, wasm_module.module_id())
                        .expect("bound wasm selection"),
                ),
            )
            .expect("wasm dispatch")
            .expect("wasm commit");
            let native = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                native_live,
                crate::common::layer_input(
                    &bb,
                    &native_arena,
                    native_index
                        .selection_for(layer.index, native_module.module_id())
                        .expect("bound native selection"),
                ),
            )
            .expect("native dispatch")
            .expect("native commit");
            (native, wasm)
        },
    );
    assert_parity_structural(
        &native_commit,
        &wasm_commit,
        ParityTolerance::default(),
        0.4,
    )
    .expect("AC-3 structural parity native vs wasm for classic-perimeters");
}
