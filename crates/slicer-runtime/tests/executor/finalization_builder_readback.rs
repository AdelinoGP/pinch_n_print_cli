//! TDD: `finalization-output-builder::get-ordered-entities` read-back.
//!
//! Packet 58_gcode-toolchange-purge-integration, Step 3 implementation.
//!
//! AC9: get-ordered-entities reflects the currently staged state (pre-existing
//! entities + in-flight pushes + in-flight inserts + in-flight permutations).

#![allow(missing_docs)]

use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth, PrintEntity, RegionKey,
    SemVer,
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

fn layer_with_2_entities() -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![make_entity(1, 0), make_entity(2, 0)],
        ..Default::default()
    }
}

// ── AC9 ────────────────────────────────────────────────────────────────────────

/// AC9 — `get_ordered_entities` reflects staged state including pre-existing,
/// pushed, and inserted entities.
///
/// Setup: layer 0 has 2 pre-existing entities (ids 1, 2). A module pushes 1
/// entity via `push_entity_to_layer`, then inserts 1 entity at position 1 via
/// `insert_entity_at`. Calling `get_ordered_entities(0, &initial)` BEFORE
/// `apply_to` is invoked must return a 4-element vector:
///
/// - Index 0: the original entity at position 0 (id=1).
/// - Index 1: the inserted entity (new synthetic id).
/// - Index 2: the original entity at position 1 (id=2).
/// - Index 3: the pushed entity (appended after the originals, then shifted
///   right by the insert at position 1 — wait: insertion came AFTER the push,
///   so the staged order after push is [orig[0], orig[1], pushed], and
///   inserting at position 1 produces [orig[0], inserted, orig[1], pushed]).
///
/// All 4 entity_id values must be distinct. The originals (1, 2) must still
/// appear. Two new ids must be generated.
#[test]
fn get_ordered_entities_reflects_staged_state() {
    let initial = vec![layer_with_2_entities()];
    let original_ids: std::collections::HashSet<u64> = initial[0]
        .ordered_entities
        .iter()
        .map(|e| e.entity_id)
        .collect();

    let mut output = FinalizationOutputBuilder::new();

    // Sanity: with no operations, the read-back equals the initial layer's entities.
    let pre = output.get_ordered_entities(0, &initial);
    assert_eq!(
        pre.len(),
        2,
        "with no staged operations, read-back should equal initial state"
    );
    assert_eq!(pre[0].entity_id, 1);
    assert_eq!(pre[1].entity_id, 2);

    // Push 1 entity to layer 0 → appended.
    output
        .push_entity_to_layer(0, path(), 1, region_key(0))
        .expect("push_entity_to_layer should succeed");

    let after_push = output.get_ordered_entities(0, &initial);
    assert_eq!(
        after_push.len(),
        3,
        "after one push, staged state should have 3 entities"
    );
    assert!(
        !original_ids.contains(&after_push[2].entity_id),
        "the newly pushed entity at index 2 should carry a fresh id"
    );

    // Insert 1 entity at position 1 → splits the original [1, 2] pair.
    output
        .insert_entity_at(0, 1, path(), 1, region_key(0))
        .expect("insert_entity_at should succeed");

    let staged = output.get_ordered_entities(0, &initial);

    // (a) Total count: 2 original + 1 pushed + 1 inserted = 4.
    assert_eq!(
        staged.len(),
        4,
        "after push + insert, staged state should have 4 entities"
    );

    // (b) Originals must still be present.
    let staged_ids: std::collections::HashSet<u64> = staged.iter().map(|e| e.entity_id).collect();
    for &orig in &original_ids {
        assert!(
            staged_ids.contains(&orig),
            "original entity_id={orig} should still be present"
        );
    }

    // (c) All 4 ids distinct.
    assert_eq!(
        staged_ids.len(),
        4,
        "all 4 staged entity ids must be distinct"
    );

    // (d) The inserted entity sits at index 1 — between original[0] (id=1)
    //     and original[1] (id=2). Verify the insert position semantics.
    assert_eq!(
        staged[0].entity_id, 1,
        "index 0 must still hold the first original entity"
    );
    assert!(
        !original_ids.contains(&staged[1].entity_id),
        "index 1 must hold the inserted (new-id) entity"
    );
    assert_eq!(
        staged[2].entity_id, 2,
        "index 2 must hold the second original entity (shifted right by the insert)"
    );

    // (e) Calling get_ordered_entities does NOT consume or apply the operations.
    //     A subsequent apply_to must still produce 4 entities in the same order.
    let mut layers = initial.clone();
    output
        .apply_to(&mut layers)
        .expect("apply_to must succeed after read-back");
    assert_eq!(
        layers[0].ordered_entities.len(),
        4,
        "apply_to should still produce 4 entities — read-back must not consume ops"
    );
}

/// AC9 — `get_ordered_entities` also reflects a `set_entity_order` permutation.
#[test]
fn get_ordered_entities_reflects_permutation() {
    let initial = vec![LayerCollectionIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![make_entity(1, 0), make_entity(2, 0), make_entity(3, 0)],
        ..Default::default()
    }];

    let mut output = FinalizationOutputBuilder::new();
    output
        .set_entity_order(0, vec![(2, false), (0, false), (1, false)])
        .expect("set_entity_order should succeed");

    let staged = output.get_ordered_entities(0, &initial);
    assert_eq!(staged.len(), 3);
    assert_eq!(staged[0].entity_id, 3, "new[0] should be original[2]");
    assert_eq!(staged[1].entity_id, 1, "new[1] should be original[0]");
    assert_eq!(staged[2].entity_id, 2, "new[2] should be original[1]");
}
