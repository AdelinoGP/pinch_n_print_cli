//! TDD: owned emission (`GCodeEmitter::emit_gcode_owned`) is emission-identical
//! to borrowed emission, and cross-layer tool rotation still does exactly what
//! it did on the borrowed path.
//!
//! Wayfinder perf-vs-orca ticket 90 (`PostPass` collection ownership). The
//! candidate moves the finalized `Vec<LayerCollectionIR>` into the emitter
//! instead of cloning it, so the emitter's working copy *is* the caller's
//! collection. These tests pin the two properties that ownership must not
//! change:
//!
//! 1. `emit_gcode_owned(layers)` returns the same `GCodeIR` and the same
//!    serialized text as `emit_gcode(&layers)` for a fixture that needs
//!    rotation, a fixture that does not, and an empty fixture.
//! 2. Rotation still reorders a layer's leading clusters to match the previous
//!    layer's ending tool (and only then): the rotated order is visible in the
//!    emitted move X coordinates and in the `T<n>` commands.
//!
//! A single-tool job is the negative control: it must emit **no** tool change
//! and must keep the authored entity order.

#![allow(missing_docs)]

use slicer_gcode::{DefaultGCodeEmitter, DefaultGCodeSerializer, GCodeEmitter, GCodeSerializer};
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, GCodeCommand, LayerCollectionIR, ObjectId, Point3WithWidth,
    PrintEntity, RegionKey,
};
use slicer_sdk::test_support::fixtures::{extrusion_path3d_base, print_entity_base};

fn pt(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        ..Default::default()
    }
}

/// One entity at a distinctive X so the emitted move order is readable.
fn entity(entity_id: u64, x: f32, role: ExtrusionRole, tool: u32) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points: vec![pt(x, 20.0, 0.2), pt(x + 1.0, 20.0, 0.2)],
            ..extrusion_path3d_base(role.clone())
        },
        tool_index: tool,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: ObjectId::from("cube"),
            region_id: tool as u64,
            variant_chain: Vec::new(),
        },
        ..print_entity_base(role)
    }
}

fn layer(global_layer_index: u32, z: f32, entities: Vec<PrintEntity>) -> LayerCollectionIR {
    LayerCollectionIR {
        global_layer_index,
        z,
        ordered_entities: entities,
        ..Default::default()
    }
}

fn emitter() -> DefaultGCodeEmitter {
    DefaultGCodeEmitter::new("t90-test".to_string())
}

fn serialize(ir: &slicer_ir::GCodeIR) -> String {
    DefaultGCodeSerializer::new()
        .serialize_gcode(ir)
        .expect("serialization")
}

/// The X coordinates of extruding moves, in emission order. The first point of
/// an entity is a positioning move with no E value, so each entity contributes
/// exactly one entry: its second point's X (`authored_x + 1.0`).
fn move_xs(ir: &slicer_ir::GCodeIR) -> Vec<f32> {
    ir.commands
        .iter()
        .filter_map(|c| match c {
            GCodeCommand::Move {
                x: Some(x),
                e: Some(_),
                ..
            } => Some(*x),
            _ => None,
        })
        .collect()
}

fn tool_changes(ir: &slicer_ir::GCodeIR) -> Vec<(u32, u32)> {
    ir.commands
        .iter()
        .filter_map(|c| match c {
            GCodeCommand::ToolChange { from, to, .. } => Some((*from, *to)),
            _ => None,
        })
        .collect()
}

/// Layer 0 prints with tool 1 only; layer 1 authors [tool 0 @ x=30, tool 1 @
/// x=40], so the borrowed path rotates the trailing tool-1 cluster to the front
/// (layer 1 then starts on tool 1, the previous layer's ending tool).
fn rotation_fixture() -> Vec<LayerCollectionIR> {
    vec![
        layer(0, 0.2, vec![entity(1, 10.0, ExtrusionRole::OuterWall, 1)]),
        layer(
            1,
            0.4,
            vec![
                entity(2, 30.0, ExtrusionRole::InnerWall, 0),
                entity(3, 40.0, ExtrusionRole::OuterWall, 1),
            ],
        ),
    ]
}

#[test]
fn owned_emission_matches_borrowed_emission_on_a_rotation_fixture() {
    let layers = rotation_fixture();
    let borrowed = emitter().emit_gcode(&layers).expect("borrowed emission");
    let owned = emitter()
        .emit_gcode_owned(layers.clone())
        .expect("owned emission");

    assert_eq!(
        owned, borrowed,
        "owned emission must be IR-identical to borrowed emission"
    );
    assert_eq!(
        serialize(&owned),
        serialize(&borrowed),
        "owned emission must serialize byte-identically to borrowed emission"
    );
}

