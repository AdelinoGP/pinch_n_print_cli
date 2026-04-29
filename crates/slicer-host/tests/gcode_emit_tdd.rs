#![allow(missing_docs)]

//! TDD red tests for TASK-119 / TASK-119a / TASK-119b / TASK-119c: OrcaSlicer-compatible GCode emission contract.
//!
//! These tests define the canonical OrcaSlicer GCode comment, role-label, and serialization
//! contract for `DefaultGCodeEmitter` and `DefaultGCodeSerializer` and must fail only on the
//! explicit todo! stub until the green implementation is completed.
//!
//! Acceptance criteria (packet: 11_orca-gcode-emission-contract):
//! - [x] API covers `DefaultGCodeEmitter::emit_gcode()` implementing `GCodeEmitter` trait
//! - [x] API covers `DefaultGCodeSerializer::serialize_gcode()` implementing `GCodeSerializer` trait
//! - [x] Tests lock down emit behavior (layer traversal, command generation)
//! - [x] Tests lock down serialize behavior (text formatting)
//! - [x] Tests lock down metadata accumulation
//! - [x] Tests lock down error propagation
//! - [x] Orca-identical layer-change header emission (;LAYER_CHANGE, ;Z:, ;HEIGHT:)
//! - [x] Orca-identical role-boundary label emission (;TYPE:)
//! - [x] Seam-started wall-loop preservation on emit
//! - [x] Canonical retract/travel/Z-hop serialization order
//! - [x] Omission of absent role labels and retraction lines
//!
//! Reference: docs/02_ir_schemas.md (IR 11 - GCodeIR), docs/04_host_scheduler.md (lines 778-810)
//! OrcaSlicer reference: OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp, GCodeProcessor.hpp

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{
    Blackboard, DefaultGCodeEmitter, DefaultGCodeSerializer, GCodeEmitter, GCodeSerializer,
};
use slicer_ir::{
    BoundingBox3, ExtrusionPath3D, ExtrusionRole, GCodeCommand, GCodeIR, IndexedTriangleSet,
    LayerCollectionIR, MeshIR, ObjectConfig, ObjectId, ObjectMesh, Point3, Point3WithWidth,
    PrintEntity, PrintMetadata, RegionKey, SemVer, ToolChange, Transform3d, ZHop,
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

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, // column 0
            0.0, 1.0, 0.0, 0.0, // column 1
            0.0, 0.0, 1.0, 0.0, // column 2
            0.0, 0.0, 0.0, 1.0, // column 3
        ],
    }
}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        schema_version: semver_fixture(),
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
    }
}

fn blackboard_fixture() -> Blackboard {
    let mesh = Arc::new(mesh_fixture());
    Blackboard::new(mesh, 0)
}

fn point3_with_width(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
    }
}

fn region_key_fixture() -> RegionKey {
    RegionKey {
        global_layer_index: 0,
        object_id: ObjectId::from("test-object"),
        region_id: 1u64,
    }
}

fn print_entity_fixture(points: Vec<Point3WithWidth>, role: ExtrusionRole) -> PrintEntity {
    PrintEntity {
        path: ExtrusionPath3D {
            points,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: region_key_fixture(),
        topo_order: 0,
    }
}

fn layer_collection_fixture(index: u32, z: f32) -> LayerCollectionIR {
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

fn layer_with_entity(index: u32, z: f32, entity: PrintEntity) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver_fixture(),
        global_layer_index: index,
        z,
        ordered_entities: vec![entity],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    }
}

fn gcode_ir_fixture(commands: Vec<GCodeCommand>) -> GCodeIR {
    GCodeIR {
        schema_version: semver_fixture(),
        commands,
        metadata: PrintMetadata {
            estimated_print_time_s: 0,
            filament_used_mm: vec![0.0],
            layer_count: 0,
            slicer_version: "1.0.0-test".to_string(),
        },
    }
}

// ============================================================================
// Test 1: Empty layers emit minimal GCodeIR
// ============================================================================

#[test]
fn emit_empty_layers_produces_minimal_gcode_ir() {
    let blackboard = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
    let layer_irs: &[LayerCollectionIR] = &[];

    let result = emitter.emit_gcode(layer_irs, &blackboard);

    assert!(
        result.is_ok(),
        "emit_gcode should succeed, got {:?}",
        result
    );
    let gcode_ir = result.unwrap();

    // Empty layers should produce empty commands list
    assert!(
        gcode_ir.commands.is_empty(),
        "empty layers should produce empty commands"
    );

    // Metadata should have layer_count = 0
    assert_eq!(
        gcode_ir.metadata.layer_count, 0,
        "layer_count should be 0 for empty input"
    );

    // Slicer version should be preserved
    assert_eq!(gcode_ir.metadata.slicer_version, "1.0.0-test");
}

// ============================================================================
// Test 2: Single layer single entity produces Move commands
// ============================================================================

