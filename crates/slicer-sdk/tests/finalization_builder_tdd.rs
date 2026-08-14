//! TDD tests for the new FinalizationOutputBuilder methods (Packet 40 / 41).
//!
//! Packet 41 migrates the three closure-typed methods to enum-typed forms:
//!   - modify_entity(layer, entity_id, EntityMutation)
//!   - sort_layer_by(layer, SortKey)
//!   - insert_synthetic_layer_after(idx, SyntheticLayerData)
//!
//! Compile-fail with "cannot find type `EntityMutation`" (and similar) is the
//! EXPECTED state for Step 1A.  Steps 2–3 define the types.
//! exit_condition_met = true.

use slicer_ir::{LayerCollectionIR, PrintEntity, TravelMove};
use slicer_sdk::prelude::*;
use slicer_sdk::test_support::fixtures::print_entity_base;
use slicer_sdk::{EntityMutation, SortKey, SyntheticLayerData};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;

// =============================================================================
// Fixture helpers
// =============================================================================

/// Build a dummy `ExtrusionPath3D` for a given role.
fn make_path(role: ExtrusionRole) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![],
        ..extrusion_path3d_base(role)
    }
}

/// Build a dummy `ExtrusionPath3D` with per-point flow_factors set.
fn make_path_with_flow(role: ExtrusionRole, flow_factor: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            slicer_ir::Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor,
                ..Default::default()
            },
            slicer_ir::Point3WithWidth {
                x: 1.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor,
                ..Default::default()
            },
        ],
        ..extrusion_path3d_base(role)
    }
}

/// Build a dummy `RegionKey`.
fn make_region_key() -> RegionKey {
    RegionKey {
        global_layer_index: 0,
        object_id: "obj-1".to_string(),
        region_id: 0,
        variant_chain: Vec::new(),
    }
}

/// Build a `PrintEntity` with a specific `entity_id` and role.
fn make_entity(entity_id: u64, role: ExtrusionRole) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: make_path(role.clone()),
        region_key: make_region_key(),
        ..print_entity_base(role)
    }
}

/// Build a `LayerCollectionIR` pre-seeded with the given entities.
fn make_layer(global_layer_index: u32, z: f32, entities: Vec<PrintEntity>) -> LayerCollectionIR {
    LayerCollectionIR {
        global_layer_index,
        z,
        ordered_entities: entities,
        ..Default::default()
    }
}

/// Build a Vec of N simple layers (no entities).
fn make_layers(n: u32) -> Vec<LayerCollectionIR> {
    (0..n)
        .map(|i| make_layer(i, i as f32 * 0.2, vec![]))
        .collect()
}

// =============================================================================
// AC-1: push_with_priority_lands_at_sorted_position
// =============================================================================

