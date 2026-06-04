#![allow(missing_docs)]

//! TDD tests for packet 52 (TASK-153): per-role feedrate emission on the live G-code path.

use slicer_gcode::{DefaultGCodeEmitter, GCodeEmitter};
use slicer_ir::*;

#[test]
fn per_role_speed_resolves_to_f_token() {
    // Three regions in sequence (OuterWall â†’ InnerWall â†’ SparseInfill) with
    // an overridden ConfigView: outer=30, inner=60, sparse=120 mm/s.
    // Expected F tokens: 1800 / 3600 / 7200 mm/min on the first print move
    // of each region.
    let mut layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
        ..Default::default()
    };

    let region_specs: [(u64, ExtrusionRole); 3] = [
        (1, ExtrusionRole::OuterWall),
        (2, ExtrusionRole::InnerWall),
        (3, ExtrusionRole::SparseInfill),
    ];
    for (entity_id, role) in &region_specs {
        let path = ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: 0.0,
                    y: *entity_id as f32,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: 10.0,
                    y: *entity_id as f32,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
            ],
            role: role.clone(),
            speed_factor: 1.0,
        };
        layer.ordered_entities.push(PrintEntity {
            entity_id: *entity_id,
            path,
            role: role.clone(),
            region_key: RegionKey {
                region_id: *entity_id,
                global_layer_index: 0,
                object_id: "obj".to_string(),
            },
            topo_order: *entity_id as u32,
        });
    }

    let config = slicer_ir::FeedrateConfig {
        outer_wall_speed: 30.0,
        inner_wall_speed: 60.0,
        sparse_infill_speed: 120.0,
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config);
    let gcode_ir = emitter.emit_gcode(&[layer]).unwrap();

    let mut firsts: Vec<f32> = Vec::new();
    for cmd in &gcode_ir.commands {
        if let GCodeCommand::Move {
            f: Some(f_val),
            role,
            ..
        } = cmd
        {
            if matches!(
                role,
                ExtrusionRole::OuterWall | ExtrusionRole::InnerWall | ExtrusionRole::SparseInfill
            ) {
                // Capture the first F for each role we encounter.
                let role_idx = match role {
                    ExtrusionRole::OuterWall => 0,
                    ExtrusionRole::InnerWall => 1,
                    ExtrusionRole::SparseInfill => 2,
                    _ => unreachable!(),
                };
                if firsts.len() == role_idx {
                    firsts.push(*f_val);
                }
            }
        }
    }

    assert_eq!(
        firsts.len(),
        3,
        "expected first F for each of three roles, got {:?}",
        firsts
    );
    assert_eq!(firsts[0], 1800.0, "outer_wall_speed=30 â†’ F1800");
    assert_eq!(firsts[1], 3600.0, "inner_wall_speed=60 â†’ F3600");
    assert_eq!(firsts[2], 7200.0, "sparse_infill_speed=120 â†’ F7200");
}

#[test]
fn speed_factor_modulates_role_speed() {
    let mut layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
        ..Default::default()
    };

    let path = ExtrusionPath3D {
        points: vec![
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
        ],
        role: ExtrusionRole::OuterWall,
        speed_factor: 0.5,
    };
    layer.ordered_entities.push(PrintEntity {
        entity_id: 1,
        path,
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
        },
        topo_order: 0,
    });

    let emitter = DefaultGCodeEmitter::new("1.0".to_string());
    let gcode_ir = emitter.emit_gcode(&[layer]).unwrap();

    let mut found_f = false;
    for cmd in &gcode_ir.commands {
        if let GCodeCommand::Move { f: Some(f_val), .. } = cmd {
            assert_eq!(*f_val, 1800.0);
            found_f = true;
        }
    }
    assert!(found_f, "F token not found");
}

#[test]
fn module_supplied_f_wins() {
    let mut layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
        ..Default::default()
    };

    layer.ordered_entities.push(PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            }],
            role: ExtrusionRole::InnerWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::InnerWall,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
        },
        topo_order: 0,
    });
    layer.travel_moves.push(TravelMove {
        entity_id: 1,
        x: Some(10.0),
        y: Some(10.0),
        z: None,
        f: Some(7200.0),
    });

    let emitter = DefaultGCodeEmitter::new("1.0".to_string());
    let gcode_ir = emitter.emit_gcode(&[layer]).unwrap();

    let mut found_f = false;
    for cmd in &gcode_ir.commands {
        if let GCodeCommand::Move {
            f: Some(f_val),
            role: ExtrusionRole::Custom(s),
            ..
        } = cmd
        {
            if s == "Travel" {
                assert_eq!(*f_val, 7200.0);
                found_f = true;
            }
        }
    }
    assert!(found_f, "F token not found on travel move");
}

