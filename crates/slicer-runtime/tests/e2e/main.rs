// crates/slicer-runtime/tests/e2e/main.rs
//
// Aggregator for e2e-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod acceptance_gate_gaps_tdd;
mod calicat_internal_bridge_arbiter_e2e_tdd;
mod calicat_internal_bridge_gating_e2e_tdd;
mod cube_4color_modifier_part_e2e_tdd;
mod cube_painted_e2e_tdd;
mod cube_painted_overrides_e2e_tdd;
mod infill_overlap_changes_gcode_tdd;
mod mixed_density_internal_bridge_rejection_e2e_tdd;
mod mm_real_fixture_gcode_tdd;
mod modifier_infill_tdd;
mod painted_fixture_parity_tdd;
mod run_slice_api_tdd;
mod scenario_traces_tdd;
mod slice_end_to_end_tdd;
mod slicer_report_html_tdd;
mod slicing_precision_integration_tdd;
mod slicing_promotion_e2e_dispatch_regression_tdd;
mod threemf_fixture_e2e_tdd;
mod threemf_subtypes_synthetic_e2e_tdd;
mod wave_overhang_bridge_fill_e2e_tdd;
mod wedge_linked_infill_report_tdd;
