#![allow(missing_docs)]
//! Regression guard: the native `Layer::Infill` dispatch must resolve each
//! region's held fill-claim set per-region (against the configured fill
//! holder), not hand every module its full declared claim set.
//!
//! Root cause (fixed): `build_native_layer_request` set `held_claims =
//! module.claims` (the module's *declared* claims) for every region. Since
//! `should_emit(role)` gates on the held set, every infill module then saw
//! itself as holding all its fill claims on every region and emitted over
//! areas claimed by other modules. The wasm leg already resolved held claims
//! per-region via `held_claims_for`; the native leg now shares the same
//! `resolve_layer_held_claims_map`.

use std::sync::Arc;

use gyroid_infill::GyroidInfill;
use rectilinear_infill::RectilinearInfill;
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, LayerStageCommit, Point2, Polygon, SliceIR, SlicedRegion,
    StageId,
};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::wasm_cache;

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

fn config() -> Arc<ConfigView> {
    Arc::new(ConfigView::from_map(
        [
            ("infill_density".into(), ConfigValue::Float(0.5)),
            ("line_width".into(), ConfigValue::Float(0.4)),
        ]
        .into_iter()
        .collect(),
    ))
}

fn slice_with_sparse_region() -> SliceIR {
    let sq = square();
    SliceIR {
        schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "obj".into(),
            region_id: 0,
            polygons: vec![sq.clone()],
            infill_areas: vec![sq.clone()],
            sparse_infill_area: vec![sq.clone()],
            effective_layer_height: 0.2,
            ..Default::default()
        }],
    }
}

/// Run the native `Layer::Infill` dispatch for `module_id` with the given
/// declared claims, returning the total sparse-path count emitted.
fn native_sparse_count(module_id: &str, claims: Vec<String>) -> usize {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut arena = LayerArena::new();
    arena
        .set_slice(slice_with_sparse_region())
        .expect("set slice");

    let cfg = config();
    let module = CompiledModuleBuilder::new(module_id)
        .claims(claims)
        .config_view(Arc::clone(&cfg))
        .build();
    let live = CompiledModuleLive::new(
        module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        module.claims(),
        Arc::clone(module.config_view()),
    )
    .with_native_entry(match module_id {
        "com.core.gyroid-infill" => GyroidInfill::__slicer_native_entry(),
        "com.core.rectilinear-infill" => RectilinearInfill::__slicer_native_entry(),
        other => panic!("unhandled module {other}"),
    });

    let layer = slicer_ir::GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::Infill".into();
    let input = crate::common::layer_input(&blackboard, &arena);
    let result = LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &live, input)
        .expect("native dispatch");
    match result {
        None => 0,
        Some(LayerStageCommit::Infill(infill)) => {
            infill.regions.iter().map(|r| r.sparse_infill.len()).sum()
        }
        Some(other) => panic!("unexpected commit {other:?}"),
    }
}

/// Negative: gyroid declares `claim:sparse-fill` but the default region's
/// `sparse_fill_holder` is `rectilinear-infill`, so gyroid holds nothing and
/// must emit zero sparse paths. Pre-fix this emitted 21 paths (the bug).
#[test]
fn native_gyroid_holds_nothing_by_default() {
    let count = native_sparse_count(
        "com.core.gyroid-infill",
        vec!["claim:sparse-fill".to_string()],
    );
    assert_eq!(
        count, 0,
        "gyroid holds no fill claim on a default region (sparse_fill_holder=rectilinear), \
         but native dispatch emitted {count} sparse paths — held_claims not resolved"
    );
}

/// Positive: rectilinear is the default `sparse_fill_holder`, so it holds the
/// claim and must still emit sparse paths. Guards against the fix over-suppressing.
#[test]
fn native_rectilinear_holds_sparse_by_default() {
    let count = native_sparse_count(
        "com.core.rectilinear-infill",
        vec!["claim:sparse-fill".to_string()],
    );
    assert!(
        count > 0,
        "rectilinear is the default sparse_fill_holder; expected ≥1 sparse path, got {count}"
    );
}
