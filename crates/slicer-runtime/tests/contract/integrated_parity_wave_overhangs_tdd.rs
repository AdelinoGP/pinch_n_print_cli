#![allow(missing_docs)]
//! Packet 246 AC-9, native-vs-wasm half.
//!
//! `deterministic_double_run` in
//! `modules/core-modules/wave-overhangs/tests/wave_overhangs_tdd.rs` covers the
//! "runs twice on identical input" half of AC-9 on the NATIVE path only. This
//! file covers the "native vs wasm dispatch" half, following the
//! `run_integrated_parity` / `assert_parity_structural` family precedent (see
//! `integrated_parity_rectilinear_infill_tdd.rs`).
//!
//! The fixture must make waves actually engage, or the comparison degrades to
//! comparing two rectilinear fallbacks and proves nothing about the ported
//! generator. Waves need supported material below the bridge, which reaches the
//! module as `prev_layer_boundary` — sourced from
//! `SurfaceClassificationIR.prev_layer_boundaries`, not from `SlicedRegion`.
//! `assert_waves_engaged` guards that: it fails if either leg produced no
//! order-locked path.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, ExtrusionRole, GlobalLayer, Point2, Polygon, RegionKey, RegionMapIR,
    RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion, StageId,
    SurfaceClassificationIR,
};
use slicer_runtime::{Blackboard, LayerArena, LayerStageRunner};
use wave_overhangs::WaveOverhangs;

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_parity_structural, ParityTolerance},
};

const OBJECT_ID: &str = "wave-parity-object";
const LAYER_INDEX: u32 = 5;
const LAYER_Z: f32 = 1.0;

/// Mirrors `supported_square_fixture` in the module's own test file: a 10x10 mm
/// unsupported square inside a wide band of supported material.
fn rect_mm(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

/// Rectangular frame (`outer` minus `inner`) as a single hole-bearing polygon.
fn frame_mm(o0: f32, o1: f32, i0: f32, i1: f32) -> ExPolygon {
    ExPolygon {
        contour: rect_mm(o0, o0, o1, o1).contour,
        holes: vec![Polygon {
            points: vec![
                Point2::from_mm(i0, i0),
                Point2::from_mm(i0, i1),
                Point2::from_mm(i1, i1),
                Point2::from_mm(i1, i0),
            ],
        }],
    }
}

fn supported() -> Vec<ExPolygon> {
    vec![frame_mm(-6.0, 16.0, 0.0, 10.0)]
}

fn bridge_slice() -> SliceIR {
    // The region polygon must cover BOTH the bridge square and the supporting
    // frame: `prev_layer_boundary` is clipped against `region.polygons` on the
    // way in, so a region limited to the square would strip the anchors.
    SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: LAYER_INDEX,
        z: LAYER_Z,
        regions: vec![SlicedRegion {
            object_id: OBJECT_ID.to_string(),
            region_id: 0,
            polygons: vec![rect_mm(-6.0, -6.0, 16.0, 16.0)],
            is_bridge: true,
            bridge_areas: vec![rect_mm(0.0, 0.0, 10.0, 10.0)],
            bridge_orientation_deg: 0.0,
            bottom_solid_fill: supported(),
            effective_layer_height: 0.2,
            ..Default::default()
        }],
    }
}

fn surface_classification() -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        prev_layer_boundaries: HashMap::from([(
            OBJECT_ID.to_string(),
            HashMap::from([(LAYER_INDEX, supported())]),
        )]),
        ..Default::default()
    }
}

fn config() -> Arc<ConfigView> {
    Arc::new(ConfigView::from_map(HashMap::from([
        (
            "nozzle_diameter".to_string(),
            slicer_ir::ConfigValue::Float(0.4),
        ),
        (
            "layer_height".to_string(),
            slicer_ir::ConfigValue::Float(0.2),
        ),
        (
            "bridge_speed".to_string(),
            slicer_ir::ConfigValue::Float(25.0),
        ),
        (
            "wave_overhang_print_speed".to_string(),
            slicer_ir::ConfigValue::Float(2.0),
        ),
        (
            "wave_overhang_flow_mm3_per_mm".to_string(),
            slicer_ir::ConfigValue::Float(0.15),
        ),
        (
            "wave_overhang_anchor_depth_mm".to_string(),
            slicer_ir::ConfigValue::Float(3.0),
        ),
        ("wall_count".to_string(), slicer_ir::ConfigValue::Int(3)),
    ])))
}

