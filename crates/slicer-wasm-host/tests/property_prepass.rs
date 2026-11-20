//! Registry-backed pre-pass properties owned by the host: `H-SUPPORT-TERRITORY`.

#![allow(missing_docs)]

mod property_prepass_common;

#[test]
fn proptest_support_territory_clearance() {
    if let Err(error) = property_prepass_common::territory::run_property() {
        panic!("H-SUPPORT-TERRITORY failed: {error}");
    }
}
