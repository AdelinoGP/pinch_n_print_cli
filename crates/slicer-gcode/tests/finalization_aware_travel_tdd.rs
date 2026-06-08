#![allow(missing_docs)]

//! TDD: finalization-aware travel coordination.
//!
//! Verifies that the `reconcile_finalization_travel` host-side pass correctly
//! adjusts `travel_moves` on `LayerCollectionIR` to route through Skirt/Brim
//! and WipeTower geometry, without modifying `ordered_entities`.
//!
//! Acceptance criteria:
//! - AC1: Brim geometry changes first model travel transition
//! - AC2: Wipe tower geometry included in travel reconciliation
//! - AC3: No finalization geometry is a reconciliation no-op

use slicer_gcode::reconcile_finalization_travel;
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, ObjectId, Point3WithWidth, PrintEntity,
    RegionKey, SemVer, ToolChange, TravelRetract,
};

// ============================================================================
// Test fixtures
// ============================================================================

fn semver_fixture() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn point3(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

fn region_key(layer_index: u32, object_id: &str) -> RegionKey {
    RegionKey {
        global_layer_index: layer_index,
        object_id: ObjectId::from(object_id),
        region_id: 1u64,
        variant_chain: Vec::new(),
    }
}

fn make_entity(
    points: Vec<Point3WithWidth>,
    role: ExtrusionRole,
    layer_index: u32,
    object_id: &str,
) -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: region_key(layer_index, object_id),
        topo_order: 0,
    }
}

fn empty_layer(index: u32, z: f32) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver_fixture(),
        global_layer_index: index,
        z,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    }
}

// ============================================================================
// AC1: Brim geometry changes first model travel transition
// ============================================================================

#[test]
fn brim_geometry_changes_first_model_travel_transition() {
    // Given a layer with [Skirt entity, Model entity] and no travel_moves,
    // when reconcile_finalization_travel runs,
    // then a TravelMove is added that routes from the skirt endpoint to the
    // model entity's start point, keyed to the skirt entity's index.

    let skirt_end_x = 10.0_f32;
    let skirt_end_y = 10.0_f32;
    let model_start_x = 50.0_f32;
    let model_start_y = 50.0_f32;
    let z = 0.2_f32;

    let skirt = make_entity(
        vec![point3(5.0, 5.0, z), point3(skirt_end_x, skirt_end_y, z)],
        ExtrusionRole::Skirt,
        0,
        "skirt-obj",
    );
    let model = make_entity(
        vec![
            point3(model_start_x, model_start_y, z),
            point3(60.0, 60.0, z),
        ],
        ExtrusionRole::OuterWall,
        0,
        "model-obj",
    );

    let mut layer = empty_layer(0, z);
    layer.ordered_entities = vec![skirt, model];

    // No travel_moves before reconciliation
    assert!(
        layer.travel_moves.is_empty(),
        "precondition: no travel_moves before reconciliation"
    );

    reconcile_finalization_travel(&mut layer, None);

    // After reconciliation: at least one travel_move routing from skirt to model
    assert!(
        !layer.travel_moves.is_empty(),
        "reconciliation must add at least one travel_move when skirt entities precede model entities"
    );

    // The travel move must be anchored after the skirt entity (entity_id=1)
    let skirt_travel: Vec<_> = layer
        .travel_moves
        .iter()
        .filter(|tm| tm.entity_id == 1u64)
        .collect();
    assert!(
        !skirt_travel.is_empty(),
        "a travel_move must be anchored after the skirt entity (entity_id=1)"
    );

    // The travel move must target the model entity's start position
    let target_move = skirt_travel
        .iter()
        .find(|tm| tm.x.is_some() && tm.y.is_some());
    assert!(
        target_move.is_some(),
        "travel_move must have x and y targeting model start"
    );
    let tm = target_move.unwrap();
    assert!(
        (tm.x.unwrap() - model_start_x).abs() < 0.01,
        "travel_move x must target model start x={}, got {:?}",
        model_start_x,
        tm.x
    );
    assert!(
        (tm.y.unwrap() - model_start_y).abs() < 0.01,
        "travel_move y must target model start y={}, got {:?}",
        model_start_y,
        tm.y
    );

    // ordered_entities must be unchanged
    assert_eq!(
        layer.ordered_entities.len(),
        2,
        "ordered_entities must not be modified"
    );
    assert_eq!(
        layer.ordered_entities[0].role,
        ExtrusionRole::Skirt,
        "first entity must still be Skirt"
    );
    assert_eq!(
        layer.ordered_entities[1].role,
        ExtrusionRole::OuterWall,
        "second entity must still be OuterWall"
    );
}

