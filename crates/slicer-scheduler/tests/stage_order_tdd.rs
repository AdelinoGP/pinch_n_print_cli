//! Scheduler stage ordering: the `PrePass::` → `Layer::` → `PostPass::` phase
//! blocks are exhaustive and never interleave.
//!
//! Retargeted from a single-stage placement pin when that stage was retired
//! (packet layer-module-preparation_08, Step 12b). The invariant it protected —
//! no prepass stage runs after a layer stage, and no layer stage runs after a
//! postpass stage — survives on the surviving stage population.

#![allow(missing_docs)]

use slicer_scheduler::execution_plan::STAGE_ORDER;

/// Phase rank of `stage`: `0` for `PrePass::`, `1` for `Layer::`, `2` for
/// `PostPass::`, `None` for a stage that belongs to no phase.
fn phase_rank(stage: &str) -> Option<u8> {
    if stage.starts_with("PrePass::") {
        Some(0)
    } else if stage.starts_with("Layer::") {
        Some(1)
    } else if stage.starts_with("PostPass::") {
        Some(2)
    } else {
        None
    }
}

#[test]
fn every_stage_belongs_to_a_known_phase() {
    assert!(!STAGE_ORDER.is_empty(), "STAGE_ORDER must not be empty");
    for stage in STAGE_ORDER {
        assert!(
            phase_rank(stage).is_some(),
            "STAGE_ORDER entry `{stage}` belongs to no known phase \
             (PrePass:: / Layer:: / PostPass::)"
        );
    }
}

#[test]
fn prepass_stages_precede_layer_stages_precede_postpass_stages() {
    let last_prepass_idx = STAGE_ORDER
        .iter()
        .rposition(|s| phase_rank(s) == Some(0))
        .expect("at least one PrePass:: stage must exist");
    let first_layer_idx = STAGE_ORDER
        .iter()
        .position(|s| phase_rank(s) == Some(1))
        .expect("at least one Layer:: stage must exist");
    let last_layer_idx = STAGE_ORDER
        .iter()
        .rposition(|s| phase_rank(s) == Some(1))
        .expect("at least one Layer:: stage must exist");
    let first_postpass_idx = STAGE_ORDER
        .iter()
        .position(|s| phase_rank(s) == Some(2))
        .expect("at least one PostPass:: stage must exist");
    assert!(
        last_prepass_idx < first_layer_idx,
        "the last PrePass:: stage (idx {last_prepass_idx}) must come before the \
         first Layer:: stage (idx {first_layer_idx})"
    );
    assert!(
        last_layer_idx < first_postpass_idx,
        "the last Layer:: stage (idx {last_layer_idx}) must come before the \
         first PostPass:: stage (idx {first_postpass_idx})"
    );
}