#[test]
fn distinct_feedrates_present() {
    let mut layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
        ..Default::default()
    };

    layer.ordered_entities.push(PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![
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
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
        },
        topo_order: 0,
    });

    layer.ordered_entities.push(PrintEntity {
        entity_id: 2,
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: 10.0,
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
            ],
            role: ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::SparseInfill,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
        },
        topo_order: 1,
    });

    let emitter = DefaultGCodeEmitter::new("1.0".to_string());
    let gcode_ir = emitter.emit_gcode(&[layer]).unwrap();

    let mut feedrates = std::collections::HashSet::new();
    let mut has_high_speed = false;
    for cmd in &gcode_ir.commands {
        if let GCodeCommand::Move { f: Some(f_val), .. } = cmd {
            feedrates.insert(f_val.to_bits());
            if *f_val > 600.0 {
                has_high_speed = true;
            }
        }
    }
    assert!(
        feedrates.len() >= 2,
        "Expected at least 2 distinct feedrates"
    );
    assert!(
        has_high_speed,
        "Expected at least one feedrate > 600 mm/min"
    );
}

#[test]
fn f_token_within_200_lines() {
    let mut layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
        ..Default::default()
    };

    layer.ordered_entities.push(PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![
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
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
        },
        topo_order: 0,
    });

    let emitter = DefaultGCodeEmitter::new("1.0".to_string());
    let gcode_ir = emitter.emit_gcode(&[layer]).unwrap();

    let mut move_count = 0;
    for cmd in &gcode_ir.commands {
        if let GCodeCommand::Move { f, .. } = cmd {
            assert!(f.is_some(), "Move without F token!");
            move_count += 1;
        }
    }
    assert!(move_count > 0);
}

#[test]
fn rejects_only_retract_speed() {
    // Negative AC: a regressed emit path produces print-Moves with f: None, so the
    // only F-tokens in the textual G-code come from retracts (F25). The
    // distinct_feedrates_present contract counts Move.f values; in this state the
    // set has 0 entries, which is < 2, so the predicate correctly rejects.
    let mut commands = Vec::new();
    for _ in 0..10 {
        commands.push(GCodeCommand::Move {
            x: Some(0.0),
            y: Some(0.0),
            z: Some(0.2),
            e: Some(0.1),
            f: None,
            role: ExtrusionRole::OuterWall,
        });
    }

    let mut feedrates = std::collections::HashSet::new();
    let mut has_high_speed = false;
    for cmd in &commands {
        if let GCodeCommand::Move { f: Some(f_val), .. } = cmd {
            feedrates.insert(f_val.to_bits());
            if *f_val > 600.0 {
                has_high_speed = true;
            }
        }
    }
    assert!(
        feedrates.len() < 2 || !has_high_speed,
        "Regression case (Moves with f: None, only F25 from retracts) must fail the distinct-F-set predicate"
    );
}

#[test]
fn rejects_stale_f_window() {
    // Negative AC: a regressed emit path emits a long run of print Moves with no
    // F-token. The "F within preceding 200 lines" predicate must reject when the
    // window exceeds 200.
    let mut commands = Vec::new();
    for _ in 0..250 {
        commands.push(GCodeCommand::Move {
            x: Some(0.0),
            y: Some(0.0),
            z: Some(0.2),
            e: Some(0.1),
            f: None,
            role: ExtrusionRole::OuterWall,
        });
    }

    let mut moves_since_last_f: usize = 0;
    let mut max_window: usize = 0;
    for cmd in &commands {
        if let GCodeCommand::Move { f, .. } = cmd {
            if f.is_some() {
                moves_since_last_f = 0;
            } else {
                moves_since_last_f += 1;
            }
            max_window = max_window.max(moves_since_last_f);
        }
    }
    assert!(
        max_window > 200,
        "Stale-F-window predicate should detect a > 200-move gap; saw max window of {}",
        max_window
    );
}

#[test]
fn filament_ironing_overrides_global_ironing() {
    let config = slicer_ir::FeedrateConfig {
        ironing_speed: 20.0,
        filament_ironing_speed: 40.0,
        ..Default::default()
    };

    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config);
    let resolved = emitter
        .resolve_feedrate(&ExtrusionRole::Ironing, 1.0, None)
        .unwrap();
    assert_eq!(resolved, 40.0 * 60.0);
}

#[test]
fn wipe_speed_resolves_correctly() {
    let config = slicer_ir::FeedrateConfig {
        wipe_speed: 96.0,
        ..Default::default()
    };

    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config);
    let resolved = emitter
        .resolve_feedrate(&ExtrusionRole::Custom("Wipe".to_string()), 1.0, None)
        .unwrap();
    assert_eq!(resolved, 96.0 * 60.0);
}