#[test]
fn emit_single_layer_single_entity_produces_move_commands() {
    let blackboard = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());

    // Create a 3-point path
    let points = vec![
        point3_with_width(0.0, 0.0, 0.2),
        point3_with_width(10.0, 0.0, 0.2),
        point3_with_width(10.0, 10.0, 0.2),
    ];
    let entity = print_entity_fixture(points, ExtrusionRole::OuterWall);
    let layer = layer_with_entity(0, 0.2, entity);
    let layer_irs = &[layer];

    let result = emitter.emit_gcode(layer_irs, &blackboard);

    assert!(
        result.is_ok(),
        "emit_gcode should succeed, got {:?}",
        result
    );
    let gcode_ir = result.unwrap();

    // Should have 3 Move commands (one per point)
    // Plus 4 header lines: ;LAYER_CHANGE, ;Z:0.2, ;HEIGHT:0.2, ;TYPE:Outer wall
    assert_eq!(
        gcode_ir.commands.len(),
        7,
        "should produce 7 commands (4 headers + 3 moves) for a single-entity layer"
    );

    // Verify first move has correct coordinates (index 4 = first Move after 3 header + 1 ;TYPE)
    match &gcode_ir.commands[4] {
        GCodeCommand::Move { x, y, z, role, .. } => {
            assert_eq!(*x, Some(0.0));
            assert_eq!(*y, Some(0.0));
            assert_eq!(*z, Some(0.2));
            assert_eq!(*role, ExtrusionRole::OuterWall);
        }
        other => panic!("expected Move command, got {:?}", other),
    }

    // Verify second move
    match &gcode_ir.commands[5] {
        GCodeCommand::Move { x, y, z, .. } => {
            assert_eq!(*x, Some(10.0));
            assert_eq!(*y, Some(0.0));
            assert_eq!(*z, Some(0.2));
        }
        other => panic!("expected Move command, got {:?}", other),
    }

    // Verify third move
    match &gcode_ir.commands[6] {
        GCodeCommand::Move { x, y, z, .. } => {
            assert_eq!(*x, Some(10.0));
            assert_eq!(*y, Some(10.0));
            assert_eq!(*z, Some(0.2));
        }
        other => panic!("expected Move command, got {:?}", other),
    }
}

// ============================================================================
// Test 3: Multiple layers preserve Z-order
// ============================================================================

#[test]
fn emit_multiple_layers_preserves_z_order() {
    let blackboard = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());

    // Two layers at different Z heights
    let entity1 = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let entity2 = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.4)],
        ExtrusionRole::OuterWall,
    );
    let layer1 = layer_with_entity(0, 0.2, entity1);
    let layer2 = layer_with_entity(1, 0.4, entity2);
    let layer_irs = &[layer1, layer2];

    let result = emitter.emit_gcode(layer_irs, &blackboard);

    assert!(
        result.is_ok(),
        "emit_gcode should succeed, got {:?}",
        result
    );
    let gcode_ir = result.unwrap();

    // Should have 2 Move commands
    // Layer 1: 3 header Raw + 1 ;TYPE Raw + 1 Move = 5 commands
    // Layer 2: 3 header Raw + 1 Move = 4 commands (no new ;TYPE - same role as prev layer)
    // Total = 9 commands
    assert_eq!(gcode_ir.commands.len(), 9);

    // First command should be at z=0.2 (Move at index 4 after 3 header + 1 ;TYPE for layer 1)
    match &gcode_ir.commands[4] {
        GCodeCommand::Move { z, .. } => {
            assert_eq!(*z, Some(0.2), "first command should be at z=0.2");
        }
        other => panic!("expected Move command, got {:?}", other),
    }

    // Second command should be at z=0.4 (Move at index 8 after 3 header for layer 2)
    match &gcode_ir.commands[8] {
        GCodeCommand::Move { z, .. } => {
            assert_eq!(*z, Some(0.4), "second command should be at z=0.4");
        }
        other => panic!("expected Move command, got {:?}", other),
    }
}

// ============================================================================
// Test 4: ToolChange inserted at correct position
// ============================================================================

