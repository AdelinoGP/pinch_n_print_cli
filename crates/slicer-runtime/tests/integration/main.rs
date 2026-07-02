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
mod extra_perimeters_config_tdd;
mod extra_perimeters_on_overhangs_tdd;
mod gap_fill_emission_tdd;
mod gcode_header_thumbnail_config_blocks_tdd;
mod gcode_part_cooling_emission_tdd;
mod gcode_skirt_brim_emission_tdd;
mod gcode_wall_closure_tdd;
mod infill_partition_e2e_tdd;
mod infill_partitioned_input_tdd;
mod live_module_loading_tdd;
mod machine_start_end_gcode_emission_tdd;
mod manifest_default_reconcile_tdd;
mod medial_axis_failure_observable_tdd;
mod mmu_per_color_fragmentation_tdd;
mod multi_infill_holder_dispatch_tdd;
mod narrow_island_smaller_perimeter_tdd;
mod nonplanar_shell_emission_tdd;
mod outer_inner_width_and_spacing_tdd;
mod overhang_classifier_refactor_regression_tdd;
mod overhang_pipeline_e2e_tdd;
mod painted_seam_enforcer_blocker_tdd;
mod per_object_config_override_tdd;
mod perimeter_postprocess_preserve_tdd;
mod pipeline_tdd;
mod precise_outer_wall_tdd;
mod progress_events_tdd;
mod region_map_cap_overflow_tdd;
mod region_mapping_paint_semantic_tdd;
mod region_mapping_tdd;
mod region_partition_tdd;
mod region_split_dispatch_filter;
mod run_pipeline_with_instrumentation_tdd;
mod runtime_wiring_tdd;
mod support_geometry_config_normalization_tdd;
mod thin_wall_emission_tdd;
mod threemf_paint_drop_on_modifier_tdd;
mod threemf_transform_tdd;
mod wasm_instance_pool_tdd;
