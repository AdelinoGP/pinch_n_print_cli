// crates/slicer-scheduler/tests/integration/main.rs
//
// Aggregator for integration-scope tests of the slicer-scheduler crate.
// Wasmtime-free; no slicer-runtime or slicer-wasm-host dependencies.

#![allow(missing_docs)]

mod capability_derived_anchor_closure;

#[test]
fn capability_derived_anchor_closure() {
    capability_derived_anchor_closure::capability_derived_anchor_closure();
}
mod config_bounds_enforcement_tdd;
mod config_resolution_paint_semantic_tdd;
mod config_resolution_tdd;
mod dag_cli_integration;
mod integrated_tier_tdd;
mod manifest_ingestion_tdd;
mod manifest_unknown_stage_tdd;
