//! TDD tests for `SliceRegionView::should_emit` covering the new
//! `ExtrusionRole::RaftInfill` -> `claim:raft-fill` arm in `views.rs`.
//!
//! AC-4: held_claims = ["claim:raft-fill"] should_emit(RaftInfill) -> true
//! AC-N1: held_claims = ["claim:sparse-fill"] should_emit(RaftInfill) -> false
//! AC-N3: held_claims = [] should_emit(RaftInfill) -> false (empty-claims suppression)

use slicer_ir::ExtrusionRole;
use slicer_sdk::views::SliceRegionView;

#[test]
fn ac4_raft_fill_claim_emits_raft_infill() {
    let mut view = SliceRegionView::default();
    view.set_held_claims(vec!["claim:raft-fill".to_string()]);
    assert!(
        view.should_emit(ExtrusionRole::RaftInfill),
        "module holding claim:raft-fill must emit RaftInfill"
    );
}

#[test]
fn ac_n1_sparse_fill_claim_does_not_emit_raft_infill() {
    let mut view = SliceRegionView::default();
    view.set_held_claims(vec!["claim:sparse-fill".to_string()]);
    assert!(
        !view.should_emit(ExtrusionRole::RaftInfill),
        "module holding only claim:sparse-fill must NOT emit RaftInfill"
    );
}

#[test]
fn ac_n3_empty_held_claims_suppress_raft_infill() {
    let view = SliceRegionView::default();
    assert!(
        view.held_claims().is_empty(),
        "default view must have empty held_claims"
    );
    assert!(
        !view.should_emit(ExtrusionRole::RaftInfill),
        "empty held_claims must suppress RaftInfill emission"
    );
}
