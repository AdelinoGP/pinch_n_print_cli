//! TDD coverage for `surface_normal_at` unit-length semantics across every host-service world.
//!
//! Verification: `cargo test -p slicer-host --test surface_normal_at_unit_length_tdd -- --nocapture`

#![allow(missing_docs)]

mod common;

use common::{
    assert_perpendicular, assert_unit_length, ctx_with_mesh, identity_transform, mesh_fixture,
    sloped_triangle_object,
};
use slicer_host::wit_host::{
    finalization::slicer::world_finalization::host_services as fhs,
    layer::slicer::world_layer::host_services as lhs,
    postpass::slicer::world_postpass::host_services as pphs,
    prepass::slicer::world_prepass::host_services as phs,
};

#[test]
fn surface_normal_returns_unit_length_normal_for_on_surface_point_across_all_worlds() {
    let mesh = mesh_fixture(vec![sloped_triangle_object("slope", identity_transform())]);
    let edge1 = [10.0, 0.0, 0.0];
    let edge2 = [0.0, 10.0, 10.0];

    let mut layer_ctx = ctx_with_mesh("mesh-query.layer.normal", mesh.clone());
    let mut prepass_ctx = ctx_with_mesh("mesh-query.prepass.normal", mesh.clone());
    let mut finalization_ctx = ctx_with_mesh("mesh-query.finalization.normal", mesh.clone());
    let mut postpass_ctx = ctx_with_mesh("mesh-query.postpass.normal", mesh.clone());

    let layer_normal =
        lhs::Host::surface_normal_at(&mut layer_ctx, "slope".to_string(), 2.0, 2.0, 2.0)
            .expect("layer surface_normal_at should not error")
            .expect("layer surface_normal_at should find a normal");
    let prepass_normal =
        phs::Host::surface_normal_at(&mut prepass_ctx, "slope".to_string(), 2.0, 2.0, 2.0)
            .expect("prepass surface_normal_at should not error")
            .expect("prepass surface_normal_at should find a normal");
    let finalization_normal =
        fhs::Host::surface_normal_at(&mut finalization_ctx, "slope".to_string(), 2.0, 2.0, 2.0)
            .expect("finalization surface_normal_at should not error")
            .expect("finalization surface_normal_at should find a normal");
    let postpass_normal =
        pphs::Host::surface_normal_at(&mut postpass_ctx, "slope".to_string(), 2.0, 2.0, 2.0)
            .expect("postpass surface_normal_at should not error")
            .expect("postpass surface_normal_at should find a normal");

    assert_unit_length(
        layer_normal.x,
        layer_normal.y,
        layer_normal.z,
        "layer magnitude",
    );
    assert_perpendicular(
        layer_normal.x,
        layer_normal.y,
        layer_normal.z,
        edge1,
        edge2,
        "layer normal",
    );

    assert_unit_length(
        prepass_normal.x,
        prepass_normal.y,
        prepass_normal.z,
        "prepass magnitude",
    );
    assert_perpendicular(
        prepass_normal.x,
        prepass_normal.y,
        prepass_normal.z,
        edge1,
        edge2,
        "prepass normal",
    );

    assert_unit_length(
        finalization_normal.x,
        finalization_normal.y,
        finalization_normal.z,
        "finalization magnitude",
    );
    assert_perpendicular(
        finalization_normal.x,
        finalization_normal.y,
        finalization_normal.z,
        edge1,
        edge2,
        "finalization normal",
    );

    assert_unit_length(
        postpass_normal.x,
        postpass_normal.y,
        postpass_normal.z,
        "postpass magnitude",
    );
    assert_perpendicular(
        postpass_normal.x,
        postpass_normal.y,
        postpass_normal.z,
        edge1,
        edge2,
        "postpass normal",
    );
}
