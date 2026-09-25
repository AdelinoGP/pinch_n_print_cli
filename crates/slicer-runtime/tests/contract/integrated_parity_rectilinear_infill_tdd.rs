#![allow(missing_docs)]

use std::sync::Arc;

use rectilinear_infill::RectilinearInfill;
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

#[test]
fn integrated_parity_rectilinear_infill() {
    let claims = vec![
        "claim:sparse-fill".to_string(),
        "claim:top-fill".to_string(),
        "claim:bottom-fill".to_string(),
        "claim:bridge-fill".to_string(),
    ];
    // Bound-view shape (packet 06 5c-prime): mirrors
    // `baseline_config` (`modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs`)
    // — every contract-required
    // read at manifest defaults, with this fixture's density/line_width winning.
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        (
            "infill_density".to_string(),
            slicer_ir::ConfigValue::Float(0.2),
        ),
        ("line_width".to_string(), slicer_ir::ConfigValue::Float(0.4)),
        (
            "bridge_line_width".to_string(),
            slicer_ir::ConfigValue::Float(0.0),
        ),
        (
            "initial_layer_line_width".to_string(),
            slicer_ir::ConfigValue::Float(0.0),
        ),
        (
            "top_surface_line_width".to_string(),
            slicer_ir::ConfigValue::Float(0.0),
        ),
        (
            "internal_solid_infill_line_width".to_string(),
            slicer_ir::ConfigValue::Float(0.0),
        ),
        (
            "sparse_infill_line_width".to_string(),
            slicer_ir::ConfigValue::Float(0.0),
        ),
        (
            "bridge_density".to_string(),
            slicer_ir::ConfigValue::Float(1.0),
        ),
        (
            "bridge_speed".to_string(),
            slicer_ir::ConfigValue::Float(25.0),
        ),
        (
            "bridge_flow".to_string(),
            slicer_ir::ConfigValue::Float(1.0),
        ),
        (
            "thick_bridges".to_string(),
            slicer_ir::ConfigValue::Bool(false),
        ),
        (
            "internal_bridge_density".to_string(),
            slicer_ir::ConfigValue::Float(1.0),
        ),
        (
            "internal_bridge_speed".to_string(),
            slicer_ir::ConfigValue::Float(37.5),
        ),
        (
            "internal_bridge_flow".to_string(),
            slicer_ir::ConfigValue::Float(1.0),
        ),
        (
            "thick_internal_bridges".to_string(),
            slicer_ir::ConfigValue::Bool(true),
        ),
        (
            "top_surface_speed".to_string(),
            slicer_ir::ConfigValue::Float(60.0),
        ),
        (
            "internal_solid_infill_speed".to_string(),
            slicer_ir::ConfigValue::Float(60.0),
        ),
        (
            "sparse_infill_speed".to_string(),
            slicer_ir::ConfigValue::Float(60.0),
        ),
        (
            "dont_filter_internal_bridges".to_string(),
            slicer_ir::ConfigValue::Bool(false),
        ),
        (
            "enable_extra_bridge_layer".to_string(),
            slicer_ir::ConfigValue::Bool(false),
        ),
        (
            "internal_bridge_angle".to_string(),
            slicer_ir::ConfigValue::Float(0.0),
        ),
        (
            "infill_shift_step".to_string(),
            slicer_ir::ConfigValue::Float(0.0),
        ),
    ])));
    let mut bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut region_map = RegionMapIR::default();
    let config_id = region_map.intern_config(ResolvedConfig {
        sparse_fill_holder: "com.core.rectilinear-infill".to_string(),
        ..Default::default()
    });
    region_map.entries.insert(
        RegionKey {
            global_layer_index: 5,
            object_id: "parity-object".into(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config: config_id,
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
    let stage: StageId = "Layer::Infill".into();
    let mut wasm_input = crate::common::layer_input(&bb, &wasm_arena);
    let mut native_input = crate::common::layer_input(&bb, &native_arena);
    wasm_input.paint_regions = Some(());
    native_input.paint_regions = Some(());
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.rectilinear-infill".into(),
            wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/rectilinear-infill/rectilinear-infill.wasm"),
            stage: "Layer::Infill".into(),
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
                major: 5,
                minor: 0,
                patch: 0,
            },
            tier: String::new(),
            claims,
            config,
            native_entry: RectilinearInfill::__slicer_native_entry(),
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
        .expect("rectilinear native/wasm parity");
}