/// Given a layer with entities [OuterWall(id=1), SparseInfill(id=2), TopSolidInfill(id=3)]
/// (producer-emit order), when push_entity_with_priority is called for Ironing at
/// ExtrusionRole::Ironing.default_priority(), and apply_to runs, the post-merge
/// ordered_entities role order is [OuterWall, SparseInfill, TopSolidInfill, Ironing].
#[test]
fn push_with_priority_lands_at_sorted_position() {
    let entities = vec![
        make_entity(1, ExtrusionRole::OuterWall),
        make_entity(2, ExtrusionRole::SparseInfill),
        make_entity(3, ExtrusionRole::TopSolidInfill),
    ];
    let mut layers = vec![make_layer(0, 0.2, entities)];

    let mut builder = FinalizationOutputBuilder::new();
    let ironing_path = make_path(ExtrusionRole::Ironing);
    let priority = ExtrusionRole::Ironing.default_priority();
    builder
        .push_entity_with_priority(0, ironing_path, 0, make_region_key(), priority)
        .expect("push_entity_with_priority should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    let roles: Vec<&ExtrusionRole> = layers[0].ordered_entities.iter().map(|e| &e.role).collect();
    assert_eq!(roles.len(), 4);
    assert_eq!(roles[0], &ExtrusionRole::OuterWall);
    assert_eq!(roles[1], &ExtrusionRole::SparseInfill);
    assert_eq!(roles[2], &ExtrusionRole::TopSolidInfill);
    assert_eq!(roles[3], &ExtrusionRole::Ironing);
}

// =============================================================================
// AC-2: modify_entity_by_id_applies (migrated from closure to enum form)
// =============================================================================

/// Given a layer with three entities (ids 1, 2, 3), entity_id==2 has speed_factor==1.0.
/// After modify_entity(layer=0, entity_id=2, EntityMutation::SetSpeedFactor(0.5)) and
/// apply_to, entity_id==2 has speed_factor==0.5 and others unchanged at 1.0.
#[test]
fn modify_entity_by_id_applies() {
    let entities = vec![
        make_entity(1, ExtrusionRole::OuterWall),
        make_entity(2, ExtrusionRole::InnerWall),
        make_entity(3, ExtrusionRole::SparseInfill),
    ];
    let mut layers = vec![make_layer(0, 0.2, entities)];

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .modify_entity(0, 2, EntityMutation::SetSpeedFactor(0.5))
        .expect("modify_entity should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    for entity in &layers[0].ordered_entities {
        if entity.entity_id == 2 {
            assert!(
                (entity.path.speed_factor - 0.5).abs() < 1e-6,
                "entity_id=2 should have speed_factor 0.5, got {}",
                entity.path.speed_factor
            );
        } else {
            assert!(
                (entity.path.speed_factor - 1.0).abs() < 1e-6,
                "entity_id={} should have speed_factor 1.0 (unchanged), got {}",
                entity.entity_id,
                entity.path.speed_factor
            );
        }
    }
}

// =============================================================================
// AC-3: sort_layer_by_priority_and_entity_id (migrated from closure to enum form)
// =============================================================================

/// With 5 entities at varying roles, after sort_layer_by(SortKey::ByPriorityAndEntityId)
/// and apply_to, the layer is sorted ascending by (role.default_priority(), entity_id).
/// Travel-anchor regression check: TravelMoves are populated with entity_id anchors that
/// include both forward references (entity later in pre-sort order) and backward references
/// (entity earlier in pre-sort order), so the assertion is non-trivial.
#[test]
fn sort_layer_by_priority_and_entity_id() {
    // Intentionally out of sorted order to make the sort observable.
    // Pre-sort order: [id=5/SparseInfill, id=1/OuterWall, id=3/Ironing, id=2/InnerWall, id=4/TopSolidInfill]
    let entities = vec![
        make_entity(5, ExtrusionRole::SparseInfill),
        make_entity(1, ExtrusionRole::OuterWall),
        make_entity(3, ExtrusionRole::Ironing),
        make_entity(2, ExtrusionRole::InnerWall),
        make_entity(4, ExtrusionRole::TopSolidInfill),
    ];

    // Populate travel_moves with anchors that mix forward and backward references
    // relative to the pre-sort entity order:
    //   TravelMove[0]: entity_id=1 (OuterWall) — forward ref (id=1 is after id=5 in pre-sort? No,
    //                  id=5 is first; id=1 is second → backward from id=3/id=4 perspective)
    //   TravelMove[1]: entity_id=4 (TopSolidInfill) — forward ref from entities earlier in pre-sort
    //   TravelMove[2]: entity_id=2 (InnerWall) — backward ref (appears at pre-sort index 3,
    //                  so it is a backward ref from id=3 and forward from id=5)
    // Using entity_ids that exist in the layer; mixed so both orphan directions are caught.
    let travel_moves = vec![
        // exhaustive: anchor test pins every travel-move transport field
        TravelMove {
            entity_id: 1,
            x: Some(100.0),
            y: None,
            z: None,
            f: None,
        },
        // exhaustive: anchor test pins every travel-move transport field
        TravelMove {
            entity_id: 4,
            x: None,
            y: Some(200.0),
            z: None,
            f: None,
        },
        // exhaustive: anchor test pins every travel-move transport field
        TravelMove {
            entity_id: 2,
            x: Some(50.0),
            y: Some(75.0),
            z: None,
            f: None,
        },
    ];

    // Capture pre-sort anchor mapping: (travel_move_index, anchor_entity_id)
    // This lets us assert post-sort that no travel move was re-aimed.
    let pre_sort_anchors: Vec<(usize, u64)> = travel_moves
        .iter()
        .enumerate()
        .map(|(i, tm)| (i, tm.entity_id))
        .collect();

    let mut layer = make_layer(0, 0.2, entities);
    layer.travel_moves = travel_moves;
    let mut layers = vec![layer];

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .sort_layer_by(0, SortKey::ByPriorityAndEntityId)
        .expect("sort_layer_by should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    let layer = &layers[0];

    // Verify ascending sort: each successive key must be >= previous
    for window in layer.ordered_entities.windows(2) {
        let key_a = (window[0].path.role.default_priority(), window[0].entity_id);
        let key_b = (window[1].path.role.default_priority(), window[1].entity_id);
        assert!(
            key_a <= key_b,
            "expected sorted order but found {:?} > {:?}",
            key_a,
            key_b
        );
    }

    // Regression check (Packet 39 anchor invariant): every TravelMove.entity_id must
    // still resolve to an entity present in ordered_entities after the sort.
    let present_ids: std::collections::HashSet<u64> =
        layer.ordered_entities.iter().map(|e| e.entity_id).collect();
    for tm in &layer.travel_moves {
        assert!(
            present_ids.contains(&tm.entity_id),
            "travel_move references entity_id={} which is no longer present after sort",
            tm.entity_id
        );
    }

    // Re-aim regression check: verify each travel_move still points to the same
    // anchor entity_id it had before the sort (no travel was re-pointed).
    assert_eq!(
        layer.travel_moves.len(),
        pre_sort_anchors.len(),
        "travel_moves count must not change after sort"
    );
    for (i, expected_anchor) in &pre_sort_anchors {
        let actual_anchor = layer.travel_moves[*i].entity_id;
        assert_eq!(
            actual_anchor, *expected_anchor,
            "travel_moves[{}] was re-aimed: expected entity_id={} but got {}",
            i, expected_anchor, actual_anchor
        );
    }
}

// =============================================================================
// AC-4: insert_synthetic_layer_after_inserts_at_position (migrated to SyntheticLayerData)
// =============================================================================

/// With 3 layers, after insert_synthetic_layer_after(0, SyntheticLayerData { z, paths })
/// and apply_to, the vec becomes [layers[0], synth, layers[1], layers[2]].
/// The synth layer's entity_id namespace does not collide with neighbors.
#[test]
fn insert_synthetic_layer_after_inserts_at_position() {
    let mut layers = make_layers(3);
    // Give layers[0] an entity with id=1, layers[1] id=2, layers[2] id=3
    layers[0]
        .ordered_entities
        .push(make_entity(1, ExtrusionRole::OuterWall));
    layers[1]
        .ordered_entities
        .push(make_entity(2, ExtrusionRole::OuterWall));
    layers[2]
        .ordered_entities
        .push(make_entity(3, ExtrusionRole::OuterWall));

    // Synth layer specifies z and one path; host stamps entity_ids.
    let synth_path = make_path(ExtrusionRole::Ironing);
    let synth = SyntheticLayerData {
        z: 0.15,
        paths: vec![synth_path],
    };

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .insert_synthetic_layer_after(0, synth)
        .expect("insert_synthetic_layer_after should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    assert_eq!(layers.len(), 4, "should have 4 layers after insertion");
    assert_eq!(layers[0].global_layer_index, 0);
    // The inserted synth layer is at index 1; its z should match what was specified
    assert!(
        (layers[1].z - 0.15).abs() < 1e-6,
        "synth layer z should be 0.15, got {}",
        layers[1].z
    );
    assert_eq!(layers[2].global_layer_index, 1);
    assert_eq!(layers[3].global_layer_index, 2);

    // Verify entity_id namespaces don't collide between synth and its neighbors
    let synth_ids: std::collections::HashSet<u64> = layers[1]
        .ordered_entities
        .iter()
        .map(|e| e.entity_id)
        .collect();
    let neighbor0_ids: std::collections::HashSet<u64> = layers[0]
        .ordered_entities
        .iter()
        .map(|e| e.entity_id)
        .collect();
    let neighbor2_ids: std::collections::HashSet<u64> = layers[2]
        .ordered_entities
        .iter()
        .map(|e| e.entity_id)
        .collect();

    assert!(
        synth_ids.is_disjoint(&neighbor0_ids),
        "synth entity_ids must not collide with layers[0]"
    );
    assert!(
        synth_ids.is_disjoint(&neighbor2_ids),
        "synth entity_ids must not collide with layers[2]"
    );
}

// =============================================================================
// AC-8: legacy_push_preserves_prepend (Skirt sorts first due to priority 0)
// =============================================================================

/// Given a layer with two OuterWall entities (perimeters), when the legacy
/// push_entity_to_layer is called with a Skirt path and apply_to runs,
/// the skirt entity appears at index 0 of ordered_entities (Skirt priority==0,
/// lowest priority value, stable-sorts to front).
#[test]
fn legacy_push_preserves_prepend() {
    let entities = vec![
        make_entity(1, ExtrusionRole::OuterWall),
        make_entity(2, ExtrusionRole::OuterWall),
    ];
    let mut layers = vec![make_layer(0, 0.2, entities)];

    let mut builder = FinalizationOutputBuilder::new();
    // Legacy alias — push_entity_to_layer(layer, path, region) wraps priority=0
    builder
        .push_entity_to_layer(0, make_path(ExtrusionRole::Skirt), 0, make_region_key())
        .expect("push_entity_to_layer (legacy) should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    assert_eq!(layers[0].ordered_entities.len(), 3);
    assert_eq!(
        layers[0].ordered_entities[0].role,
        ExtrusionRole::Skirt,
        "Skirt (priority 0) should sort to index 0"
    );
}

// =============================================================================
// NEG-1: modify_entity_unknown_id_errors
// =============================================================================

/// Given layer entities with ids {1, 2}, when modify_entity(layer=0, entity_id=99, ...)
/// is recorded and apply_to runs, the result is Err whose message contains both
/// the literal substrings "entity_id" and "99". No entity is mutated.
#[test]
fn modify_entity_unknown_id_errors() {
    let entities = vec![
        make_entity(1, ExtrusionRole::OuterWall),
        make_entity(2, ExtrusionRole::InnerWall),
    ];
    let original_speed_factors: Vec<f32> = entities.iter().map(|e| e.path.speed_factor).collect();
    let mut layers = vec![make_layer(0, 0.2, entities)];

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .modify_entity(0, 99, EntityMutation::SetSpeedFactor(0.1))
        .expect("recording modify_entity should succeed (error deferred to apply_to)");

    let result = builder.apply_to(&mut layers);
    assert!(
        result.is_err(),
        "apply_to should return Err for unknown entity_id"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("entity_id"),
        "error message should contain 'entity_id', got: {:?}",
        msg
    );
    assert!(
        msg.contains("99"),
        "error message should contain '99', got: {:?}",
        msg
    );

    // No entity should have been mutated
    for (entity, original_sf) in layers[0]
        .ordered_entities
        .iter()
        .zip(original_speed_factors.iter())
    {
        assert!(
            (entity.path.speed_factor - original_sf).abs() < 1e-6,
            "entity_id={} should be unchanged",
            entity.entity_id
        );
    }
}

// =============================================================================
// NEG-2: insert_synthetic_layer_out_of_bounds_errors
// =============================================================================

/// With 3 layers, when insert_synthetic_layer_after(99, synth) is recorded and
/// apply_to runs, the result is Err with message containing "synthetic" and "99".
/// The original Vec<LayerCollectionIR> length is unchanged.
#[test]
fn insert_synthetic_layer_out_of_bounds_errors() {
    let mut layers = make_layers(3);
    let synth = SyntheticLayerData {
        z: 0.15,
        paths: vec![],
    };

    let mut builder = FinalizationOutputBuilder::new();
    builder.insert_synthetic_layer_after(99, synth).expect(
        "recording insert_synthetic_layer_after should succeed (error deferred to apply_to)",
    );

    let result = builder.apply_to(&mut layers);
    assert!(
        result.is_err(),
        "apply_to should return Err for out-of-bounds index"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("synthetic"),
        "error message should contain 'synthetic', got: {:?}",
        msg
    );
    assert!(
        msg.contains("99"),
        "error message should contain '99', got: {:?}",
        msg
    );

    // Original vec length must be unchanged
    assert_eq!(
        layers.len(),
        3,
        "layer count should remain 3 after failed insert"
    );
}

// =============================================================================
// NEG-3: ties_preserve_insertion_order (stable-sort tiebreaker)
// =============================================================================

/// Given a layer with two distinct Ironing entities pushed by separate
/// push_entity_with_priority(..., priority) calls at the same priority,
/// the relative order of those two entities post-merge is the producer-call order
/// (stable-sort guarantee).
#[test]
fn ties_preserve_insertion_order() {
    // Start with an empty layer (no pre-existing entities)
    let mut layers = vec![make_layer(0, 0.2, vec![])];

    let ironing_priority = ExtrusionRole::Ironing.default_priority();

    let mut builder = FinalizationOutputBuilder::new();

    // Push "ironing A" first, then "ironing B" — both at same priority
    let path_a = ExtrusionPath3D {
        points: vec![],
        speed_factor: 0.8, // marker to distinguish A
        ..extrusion_path3d_base(ExtrusionRole::Ironing)
    };
    let path_b = ExtrusionPath3D {
        points: vec![],
        speed_factor: 0.6, // marker to distinguish B
        ..extrusion_path3d_base(ExtrusionRole::Ironing)
    };

    builder
        .push_entity_with_priority(0, path_a, 0, make_region_key(), ironing_priority)
        .expect("push A should succeed");
    builder
        .push_entity_with_priority(0, path_b, 0, make_region_key(), ironing_priority)
        .expect("push B should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    assert_eq!(
        layers[0].ordered_entities.len(),
        2,
        "should have 2 ironing entities"
    );

    // A was pushed first → must appear before B (stable-sort preserves insertion order on ties)
    let sf0 = layers[0].ordered_entities[0].path.speed_factor;
    let sf1 = layers[0].ordered_entities[1].path.speed_factor;
    assert!(
        (sf0 - 0.8).abs() < 1e-6,
        "first entity should be ironing A (speed_factor=0.8), got {}",
        sf0
    );
    assert!(
        (sf1 - 0.6).abs() < 1e-6,
        "second entity should be ironing B (speed_factor=0.6), got {}",
        sf1
    );
}

// =============================================================================
// NEW AC-1: modify_entity_set_speed_factor_applies
// =============================================================================

/// Explicit fixture test for EntityMutation::SetSpeedFactor (AC-1 per packet.spec.md).
/// Three entities (ids 1, 2, 3); entity 2 starts at speed_factor==1.0.
/// After SetSpeedFactor(0.5) on entity 2, entity 2 has speed_factor==0.5 and
/// entities 1 and 3 are byte-unchanged (speed_factor still 1.0).
#[test]
fn modify_entity_set_speed_factor_applies() {
    let entities = vec![
        make_entity(1, ExtrusionRole::OuterWall),
        make_entity(2, ExtrusionRole::InnerWall),
        make_entity(3, ExtrusionRole::SparseInfill),
    ];
    let mut layers = vec![make_layer(0, 0.2, entities)];

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .modify_entity(0, 2, EntityMutation::SetSpeedFactor(0.5))
        .expect("modify_entity should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    for entity in &layers[0].ordered_entities {
        if entity.entity_id == 2 {
            assert!(
                (entity.path.speed_factor - 0.5).abs() < 1e-6,
                "entity_id=2 should have speed_factor 0.5, got {}",
                entity.path.speed_factor
            );
        } else {
            assert!(
                (entity.path.speed_factor - 1.0).abs() < 1e-6,
                "entity_id={} should have speed_factor 1.0 (unchanged), got {}",
                entity.entity_id,
                entity.path.speed_factor
            );
        }
    }

    // Packet 189: a whole-entity SetSpeedFactor must NOT create a per-point
    // carrier row. `speed_profiles` stays empty for the pre-packet behaviour.
    assert!(
        layers[0].speed_profiles.is_empty(),
        "SetSpeedFactor must not write any EntitySpeedProfile row, got {}",
        layers[0].speed_profiles.len()
    );
}

// =============================================================================
// NEW AC-2: modify_entity_set_flow_factor_applies
// =============================================================================

/// Explicit fixture test for EntityMutation::SetFlowFactor (AC-2 per packet.spec.md).
/// Three entities (ids 1, 2, 3); entity 2 has two Point3WithWidth points each with
/// flow_factor==1.0.  After SetFlowFactor(0.7) on entity 2, EVERY points[i].flow_factor
/// for entity 2 is 0.7.  Entities 1 and 3 per-point flow_factors are unchanged.
#[test]
fn modify_entity_set_flow_factor_applies() {
    let entity1 = PrintEntity {
        entity_id: 1,
        path: make_path_with_flow(ExtrusionRole::OuterWall, 1.0),
        region_key: make_region_key(),
        ..print_entity_base(ExtrusionRole::OuterWall)
    };
    let entity2 = PrintEntity {
        entity_id: 2,
        path: make_path_with_flow(ExtrusionRole::InnerWall, 1.0),
        region_key: make_region_key(),
        topo_order: 1,
        ..print_entity_base(ExtrusionRole::InnerWall)
    };
    let entity3 = PrintEntity {
        entity_id: 3,
        path: make_path_with_flow(ExtrusionRole::SparseInfill, 1.0),
        region_key: make_region_key(),
        topo_order: 2,
        ..print_entity_base(ExtrusionRole::SparseInfill)
    };

    let mut layers = vec![make_layer(0, 0.2, vec![entity1, entity2, entity3])];

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .modify_entity(0, 2, EntityMutation::SetFlowFactor(0.7))
        .expect("modify_entity should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    for entity in &layers[0].ordered_entities {
        if entity.entity_id == 2 {
            assert!(
                !entity.path.points.is_empty(),
                "entity_id=2 should have points"
            );
            for (i, pt) in entity.path.points.iter().enumerate() {
                assert!(
                    (pt.flow_factor - 0.7).abs() < 1e-6,
                    "entity_id=2 points[{}].flow_factor should be 0.7, got {}",
                    i,
                    pt.flow_factor
                );
            }
        } else {
            for (i, pt) in entity.path.points.iter().enumerate() {
                assert!(
                    (pt.flow_factor - 1.0).abs() < 1e-6,
                    "entity_id={} points[{}].flow_factor should be 1.0 (unchanged), got {}",
                    entity.entity_id,
                    i,
                    pt.flow_factor
                );
            }
        }
    }
}

// =============================================================================
// NEW NEG-4: closure_api_is_fully_removed
// =============================================================================

/// Grep regression: asserts that after Step 3 lands, the closure-based generic
/// bounds for `FnOnce(&mut PrintEntity)` and `Fn(&PrintEntity)` are gone from
/// the FinalizationOutputBuilder impl block in traits.rs.
///
/// This test FAILS NOW (Step 1A red-bar) because those bounds still exist.
/// It will pass once Step 3 removes the closure-typed method signatures.
#[test]
fn closure_api_is_fully_removed() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let traits_path = format!("{}/src/traits.rs", manifest_dir);
    let source = std::fs::read_to_string(&traits_path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", traits_path, e));

    assert!(
        !source.contains("F: FnOnce(&mut PrintEntity)"),
        "closure bound 'F: FnOnce(&mut PrintEntity)' must be removed from traits.rs"
    );
    assert!(
        !source.contains("F: Fn(&PrintEntity)"),
        "closure bound 'F: Fn(&PrintEntity)' must be removed from traits.rs"
    );
}

// =============================================================================
// Packet 189 AC: modify_entity_set_point_speed_factors_applies
// =============================================================================

/// Build an `ExtrusionPath3D` with exactly `n` points along +X.
fn make_path_with_n_points(role: ExtrusionRole, n: usize) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: (0..n)
            .map(|i| slicer_ir::Point3WithWidth {
                x: i as f32,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            })
            .collect(),
        ..extrusion_path3d_base(role)
    }
}

/// Build a `PrintEntity` whose path has exactly `n` points.
fn make_entity_with_n_points(entity_id: u64, role: ExtrusionRole, n: usize) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: make_path_with_n_points(role.clone(), n),
        region_key: make_region_key(),
        ..print_entity_base(role)
    }
}