// ============================================================================
// AC2: Wipe tower geometry included in travel reconciliation
// ============================================================================

#[test]
fn wipe_tower_geometry_is_included_in_travel_reconciliation() {
    // Given a finalized layer with one ToolChange and one appended WipeTower
    // entity block, when the reconciliation pass runs, then the emitted travel
    // sequence includes the wipe-tower detour between the tool change and the
    // next model entity and preserves the matched retract/unretract pairing.

    let wipe_x = 100.0_f32;
    let wipe_y = 100.0_f32;
    let z = 0.2_f32;

    let model1 = make_entity(
        vec![point3(10.0, 10.0, z), point3(20.0, 20.0, z)],
        ExtrusionRole::OuterWall,
        0,
        "obj1",
    );
    let wipe_tower = make_entity(
        vec![point3(wipe_x, wipe_y, z), point3(110.0, 100.0, z)],
        ExtrusionRole::WipeTower,
        0,
        "wipe",
    );
    let model2 = make_entity(
        vec![point3(30.0, 30.0, z), point3(40.0, 40.0, z)],
        ExtrusionRole::OuterWall,
        0,
        "obj1",
    );

    // ToolChange after entity 0 (model1), as packet 15 would emit.
    let tool_change = ToolChange {
        after_entity_index: 0,
        from_tool: 0,
        to_tool: 1,
    };

    // Retract/unretract pairing from packet 15, anchored at entity 0.
    let pre_retracts = vec![
        TravelRetract {
            after_entity_index: 0,
            length: 1.0,
            speed: 30.0,
            is_unretract: false, // retract
            mode: slicer_ir::RetractMode::Gcode,
        },
        TravelRetract {
            after_entity_index: 0,
            length: 1.0,
            speed: 30.0,
            is_unretract: true, // unretract
            mode: slicer_ir::RetractMode::Gcode,
        },
    ];

    let mut layer = empty_layer(0, z);
    layer.ordered_entities = vec![model1, wipe_tower, model2];
    layer.tool_changes = vec![tool_change];
    layer.retracts = pre_retracts.clone();

    assert!(
        layer.travel_moves.is_empty(),
        "precondition: no travel_moves before reconciliation"
    );

    reconcile_finalization_travel(&mut layer, None);

    // After reconciliation: travel_moves must exist routing to/from wipe tower
    assert!(
        !layer.travel_moves.is_empty(),
        "reconciliation must add travel_moves when wipe tower entities exist"
    );

    // At least one travel move must target the wipe tower start position
    let wipe_travel: Vec<_> = layer
        .travel_moves
        .iter()
        .filter(|tm| {
            tm.x.is_some_and(|x| (x - wipe_x).abs() < 0.01)
                && tm.y.is_some_and(|y| (y - wipe_y).abs() < 0.01)
        })
        .collect();
    assert!(
        !wipe_travel.is_empty(),
        "at least one travel_move must target the wipe tower start ({}, {}), travel_moves: {:?}",
        wipe_x,
        wipe_y,
        layer.travel_moves
    );

    // The wipe-tower travel move must be anchored between the entity before
    // the wipe tower (entity_id=1, which is model1 at index 0).
    let wipe_detour = wipe_travel[0];
    assert!(
        wipe_detour.entity_id == 1u64,
        "wipe-tower detour must be anchored after model1 (entity_id=1, before wipe tower), got entity_id={}",
        wipe_detour.entity_id
    );

    // Retract/unretract pairing from packet 15 must be preserved unchanged.
    assert_eq!(
        layer.retracts.len(),
        pre_retracts.len(),
        "retract count must be unchanged after reconciliation"
    );
    for (i, (actual, expected)) in layer.retracts.iter().zip(pre_retracts.iter()).enumerate() {
        assert_eq!(
            actual.after_entity_index, expected.after_entity_index,
            "retract[{}].after_entity_index mismatch",
            i
        );
        assert_eq!(
            actual.is_unretract, expected.is_unretract,
            "retract[{}].is_unretract mismatch",
            i
        );
        assert!(
            (actual.length - expected.length).abs() < 0.001,
            "retract[{}].length mismatch",
            i
        );
    }

    // ordered_entities must be unchanged
    assert_eq!(
        layer.ordered_entities.len(),
        3,
        "ordered_entities must not be modified"
    );
}