#[test]
fn owned_emission_matches_borrowed_emission_on_single_tool_and_empty_fixtures() {
    let single_tool = vec![
        layer(
            0,
            0.2,
            vec![
                entity(1, 10.0, ExtrusionRole::OuterWall, 0),
                entity(2, 20.0, ExtrusionRole::InnerWall, 0),
            ],
        ),
        layer(1, 0.4, vec![entity(3, 30.0, ExtrusionRole::OuterWall, 0)]),
    ];
    let borrowed = emitter().emit_gcode(&single_tool).expect("borrowed");
    let owned = emitter()
        .emit_gcode_owned(single_tool.clone())
        .expect("owned");
    assert_eq!(owned, borrowed);
    assert_eq!(serialize(&owned), serialize(&borrowed));

    let empty: Vec<LayerCollectionIR> = Vec::new();
    let borrowed = emitter().emit_gcode(&empty).expect("borrowed empty");
    let owned = emitter().emit_gcode_owned(empty).expect("owned empty");
    assert_eq!(owned, borrowed);
    assert_eq!(owned.commands, borrowed.commands);
}

#[test]
fn rotation_reorders_the_leading_cluster_to_the_previous_layers_tool() {
    let layers = rotation_fixture();
    // Authored order: layer 1 = [tool 0 @ 30, tool 1 @ 40].
    let authored: Vec<f32> = layers[1]
        .ordered_entities
        .iter()
        .map(|e| e.path.points[0].x)
        .collect();
    assert_eq!(authored, vec![30.0, 40.0]);

    let ir = emitter().emit_gcode(&layers).expect("emission");
    let xs = move_xs(&ir);
    // Layer 0 prints its entity first; layer 1 must then print the tool-1
    // cluster (authored @40) before the tool-0 cluster (authored @30).
    assert_eq!(
        xs,
        vec![11.0, 41.0, 31.0],
        "rotation must move the trailing tool-1 cluster of layer 1 to the front"
    );
    // Exactly two tool changes: T1 entering layer 0, and the intra-layer
    // T1→T0 that rotation synthesized after layer 1's leading tool-1 cluster.
    // The layer's first move is on the inherited tool, which is the point of
    // the pass: no boundary tool change before layer 1's first extrusion.
    assert_eq!(tool_changes(&ir), vec![(0, 1), (1, 0)]);

    let first_move_xs = ir
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::Move { x: Some(x), e: Some(_), .. } if *x == 41.0))
        .expect("layer 1's rotated leading entity is emitted");
    let second_change = ir
        .commands
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, GCodeCommand::ToolChange { .. }))
        .nth(1)
        .map(|(i, _)| i)
        .expect("intra-layer tool change after rotation");
    assert!(
        first_move_xs < second_change,
        "layer 1 must print its inherited-tool cluster before any tool change"
    );
}

#[test]
fn single_tool_job_emits_no_tool_change_and_keeps_entity_order() {
    let layers = vec![
        layer(
            0,
            0.2,
            vec![
                entity(1, 10.0, ExtrusionRole::OuterWall, 0),
                entity(2, 20.0, ExtrusionRole::InnerWall, 0),
            ],
        ),
        layer(
            1,
            0.4,
            vec![
                entity(3, 30.0, ExtrusionRole::OuterWall, 0),
                entity(4, 40.0, ExtrusionRole::InnerWall, 0),
            ],
        ),
    ];
    let ir = emitter().emit_gcode_owned(layers).expect("emission");

    assert!(
        tool_changes(&ir).is_empty(),
        "a single-tool job must not emit a tool change"
    );
    assert_eq!(
        move_xs(&ir),
        vec![11.0, 21.0, 31.0, 41.0],
        "a single-tool job must keep the authored entity order"
    );
    assert!(
        !serialize(&ir)
            .lines()
            .any(|l| l.starts_with('T') && l[1..].trim().parse::<u32>().is_ok()),
        "a single-tool job must not emit a bare T<n> line"
    );
}

#[test]
fn owned_emission_reports_the_same_error_as_borrowed_emission() {
    // A corrupt tool index trips the emitter's allocation guard; the owned
    // entry point must surface exactly the borrowed entry point's error rather
    // than swallowing it (the collection is consumed either way).
    let layers = vec![layer(
        0,
        0.2,
        // 5_000 is far past the emitter's 1_024-slot plausible-tool cap.
        vec![entity(1, 10.0, ExtrusionRole::OuterWall, 5_000)],
    )];

    let borrowed = emitter().emit_gcode(&layers);
    let owned = emitter().emit_gcode_owned(layers);
    assert_eq!(
        format!("{:?}", owned.expect_err("tool index guard rejects")),
        format!("{:?}", borrowed.expect_err("tool index guard rejects"))
    );
}

#[test]
fn owned_emission_keeps_layer_annotation_order() {
    // Entity order is not the only order the emitter walks: `;TYPE:` role
    // markers follow the rotated entity order, and the metadata layer count
    // must be unaffected by ownership. This pins the serialized text's
    // `;TYPE:` sequence for the rotation fixture.
    let ir = emitter()
        .emit_gcode_owned(rotation_fixture())
        .expect("emission");
    let text = serialize(&ir);
    let types: Vec<&str> = text.lines().filter(|l| l.starts_with(";TYPE:")).collect();
    assert_eq!(
        types,
        vec![
            ";TYPE:Outer wall", // layer 0's tool-1 entity
            ";TYPE:Inner wall", // layer 1's tool-0 entity, emitted last after rotation
        ],
        "role markers must follow the rotated entity order (layer 1's leading \
         tool-1 entity repeats the previous role, so no marker is re-emitted)"
    );
    assert_eq!(ir.metadata.layer_count, 2);
}
