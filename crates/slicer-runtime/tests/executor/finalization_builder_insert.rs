//! TDD: `finalization-output-builder::insert-entity-at` semantics.
//!
//! Packet 58_gcode-toolchange-purge-integration, Step 3 implementation.
//!
//! AC7: insert-entity-at positional insert + index remap.
//! NC5: insert-entity-at out-of-bounds position is rejected.

#![allow(missing_docs)]

use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth, PrintEntity, RegionKey,
    SemVer, ToolChange, ZHop,
};
use slicer_sdk::traits::FinalizationOutputBuilder;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn pt() -> Point3WithWidth {
    Point3WithWidth {
        x: 0.0,
        y: 0.0,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

fn path() -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![pt()],
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    }
}

fn region_key(layer: u32) -> RegionKey {
    RegionKey {
        global_layer_index: layer,
        object_id: "obj".to_string(),
        region_id: 1,
        variant_chain: Vec::new(),
    }
}

fn make_entity(entity_id: u64, layer: u32) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: path(),
        role: ExtrusionRole::OuterWall,
        tool_index: 1,
        region_key: region_key(layer),
        topo_order: (entity_id - 1) as u32,
    }
}

fn layer_with_3_entities() -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![make_entity(1, 0), make_entity(2, 0), make_entity(3, 0)],
        tool_changes: vec![
            ToolChange {
                after_entity_index: 1,
                from_tool: 0,
                to_tool: 1,
            },
            ToolChange {
                after_entity_index: 2,
                from_tool: 1,
                to_tool: 0,
            },
        ],
        z_hops: vec![ZHop {
            after_entity_index: 2,
            hop_height: 0.5,
        }],
        ..Default::default()
    }
}

// ── AC7 ────────────────────────────────────────────────────────────────────────

/// AC7 — insert-entity-at semantics (positional insert + index remap).
///
/// Given a layer with 3 entities (ids 1,2,3) and tool changes at
/// after_entity_index=1 and after_entity_index=2, inserting at position=2 must
/// leave the new entity at index 2; shift original entity[2] (id=3) to index 3;
/// keep ToolChange at after_entity_index=1 unchanged (1 < 2); bump ToolChange
/// at after_entity_index=2 to 3 (2 >= position=2); and bump ZHop at
/// after_entity_index=2 to 3.
#[test]
fn insert_at_position_remaps_indices() {
    let mut layers = vec![layer_with_3_entities()];
    let mut output = FinalizationOutputBuilder::new();

    output
        .insert_entity_at(0, 2, path(), 1, region_key(0))
        .expect("insert_entity_at should not fail at record time");

    let result = output.apply_to(&mut layers);
    assert!(result.is_ok(), "apply_to failed: {:?}", result);

    let layer = &layers[0];
    // (a) now has 4 entities
    assert_eq!(
        layer.ordered_entities.len(),
        4,
        "should have 4 entities after insert"
    );

    // (b) the new entity is at index 2; the original entity at index 2 (id=3) is now at 3
    assert_eq!(
        layer.ordered_entities[3].entity_id, 3,
        "original entity[2] (id=3) should now be at index 3"
    );

    // ToolChange at after_entity_index=1 unchanged (1 < position=2)
    let tc_unchanged = layer
        .tool_changes
        .iter()
        .find(|tc| tc.from_tool == 0 && tc.to_tool == 1)
        .expect("tool change 0->1 should still exist");
    assert_eq!(
        tc_unchanged.after_entity_index, 1,
        "ToolChange at 1 should be unchanged"
    );

    // ToolChange at original after_entity_index=2 should now be 3 (2 >= position=2)
    let tc_remapped = layer
        .tool_changes
        .iter()
        .find(|tc| tc.from_tool == 1 && tc.to_tool == 0)
        .expect("tool change 1->0 should still exist");
    assert_eq!(
        tc_remapped.after_entity_index, 3,
        "ToolChange at 2 should increment to 3"
    );

    // ZHop at original after_entity_index=2 should now be 3
    assert_eq!(layer.z_hops.len(), 1);
    assert_eq!(
        layer.z_hops[0].after_entity_index, 3,
        "ZHop at 2 should increment to 3"
    );
}

// ── NC5 ────────────────────────────────────────────────────────────────────────

/// NC5 — insert-entity-at out-of-bounds position is rejected.
///
/// Given a layer with 3 entities, inserting at position=99 must cause
/// apply_to to return Err containing "position 99 out of bounds", leave the
/// layer's entity count unchanged at 3, and leave the entities intact (no
/// partial mutation).
#[test]
fn insert_at_oob_position_rejected() {
    let mut layers = vec![layer_with_3_entities()];
    let original_entity_0_id = layers[0].ordered_entities[0].entity_id;

    let mut output = FinalizationOutputBuilder::new();
    output
        .insert_entity_at(0, 99, path(), 1, region_key(0))
        .expect("insert_entity_at should not fail at record time");

    let result = output.apply_to(&mut layers);
    assert!(
        result.is_err(),
        "apply_to should return Err for OOB position"
    );

    let err = result.unwrap_err();
    assert!(
        err.contains("position 99 out of bounds"),
        "error message should mention 'position 99 out of bounds', got: {err}"
    );

    // Layer is unchanged
    assert_eq!(
        layers[0].ordered_entities.len(),
        3,
        "layer should still have 3 entities"
    );
    assert_eq!(
        layers[0].ordered_entities[0].entity_id, original_entity_0_id,
        "entity[0] should be unchanged"
    );
}