/// `EntityMutation::SetPointSpeedFactors` on a 4-point entity writes exactly one
/// `EntitySpeedProfile` row (entity_id == 2, factors == the supplied vector) into
/// `layers[0].speed_profiles`, leaves the untouched entity without a row, and does
/// NOT overwrite any entity's whole-entity `path.speed_factor`.
#[test]
fn modify_entity_set_point_speed_factors_applies() {
    let entities = vec![
        make_entity_with_n_points(1, ExtrusionRole::OuterWall, 4),
        make_entity_with_n_points(2, ExtrusionRole::InnerWall, 4),
    ];
    let mut layers = vec![make_layer(0, 0.2, entities)];

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .modify_entity(
            0,
            2,
            EntityMutation::SetPointSpeedFactors(vec![0.5, 0.6, 0.7, 0.8]),
        )
        .expect("modify_entity should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    assert_eq!(
        layers[0].speed_profiles.len(),
        1,
        "exactly one EntitySpeedProfile row should be written, got {}",
        layers[0].speed_profiles.len()
    );
    let profile: &slicer_ir::EntitySpeedProfile = &layers[0].speed_profiles[0];
    assert_eq!(
        profile.entity_id, 2,
        "the profile row must belong to entity_id 2"
    );
    assert_eq!(
        profile.factors,
        vec![0.5f32, 0.6, 0.7, 0.8],
        "profile factors must match the mutation payload verbatim"
    );

    // The untouched entity contributes no row.
    assert!(
        !layers[0].speed_profiles.iter().any(|p| p.entity_id == 1),
        "entity_id=1 was not mutated and must not have a profile row"
    );

    // Whole-entity scalars are untouched by the per-point carrier.
    for entity in &layers[0].ordered_entities {
        assert!(
            (entity.path.speed_factor - 1.0).abs() < 1e-6,
            "entity_id={} path.speed_factor must stay 1.0, got {}",
            entity.entity_id,
            entity.path.speed_factor
        );
    }
}

