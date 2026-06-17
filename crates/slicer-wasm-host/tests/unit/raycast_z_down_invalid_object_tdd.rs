//! TDD coverage for invalid-object `raycast_z_down` diagnostics across every host-service world.
//!
//! Verification: `cargo test -p slicer-runtime --test raycast_z_down_invalid_object_tdd -- --nocapture`

#![allow(missing_docs)]

use crate::common::{ctx_with_mesh, flat_plate_object, identity_transform, mesh_fixture};
use slicer_wasm_host::host::{
    finalization::slicer::common::host_services as fhs,
    layer::slicer::common::host_services as lhs, postpass::slicer::common::host_services as pphs,
    prepass::slicer::common::host_services as phs,
};

fn assert_object_not_found(message: &str, label: &str) {
    assert!(
        message.contains("OBJECT_NOT_FOUND"),
        "{label} should mention OBJECT_NOT_FOUND: {message}"
    );
    assert!(
        message.contains("missing-object"),
        "{label} should name the missing object: {message}"
    );
}

#[test]
fn raycast_invalid_object_returns_object_not_found_across_all_worlds() {
    let mesh = mesh_fixture(vec![flat_plate_object("plate", 0.0, identity_transform())]);

    let mut layer_ctx = ctx_with_mesh("mesh-query.layer.invalid", mesh.clone());
    let mut prepass_ctx = ctx_with_mesh("mesh-query.prepass.invalid", mesh.clone());
    let mut finalization_ctx = ctx_with_mesh("mesh-query.finalization.invalid", mesh.clone());
    let mut postpass_ctx = ctx_with_mesh("mesh-query.postpass.invalid", mesh.clone());

    let layer_error =
        lhs::Host::raycast_z_down(&mut layer_ctx, "missing-object".to_string(), 5.0, 5.0, 10.0)
            .expect_err("layer invalid-object raycast should error")
            .to_string();
    let prepass_error = phs::Host::raycast_z_down(
        &mut prepass_ctx,
        "missing-object".to_string(),
        5.0,
        5.0,
        10.0,
    )
    .expect_err("prepass invalid-object raycast should error")
    .to_string();
    let finalization_error = fhs::Host::raycast_z_down(
        &mut finalization_ctx,
        "missing-object".to_string(),
        5.0,
        5.0,
        10.0,
    )
    .expect_err("finalization invalid-object raycast should error")
    .to_string();
    let postpass_error = pphs::Host::raycast_z_down(
        &mut postpass_ctx,
        "missing-object".to_string(),
        5.0,
        5.0,
        10.0,
    )
    .expect_err("postpass invalid-object raycast should error")
    .to_string();

    assert_object_not_found(&layer_error, "layer error");
    assert_object_not_found(&prepass_error, "prepass error");
    assert_object_not_found(&finalization_error, "finalization error");
    assert_object_not_found(&postpass_error, "postpass error");
}
