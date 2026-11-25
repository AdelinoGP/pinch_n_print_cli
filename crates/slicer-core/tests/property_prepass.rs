//! Registry-backed pre-pass properties: `H-REGIONMAP`.
//!
//! Each test reads its exact name and bounds from the registry context
//! (`PNP_PROPERTY_CONTEXT`) through `PropertyRun`, so the generative body lives
//! in the target-private `property_prepass_common` module this file mounts.
//! It never hardcodes a Proptest configuration.
//!
//! The charter, oracle and failure destination are documented in
//! `docs/testing/properties.md#h-regionmap`; the executed evidence is
//! `docs/testing/proptest-prepass-evidence.md`.

#![allow(missing_docs)]

mod property_prepass_common;

#[test]
fn proptest_region_mapping_expansion() {
    if let Err(error) = property_prepass_common::model::run_property() {
        panic!("H-REGIONMAP failed: {error}");
    }
}

#[test]
fn proptest_paint_composition_cells() {
    if let Err(error) = property_prepass_common::paint::run_property() {
        panic!("H-PAINT-COMPOSE failed: {error}");
    }
}

#[test]
fn proptest_overhang_adjacent_layer_bands() {
    if let Err(error) = property_prepass_common::overhang::run_property() {
        panic!("H-PAINT-OVERHANG failed: {error}");
    }
}

#[test]
fn proptest_paint_driver_source_isolation() {
    if let Err(error) = property_prepass_common::driver::run_property() {
        panic!("H-PAINT-DRIVER failed: {error}");
    }
}

#[test]
fn proptest_support_schedule_and_sources() {
    if let Err(error) = property_prepass_common::support::run_property() {
        panic!("H-SUPPORT-SCHEDULE failed: {error}");
    }
}

#[test]
fn proptest_support_role_partition() {
    if let Err(error) = property_prepass_common::roles::run_property() {
        panic!("H-SUPPORT-ROLES failed: {error}");
    }
}

#[test]
fn proptest_support_contact_rectangles() {
    if let Err(error) = property_prepass_common::contact::run_property() {
        panic!("H-SUPPORT-CONTACT failed: {error}");
    }
}
