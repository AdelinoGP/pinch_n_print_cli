// crates/slicer-scheduler/tests/integration/main.rs
//
// Aggregator for integration-scope tests of the slicer-scheduler crate.
// Wasmtime-free; no slicer-runtime or slicer-wasm-host dependencies.

#![allow(missing_docs)]

mod config_bounds_enforcement_tdd;
mod config_resolution_paint_semantic_tdd;
mod config_resolution_tdd;
mod dag_cli_integration;
mod manifest_ingestion_tdd;
mod manifest_unknown_stage_tdd;
