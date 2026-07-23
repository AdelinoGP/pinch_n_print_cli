#![allow(missing_docs)]

//! TDD red tests for packet `39_stable-entity-ids` — IR validation helper.
//!
//! These tests are EXPECTED to fail to compile until Step 3 lands
//! (adds `slicer_ir::validate_travel_anchors`).
//!
//! Negative acceptance criteria exercised:
//!   - Dangling TravelMove.entity_id that is not present in ordered_entities
//!     must be rejected with an Err whose diagnostic contains "entity_id" and the ID number.

use slicer_ir::{
    validate_travel_anchors, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, ObjectId,
    Point3WithWidth, PrintEntity, RegionKey, SemVer, TravelMove,
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

fn make_entity(entity_id: u64, x: f32, y: f32, z: f32) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points: vec![point(x, y, z), point(x + 5.0, y, z)],
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
// Test 1: dangling_travel_anchor_rejected
// ============================================================================

#[test]
fn dangling_travel_anchor_rejected() {
    // Construct a LayerCollectionIR with 2 entities (IDs 1 and 2)
    // and 1 TravelMove whose entity_id is 99 (not present).
    let entity_a = make_entity(1, 0.0, 0.0, 0.2);
    let entity_b = make_entity(2, 10.0, 0.0, 0.2);

    let dangling_travel = TravelMove {
        entity_id: 99, // not present in ordered_entities
        x: Some(50.0),
        y: Some(50.0),
        z: None,
        f: None,
    };

    let layer = make_layer(vec![entity_a, entity_b], vec![dangling_travel]);

    // Call validate_travel_anchors — must return Err
    let result = validate_travel_anchors(&layer);

    assert!(
        result.is_err(),
        "validate_travel_anchors must return Err for a dangling entity_id=99, got Ok"
    );

    let err = result.unwrap_err();
    let err_str = err.to_string();

    assert!(
        err_str.contains("entity_id"),
        "error message must contain literal substring 'entity_id', got: {:?}",
        err_str
    );
    assert!(
        err_str.contains("99"),
        "error message must contain the offending ID '99', got: {:?}",
        err_str
    );
}
