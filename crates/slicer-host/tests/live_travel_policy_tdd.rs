//! TDD: live travel policy on the host PathOptimization dispatch path.
//!
//! Verifies that `commit_layer_outputs_for_test` for `Layer::PathOptimization`
//! correctly routes Retract/Unretract decisions into the deferred-retract queue
//! and ZHop decisions into the deferred-z-hop queue — and that no-retract
//! fixtures produce no orphan entries in either queue.
//!
//! These tests exercise the HOST DISPATCH commit path via
//! `commit_layer_outputs_for_test`, not the WASM boundary. The WASM-level
//! coverage (building path-optimization-default.wasm with travel policy) is
//! handled by the rebuild gate in `build-core-modules.sh`.

#![allow(missing_docs)]

use slicer_host::commit_layer_outputs_for_test;
use slicer_host::wit_host::{
    ExtrusionRole, GcodeCommandCollected, GcodeMoveCmd, HostExecutionContext,
};
use slicer_host::LayerArena;
use slicer_ir::{LayerCollectionIR, RetractMode, SemVer};

/// Helper: make a fresh `HostExecutionContext` for PathOptimization tests.
fn make_ctx(module_id: &str) -> HostExecutionContext {
    HostExecutionContext::new(
        module_id.to_string(),
        0.2,  // layer_z
        0.2,  // effective_layer_height
        None, // catchup_z_bottom
        None, // mesh_ir
    )
}

/// Simulate what layer_executor does: flush deferred queues into LayerCollectionIR.
fn flush_to_layer_collection(arena: &mut LayerArena) -> slicer_ir::LayerCollectionIR {
    // Snapshot the staged ordered_entities for entity_id lookup (mirrors
    // production layer_executor behaviour).
    let staged_entities: Vec<_> = arena
        .layer_collection()
        .map(|lc| lc.ordered_entities.clone())
        .unwrap_or_default();

    let mut layer_collection = slicer_ir::LayerCollectionIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    };
    layer_collection.z_hops.extend(arena.take_deferred_z_hops());
    layer_collection
        .retracts
        .extend(
            arena
                .take_deferred_retracts()
                .into_iter()
                .map(|r| slicer_ir::TravelRetract {
                    after_entity_index: r.after_entity_index,
                    length: r.length,
                    speed: r.speed,
                    is_unretract: r.is_unretract,
                    mode: r.mode,
                }),
        );
    layer_collection
        .travel_moves
        .extend(arena.take_deferred_travel_moves().into_iter().map(|m| {
            slicer_ir::TravelMove {
                entity_id: staged_entities
                    .get(m.after_entity_index as usize)
                    .map(|e| e.entity_id)
                    .unwrap_or(0),
                x: m.x,
                y: m.y,
                z: m.z,
                f: m.f,
            }
        }));
    layer_collection
}

/// AC positive: retracting travel populates a matching ZHop and retract pair.
///
/// Simulates path-optimization-default emitting the OrcaSlicer canonical sequence:
///   Retract { length: 0.8 } → ZHop { after: 0, height: 0.2 } → Move(travel) → Unretract { length: 0.8 }
/// and verifies the dispatch stores them in the correct deferred queues.
#[test]
fn retracting_travel_populates_matching_z_hop_and_retract_pair() {
    let mut ctx = make_ctx("com.test.path-opt-retract");

    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Retract {
            length: 0.8,
            speed: 25.0,
            mode: RetractMode::Gcode,
        });
    ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop {
        after_entity_index: 0,
        hop_height: 0.2,
    });
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Move(GcodeMoveCmd {
            x: Some(50.0),
            y: Some(50.0),
            z: None,
            e: None,
            f: None,
            role: ExtrusionRole::Custom("travel".to_string()),
        }));
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Unretract {
            length: 0.8,
            speed: 25.0,
            mode: RetractMode::Gcode,
        });

    let mut arena = LayerArena::new();

    commit_layer_outputs_for_test(
        "Layer::PathOptimization",
        "com.test.path-opt-retract",
        0,
        &ctx,
        &mut arena,
        None,
    )
    .expect(
        "commit must succeed — Retract/ZHop/Unretract from PathOptimization must not be rejected",
    );

    let layer_collection = flush_to_layer_collection(&mut arena);

    let z_hops = &layer_collection.z_hops;
    assert_eq!(
        z_hops.len(),
        1,
        "expected exactly one ZHop, got {}: {z_hops:?}",
        z_hops.len()
    );
    assert!(
        (z_hops[0].hop_height - 0.2_f32).abs() < 1e-4,
        "ZHop hop_height must be 0.2, got {}",
        z_hops[0].hop_height
    );

    let retracts = &layer_collection.retracts;
    assert_eq!(
        retracts.len(),
        2,
        "expected Retract + Unretract = 2 entries, got {}: {retracts:?}",
        retracts.len()
    );

    let r = &retracts[0];
    assert!(!r.is_unretract, "first entry must be Retract, got {r:?}");
    assert!(
        (r.length - 0.8_f32).abs() < 1e-4,
        "Retract length must be 0.8, got {}",
        r.length
    );

    let u = &retracts[1];
    assert!(u.is_unretract, "second entry must be Unretract, got {u:?}");
    assert!(
        (u.length - 0.8_f32).abs() < 1e-4,
        "Unretract length must be 0.8, got {}",
        u.length
    );

    let travel_moves = &layer_collection.travel_moves;
    assert_eq!(
        travel_moves.len(),
        1,
        "expected exactly one deferred travel move, got {}: {travel_moves:?}",
        travel_moves.len()
    );
    assert_eq!(travel_moves[0].x, Some(50.0), "travel move x must be 50.0");
    assert_eq!(travel_moves[0].y, Some(50.0), "travel move y must be 50.0");
}

