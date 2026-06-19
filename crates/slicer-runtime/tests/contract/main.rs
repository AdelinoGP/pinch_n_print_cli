// crates/slicer-runtime/tests/contract/main.rs
//
// Aggregator for contract-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod config_view_binding_tdd;
mod config_view_encapsulation_source_tdd;
mod dispatch_config_tdd;
mod dispatch_identity_tdd;
mod dispatch_infill_output_tdd;
mod dispatch_pathopt_tdd;
mod dispatch_perimeter_output_tdd;
mod dispatch_prepass_harvest_tdd;
mod dispatch_protocol_tdd;
mod dispatch_support_output_tdd;
mod guest_fixture_freshness_tdd;
mod layer_stage_commit_stages_tdd;
mod macro_all_worlds_roundtrip_tdd;
mod macro_postpass_text_roundtrip_tdd;
mod paint_region_transport_widening_tdd;
mod per_layer_config_override_tdd;
mod perimeter_builder_capacity_error_tdd;
mod postpass_gcode_boundary_tdd;
mod postpass_gcode_command_preservation_tdd;
mod postpass_gcode_emit_contract_tdd;
mod postpass_gcode_empty_list_tdd;
mod wit_drift_detection_tdd;
mod wit_single_source_tdd;
