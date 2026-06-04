// crates/slicer-scheduler/tests/contract/main.rs
//
// Aggregator for contract-scope tests of the slicer-scheduler crate.
// Wasmtime-free; no slicer-runtime or slicer-wasm-host dependencies.

#![allow(missing_docs)]

mod claim_transition_matrix_tdd;
mod core_module_ir_access_contract_tdd;
mod module_manifest_tdd;
mod stage_list_consistency_tdd;
