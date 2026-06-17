//! TDD regression test for packet-115: finalization role recovery.
//!
//! `finalization_role_wit_to_ir` is lossy: when a guest emits
//! `Custom("slicer.builtin/skirt@1")` the committed IR `PrintEntity.role`
//! must be `ExtrusionRole::Skirt`, not `Custom("slicer.builtin/skirt@1")`.
//!
//! These tests exercise the real commit path (no WASM guest needed):
//!   `HostFinalizationOutputBuilder::push_entity_to_layer`
//!     → `finalization_path_wit_to_ir`
//!       → `finalization_role_wit_to_ir`   ← lossy call site (packet-115)
//!
//! Step 1 (RED): tests FAIL until the fix in step 2 repoints the call site to
//! the recovering converter.

#![allow(missing_docs)]

use slicer_wasm_host::host::finalization::slicer::types::geometry as fgeo;
use slicer_wasm_host::host::{
    finalization, FinalizationBuilderPush, HostExecutionContextBuilder,
    BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG, BUILTIN_EXTRUSION_ROLE_SKIRT_TAG,
};

fn make_finalization_path(role_tag: &str) -> fgeo::ExtrusionPath3d {
    fgeo::ExtrusionPath3d {
        points: vec![fgeo::Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        }],
        role: fgeo::ExtrusionRole::Custom(role_tag.to_string()),
        speed_factor: 1.0,
    }
}

/// Drive the finalization commit path directly (no WASM) and assert the
/// committed IR role is recovered to the native variant.
///
/// RED (expected FAIL before packet-115): committed role is
/// `Custom("slicer.builtin/skirt@1")` / `Custom("slicer.builtin/prime-tower@1")`
/// instead of `Skirt` / `PrimeTower`.
#[test]
fn finalization_role_round_trip() {
    let cases: &[(&str, slicer_ir::ExtrusionRole)] = &[
        (
            BUILTIN_EXTRUSION_ROLE_SKIRT_TAG,
            slicer_ir::ExtrusionRole::Skirt,
        ),
        (
            BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG,
            slicer_ir::ExtrusionRole::PrimeTower,
        ),
    ];

    for (tag, expected_ir_role) in cases {
        let mut ctx =
            HostExecutionContextBuilder::new("test.finalization.role.115", 0.0, 0.2).build();

        let builder_handle = ctx
            .push_finalization_output_builder()
            .expect("push_finalization_output_builder must succeed");

        // Record rep before the handle is consumed by the push call.
        let builder_rep = builder_handle.rep();

        // Call the real host impl — routes through finalization_path_wit_to_ir
        // → finalization_role_wit_to_ir (the lossy converter, packet-115).
        let push_result = <slicer_wasm_host::host::HostExecutionContext
            as finalization::HostFinalizationOutputBuilder>::push_entity_to_layer(
            &mut ctx,
            builder_handle,
            0,
            make_finalization_path(tag),
            finalization::RegionKey {
                layer_index: 0,
                object_id: "obj-1".to_string(),
                // Canonical region-id: decimal integer string.
                region_id: "1".to_string(),
            },
        )
        .expect("wasmtime call must not trap");

        assert!(
            push_result.is_ok(),
            "push_entity_to_layer must accept valid path for tag {tag}: {:?}",
            push_result
        );

        // Drop the builder resource explicitly so its pushes are moved onto
        // ctx.finalization_pushes — mimicking the drop that wasmtime issues
        // when the guest releases its handle at the end of run-finalization.
        let drop_handle =
            wasmtime::component::Resource::<finalization::FinalizationOutputBuilder>::new_own(
                builder_rep,
            );
        <slicer_wasm_host::host::HostExecutionContext
            as finalization::HostFinalizationOutputBuilder>::drop(
            &mut ctx,
            drop_handle,
        )
        .expect("builder drop must not trap");

        // Drain: pushes have now been moved from the builder's ResourceTable
        // entry onto ctx.finalization_pushes.
        let pushes = ctx.drain_finalization_output_builder();
        assert_eq!(
            pushes.len(),
            1,
            "expected exactly one committed push for tag {tag}"
        );

        let committed_role = match &pushes[0] {
            FinalizationBuilderPush::EntityToLayer { path, .. } => path.role.clone(),
            other => panic!(
                "expected EntityToLayer push for tag {tag}, got unexpected variant: {other:?}"
            ),
        };

        // This assertion FAILS before packet-115 repoints the call site:
        // committed_role is Custom(tag) because finalization_role_wit_to_ir
        // does not recover PrimeTower/Skirt.
        assert_eq!(
            committed_role, *expected_ir_role,
            "finalization commit path must recover {:?} from Custom tag {:?} — \
             got {:?} (finalization_role_wit_to_ir is lossy; packet-115 not yet fixed)",
            expected_ir_role, tag, committed_role,
        );
    }
}
