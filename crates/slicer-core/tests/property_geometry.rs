//! Registry-backed geometry properties: `H-BOOL-SET`, `H-BOOL-CLIP`.
//!
//! Each test reads its exact name and bounds from the registry context
//! (`PNP_PROPERTY_CONTEXT`) through `PropertyRun`, so the generative bodies and
//! their independent references live in the target-private
//! `property_geometry_common` module this file mounts. None of them may
//! hardcode a Proptest configuration. Deterministic, RNG-independent witnesses
//! run before each generated campaign.
//!
//! The charters, oracles and failure destinations are documented in
//! `docs/testing/properties.md#h-bool-set` and `#h-bool-clip`.

#![allow(missing_docs)]

mod property_geometry_common;

#[test]
fn proptest_boolean_set_algebra() {
    property_geometry_common::boolean::run_property();
}

#[test]
fn proptest_clip_polylines_full_segments() {
    property_geometry_common::clip::run_property();
}

#[test]
fn proptest_offset_rectangular_clearance() {
    property_geometry_common::offset::run_property();
}

#[test]
fn proptest_smooth_outward_safe_families() {
    property_geometry_common::smooth::run_property();
}

#[test]
fn proptest_slice_analytical_solids() {
    property_geometry_common::mesh_slice::run_property();
}

#[test]
fn proptest_mesh_transform_and_aabb() {
    property_geometry_common::mesh_query::run_property();
}