#[test]
fn set_path_points_then_point_speed_factors_applies_in_order() {
    let original_point_count = 3;
    let entities = vec![make_entity_with_n_points(
        2,
        ExtrusionRole::SparseInfill,
        original_point_count,
    )];
    let mut layers = vec![make_layer(0, 0.2, entities)];
    let new_points = make_path_with_n_points(ExtrusionRole::SparseInfill, 5).points;
    let new_factors = vec![0.5_f32, 0.6, 0.7, 0.8, 0.9];

    assert!(new_points.len() > original_point_count);
    assert_eq!(new_points.len(), new_factors.len());

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .modify_entity(0, 2, EntityMutation::SetPathPoints(new_points.clone()))
        .expect("recording SetPathPoints should succeed");
    builder
        .modify_entity(
            0,
            2,
            EntityMutation::SetPointSpeedFactors(new_factors.clone()),
        )
        .expect("recording SetPointSpeedFactors should succeed");

    builder
        .apply_to(&mut layers)
        .expect("path points must be applied before point speed factors");

    let entity = &layers[0].ordered_entities[0];
    assert_eq!(entity.path.points, new_points);
    let profile = layers[0]
        .speed_profiles
        .iter()
        .find(|profile| profile.entity_id == 2)
        .expect("expected a speed profile for entity_id 2");
    assert_eq!(profile.factors, new_factors);
    assert_eq!(profile.factors.len(), entity.path.points.len());
}

