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
    validate_travel_anchors, AnchoredEntity, AnchoredEntityProvenance, AnchoredEventRuntimeHooks,
    AnchoredGeometryContract, CapabilityDerivedEventClosure, ExtrusionPath3D, ExtrusionRole,
    LayerCollectionIR, ObjectId, OrderedEventCollection, Point3WithWidth, PrintEntity, RegionKey,
    SemVer, TravelMove,
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
        ..Default::default()
    }
}

fn make_entity(entity_id: u64, x: f32, y: f32, z: f32) -> PrintEntity {
    // exhaustive: file-local base; sdk fixture home would pull host-algos into this crate's dev graph (packet 196 [FWD])
    PrintEntity {
        entity_id,
        // exhaustive: validation fixture pins the complete IR path
        path: ExtrusionPath3D {
            points: vec![point(x, y, z), point(x + 5.0, y, z)],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
            tool_index: None,
            order_lock: None,
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
        z: 0.2,
        ordered_entities: entities,
        travel_moves,
        ..Default::default()
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
        ..Default::default()
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

#[test]
fn anchored_contracts_construct_round_trip_and_order() {
    // exhaustive: no Default impl for AnchoredEntity; anchored-contract fixture pins every field
    let planar = AnchoredEntity {
        local_id: 2,
        anchor_global_layer_index: 4,
        geometry: AnchoredGeometryContract::Planar { z: 2_000 },
        input_capabilities: vec!["geometry".into()],
        output_capabilities: vec!["paths".into()],
        provenance: AnchoredEntityProvenance {
            requesting_feature: "support".into(),
            source_plan_entry: "entry-2".into(),
        },
        path_points: vec![Point3WithWidth {
            x: 1.0,
            y: 2.0,
            z: 0.2,
            width: 0.45,
            flow_factor: 1.0,
            ..Default::default()
        }],
        role: ExtrusionRole::SupportMaterial,
    };
    // exhaustive: no Default impl for AnchoredEntity; anchored-contract fixture pins every field
    let spanning = AnchoredEntity {
        local_id: 1,
        anchor_global_layer_index: 4,
        geometry: AnchoredGeometryContract::ZSpanning {
            min_z: 1_000,
            max_z: 3_000,
        },
        input_capabilities: vec!["geometry".into()],
        output_capabilities: vec!["paths".into()],
        provenance: AnchoredEntityProvenance {
            requesting_feature: "support".into(),
            source_plan_entry: "entry-1".into(),
        },
        path_points: Vec::new(),
        role: ExtrusionRole::SupportMaterial,
    };
    assert!(AnchoredGeometryContract::Planar { z: 2_000 }.contains_z(2_000));
    assert!(!AnchoredGeometryContract::Planar { z: 2_000 }.contains_z(2_001));
    assert!(spanning.geometry.contains_z(2_500));
    assert_eq!(planar.path_points[0].z, 0.2);

    let closure = CapabilityDerivedEventClosure::derive(
        &planar.input_capabilities,
        &planar.output_capabilities,
    );
    assert_eq!(closure.derived_capabilities, vec!["geometry", "paths"]);

    let mut collection = OrderedEventCollection {
        anchor_global_layer_index: 4,
        events: vec![planar, spanning],
        runtime_hooks: AnchoredEventRuntimeHooks::default(),
    };
    collection.sort_deterministically();
    assert_eq!(collection.events[0].local_id, 1);
    let encoded = serde_json::to_string(&collection).unwrap();
    assert_eq!(
        serde_json::from_str::<OrderedEventCollection>(&encoded).unwrap(),
        collection
    );
}

#[test]
fn flat_layer_contract_remains_unchanged() {
    let layer = make_layer(vec![make_entity(1, 0.0, 0.0, 0.2)], Vec::new());
    assert_eq!(layer.global_layer_index, 0);
    assert_eq!(layer.z, 0.2);
    assert_eq!(layer.ordered_entities.len(), 1);
}
