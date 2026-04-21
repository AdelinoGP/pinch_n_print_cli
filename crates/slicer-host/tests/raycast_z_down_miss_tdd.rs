//! TDD coverage for `raycast_z_down` miss semantics across every host-service world.
//!
//! Verification: `cargo test -p slicer-host --test raycast_z_down_miss_tdd -- --nocapture`

#![allow(missing_docs)]

mod common;

use common::{ctx_with_mesh, flat_plate_object, identity_transform, mesh_fixture};
use slicer_host::wit_host::{
    finalization::slicer::world_finalization::host_services as fhs,
    layer::slicer::world_layer::host_services as lhs,
    postpass::slicer::world_postpass::host_services as pphs,
    prepass::slicer::world_prepass::host_services as phs,
};

#[test]
fn raycast_miss_returns_none_across_all_worlds() {
    let mesh = mesh_fixture(vec![flat_plate_object("plate", 0.0, identity_transform())]);

    let mut layer_ctx = ctx_with_mesh("mesh-query.layer.miss", mesh.clone());
    let mut prepass_ctx = ctx_with_mesh("mesh-query.prepass.miss", mesh.clone());
    let mut finalization_ctx = ctx_with_mesh("mesh-query.finalization.miss", mesh.clone());
    let mut postpass_ctx = ctx_with_mesh("mesh-query.postpass.miss", mesh.clone());

    let layer_hit = lhs::Host::raycast_z_down(&mut layer_ctx, "plate".to_string(), 5.0, 5.0, -1.0)
        .expect("layer miss query should not error");
    let prepass_hit = phs::Host::raycast_z_down(&mut prepass_ctx, "plate".to_string(), 5.0, 5.0, -1.0)
        .expect("prepass miss query should not error");
    let finalization_hit = fhs::Host::raycast_z_down(&mut finalization_ctx, "plate".to_string(), 5.0, 5.0, -1.0)
        .expect("finalization miss query should not error");
    let postpass_hit = pphs::Host::raycast_z_down(&mut postpass_ctx, "plate".to_string(), 5.0, 5.0, -1.0)
        .expect("postpass miss query should not error");

    assert_eq!(layer_hit, None, "layer miss should return None");
    assert_eq!(prepass_hit, None, "prepass miss should return None");
    assert_eq!(finalization_hit, None, "finalization miss should return None");
    assert_eq!(postpass_hit, None, "postpass miss should return None");
}