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
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                Point3WithWidth {
                    x: 10.0,
                    y: *entity_id as f32,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
            ],
            role: role.clone(),
            speed_factor: 1.0,
        };
        layer.ordered_entities.push(PrintEntity {
            entity_id: *entity_id,
            path,
            role: role.clone(),
            tool_index: *entity_id as u32,
            region_key: RegionKey {
                region_id: *entity_id,
                global_layer_index: 0,
                object_id: "obj".to_string(),
                variant_chain: Vec::new(),
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
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            },
            Point3WithWidth {
                x: 10.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            },
        ],
        role: ExtrusionRole::OuterWall,
        speed_factor: 0.5,
    };
    layer.ordered_entities.push(PrintEntity {
        entity_id: 1,
        path,
        role: ExtrusionRole::OuterWall,
        tool_index: 0,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
            variant_chain: Vec::new(),
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
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            }],
            role: ExtrusionRole::InnerWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::InnerWall,
        tool_index: 0,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
            variant_chain: Vec::new(),
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
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                Point3WithWidth {
                    x: 10.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        tool_index: 0,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
            variant_chain: Vec::new(),
        },
        topo_order: 0,
    });

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
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                Point3WithWidth {
                    x: 10.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        tool_index: 0,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
            variant_chain: Vec::new(),
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
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                Point3WithWidth {
                    x: 10.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        tool_index: 0,
        region_key: RegionKey {
            region_id: 0,
            global_layer_index: 0,
            object_id: "obj".to_string(),
            variant_chain: Vec::new(),
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
        .resolve_feedrate(&ExtrusionRole::Ironing, 1.0)
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
        .resolve_feedrate(&ExtrusionRole::Custom("Wipe".to_string()), 1.0)
        .unwrap();
    assert_eq!(resolved, 96.0 * 60.0);
}

// =============================================================================
// Packet 189 - per-point speed factor carrier (EntitySpeedProfile)
// =============================================================================

/// Build a `Point3WithWidth` at (x, y) on the z=0.2 plane.
fn p189_point(x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
        overhang_distance_mm: None,
    }
}

/// Build a `PrintEntity` from an explicit point list.
fn p189_entity(entity_id: u64, role: ExtrusionRole, points: Vec<Point3WithWidth>) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        tool_index: 0,
        region_key: RegionKey {
            region_id: entity_id,
            global_layer_index: 0,
            object_id: "obj".to_string(),
            variant_chain: Vec::new(),
        },
        topo_order: 0,
    }
}

/// A `ResolvedConfig` that disables BOTH simplification passes, so emitted
/// `Move`s map 1:1 onto the input points.
fn p189_no_simplification_config() -> ResolvedConfig {
    ResolvedConfig {
        gcode_resolution: 0.0,
        infill_resolution: 0.0,
        min_segment_length: 0.0,
        ..Default::default()
    }
}

/// Collect the `f` value of every `Move` carrying the given role.
fn p189_f_values(gcode_ir: &GCodeIR, want_role: &ExtrusionRole) -> Vec<f32> {
    gcode_ir
        .commands
        .iter()
        .filter_map(|cmd| match cmd {
            GCodeCommand::Move {
                f: Some(f_val),
                role,
                ..
            } if role == want_role => Some(*f_val),
            _ => None,
        })
        .collect()
}

/// A `speed_profiles` row supplies one factor per point, so a single entity
/// emits several distinct F tokens instead of one repeated whole-entity value.
#[test]
fn per_point_speed_profile_varies_f_within_one_entity() {
    let entity = p189_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![
            p189_point(0.0, 0.0),
            p189_point(5.0, 0.0),
            p189_point(10.0, 0.0),
            p189_point(15.0, 0.0),
        ],
    );

    let layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity],
        speed_profiles: vec![EntitySpeedProfile {
            entity_id: 1,
            factors: vec![1.0, 0.5, 0.5, 0.25],
        }],
        ..Default::default()
    };

    let config = FeedrateConfig {
        outer_wall_speed: 60.0,
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config)
        .with_resolved_config(p189_no_simplification_config());
    let gcode_ir = emitter.emit_gcode(&[layer]).unwrap();

    let fs = p189_f_values(&gcode_ir, &ExtrusionRole::OuterWall);
    assert_eq!(
        fs,
        vec![3600.0f32, 1800.0, 1800.0, 900.0],
        "each point F must be base_speed*60*factors[i], got {:?}",
        fs
    );

    let mut distinct: Vec<f32> = fs.clone();
    distinct.sort_by(|a, b| a.partial_cmp(b).unwrap());
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        3,
        "expected three distinct F values across the entity, got {:?}",
        fs
    );
}

