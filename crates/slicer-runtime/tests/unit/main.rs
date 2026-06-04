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
mod dag_validation_tdd;
mod host_keys_doc_lock_tdd;
mod layer_collection_builder_tdd;
mod mesh_analysis_tdd;
mod multi_object_transform_world_z_tdd;
mod paint_region_annotator_host_tdd;
mod path_ordering_tdd;
mod region_mapping_resolved_config_tdd;
mod rotated_object_world_extent_tdd;
mod tool_ordering_tdd;
mod transformed_model_world_z_tdd;
mod translated_object_z_floor_tdd;