#[test]
fn emit_tool_change_at_correct_position() {
    let blackboard = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());

    // Layer with 3 entities and a tool change after entity 1
    let entity0 = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let entity1 = print_entity_fixture(
        vec![point3_with_width(10.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let entity2 = print_entity_fixture(
        vec![point3_with_width(20.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let mut layer = layer_collection_fixture(0, 0.2);
    layer.ordered_entities = vec![entity0, entity1, entity2];
    layer.tool_changes = vec![ToolChange {
        after_entity_index: 1,
        from_tool: 0,
        to_tool: 1,
    }];

    let layer_irs = &[layer];

    let result = emitter.emit_gcode(layer_irs, &blackboard);

    assert!(
        result.is_ok(),
        "emit_gcode should succeed, got {:?}",
        result
    );
    let gcode_ir = result.unwrap();

    // Should have: 3 header + 1 ;TYPE + 3 Move + 1 ToolChange = 8 commands
    assert_eq!(gcode_ir.commands.len(), 8, "should produce 8 commands");

    // Commands 4 and 5 should be Move (after 3 header + 1 ;TYPE lines)
    assert!(matches!(&gcode_ir.commands[4], GCodeCommand::Move { .. }));
    assert!(matches!(&gcode_ir.commands[5], GCodeCommand::Move { .. }));

    // Command 6 should be ToolChange
    match &gcode_ir.commands[6] {
        GCodeCommand::ToolChange {
            after_entity_index: _,
            from,
            to,
        } => {
            assert_eq!(*from, 0);
            assert_eq!(*to, 1);
        }
        other => panic!("expected ToolChange command at index 6, got {:?}", other),
    }

    // Command 7 should be Move
    assert!(matches!(&gcode_ir.commands[7], GCodeCommand::Move { .. }));
}

// ============================================================================
// Test 5: ZHop generates travel sequence
// ============================================================================

#[test]
fn emit_zhop_generates_travel_sequence() {
    let blackboard = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());

    // Layer with 2 entities and a Z hop after entity 0
    let entity0 = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let entity1 = print_entity_fixture(
        vec![point3_with_width(10.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let mut layer = layer_collection_fixture(0, 0.2);
    layer.ordered_entities = vec![entity0, entity1];
    layer.z_hops = vec![ZHop {
        after_entity_index: 0,
        hop_height: 0.5,
    }];

    let layer_irs = &[layer];

    let result = emitter.emit_gcode(layer_irs, &blackboard);

    assert!(
        result.is_ok(),
        "emit_gcode should succeed, got {:?}",
        result
    );
    let gcode_ir = result.unwrap();

    // Should have: Move, Move(Z+0.5), Move(Z back to 0.2), Move (4 commands)
    // The Z-hop should lift to z=0.2+0.5=0.7, then return to z=0.2
    assert!(
        gcode_ir.commands.len() >= 3,
        "should produce at least 3 commands"
    );

    // Find the Z-hop moves (z > layer z)
    let z_hop_moves: Vec<_> = gcode_ir
        .commands
        .iter()
        .filter_map(|cmd| match cmd {
            GCodeCommand::Move { z: Some(z), .. } if *z > 0.25 => Some(*z),
            _ => None,
        })
        .collect();

    // Should have at least one move at the hopped height
    assert!(
        z_hop_moves.iter().any(|z| (*z - 0.7).abs() < 0.01),
        "should have a Z-hop to z=0.7, got {:?}",
        z_hop_moves
    );
}

// ============================================================================
// Test 6: PrintMetadata accumulates layer_count
// ============================================================================

#[test]
fn emit_metadata_accumulates_layer_count() {
    let blackboard = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());

    // 3 layers
    let layer_irs = &[
        layer_collection_fixture(0, 0.2),
        layer_collection_fixture(1, 0.4),
        layer_collection_fixture(2, 0.6),
    ];

    let result = emitter.emit_gcode(layer_irs, &blackboard);

    assert!(
        result.is_ok(),
        "emit_gcode should succeed, got {:?}",
        result
    );
    let gcode_ir = result.unwrap();

    assert_eq!(gcode_ir.metadata.layer_count, 3, "layer_count should be 3");
}

// ============================================================================
// Test 7: PrintMetadata accumulates filament_used_mm per tool
// ============================================================================

#[test]
fn emit_metadata_accumulates_filament_used_mm() {
    let blackboard = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());

    // Create entities with known extrusion amounts
    // E value is computed from path length * width * flow_factor
    let points = vec![
        point3_with_width(0.0, 0.0, 0.2),
        point3_with_width(10.0, 0.0, 0.2), // 10mm travel
    ];
    let entity = print_entity_fixture(points, ExtrusionRole::OuterWall);
    let layer = layer_with_entity(0, 0.2, entity);
    let layer_irs = &[layer];

    let result = emitter.emit_gcode(layer_irs, &blackboard);

    assert!(
        result.is_ok(),
        "emit_gcode should succeed, got {:?}",
        result
    );
    let gcode_ir = result.unwrap();

    // Should have at least one tool's filament usage
    assert!(
        !gcode_ir.metadata.filament_used_mm.is_empty(),
        "should track filament usage"
    );

    // Total filament used should be > 0 for extrusion moves
    let total_filament: f32 = gcode_ir.metadata.filament_used_mm.iter().sum();
    assert!(
        total_filament > 0.0,
        "filament_used_mm should be > 0 for extrusion moves"
    );
}

// ============================================================================
// Test 8: Serialize Move command
// ============================================================================

#[test]
fn serialize_move_command() {
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = gcode_ir_fixture(vec![GCodeCommand::Move {
        x: Some(10.0),
        y: Some(20.0),
        z: Some(0.2),
        e: Some(1.5),
        f: Some(1200.0),
        role: ExtrusionRole::OuterWall,
    }]);

    let result = serializer.serialize_gcode(&gcode_ir);

    assert!(
        result.is_ok(),
        "serialize_gcode should succeed, got {:?}",
        result
    );
    let text = result.unwrap();

    // Should contain G1 with X Y Z E F
    assert!(text.contains("G1"), "should contain G1 command");
    assert!(text.contains("X10"), "should contain X10");
    assert!(text.contains("Y20"), "should contain Y20");
    assert!(text.contains("Z0.2"), "should contain Z0.2");
    assert!(text.contains("E1.5"), "should contain E1.5");
    assert!(text.contains("F1200"), "should contain F1200");
}

// ============================================================================
// Test 9: Serialize FanSpeed command
// ============================================================================

#[test]
fn serialize_fan_speed_command() {
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = gcode_ir_fixture(vec![GCodeCommand::FanSpeed { value: 255 }]);

    let result = serializer.serialize_gcode(&gcode_ir);

    assert!(
        result.is_ok(),
        "serialize_gcode should succeed, got {:?}",
        result
    );
    let text = result.unwrap();

    // Should contain M106 S255
    assert!(text.contains("M106"), "should contain M106 command");
    assert!(text.contains("S255"), "should contain S255");
}

// ============================================================================
// Test 10: Serialize Temperature command (wait=false)
// ============================================================================

#[test]
fn serialize_temperature_command_no_wait() {
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = gcode_ir_fixture(vec![GCodeCommand::Temperature {
        tool: 0,
        celsius: 200.0,
        wait: false,
    }]);

    let result = serializer.serialize_gcode(&gcode_ir);

    assert!(
        result.is_ok(),
        "serialize_gcode should succeed, got {:?}",
        result
    );
    let text = result.unwrap();

    // Should contain M104 (no wait)
    assert!(
        text.contains("M104"),
        "should contain M104 command for no-wait temperature"
    );
    assert!(text.contains("T0"), "should contain T0");
    assert!(text.contains("S200"), "should contain S200");
}

// ============================================================================
// Test 11: Serialize Temperature command (wait=true)
// ============================================================================

#[test]
fn serialize_temperature_command_with_wait() {
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = gcode_ir_fixture(vec![GCodeCommand::Temperature {
        tool: 0,
        celsius: 200.0,
        wait: true,
    }]);

    let result = serializer.serialize_gcode(&gcode_ir);

    assert!(
        result.is_ok(),
        "serialize_gcode should succeed, got {:?}",
        result
    );
    let text = result.unwrap();

    // Should contain M109 (wait)
    assert!(
        text.contains("M109"),
        "should contain M109 command for wait temperature"
    );
    assert!(text.contains("T0"), "should contain T0");
    assert!(text.contains("S200"), "should contain S200");
}

// ============================================================================
// Test 12: Serialize ToolChange command
// ============================================================================

#[test]
fn serialize_tool_change_command() {
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = gcode_ir_fixture(vec![GCodeCommand::ToolChange {
        after_entity_index: 0,
        from: 0,
        to: 1,
    }]);

    let result = serializer.serialize_gcode(&gcode_ir);

    assert!(
        result.is_ok(),
        "serialize_gcode should succeed, got {:?}",
        result
    );
    let text = result.unwrap();

    // Should contain T1
    assert!(text.contains("T1"), "should contain T1 tool change command");
}

// ============================================================================
// Test 13: Serialize Comment command
// ============================================================================

#[test]
fn serialize_comment_command() {
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = gcode_ir_fixture(vec![GCodeCommand::Comment {
        text: "layer 1".to_string(),
    }]);

    let result = serializer.serialize_gcode(&gcode_ir);

    assert!(
        result.is_ok(),
        "serialize_gcode should succeed, got {:?}",
        result
    );
    let text = result.unwrap();

    // Should contain ; layer 1
    assert!(
        text.contains("; layer 1"),
        "should contain comment with semicolon prefix"
    );
}

// ============================================================================
// Test 14: Serialize Raw command
// ============================================================================

#[test]
fn serialize_raw_command() {
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = gcode_ir_fixture(vec![GCodeCommand::Raw {
        text: "G28 ; home".to_string(),
    }]);

    let result = serializer.serialize_gcode(&gcode_ir);

    assert!(
        result.is_ok(),
        "serialize_gcode should succeed, got {:?}",
        result
    );
    let text = result.unwrap();

    // Should contain raw text passthrough
    assert!(
        text.contains("G28 ; home"),
        "should contain raw text passthrough"
    );
}

// ============================================================================
// Test 15: Serialize Retract/Unretract commands
// ============================================================================

#[test]
fn serialize_retract_unretract_commands() {
    let serializer = DefaultGCodeSerializer::new();

    let gcode_ir = gcode_ir_fixture(vec![
        GCodeCommand::Retract {
            length: 0.8,
            speed: 2400.0,
        },
        GCodeCommand::Unretract {
            length: 0.8,
            speed: 2400.0,
        },
    ]);

    let result = serializer.serialize_gcode(&gcode_ir);

    assert!(
        result.is_ok(),
        "serialize_gcode should succeed, got {:?}",
        result
    );
    let text = result.unwrap();

    // Should contain G1 E-0.8 F2400 for retract
    assert!(text.contains("G1"), "should contain G1 command");
    assert!(
        text.contains("E-0.8") || text.contains("E-0.80"),
        "should contain E-0.8 for retract"
    );
    assert!(text.contains("F2400"), "should contain F2400");

    // Should also contain positive E for unretract
    assert!(
        text.contains("E0.8") || text.contains("E0.80"),
        "should contain E0.8 for unretract"
    );
}

#[test]
fn emit_inserts_comment_and_raw_annotations_after_anchor_entity() {
    use slicer_ir::{LayerAnnotation, LayerAnnotationKind};
    let entity = print_entity_fixture(
        vec![
            point3_with_width(0.0, 0.0, 0.2),
            point3_with_width(1.0, 0.0, 0.2),
        ],
        ExtrusionRole::OuterWall,
    );
    let mut layer = layer_with_entity(0, 0.2, entity);
    layer.annotations = vec![
        LayerAnnotation {
            after_entity_index: 0,
            kind: LayerAnnotationKind::Comment("hello".into()),
        },
        LayerAnnotation {
            after_entity_index: 0,
            kind: LayerAnnotationKind::Raw("M117 hi".into()),
        },
    ];

    let emitter = DefaultGCodeEmitter::new("test".into());
    let bb = blackboard_fixture();
    let ir = emitter.emit_gcode(&[layer], &bb).unwrap();

    // Find the indices of Comment and Raw — they must come AFTER all Move
    // commands for entity 0 (declaration order preserved).
    let last_move = ir
        .commands
        .iter()
        .rposition(|c| matches!(c, GCodeCommand::Move { .. }))
        .unwrap();
    let comment_idx = ir
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::Comment { text } if text == "hello"))
        .expect("comment emitted");
    let raw_idx = ir
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::Raw { text } if text == "M117 hi"))
        .expect("raw emitted");
    assert!(
        comment_idx > last_move,
        "comment must appear after the entity's moves"
    );
    assert!(
        raw_idx > comment_idx,
        "raw must appear after comment (declaration order)"
    );
}

#[test]
fn emit_preserves_tool_change_path_with_annotations_present() {
    use slicer_ir::{LayerAnnotation, LayerAnnotationKind};
    let entity = print_entity_fixture(
        vec![
            point3_with_width(0.0, 0.0, 0.2),
            point3_with_width(1.0, 0.0, 0.2),
        ],
        ExtrusionRole::OuterWall,
    );
    let mut layer = layer_with_entity(0, 0.2, entity);
    layer.tool_changes = vec![ToolChange {
        after_entity_index: 0,
        from_tool: 0,
        to_tool: 1,
    }];
    layer.annotations = vec![LayerAnnotation {
        after_entity_index: 0,
        kind: LayerAnnotationKind::Comment("post-tc".into()),
    }];

    let emitter = DefaultGCodeEmitter::new("test".into());
    let bb = blackboard_fixture();
    let ir = emitter.emit_gcode(&[layer], &bb).unwrap();

    let tc_idx = ir
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::ToolChange { .. }))
        .expect("tool change emitted");
    let comment_idx = ir
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::Comment { .. }))
        .expect("comment emitted");
    assert!(
        tc_idx < comment_idx,
        "tool-change comes before comment at same anchor"
    );
}