#[test]
fn set_path_points_rejects_empty_or_unclosed_loop() {
    let make_point = |x: f32, y: f32| slicer_ir::Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        ..Default::default()
    };
    let original_points = vec![
        make_point(0.0, 0.0),
        make_point(1.0, 0.0),
        make_point(1.0, 1.0),
        make_point(0.0, 0.0),
    ];

    assert!(ExtrusionRole::OuterWall.is_loop());

    let mut empty_layers = {
        let mut entity = make_entity_with_n_points(1, ExtrusionRole::OuterWall, 4);
        entity.path.points = original_points.clone();
        assert!(entity.path.is_closed());
        vec![make_layer(0, 0.2, vec![entity])]
    };
    let mut empty_builder = FinalizationOutputBuilder::new();
    empty_builder
        .modify_entity(0, 1, EntityMutation::SetPathPoints(Vec::new()))
        .expect("recording empty SetPathPoints should succeed");
    let empty_result = empty_builder.apply_to(&mut empty_layers);
    assert!(empty_result.is_err());
    assert_eq!(
        empty_layers[0].ordered_entities[0].path.points,
        original_points
    );

    let mut unclosed_layers = {
        let mut entity = make_entity_with_n_points(1, ExtrusionRole::OuterWall, 4);
        entity.path.points = original_points.clone();
        vec![make_layer(0, 0.2, vec![entity])]
    };
    let unclosed_points = vec![
        make_point(0.0, 0.0),
        make_point(1.0, 0.0),
        make_point(1.0, 1.0),
    ];
    assert_ne!(
        (
            unclosed_points.first().unwrap().x,
            unclosed_points.first().unwrap().y
        ),
        (
            unclosed_points.last().unwrap().x,
            unclosed_points.last().unwrap().y
        )
    );
    let mut unclosed_builder = FinalizationOutputBuilder::new();
    unclosed_builder
        .modify_entity(0, 1, EntityMutation::SetPathPoints(unclosed_points))
        .expect("recording unclosed SetPathPoints should succeed");
    let unclosed_result = unclosed_builder.apply_to(&mut unclosed_layers);
    assert!(unclosed_result.is_err());
    assert_eq!(
        unclosed_layers[0].ordered_entities[0].path.points,
        original_points
    );
}

