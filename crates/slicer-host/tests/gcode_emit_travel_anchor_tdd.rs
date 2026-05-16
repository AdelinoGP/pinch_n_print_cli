#![allow(missing_docs)]

//! TDD red tests for packet `39_stable-entity-ids` — gcode emit travel anchor resolution.
//!
//! These tests are EXPECTED to fail to compile until Steps 2 and 5 land:
//!   - Step 2 adds `entity_id: u64` to `PrintEntity` and replaces `TravelMove.after_entity_index`
//!     with `TravelMove.entity_id: u64`.
//!   - Step 5 migrates `gcode_emit.rs` to resolve travel anchors via a per-layer
//!     `HashMap<u64, usize>` instead of using the positional index.
//!
//! Acceptance criteria exercised:
//!   - AC-3: TravelMove emits a G0 line whose X/Y destination matches the fixture coords.
//!   - AC-4: Reordering ordered_entities does not break travel resolution — the anchor is
//!     entity_id-based, not positional.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{
    Blackboard, DefaultGCodeEmitter, DefaultGCodeSerializer, GCodeEmitter, GCodeSerializer,
};
use slicer_ir::{
    BoundingBox3, ExtrusionPath3D, ExtrusionRole, IndexedTriangleSet, LayerCollectionIR, MeshIR,
    ObjectConfig, ObjectId, ObjectMesh, Point3, Point3WithWidth, PrintEntity, RegionKey, SemVer,
    Transform3d, TravelMove,
};

// ============================================================================
// Helper fixtures (same style as gcode_emit_tdd.rs)
// ============================================================================

fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn blackboard_fixture() -> Blackboard {
    let mesh = Arc::new(MeshIR {
        schema_version: semver(),
        objects: vec![ObjectMesh {
            id: ObjectId::from("test-object"),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 5.0,
                        y: 10.0,
                        z: 0.0,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 220.0,
                y: 220.0,
                z: 250.0,
            },
        },
    });
    Blackboard::new(mesh, 0)
}

fn region_key() -> RegionKey {
    RegionKey {
        global_layer_index: 0,
        object_id: ObjectId::from("test-object"),
        region_id: 1u64,
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
    }
}

/// Build a PrintEntity with a two-point extrusion path from (x_start, y) to (x_end, y) at z.
fn make_entity(entity_id: u64, x_start: f32, x_end: f32, y: f32, z: f32) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points: vec![point(x_start, y, z), point(x_end, y, z)],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
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

/// Parse an f32 from a G-code parameter token like "X20" or "X20.000".
fn parse_gcode_param(line: &str, param: char) -> Option<f32> {
    // Find the param character in the line, then parse the number that follows.
    let needle = param.to_string();
    let start = line.find(&needle)?;
    // Skip the param character itself
    let after = &line[start + 1..];
    // Extract digits, sign, and decimal point
    let end = after.find([' ', '\t', ';']).unwrap_or(after.len());
    after[..end].parse::<f32>().ok()
}

// ============================================================================
// Test 1: travel_emitted_at_entity_id_endpoints
// ============================================================================

#[test]
fn travel_emitted_at_entity_id_endpoints() {
    // Entity A: line from (0, 0) to (10, 0) at z=0.2  — entity_id = 1
    // Entity B: line from (20, 5) to (30, 5) at z=0.2 — entity_id = 2
    // TravelMove: anchored to entity A (entity_id = 1), destination X=20.0, Y=5.0
    //
    // After emit + serialize, find the G0 line that follows entity A.
    // Assert X ≈ 20.0, Y ≈ 5.0.

    let entity_a = make_entity(1, 0.0, 10.0, 0.0, 0.2);
    let entity_b = make_entity(2, 20.0, 30.0, 5.0, 0.2);

    let travel = TravelMove {
        entity_id: 1, // anchored to entity A
        x: Some(20.0),
        y: Some(5.0),
        z: None,
        f: None,
    };

    let layer = make_layer(vec![entity_a, entity_b], vec![travel]);

    let bb = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("test".to_string());
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = emitter
        .emit_gcode(&[layer], &bb)
        .expect("emit_gcode must succeed");
    let text = serializer
        .serialize_gcode(&gcode_ir)
        .expect("serialize_gcode must succeed");

    // Find all G0 lines (travel moves are emitted as G0 in the serializer)
    let lines: Vec<&str> = text.lines().collect();
    let g0_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("G0"))
        .cloned()
        .collect();

    assert!(
        !g0_lines.is_empty(),
        "expected at least one G0 travel line after entity A; full gcode:\n{}",
        text
    );

    // The first G0 line must have X ≈ 20.0 and Y ≈ 5.0
    let g0 = g0_lines[0];
    let x = parse_gcode_param(g0, 'X')
        .unwrap_or_else(|| panic!("G0 line has no X param: '{}'\nfull gcode:\n{}", g0, text));
    let y = parse_gcode_param(g0, 'Y')
        .unwrap_or_else(|| panic!("G0 line has no Y param: '{}'\nfull gcode:\n{}", g0, text));

    assert!(
        (x - 20.0).abs() < 0.01,
        "G0 X should be ≈ 20.0 (start of entity B), got {} in '{}'\nfull gcode:\n{}",
        x,
        g0,
        text
    );
    assert!(
        (y - 5.0).abs() < 0.01,
        "G0 Y should be ≈ 5.0 (start of entity B), got {} in '{}'\nfull gcode:\n{}",
        y,
        g0,
        text
    );
}