/// AC negative: no-retract policy produces no orphan retracts or stray ZHops.
///
/// Simulates a path-optimization run that emits only a marker comment (no retracts)
/// and verifies both deferred queues remain empty.
#[test]
fn no_retract_policy_emits_no_orphan_retracts_or_z_hops() {
    let mut ctx = make_ctx("com.test.path-opt-no-retract");

    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Comment(
            "path-optimization layer 0 regions=1 entities=2".to_string(),
        ));

    let mut arena = LayerArena::new();

    commit_layer_outputs_for_test(
        "Layer::PathOptimization",
        "com.test.path-opt-no-retract",
        0,
        &ctx,
        &mut arena,
        None,
    )
    .expect("commit must succeed with comment-only output");

    let layer_collection = flush_to_layer_collection(&mut arena);

    let retracts = &layer_collection.retracts;
    assert!(
        retracts.is_empty(),
        "no-retract policy must produce empty retracts in LayerCollectionIR, got {retracts:?}"
    );

    let z_hops = &layer_collection.z_hops;
    assert!(
        z_hops.is_empty(),
        "no-retract policy must produce empty z_hops in LayerCollectionIR, got {z_hops:?}"
    );

    let travel_moves = &layer_collection.travel_moves;
    assert!(
        travel_moves.is_empty(),
        "no-retract policy must produce empty travel_moves in LayerCollectionIR, got {travel_moves:?}"
    );
}

/// Determinism: same fixture dispatched twice produces byte-identical deferred queues.
#[test]
fn travel_policy_is_deterministic_across_repeated_runs() {
    let make_ctx_with_travel = || {
        let mut ctx = make_ctx("com.test.path-opt-determ");
        ctx.gcode_output
            .commands
            .push(GcodeCommandCollected::Retract {
                length: 0.5,
                speed: 30.0,
                mode: RetractMode::Gcode,
            });
        ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop {
            after_entity_index: 0,
            hop_height: 0.1,
        });
        ctx.gcode_output
            .commands
            .push(GcodeCommandCollected::Move(GcodeMoveCmd {
                x: Some(50.0),
                y: Some(50.0),
                z: None,
                e: None,
                f: None,
                role: ExtrusionRole::Custom("travel".to_string()),
            }));
        ctx.gcode_output
            .commands
            .push(GcodeCommandCollected::Unretract {
                length: 0.5,
                speed: 30.0,
                mode: RetractMode::Gcode,
            });
        ctx
    };

    let mut arena1 = LayerArena::new();
    commit_layer_outputs_for_test(
        "Layer::PathOptimization",
        "com.test.path-opt-determ",
        0,
        &make_ctx_with_travel(),
        &mut arena1,
        None,
    )
    .expect("first run must succeed");
    let layer_collection1 = flush_to_layer_collection(&mut arena1);

    let mut arena2 = LayerArena::new();
    commit_layer_outputs_for_test(
        "Layer::PathOptimization",
        "com.test.path-opt-determ",
        0,
        &make_ctx_with_travel(),
        &mut arena2,
        None,
    )
    .expect("second run must succeed");
    let layer_collection2 = flush_to_layer_collection(&mut arena2);

    assert_eq!(
        format!("{:?}", layer_collection1.z_hops),
        format!("{:?}", layer_collection2.z_hops),
        "z_hops must be byte-identical across repeated runs"
    );
    assert_eq!(
        format!("{:?}", layer_collection1.retracts),
        format!("{:?}", layer_collection2.retracts),
        "retract decisions must be byte-identical across repeated runs"
    );
    assert_eq!(
        format!("{:?}", layer_collection1.travel_moves),
        format!("{:?}", layer_collection2.travel_moves),
        "travel move destinations must be byte-identical across repeated runs"
    );
}