#[test]
fn emit_is_deterministic_with_annotations() {
    use slicer_ir::{LayerAnnotation, LayerAnnotationKind};
    let mk = || {
        let entity = print_entity_fixture(
            vec![
                point3_with_width(0.0, 0.0, 0.2),
                point3_with_width(1.0, 0.0, 0.2),
            ],
            ExtrusionRole::OuterWall,
        );
        let mut layer = layer_with_entity(0, 0.2, entity);
        layer.annotations = vec![
            LayerAnnotation {
                after_entity_index: 0,
                kind: LayerAnnotationKind::Comment("a".into()),
            },
            LayerAnnotation {
                after_entity_index: 0,
                kind: LayerAnnotationKind::Raw("b".into()),
            },
        ];
        layer
    };
    let emitter = DefaultGCodeEmitter::new("test".into());
    let bb = blackboard_fixture();
    let r1 = emitter.emit_gcode(&[mk()], &bb).unwrap();
    let r2 = emitter.emit_gcode(&[mk()], &bb).unwrap();
    assert_eq!(r1.commands.len(), r2.commands.len());
    assert_eq!(r1, r2);
}

#[test]
fn emit_emits_trailing_annotations_on_empty_layer() {
    use slicer_ir::{LayerAnnotation, LayerAnnotationKind};
    let mut layer = layer_collection_fixture(0, 0.0);
    layer.annotations = vec![LayerAnnotation {
        after_entity_index: 0,
        kind: LayerAnnotationKind::Comment("only".into()),
    }];
    let emitter = DefaultGCodeEmitter::new("test".into());
    let bb = blackboard_fixture();
    let ir = emitter.emit_gcode(&[layer], &bb).unwrap();
    assert!(ir
        .commands
        .iter()
        .any(|c| matches!(c, GCodeCommand::Comment { text } if text == "only")));
}