// =============================================================================
// Packet 189 NEG: modify_entity_set_point_speed_factors_length_mismatch_errors
// =============================================================================

/// A `SetPointSpeedFactors` payload whose length differs from the target entity's
/// point count makes `apply_to` return Err naming BOTH lengths, and writes no
/// partial profile row.
#[test]
fn modify_entity_set_point_speed_factors_length_mismatch_errors() {
    let entities = vec![
        make_entity_with_n_points(1, ExtrusionRole::OuterWall, 4),
        make_entity_with_n_points(2, ExtrusionRole::InnerWall, 4),
    ];
    let mut layers = vec![make_layer(0, 0.2, entities)];

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .modify_entity(0, 2, EntityMutation::SetPointSpeedFactors(vec![0.5, 0.6]))
        .expect("recording modify_entity should succeed (error deferred to apply_to)");

    let result = builder.apply_to(&mut layers);
    assert!(
        result.is_err(),
        "apply_to should return Err on a factors/points length mismatch"
    );
    let msg = result.unwrap_err();
    // The message must name both the supplied length (2) and the point count (4).
    // Match on standalone tokens so an unrelated "24" cannot satisfy the check.
    let has_token = |msg: &str, tok: char| {
        msg.char_indices().any(|(i, c)| {
            c == tok
                && !msg[..i].ends_with(|p: char| p.is_ascii_digit())
                && !msg[i + 1..].starts_with(|p: char| p.is_ascii_digit())
        })
    };
    assert!(
        has_token(&msg, '2'),
        "error message should name the supplied factor count 2, got: {:?}",
        msg
    );
    assert!(
        has_token(&msg, '4'),
        "error message should name the entity point count 4, got: {:?}",
        msg
    );

    assert!(
        layers[0].speed_profiles.is_empty(),
        "no partial EntitySpeedProfile row may be written on error, got {}",
        layers[0].speed_profiles.len()
    );
}

