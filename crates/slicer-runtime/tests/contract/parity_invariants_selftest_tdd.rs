#![allow(missing_docs)]

//! Self-tests for the structural parity comparator (packet 204, Step 3).
//! These prove the comparator is neither vacuous (it rejects dropped
//! geometry) nor byte-exact (it accepts ULP-scale drift) BEFORE any pilot
//! module is compared — AC-N2, AC-N3 (layer family), AC-N6 (prepass family).

use std::sync::Arc;

use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerStageCommit, LoopType, PerimeterIR, PerimeterRegion,
    Point3WithWidth, SupportPlanEntry, SupportPlanIR, WallBoundaryType, WallFeatureFlags, WallLoop,
    WidthProfile,
};
use slicer_runtime::PrepassStageOutput;

use crate::common::parity_invariants::{
    assert_parity_structural, assert_prepass_parity_structural, ParityTolerance,
};
use crate::common::semver;

fn pt(x: f32, y: f32, z: f32, width: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width,
        ..Default::default()
    }
}

/// Closed square wall centred on origin; one wider bead at point index 1 so
/// the bead-count sequence carries a real transition pair. `jitter` shifts
/// every coordinate (never widths) to simulate ULP-scale drift.
fn square_wall(perimeter_index: u32, half: f32, jitter: f32) -> WallLoop {
    let coords = [
        (-half, -half),
        (half, -half),
        (half, half),
        (-half, half),
        (-half, -half),
    ];
    let widths = [0.4, 0.6, 0.4, 0.4, 0.4];
    let points: Vec<Point3WithWidth> = coords
        .iter()
        .zip(widths)
        .map(|((x, y), w)| pt(x + jitter, y + jitter, 3.0 + jitter, w))
        .collect();
    WallLoop {
        perimeter_index,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points,
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile {
            widths: widths.to_vec(),
        },
        feature_flags: vec![WallFeatureFlags::default(); 5],
        boundary_type: WallBoundaryType::Interior,
    }
}

fn perimeter_commit(jitter: f32) -> LayerStageCommit {
    LayerStageCommit::Perimeters(PerimeterIR {
        schema_version: semver(),
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: "parity-object".to_string(),
            region_id: 0,
            walls: vec![square_wall(0, 5.0, jitter), square_wall(1, 2.5, jitter)],
            ..Default::default()
        }],
    })
}

fn perimeters_of(commit: LayerStageCommit) -> PerimeterIR {
    match commit {
        LayerStageCommit::Perimeters(ir) => ir,
        other => panic!("expected Perimeters, got {:?}", other.stage_id()),
    }
}

#[test]
fn parity_comparator_accepts_ulp_perturbation() {
    // AC-N2: structurally identical commits whose every coordinate differs by
    // 1e-6 mm must pass — the gate is structural, never byte-exact.
    let native = perimeter_commit(0.0);
    let wasm = perimeter_commit(1e-6);
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("ULP-scale perturbation must be accepted");
}

#[test]
fn parity_comparator_rejects_dropped_loop() {
    // AC-N3(i): the wasm path missing one closed loop must fail on loop count.
    let native = perimeter_commit(0.0);
    let mut dropped = perimeters_of(perimeter_commit(0.0));
    dropped.regions[0].walls.pop();
    let wasm = LayerStageCommit::Perimeters(dropped);
    let err = assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect_err("dropped loop must be rejected");
    assert!(
        err.contains("loop count"),
        "error must name the loop count invariant: {err}"
    );

    // AC-N3(ii): equal loop count but a differing point count in one loop.
    let mut shrunk = perimeters_of(perimeter_commit(0.0));
    shrunk.regions[0].walls[1].path.points.pop();
    let wasm = LayerStageCommit::Perimeters(shrunk);
    let err = assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect_err("differing point count must be rejected");
    assert!(
        err.contains("point count"),
        "error must name the point count invariant: {err}"
    );
}

// ── Prepass family fixtures ─────────────────────────────────────────────────

