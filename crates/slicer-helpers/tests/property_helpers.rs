//! Registry-backed helper properties: `H-HELPERS-SPLIT`.
//!
//! The test reads its exact name and bounds from the registry context
//! (`PNP_PROPERTY_CONTEXT`) through `PropertyRun`, so the generative body lives
//! in the target-private module this file mounts. It must not hardcode a
//! Proptest configuration.
//!
//! The charter, oracle and failure destination are documented in
//! `docs/testing/properties.md#h-helpers-split`; the executed evidence belongs to
//! `docs/testing/proptest-geometry-evidence.md`.

#![allow(missing_docs)]

mod property_helper_common;

#[test]
fn proptest_helpers_split_components() {
    property_helper_common::split::run_property();
}