// ============================================================================
// AC3: No finalization geometry is a reconciliation no-op
// ============================================================================

#[test]
fn no_finalization_geometry_is_a_reconciliation_no_op() {
    // Given a layer with only model entities (no Skirt, no WipeTower),
    // when reconcile_finalization_travel runs,
    // then travel_moves and retracts are identical to before.

    let model = make_entity(
        vec![point3(10.0, 10.0, 0.2), point3(20.0, 20.0, 0.2)],
        ExtrusionRole::OuterWall,
        0,
        "obj1",
    );
    let model2 = make_entity(
        vec![point3(30.0, 30.0, 0.2), point3(40.0, 40.0, 0.2)],
        ExtrusionRole::InnerWall,
        0,
        "obj1",
    );

    let mut layer = empty_layer(0, 0.2);
    layer.ordered_entities = vec![model, model2];

    // Snapshot pre-reconciliation state
    let pre_travel_count = layer.travel_moves.len();
    let pre_retract_count = layer.retracts.len();

    reconcile_finalization_travel(&mut layer, None);

    assert_eq!(
        layer.travel_moves.len(),
        pre_travel_count,
        "travel_moves must be unchanged when no finalization geometry exists"
    );
    assert_eq!(
        layer.retracts.len(),
        pre_retract_count,
        "retracts must be unchanged when no finalization geometry exists"
    );
    assert!(
        layer.travel_moves.is_empty(),
        "travel_moves must remain empty for no-op case"
    );
    assert!(
        layer.retracts.is_empty(),
        "retracts must remain empty for no-op case"
    );
}

// ============================================================================
// Negative regression: reconciliation preserves entity order
// ============================================================================

