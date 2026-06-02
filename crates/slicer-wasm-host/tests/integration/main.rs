//! Aggregator for `slicer-wasm-host` integration-scope tests.
//! One Cargo integration-test binary; each test file below is a submodule.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod wasm_instance_tdd;