fn support_segment(point_count: usize, role: ExtrusionRole, jitter: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: (0..point_count)
            .map(|i| pt(i as f32 + jitter, jitter, 1.0 + jitter, 0.4))
            .collect(),
        role,
        speed_factor: 1.0,
    }
}

fn support_entry(layer: i32, segments: Vec<ExtrusionPath3D>) -> SupportPlanEntry {
    SupportPlanEntry {
        global_layer_index: layer,
        object_id: "parity-object".to_string(),
        region_id: 0,
        branch_segments: segments,
    }
}

fn support_plan(entries: Vec<SupportPlanEntry>) -> PrepassStageOutput {
    PrepassStageOutput::SupportPlan(Arc::new(SupportPlanIR {
        entries,
        ..Default::default()
    }))
}

fn base_plan(jitter: f32) -> PrepassStageOutput {
    support_plan(vec![
        support_entry(
            0,
            vec![support_segment(3, ExtrusionRole::SupportMaterial, jitter)],
        ),
        support_entry(
            1,
            vec![
                support_segment(3, ExtrusionRole::SupportMaterial, jitter),
                support_segment(2, ExtrusionRole::SupportInterface, jitter),
            ],
        ),
    ])
}

#[test]
fn parity_comparator_rejects_dropped_support_entry_whole_entry() {
    // Non-vacuousness in the accept direction for the prepass family: an
    // identical plan and a ULP-perturbed plan both pass.
    let native = base_plan(0.0);
    assert_prepass_parity_structural(&native, &base_plan(0.0), ParityTolerance::default())
        .expect("identical plans must be accepted");
    assert_prepass_parity_structural(&native, &base_plan(1e-6), ParityTolerance::default())
        .expect("ULP-scale perturbation must be accepted");

    // AC-N6(a): second plan missing one whole SupportPlanEntry.
    let wasm = support_plan(vec![support_entry(
        0,
        vec![support_segment(3, ExtrusionRole::SupportMaterial, 0.0)],
    )]);
    let err = assert_prepass_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped entry must be rejected");
    assert!(
        err.contains("entries count"),
        "error must name the entries count invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_dropped_support_entry_shifted_layer_index() {
    // AC-N6(b): same (object_id, region_id) but a shifted global_layer_index
    // must fail — the entry key is the FULL triple, not the id pair.
    let native = base_plan(0.0);
    let wasm = support_plan(vec![
        support_entry(
            7,
            vec![support_segment(3, ExtrusionRole::SupportMaterial, 0.0)],
        ),
        support_entry(
            1,
            vec![
                support_segment(3, ExtrusionRole::SupportMaterial, 0.0),
                support_segment(2, ExtrusionRole::SupportInterface, 0.0),
            ],
        ),
    ]);
    let err = assert_prepass_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("shifted global_layer_index must be rejected");
    assert!(
        err.contains("entry key set"),
        "error must name the entry key set invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_dropped_support_entry_dropped_segment() {
    // AC-N6(c): one ExtrusionPath3D missing from an entry's branch_segments.
    let native = base_plan(0.0);
    let wasm = support_plan(vec![
        support_entry(
            0,
            vec![support_segment(3, ExtrusionRole::SupportMaterial, 0.0)],
        ),
        support_entry(
            1,
            vec![support_segment(3, ExtrusionRole::SupportMaterial, 0.0)],
        ),
    ]);
    let err = assert_prepass_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped branch segment must be rejected");
    assert!(
        err.contains("branch_segments count"),
        "error must name the branch_segments count invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_dropped_support_entry_dropped_point() {
    // AC-N6(d): one Point3WithWidth missing from a segment.
    let native = base_plan(0.0);
    let wasm = support_plan(vec![
        support_entry(
            0,
            vec![support_segment(2, ExtrusionRole::SupportMaterial, 0.0)],
        ),
        support_entry(
            1,
            vec![
                support_segment(3, ExtrusionRole::SupportMaterial, 0.0),
                support_segment(2, ExtrusionRole::SupportInterface, 0.0),
            ],
        ),
    ]);
    let err = assert_prepass_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped point must be rejected");
    assert!(
        err.contains("points count"),
        "error must name the points count invariant: {err}"
    );
}
