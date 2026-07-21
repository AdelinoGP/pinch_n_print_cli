//! TDD: `finalization-output-builder::set-entity-order` semantics.
//!
//! Packet 58_gcode-toolchange-purge-integration, Step 3 implementation.
//!
//! AC8: set-entity-order permutes entities and remaps ToolChange indices.
//! NC6: set-entity-order with malformed proposal is rejected.

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
        dist_to_top_mm: 0.0,
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
        ordered_entities: vec![
            make_entity(1, 0), // index 0
            make_entity(2, 0), // index 1
            make_entity(3, 0), // index 2
        ],
        // ToolChange referencing old index 1 (entity_id=2).
        tool_changes: vec![ToolChange {
            after_entity_index: 1,
            from_tool: 0,
            to_tool: 1,
        }],
        z_hops: vec![ZHop {
            after_entity_index: 0,
            hop_height: 0.3,
        }],
        ..Default::default()
    }
}

// ── AC8 ────────────────────────────────────────────────────────────────────────

/// AC8 — set-entity-order remaps entity indices and ToolChange references.
///
/// Given a layer with 3 entities at indices 0,1,2 (ids 1,2,3), calling
/// set_entity_order with permutation [(2,false), (0,false), (1,false)] must,
/// after apply_to, leave entities in order [original[2], original[0],
/// original[1]] (i.e. [id=3, id=1, id=2]); remap ToolChange.after_entity_index
/// from 1 to 2 (entity originally at index 1 moves to new index 2 via
/// items[2] = (1, false)); and remap ZHop.after_entity_index from 0 to 1
/// (entity originally at index 0 moves to new index 1 via items[1] = (0, false)).
#[test]
fn set_entity_order_remaps_indices() {
    let mut layers = vec![layer_with_3_entities()];
    let orig = layers[0].ordered_entities.clone();

    let mut output = FinalizationOutputBuilder::new();
    // Permutation: new[0]=orig[2], new[1]=orig[0], new[2]=orig[1]
    output
        .set_entity_order(0, vec![(2, false), (0, false), (1, false)])
        .expect("set_entity_order should not fail at record time");

    let result = output.apply_to(&mut layers);
    assert!(result.is_ok(), "apply_to failed: {:?}", result);

    let layer = &layers[0];
    // (a) verify reordering: new[0]=orig[2], new[1]=orig[0], new[2]=orig[1]
    assert_eq!(
        layer.ordered_entities[0].entity_id, orig[2].entity_id,
        "new[0] should be orig[2]"
    );
    assert_eq!(
        layer.ordered_entities[1].entity_id, orig[0].entity_id,
        "new[1] should be orig[0]"
    );
    assert_eq!(
        layer.ordered_entities[2].entity_id, orig[1].entity_id,
        "new[2] should be orig[1]"
    );

    // (b) ToolChange was at index 1 (entity_id=2). That entity moves to new index 2.
    // inverse: orig[0]→new[1], orig[1]→new[2], orig[2]→new[0]
    // so after_entity_index=1 (orig[1]) → new[2]
    assert_eq!(layer.tool_changes.len(), 1);
    assert_eq!(
        layer.tool_changes[0].after_entity_index, 2,
        "ToolChange.after_entity_index should be remapped from 1 to 2"
    );

    // (c) ZHop was at index 0 (entity_id=1). That entity moves to new index 1.
    // orig[0]→new[1], so after_entity_index=0 → new[1]=1
    assert_eq!(layer.z_hops.len(), 1);
    assert_eq!(
        layer.z_hops[0].after_entity_index, 1,
        "ZHop.after_entity_index should be remapped from 0 to 1"
    );
}

// ── NC6 ────────────────────────────────────────────────────────────────────────

/// NC6 — set-entity-order with malformed proposal (duplicate index, missing index) is rejected.
///
/// Given a layer with 3 entities, calling set_entity_order with
/// [(0, false), (0, false), (2, false)] (index 0 duplicated, index 1 missing)
/// must cause apply_to to return Err and leave the layer's entity order
/// unchanged.
#[test]
fn set_entity_order_malformed_rejected() {
    let mut layers = vec![layer_with_3_entities()];
    let original_ids: Vec<u64> = layers[0]
        .ordered_entities
        .iter()
        .map(|e| e.entity_id)
        .collect();

    let mut output = FinalizationOutputBuilder::new();
    // Duplicate index 0, missing index 1 — invalid permutation
    output
        .set_entity_order(0, vec![(0, false), (0, false), (2, false)])
        .expect("set_entity_order should not fail at record time");

    let result = output.apply_to(&mut layers);
    assert!(
        result.is_err(),
        "apply_to should return Err for malformed permutation"
    );

    // Layer is unchanged
    let layer = &layers[0];
    let current_ids: Vec<u64> = layer.ordered_entities.iter().map(|e| e.entity_id).collect();
    assert_eq!(
        current_ids, original_ids,
        "entity order should be unchanged after rejected set_entity_order"
    );
}

// ── Single-permutation invariant (packet 58 locked invariant) ─────────────────

/// Packet-58 locked invariant: `set-entity-order` may be called at most once per
/// `(layer, run_finalization invocation)`. The second call on the same layer
/// must return `Err` at record time and leave the builder's queue unaffected so
/// the first permutation still applies correctly.
#[test]
fn set_entity_order_called_twice_rejected() {
    let mut layers = vec![layer_with_3_entities()];
    let orig = layers[0].ordered_entities.clone();

    let mut output = FinalizationOutputBuilder::new();
    // First call on layer 0: a valid permutation.
    output
        .set_entity_order(0, vec![(2, false), (0, false), (1, false)])
        .expect("first set_entity_order on layer 0 should succeed");

    // Second call on the SAME layer: must be rejected at record time.
    let second = output.set_entity_order(0, vec![(0, false), (1, false), (2, false)]);
    assert!(
        second.is_err(),
        "second set_entity_order on the same layer must return Err"
    );
    let err = second.unwrap_err();
    assert!(
        err.contains("twice") && err.contains("layer 0"),
        "error must name the duplicate-layer condition; got: {err}"
    );

    // The first permutation still applies cleanly — no corruption from the rejected call.
    output
        .apply_to(&mut layers)
        .expect("apply_to should succeed for the single accepted permutation");
    assert_eq!(layers[0].ordered_entities[0].entity_id, orig[2].entity_id);
    assert_eq!(layers[0].ordered_entities[1].entity_id, orig[0].entity_id);
    assert_eq!(layers[0].ordered_entities[2].entity_id, orig[1].entity_id);
}

/// Permuting two distinct layers in one builder is allowed — the invariant is
/// per-layer, not per-builder.
#[test]
fn set_entity_order_on_distinct_layers_allowed() {
    let mut layers = vec![
        layer_with_3_entities(),
        LayerCollectionIR {
            global_layer_index: 1,
            z: 0.4,
            ordered_entities: vec![make_entity(10, 1), make_entity(11, 1)],
            ..Default::default()
        },
    ];

    let mut output = FinalizationOutputBuilder::new();
    output
        .set_entity_order(0, vec![(2, false), (0, false), (1, false)])
        .expect("set_entity_order on layer 0 should succeed");
    output
        .set_entity_order(1, vec![(1, false), (0, false)])
        .expect("set_entity_order on layer 1 (distinct) should also succeed");

    output
        .apply_to(&mut layers)
        .expect("apply_to should accept two distinct-layer permutations");
}
