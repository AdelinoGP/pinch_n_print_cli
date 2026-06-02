/// TDD test for AC-6 / AC-N2 (Packet 83, Step 2).
///
/// Asserts that `export_for_stage_id` is total over `STAGES` (every known
/// stage id maps to its `wit_export`) and correctly rejects unknown ids.

#[test]
fn export_for_stage_id_is_total_over_stages_and_rejects_unknown() {
    // AC-N2 part 1: must return Some(wit_export) for every entry in STAGES.
    for stage in slicer_schema::STAGES {
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
