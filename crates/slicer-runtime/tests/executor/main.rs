// crates/slicer-runtime/tests/executor/main.rs
//
// Aggregator for executor-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod cube_4color_paint_tdd;
mod cube_fuzzy_painted_tdd;
mod finalization_builder_insert;
mod finalization_builder_permute;
mod finalization_builder_readback;
mod finalization_live_tdd;
mod finalization_mutation_roundtrip_tdd;
mod finalization_world_deep_copy_tdd;
mod layer_executor_tdd;
mod layer_finalization_tdd;
mod layer_slice_tdd;
mod layer_world_deep_copy_tdd;
mod live_layer_support_tdd;
mod live_seam_path_tdd;
mod live_top_bottom_fill_tdd;
mod live_travel_policy_tdd;
mod macro_finalization_deep_copy_tdd;
mod mesh_segmentation_executor_tdd;
mod paint_segmentation_executor_tdd;
mod paint_segmentation_host_tdd;
mod postpass_executor_tdd;
mod prepass_execution_order_tdd;
mod prepass_executor_tdd;
mod prepass_seam_planning_macro_path_tdd;
mod prepass_slice_and_shell_tdd;
mod prepass_support_geometry_layer_plan_tdd;
mod prepass_support_geometry_tdd;
mod slice_postprocess_paint_annotation_tdd;
mod slicing_promotion_e2e_regression_tdd;
mod support_geometry_slice_consumption_tdd;
