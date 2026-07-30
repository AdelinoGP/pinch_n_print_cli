//! TDD test for AC-6 / AC-N2 (Packet 83, Step 2).
//!
//! Asserts that `export_for_stage_id` is total over `STAGES` (every known
//! stage id maps to its `wit_export`) and correctly rejects unknown ids.

#![allow(missing_docs)]

#[test]
fn export_for_stage_id_is_total_over_stages_and_rejects_unknown() {
    // AC-N2 part 1: must return Some(wit_export) for every WIT-backed entry in STAGES.
    for stage in slicer_schema::STAGES {
        // The sole host-built-in stage (packet 97) has no WIT export;
        // every lookup returns None for it. Skip it.
        if stage.stage_id == "PrePass::PaintSegmentation" {
            assert_eq!(
                slicer_schema::export_for_stage_id(stage.stage_id),
                None,
                "host-built-in stage {} should map to None",
                stage.stage_id,
            );
            continue;
        }
        assert_eq!(
            slicer_schema::export_for_stage_id(stage.stage_id),
            Some(stage.wit_export),
            "export_for_stage_id({:?}) returned wrong value",
            stage.stage_id,
        );
    }

    // AC-N2 part 2: unknown ids must return None.
    assert_eq!(
        slicer_schema::export_for_stage_id("NotAStage"),
        None,
        "expected None for unknown stage id \"NotAStage\""
    );
    assert_eq!(
        slicer_schema::export_for_stage_id(""),
        None,
        "expected None for empty stage id"
    );
}