// ============================================================================
// Orca GCode Emission Contract Tests (TASK-119 / TASK-119a / TASK-119b)
// ============================================================================
//
// These tests define the canonical OrcaSlicer GCode emission contract.
// Each test is a TDD red stub: it will fail with a concrete assertion once
// the emitting logic is implemented, and pass once the contract is fulfilled.
//
// Acceptance criteria (packet: 11_orca-gcode-emission-contract):
// - LayerChange / Z / Height headers emitted before first move of each layer
// - Role-boundary ;TYPE: labels emitted at contiguous role transitions
// - Seam-started wall loops preserved (no travel prepended that shifts loop start)
// - Retract/unretract/travel/Z-hop in canonical Orca order
// - Absent roles and retract decisions produce no fabricated lines

#[test]
fn emits_orca_layer_headers_before_first_extrusion() {
    // Given two consecutive LayerCollectionIR entries with global_layer_index=7, z=1.4
    // and global_layer_index=8, z=1.6, when DefaultGCodeEmitter + DefaultGCodeSerializer
    // emit text, then the first layer block begins with exactly ;LAYER_CHANGE,
    // ;Z:1.4, and ;HEIGHT:0.2 in that order before the first emitted G1 line for layer 7.

    let bb = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
    let serializer = DefaultGCodeSerializer::new();

    // Two consecutive layers
    let entity7 = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 1.4)],
        ExtrusionRole::OuterWall,
    );
    let layer7 = layer_with_entity(7, 1.4, entity7);

    let entity8 = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 1.6)],
        ExtrusionRole::OuterWall,
    );
    let layer8 = layer_with_entity(8, 1.6, entity8);

    let layer_irs = &[layer7, layer8];
    let gcode_ir = emitter.emit_gcode(layer_irs, &bb).unwrap();
    let text = serializer.serialize_gcode(&gcode_ir).unwrap();

    let lines: Vec<&str> = text.lines().collect();

    // Find the first G1 line for layer 7
    let first_g1_idx = lines
        .iter()
        .position(|l| l.starts_with("G1"))
        .expect("should have a G1 line");

    // Layer-change header lines must appear BEFORE the first G1
    let header_lines = &lines[..first_g1_idx];
    let header_text = header_lines.join("\n");

    assert!(
        header_text.contains(";LAYER_CHANGE"),
        "missing ;LAYER_CHANGE before first G1 for layer 7"
    );
    assert!(
        header_text.contains(";Z:1.4"),
        "missing ;Z:1.4 before first G1 for layer 7"
    );
    assert!(
        header_text.contains(";HEIGHT:0.2"),
        "missing ;HEIGHT:0.2 (z delta 1.6 - 1.4) before first G1 for layer 7"
    );

    // Headers must appear in canonical order: ;LAYER_CHANGE, ;Z:, ;HEIGHT:
    let lc_idx = header_text.find(";LAYER_CHANGE").unwrap();
    let z_idx = header_text.find(";Z:1.4").unwrap();
    let h_idx = header_text.find(";HEIGHT:0.2").unwrap();
    assert!(
        lc_idx < z_idx && z_idx < h_idx,
        "header order must be ;LAYER_CHANGE, ;Z:, ;HEIGHT:, got: {:?}",
        header_lines
    );
}

