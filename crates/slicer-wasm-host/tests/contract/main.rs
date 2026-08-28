//! Aggregator for `slicer-wasm-host` contract-scope tests.
//! One Cargo integration-test binary; each test file below is a submodule.

#![allow(missing_docs)]

#[path = "../common/mod.rs"]
mod common;

mod authored_coloring_grant_and_strip_tdd;
mod effective_perimeter_origin_integration_tdd;
mod exact_z_support_query;
mod finalization_role_round_trip_tdd;
mod host_services_tdd;
mod infill_holder_resolution_painted_region_tdd;
mod layer_collection_builder_contract_tdd;
mod lightning_dispatch_per_region_keying_tdd;
mod lightning_infill_guest_calls_lightning_tree_segments_tdd;
mod order_lock_marshal_round_trip_tdd;
mod perimeter_infill_per_origin_route_tdd;
mod prepass_output_builder_validation_tdd;
mod production_guest_smoke_tdd;
mod seam_plan_harvest_custom_paint_value_tdd;
mod set_current_origin_routes_to_correct_bucket_tdd;
mod slice_region_view_contract_tdd;
mod support_decline_contract;
mod support_identity_layer_dispatch_tdd;
mod support_plan_structural_contract;
mod support_plan_validation;
mod surface_group_resolution_tdd;
mod typed_config_boundary_tdd;
mod view_seam_identity_tdd;
mod wit_boundary_tdd;
mod z_envelope_contract_tdd;

#[test]
fn exact_z_support_query() {
    exact_z_support_query::exact_z_support_query();
}

#[test]
fn support_decline_contract() {
    support_decline_contract::support_decline_contract();
}

#[test]
fn support_plan_validation() {
    support_plan_validation::support_plan_validation();
}

#[test]
fn support_plan_structural_contract() {
    support_plan_structural_contract::support_plan_structural_contract();
}
