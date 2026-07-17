//! ADR-0020 meta-test: the `LayerStageCommit` enum and the canonical
//! `slicer-schema::STAGES` table cannot drift.
//!
//! `LayerStageCommit`'s production variants must mirror exactly the eight
//! `world-layer` stages in `STAGES`. Adding a ninth per-layer stage, renaming
//! one, or dropping a variant breaks this test — closing the one drift gap the
//! per-stage enum could otherwise hide.

use std::collections::BTreeSet;

use slicer_ir::{
    InfillIR, LayerCollectionIR, LayerStageCommit, PathOptimizationCommit, PerimeterIR, SupportIR,
};
use slicer_schema::STAGES;

const LAYER_WORLD: &str = slicer_schema::WORLD_LAYER;

#[test]
fn production_variants_match_world_layer_stages_exactly() {
    // The canonical truth: every `world-layer` stage id in STAGES.
    let expected: BTreeSet<&'static str> = STAGES
        .iter()
        .filter(|s| s.world_id == LAYER_WORLD)
        .map(|s| s.stage_id)
        .collect();

    // One commit of every production variant. Payloads are throwaway —
    // `stage_id()` keys off the discriminant only.
    let production = [
        LayerStageCommit::Perimeters(PerimeterIR::default()),
        LayerStageCommit::PerimetersPostProcess(None),
        LayerStageCommit::Infill(InfillIR::default()),
        LayerStageCommit::InfillPostProcess(InfillIR::default()),
        LayerStageCommit::Support(SupportIR::default()),
        LayerStageCommit::SupportPostProcess(SupportIR::default()),
        LayerStageCommit::SlicePostProcess {
            polygon_updates: Vec::new(),
            path_z_updates: Vec::new(),
        },
        LayerStageCommit::PathOptimization(PathOptimizationCommit::default()),
    ];

    let actual: BTreeSet<&'static str> = production
        .iter()
        .map(|c| {
            c.stage_id()
                .expect("every production variant maps to a stage")
        })
        .collect();

    assert_eq!(
        actual, expected,
        "LayerStageCommit production variants must mirror the world-layer STAGES \
         rows exactly — the enum and the canonical stage table have drifted"
    );

    // Sanity: there are exactly eight, and the count matches the variant list.
    assert_eq!(expected.len(), 8, "expected 8 world-layer stages");
    assert_eq!(
        production.len(),
        expected.len(),
        "one production variant per world-layer stage, no duplicates"
    );
}

#[test]
fn seed_layer_collection_is_test_only_with_no_stage() {
    // The escape hatch is not a production stage and must not collide with STAGES.
    assert_eq!(
        LayerStageCommit::SeedLayerCollection(LayerCollectionIR::default()).stage_id(),
        None,
        "SeedLayerCollection is test-only and has no canonical stage"
    );
}
