//! Deterministic controls for the geometry charters (`H-BOOL-*`).
//!
//! Every control here runs under a plain
//! `cargo test -p slicer-core --features property_testing,host-algos --test property_geometry_controls`.
//! They are hand-built witnesses that prove the independent references used by
//! the registry properties reject the faults those properties must catch, that
//! the real SUT passes them on named fixtures, and that invalid profile bounds
//! fail before sampling. No control in this binary substitutes for a registry
//! execution.

#![allow(missing_docs)]

// The module's own inner `#![allow(dead_code)]` covers the subset this target
// uses; repeating it here is rejected by `clippy::duplicated_attributes`.
mod property_geometry_common;

#[test]
fn geometry_controls_boolean_and_clip_mutations() {
    property_geometry_common::controls_boolean_clip::run();
}

#[test]
fn geometry_controls_profile_bounds_and_envelopes() {
    property_geometry_common::bounds::controls_run();
}

#[test]
fn geometry_controls_offset_and_smooth_mutations() {
    property_geometry_common::controls_offset_smooth::run();
}

#[test]
fn geometry_controls_slice_and_query_mutations() {
    property_geometry_common::controls_slice_query::run();
}

#[test]
fn geometry_controls_vertex_plane_and_hole_witnesses() {
    property_geometry_common::controls_vertex_plane_hole::run();
}

#[test]
fn geometry_controls_owned_shrink_witnesses() {
    property_geometry_common::controls_shrink::run();
}

#[test]
fn geometry_controls_seeded_fault_is_minimised() {
    property_geometry_common::controls_seeded_fault::run();
}
