//! Regression pin for the Step 5 bug where `cmd_validate`'s validator stage
//! list (15 entries) had silently drifted short of the scheduler's canonical
//! `STAGE_ORDER` (22 entries). Step 5 consolidated the validator list into
//! `slicer_schema::VALID_STAGES` and reconciled it to the user-targetable
//! subset of `STAGE_ORDER`. This test pins that reconciliation so future
//! additions to `STAGE_ORDER` must be classified explicitly as either
//! user-targetable (added to `VALID_STAGES`) or host-only (added to
//! `HOST_ONLY_STAGES` below).
//!
//! The test lives in `slicer-runtime/tests/` rather than `slicer-schema/tests/`
//! because `slicer-schema` cannot import `slicer-runtime` (would invert the
//! workspace dep graph). `slicer-runtime` already depends on `slicer-schema`
//! and owns `STAGE_ORDER`, so this is the lowest place both lists are
//! visible.

use std::collections::HashSet;

use slicer_scheduler::execution_plan::STAGE_ORDER;
use slicer_schema::VALID_STAGES;

/// Stages produced exclusively by host built-in producers (or host-internal
/// synthetic stages). Modules cannot target these via their manifest's
/// `stage` field, so they intentionally do NOT appear in `VALID_STAGES`.
///
/// If you add a stage to `STAGE_ORDER`, decide whether it is module-targetable.
/// If yes, add it to `VALID_STAGES` in `slicer-schema/src/lib.rs`. If no, add
/// it here.
const HOST_ONLY_STAGES: &[&str] = &[
    "PrePass::RegionMapping",
    "PrePass::Slice",
    "PrePass::ShellClassification",
    "PrePass::OverhangAnnotation",
    "Layer::PaintRegionAnnotation",
    "PostPass::GCodeEmit",
];

#[test]
fn valid_stages_is_subset_of_stage_order() {
    let stage_order: HashSet<&str> = STAGE_ORDER.iter().copied().collect();
    for stage in VALID_STAGES {
        assert!(
            stage_order.contains(stage),
            "VALID_STAGES entry `{stage}` is not in STAGE_ORDER. Either rename in slicer-schema or add to STAGE_ORDER.",
        );
    }
}

#[test]
fn host_only_stages_partition_stage_order_into_valid_stages() {
    let stage_order: HashSet<&str> = STAGE_ORDER.iter().copied().collect();
    let host_only: HashSet<&str> = HOST_ONLY_STAGES.iter().copied().collect();
    let valid_stages: HashSet<&str> = VALID_STAGES.iter().copied().collect();

    for stage in &host_only {
        assert!(
            stage_order.contains(stage),
            "HOST_ONLY_STAGES entry `{stage}` is not in STAGE_ORDER (stale exclusion?)",
        );
        assert!(
            !valid_stages.contains(stage),
            "stage `{stage}` is in both HOST_ONLY_STAGES and VALID_STAGES; a stage cannot be both module-targetable and host-only",
        );
    }

    let expected_user_facing: HashSet<&str> = stage_order
        .iter()
        .copied()
        .filter(|s| !host_only.contains(s))
        .collect();

    let missing_from_valid: Vec<&&str> = expected_user_facing.difference(&valid_stages).collect();
    let extra_in_valid: Vec<&&str> = valid_stages.difference(&expected_user_facing).collect();

    assert!(
        missing_from_valid.is_empty() && extra_in_valid.is_empty(),
        "VALID_STAGES drifted from canonical user-facing partition of STAGE_ORDER.\n  \
         missing from VALID_STAGES (in STAGE_ORDER but not host-only): {missing_from_valid:?}\n  \
         extra in VALID_STAGES (not present as a user-facing stage in STAGE_ORDER): {extra_in_valid:?}\n  \
         Fix by either updating VALID_STAGES in slicer-schema/src/lib.rs OR updating HOST_ONLY_STAGES in this test.",
    );
}
