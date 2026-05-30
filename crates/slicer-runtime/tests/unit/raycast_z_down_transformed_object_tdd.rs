//! TDD coverage for transform-aware `raycast_z_down` semantics across every host-service world.
//!
//! Verification: `cargo test -p slicer-runtime --test raycast_z_down_transformed_object_tdd -- --nocapture`

#![allow(missing_docs)]

use crate::common::{
    assert_close, ctx_with_mesh, flat_plate_object, mesh_fixture, translation_transform,
};
use slicer_runtime::wit_host::{
    finalization::slicer::world_finalization::host_services as fhs,
    layer::slicer::world_layer::host_services as lhs,
    postpass::slicer::world_postpass::host_services as pphs,
    prepass::slicer::world_prepass::host_services as phs,
};

#[test]
fn raycast_transformed_object_returns_world_space_z_across_all_worlds() {
    let mesh = mesh_fixture(vec![flat_plate_object(
        "translated-plate",
        0.0,
        translation_transform(5.0, 7.0, 10.0),
    )]);

    let mut layer_ctx = ctx_with_mesh("mesh-query.layer.transform", mesh.clone());
    let mut prepass_ctx = ctx_with_mesh("mesh-query.prepass.transform", mesh.clone());
    let mut finalization_ctx = ctx_with_mesh("mesh-query.finalization.transform", mesh.clone());
    let mut postpass_ctx = ctx_with_mesh("mesh-query.postpass.transform", mesh.clone());

    let layer_hit = lhs::Host::raycast_z_down(
        &mut layer_ctx,
        "translated-plate".to_string(),
        10.0,
        12.0,
        25.0,
    )
    .expect("layer transformed raycast should not error")
    .expect("layer transformed raycast should hit");
    let prepass_hit = phs::Host::raycast_z_down(
        &mut prepass_ctx,
        "translated-plate".to_string(),
        10.0,
        12.0,
        25.0,
    )
    .expect("prepass transformed raycast should not error")
    .expect("prepass transformed raycast should hit");
    let finalization_hit = fhs::Host::raycast_z_down(
        &mut finalization_ctx,
        "translated-plate".to_string(),
        10.0,
        12.0,
        25.0,
    )
    .expect("finalization transformed raycast should not error")
    .expect("finalization transformed raycast should hit");
    let postpass_hit = pphs::Host::raycast_z_down(
        &mut postpass_ctx,
        "translated-plate".to_string(),
        10.0,
        12.0,
        25.0,
    )
    .expect("postpass transformed raycast should not error")
    .expect("postpass transformed raycast should hit");

    assert_close(layer_hit, 10.0, "layer transformed world_z");
    assert_close(prepass_hit, 10.0, "prepass transformed world_z");
    assert_close(finalization_hit, 10.0, "finalization transformed world_z");
    assert_close(postpass_hit, 10.0, "postpass transformed world_z");
}
