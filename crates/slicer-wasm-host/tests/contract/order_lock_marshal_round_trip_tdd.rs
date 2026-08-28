//! Contract: a guest-authored `order-lock` tag SURVIVES the host marshal.
//!
//! Packet 244 added `ExtrusionPath3D::order_lock` and the matching WIT record
//! field; packet 245 built the linker / optimizer / emitter consumers on top of
//! it. Both were shipped with `convert_extrusion_path` (`marshal/leaf.rs`)
//! hardcoding `order_lock: None`, so every infill path lost its tag at the
//! leaf marshal and the whole feature chain was inert end-to-end. The same
//! class of drop existed in `finalization_path_wit_to_ir` (`host.rs`).
//!
//! These tests drive the real conversion functions (no WASM guest needed):
//!   `convert_infill_output` → `convert_extrusion_path`            ← leaf drop
//!   `push_entity_to_layer`  → `finalization_path_wit_to_ir`       ← same class
//!
//! RED against the pre-fix code: committed `order_lock` is `None` for the
//! tagged paths.

#![allow(missing_docs)]

use slicer_wasm_host::host::finalization_types as fm;
use slicer_wasm_host::host::{
    ExtrusionPath3d, ExtrusionRole, FinalizationBuilderPush, HostExecutionContextBuilder,
    Point3WithWidth,
};
use slicer_wasm_host::marshal::accumulators::InfillOutputCollected;
use slicer_wasm_host::marshal::{convert_infill_output, OriginId};

const OBJECT_ID: &str = "cube";
const REGION_ID: u64 = 3;

/// A global-space order-lock tag, as the host allocator mints them.
const LOCK_TAG: u64 = (1 << 63) | 42;

fn point(x: f32) -> Point3WithWidth {
    // exhaustive: Point3WithWidth has no Default; fixture pins all geometry fields
    Point3WithWidth {
        x,
        y: 0.0,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
        overhang_distance_mm: None,
    }
}

/// One two-point path carrying `order_lock`, distinguished by its x offset so
/// committed paths can be told apart independently of ordering.
fn path_with_lock(x0: f32, order_lock: Option<u64>) -> ExtrusionPath3d {
    // exhaustive: ExtrusionPath3d has no Default; fixture pins all path fields
    ExtrusionPath3d {
        points: vec![point(x0), point(x0 + 10.0)],
        role: ExtrusionRole::TopSolidInfill,
        speed_factor: 1.0,
        tool_index: None,
        order_lock,
    }
}

fn origin() -> Option<OriginId> {
    Some(OriginId {
        object_id: OBJECT_ID.to_string(),
        region_id: REGION_ID,
    })
}

/// A guest-authored order-lock tag on an infill path must round-trip EXACTLY
/// through `convert_infill_output`: `Some(tag)` stays `Some(tag)`, `None`
/// stays `None`.
///
/// RED before the fix: `convert_extrusion_path` hardcodes `order_lock: None`,
/// so the first path's tag is dropped and no infill module can ever deliver a
/// lock to the arena.
#[test]
fn order_lock_survives_infill_marshal() {
    let collected = InfillOutputCollected {
        solid_paths: vec![
            path_with_lock(0.0, Some(LOCK_TAG)),
            path_with_lock(100.0, None),
            path_with_lock(200.0, Some(LOCK_TAG)),
        ],
        solid_path_origins: vec![origin(), origin(), origin()],
        ..Default::default()
    };

    let ir = convert_infill_output(&collected, 0, None).expect("marshal must commit");

    assert_eq!(ir.regions.len(), 1, "expected exactly one committed region");
    let solid = &ir.regions[0].solid_infill;
    assert_eq!(solid.len(), 3, "expected all three solid paths committed");

    let observed: Vec<Option<u64>> = solid.iter().map(|p| p.order_lock).collect();
    assert_eq!(
        observed,
        vec![Some(LOCK_TAG), None, Some(LOCK_TAG)],
        "order-lock tags must survive convert_infill_output verbatim \
         (convert_extrusion_path must carry path.order_lock, not hardcode None)"
    );

    // The tags must be carried, not re-minted: the exact value matters because
    // the host arena keys contiguous locked blocks by tag equality.
    assert_eq!(
        solid[0].order_lock, solid[2].order_lock,
        "two paths sharing one authored tag must still share it after marshal"
    );
}

/// Same class, finalization direction: a finalization module that authors an
/// order-locked entity must have that tag committed by
/// `finalization_path_wit_to_ir`, not silently dropped.
#[test]
fn order_lock_survives_finalization_marshal() {
    for expected in [Some(LOCK_TAG), None] {
        let mut ctx =
            HostExecutionContextBuilder::new("test.order-lock.marshal".to_string(), 0.0, 0.2)
                .build();

        let builder_handle = ctx
            .push_finalization_output_builder()
            .expect("push_finalization_output_builder must succeed");
        let builder_rep = builder_handle.rep();

        // The finalization world's `ExtrusionPath3d` is the shared layer-world
        // record (bindgen `with:` remap), so the fixture is reused verbatim.
        let fin_path = path_with_lock(0.0, expected);

        let push_result = <slicer_wasm_host::host::HostExecutionContext
            as fm::HostFinalizationOutputBuilder>::push_entity_to_layer(
            &mut ctx,
            builder_handle,
            0,
            fin_path,
            1,
            fm::RegionKey {
                layer_index: 0,
                object_id: OBJECT_ID.to_string(),
                region_id: "3".to_string(),
            },
        )
        .expect("wasmtime call must not trap");
        assert!(
            push_result.is_ok(),
            "push_entity_to_layer must accept the fixture path: {push_result:?}"
        );

        let drop_handle =
            wasmtime::component::Resource::<fm::FinalizationOutputBuilder>::new_own(builder_rep);
        <slicer_wasm_host::host::HostExecutionContext as fm::HostFinalizationOutputBuilder>::drop(
            &mut ctx,
            drop_handle,
        )
        .expect("builder drop must not trap");

        let pushes = ctx.drain_finalization_output_builder();
        assert_eq!(pushes.len(), 1, "expected exactly one committed push");
        let committed = match &pushes[0] {
            FinalizationBuilderPush::EntityToLayer { path, .. } => path.order_lock,
            other => panic!("expected EntityToLayer push, got: {other:?}"),
        };

        assert_eq!(
            committed, expected,
            "finalization commit path must carry order_lock verbatim \
             (finalization_path_wit_to_ir must not hardcode None)"
        );
    }
}
