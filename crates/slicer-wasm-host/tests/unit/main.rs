//! Aggregator for `slicer-wasm-host` unit-scope tests.
//! One Cargo integration-test binary; each test file below is a submodule.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod macro_mesh_raycast_z_down_tdd;
mod object_bounds_transform_tdd;
mod raycast_z_down_hit_tdd;
mod raycast_z_down_invalid_object_tdd;
mod raycast_z_down_miss_tdd;
mod raycast_z_down_transformed_object_tdd;
mod surface_normal_at_oob_tdd;
mod surface_normal_at_unit_length_tdd;
