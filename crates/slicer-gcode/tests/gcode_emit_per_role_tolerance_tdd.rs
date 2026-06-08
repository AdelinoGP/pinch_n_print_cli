#![allow(missing_docs)]

//! TDD tests for TASK-201 / packet 60 Step 7: per-ExtrusionRole tolerance dispatch.
//!
//! AC-8: A LayerCollectionIR with one perimeter, one infill, one support, and one travel
//! polyline (each with intentional sub-tolerance wobble) is emitted with:
//!   - perimeter simplified with gcode_resolution  (0.0125 mm)
//!   - infill    simplified with infill_resolution  (0.04   mm)
//!   - support   simplified with support_resolution (0.0375 mm)
//!   - travel    emitted UNCHANGED (tol = 0.0)
//!
//! Vertex-count assertions: a 12-point wobbled segment (start + 10 wobble + end)
//! collapses to 2 points under any positive tolerance when the wobble (0.005 mm)
//! is below all three role tolerances. Travel retains all 12 points.

use slicer_gcode::{tolerance_for_role, DefaultGCodeEmitter, GCodeEmitter};
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, GCodeCommand, LayerCollectionIR, ObjectId, Point3WithWidth,
    PrintEntity, RegionKey, ResolvedConfig,
};

// ============================================================================
// Fixtures
// ============================================================================

fn region_key() -> RegionKey {
    RegionKey {
        global_layer_index: 0,
        object_id: ObjectId::from("test"),
        region_id: 0u64, // 0 = default tool to avoid spurious ToolChange commands
        variant_chain: Vec::new(),
    }
}

