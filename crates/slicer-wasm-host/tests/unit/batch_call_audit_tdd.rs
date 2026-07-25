//! TDD coverage for how batched mesh queries are recorded for the access audit.
//!
//! The contract (ADR-0049): a batch records **one** `runtime_reads` entry and
//! **one** `batch_calls` entry carrying the batch size — never N of either. The
//! read *set* a module declares is therefore unchanged by adopting a batch,
//! while the read *volume* stays visible through `batch_calls`.
//!
//! Verification: `cargo test -p slicer-wasm-host --test unit -- batch_call_audit`

#![allow(missing_docs)]

use crate::common::{ctx_with_mesh, flat_plate_object, identity_transform, mesh_fixture};
use slicer_wasm_host::host::prepass::slicer::common::host_services as phs;

fn raycast_request(x: f32, y: f32) -> phs::RaycastRequest {
    phs::RaycastRequest {
        object_id: "plate".to_string(),
        x,
        y,
        start_z: 10.0,
    }
}

/// A 4-ray batch must leave exactly one `"MeshIR"` read behind, plus one
/// `("MeshIR", 4)` batch record — not four reads, and not four batch records.
#[test]
fn raycast_batch_records_one_read_and_one_sized_batch_entry() {
    let mesh = mesh_fixture(vec![flat_plate_object("plate", 0.0, identity_transform())]);
    let mut ctx = ctx_with_mesh("batch-audit.raycast", mesh);

    let results = phs::Host::raycast_z_down_batch(
        &mut ctx,
        vec![
            raycast_request(1.0, 1.0),
            raycast_request(2.0, 2.0),
            raycast_request(3.0, 3.0),
            raycast_request(4.0, 4.0),
        ],
    )
    .expect("batched raycast should not error");

    // The batch really ran: every ray hit the plate at z = 0.
    assert_eq!(results.len(), 4, "one result per request, in input order");
    assert!(
        results.iter().all(|hit| hit.is_some()),
        "all four rays should hit the flat plate, got {results:?}"
    );

    assert_eq!(
        ctx.runtime_reads(),
        ["MeshIR".to_string()],
        "a 4-item batch must record exactly one read, not one per item"
    );
    assert_eq!(
        ctx.batch_calls(),
        [("MeshIR".to_string(), 4u32)],
        "the batch must be recorded once, carrying its size"
    );
}

/// The singular form is the contrast case that gives the assertion above its
/// meaning: four separate calls produce four reads and no batch records at all.
#[test]
fn singular_raycast_records_one_read_per_call_and_no_batch_entry() {
    let mesh = mesh_fixture(vec![flat_plate_object("plate", 0.0, identity_transform())]);
    let mut ctx = ctx_with_mesh("batch-audit.singular", mesh);

    for i in 0..4 {
        phs::Host::raycast_z_down(&mut ctx, "plate".to_string(), i as f32, i as f32, 10.0)
            .expect("singular raycast should not error");
    }

    assert_eq!(
        ctx.runtime_reads().len(),
        4,
        "four singular calls record four reads"
    );
    assert!(
        ctx.batch_calls().is_empty(),
        "singular calls record no batch entry, got {:?}",
        ctx.batch_calls()
    );
}

/// `surface-normal-at-batch` is the second batched mesh query and follows the
/// same recording rule.
#[test]
fn surface_normal_batch_records_one_read_and_one_sized_batch_entry() {
    let mesh = mesh_fixture(vec![flat_plate_object("plate", 0.0, identity_transform())]);
    let mut ctx = ctx_with_mesh("batch-audit.normal", mesh);

    let requests: Vec<phs::SurfaceNormalRequest> = (0..3)
        .map(|i| phs::SurfaceNormalRequest {
            object_id: "plate".to_string(),
            x: 1.0 + i as f32,
            y: 1.0 + i as f32,
            z: 0.0,
        })
        .collect();

    let results = phs::Host::surface_normal_at_batch(&mut ctx, requests)
        .expect("batched surface-normal should not error");

    assert_eq!(results.len(), 3, "one result per request, in input order");
    assert_eq!(
        ctx.runtime_reads(),
        ["MeshIR".to_string()],
        "a 3-item batch must record exactly one read"
    );
    assert_eq!(
        ctx.batch_calls(),
        [("MeshIR".to_string(), 3u32)],
        "the batch must be recorded once, carrying its size"
    );
}

/// Two batches on one context accumulate as two entries, so a module that
/// batches twice is distinguishable from one that batches once at double size.
#[test]
fn successive_batches_accumulate_as_separate_entries() {
    let mesh = mesh_fixture(vec![flat_plate_object("plate", 0.0, identity_transform())]);
    let mut ctx = ctx_with_mesh("batch-audit.successive", mesh);

    phs::Host::raycast_z_down_batch(&mut ctx, vec![raycast_request(1.0, 1.0)])
        .expect("first batch should not error");
    phs::Host::raycast_z_down_batch(
        &mut ctx,
        vec![raycast_request(2.0, 2.0), raycast_request(3.0, 3.0)],
    )
    .expect("second batch should not error");

    assert_eq!(
        ctx.batch_calls(),
        [("MeshIR".to_string(), 1u32), ("MeshIR".to_string(), 2u32)],
        "each batch is its own entry, in call order"
    );
}
