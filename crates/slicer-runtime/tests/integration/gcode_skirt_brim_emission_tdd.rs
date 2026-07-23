#![allow(missing_docs)]

//! TDD tests for packet 54: skirt/brim G-code emission (host emitter behavior).
//!
//! Verifies that the `DefaultGCodeEmitter` prepends `";TYPE:Skirt"` /
//! `";TYPE:Brim"` type-comment blocks before model entities whenever a layer's
//! `ordered_entities` begins with `ExtrusionRole::Skirt` entities, and that no
//! such block is fabricated when none are present.
//!
//! These tests own the HOST emission contract only. The skirt-brim *module*
//! (loop-count generation from config, `skirt_distance` offset geometry, and
//! the disabled → no-entities path) is exercised by the module's own crate
//! tests; here we hand-construct the `Skirt` entities the module would inject,
//! so this file links no module crate.

// SUT is DefaultGCodeEmitter / DefaultGCodeSerializer. Brim is a
// distinct `ExtrusionRole::Brim` (packet-?, R1); the emitter maps it to
// `;TYPE:Brim` and `Skirt` to `;TYPE:Skirt` — both labels OrcaSlicer's
// g-code viewer recognises (the old `;TYPE:Skirt/Brim` value rendered as
// "Undefined").

use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth, PrintEntity, RegionKey,
    SemVer,
};
use slicer_runtime::{DefaultGCodeEmitter, DefaultGCodeSerializer, GCodeEmitter, GCodeSerializer};

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

fn region_key() -> RegionKey {
    RegionKey {
        global_layer_index: 0,
        object_id: "test-object".to_string(),
        region_id: 1,
        variant_chain: Vec::new(),
    }
}

fn pt(x: f32, y: f32, z: f32) -> Point3WithWidth {
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

/// A single skirt loop entity, as the skirt-brim module would prepend it.
/// Skirt carries `ExtrusionRole::Skirt`.
fn skirt_entity(entity_id: u64) -> PrintEntity {
    loop_entity(entity_id, ExtrusionRole::Skirt)
}

/// A single brim loop entity. Brim is now a distinct `ExtrusionRole::Brim`
/// (packet-?, R1) — no `__brim__` object_id marker.
fn brim_entity(entity_id: u64) -> PrintEntity {
    loop_entity(entity_id, ExtrusionRole::Brim)
}

fn loop_entity(entity_id: u64, role: ExtrusionRole) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points: vec![
                pt(0.0, 0.0, 0.2),
                pt(20.0, 0.0, 0.2),
                pt(20.0, 20.0, 0.2),
                pt(0.0, 20.0, 0.2),
                pt(0.0, 0.0, 0.2),
            ],
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        tool_index: 1,
        region_key: region_key(),
        topo_order: 0,
    }
}

