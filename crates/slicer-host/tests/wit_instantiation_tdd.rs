//! TDD test: a guest exporting the obsolete pre-31a-REV2 support-generation symbol
//! must fail WIT instantiation (the symbol is not present in the
//! `slicer:world-prepass` WIT interface after the 31a-REV2 revert).
//!
//! # Why this test is `#[ignore]`
//!
//! The host's WIT instantiation path (`WasmComponent` / `WasmInstance`) requires
//! a pre-compiled `.wasm` component binary — there is no mechanism in the current
//! test infrastructure to construct a synthetic, in-memory WASM component that
//! exports a specific WIT function name without building an actual WASM module.
//!
//! The equivalent check is already exercised transitively: the WIT drift-detection
//! test (`wit_drift_detection_tdd.rs`) asserts that the canonical `wit/world-prepass.wit`
//! no longer contains `run-support-geometry`, and the manifest loader rejects the
//! removed stage id (`manifest_unknown_stage_tdd.rs`). Together those form the
//! negative-case boundary that this test is intended to cover.
//!
//! To make this a live test, build a guest WASM that exports `run-support-geometry`
//! under `slicer:world-prepass` and provide it at the path below; then remove
//! the `#[ignore]` attribute and implement the assertion.

#![allow(missing_docs)]

/// Path where a pre-built "obsolete export" guest component would live.
/// This component does not exist and is never built; the test is ignored.
const _OBSOLETE_GUEST_COMPONENT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/obsolete-support-geometry-guest.component.wasm"
);

/// Asserts that a guest component exporting the obsolete pre-31a-REV2 support-geometry
/// WIT function fails linker instantiation.
///
/// Marked `#[ignore]` because no mechanism exists to build a synthetic
/// in-memory WASM component for this purpose from a pure Rust test. See
/// module-level doc for the rationale and the transitively equivalent checks
/// that cover the same invariant.
#[test]
#[ignore = "requires a pre-built obsolete-export guest WASM; \
            see module doc for rationale and equivalent coverage"]
fn obsolete_run_support_geometry_export_rejected() {
    // When un-ignoring: load the bytes from _OBSOLETE_GUEST_COMPONENT_PATH,
    // attempt to instantiate via WasmComponent / WasmInstance, and assert
    // that the result is an error whose message mentions `run-support-geometry`
    // or indicates that the export is not present in the host interface.
    todo!("build a guest exporting run-support-geometry and assert instantiation fails");
}
