//! Regression: a closed wall loop (N+1 points with last == first) emits the
//! complete loop in G-code — 1 travel move + N edge G1 moves (one of which
//! closes the loop). Pre-fix, the 20mm cube's outer wall was emitted as 3 of
//! 4 edges because the IR carried N points and the emitter only iterated
//! them, dropping the closing edge.
//!
//! With construction-time normalisation (classic-perimeters /
//! arachne-perimeters now append the closing repeat), the same iteration
//! naturally produces the closing edge. This test pins that contract at the
//! emitter boundary so a future regression in either the IR convention or
//! the emitter is caught immediately.

use slicer_gcode::{DefaultGCodeEmitter, GCodeEmitter};
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, GCodeCommand, LayerCollectionIR, Point3WithWidth, PrintEntity,
    RegionKey,
};

fn point(x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

/// A 10mm square outer wall with explicit closing repeat (5 points).
fn closed_square_outer_wall() -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![
                point(0.0, 0.0),
                point(10.0, 0.0),
                point(10.0, 10.0),
                point(0.0, 10.0),
                point(0.0, 0.0), // closing repeat
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: "obj".into(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        topo_order: 0,
    }
}

fn layer_with(entities: Vec<PrintEntity>) -> LayerCollectionIR {
    LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: entities,
        ..Default::default()
    }
}

/// Count G1 (extrusion or travel) commands that fall under the most recent
/// `;TYPE:<role>` comment matching `role_label`.
fn count_g1_moves_under(label: &str, commands: &[GCodeCommand]) -> usize {
    let mut count = 0;
    let mut active = false;
    for cmd in commands {
        match cmd {
            GCodeCommand::Raw { text } if text.starts_with(";TYPE:") => {
                active = text.trim() == label;
            }
            GCodeCommand::Move { .. } if active => count += 1,
            _ => {}
        }
    }
    count
}

#[test]
fn closed_outer_wall_emits_closing_edge() {
    let layer = layer_with(vec![closed_square_outer_wall()]);
    let emitter = DefaultGCodeEmitter::new("test-1.0".into());
    let gcode = emitter.emit_gcode(&[layer]).expect("emit");

    let g1_count = count_g1_moves_under(";TYPE:Outer wall", &gcode.commands);
    // 1 travel to start + 4 edges (right, top, left, AND closing bottom) = 5
    assert_eq!(
        g1_count, 5,
        "closed 4-vertex square outer wall must emit 5 G1 moves (travel + 4 edges); \
         got {g1_count}. The pre-fix bug emitted only 4 (missing the closing edge)."
    );
}

#[test]
fn closed_outer_wall_returns_to_start_point() {
    let layer = layer_with(vec![closed_square_outer_wall()]);
    let emitter = DefaultGCodeEmitter::new("test-1.0".into());
    let gcode = emitter.emit_gcode(&[layer]).expect("emit");

    let mut active = false;
    let mut moves: Vec<(Option<f32>, Option<f32>)> = Vec::new();
    for cmd in &gcode.commands {
        match cmd {
            GCodeCommand::Raw { text } if text.starts_with(";TYPE:") => {
                active = text.trim() == ";TYPE:Outer wall";
            }
            GCodeCommand::Move { x, y, .. } if active => moves.push((*x, *y)),
            _ => {}
        }
    }
    assert!(
        moves.len() >= 5,
        "expected at least 5 wall moves; got {}",
        moves.len()
    );
    let first = moves[0];
    let last = moves[moves.len() - 1];
    assert_eq!(
        first, last,
        "first wall move (travel to start) and last wall move (closing extrusion) \
         must share XY: first={first:?}, last={last:?}"
    );
}