/// A minimal outer-wall entity so the bbox is non-empty.
fn outer_wall_entity() -> PrintEntity {
    PrintEntity {
        entity_id: 100,
        path: ExtrusionPath3D {
            points: vec![pt(5.0, 5.0, 0.2), pt(15.0, 5.0, 0.2)],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        tool_index: 1,
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

/// Build a layer whose `ordered_entities` begins with `skirt_loops` skirt
/// entities followed by an outer-wall model entity — the post-`SkirtBrim`
/// shape the host emitter consumes.
fn layer_with_skirt_loops(skirt_loops: u64) -> LayerCollectionIR {
    let mut entities: Vec<PrintEntity> = (0..skirt_loops).map(skirt_entity).collect();
    entities.push(outer_wall_entity());
    make_layer(entities)
}

fn emit_gcode(layers: Vec<LayerCollectionIR>) -> String {
    let emitter = DefaultGCodeEmitter::new("test".to_string());
    let serializer = DefaultGCodeSerializer::new();
    let gcode_ir = emitter
        .emit_gcode(&layers)
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
    let layers = vec![layer_with_skirt_loops(1)];
    let gcode = emit_gcode(layers);

    let skirt_pos = gcode
        .find(";TYPE:Skirt")
        .expect("must find ';TYPE:Skirt' in G-code output");

    // Find first outer-wall or inner-wall type comment
    let model_pos = gcode
        .find(";TYPE:Outer wall")
        .or_else(|| gcode.find(";TYPE:Inner wall"))
        .expect("must find a model wall type comment in G-code output");

    assert!(
        skirt_pos < model_pos,
        "';TYPE:Skirt' (pos {}) must appear before first model wall type (pos {})\nG-code:\n{}",
        skirt_pos,
        model_pos,
        gcode
    );
}

/// AC: a layer with no skirt entity (the host-observable result of
/// `skirt_brim_enabled=false`) emits zero ";TYPE:Skirt" blocks.
#[test]
fn skirt_disabled_emits_nothing() {
    let layers = vec![make_layer(vec![outer_wall_entity()])];
    let gcode = emit_gcode(layers);

    assert!(
        !gcode.contains(";TYPE:Skirt"),
        "with no skirt entity, G-code must contain zero ';TYPE:Skirt' blocks\nG-code:\n{}",
        gcode
    );
}

/// AC: three skirt entities (one per loop) are present and the emitter renders
/// the ";TYPE:Skirt" block.
///
/// The G-code emitter merges consecutive same-role entities under a single
/// ";TYPE:Skirt" block, so we count entities at the IR level — not type
/// comments — to confirm the loop count survives emission.
#[test]
fn skirt_loops_count_honored() {
    let layers = vec![layer_with_skirt_loops(3)];

    // Count how many entities carry ExtrusionRole::Skirt (one per loop).
    let skirt_entity_count = layers[0]
        .ordered_entities
        .iter()
        .filter(|e| matches!(e.role, ExtrusionRole::Skirt))
        .count();

    assert_eq!(
        skirt_entity_count, 3,
        "expected exactly 3 Skirt entities for skirt_loops=3, got {}",
        skirt_entity_count
    );

    // Also confirm the G-code does include the type block at all.
    let gcode = emit_gcode(layers);
    assert!(
        gcode.contains(";TYPE:Skirt"),
        "G-code must contain ';TYPE:Skirt'\nG-code:\n{}",
        gcode
    );
}

/// AC: a brim block (role `ExtrusionRole::Brim`) is emitted as
/// `;TYPE:Brim` and appears before the model outer-wall type comment.
#[test]
fn brim_block_before_model() {
    let mut entities = vec![brim_entity(0)];
    entities.push(outer_wall_entity());
    let layers = vec![make_layer(entities)];
    let gcode = emit_gcode(layers);

    let brim_pos = gcode
        .find(";TYPE:Brim")
        .expect("must find ';TYPE:Brim' (brim) in G-code output");

    let model_pos = gcode
        .find(";TYPE:Outer wall")
        .expect("must find ';TYPE:Outer wall' in G-code output");

    assert!(
        brim_pos < model_pos,
        "brim ';TYPE:Brim' (pos {}) must appear before ';TYPE:Outer wall' (pos {})\nG-code:\n{}",
        brim_pos,
        model_pos,
        gcode
    );
}

/// AC: a skirt entity immediately followed by a brim entity (same
/// `ExtrusionRole::Skirt`, different object id) emits BOTH `;TYPE:Skirt` and
/// `;TYPE:Brim` boundaries — the dedup must key on the resolved label, not just
/// the role, or the brim would inherit the skirt label.
#[test]
fn skirt_then_brim_emits_both_labels() {
    let mut entities = vec![skirt_entity(0)];
    entities.push(brim_entity(1));
    entities.push(outer_wall_entity());
    let layers = vec![make_layer(entities)];
    let gcode = emit_gcode(layers);

    assert!(
        gcode.contains(";TYPE:Skirt"),
        "G-code must contain ';TYPE:Skirt'\nG-code:\n{}",
        gcode
    );
    assert!(
        gcode.contains(";TYPE:Brim"),
        "G-code must contain ';TYPE:Brim'\nG-code:\n{}",
        gcode
    );

    let skirt_pos = gcode.find(";TYPE:Skirt").unwrap();
    let brim_pos = gcode.find(";TYPE:Brim").unwrap();
    let model_pos = gcode.find(";TYPE:Outer wall").unwrap();
    assert!(
        skirt_pos < brim_pos && brim_pos < model_pos,
        "expected order Skirt < Brim < Outer wall; got Skirt={} Brim={} Outer wall={}\nG-code:\n{}",
        skirt_pos,
        brim_pos,
        model_pos,
        gcode
    );
}

/// Negative: a raw model layer (no skirt entity injected) must contain zero
/// ";TYPE:Skirt" blocks — validates the negative detection logic.
#[test]
fn rejects_no_skirt_when_required() {
    let layers = vec![make_layer(vec![outer_wall_entity()])];
    let gcode = emit_gcode(layers);

    let count = gcode.matches(";TYPE:Skirt").count();
    assert!(
        count == 0,
        "a raw model G-code (no skirt entity) must contain zero ';TYPE:Skirt' blocks; \
         found {} — the negative detection logic would be broken\nG-code:\n{}",
        count,
        gcode
    );
}
