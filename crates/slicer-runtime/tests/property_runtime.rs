//! Registry-backed runtime properties: `H-RUNTIME-DISPATCH`, `H-RUNTIME-SELECTION`.
//!
//! Each test reads its exact name and bounds from the registry context
//! (`PNP_PROPERTY_CONTEXT`) through `PropertyRun`, so the generative bodies live
//! in the target-private model modules this file mounts. None of them may
//! hardcode a Proptest configuration.
//!
//! The charters, oracles and failure destinations are documented in
//! `docs/testing/properties.md#h-runtime-dispatch` and
//! `#h-runtime-selection`; the executed evidence is
//! `docs/testing/proptest-runtime-evidence.md`.

#![allow(missing_docs)]

#[path = "common/property_runtime_dispatch.rs"]
mod dispatch;

#[path = "common/property_runtime_selection.rs"]
mod selection;

#[test]
fn proptest_runtime_semantic_dispatch() {
    dispatch::run_property();
}

#[test]
fn proptest_runtime_selection_authority() {
    selection::run_property();
}
