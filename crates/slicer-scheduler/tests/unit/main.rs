// crates/slicer-scheduler/tests/unit/main.rs
//
// Aggregator for unit-scope tests of the slicer-scheduler crate.
// Wasmtime-free; no slicer-runtime or slicer-wasm-host dependencies.

#![allow(missing_docs)]

mod dag_construction_tdd;
mod execution_plan_tdd;
mod stage_canon_seam_support_tdd;
mod topological_sort_tdd;
