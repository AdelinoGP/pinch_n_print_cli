#![allow(missing_docs)]

//! Acceptance tests for packet 57 (TASK-182): overhang-speed quartile dispatch.
//!
//! Covers AC-1 through AC-5 + the negative criterion AC-N1. AC-6 lives in
//! `crates/slicer-ir/tests/point3_overhang_quartile_roundtrip.rs`.

use slicer_ir::*;
use slicer_runtime::{Blackboard, DefaultGCodeEmitter, GCodeEmitter};
use std::sync::Arc;

// â”€â”€â”€ shared fixture helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn dummy_blackboard() -> Blackboard {
    let mesh_ir = MeshIR {
        objects: vec![],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 250.0,
                y: 210.0,
                z: 256.0,
            },
        },
        ..Default::default()
    };
    Blackboard::new(Arc::new(mesh_ir), 1)
}

fn make_layer(global_layer_index: u32, z: f32) -> LayerCollectionIR {
    LayerCollectionIR {
        global_layer_index,
        z,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
        ..Default::default()
    }
}

fn make_wall_path(
    points: Vec<Point3WithWidth>,
    role: ExtrusionRole,
    speed_factor: f32,
) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points,
        role,
        speed_factor,
    }
}

fn make_entity(entity_id: u64, path: ExtrusionPath3D, role: ExtrusionRole) -> PrintEntity {
    PrintEntity {
        entity_id,
        path,
        role: role.clone(),
        region_key: RegionKey {
            region_id: entity_id,
            global_layer_index: 0,
            object_id: "obj".to_string(),
        },
        topo_order: entity_id as u32,
    }
}

/// Collect every F-token from Move commands in the GCodeIR output.
fn collect_f_tokens(gcode_ir: &GCodeIR) -> Vec<f32> {
    gcode_ir
        .commands
        .iter()
        .filter_map(|cmd| {
            if let GCodeCommand::Move { f: Some(f_val), .. } = cmd {
                Some(*f_val)
            } else {
                None
            }
        })
        .collect()
}

