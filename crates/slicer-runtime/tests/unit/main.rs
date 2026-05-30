// crates/slicer-runtime/tests/unit/main.rs
//
// Aggregator for unit-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod blackboard_layer_arena_tdd;
mod blackboard_support_geometry_slot_tdd;
mod bridge_detector_tdd;
mod builtin_producers_tdd;
mod dag_construction_tdd;
mod dag_validation_tdd;
mod execution_plan_tdd;
mod gcode_emit_format_tdd;
mod gcode_emit_per_role_tolerance_tdd;
mod layer_collection_builder_tdd;
mod macro_mesh_raycast_z_down_tdd;
mod mesh_analysis_tdd;
mod multi_object_transform_world_z_tdd;
mod non_uniform_scale_tdd;
mod object_bounds_transform_tdd;
mod overhang_speed_tdd;
mod paint_region_annotator_host_tdd;
mod path_ordering_tdd;
mod raycast_z_down_hit_tdd;
mod raycast_z_down_invalid_object_tdd;
mod raycast_z_down_miss_tdd;
mod raycast_z_down_transformed_object_tdd;
mod region_mapping_resolved_config_tdd;
mod rotated_object_world_extent_tdd;
mod support_layer_height_validation_tdd;
mod surface_normal_at_oob_tdd;
mod surface_normal_at_unit_length_tdd;
mod tool_ordering_tdd;
mod topological_sort_tdd;
mod transformed_model_world_z_tdd;
mod translated_object_z_floor_tdd;
mod world_z_below_floor_tdd;
mod world_z_canonical_surface_tdd;
