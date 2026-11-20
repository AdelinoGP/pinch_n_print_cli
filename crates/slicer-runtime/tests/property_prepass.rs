//! Registry-backed pre-pass properties owned by the runtime: `H-SUPPORT-ANALYSIS`.

#![allow(missing_docs)]

mod property_prepass_common;

#[test]
fn proptest_support_analysis_layered_scenes() {
    if let Err(error) = property_prepass_common::analysis::run_property() {
        panic!("H-SUPPORT-ANALYSIS failed: {error}");
    }
}
