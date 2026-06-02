//! TDD coverage for `raycast_z_down` hit semantics across every host-service world.
//!
//! Verification: `cargo test -p slicer-runtime --test raycast_z_down_hit_tdd -- --nocapture`

#![allow(missing_docs)]

use crate::common::{
    assert_close, ctx_with_mesh, flat_plate_object, identity_transform, mesh_fixture,
};
use slicer_wasm_host::host::{
    finalization::slicer::world_finalization::host_services as fhs,
    layer::slicer::world_layer::host_services as lhs,
    postpass::slicer::world_postpass::host_services as pphs,
    prepass::slicer::world_prepass::host_services as phs,
};

#[test]
fn raycast_hit_returns_some_world_z_across_all_worlds() {
    let mesh = mesh_fixture(vec![flat_plate_object("plate", 0.0, identity_transform())]);

    let mut layer_ctx = ctx_with_mesh("mesh-query.layer.hit", mesh.clone());
    let mut prepass_ctx = ctx_with_mesh("mesh-query.prepass.hit", mesh.clone());
    let mut finalization_ctx = ctx_with_mesh("mesh-query.finalization.hit", mesh.clone());
    let mut postpass_ctx = ctx_with_mesh("mesh-query.postpass.hit", mesh.clone());

    let layer_hit = lhs::Host::raycast_z_down(&mut layer_ctx, "plate".to_string(), 5.0, 5.0, 10.0)
        .expect("layer raycast should not error")
        .expect("layer raycast should hit the flat plate");
    let prepass_hit =
        phs::Host::raycast_z_down(&mut prepass_ctx, "plate".to_string(), 5.0, 5.0, 10.0)
            .expect("prepass raycast should not error")
            .expect("prepass raycast should hit the flat plate");
    let finalization_hit =
        fhs::Host::raycast_z_down(&mut finalization_ctx, "plate".to_string(), 5.0, 5.0, 10.0)
            .expect("finalization raycast should not error")
            .expect("finalization raycast should hit the flat plate");
    let postpass_hit =
        pphs::Host::raycast_z_down(&mut postpass_ctx, "plate".to_string(), 5.0, 5.0, 10.0)
            .expect("postpass raycast should not error")
            .expect("postpass raycast should hit the flat plate");

    assert_close(layer_hit, 0.0, "layer world_z");
    assert_close(prepass_hit, 0.0, "prepass world_z");
    assert_close(finalization_hit, 0.0, "finalization world_z");
    assert_close(postpass_hit, 0.0, "postpass world_z");
}
