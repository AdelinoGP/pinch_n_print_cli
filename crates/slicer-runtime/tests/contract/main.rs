// crates/slicer-runtime/tests/contract/main.rs
//
// Aggregator for contract-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod authored_tool_index_tdd;
mod config_view_binding_tdd;
mod config_view_encapsulation_source_tdd;
mod dispatch_config_tdd;
mod dispatch_identity_tdd;
mod dispatch_infill_output_tdd;
mod dispatch_missing_component_tdd;
mod dispatch_pathopt_tdd;
mod dispatch_perimeter_output_tdd;
mod dispatch_prepass_harvest_tdd;
mod dispatch_protocol_tdd;
mod dispatch_support_output_tdd;
mod guest_fixture_freshness_tdd;
mod infill_postprocess_contract_tdd;
mod inner_wall_boundary_type_tdd;
mod integrated_parity_arachne_perimeters_tdd;
mod integrated_parity_classic_perimeters_tdd;
mod integrated_parity_fuzzy_skin_tdd;
mod integrated_parity_gyroid_infill_tdd;
mod integrated_parity_infill_linker_tdd;
mod integrated_parity_layer_planner_tdd;
mod integrated_parity_lightning_infill_tdd;
mod integrated_parity_machine_gcode_emit_tdd;
mod integrated_parity_overhang_classifier_tdd;
mod integrated_parity_part_cooling_tdd;
mod integrated_parity_path_optimization_tdd;
mod integrated_parity_rectilinear_infill_tdd;
mod integrated_parity_seam_placer_tdd;
mod integrated_parity_seam_planner_tdd;
mod integrated_parity_skirt_brim_tdd;
mod integrated_parity_support_planner_tdd;
mod integrated_parity_support_surface_ironing_tdd;
mod integrated_parity_top_surface_ironing_tdd;
mod integrated_parity_traditional_support_tdd;
mod integrated_parity_tree_support_tdd;
mod integrated_parity_wave_overhangs_tdd;
mod integrated_parity_wipe_tower_tdd;
mod layer_stage_commit_stages_tdd;
mod lightning_tree_per_region_roundtrip_tdd;
mod lightning_tree_view_roundtrip_tdd;
mod macro_all_worlds_roundtrip_tdd;
mod macro_postpass_text_roundtrip_tdd;
mod modifier_split_subregion_density_tdd;
mod native_adapter_tdd;
mod native_dispatch_parity_seam_tdd;
mod native_infill_claim_resolution_tdd;
mod only_one_wall_first_layer_tdd;
mod only_one_wall_top_tdd;
mod overhang_areas_empty_until_p106_tdd;
mod paint_region_transport_widening_tdd;
mod parity_invariants_selftest_tdd;
mod per_layer_config_override_tdd;
mod per_region_config_tdd;
mod per_vertex_is_bridge_propagation_tdd;
mod perimeter_builder_capacity_error_tdd;
mod postpass_gcode_boundary_tdd;
mod postpass_gcode_command_preservation_tdd;
mod postpass_gcode_emit_contract_tdd;
mod postpass_gcode_empty_list_tdd;
mod slice_region_view_overhang_areas_non_empty_tdd;
mod wit_drift_detection_tdd;
mod wit_single_source_tdd;