#[test]
fn emits_orca_type_comments_at_role_boundaries() {
    // Given one emitted layer whose ordered_entities[*].path.role sequence crosses
    // OuterWall -> TopSolidInfill -> SparseInfill -> SupportMaterial -> SupportInterface
    // -> Skirt -> WipeTower, when the host serializes, then it inserts role-boundary
    // comments with exact labels at the first command of each contiguous role block
    // and never duplicates a label inside the same block.

    let bb = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
    let serializer = DefaultGCodeSerializer::new();

    // Build a layer with one entity per role, no interspersed travel
    let entity_outer = print_entity_fixture(
        vec![
            point3_with_width(0.0, 0.0, 0.2),
            point3_with_width(1.0, 0.0, 0.2),
        ],
        ExtrusionRole::OuterWall,
    );
    let entity_top = print_entity_fixture(
        vec![
            point3_with_width(1.0, 0.0, 0.2),
            point3_with_width(2.0, 0.0, 0.2),
        ],
        ExtrusionRole::TopSolidInfill,
    );
    let entity_sparse = print_entity_fixture(
        vec![
            point3_with_width(2.0, 0.0, 0.2),
            point3_with_width(3.0, 0.0, 0.2),
        ],
        ExtrusionRole::SparseInfill,
    );
    let entity_support = print_entity_fixture(
        vec![
            point3_with_width(3.0, 0.0, 0.2),
            point3_with_width(4.0, 0.0, 0.2),
        ],
        ExtrusionRole::SupportMaterial,
    );
    let entity_supp_iface = print_entity_fixture(
        vec![
            point3_with_width(4.0, 0.0, 0.2),
            point3_with_width(5.0, 0.0, 0.2),
        ],
        ExtrusionRole::SupportInterface,
    );
    let entity_skirt = print_entity_fixture(
        vec![
            point3_with_width(5.0, 0.0, 0.2),
            point3_with_width(6.0, 0.0, 0.2),
        ],
        ExtrusionRole::Skirt,
    );
    let entity_wipe = print_entity_fixture(
        vec![
            point3_with_width(6.0, 0.0, 0.2),
            point3_with_width(7.0, 0.0, 0.2),
        ],
        ExtrusionRole::PrimeTower,
    );

    let mut layer = layer_collection_fixture(0, 0.2);
    layer.ordered_entities = vec![
        entity_outer,
        entity_top,
        entity_sparse,
        entity_support,
        entity_supp_iface,
        entity_skirt,
        entity_wipe,
    ];

    let gcode_ir = emitter.emit_gcode(&[layer], &bb).unwrap();
    let text = serializer.serialize_gcode(&gcode_ir).unwrap();

    // Each role must emit exactly one ;TYPE: label (no duplicates within a contiguous block)
    assert!(
        text.contains(";TYPE:Outer wall"),
        "missing ;TYPE:Outer wall for OuterWall"
    );
    assert!(
        text.contains(";TYPE:Top surface"),
        "missing ;TYPE:Top surface for TopSolidInfill"
    );
    assert!(
        text.contains(";TYPE:Sparse infill"),
        "missing ;TYPE:Sparse infill for SparseInfill"
    );
    assert!(
        text.contains(";TYPE:Support"),
        "missing ;TYPE:Support for SupportMaterial"
    );
    assert!(
        text.contains(";TYPE:Support interface"),
        "missing ;TYPE:Support interface for SupportInterface"
    );
    assert!(
        text.contains(";TYPE:Skirt/Brim"),
        "missing ;TYPE:Skirt/Brim for Skirt"
    );
    assert!(
        text.contains(";TYPE:Prime tower"),
        "missing ;TYPE:Prime tower for PrimeTower"
    );

    // No role label should appear more than once (use line-based matching to avoid
    // substring collisions: ";TYPE:Support" is a prefix of ";TYPE:Support interface")
    let type_labels = [
        ";TYPE:Outer wall",
        ";TYPE:Top surface",
        ";TYPE:Sparse infill",
        ";TYPE:Support",
        ";TYPE:Support interface",
        ";TYPE:Skirt/Brim",
        ";TYPE:Prime tower",
    ];
    let lines: Vec<&str> = text.lines().collect();
    for label in type_labels {
        let count = lines.iter().filter(|l| l.to_string() == label).count();
        assert_eq!(
            count, 1,
            "label '{}' should appear exactly once, found {}",
            label, count
        );
    }
}