// =============================================================================
// Packet 189 / ADR-0052 Decision 1: the upsert guarantee
// =============================================================================

/// A second `SetPointSpeedFactors` for the SAME `entity_id` REPLACES the existing
/// `EntitySpeedProfile` row rather than appending a second one, and the later
/// submission wins. Packet 191 relies on this (geometry mutation followed by a
/// resized profile for the same entity).
#[test]
fn modify_entity_set_point_speed_factors_upsert_replaces_row() {
    let entities = vec![
        make_entity_with_n_points(1, ExtrusionRole::OuterWall, 4),
        make_entity_with_n_points(2, ExtrusionRole::InnerWall, 4),
    ];
    let mut layers = vec![make_layer(0, 0.2, entities)];

    let mut builder = FinalizationOutputBuilder::new();
    builder
        .modify_entity(
            0,
            2,
            EntityMutation::SetPointSpeedFactors(vec![0.5, 0.6, 0.7, 0.8]),
        )
        .expect("first modify_entity should succeed");
    builder
        .modify_entity(
            0,
            2,
            EntityMutation::SetPointSpeedFactors(vec![0.1, 0.2, 0.3, 0.4]),
        )
        .expect("second modify_entity should succeed");

    builder
        .apply_to(&mut layers)
        .expect("apply_to should succeed");

    assert_eq!(
        layers[0].speed_profiles.len(),
        1,
        "the second write must REPLACE the row, not append: got {} rows ({:?})",
        layers[0].speed_profiles.len(),
        layers[0].speed_profiles
    );
    let profile: &slicer_ir::EntitySpeedProfile = &layers[0].speed_profiles[0];
    assert_eq!(
        profile.entity_id, 2,
        "the surviving row must still belong to entity_id 2"
    );
    assert_eq!(
        profile.factors,
        vec![0.1f32, 0.2, 0.3, 0.4],
        "submission order wins: the SECOND payload must be the stored one, got {:?}",
        profile.factors
    );
}