// â”€â”€â”€ AC-1: cantilever_emits_overhang_speed â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Two-layer cantilever scene: layer 0 is a 20Ã—20mm square (no overhang),
/// layer 1 has its outer wall marked with `overhang_quartile = Some(1)`.
/// With `overhang_1_4_speed = 10.0 mm/s` the outer-wall move on layer 1
/// must emit `F600` (10 mm/s Ã— 60 = 600 mm/min).
#[test]
fn cantilever_emits_overhang_speed() {
    let config = slicer_ir::FeedrateConfig {
        outer_wall_speed: 60.0,
        overhang_1_4_speed: 10.0,
        overhang_2_4_speed: 0.0,
        overhang_3_4_speed: 0.0,
        overhang_4_4_speed: 0.0,
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config);
    let blackboard = dummy_blackboard();

    // Layer 0: 20Ã—20mm square outer wall, no overhang.
    let mut layer0 = make_layer(0, 0.2);
    let pts0 = vec![
        Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        Point3WithWidth {
            x: 20.0,
            y: 0.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        Point3WithWidth {
            x: 20.0,
            y: 20.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        Point3WithWidth {
            x: 0.0,
            y: 20.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
    ];
    layer0.ordered_entities.push(make_entity(
        1,
        make_wall_path(pts0, ExtrusionRole::OuterWall, 1.0),
        ExtrusionRole::OuterWall,
    ));

    // Layer 1: 20Ã—30mm rectangle; the +y extension (y > 20.0) is cantilever
    // territory and its points are classified as overhang quartile 1.
    let mut layer1 = make_layer(1, 0.4);
    let pts1 = vec![
        // Supported region (y <= 20) â€” no quartile
        Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.4,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        Point3WithWidth {
            x: 20.0,
            y: 0.0,
            z: 0.4,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        Point3WithWidth {
            x: 20.0,
            y: 20.0,
            z: 0.4,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        // Cantilever extension (y > 20) â€” quartile 1
        Point3WithWidth {
            x: 20.0,
            y: 30.0,
            z: 0.4,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: Some(1),
        },
        Point3WithWidth {
            x: 0.0,
            y: 30.0,
            z: 0.4,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: Some(1),
        },
        Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.4,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
    ];
    layer1.ordered_entities.push(make_entity(
        2,
        make_wall_path(pts1, ExtrusionRole::OuterWall, 1.0),
        ExtrusionRole::OuterWall,
    ));

    let gcode_ir = emitter.emit_gcode(&[layer0, layer1], &blackboard).unwrap();

    // Find the F-token emitted for a move that has overhang_quartile = Some(1).
    // After implementation, that move must use F600 (10 mm/s Ã— 60).
    let overhang_f: Vec<f32> = gcode_ir
        .commands
        .iter()
        .filter_map(|cmd| {
            if let GCodeCommand::Move { f: Some(f_val), .. } = cmd {
                Some(*f_val)
            } else {
                None
            }
        })
        .collect();

    assert!(
        overhang_f.contains(&600.0),
        "Expected F600 for overhang_1_4_speed=10.0; got {:?}",
        overhang_f
    );
}

// â”€â”€â”€ AC-2: zero_config_byte_identical_baseline â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// When all four `overhang_N_4_speed = 0.0`, the emitted G-code must be
/// byte-identical to a pre-feature baseline (the classifier short-circuits
/// and `resolve_feedrate`'s overhang arm falls through to the role base
/// speed), and `classify_layers` must not write `Some(_)` onto any point.
#[test]
fn zero_config_byte_identical_baseline() {
    use slicer_core::algos::overhang_classifier::classify_layers;

    let zero_config = slicer_ir::FeedrateConfig {
        outer_wall_speed: 60.0,
        overhang_1_4_speed: 0.0,
        overhang_2_4_speed: 0.0,
        overhang_3_4_speed: 0.0,
        overhang_4_4_speed: 0.0,
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), zero_config.clone());
    let blackboard = dummy_blackboard();

    // Two-layer cantilever scene, same shape as AC-1. Caller picks the seed
    // value placed on the cantilever points; classify_layers must short-circuit
    // under zero config so the seed survives into the emitter's clone, and
    // resolve_feedrate's overhang arm must fall through because every
    // overhang_*_4_speed is 0.0.
    let make_pts = |y_max: f32, z: f32, seed: Option<u8>| {
        vec![
            Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: 20.0,
                y: 0.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: 20.0,
                y: y_max,
                z,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: seed,
            },
            Point3WithWidth {
                x: 0.0,
                y: y_max,
                z,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: seed,
            },
            Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
        ]
    };
    let build_scene = |cantilever_seed: Option<u8>| {
        let mut l0 = make_layer(0, 0.2);
        l0.ordered_entities.push(make_entity(
            1,
            make_wall_path(make_pts(20.0, 0.2, None), ExtrusionRole::OuterWall, 1.0),
            ExtrusionRole::OuterWall,
        ));
        let mut l1 = make_layer(1, 0.4);
        l1.ordered_entities.push(make_entity(
            2,
            make_wall_path(
                make_pts(30.0, 0.4, cantilever_seed),
                ExtrusionRole::OuterWall,
                1.0,
            ),
            ExtrusionRole::OuterWall,
        ));
        vec![l0, l1]
    };

    // Pre-feature baseline: scene with no Some(_) seeds, emitted under zero
    // config. Captured as a Debug-formatted string of the full `commands` Vec
    // â€” every byte of the emitter output is now pinned to this snapshot for
    // the remainder of the test.
    let scene_baseline = build_scene(None);
    let gcode_baseline = emitter.emit_gcode(&scene_baseline, &blackboard).unwrap();
    let baseline_str = format!("{:?}", gcode_baseline.commands);

    // â”€â”€ Half 1 of AC-2: byte-identical G-code under zero config â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    //
    // Seed the cantilever points with `Some(2)` quartiles. Under zero config,
    // (a) classify_layers must short-circuit (leaving the seeds intact in the
    //     emitter's cloned layer set), and
    // (b) resolve_feedrate's overhang arm guards on `overhang_2_4_speed > 0.0`
    //     and falls through to outer-wall base speed.
    // â‡’ emitter output must be byte-identical to the all-None baseline.
    let scene_seeded = build_scene(Some(2));
    let gcode_seeded = emitter.emit_gcode(&scene_seeded, &blackboard).unwrap();
    let seeded_str = format!("{:?}", gcode_seeded.commands);

    assert_eq!(
        seeded_str, baseline_str,
        "AC-2: zero-config G-code with seeded Some(2) is not byte-identical to all-None baseline \
         (classifier leaked or overhang dispatch arm fired when speed == 0.0)"
    );

    // Cross-check the F-token vector explicitly: every emitted F-token must
    // equal the outer-wall base speed (60 mm/s Ã— 60 = 3600 mm/min). Any
    // divergence proves the overhang arm fired despite zero config.
    let f_tokens = collect_f_tokens(&gcode_baseline);
    let base_f = 60.0_f32 * 60.0;
    assert!(
        !f_tokens.is_empty(),
        "AC-2 sanity: zero-config scene produced no F tokens"
    );
    for &f in &f_tokens {
        assert!(
            (f - base_f).abs() < 1.0,
            "AC-2: F-token {f} diverged from outer-wall base speed {base_f}"
        );
    }

    // Sanity: prove the equality test is non-degenerate by showing an active
    // config produces a DIFFERENT G-code on the same scene. Without this,
    // baseline equality would be vacuously true.
    let active_config = slicer_ir::FeedrateConfig {
        outer_wall_speed: 60.0,
        overhang_1_4_speed: 10.0,
        ..Default::default()
    };
    let active_emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), active_config);
    let scene_active = build_scene(None);
    let gcode_active = active_emitter
        .emit_gcode(&scene_active, &blackboard)
        .unwrap();
    let active_str = format!("{:?}", gcode_active.commands);
    assert_ne!(
        active_str, baseline_str,
        "AC-2 sanity: active overhang config produced the same G-code as the baseline \
         (the byte-identical assertion would be vacuous)"
    );

    // â”€â”€ Half 2 of AC-2: classifier writes no `Some(_)` under zero config â”€â”€â”€
    let mut scene_for_classifier = build_scene(None);
    classify_layers(&mut scene_for_classifier, &zero_config);
    for layer in &scene_for_classifier {
        for entity in &layer.ordered_entities {
            for pt in &entity.path.points {
                assert_eq!(
                    pt.overhang_quartile, None,
                    "AC-2: classifier wrote Some(_) under zero config at layer {} point ({},{})",
                    layer.global_layer_index, pt.x, pt.y
                );
            }
        }
    }
}

// â”€â”€â”€ AC-3: non_wall_roles_ignore_overhang_quartile â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Non-wall roles (`SparseInfill`, `BridgeInfill`, `SupportMaterial`,
/// `SupportInterface`) with `overhang_quartile = Some(1)` and all four
/// overhang speeds set to 10/20/30/40 mm/s must still emit their own
/// role-base F-token, not the overhang speed.
#[test]
fn non_wall_roles_ignore_overhang_quartile() {
    let config = slicer_ir::FeedrateConfig {
        sparse_infill_speed: 100.0,
        bridge_speed: 25.0,
        support_speed: 80.0,
        support_interface_speed: 80.0,
        overhang_1_4_speed: 10.0,
        overhang_2_4_speed: 20.0,
        overhang_3_4_speed: 30.0,
        overhang_4_4_speed: 40.0,
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config);
    let blackboard = dummy_blackboard();

    let role_cases: &[(ExtrusionRole, f32)] = &[
        (ExtrusionRole::SparseInfill, 100.0 * 60.0),
        (ExtrusionRole::BridgeInfill, 25.0 * 60.0),
        (ExtrusionRole::SupportMaterial, 80.0 * 60.0),
        (ExtrusionRole::SupportInterface, 80.0 * 60.0),
    ];

    for (idx, (role, expected_f)) in role_cases.iter().enumerate() {
        let mut layer = make_layer(0, 0.2);
        let pts = vec![
            Point3WithWidth {
                x: 0.0,
                y: idx as f32 * 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: Some(1), // explicitly set, but role is non-wall
            },
            Point3WithWidth {
                x: 10.0,
                y: idx as f32 * 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: Some(1),
            },
        ];
        layer.ordered_entities.push(make_entity(
            idx as u64 + 1,
            make_wall_path(pts, role.clone(), 1.0),
            role.clone(),
        ));

        let gcode_ir = emitter.emit_gcode(&[layer], &blackboard).unwrap();
        let f_tokens = collect_f_tokens(&gcode_ir);

        assert!(
            f_tokens.iter().any(|&f| (f - expected_f).abs() < 1.0),
            "non_wall_roles_ignore_overhang_quartile: role {:?} expected F{:.0}, got {:?}",
            role,
            expected_f,
            f_tokens
        );

        // Must NOT emit an overhang-speed F-token for non-wall roles.
        let forbidden_f_values: Vec<f32> = [10.0_f32, 20.0, 30.0, 40.0]
            .iter()
            .map(|s| s * 60.0)
            .collect();
        for forbidden in &forbidden_f_values {
            assert!(
                !f_tokens.iter().any(|&f| (f - forbidden).abs() < 1.0),
                "non_wall_roles_ignore_overhang_quartile: role {:?} emitted forbidden overhang F{:.0}",
                role,
                forbidden
            );
        }
    }
}

// â”€â”€â”€ AC-4: first_layer_quartile_is_none â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Single-layer scene: every `overhang_quartile` in layer 0 must be `None`
/// because the first layer has no layer below it to compare against.
#[test]
fn first_layer_quartile_is_none() {
    use slicer_ir::FeedrateConfig;

    let config = FeedrateConfig {
        overhang_1_4_speed: 10.0,
        ..Default::default()
    };

    // Single-layer scene: a wall path on layer 0
    let pts = vec![
        Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        Point3WithWidth {
            x: 10.0,
            y: 0.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
    ];
    let mut layer = make_layer(0, 0.2);
    layer.ordered_entities.push(make_entity(
        1,
        make_wall_path(pts, ExtrusionRole::OuterWall, 1.0),
        ExtrusionRole::OuterWall,
    ));

    let mut layers = vec![layer];
    slicer_core::algos::overhang_classifier::classify_layers(&mut layers, &config);

    for entity in &layers[0].ordered_entities {
        for pt in &entity.path.points {
            assert_eq!(
                pt.overhang_quartile, None,
                "first layer must have overhang_quartile == None; got {:?}",
                pt.overhang_quartile
            );
        }
    }
}

// â”€â”€â”€ AC-5: quartile_to_key_mapping â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Four outer-wall path segments with `overhang_quartile` explicitly set to
/// `Some(1)` through `Some(4)`.  With `overhang_{1,2,3,4}_4_speed` =
/// 10/20/30/40 mm/s the emitter must produce F600/F1200/F1800/F2400
/// respectively.
#[test]
fn quartile_to_key_mapping() {
    let config = slicer_ir::FeedrateConfig {
        outer_wall_speed: 60.0,
        overhang_1_4_speed: 10.0,
        overhang_2_4_speed: 20.0,
        overhang_3_4_speed: 30.0,
        overhang_4_4_speed: 40.0,
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config);
    let blackboard = dummy_blackboard();

    // Expected: quartile 1â†’F600, 2â†’F1200, 3â†’F1800, 4â†’F2400
    let expected: &[(u8, f32)] = &[(1, 600.0), (2, 1200.0), (3, 1800.0), (4, 2400.0)];

    for (quartile, expected_f) in expected {
        let mut layer = make_layer(0, 0.2);
        let pts = vec![
            Point3WithWidth {
                x: 0.0,
                y: *quartile as f32,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: Some(*quartile),
            },
            Point3WithWidth {
                x: 10.0,
                y: *quartile as f32,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: Some(*quartile),
            },
        ];
        layer.ordered_entities.push(make_entity(
            *quartile as u64,
            make_wall_path(pts, ExtrusionRole::OuterWall, 1.0),
            ExtrusionRole::OuterWall,
        ));

        let gcode_ir = emitter.emit_gcode(&[layer], &blackboard).unwrap();
        let f_tokens = collect_f_tokens(&gcode_ir);

        assert!(
            f_tokens.iter().any(|&f| (f - expected_f).abs() < 1.0),
            "quartile_to_key_mapping: quartile {} expected F{:.0}, got {:?}",
            quartile,
            expected_f,
            f_tokens
        );
    }
}

// â”€â”€â”€ AC-N1: quartile_zero_is_reserved â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Quartile value 0 is reserved. Per AC-N1, the public classifier output
/// enumeration must contain only `None | Some(1..=4)` over a 1000-point
/// random fixture (the release-style assertion-off branch of AC-N1).
/// In `debug_assertions` builds the in-classifier
/// `debug_assert!((1..=4).contains(&q))` guards the same invariant.
#[test]
fn quartile_zero_is_reserved() {
    use slicer_ir::FeedrateConfig;

    let config = FeedrateConfig {
        overhang_1_4_speed: 10.0,
        overhang_2_4_speed: 20.0,
        overhang_3_4_speed: 30.0,
        overhang_4_4_speed: 40.0,
        ..Default::default()
    };

    let mk_pt = |x: f32, y: f32, z: f32| Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    };

    // Small fixture: two layers with a deliberate cantilever (layer 1 extends
    // beyond layer 0 on every side), exercising every quartile band.
    let mut layer0 = make_layer(0, 0.2);
    layer0.ordered_entities.push(make_entity(
        1,
        make_wall_path(
            vec![
                mk_pt(0.0, 0.0, 0.2),
                mk_pt(10.0, 0.0, 0.2),
                mk_pt(10.0, 10.0, 0.2),
                mk_pt(0.0, 10.0, 0.2),
            ],
            ExtrusionRole::OuterWall,
            1.0,
        ),
        ExtrusionRole::OuterWall,
    ));

    // 1000-point random sweep: scatter points across [-15, 25] Ã— [-15, 25],
    // covering the prev-layer 0..10 square and its surroundings. Uses a
    // deterministic LCG so the fixture is reproducible across runs and
    // platforms.
    let mut state: u64 = 0xc0ff_eed0_0d0b_adf0;
    let mut pts_1k: Vec<Point3WithWidth> = Vec::with_capacity(1000);
    for _ in 0..1000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = ((state >> 32) as u32 % 4000) as f32 / 100.0 - 15.0;
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let y = ((state >> 32) as u32 % 4000) as f32 / 100.0 - 15.0;
        pts_1k.push(mk_pt(x, y, 0.4));
    }

    let mut layer1 = make_layer(1, 0.4);
    layer1.ordered_entities.push(make_entity(
        2,
        make_wall_path(pts_1k, ExtrusionRole::OuterWall, 1.0),
        ExtrusionRole::OuterWall,
    ));

    let mut layers = vec![layer0, layer1];
    slicer_core::algos::overhang_classifier::classify_layers(&mut layers, &config);

    // Exhaustive enumeration: classifier output must be exactly None or one
    // of Some(1)..=Some(4). Any other value (Some(0), Some(5), â€¦) is a
    // bucketization bug.
    for (li, layer) in layers.iter().enumerate() {
        for entity in &layer.ordered_entities {
            for pt in &entity.path.points {
                match pt.overhang_quartile {
                    None | Some(1) | Some(2) | Some(3) | Some(4) => {}
                    other => panic!(
                        "AC-N1: classifier produced invalid quartile {:?} at layer {} point ({},{})",
                        other, li, pt.x, pt.y
                    ),
                }
            }
        }
    }
}