#[test]
fn preserves_seam_started_wall_loop_order_in_output() {
    // Given a wall-loop entity whose first Point3WithWidth is already the resolved
    // seam start (20.0, 10.0, 0.2), when the host serializes that wall loop, then
    // the first extruding move for that loop is emitted at X20 Y10 Z0.2 and the
    // emit path does not prepend a travel-only move that changes the loop start point.

    let bb = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
    let serializer = DefaultGCodeSerializer::new();

    // First point of the wall loop is already the seam start (20.0, 10.0, 0.2)
    let seam_x = 20.0;
    let seam_y = 10.0;
    let seam_z = 0.2;
    let points = vec![
        point3_with_width(seam_x, seam_y, seam_z), // seam start
        point3_with_width(seam_x + 1.0, seam_y, seam_z), // second point
        point3_with_width(seam_x + 1.0, seam_y + 1.0, seam_z),
    ];
    let entity = print_entity_fixture(points, ExtrusionRole::OuterWall);
    let layer = layer_with_entity(0, seam_z, entity);

    let gcode_ir = emitter.emit_gcode(&[layer], &bb).unwrap();
    let text = serializer.serialize_gcode(&gcode_ir).unwrap();

    let lines: Vec<&str> = text.lines().collect();

    // Find the first G1 line that has X and Y (extruding move)
    let first_extrude_idx = lines
        .iter()
        .position(|l| l.starts_with("G1") && (l.contains("X20") || l.contains("X 20")));
    assert!(
        first_extrude_idx.is_some(),
        "should find an extruding G1 with X20, got:\n{}",
        text
    );
    let first_extrude = lines[first_extrude_idx.unwrap()];

    // First extruding move must be AT the seam start coordinates
    assert!(
        first_extrude.contains(&format!("X{}", seam_x))
            && first_extrude.contains(&format!("Y{}", seam_y)),
        "first extruding move should be at seam start ({}, {}), got: {}",
        seam_x,
        seam_y,
        first_extrude
    );

    // No travel-only line with X20 Y10 should appear BEFORE the first extruding G1
    // (travel lines have no E, extruding lines do)
    let travel_with_seam_start: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(idx, l)| {
            let line_idx = *idx;
            l.starts_with("G1")
                && (l.contains(&format!("X{}", seam_x)) || l.contains(&format!("Y{}", seam_y)))
                && !l.contains('E')
                && line_idx < first_extrude_idx.unwrap()
        })
        .map(|(idx, _)| idx)
        .collect();

    assert!(
        travel_with_seam_start.is_empty(),
        "no travel-only move should prepend the seam-started loop, violating lines: {:?}",
        travel_with_seam_start
    );
}

