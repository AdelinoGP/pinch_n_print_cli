#![allow(missing_docs)]

//! Self-tests for the structural parity comparator (packet 204, Step 3).
//! These prove the comparator is neither vacuous (it rejects dropped
//! geometry) nor byte-exact (it accepts ULP-scale drift) BEFORE any pilot
//! module is compared — AC-N2, AC-N3 (layer family), AC-N6 (prepass family).

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, ExtrusionPath3D, ExtrusionRole, GCodeCommand, GlobalLayer, InfillIR,
    InfillRegion, LayerCollectionIR, LayerPlanIR, LayerStageCommit, LoopType, ObjectLayerRef,
    PathOptimizationCommit, PerimeterIR, PerimeterRegion, Point3WithWidth, PrintEntity, RegionKey,
    ScoredSeamCandidate, SeamPlanEntry, SeamPlanIR, SeamPosition, SeamReason, SupportEntry,
    SupportIR, SupportPlanEntry, SupportPlanIR, SupportRole, TravelMoveDest, WallBoundaryType,
    WallFeatureFlags, WallLoop, WidthProfile,
};
use slicer_runtime::PrepassStageOutput;

use crate::common::parity_invariants::{
    assert_finalization_parity_structural, assert_gcode_sequence_parity_structural,
    assert_layer_plan_parity_structural, assert_parity_structural,
    assert_prepass_parity_structural, assert_seam_parity_structural, ParityTolerance,
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
    // exhaustive: invariant selftest pins every field explicitly
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

fn pathopt_commit() -> LayerStageCommit {
    LayerStageCommit::PathOptimization(PathOptimizationCommit {
        travel_moves: vec![TravelMoveDest {
            x: Some(10.0),
            y: Some(20.0),
            z: None,
            f: Some(60.0),
        }],
        ..Default::default()
    })
}

#[test]
fn parity_comparator_rejects_dropped_path() {
    let native = pathopt_commit();
    let wasm = LayerStageCommit::PathOptimization(PathOptimizationCommit::default());
    let err = assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect_err("dropped travel path must be rejected");
    assert!(
        err.contains("travel_moves count"),
        "error must name travel_moves: {err}"
    );
}

#[test]
fn parity_comparator_rejects_dropped_gcode_command() {
    let native = vec![GCodeCommand::Comment {
        text: "path".into(),
    }];
    let err = assert_gcode_sequence_parity_structural(&native, &[], ParityTolerance::default())
        .expect_err("dropped gcode command must be rejected");
    assert!(
        err.contains("command count"),
        "error must name command count: {err}"
    );
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
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index: layer,
        object_id: "parity-object".to_string(),
        region_id: 0,
        family_id: "tree".into(),
        demand_ids: vec!["demand".into()],
        body_ids: vec!["body".into()],
        anchor_layer_index: layer.max(0) as u32,
        anchor_z: 100,
        roles: segments
            .into_iter()
            .map(|segment| slicer_ir::SupportPlanRoleRegion {
                role: match segment.role {
                    ExtrusionRole::SupportInterface => slicer_ir::SupportPlanRole::TopInterface,
                    _ => slicer_ir::SupportPlanRole::SupportBody,
                },
                regions: vec![],
            })
            .collect(),
        skeleton: None,
        capabilities: vec![],
        provenance: vec![],
        decline_reason: None,
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

// ── Finalization family fixtures ────────────────────────────────────────────

fn finalization_entity(
    entity_id: u64,
    role: ExtrusionRole,
    point_count: usize,
    jitter: f32,
) -> PrintEntity {
    // exhaustive: invariant selftest pins every field explicitly
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points: (0..point_count)
                .map(|i| pt(i as f32 + jitter, jitter, 1.0 + jitter, 0.4))
                .collect(),
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: "parity-object".to_string(),
            region_id: 0,
            ..Default::default()
        },
        topo_order: (entity_id - 1) as u32,
        tool_index: 0,
    }
}

fn finalization_layer(global_layer_index: u32, jitter: f32) -> LayerCollectionIR {
    LayerCollectionIR {
        global_layer_index,
        z: 0.2 * (global_layer_index + 1) as f32,
        ordered_entities: vec![
            finalization_entity(1, ExtrusionRole::OuterWall, 4, jitter),
            finalization_entity(2, ExtrusionRole::InnerWall, 3, jitter),
        ],
        ..Default::default()
    }
}

fn finalization_layers(jitter: f32) -> Vec<LayerCollectionIR> {
    vec![finalization_layer(0, jitter), finalization_layer(1, jitter)]
}

