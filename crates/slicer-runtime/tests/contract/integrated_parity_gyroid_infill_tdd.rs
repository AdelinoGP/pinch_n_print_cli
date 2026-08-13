#![allow(missing_docs)]

use std::sync::Arc;

use gyroid_infill::GyroidInfill;
use slicer_ir::{
    ConfigView, ExPolygon, GlobalLayer, Point2, Polygon, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SemVer, SliceIR, SlicedRegion, StageId,
};
use slicer_runtime::{Blackboard, LayerArena, LayerStageRunner};

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_parity_structural, ParityTolerance},
};

fn non_empty_slice() -> SliceIR {
    let polygon = ExPolygon {
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

#[test]
fn integrated_parity_gyroid_infill() {
    let mut bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut region_map = RegionMapIR::default();
    let resolved = ResolvedConfig {
        sparse_fill_holder: "com.core.gyroid-infill".to_string(),
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
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.gyroid-infill".into(),
            wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/gyroid-infill/gyroid-infill.wasm"),
            stage: stage.clone(),
            version: SemVer {
                major: 1,
                minor: 0,
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
            claims: vec![
                "claim:sparse-fill".into(),
                "claim:top-fill".into(),
                "claim:bottom-fill".into(),
                "claim:bridge-fill".into(),
            ],
            config: Arc::new(ConfigView::new()),
            native_entry: GyroidInfill::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live| {
            let wasm =
                LayerStageRunner::run_stage(dispatcher, &stage, &layer, wasm_live, wasm_input)
                    .expect("wasm dispatch")
                    .expect("wasm commit");
            let native =
                LayerStageRunner::run_stage(dispatcher, &stage, &layer, native_live, native_input)
                    .expect("native dispatch")
                    .expect("native commit");
            (native, wasm)
        },
    );
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("gyroid native/wasm parity");
}