#[test]
fn reconciliation_preserves_model_extrusion_entity_order() {
    // Given a layer with [Skirt, ModelWall1, WipeTower, ModelWall2, InnerWall],
    // when reconcile_finalization_travel runs,
    // then ordered_entities is identical in length, roles, topo_orders, and
    // first-point content.  This proves the reconciliation pass never reorders
    // model extrusion entities.

    let z = 0.2_f32;

    let skirt = make_entity(
        vec![point3(0.0, 0.0, z), point3(5.0, 5.0, z)],
        ExtrusionRole::Skirt,
        0,
        "skirt-obj",
    );
    let mut model_wall1 = make_entity(
        vec![point3(10.0, 10.0, z), point3(20.0, 20.0, z)],
        ExtrusionRole::OuterWall,
        0,
        "model-obj",
    );
    model_wall1.topo_order = 1;

    let mut wipe_tower = make_entity(
        vec![point3(100.0, 100.0, z), point3(110.0, 100.0, z)],
        ExtrusionRole::WipeTower,
        0,
        "wipe",
    );
    wipe_tower.topo_order = 2;

    let mut model_wall2 = make_entity(
        vec![point3(30.0, 30.0, z), point3(40.0, 40.0, z)],
        ExtrusionRole::OuterWall,
        0,
        "model-obj",
    );
    model_wall2.topo_order = 3;

    let mut inner_wall = make_entity(
        vec![point3(50.0, 50.0, z), point3(60.0, 60.0, z)],
        ExtrusionRole::InnerWall,
        0,
        "model-obj",
    );
    inner_wall.topo_order = 4;

    let mut layer = empty_layer(0, z);
    layer.ordered_entities = vec![skirt, model_wall1, wipe_tower, model_wall2, inner_wall];

    // Snapshot pre-reconciliation state
    let pre_roles: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| e.role.clone())
        .collect();
    let pre_topo: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| e.topo_order)
        .collect();
    let pre_first_points: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| (e.path.points[0].x, e.path.points[0].y))
        .collect();
    let pre_len = layer.ordered_entities.len();

    reconcile_finalization_travel(&mut layer, None);

    // Length must be unchanged
    assert_eq!(
        layer.ordered_entities.len(),
        pre_len,
        "ordered_entities length must not change after reconciliation"
    );

    // Roles must be in the same order
    let post_roles: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| e.role.clone())
        .collect();
    assert_eq!(
        post_roles, pre_roles,
        "entity roles must remain in the same order after reconciliation"
    );

    // Topo_orders must be in the same order
    let post_topo: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| e.topo_order)
        .collect();
    assert_eq!(
        post_topo, pre_topo,
        "entity topo_orders must remain in the same order after reconciliation"
    );

    // First points must match (proves content wasn't swapped)
    let post_first_points: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| (e.path.points[0].x, e.path.points[0].y))
        .collect();
    assert_eq!(
        post_first_points, pre_first_points,
        "entity first-point content must remain unchanged after reconciliation"
    );
}

#[test]
fn reconciliation_preserves_model_extrusion_entity_order_with_wipe_tower() {
    // Specifically verifies that the presence of a WipeTower entity does NOT
    // cause model extrusion entities to be reordered.  The wipe tower sits
    // between two model entities; reconciliation must leave the ordering
    // untouched while adding travel_moves that route through the tower.

    let z = 0.2_f32;

    let model1 = make_entity(
        vec![point3(10.0, 10.0, z), point3(20.0, 20.0, z)],
        ExtrusionRole::OuterWall,
        0,
        "model-obj",
    );
    let mut wipe_tower = make_entity(
        vec![point3(100.0, 100.0, z), point3(110.0, 100.0, z)],
        ExtrusionRole::WipeTower,
        0,
        "wipe",
    );
    wipe_tower.topo_order = 1;

    let mut model2 = make_entity(
        vec![point3(30.0, 30.0, z), point3(40.0, 40.0, z)],
        ExtrusionRole::OuterWall,
        0,
        "model-obj",
    );
    model2.topo_order = 2;

    let mut layer = empty_layer(0, z);
    layer.ordered_entities = vec![model1, wipe_tower, model2];

    // Snapshot
    let pre_len = layer.ordered_entities.len();
    let pre_roles: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| e.role.clone())
        .collect();
    let pre_topo: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| e.topo_order)
        .collect();
    let pre_first_points: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| (e.path.points[0].x, e.path.points[0].y))
        .collect();

    reconcile_finalization_travel(&mut layer, None);

    // Length unchanged
    assert_eq!(
        layer.ordered_entities.len(),
        pre_len,
        "ordered_entities length must not change with wipe tower present"
    );

    // Roles unchanged
    let post_roles: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| e.role.clone())
        .collect();
    assert_eq!(
        post_roles, pre_roles,
        "entity roles must not change when wipe tower is present"
    );

    // Topo_orders unchanged
    let post_topo: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| e.topo_order)
        .collect();
    assert_eq!(
        post_topo, pre_topo,
        "entity topo_orders must not change when wipe tower is present"
    );

    // First points unchanged
    let post_first_points: Vec<_> = layer
        .ordered_entities
        .iter()
        .map(|e| (e.path.points[0].x, e.path.points[0].y))
        .collect();
    assert_eq!(
        post_first_points, pre_first_points,
        "entity content must not change when wipe tower is present"
    );
}
