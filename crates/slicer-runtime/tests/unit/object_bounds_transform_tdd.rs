//! TDD coverage for `object_bounds` transform-aware semantics across every host-service world.
//!
//! Verification: `cargo test -p slicer-runtime --test object_bounds_transform_tdd -- --nocapture`

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
fn object_bounds_returns_world_transform_bounds_across_all_worlds() {
    let mesh = mesh_fixture(vec![flat_plate_object(
        "plate",
        0.0,
        translation_transform(5.0, 7.0, 10.0),
    )]);

    let mut layer_ctx = ctx_with_mesh("mesh-query.layer.bounds", mesh.clone());
    let mut prepass_ctx = ctx_with_mesh("mesh-query.prepass.bounds", mesh.clone());
    let mut finalization_ctx = ctx_with_mesh("mesh-query.finalization.bounds", mesh.clone());
    let mut postpass_ctx = ctx_with_mesh("mesh-query.postpass.bounds", mesh.clone());

    let layer_bounds = lhs::Host::object_bounds(&mut layer_ctx, "plate".to_string())
        .expect("layer object_bounds should not error");
    let prepass_bounds = phs::Host::object_bounds(&mut prepass_ctx, "plate".to_string())
        .expect("prepass object_bounds should not error");
    let finalization_bounds = fhs::Host::object_bounds(&mut finalization_ctx, "plate".to_string())
        .expect("finalization object_bounds should not error");
    let postpass_bounds = pphs::Host::object_bounds(&mut postpass_ctx, "plate".to_string())
        .expect("postpass object_bounds should not error");

    for (label, min_x, min_y, min_z, max_x, max_y, max_z) in [
        (
            "layer",
            layer_bounds.min.x,
            layer_bounds.min.y,
            layer_bounds.min.z,
            layer_bounds.max.x,
            layer_bounds.max.y,
            layer_bounds.max.z,
        ),
        (
            "prepass",
            prepass_bounds.min.x,
            prepass_bounds.min.y,
            prepass_bounds.min.z,
            prepass_bounds.max.x,
            prepass_bounds.max.y,
            prepass_bounds.max.z,
        ),
        (
            "finalization",
            finalization_bounds.min.x,
            finalization_bounds.min.y,
            finalization_bounds.min.z,
            finalization_bounds.max.x,
            finalization_bounds.max.y,
            finalization_bounds.max.z,
        ),
        (
            "postpass",
            postpass_bounds.min.x,
            postpass_bounds.min.y,
            postpass_bounds.min.z,
            postpass_bounds.max.x,
            postpass_bounds.max.y,
            postpass_bounds.max.z,
        ),
    ] {
        assert_close(min_x, 5.0, &format!("{label} min.x"));
        assert_close(min_y, 7.0, &format!("{label} min.y"));
        assert_close(min_z, 10.0, &format!("{label} min.z"));
        assert_close(max_x, 15.0, &format!("{label} max.x"));
        assert_close(max_y, 17.0, &format!("{label} max.y"));
        assert_close(max_z, 10.0, &format!("{label} max.z"));
    }
}