#[test]
fn serializes_retract_travel_and_z_hop_in_canonical_order() {
    // Given a postpass command sequence containing Retract {length: 0.8, speed: 1800.0},
    // one travel move with no E, one Z-hop up to 0.6, one Z-hop return to 0.4, and
    // Unretract {length: 0.8, speed: 1800.0}, when the final text is serialized, then
    // it contains G1 E-0.8 F1800, hop-up G1 Z0.6, XY travel without E, hop-down G1 Z0.4,
    // and G1 E0.8 F1800 in that exact order.

    let serializer = DefaultGCodeSerializer::new();

    // Build a GCodeIR with the Orca-canonical order: retract -> hop-up (Z-only,
    // no XY) -> travel (XY, no E) -> hop-down (Z-only) -> unretract
    let commands = vec![
        GCodeCommand::Retract {
            length: 0.8,
            speed: 1800.0,
        },
        GCodeCommand::Move {
            x: None,
            y: None,
            z: Some(0.6),
            e: None,
            f: None,
            role: ExtrusionRole::Custom("Travel".to_string()),
        },
        GCodeCommand::Move {
            x: Some(50.0),
            y: Some(50.0),
            z: Some(0.4),
            e: None,
            f: None,
            role: ExtrusionRole::Custom("Travel".to_string()),
        },
        GCodeCommand::Move {
            x: None,
            y: None,
            z: Some(0.4),
            e: None,
            f: None,
            role: ExtrusionRole::Custom("Travel".to_string()),
        },
        GCodeCommand::Unretract {
            length: 0.8,
            speed: 1800.0,
        },
    ];

    let gcode_ir = gcode_ir_fixture(commands);
    let text = serializer.serialize_gcode(&gcode_ir).unwrap();
    let lines: Vec<&str> = text.lines().collect();

    // Find indices of key lines
    let retract_idx = lines
        .iter()
        .position(|l| l.contains("E-0.8"))
        .expect("should have retract E-0.8");
    let hop_up_idx = lines
        .iter()
        .position(|l| l.contains("Z0.6") && !l.contains("X"))
        .expect("should have hop-up Z0.6");
    let travel_idx = lines
        .iter()
        .position(|l| l.contains("X50") || l.contains("X 50"))
        .expect("should have XY travel");
    let hop_down_idx = lines
        .iter()
        .position(|l| l.contains("Z0.4") && !l.contains("X"))
        .expect("should have hop-down Z0.4");
    let unretract_idx = lines
        .iter()
        .position(|l| l.contains("E0.8"))
        .expect("should have unretract E0.8");

    // Canonical order: retract BEFORE hop-up, hop-up BEFORE travel-without-E,
    // travel BEFORE hop-down, hop-down BEFORE unretract
    assert!(retract_idx < hop_up_idx, "retract must come before hop-up");
    assert!(hop_up_idx < travel_idx, "hop-up must come before travel");
    assert!(
        travel_idx < hop_down_idx,
        "travel must come before hop-down"
    );
    assert!(
        hop_down_idx < unretract_idx,
        "hop-down must come before unretract"
    );
}

#[test]
fn omits_absent_role_labels_and_retraction_lines() {
    // Given a layer whose entities contain only OuterWall and SparseInfill roles
    // and whose postpass queue contains no retracts, unretracts, or support entities,
    // when the host serializes, then the output contains no ;TYPE:Support,
    // no ;TYPE:Support interface, no ;TYPE:Skirt/Brim, no ;TYPE:Prime tower,
    // and no retract line matching G1 E-.

    let bb = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
    let serializer = DefaultGCodeSerializer::new();

    // Only OuterWall + SparseInfill — no support, skirt, wipe/prime tower
    let entity_outer = print_entity_fixture(
        vec![
            point3_with_width(0.0, 0.0, 0.2),
            point3_with_width(1.0, 0.0, 0.2),
        ],
        ExtrusionRole::OuterWall,
    );
    let entity_sparse = print_entity_fixture(
        vec![
            point3_with_width(1.0, 0.0, 0.2),
            point3_with_width(2.0, 0.0, 0.2),
        ],
        ExtrusionRole::SparseInfill,
    );

    let mut layer = layer_collection_fixture(0, 0.2);
    layer.ordered_entities = vec![entity_outer, entity_sparse];
    // No tool_changes, no z_hops — those are the only source of retract/unretract

    let gcode_ir = emitter.emit_gcode(&[layer], &bb).unwrap();
    let text = serializer.serialize_gcode(&gcode_ir).unwrap();

    // Absent roles must not appear
    assert!(
        !text.contains(";TYPE:Support"),
        "must not fabricate ;TYPE:Support when no SupportMaterial entity exists"
    );
    assert!(
        !text.contains(";TYPE:Support interface"),
        "must not fabricate ;TYPE:Support interface"
    );
    assert!(
        !text.contains(";TYPE:Skirt"),
        "must not fabricate ;TYPE:Skirt/Brim"
    );
    assert!(
        !text.contains(";TYPE:Prime tower"),
        "must not fabricate ;TYPE:Prime tower"
    );
    assert!(
        !text.contains(";TYPE:Wipe"),
        "must not fabricate ;TYPE:Wipe tower"
    );

    // No retract lines when no retract was queued
    assert!(
        !text.contains("E-"),
        "must not emit retract lines when no retract was queued"
    );
}
