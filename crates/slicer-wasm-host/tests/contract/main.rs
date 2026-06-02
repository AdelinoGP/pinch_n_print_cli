//! Aggregator for `slicer-wasm-host` contract-scope tests.
//! One Cargo integration-test binary; each test file below is a submodule.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod host_services_tdd;
mod prepass_output_builder_validation_tdd;
mod typed_config_boundary_tdd;
mod wit_boundary_tdd;
mod z_envelope_contract_tdd;