/// Guard against a vacuous parity pass: if waves never engaged, both legs are
/// conventional rectilinear fallback and the comparison says nothing about the
/// ported generator.
fn assert_waves_engaged(commit: &slicer_ir::LayerStageCommit, leg: &str) {
    let slicer_ir::LayerStageCommit::Infill(commit) = commit else {
        panic!("{leg} leg did not produce a Layer::Infill commit");
    };
    let locked = commit
        .regions
        .iter()
        .flat_map(|region| region.solid_infill.iter())
        .filter(|path| path.order_lock.is_some())
        .count();
    assert!(
        locked > 0,
        "{leg} leg produced no order-locked wave path; parity would be vacuous"
    );
    for region in &commit.regions {
        for path in &region.solid_infill {
            if path.order_lock.is_some() {
                assert_eq!(
                    path.role,
                    ExtrusionRole::BridgeInfill,
                    "{leg} leg emitted a locked path with an unexpected role"
                );
            }
        }
    }
}

/// Pins the native/integrated dispatch leg against the wasm leg.
///
/// This test was written red and quarantined: `build_native_layer_request`
/// (`crates/slicer-wasm-host/src/marshal/native.rs`) constructed each region
/// with `SliceRegionView::from_ir` and then set only `config`, never populating
/// the four `SurfaceClassificationIR`-derived fields that the wasm leg's
/// `sliced_region_to_data` (`crates/slicer-wasm-host/src/marshal/in_.rs`) does:
/// `prev_layer_boundary`, `overhang_areas`, `overhang_quartile_polygons`, and
/// `surface_group`. The native leg saw `prev_layer_boundary == []`, found no
/// anchors, and fell back to unlocked rectilinear scanlines while the wasm leg
/// emitted order-locked wave paths.
///
/// The defect was never wave-overhangs-specific: any module reading those
/// fields lost them under native/integrated dispatch.
/// `populate_surface_classification_fields` in
/// `crates/slicer-wasm-host/src/marshal/native.rs` now mirrors the wasm leg's
/// derivation, and this test is the regression pin for it. It is deliberately
/// strict: `assert_waves_engaged` rejects a vacuous pass where both legs fall
/// back, and the structural comparison must hold path-for-path.
#[test]
fn integrated_parity_wave_overhangs_native_matches_wasm() {
    let claims = vec!["claim:bridge-fill".to_string()];

    let mut bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut region_map = RegionMapIR::default();
    let config_id = region_map.intern_config(ResolvedConfig {
        bridge_fill_holder: "com.core.wave-overhangs".to_string(),
        ..Default::default()
    });
    region_map.entries.insert(
        RegionKey {
            global_layer_index: LAYER_INDEX,
            object_id: OBJECT_ID.into(),
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
    bb.commit_surface_classification(Arc::new(surface_classification()))
        .expect("commit surface classification");

    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena.set_slice(bridge_slice()).expect("wasm slice");
    native_arena.set_slice(bridge_slice()).expect("native slice");

    let layer = GlobalLayer {
        index: LAYER_INDEX,
        z: LAYER_Z,
        ..Default::default()
    };
    let stage: StageId = "Layer::Infill".into();
    let mut wasm_input = crate::common::layer_input(&bb, &wasm_arena);
    let mut native_input = crate::common::layer_input(&bb, &native_arena);
    wasm_input.paint_regions = Some(());
    native_input.paint_regions = Some(());

    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.wave-overhangs".into(),
            wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/wave-overhangs/wave-overhangs.wasm"),
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
            config: config(),
            native_entry: WaveOverhangs::__slicer_native_entry(),
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

    assert_waves_engaged(&wasm, "wasm");
    assert_waves_engaged(&native, "native");
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("wave-overhangs native/wasm parity");
}

