#![allow(missing_docs)]

//! Wayfinder perf-vs-orca ticket 37 — the `Layer::InfillPostProcess`
//! empty-output protocol at the producer boundary, on BOTH legs.
//!
//! ADR-0028 §Amendment 2026-09-29: an invocation that *ran* and re-emitted
//! nothing commits the empty replacement set, because zero paths there is a
//! clip verdict; only genuinely absent invocations leave the prior `InfillIR`
//! in place. `Layer::Infill` (merge semantics) is unaffected.

use slicer_ir::LayerStageCommit;
use slicer_sdk::native::NativeLayerResponse;
use slicer_wasm_host::dispatch::deconstruct_layer_ctx;
use slicer_wasm_host::host::HostExecutionContextBuilder;
use slicer_wasm_host::marshal::native::commit_native_layer_response;

fn empty_native_infill_response() -> NativeLayerResponse {
    // exhaustive: producer-boundary fixture names every response slot explicitly
    NativeLayerResponse {
        infill: Some(slicer_sdk::builders::InfillOutputBuilder::new()),
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    }
}

/// Wasm leg: a `Layer::InfillPostProcess` invocation with an all-empty infill
/// output must produce `Some(InfillPostProcess(empty))`, not `None`.
#[test]
fn wasm_infill_postprocess_empty_output_commits_the_empty_replacement() {
    let ctx = HostExecutionContextBuilder::new("t37-empty-wasm", 0.2, 0.2).build();
    let commit = deconstruct_layer_ctx(
        "Layer::InfillPostProcess",
        "t37-empty-wasm",
        4,
        None,
        ctx,
        None,
    )
    .expect("empty postprocess output is a valid commit")
    .expect("a ran InfillPostProcess invocation must commit");

    let LayerStageCommit::InfillPostProcess(ir) = &commit else {
        panic!("expected InfillPostProcess, got {commit:?}");
    };
    assert!(
        ir.regions.is_empty() && ir.raft_regions.is_empty(),
        "the empty replacement set carries no region buckets and no raft regions"
    );
    assert_eq!(ir.global_layer_index, 4, "layer index is carried through");
}

/// Wasm leg contrast: `Layer::Infill` keeps its merge semantics — an all-empty
/// output there is "no contribution" and stays `None`.
#[test]
fn wasm_infill_empty_output_still_commits_nothing() {
    let ctx = HostExecutionContextBuilder::new("t37-empty-infill", 0.2, 0.2).build();
    let commit = deconstruct_layer_ctx("Layer::Infill", "t37-empty-infill", 4, None, ctx, None)
        .expect("empty infill output is not an error");
    assert!(
        commit.is_none(),
        "Layer::Infill is a merge stage: empty output means no contribution; got {commit:?}"
    );
}

/// Native leg: same rule, same result, so the two legs cannot diverge.
#[test]
fn native_infill_postprocess_empty_output_commits_the_empty_replacement() {
    let response = empty_native_infill_response();
    let commit = commit_native_layer_response(&response, "Layer::InfillPostProcess", 4, None)
        .expect("native empty postprocess output is a valid commit")
        .expect("a ran native InfillPostProcess invocation must commit");

    let LayerStageCommit::InfillPostProcess(ir) = &commit else {
        panic!("expected InfillPostProcess, got {commit:?}");
    };
    assert!(
        ir.regions.is_empty() && ir.raft_regions.is_empty(),
        "the native leg must emit the same empty replacement set as the wasm leg"
    );
}

/// Native leg contrast: `Layer::Infill` still commits nothing for empty output.
#[test]
fn native_infill_empty_output_still_commits_nothing() {
    let response = empty_native_infill_response();
    let commit = commit_native_layer_response(&response, "Layer::Infill", 4, None)
        .expect("native empty infill output is not an error");
    assert!(
        commit.is_none(),
        "Layer::Infill merge semantics: empty output means no contribution; got {commit:?}"
    );
}

/// Both legs agree byte-for-byte on the empty replacement, so the
/// integrated/external parity gate is not perturbed by this protocol.
#[test]
fn both_legs_agree_on_the_empty_replacement_shape() {
    let ctx = HostExecutionContextBuilder::new("t37-empty-parity", 0.2, 0.2).build();
    let wasm = deconstruct_layer_ctx(
        "Layer::InfillPostProcess",
        "t37-empty-parity",
        9,
        None,
        ctx,
        None,
    )
    .expect("wasm commit")
    .expect("wasm commit present");
    let native = commit_native_layer_response(
        &empty_native_infill_response(),
        "Layer::InfillPostProcess",
        9,
        None,
    )
    .expect("native commit")
    .expect("native commit present");
    assert_eq!(
        wasm, native,
        "the two legs must produce identical empty replacement sets"
    );
}

/// A raft-only output is NOT the empty case on either leg: it must fall through
/// and commit its raft regions. Both legs previously disagreed here — the wasm
/// check included `raft_fill`, the native one did not — so the native leg
/// silently dropped a raft-only InfillPostProcess output.
#[test]
fn raft_only_output_is_not_the_empty_case_on_either_leg() {
    use slicer_ir::{ExPolygon, Point2, Polygon};

    fn square_region() -> Vec<ExPolygon> {
        vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(1.0, 0.0),
                    Point2::from_mm(1.0, 1.0),
                    Point2::from_mm(0.0, 1.0),
                ],
            },
            holes: Vec::new(),
        }]
    }

    // Wasm leg: push one raft group, no paths. The host context's raft slot
    // takes the WIT-shaped polygon, so marshal through the public converter.
    let wit_polygons = slicer_wasm_host::marshal::ir_to_wit_expolygons(&square_region());
    let mut ctx = HostExecutionContextBuilder::new("t37-raft-only", 0.2, 0.2).build();
    ctx.infill_output_mut().raft_fill.push(wit_polygons);
    ctx.infill_output_mut().raft_fill_origins.push(None);
    let wasm = deconstruct_layer_ctx(
        "Layer::InfillPostProcess",
        "t37-raft-only",
        3,
        None,
        ctx,
        None,
    )
    .expect("wasm raft-only commit")
    .expect("raft-only output must commit");

    // Native leg: same payload.
    let mut builder = slicer_sdk::builders::InfillOutputBuilder::new();
    builder
        .push_raft_fill(square_region())
        .expect("push raft fill");
    // exhaustive: producer-boundary fixture names every response slot explicitly
    let response = NativeLayerResponse {
        infill: Some(builder),
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    };
    let native = commit_native_layer_response(&response, "Layer::InfillPostProcess", 3, None)
        .expect("native raft-only commit")
        .expect("native raft-only output must commit");

    assert_eq!(
        wasm, native,
        "a raft-only output must commit identically on both legs"
    );
    let LayerStageCommit::InfillPostProcess(ir) = &wasm else {
        panic!("expected InfillPostProcess, got {wasm:?}");
    };
    assert!(
        !ir.raft_regions.is_empty(),
        "the raft regions carried by a raft-only output must survive"
    );
}
