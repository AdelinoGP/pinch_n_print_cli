//! Registry-backed scripted geometry properties: `H-BOOL-SCAD`, `H-MESH-SCAD`.
//!
//! Both tests read their exact name and bounds from the registry context
//! (`PNP_PROPERTY_CONTEXT`) through `PropertyRun`, so the generative bodies and
//! their independent references live in the target-private
//! `property_geometry_common` module this file mounts. Neither test hardcodes a
//! Proptest configuration nor consults the SCAD engine as an oracle: the
//! converted contours and meshes are stimulus only.
//!
//! This target is registered with `required-features = ["property_testing_scad"]`
//! and is never part of `default` or `property_testing`, so non-SCAD execution
//! never depends on the engine. The charters live in
//! `docs/testing/properties.md#h-bool-scad` and `#h-mesh-scad`.

#![allow(missing_docs)]

mod property_geometry_common;

#[test]
fn proptest_scripted_boolean_membership() {
    property_geometry_common::scad_boolean::run_property();
}

#[test]
fn proptest_scripted_mesh_cross_sections() {
    property_geometry_common::scad_mesh::run_property();
}
