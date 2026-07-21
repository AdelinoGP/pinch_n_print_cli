#![allow(missing_docs)]

//! TDD red tests for packet `39_stable-entity-ids`.
//!
//! These tests are EXPECTED to fail to compile until Step 2 lands
//! (adds `entity_id: u64` to `PrintEntity`, replaces `TravelMove.after_entity_index`
//! with `TravelMove.entity_id: u64`, and exports `LayerEntityIdGen`).
//!
//! Acceptance criteria exercised:
//!   - AC-1: unique entity IDs and resolvable travel anchors within a layer
//!   - AC-4: entity_id round-trips through serde/postcard
//!   - AC-5: LayerEntityIdGen is strictly monotonic
//!   - Negative: LayerEntityIdGen is !Send + !Sync (static_assertions form)

use std::collections::HashSet;

use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, LayerEntityIdGen, ObjectId, Point3WithWidth,
    PrintEntity, RegionKey, SemVer, TravelMove,
};

// ============================================================================
// Helper fixtures
// ============================================================================

fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn region_key() -> RegionKey {
    RegionKey {
        global_layer_index: 0,
        object_id: ObjectId::from("test-object"),
        region_id: 1u64,
        variant_chain: Vec::new(),
    }
}

fn point(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

fn make_entity(entity_id: u64, x_start: f32, x_end: f32, y: f32, z: f32) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points: vec![point(x_start, y, z), point(x_end, y, z)],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        tool_index: 1,
        region_key: region_key(),
        topo_order: 0,
    }
}

fn make_layer(entities: Vec<PrintEntity>, travel_moves: Vec<TravelMove>) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: entities,
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves,
    }
}

// ============================================================================
// Test 1: unique_per_layer_and_resolvable
// ============================================================================

#[test]
fn unique_per_layer_and_resolvable() {
    // Construct a LayerCollectionIR with N=3 PrintEntity entries (entity_ids 1, 2, 3)
    // and 2 TravelMoves anchored by entity_id to entities present in the layer.
    let entity_a = make_entity(1, 0.0, 10.0, 0.0, 0.2);
    let entity_b = make_entity(2, 20.0, 30.0, 5.0, 0.2);
    let entity_c = make_entity(3, 40.0, 50.0, 10.0, 0.2);

    let travel1 = TravelMove {
        entity_id: 1,
        x: Some(20.0),
        y: Some(5.0),
        z: None,
        f: None,
    };
    let travel2 = TravelMove {
        entity_id: 2,
        x: Some(40.0),
        y: Some(10.0),
        z: None,
        f: None,
    };

    let layer = make_layer(vec![entity_a, entity_b, entity_c], vec![travel1, travel2]);

    // Assert: every entity_id in ordered_entities is unique within the layer
    let ids: Vec<u64> = layer.ordered_entities.iter().map(|e| e.entity_id).collect();
    let unique_ids: HashSet<u64> = ids.iter().cloned().collect();
    assert_eq!(
        ids.len(),
        unique_ids.len(),
        "entity_ids must be unique within a layer, found duplicates in {:?}",
        ids
    );

    // Assert: every TravelMove.entity_id is contained in the set of entity IDs
    for tm in &layer.travel_moves {
        assert!(
            unique_ids.contains(&tm.entity_id),
            "TravelMove.entity_id={} is not present in layer entity IDs {:?}",
            tm.entity_id,
            unique_ids
        );
    }
}

// ============================================================================
// Test 2: entity_id_round_trips_through_serde
// ============================================================================

#[test]
fn entity_id_round_trips_through_serde() {
    // Build a LayerCollectionIR, serialize via postcard, deserialize back.
    // Assert entity_ids and TravelMove.entity_id round-trip exactly.
    let entity_a = make_entity(1, 0.0, 10.0, 0.0, 0.2);
    let entity_b = make_entity(2, 20.0, 30.0, 5.0, 0.2);
    let entity_c = make_entity(3, 40.0, 50.0, 10.0, 0.2);

    let travel1 = TravelMove {
        entity_id: 1,
        x: Some(20.0),
        y: Some(5.0),
        z: None,
        f: None,
    };
    let travel2 = TravelMove {
        entity_id: 3,
        x: Some(0.0),
        y: Some(0.0),
        z: None,
        f: None,
    };

    let original = make_layer(vec![entity_a, entity_b, entity_c], vec![travel1, travel2]);

    // Serialize and deserialize via postcard
    let bytes = postcard::to_allocvec(&original).expect("postcard serialize should succeed");
    let roundtripped: LayerCollectionIR =
        postcard::from_bytes(&bytes).expect("postcard deserialize should succeed");

    // Verify entity_ids round-trip
    assert_eq!(
        original.ordered_entities.len(),
        roundtripped.ordered_entities.len(),
        "entity count must be preserved"
    );
    for (orig, rt) in original
        .ordered_entities
        .iter()
        .zip(roundtripped.ordered_entities.iter())
    {
        assert_eq!(
            orig.entity_id, rt.entity_id,
            "entity_id must round-trip through postcard serde"
        );
    }

    // Verify TravelMove.entity_id round-trips
    assert_eq!(
        original.travel_moves.len(),
        roundtripped.travel_moves.len(),
        "travel_move count must be preserved"
    );
    for (orig, rt) in original
        .travel_moves
        .iter()
        .zip(roundtripped.travel_moves.iter())
    {
        assert_eq!(
            orig.entity_id, rt.entity_id,
            "TravelMove.entity_id must round-trip through postcard serde"
        );
    }
}

// ============================================================================
// Test 3: id_gen_is_strictly_monotonic
// ============================================================================

#[test]
fn id_gen_is_strictly_monotonic() {
    // Construct a fresh LayerEntityIdGen, call .next() 5 times.
    // Assert: first ID is 1, each subsequent ID is strictly greater, all 5 distinct.
    let gen = LayerEntityIdGen::new();

    let ids: Vec<u64> = (0..5).map(|_| gen.next()).collect();

    assert_eq!(
        ids[0], 1,
        "first ID from LayerEntityIdGen must be 1, got {}",
        ids[0]
    );

    for window in ids.windows(2) {
        assert!(
            window[1] > window[0],
            "LayerEntityIdGen must be strictly monotonic: {} <= {}",
            window[1],
            window[0]
        );
    }

    let unique: HashSet<u64> = ids.iter().cloned().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "LayerEntityIdGen must produce all distinct IDs, got {:?}",
        ids
    );
}

// ============================================================================
// Test 4: id_gen_no_collision_under_contention
// ============================================================================
//
// Design: LayerEntityIdGen uses Cell<u64> and is intentionally !Send + !Sync.
// We use static_assertions to lock this invariant at compile time.
// If Step 2 deviates to AtomicU64 (making it Send+Sync), this test will fail
// to compile, signalling Step 2's worker to switch to the threaded form.

#[test]
fn id_gen_no_collision_under_contention() {
    // The real guard is the static assertion below.
    // This runtime body verifies sequential uniqueness as a sanity check.
    let gen = LayerEntityIdGen::new();
    let ids: Vec<u64> = (0..10).map(|_| gen.next()).collect();
    let unique: HashSet<u64> = ids.iter().cloned().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "sequential next() calls must all produce distinct IDs, got {:?}",
        ids
    );
}

// Compile-time assertion: LayerEntityIdGen must be !Send AND !Sync.
// If Step 2 uses AtomicU64 instead of Cell<u64>, this will be a compile error.
static_assertions::assert_not_impl_any!(slicer_ir::LayerEntityIdGen: Send, Sync);
