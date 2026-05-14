#![allow(missing_docs)]

//! TDD tests for packet 54: skirt/brim G-code emission.
//!
//! Verifies that `SkirtBrim::process` prepends `";TYPE:Skirt/Brim"` type-comment
//! blocks before model entities, that the loop count and disabled-path are
//! exercised, and that brim paths also appear before outer-wall entities.

use std::collections::HashMap;
use std::sync::Arc;

use skirt_brim::SkirtBrim;
use slicer_host::{
    Blackboard, DefaultGCodeEmitter, DefaultGCodeSerializer, GCodeEmitter, GCodeSerializer,
};
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, IndexedTriangleSet,
    LayerCollectionIR, MeshIR, ObjectConfig, ObjectMesh, Point3, Point3WithWidth, PrintEntity,
    RegionKey, SemVer, Transform3d,
};

// ============================================================================
// Fixtures
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
            id: "test-object".to_string(),
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
        object_id: "test-object".to_string(),
        region_id: 1,
    }
}

fn pt(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
    }
}

/// A minimal outer-wall entity so the bbox is non-empty.
fn outer_wall_entity() -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![pt(5.0, 5.0, 0.2), pt(15.0, 5.0, 0.2)],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: region_key(),
        topo_order: 0,
    }
}

fn make_layer(entities: Vec<PrintEntity>) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: entities,
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    }
}

fn build_config(enabled: bool, skirt_loops: u32, brim_width: f32) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("skirt_brim_enabled".to_string(), ConfigValue::Bool(enabled));
    fields.insert(
        "skirt_loops".to_string(),
        ConfigValue::Int(skirt_loops as i64),
    );
    fields.insert("skirt_distance".to_string(), ConfigValue::Float(3.0));
    fields.insert("skirt_height".to_string(), ConfigValue::Int(1));
    fields.insert(
        "brim_width".to_string(),
        ConfigValue::Float(brim_width as f64),
    );
    fields.insert("line_width".to_string(), ConfigValue::Float(0.4));
    ConfigView::from_map(fields)
}

fn emit_gcode(layers: Vec<LayerCollectionIR>) -> String {
    let bb = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("test".to_string());
    let serializer = DefaultGCodeSerializer::new();
    let gcode_ir = emitter
        .emit_gcode(&layers, &bb)
        .expect("emit_gcode must succeed");
    serializer
        .serialize_gcode(&gcode_ir)
        .expect("serialize_gcode must succeed")
}

// ============================================================================
// Tests
// ============================================================================

/// AC: skirt block appears before the first model entity type comment.
#[test]
fn skirt_block_before_model() {
    let cfg = build_config(true, 1, 0.0);
    let skirt_brim = SkirtBrim::from_config(&cfg).expect("from_config must succeed");

    let mut layers = vec![make_layer(vec![outer_wall_entity()])];
    skirt_brim
        .process(&mut layers)
        .expect("process must succeed");

    let gcode = emit_gcode(layers);

    let skirt_pos = gcode
        .find(";TYPE:Skirt/Brim")
        .expect("must find ';TYPE:Skirt/Brim' in G-code output");

    // Find first outer-wall or inner-wall type comment
    let model_pos = gcode
        .find(";TYPE:Outer wall")
        .or_else(|| gcode.find(";TYPE:Inner wall"))
        .expect("must find a model wall type comment in G-code output");

    assert!(
        skirt_pos < model_pos,
        "';TYPE:Skirt/Brim' (pos {}) must appear before first model wall type (pos {})\nG-code:\n{}",
        skirt_pos,
        model_pos,
        gcode
    );
}

/// AC: when skirt_brim_enabled=false, no ";TYPE:Skirt/Brim" is emitted.
#[test]
fn skirt_disabled_emits_nothing() {
    let cfg = build_config(false, 1, 0.0);
    let skirt_brim = SkirtBrim::from_config(&cfg).expect("from_config must succeed");

    let mut layers = vec![make_layer(vec![outer_wall_entity()])];
    skirt_brim
        .process(&mut layers)
        .expect("process must succeed");

    let gcode = emit_gcode(layers);

    assert!(
        !gcode.contains(";TYPE:Skirt/Brim"),
        "when disabled, G-code must contain zero ';TYPE:Skirt/Brim' blocks\nG-code:\n{}",
        gcode
    );
}

/// AC: skirt_loops=3 inserts 3 skirt entities (verified via entity count in the layer).
///
/// The G-code emitter merges consecutive same-role entities under a single
/// ";TYPE:Skirt/Brim" block, so we count entities at the IR level — not type
/// comments — to confirm the loop count was honored.
#[test]
fn skirt_loops_count_honored() {
    let cfg = build_config(true, 3, 0.0);
    let skirt_brim = SkirtBrim::from_config(&cfg).expect("from_config must succeed");

    let mut layers = vec![make_layer(vec![outer_wall_entity()])];
    skirt_brim
        .process(&mut layers)
        .expect("process must succeed");

    // Count how many entities carry ExtrusionRole::Skirt (one per loop).
    let skirt_entity_count = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| matches!(e.role, ExtrusionRole::Skirt))
        .count();

    assert!(
        skirt_entity_count >= 3,
        "expected at least 3 Skirt entities for skirt_loops=3, got {}",
        skirt_entity_count
    );

    // Also confirm the G-code does include the type block at all.
    let gcode = emit_gcode(layers);
    assert!(
        gcode.contains(";TYPE:Skirt/Brim"),
        "G-code must contain ';TYPE:Skirt/Brim'\nG-code:\n{}",
        gcode
    );
}

/// AC: brim block appears before model outer-wall type comment.
#[test]
fn brim_block_before_model() {
    let cfg = build_config(true, 0, 4.0);
    let skirt_brim = SkirtBrim::from_config(&cfg).expect("from_config must succeed");

    let mut layers = vec![make_layer(vec![outer_wall_entity()])];
    skirt_brim
        .process(&mut layers)
        .expect("process must succeed");

    let gcode = emit_gcode(layers);

    let skirt_pos = gcode
        .find(";TYPE:Skirt/Brim")
        .expect("must find ';TYPE:Skirt/Brim' (brim) in G-code output");

    let model_pos = gcode
        .find(";TYPE:Outer wall")
        .expect("must find ';TYPE:Outer wall' in G-code output");

    assert!(
        skirt_pos < model_pos,
        "brim ';TYPE:Skirt/Brim' (pos {}) must appear before ';TYPE:Outer wall' (pos {})\nG-code:\n{}",
        skirt_pos,
        model_pos,
        gcode
    );
}

/// Negative: given a G-code string with zero ";TYPE:Skirt/Brim", assert the
/// absence is detectable — validates that our checking logic works.
#[test]
fn rejects_no_skirt_when_required() {
    // Build a G-code output WITHOUT running SkirtBrim::process (no skirt injected).
    let layers = vec![make_layer(vec![outer_wall_entity()])];
    let gcode = emit_gcode(layers);

    // There must be zero ";TYPE:Skirt/Brim" lines in the raw model output.
    let count = gcode.matches(";TYPE:Skirt/Brim").count();
    assert!(
        count == 0,
        "a raw model G-code (no skirt module run) must contain zero ';TYPE:Skirt/Brim' blocks; \
         found {} — the negative detection logic would be broken\nG-code:\n{}",
        count,
        gcode
    );
}
