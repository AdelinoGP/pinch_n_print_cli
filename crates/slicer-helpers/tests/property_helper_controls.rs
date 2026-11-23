//! Deterministic controls for the helper charter (`H-HELPERS-SPLIT`).
//!
//! Every test here runs under a plain
//! `cargo test -p slicer-helpers --features property_testing --test property_helper_controls`.
//! They are hand-built witnesses that prove the independent face-partition model
//! and the local value tree reject the faults the randomized property must catch,
//! and that the real `split_connected_components` output passes. No control in
//! this binary substitutes for a registry execution.
//!
//! The mounted module is shared with `tests/property_helpers.rs` and declares its
//! own `#![allow(dead_code)]`: each binary uses only half of it, and repeating the
//! allow on this `mod` line would be a duplicated attribute.

#![allow(missing_docs)]

mod property_helper_common;

#[test]
fn helper_controls_edge_adjacency_and_singleton_mutations() {
    property_helper_common::controls::edge_adjacency_and_singleton_mutations();
}

#[test]
fn helper_controls_owned_shrink_witnesses() {
    property_helper_common::controls::owned_shrink_witnesses();
}