/// Anchor alignment: ZHop, Retract, and TravelMove share the same after_entity_index
/// when the layer has pre-staged entities (entity_count > 0).
///
/// Verifies that the dispatch normalization (MED-1 fix) routes all three deferred
/// queue entries to anchor = entity_count-1, so gcode_emit.rs emits the canonical
/// Retract→ZHop→Travel→Unretract sequence as a coherent block after the last entity.
#[test]
fn z_hop_anchor_aligns_with_retract_anchor_when_entities_present() {
    let mut ctx = make_ctx("com.test.path-opt-anchor");

    // Emit the OrcaSlicer canonical travel sequence.
    // ZHop carries an arbitrary entity index (999) that the dispatch must override.
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Retract {
            length: 0.8,
            speed: 25.0,
            mode: RetractMode::Gcode,
        });
    ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop {
        after_entity_index: 999,
        hop_height: 0.2,
    });
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Move(GcodeMoveCmd {
            x: Some(50.0),
            y: Some(50.0),
            z: None,
            e: None,
            f: None,
            role: ExtrusionRole::Custom("travel".to_string()),
        }));
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Unretract {
            length: 0.8,
            speed: 25.0,
            mode: RetractMode::Gcode,
        });

    // Pre-stage 3 entities → entity_count=3, anchor=2 (last entity index, entity_id=3).
    let make_entity = |id: u64| slicer_ir::PrintEntity {
        entity_id: id,
        path: slicer_ir::ExtrusionPath3D {
            points: vec![slicer_ir::Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            }],
            role: slicer_ir::ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: slicer_ir::ExtrusionRole::OuterWall,
        region_key: slicer_ir::RegionKey {
            global_layer_index: 0,
            object_id: String::new(),
            region_id: 0,
        },
        topo_order: 0,
    };
    let mut arena = LayerArena::new();
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![make_entity(1), make_entity(2), make_entity(3)],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    });

    commit_layer_outputs_for_test(
        "Layer::PathOptimization",
        "com.test.path-opt-anchor",
        0,
        &ctx,
        &mut arena,
        None,
    )
    .expect("commit must succeed with pre-staged entities");

    let layer_collection = flush_to_layer_collection(&mut arena);
    let expected_anchor_idx = 2u32; // entity_count=3, anchor = 3-1 = 2
    let expected_travel_entity_id = 3u64; // entity at index 2 has entity_id=3

    let z_hops = &layer_collection.z_hops;
    assert_eq!(z_hops.len(), 1, "expected one ZHop");
    assert_eq!(
        z_hops[0].after_entity_index, expected_anchor_idx,
        "ZHop must be normalized to global anchor {expected_anchor_idx}, got {}",
        z_hops[0].after_entity_index
    );

    let retracts = &layer_collection.retracts;
    assert_eq!(retracts.len(), 2, "expected Retract + Unretract");
    assert_eq!(
        retracts[0].after_entity_index, expected_anchor_idx,
        "Retract anchor must match ZHop anchor, got {}",
        retracts[0].after_entity_index
    );
    assert_eq!(
        retracts[1].after_entity_index, expected_anchor_idx,
        "Unretract anchor must match ZHop anchor, got {}",
        retracts[1].after_entity_index
    );

    let travel_moves = &layer_collection.travel_moves;
    assert_eq!(travel_moves.len(), 1, "expected one TravelMove");
    assert_eq!(
        travel_moves[0].entity_id, expected_travel_entity_id,
        "TravelMove entity_id must be the entity_id of the last entity (index 2 → id 3), got {}",
        travel_moves[0].entity_id
    );
}
