// crates/slicer-runtime/tests/integration/main.rs
//
// Aggregator for integration-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod adapt_slice_regions_completeness_tdd;
mod core_module_components_tdd;
mod core_module_macro_adoption_tdd;
mod gcode_header_thumbnail_config_blocks_tdd;
mod gcode_part_cooling_emission_tdd;
mod gcode_skirt_brim_emission_tdd;
mod gcode_wall_closure_tdd;
mod infill_partition_e2e_tdd;
mod infill_partitioned_input_tdd;
mod live_module_loading_tdd;
mod machine_start_end_gcode_emission_tdd;
mod multi_infill_holder_dispatch_tdd;
mod perimeter_postprocess_preserve_tdd;
mod pipeline_tdd;
mod progress_events_tdd;
mod region_map_cap_overflow_tdd;
mod region_mapping_paint_semantic_tdd;
mod region_mapping_tdd;
mod region_partition_tdd;
mod region_split_dispatch_filter;
mod run_pipeline_with_instrumentation_tdd;
mod runtime_wiring_tdd;
mod support_geometry_config_normalization_tdd;
mod threemf_paint_drop_on_modifier_tdd;
mod threemf_transform_tdd;
mod wasm_instance_pool_tdd;