/// The profile is indexed by the point ORIGINAL index. With `gcode_resolution`
/// at 0.0 the Douglas-Peucker pass is skipped entirely, so the ONLY simplification
/// is `drop_short_segments_mm`: interior point index 2 sits 0.02 mm from index 1,
/// below the 0.05 mm `min_segment_length`, so it is dropped. The surviving points
/// (original indices 0, 1, 3, 4) must read factors[0], [1], [3], [4] - not the
/// first four entries of the factor vector.
#[test]
fn per_point_speed_profile_indexes_original_points_after_simplification() {
    let entity = p189_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![
            p189_point(0.0, 0.0),  // idx 0 - kept (first)
            p189_point(5.0, 0.0),  // idx 1 - kept
            p189_point(5.02, 0.0), // idx 2 - DROPPED (0.02 mm < min_segment_length 0.05)
            p189_point(10.0, 0.0), // idx 3 - kept
            p189_point(15.0, 0.0), // idx 4 - kept (last)
        ],
    );

    let layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity],
        speed_profiles: vec![EntitySpeedProfile {
            entity_id: 1,
            factors: vec![1.0, 0.5, 0.9, 0.25, 0.75],
        }],
        ..Default::default()
    };

    let config = FeedrateConfig {
        outer_wall_speed: 60.0,
        ..Default::default()
    };
    // D-P off (gcode_resolution 0.0); min-segment pruning ON at its 0.05 mm default.
    let resolved = ResolvedConfig {
        gcode_resolution: 0.0,
        min_segment_length: 0.05,
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config)
        .with_resolved_config(resolved);
    let gcode_ir = emitter.emit_gcode(&[layer]).unwrap();

    let fs = p189_f_values(&gcode_ir, &ExtrusionRole::OuterWall);
    assert_eq!(
        fs.len(),
        4,
        "interior point index 2 must be dropped by min-segment pruning, got {:?}",
        fs
    );
    // ORIGINAL-index reading: factors[0], [1], [3], [4].
    assert_eq!(
        fs,
        vec![3600.0f32, 1800.0, 900.0, 2700.0],
        "surviving points must read factors[original_index], got {:?}",
        fs
    );
    // Position-in-kept reading would have produced factors[0..4].
    assert_ne!(
        fs,
        vec![3600.0f32, 1800.0, 3240.0, 900.0],
        "profile must NOT be indexed by position among the kept points"
    );
}

/// An entity with no `speed_profiles` row in a layer that has one keeps the exact
/// pre-packet behaviour: a single F from `resolve_feedrate(role, path.speed_factor)`.
#[test]
fn unprofiled_entity_in_a_profiled_layer_keeps_whole_entity_speed() {
    let profiled = p189_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![
            p189_point(0.0, 0.0),
            p189_point(5.0, 0.0),
            p189_point(10.0, 0.0),
        ],
    );
    let mut unprofiled = p189_entity(
        2,
        ExtrusionRole::InnerWall,
        vec![
            p189_point(0.0, 5.0),
            p189_point(5.0, 5.0),
            p189_point(10.0, 5.0),
        ],
    );
    unprofiled.path.speed_factor = 0.5;

    let layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![profiled, unprofiled],
        speed_profiles: vec![EntitySpeedProfile {
            entity_id: 1,
            factors: vec![1.0, 0.5, 0.25],
        }],
        ..Default::default()
    };

    let config = FeedrateConfig {
        outer_wall_speed: 60.0,
        inner_wall_speed: 60.0,
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config)
        .with_resolved_config(p189_no_simplification_config());
    let gcode_ir = emitter.emit_gcode(&[layer]).unwrap();

    // Entity 1 (profiled) varies.
    let profiled_fs = p189_f_values(&gcode_ir, &ExtrusionRole::OuterWall);
    assert_eq!(
        profiled_fs,
        vec![3600.0f32, 1800.0, 900.0],
        "profiled entity F must follow its per-point factors, got {:?}",
        profiled_fs
    );

    // Entity 2 (unprofiled) is flat at the whole-entity value: 60 * 60 * 0.5.
    let expected = emitter
        .resolve_feedrate(&ExtrusionRole::InnerWall, 0.5)
        .expect("InnerWall feedrate should resolve");
    let unprofiled_fs = p189_f_values(&gcode_ir, &ExtrusionRole::InnerWall);
    assert_eq!(
        unprofiled_fs.len(),
        3,
        "unprofiled entity should emit one Move per point, got {:?}",
        unprofiled_fs
    );
    for f in &unprofiled_fs {
        assert_eq!(
            *f, expected,
            "unprofiled entity must keep the whole-entity F {}, got {:?}",
            expected, unprofiled_fs
        );
    }
    assert_eq!(expected, 1800.0, "sanity: 60 mm/s * 60 * 0.5 = F1800");
}

