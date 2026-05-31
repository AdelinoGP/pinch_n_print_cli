// crates/slicer-runtime/tests/integration/main.rs
//
// Aggregator for integration-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod config_bounds_enforcement_tdd;
mod config_resolution_paint_semantic_tdd;
mod config_resolution_tdd;
mod core_module_components_tdd;
mod core_module_macro_adoption_tdd;
mod dag_cli_integration;
mod gcode_emit_tdd;
mod gcode_emit_travel_anchor_tdd;
mod gcode_feedrate_emission_tdd;
mod gcode_header_thumbnail_config_blocks_tdd;
mod gcode_part_cooling_emission_tdd;
mod gcode_relative_extrusion_tdd;
mod gcode_skirt_brim_emission_tdd;
mod live_module_loading_tdd;
mod machine_start_end_gcode_emission_tdd;
mod macro_mesh_segmentation_geometry_tdd;
mod manifest_ingestion_tdd;
mod manifest_unknown_stage_tdd;
mod model_loader_tdd;
mod model_writer_roundtrip_tdd;
mod paint_annotation_integration_tdd;
mod pipeline_tdd;
mod prepass_paint_semantic_override_ordering_tdd;
mod progress_events_tdd;
mod region_mapping_paint_semantic_tdd;
mod region_mapping_tdd;
mod run_pipeline_with_instrumentation_tdd;
mod runtime_wiring_tdd;
mod support_geometry_config_normalization_tdd;
mod threemf_paint_drop_on_modifier_tdd;
mod threemf_sidecar_classification_tdd;
mod threemf_transform_tdd;
mod wasm_instance_pool_tdd;
mod wasm_instance_tdd;
