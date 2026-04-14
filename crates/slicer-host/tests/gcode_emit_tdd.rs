#![allow(missing_docs)]

//! TDD red tests for TASK-034: GCodeEmit built-in serializer.
//!
//! These tests define the contract for `DefaultGCodeEmitter` and `DefaultGCodeSerializer`
//! and must fail only on the explicit todo! stub until the green implementation is completed.
//!
//! Acceptance criteria:
//! - [x] API covers `DefaultGCodeEmitter::emit_gcode()` implementing `GCodeEmitter` trait
//! - [x] API covers `DefaultGCodeSerializer::serialize_gcode()` implementing `GCodeSerializer` trait
//! - [x] Tests lock down emit behavior (layer traversal, command generation)
//! - [x] Tests lock down serialize behavior (text formatting)
//! - [x] Tests lock down metadata accumulation
//! - [x] Tests lock down error propagation
//!
//! Reference: docs/02_ir_schemas.md (IR 11 - GCodeIR), docs/04_host_scheduler.md (lines 778-810)
//! OrcaSlicer reference: OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp

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
    assert_eq!(
        gcode_ir.commands.len(),
        3,
        "should produce 3 Move commands for 3 points"
    );

    // Verify first move has correct coordinates
    match &gcode_ir.commands[0] {
        GCodeCommand::Move { x, y, z, role, .. } => {
            assert_eq!(*x, Some(0.0));
            assert_eq!(*y, Some(0.0));
            assert_eq!(*z, Some(0.2));
            assert_eq!(*role, ExtrusionRole::OuterWall);
        }
        other => panic!("expected Move command, got {:?}", other),
    }

    // Verify second move
    match &gcode_ir.commands[1] {
        GCodeCommand::Move { x, y, z, .. } => {
            assert_eq!(*x, Some(10.0));
            assert_eq!(*y, Some(0.0));
            assert_eq!(*z, Some(0.2));
        }
        other => panic!("expected Move command, got {:?}", other),
    }

    // Verify third move
    match &gcode_ir.commands[2] {
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
    assert_eq!(gcode_ir.commands.len(), 2);

    // First command should be at z=0.2
    match &gcode_ir.commands[0] {
        GCodeCommand::Move { z, .. } => {
            assert_eq!(*z, Some(0.2), "first command should be at z=0.2");
        }
        other => panic!("expected Move command, got {:?}", other),
    }

    // Second command should be at z=0.4
    match &gcode_ir.commands[1] {
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

    // Should have: Move, Move, ToolChange, Move (4 commands)
    assert_eq!(gcode_ir.commands.len(), 4, "should produce 4 commands");

    // Commands 0 and 1 should be Move
    assert!(matches!(&gcode_ir.commands[0], GCodeCommand::Move { .. }));
    assert!(matches!(&gcode_ir.commands[1], GCodeCommand::Move { .. }));

    // Command 2 should be ToolChange
    match &gcode_ir.commands[2] {
        GCodeCommand::ToolChange { from, to } => {
            assert_eq!(*from, 0);
            assert_eq!(*to, 1);
        }
        other => panic!("expected ToolChange command at index 2, got {:?}", other),
    }

    // Command 3 should be Move
    assert!(matches!(&gcode_ir.commands[3], GCodeCommand::Move { .. }));
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

    let gcode_ir = gcode_ir_fixture(vec![GCodeCommand::ToolChange { from: 0, to: 1 }]);

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
        vec![point3_with_width(0.0, 0.0, 0.2), point3_with_width(1.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let mut layer = layer_with_entity(0, 0.2, entity);
    layer.annotations = vec![
        LayerAnnotation { after_entity_index: 0, kind: LayerAnnotationKind::Comment("hello".into()) },
        LayerAnnotation { after_entity_index: 0, kind: LayerAnnotationKind::Raw("M117 hi".into()) },
    ];

    let emitter = DefaultGCodeEmitter::new("test".into());
    let bb = blackboard_fixture();
    let ir = emitter.emit_gcode(&[layer], &bb).unwrap();

    // Find the indices of Comment and Raw — they must come AFTER all Move
    // commands for entity 0 (declaration order preserved).
    let last_move = ir.commands.iter().rposition(|c| matches!(c, GCodeCommand::Move { .. })).unwrap();
    let comment_idx = ir.commands.iter().position(|c| matches!(c, GCodeCommand::Comment { text } if text == "hello")).expect("comment emitted");
    let raw_idx = ir.commands.iter().position(|c| matches!(c, GCodeCommand::Raw { text } if text == "M117 hi")).expect("raw emitted");
    assert!(comment_idx > last_move, "comment must appear after the entity's moves");
    assert!(raw_idx > comment_idx, "raw must appear after comment (declaration order)");
}

#[test]
fn emit_preserves_tool_change_path_with_annotations_present() {
    use slicer_ir::{LayerAnnotation, LayerAnnotationKind};
    let entity = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2), point3_with_width(1.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let mut layer = layer_with_entity(0, 0.2, entity);
    layer.tool_changes = vec![ToolChange { after_entity_index: 0, from_tool: 0, to_tool: 1 }];
    layer.annotations = vec![LayerAnnotation {
        after_entity_index: 0,
        kind: LayerAnnotationKind::Comment("post-tc".into()),
    }];

    let emitter = DefaultGCodeEmitter::new("test".into());
    let bb = blackboard_fixture();
    let ir = emitter.emit_gcode(&[layer], &bb).unwrap();

    let tc_idx = ir.commands.iter().position(|c| matches!(c, GCodeCommand::ToolChange { .. })).expect("tool change emitted");
    let comment_idx = ir.commands.iter().position(|c| matches!(c, GCodeCommand::Comment { .. })).expect("comment emitted");
    assert!(tc_idx < comment_idx, "tool-change comes before comment at same anchor");
}

#[test]
fn emit_is_deterministic_with_annotations() {
    use slicer_ir::{LayerAnnotation, LayerAnnotationKind};
    let mk = || {
        let entity = print_entity_fixture(
            vec![point3_with_width(0.0, 0.0, 0.2), point3_with_width(1.0, 0.0, 0.2)],
            ExtrusionRole::OuterWall,
        );
        let mut layer = layer_with_entity(0, 0.2, entity);
        layer.annotations = vec![
            LayerAnnotation { after_entity_index: 0, kind: LayerAnnotationKind::Comment("a".into()) },
            LayerAnnotation { after_entity_index: 0, kind: LayerAnnotationKind::Raw("b".into()) },
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
    assert!(ir.commands.iter().any(|c| matches!(c, GCodeCommand::Comment { text } if text == "only")));
}
