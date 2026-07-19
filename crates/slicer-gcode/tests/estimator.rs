#![allow(missing_docs)]

//! TDD tests for packet 169 Step 2: Marlin-style simplified trapezoid print
//! time estimator + per-tool extruded volume accounting.
//!
//! Contract facts locked by these tests (mirroring `serialize.rs`):
//! - `GCodeIR` `Move.e` is ALWAYS an absolute E position; `ExtrusionMode`
//!   affects serialization only, never IR interpretation.
//! - `Retract`/`Unretract` are e-axis deltas (excluded from volume) that
//!   adjust the absolute-E accumulator.
//! - Estimator operates in mm (G-code space).

use std::collections::BTreeMap;

use slicer_gcode::{
    estimate_print, DefaultGCodeEmitter, EstimatorLimits, GCodeEmitter, PrintEstimate,
};
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, GCodeCommand, GCodeIR, ObjectId, Point3WithWidth, PrintEntity,
    RegionKey, ResolvedConfig,
};

fn mv(x: f32, y: f32, e: Option<f32>, f: Option<f32>) -> GCodeCommand {
    GCodeCommand::Move {
        x: Some(x),
        y: Some(y),
        z: None,
        e,
        f,
        role: ExtrusionRole::OuterWall,
    }
}

fn ir_with(commands: Vec<GCodeCommand>) -> GCodeIR {
    GCodeIR {
        commands,
        ..Default::default()
    }
}

// ============================================================================
// Test 1: single straight 100 mm segment, analytic trapezoid
// ============================================================================

#[test]
fn trapezoid_single_segment_analytic() {
    let limits = EstimatorLimits {
        max_acceleration: 500.0,
        max_acceleration_travel: 500.0,
        jerk_xy: 9.0,
        ..Default::default()
    };
    // Three Move commands forming one straight 100 mm XY segment at F3000
    // (50 mm/s). The first Move only establishes position (no prior point).
    let ir = ir_with(vec![
        mv(0.0, 0.0, None, Some(3000.0)),
        mv(50.0, 0.0, None, None),
        mv(100.0, 0.0, None, None),
    ]);

    let est: PrintEstimate = estimate_print(&ir, &limits, &BTreeMap::new());

    // Analytic trapezoid: v = 50 mm/s, a = 500 mm/s², entry/exit junction
    // speed v_j = min(jerk_xy, v) = 9 mm/s, distance d = 100 mm.
    let v = 50.0_f64;
    let a = 500.0_f64;
    let vj = 9.0_f64;
    let d = 100.0_f64;
    let analytic = 2.0 * (v - vj) / a + (d - (v * v - vj * vj) / a) / v;

    assert!(
        (est.total_time_s - analytic).abs() <= 0.02 * analytic,
        "total_time_s = {} not within 2% of analytic {}",
        est.total_time_s,
        analytic
    );
    assert!(
        est.total_time_s > 2.0,
        "total_time_s = {} must be strictly > 2.0 s",
        est.total_time_s
    );
}

// ============================================================================
// Test 2: two-tool volume map + toolchange count
// ============================================================================

#[test]
fn two_tool_volume_map_and_toolchange_count() {
    let limits = EstimatorLimits::default();
    let ir = ir_with(vec![
        mv(0.0, 0.0, None, Some(3000.0)),
        // Tool 0 extrudes an absolute-E delta of 10.0 mm.
        mv(20.0, 0.0, Some(10.0), None),
        GCodeCommand::ToolChange {
            after_entity_index: 0,
            from: 0,
            to: 1,
        },
        // Tool 1 extrudes an absolute-E delta of 5.0 mm (10.0 -> 15.0).
        mv(20.0, 10.0, Some(15.0), None),
    ]);
    let diameters = BTreeMap::from([(0u32, 1.75f32), (1u32, 1.75f32)]);

    let est = estimate_print(&ir, &limits, &diameters);

    let area = std::f64::consts::PI * (1.75f64 / 2.0).powi(2);

    assert_eq!(
        est.extruded_volume_mm3.keys().copied().collect::<Vec<_>>(),
        vec![0u32, 1u32],
        "extruded_volume_mm3 must have exactly keys {{0, 1}}"
    );
    assert!(
        (est.extruded_volume_mm3[&0] - 10.0 * area).abs() < 1e-3,
        "tool 0 volume {} != {}",
        est.extruded_volume_mm3[&0],
        10.0 * area
    );
    assert!(
        (est.extruded_volume_mm3[&1] - 5.0 * area).abs() < 1e-3,
        "tool 1 volume {} != {}",
        est.extruded_volume_mm3[&1],
        5.0 * area
    );
    assert!((est.filament_length_mm[&0] - 10.0).abs() < 1e-6);
    assert!((est.filament_length_mm[&1] - 5.0).abs() < 1e-6);
    assert_eq!(est.toolchange_count, 1);
}

