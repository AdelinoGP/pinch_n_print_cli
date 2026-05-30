// crates/slicer-runtime/tests/contract/main.rs
//
// Aggregator for contract-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod claim_transition_matrix_tdd;
mod config_view_binding_tdd;
mod config_view_encapsulation_source_tdd;
mod core_module_ir_access_contract_tdd;
mod dispatch_tdd;
mod guest_fixture_freshness_tdd;
mod host_services_tdd;
mod macro_all_worlds_roundtrip_tdd;
mod macro_mesh_segmentation_output_roundtrip_tdd;
mod macro_paint_region_roundtrip_tdd;
mod macro_paint_segmentation_output_roundtrip_tdd;
mod macro_postpass_text_roundtrip_tdd;
mod module_manifest_tdd;
mod paint_region_transport_widening_tdd;
mod postpass_gcode_boundary_tdd;
mod postpass_gcode_command_preservation_tdd;
mod postpass_gcode_emit_contract_tdd;
mod postpass_gcode_empty_list_tdd;
mod prepass_output_builder_validation_tdd;
mod stage_list_consistency_tdd;
mod typed_config_boundary_tdd;
mod wit_boundary_tdd;
mod wit_drift_detection_tdd;
mod wit_single_source_tdd;
mod z_envelope_contract_tdd;