fn p3w(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

/// Build a 12-point polyline: a 1 mm horizontal segment (x: 0â†’1, y=0, z=0.2)
/// broken into 10 interior collinear points with a 0.005 mm perpendicular wobble.
/// 0.005 mm < gcode_resolution (0.0125), infill_resolution (0.04), and
/// support_resolution (0.0375), so D-P will collapse all interior points for any
/// of the three non-travel roles, leaving only the two endpoints.
fn wobbled_polyline(z: f32) -> Vec<Point3WithWidth> {
    // Endpoints
    let start = p3w(0.0, 0.0, z);
    let end = p3w(1.0, 0.0, z);

    let mut pts = vec![start];
    for i in 1..=10 {
        let t = i as f32 / 11.0; // 0.0909 â€¦ 0.9090
        let x = t;
        // Alternate tiny wobble above/below chord; 0.005 mm is below all role tolerances
        let wobble: f32 = if i % 2 == 0 { 0.005 } else { -0.005 };
        pts.push(p3w(x, wobble, z));
    }
    pts.push(end);
    pts // 12 points total
}

fn make_entity(id: u64, points: Vec<Point3WithWidth>, role: ExtrusionRole) -> PrintEntity {
    PrintEntity {
        entity_id: id,
        path: ExtrusionPath3D {
            points,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: region_key(),
        topo_order: id as u32,
    }
}

// ============================================================================
// Helper: count Move commands with a given role in a GCodeCommand list
// ============================================================================

fn count_moves_with_role(commands: &[GCodeCommand], target: &ExtrusionRole) -> usize {
    commands
        .iter()
        .filter(|cmd| matches!(cmd, GCodeCommand::Move { role, .. } if role == target))
        .count()
}

// ============================================================================
// AC-8: per_role_tolerance_dispatch
// ============================================================================

/// AC-8: Verify per-ExtrusionRole tolerance dispatch in the G-code emitter.
///
/// Precomputed vertex counts (Move commands per role):
///   - OuterWall  (perimeter):  2   â€” wobble (0.005 mm) < gcode_resolution (0.0125 mm) â†’ 10 interior points dropped
///   - SparseInfill (infill):   2   â€” wobble (0.005 mm) < infill_resolution (0.04 mm)  â†’ 10 interior points dropped
///   - SupportMaterial:         2   â€” wobble (0.005 mm) < support_resolution (0.0375 mm) â†’ 10 interior points dropped
///   - Custom("Travel"):        12  â€” tol = 0.0, no simplification, all points preserved
#[test]
fn per_role_tolerance_dispatch() {
    let z = 0.2_f32;

    let perimeter_points = wobbled_polyline(z);
    let infill_points = wobbled_polyline(z);
    let support_points = wobbled_polyline(z);
    let travel_points = wobbled_polyline(z);

    let perimeter_entity = make_entity(1, perimeter_points, ExtrusionRole::OuterWall);
    let infill_entity = make_entity(2, infill_points, ExtrusionRole::SparseInfill);
    let support_entity = make_entity(3, support_points, ExtrusionRole::SupportMaterial);
    let travel_entity = make_entity(
        4,
        travel_points,
        ExtrusionRole::Custom("Travel".to_string()),
    );

    let layer = LayerCollectionIR {
        global_layer_index: 0,
        z,
        ordered_entities: vec![
            perimeter_entity,
            infill_entity,
            support_entity,
            travel_entity,
        ],
        ..Default::default()
    };

    // Use default ResolvedConfig (gcode_resolution=0.0125, infill_resolution=0.04,
    // support_resolution=0.0375, min_segment_length=0.05).
    let cfg = ResolvedConfig::default();

    // Confirm tolerance_for_role dispatch
    assert!(
        (tolerance_for_role(&ExtrusionRole::OuterWall, &cfg) - 0.0125).abs() < 1e-6,
        "OuterWall tolerance must equal gcode_resolution"
    );
    assert!(
        (tolerance_for_role(&ExtrusionRole::SparseInfill, &cfg) - 0.04).abs() < 1e-6,
        "SparseInfill tolerance must equal infill_resolution"
    );
    assert!(
        (tolerance_for_role(&ExtrusionRole::SupportMaterial, &cfg) - 0.0375).abs() < 1e-6,
        "SupportMaterial tolerance must equal support_resolution"
    );
    assert_eq!(
        tolerance_for_role(&ExtrusionRole::Custom("Travel".to_string()), &cfg),
        0.0,
        "Travel tolerance must be 0.0"
    );

    let emitter = DefaultGCodeEmitter::new("test".to_string()).with_resolved_config(cfg);
    let gcode_ir = emitter
        .emit_gcode(&[layer])
        .expect("emit_gcode must succeed");

    // Precomputed table:
    //   OuterWall      â†’ 2 Move commands (start + end; 10 interior wobble points dropped)
    //   SparseInfill   â†’ 2 Move commands
    //   SupportMaterialâ†’ 2 Move commands
    //   Custom("Travel")â†’ 12 Move commands (no simplification)
    let perimeter_moves = count_moves_with_role(&gcode_ir.commands, &ExtrusionRole::OuterWall);
    let infill_moves = count_moves_with_role(&gcode_ir.commands, &ExtrusionRole::SparseInfill);
    let support_moves = count_moves_with_role(&gcode_ir.commands, &ExtrusionRole::SupportMaterial);
    let travel_moves = count_moves_with_role(
        &gcode_ir.commands,
        &ExtrusionRole::Custom("Travel".to_string()),
    );

    assert_eq!(
        perimeter_moves, 2,
        "OuterWall polyline: expected 2 moves (start+end) after D-P with tol=0.0125, got {perimeter_moves}"
    );
    assert_eq!(
        infill_moves, 2,
        "SparseInfill polyline: expected 2 moves (start+end) after D-P with tol=0.04, got {infill_moves}"
    );
    assert_eq!(
        support_moves, 2,
        "SupportMaterial polyline: expected 2 moves (start+end) after D-P with tol=0.0375, got {support_moves}"
    );
    assert_eq!(
        travel_moves, 12,
        "Travel polyline: expected 12 moves (all points retained, no simplification), got {travel_moves}"
    );
}