// ============================================================================
// Test 3: emit_gcode fills metadata.estimated_print_time_s
// ============================================================================

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

fn entity_fixture(points: Vec<Point3WithWidth>) -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points,
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        tool_index: 0,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: ObjectId::from("test-object"),
            region_id: 0u64,
            variant_chain: Vec::new(),
        },
        topo_order: 0,
    }
}

#[test]
fn metadata_estimated_time_filled() {
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
    let entity = entity_fixture(vec![
        point3(0.0, 0.0, 0.2),
        point3(200.0, 0.0, 0.2),
        point3(200.0, 200.0, 0.2),
    ]);
    let layer = slicer_ir::LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity],
        ..Default::default()
    };

    let gcode_ir = emitter.emit_gcode(&[layer]).expect("emit should succeed");

    assert!(
        gcode_ir.metadata.estimated_print_time_s > 0,
        "estimated_print_time_s must be > 0 for a non-empty move stream"
    );

    // Must equal the estimator's total rounded to whole seconds, using the
    // same limits (default ResolvedConfig -> all-None machine limits ->
    // EstimatorLimits::default()) and default 1.75 mm diameters.
    let limits = EstimatorLimits::from_config(&ResolvedConfig::default());
    let est = estimate_print(&gcode_ir, &limits, &BTreeMap::new());
    assert_eq!(
        gcode_ir.metadata.estimated_print_time_s,
        est.total_time_s.round() as u32,
        "metadata time {} != estimator total {} rounded",
        gcode_ir.metadata.estimated_print_time_s,
        est.total_time_s
    );
}

// ============================================================================
// Test 4: fallback defaults when machine limits absent
// ============================================================================

#[test]
fn fallback_defaults_when_machine_limits_absent() {
    // ResolvedConfig::default() leaves every machine-limit/jerk key None.
    let cfg = ResolvedConfig::default();
    assert!(cfg.machine_max_acceleration_extruding.is_none());
    assert!(cfg.machine_max_jerk_x.is_none());

    let limits = EstimatorLimits::from_config(&cfg);

    // Documented fallbacks.
    assert_eq!(limits.max_acceleration, 1500.0);
    assert_eq!(limits.max_acceleration_travel, 1500.0);
    assert_eq!(limits.max_speed_xy, 200.0);
    assert_eq!(limits.max_speed_z, 12.0);
    assert_eq!(limits.max_speed_e, 25.0);
    assert_eq!(limits.jerk_xy, 9.0);
    assert_eq!(limits.jerk_z, 0.2);
    assert_eq!(limits.jerk_e, 2.5);

    let ir = ir_with(vec![
        mv(0.0, 0.0, None, Some(3000.0)),
        mv(30.0, 0.0, Some(2.0), None),
    ]);
    let est = estimate_print(&ir, &limits, &BTreeMap::new());
    assert!(
        est.total_time_s > 0.0,
        "total_time_s must be > 0 with fallback limits, got {}",
        est.total_time_s
    );
}
#[test]
fn elapsed_time_tracks_command_boundaries() {
    use std::collections::BTreeMap;

    use slicer_gcode::estimator::{estimate_print_with_elapsed, EstimatorLimits};
    use slicer_ir::{ExtrusionRole, GCodeCommand, GCodeIR};

    let gcode_ir = GCodeIR {
        commands: vec![
            GCodeCommand::Move {
                x: Some(0.0),
                y: Some(0.0),
                z: Some(0.0),
                e: None,
                f: Some(60.0),
                role: ExtrusionRole::Custom("Travel".to_string()),
            },
            GCodeCommand::Comment {
                text: ";LAYER_CHANGE".to_string(),
            },
            GCodeCommand::Move {
                x: Some(10.0),
                y: Some(0.0),
                z: Some(0.0),
                e: None,
                f: None,
                role: ExtrusionRole::Custom("Travel".to_string()),
            },
            GCodeCommand::Move {
                x: Some(20.0),
                y: Some(0.0),
                z: Some(0.0),
                e: None,
                f: None,
                role: ExtrusionRole::Custom("Travel".to_string()),
            },
        ],
        ..Default::default()
    };

    let (estimate, elapsed) =
        estimate_print_with_elapsed(&gcode_ir, &EstimatorLimits::default(), &BTreeMap::new());

    assert_eq!(elapsed.len(), gcode_ir.commands.len());
    assert!(elapsed.windows(2).all(|window| window[0] <= window[1]));
    assert!((elapsed.last().copied().unwrap() - estimate.total_time_s).abs() < f64::EPSILON);
    assert_eq!(elapsed[0], 0.0);
    assert_eq!(elapsed[1], 0.0);
    assert!(elapsed[2] > 0.0);
}
