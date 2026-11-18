//! Deterministic controls and concrete regressions for `H-ARACHNE-BEADING`.
//!
//! Every test here runs under a plain
//! `cargo test -p slicer-core --features host-algos,property_testing --test property_arachne_contracts`.
//! They are hand-built witnesses that (a) prove the independent comparator the
//! registry property uses rejects the faults that property must catch, (b) show
//! the real strategies pass that same comparator on named fixtures, (c) reject
//! unusable registry bounds before any sampling, and (d) pin two concrete
//! real-kernel widening cases at unit scale. No control in this binary
//! substitutes for a registry execution, and none of them reads
//! `PNP_PROPERTY_CONTEXT`.
//!
//! The module's own inner `#![allow(dead_code)]` covers the subset this target
//! uses; repeating it here is rejected by `clippy::duplicated_attributes`.

#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

mod property_arachne_common;

#[test]
fn arachne_beading_oracle_controls() {
    property_arachne_common::controls_beading::oracle_controls();
}

#[test]
fn arachne_property_domain_rejections() {
    property_arachne_common::controls_beading::domain_rejections();
}

#[test]
fn arachne_beading_property_body_smoke() {
    property_arachne_common::controls_beading::beading_body_smoke();
}

#[test]
fn widening_two_requested_below_optimal_delegates_to_parent() {
    property_arachne_common::controls_beading::widening_two_requested_below_optimal_delegates_to_parent();
}

#[test]
fn widening_transition_propagation_preserves_two_bead_endpoint() {
    property_arachne_common::controls_beading::widening_transition_propagation_preserves_two_bead_endpoint();
}