// ============================================================================
// Test 2: travel_survives_entity_reorder
// ============================================================================

#[test]
fn travel_survives_entity_reorder() {
    // Entity A: (0, 0) → (10, 0), entity_id = 1
    // Entity B: (20, 5) → (30, 5), entity_id = 2
    // Entity C: (40, 10) → (50, 10), entity_id = 3
    //
    // TravelMove anchored to entity C (entity_id = 3), destination X=0.0, Y=0.0.
    //
    // Then reorder ordered_entities to [C, A, B] via rotate_left(2).
    // Run gcode_emit. The G0 after entity C in the new order must still resolve
    // correctly to the TravelMove's destination (X=0.0, Y=0.0), proving the anchor
    // is index-independent (it follows entity C regardless of C's position).

    let entity_a = make_entity(1, 0.0, 10.0, 0.0, 0.2);
    let entity_b = make_entity(2, 20.0, 30.0, 5.0, 0.2);
    let entity_c = make_entity(3, 40.0, 50.0, 10.0, 0.2);

    // TravelMove anchored to entity C (entity_id=3), destination X=0.0, Y=0.0
    let travel = TravelMove {
        entity_id: 3, // anchored to entity C by stable ID
        x: Some(0.0),
        y: Some(0.0),
        z: None,
        f: None,
    };

    // Start with [A, B, C], then rotate_left(2) → [C, A, B]
    let mut entities = vec![entity_a, entity_b, entity_c];
    entities.rotate_left(2); // now [C, A, B]

    let layer = make_layer(entities, vec![travel]);

    // Verify reorder happened correctly
    assert_eq!(
        layer.ordered_entities[0].entity_id, 3,
        "first entity after rotate must be C (entity_id=3)"
    );
    assert_eq!(
        layer.ordered_entities[1].entity_id, 1,
        "second entity after rotate must be A (entity_id=1)"
    );
    assert_eq!(
        layer.ordered_entities[2].entity_id, 2,
        "third entity after rotate must be B (entity_id=2)"
    );

    let bb = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("test".to_string());
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = emitter
        .emit_gcode(&[layer], &bb)
        .expect("emit_gcode must succeed");
    let text = serializer
        .serialize_gcode(&gcode_ir)
        .expect("serialize_gcode must succeed");

    // Find all G0 lines
    let lines: Vec<&str> = text.lines().collect();
    let g0_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("G0"))
        .cloned()
        .collect();

    assert!(
        !g0_lines.is_empty(),
        "expected at least one G0 travel line; the entity_id anchor must resolve for entity C \
         even after reorder; full gcode:\n{}",
        text
    );

    // The G0 must have X ≈ 0.0 and Y ≈ 0.0 (the TravelMove destination)
    let g0 = g0_lines[0];
    let x = parse_gcode_param(g0, 'X')
        .unwrap_or_else(|| panic!("G0 line has no X param: '{}'\nfull gcode:\n{}", g0, text));
    let y = parse_gcode_param(g0, 'Y')
        .unwrap_or_else(|| panic!("G0 line has no Y param: '{}'\nfull gcode:\n{}", g0, text));

    assert!(
        (x - 0.0).abs() < 0.01,
        "G0 X should be ≈ 0.0 (TravelMove destination), got {} in '{}'\nfull gcode:\n{}",
        x,
        g0,
        text
    );
    assert!(
        (y - 0.0).abs() < 0.01,
        "G0 Y should be ≈ 0.0 (TravelMove destination), got {} in '{}'\nfull gcode:\n{}",
        y,
        g0,
        text
    );
}
