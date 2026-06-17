//! TDD coverage for off-surface `surface_normal_at` semantics across every host-service world.
//!
//! Verification: `cargo test -p slicer-runtime --test surface_normal_at_oob_tdd -- --nocapture`

#![allow(missing_docs)]

use crate::common::{ctx_with_mesh, identity_transform, mesh_fixture, sloped_triangle_object};
use slicer_wasm_host::host::{
    finalization::slicer::common::host_services as fhs,
    layer::slicer::common::host_services as lhs, postpass::slicer::common::host_services as pphs,
    prepass::slicer::common::host_services as phs,
};

#[test]
fn surface_normal_outside_surface_returns_none_across_all_worlds() {
    let mesh = mesh_fixture(vec![sloped_triangle_object("slope", identity_transform())]);

    let mut layer_ctx = ctx_with_mesh("mesh-query.layer.oob", mesh.clone());
    let mut prepass_ctx = ctx_with_mesh("mesh-query.prepass.oob", mesh.clone());
    let mut finalization_ctx = ctx_with_mesh("mesh-query.finalization.oob", mesh.clone());
    let mut postpass_ctx = ctx_with_mesh("mesh-query.postpass.oob", mesh.clone());

    let layer_normal =
        lhs::Host::surface_normal_at(&mut layer_ctx, "slope".to_string(), 30.0, 30.0, 30.0)
            .expect("layer off-surface query should not error");
    let prepass_normal =
        phs::Host::surface_normal_at(&mut prepass_ctx, "slope".to_string(), 30.0, 30.0, 30.0)
            .expect("prepass off-surface query should not error");
    let finalization_normal =
        fhs::Host::surface_normal_at(&mut finalization_ctx, "slope".to_string(), 30.0, 30.0, 30.0)
            .expect("finalization off-surface query should not error");
    let postpass_normal =
        pphs::Host::surface_normal_at(&mut postpass_ctx, "slope".to_string(), 30.0, 30.0, 30.0)
            .expect("postpass off-surface query should not error");

    assert!(
        layer_normal.is_none(),
        "layer off-surface query should return None"
    );
    assert!(
        prepass_normal.is_none(),
        "prepass off-surface query should return None"
    );
    assert!(
        finalization_normal.is_none(),
        "finalization off-surface query should return None"
    );
    assert!(
        postpass_normal.is_none(),
        "postpass off-surface query should return None"
    );
}
