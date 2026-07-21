//! Packet 137: `PrePass::LightningTreeGen` stage ordering (AC-1).
//!
//! Asserts the new lightning prepass stage is present in `STAGE_ORDER` and
//! positioned after the last existing `PrePass::*` stage and before the
//! first `Layer::*` stage (per ADR-0029).

#![allow(missing_docs)]

use slicer_scheduler::execution_plan::STAGE_ORDER;

#[test]
fn lightning_tree_gen_stage_is_present() {
    assert!(
        STAGE_ORDER.contains(&"PrePass::LightningTreeGen"),
        "STAGE_ORDER must include the new lightning prepass stage"
    );
}

#[test]
fn lightning_tree_gen_stage_is_after_last_prepass_and_before_first_layer() {
    let lightning_idx = STAGE_ORDER
        .iter()
        .position(|s| *s == "PrePass::LightningTreeGen")
        .expect("PrePass::LightningTreeGen must be present in STAGE_ORDER");
    let second_to_last_prepass_idx = STAGE_ORDER
        .iter()
        .rposition(|s| s.starts_with("PrePass::") && *s != "PrePass::LightningTreeGen")
        .expect("at least one prior PrePass:: stage must exist");
    let first_layer_idx = STAGE_ORDER
        .iter()
        .position(|s| s.starts_with("Layer::"))
        .expect("at least one Layer:: stage must exist");
    assert!(
        lightning_idx > second_to_last_prepass_idx,
        "PrePass::LightningTreeGen (idx {lightning_idx}) must come after the previous last prepass stage (idx {second_to_last_prepass_idx})"
    );
    assert!(
        lightning_idx < first_layer_idx,
        "PrePass::LightningTreeGen (idx {lightning_idx}) must come before the first Layer:: stage (idx {first_layer_idx})"
    );
}
