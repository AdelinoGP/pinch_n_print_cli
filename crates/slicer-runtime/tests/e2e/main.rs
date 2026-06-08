// crates/slicer-runtime/tests/e2e/main.rs
//
// Aggregator for e2e-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod acceptance_gate_gaps_tdd;
mod benchy_end_to_end_tdd;
mod cube_4color_modifier_part_e2e_tdd;
mod cube_painted_e2e_tdd;
mod cube_painted_overrides_e2e_tdd;
mod run_slice_api_tdd;
mod scenario_traces_tdd;
mod slicer_report_html_tdd;
mod slicing_precision_integration_tdd;
mod slicing_promotion_e2e_dispatch_regression_tdd;
mod threemf_fixture_e2e_tdd;
mod threemf_subtypes_synthetic_e2e_tdd;