#[test]
fn parity_comparator_accepts_finalization_ulp_perturbation() {
    // Non-vacuousness in the accept direction for the finalization family: an
    // identical merged collection and a ULP-perturbed one both pass.
    let native = finalization_layers(0.0);
    assert_finalization_parity_structural(
        &native,
        &finalization_layers(0.0),
        ParityTolerance::default(),
    )
    .expect("identical layer collections must be accepted");
    assert_finalization_parity_structural(
        &native,
        &finalization_layers(1e-6),
        ParityTolerance::default(),
    )
    .expect("ULP-scale perturbation must be accepted");
}

#[test]
fn parity_comparator_rejects_dropped_finalization_layer() {
    // The wasm path missing one merged layer must fail on layer count.
    let native = finalization_layers(0.0);
    let wasm = finalization_layers(0.0)[..1].to_vec();
    let err = assert_finalization_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped layer must be rejected");
    assert!(
        err.contains("layer count"),
        "error must name the layer count invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_dropped_finalization_entity() {
    // One PrintEntity missing from a merged layer must fail on entity count.
    let native = finalization_layers(0.0);
    let mut wasm = finalization_layers(0.0);
    wasm[1].ordered_entities.pop();
    let err = assert_finalization_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped entity must be rejected");
    assert!(
        err.contains("entity count"),
        "error must name the entity count invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_dropped_finalization_point() {
    // One point missing from an entity's path must fail on points count.
    let native = finalization_layers(0.0);
    let mut wasm = finalization_layers(0.0);
    wasm[0].ordered_entities[0].path.points.pop();
    let err = assert_finalization_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped point must be rejected");
    assert!(
        err.contains("points count"),
        "error must name the points count invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_moved_finalization_point() {
    // A coordinate moved well outside coord_mm must fail on coordinates.
    let native = finalization_layers(0.0);
    let mut wasm = finalization_layers(0.0);
    wasm[0].ordered_entities[1].path.points[1].x += 0.5;
    let err = assert_finalization_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("moved point must be rejected");
    assert!(
        err.contains("point (x, y, z, width)"),
        "error must name the point coordinates invariant: {err}"
    );
}

// ── Seam-plan prepass family fixtures ───────────────────────────────────────

fn seam_candidate(x: f32, score: f32, reason: SeamReason, jitter: f32) -> ScoredSeamCandidate {
    ScoredSeamCandidate {
        position: pt(x + jitter, jitter, 1.0 + jitter, 0.4),
        score,
        reason,
    }
}

fn seam_entry(layer: u32, jitter: f32) -> SeamPlanEntry {
    // exhaustive: invariant selftest pins every field explicitly
    SeamPlanEntry {
        region_key: RegionKey {
            global_layer_index: layer,
            object_id: "parity-object".to_string(),
            region_id: 0,
            ..Default::default()
        },
        chosen_candidate: SeamPosition {
            point: pt(2.0 + jitter, jitter, 1.0 + jitter, 0.4),
            wall_index: 0,
        },
        scored_candidates: vec![
            seam_candidate(2.0, 0.1, SeamReason::Concave, jitter),
            seam_candidate(4.0, 0.9, SeamReason::Sharp, jitter),
        ],
    }
}

fn seam_plan_output(entries: Vec<SeamPlanEntry>) -> PrepassStageOutput {
    PrepassStageOutput::SeamPlan(Arc::new(SeamPlanIR {
        entries,
        ..Default::default()
    }))
}

fn base_seam_plan(jitter: f32) -> PrepassStageOutput {
    seam_plan_output(vec![seam_entry(0, jitter), seam_entry(1, jitter)])
}

fn seam_plan_ir_of(output: &PrepassStageOutput) -> SeamPlanIR {
    match output {
        PrepassStageOutput::SeamPlan(ir) => (**ir).clone(),
        _ => panic!("expected SeamPlan"),
    }
}

#[test]
fn parity_comparator_accepts_seam_plan_ulp_perturbation() {
    // Non-vacuousness in the accept direction for the seam-plan family.
    let native = base_seam_plan(0.0);
    assert_seam_parity_structural(&native, &base_seam_plan(0.0), ParityTolerance::default())
        .expect("identical seam plans must be accepted");
    assert_seam_parity_structural(&native, &base_seam_plan(1e-6), ParityTolerance::default())
        .expect("ULP-scale perturbation must be accepted");
}

#[test]
fn parity_comparator_rejects_dropped_seam_entry() {
    // Second plan missing one whole SeamPlanEntry must fail on entries count.
    let native = base_seam_plan(0.0);
    let wasm = seam_plan_output(vec![seam_entry(0, 0.0)]);
    let err = assert_seam_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped seam entry must be rejected");
    assert!(
        err.contains("entries count"),
        "error must name the entries count invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_dropped_seam_candidate() {
    // One ScoredSeamCandidate missing from an entry's evidence list.
    let native = base_seam_plan(0.0);
    let mut dropped = seam_plan_ir_of(&base_seam_plan(0.0));
    dropped.entries[1].scored_candidates.pop();
    let wasm = seam_plan_output(dropped.entries);
    let err = assert_seam_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped seam candidate must be rejected");
    assert!(
        err.contains("scored_candidates count"),
        "error must name the scored_candidates count invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_moved_seam_position() {
    // Chosen seam position moved well outside coord_mm must fail.
    let native = base_seam_plan(0.0);
    let mut moved = seam_plan_ir_of(&base_seam_plan(0.0));
    moved.entries[0].chosen_candidate.point.x += 0.5;
    let wasm = seam_plan_output(moved.entries);
    let err = assert_seam_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("moved seam position must be rejected");
    assert!(
        err.contains("chosen seam position"),
        "error must name the chosen seam position invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_shifted_seam_score() {
    // A candidate score shifted well outside the float tolerance must fail.
    let native = base_seam_plan(0.0);
    let mut shifted = seam_plan_ir_of(&base_seam_plan(0.0));
    shifted.entries[0].scored_candidates[1].score += 0.5;
    let wasm = seam_plan_output(shifted.entries);
    let err = assert_seam_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("shifted seam score must be rejected");
    assert!(
        err.contains("candidate score"),
        "error must name the candidate score invariant: {err}"
    );
}

// ── Layer-plan prepass family fixtures ──────────────────────────────────────

fn layer_plan_output(jitter: f32) -> PrepassStageOutput {
    let active_region = || ActiveRegion {
        object_id: "parity-object".to_string(),
        region_id: 0,
        effective_layer_height: 0.2 + jitter,
        tool_index: 0,
        ..Default::default()
    };
    let global_layers = vec![
        // exhaustive: invariant selftest pins every field explicitly
        GlobalLayer {
            index: 0,
            z: 0.2 + jitter,
            active_regions: vec![active_region()],
            has_nonplanar: false,
            is_sync_layer: false,
        },
        // exhaustive: invariant selftest pins every field explicitly
        GlobalLayer {
            index: 1,
            z: 0.4 + jitter,
            active_regions: vec![active_region()],
            has_nonplanar: false,
            is_sync_layer: false,
        },
    ];
    let mut object_participation = HashMap::new();
    object_participation.insert(
        "parity-object".to_string(),
        vec![
            ObjectLayerRef {
                local_layer_index: 0,
                global_layer_index: 0,
                effective_layer_height: 0.2 + jitter,
            },
            ObjectLayerRef {
                local_layer_index: 1,
                global_layer_index: 1,
                effective_layer_height: 0.2 + jitter,
            },
        ],
    );
    PrepassStageOutput::LayerPlan(Arc::new(LayerPlanIR {
        global_layers,
        object_participation,
        ..Default::default()
    }))
}

fn layer_plan_ir_of(output: &PrepassStageOutput) -> LayerPlanIR {
    match output {
        PrepassStageOutput::LayerPlan(ir) => (**ir).clone(),
        _ => panic!("expected LayerPlan"),
    }
}

#[test]
fn parity_comparator_accepts_layer_plan_ulp_perturbation() {
    // Non-vacuousness in the accept direction for the layer-plan family.
    let native = layer_plan_output(0.0);
    assert_layer_plan_parity_structural(
        &native,
        &layer_plan_output(0.0),
        ParityTolerance::default(),
    )
    .expect("identical layer plans must be accepted");
    assert_layer_plan_parity_structural(
        &native,
        &layer_plan_output(1e-6),
        ParityTolerance::default(),
    )
    .expect("ULP-scale perturbation must be accepted");
}

#[test]
fn parity_comparator_rejects_dropped_layer_plan_layer() {
    // Second plan missing one whole GlobalLayer must fail on layer count.
    let native = layer_plan_output(0.0);
    let mut dropped = layer_plan_ir_of(&layer_plan_output(0.0));
    dropped.global_layers.pop();
    let wasm = PrepassStageOutput::LayerPlan(Arc::new(dropped));
    let err = assert_layer_plan_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped global layer must be rejected");
    assert!(
        err.contains("global_layers count"),
        "error must name the global_layers count invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_dropped_layer_plan_participation_entry() {
    // Participation map missing one object key must fail on the key set.
    let native = layer_plan_output(0.0);
    let mut dropped = layer_plan_ir_of(&layer_plan_output(0.0));
    dropped.object_participation.remove("parity-object");
    let wasm = PrepassStageOutput::LayerPlan(Arc::new(dropped));
    let err = assert_layer_plan_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("dropped participation entry must be rejected");
    assert!(
        err.contains("participation"),
        "error must name the participation invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_moved_layer_plan_z() {
    // A global layer Z moved well outside coord_mm must fail.
    let native = layer_plan_output(0.0);
    let mut moved = layer_plan_ir_of(&layer_plan_output(0.0));
    moved.global_layers[1].z += 0.5;
    let wasm = PrepassStageOutput::LayerPlan(Arc::new(moved));
    let err = assert_layer_plan_parity_structural(&native, &wasm, ParityTolerance::default())
        .expect_err("moved layer z must be rejected");
    assert!(
        err.contains("layer z"),
        "error must name the layer z invariant: {err}"
    );
}

// ── Infill layer family fixtures ────────────────────────────────────────────

fn infill_region(jitter: f32) -> InfillRegion {
    // exhaustive: invariant selftest pins every field explicitly
    InfillRegion {
        object_id: "parity-object".to_string(),
        region_id: 0,
        sparse_infill: vec![support_segment(3, ExtrusionRole::SparseInfill, jitter)],
        solid_infill: vec![support_segment(
            2,
            ExtrusionRole::InternalSolidInfill,
            jitter,
        )],
        ironing: vec![support_segment(2, ExtrusionRole::Ironing, jitter)],
    }
}

fn infill_ir(jitter: f32) -> InfillIR {
    InfillIR {
        schema_version: semver(),
        global_layer_index: 0,
        regions: vec![infill_region(jitter)],
    }
}

#[test]
fn parity_comparator_accepts_infill_ulp_perturbation() {
    // Non-vacuousness in the accept direction for the infill family: identical
    // and ULP-perturbed commits pass for both the base and post-process arms.
    let native = LayerStageCommit::Infill(infill_ir(0.0));
    let wasm = LayerStageCommit::Infill(infill_ir(1e-6));
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("ULP-scale infill perturbation must be accepted");

    let native = LayerStageCommit::InfillPostProcess(infill_ir(0.0));
    let wasm = LayerStageCommit::InfillPostProcess(infill_ir(1e-6));
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("ULP-scale infill-post-process perturbation must be accepted");
}

#[test]
fn parity_comparator_rejects_dropped_infill_region() {
    // The wasm path missing one whole InfillRegion must fail on region count.
    let native = LayerStageCommit::Infill(infill_ir(0.0));
    let mut dropped = infill_ir(0.0);
    dropped.regions.pop();
    let wasm = LayerStageCommit::Infill(dropped);
    let err = assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect_err("dropped infill region must be rejected");
    assert!(
        err.contains("region count"),
        "error must name the region count invariant: {err}"
    );
}

#[test]
fn parity_comparator_rejects_dropped_infill_path() {
    // One ExtrusionPath3D missing from a region's sparse_infill must fail on
    // the per-field path count, naming the region key and field.
    let native = LayerStageCommit::Infill(infill_ir(0.0));
    let mut dropped = infill_ir(0.0);
    dropped.regions[0].sparse_infill.pop();
    let wasm = LayerStageCommit::Infill(dropped);
    let err = assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect_err("dropped infill path must be rejected");
    assert!(
        err.contains("sparse_infill"),
        "error must name the sparse_infill field: {err}"
    );
}

#[test]
fn parity_comparator_rejects_moved_infill_point() {
    // A coordinate moved well outside coord_mm in a solid_infill path must
    // fail on the point coordinates invariant.
    let native = LayerStageCommit::Infill(infill_ir(0.0));
    let mut moved = infill_ir(0.0);
    moved.regions[0].solid_infill[0].points[1].x += 0.5;
    let wasm = LayerStageCommit::Infill(moved);
    let err = assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect_err("moved infill point must be rejected");
    assert!(
        err.contains("point (x, y, z, width)"),
        "error must name the point coordinates invariant: {err}"
    );
}

// ── Support layer family fixtures ───────────────────────────────────────────

fn support_ir(jitter: f32) -> SupportIR {
    // exhaustive: invariant selftest pins every field explicitly
    SupportIR {
        schema_version: semver(),
        global_layer_index: 0,
        entries: vec![
            // exhaustive: support identity contract fixture pins the full family/body/demand/object/region/role tuple
            SupportEntry {
                family_id: "fixture-family".into(),
                body_id: "fixture-body".into(),
                demand_ids: vec!["fixture-demand".into()],
                object_id: "obj".into(),
                region_id: 0,
                role: SupportRole::SupportBody,
                paths: vec![support_segment(3, ExtrusionRole::SupportMaterial, jitter)],
            },
            // exhaustive: support identity contract fixture pins the full family/body/demand/object/region/role tuple
            SupportEntry {
                family_id: "fixture-family".into(),
                body_id: "fixture-body".into(),
                demand_ids: vec!["fixture-demand".into()],
                object_id: "obj".into(),
                region_id: 0,
                role: SupportRole::TopInterface,
                paths: vec![
                    support_segment(2, ExtrusionRole::SupportInterface, jitter),
                    support_segment(2, ExtrusionRole::RaftInfill, jitter),
                ],
            },
        ],
    }
}

#[test]
fn parity_comparator_accepts_support_ulp_perturbation() {
    // Non-vacuousness in the accept direction for the support family.
    let native = LayerStageCommit::Support(support_ir(0.0));
    let wasm = LayerStageCommit::Support(support_ir(1e-6));
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("ULP-scale support perturbation must be accepted");

    let native = LayerStageCommit::SupportPostProcess(support_ir(0.0));
    let wasm = LayerStageCommit::SupportPostProcess(support_ir(1e-6));
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("ULP-scale support-post-process perturbation must be accepted");
}

#[test]
fn parity_comparator_rejects_dropped_support_path() {
    // One ExtrusionPath3D missing from the interface entry must fail on the
    // per-field path count, naming the field.
    let native = LayerStageCommit::Support(support_ir(0.0));
    let mut dropped = support_ir(0.0);
    dropped.entries[1].paths.pop();
    let wasm = LayerStageCommit::Support(dropped);
    let err = assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect_err("dropped support path must be rejected");
    assert!(
        err.contains("entry[1] paths"),
        "error must name the interface entry paths: {err}"
    );
}

#[test]
fn parity_comparator_rejects_moved_support_point() {
    // A coordinate moved well outside coord_mm in a support path must fail on
    // the point coordinates invariant.
    let native = LayerStageCommit::Support(support_ir(0.0));
    let mut moved = support_ir(0.0);
    moved.entries[0].paths[0].points[1].x += 0.5;
    let wasm = LayerStageCommit::Support(moved);
    let err = assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect_err("moved support point must be rejected");
    assert!(
        err.contains("point (x, y, z, width)"),
        "error must name the point coordinates invariant: {err}"
    );
}

// ── PerimetersPostProcess layer family fixtures ─────────────────────────────

#[test]
fn parity_comparator_accepts_perimeters_postprocess_some_some() {
    // Some-vs-Some with identical perimeters passes; None-vs-None passes.
    let native =
        LayerStageCommit::PerimetersPostProcess(Some(perimeters_of(perimeter_commit(0.0))));
    let wasm = LayerStageCommit::PerimetersPostProcess(Some(perimeters_of(perimeter_commit(1e-6))));
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("Some-vs-Some perimeters-postprocess must be accepted");

    let native = LayerStageCommit::PerimetersPostProcess(None);
    let wasm = LayerStageCommit::PerimetersPostProcess(None);
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("None-vs-None perimeters-postprocess must be accepted");
}

#[test]
fn parity_comparator_rejects_perimeters_postprocess_some_none_mismatch() {
    // Some-vs-None (and None-vs-Some) must fail on the Option state.
    let some = LayerStageCommit::PerimetersPostProcess(Some(perimeters_of(perimeter_commit(0.0))));
    let none = LayerStageCommit::PerimetersPostProcess(None);
    let err = assert_parity_structural(&some, &none, ParityTolerance::default(), 0.4)
        .expect_err("Some-vs-None must be rejected");
    assert!(
        err.contains("Some/None mismatch"),
        "error must name the Some/None mismatch invariant: {err}"
    );
    let err = assert_parity_structural(&none, &some, ParityTolerance::default(), 0.4)
        .expect_err("None-vs-Some must be rejected");
    assert!(
        err.contains("Some/None mismatch"),
        "error must name the Some/None mismatch invariant: {err}"
    );
}