/// A present per-point factor REPLACES `entity.path.speed_factor` for that point;
/// it is NOT composed with (multiplied by) it.
///
/// Every other packet-189 fixture uses `path.speed_factor == 1.0`, where replace
/// and multiply are indistinguishable. Here the whole-entity factor is 0.5, so the
/// two readings diverge on every point:
///
/// | idx | profile factor | REPLACE (correct) | MULTIPLY (regression) |
/// |-----|----------------|-------------------|-----------------------|
/// |  0  | 1.0            | 3600              | 1800                  |
/// |  1  | 0.8            | 2880              | 1440                  |
/// |  2  | 0.4            | 1440              |  720                  |
/// |  3  | 0.2            |  720              |  360                  |
///
/// No value is clamped under either reading (the smallest composed factor is
/// 0.1, still above the 0.05 lower clamp), so the clamp cannot mask the
/// difference. If `emit.rs`'s `unwrap_or(entity.path.speed_factor)` were turned
/// into a multiplication, this test fails on the very first element.
#[test]
fn per_point_profile_replaces_rather_than_scales_whole_entity_speed_factor() {
    let mut entity = p189_entity(
        1,
        ExtrusionRole::OuterWall,
        vec![
            p189_point(0.0, 0.0),
            p189_point(5.0, 0.0),
            p189_point(10.0, 0.0),
            p189_point(15.0, 0.0),
        ],
    );
    // Deliberately NOT 1.0: this is the value the per-point factors must override.
    entity.path.speed_factor = 0.5;

    let layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity],
        speed_profiles: vec![EntitySpeedProfile {
            entity_id: 1,
            factors: vec![1.0, 0.8, 0.4, 0.2],
        }],
        ..Default::default()
    };

    let config = FeedrateConfig {
        outer_wall_speed: 60.0,
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config("1.0".to_string(), config)
        .with_resolved_config(p189_no_simplification_config());
    let gcode_ir = emitter.emit_gcode(&[layer]).unwrap();

    let fs = p189_f_values(&gcode_ir, &ExtrusionRole::OuterWall);

    // Expected: resolve_feedrate(role, factors[i]) - the per-point factor ALONE.
    let expected_replace: Vec<f32> = [1.0f32, 0.8, 0.4, 0.2]
        .iter()
        .map(|f| {
            emitter
                .resolve_feedrate(&ExtrusionRole::OuterWall, *f)
                .expect("OuterWall feedrate should resolve")
        })
        .collect();
    assert_eq!(
        expected_replace,
        vec![3600.0f32, 2880.0, 1440.0, 720.0],
        "sanity: 60 mm/s * 60 * factor"
    );
    assert_eq!(
        fs, expected_replace,
        "per-point factor must REPLACE path.speed_factor (0.5), got {:?}",
        fs
    );

    // The composition reading (factor * path.speed_factor) must NOT be observed.
    let expected_multiply: Vec<f32> = [1.0f32, 0.8, 0.4, 0.2]
        .iter()
        .map(|f| {
            emitter
                .resolve_feedrate(&ExtrusionRole::OuterWall, *f * 0.5)
                .expect("OuterWall feedrate should resolve")
        })
        .collect();
    assert_eq!(
        expected_multiply,
        vec![1800.0f32, 1440.0, 720.0, 360.0],
        "sanity: the composition variant halves every F"
    );
    assert_ne!(
        fs, expected_multiply,
        "per-point factor must NOT be multiplied by path.speed_factor"
    );
    // Element-wise: the two readings differ at every single point, so no partial
    // regression can slip through a whole-vector comparison.
    for (i, (r, m)) in expected_replace.iter().zip(&expected_multiply).enumerate() {
        assert_ne!(r, m, "readings must diverge at point {}", i);
        assert_eq!(fs[i], *r, "point {} must read the replace value", i);
    }
}
