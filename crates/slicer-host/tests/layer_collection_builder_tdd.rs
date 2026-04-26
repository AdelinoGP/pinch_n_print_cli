//! TDD: host validation and application of `set-entity-order` proposals.
//!
//! Covers TASK-152g (packet 32 — `layer-collection-builder` WIT surface).
//! All tests exercise `apply_entity_order_proposal` directly; the live
//! dispatch path is covered by `path_ordering_tdd` (host fallback) and
//! by packet 33's module-side tests once `path-optimization-default`
//! migrates to call `set_entity_order`.

#![allow(missing_docs)]

use slicer_host::{apply_entity_order_proposal, LayerArena};
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth, PrintEntity, RegionKey,
    SemVer,
};

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn semver() -> SemVer {
    SemVer { major: 1, minor: 0, patch: 0 }
}

fn pt(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth { x, y, z, width: 0.4, flow_factor: 1.0 }
}

fn entity_with_points(points: Vec<Point3WithWidth>, original_idx: u32) -> PrintEntity {
    let role = ExtrusionRole::SparseInfill;
    PrintEntity {
        path: ExtrusionPath3D {
            points,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: "obj".to_string(),
            region_id: 0,
        },
        topo_order: original_idx,
    }
}

fn three_entity_arena() -> LayerArena {
    // Raw start-x: [30.0, 0.0, 10.0] — the same fixture shape as the
    // packet.spec.md acceptance criteria.
    let entities = vec![
        entity_with_points(vec![pt(30.0, 0.0, 0.2)], 0),
        entity_with_points(vec![pt(0.0, 0.0, 0.2)], 1),
        entity_with_points(vec![pt(10.0, 0.0, 0.2)], 2),
    ];
    let mut arena = LayerArena::new();
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: entities,
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    });
    arena
}

fn single_entity_arena_with_path(start: Point3WithWidth, end: Point3WithWidth) -> LayerArena {
    let entity = entity_with_points(vec![start, end], 0);
    let mut arena = LayerArena::new();
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity],
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    });
    arena
}

fn ordered_start_xs(arena: &LayerArena) -> Vec<f32> {
    arena
        .layer_collection()
        .expect("layer_collection must be staged")
        .ordered_entities
        .iter()
        .map(|e| e.path.points[0].x)
        .collect()
}

fn ordered_topo_orders(arena: &LayerArena) -> Vec<u32> {
    arena
        .layer_collection()
        .expect("layer_collection must be staged")
        .ordered_entities
        .iter()
        .map(|e| e.topo_order)
        .collect()
}

// ── Positive path: permutation applied ───────────────────────────────────────

#[test]
fn valid_permutation_is_applied_to_ordered_entities() {
    let mut arena = three_entity_arena();

    // Raw x is [30.0, 0.0, 10.0]. Proposal [(2,false),(0,false),(1,false)]
    // moves slot-2 (x=10) → first, slot-0 (x=30) → second, slot-1 (x=0) → third.
    let proposal: Vec<(u32, bool)> = vec![(2, false), (0, false), (1, false)];

    apply_entity_order_proposal(&mut arena, &proposal)
        .expect("valid proposal must apply without error");

    assert_eq!(
        ordered_start_xs(&arena),
        vec![10.0, 30.0, 0.0],
        "permutation must yield x=[10.0, 30.0, 0.0]"
    );
    assert_eq!(
        ordered_topo_orders(&arena),
        vec![0, 1, 2],
        "topo_order must be reassigned to post-permutation 0-based slot index"
    );
}

// ── Reversal flag: per-entity Vec::reverse() on path.points ──────────────────

#[test]
fn reversal_flag_reverses_path_points_in_place() {
    let mut arena =
        single_entity_arena_with_path(pt(0.0, 0.0, 0.2), pt(5.0, 0.0, 0.2));

    let proposal: Vec<(u32, bool)> = vec![(0, true)];

    apply_entity_order_proposal(&mut arena, &proposal)
        .expect("single-entity reversal proposal must apply");

    let lc = arena.layer_collection().expect("layer_collection staged");
    let entity = &lc.ordered_entities[0];
    assert_eq!(
        entity.path.points.first().expect("nonempty points").x,
        5.0,
        "after reversal first point x must be 5.0"
    );
    assert_eq!(
        entity.path.points.last().expect("nonempty points").x,
        0.0,
        "after reversal last point x must be 0.0"
    );
}

// ── Negative: duplicate index in proposal ────────────────────────────────────

#[test]
fn duplicate_index_is_rejected_with_fatal_diagnostic() {
    let mut arena = three_entity_arena();

    let proposal: Vec<(u32, bool)> = vec![(0, false), (0, false), (1, false)];

    let err = apply_entity_order_proposal(&mut arena, &proposal)
        .expect_err("duplicate index must produce an Err");

    assert!(
        err.contains("set-entity-order: duplicate index 0"),
        "diagnostic must mention the duplicate index 0; got: {err}"
    );
}

// ── Negative: out-of-range index ─────────────────────────────────────────────

#[test]
fn out_of_range_index_is_rejected_with_fatal_diagnostic() {
    let mut arena = three_entity_arena();

    let proposal: Vec<(u32, bool)> = vec![(99, false), (0, false), (1, false)];

    let err = apply_entity_order_proposal(&mut arena, &proposal)
        .expect_err("out-of-range index must produce an Err");

    assert!(
        err.contains("set-entity-order: index 99 out of range [0, 3)"),
        "diagnostic must name the offending index and the valid range; got: {err}"
    );
}

// ── Negative: wrong-length proposal ──────────────────────────────────────────

#[test]
fn wrong_length_proposal_is_rejected_with_fatal_diagnostic() {
    let mut arena = three_entity_arena();

    let proposal: Vec<(u32, bool)> = vec![(0, false), (1, false)];

    let err = apply_entity_order_proposal(&mut arena, &proposal)
        .expect_err("wrong-length proposal must produce an Err");

    assert!(
        err.contains("set-entity-order: expected 3 indices, got 2"),
        "diagnostic must name expected and received counts; got: {err}"
    );
}

// ── Negative: arena has no LayerCollectionIR staged ─────────────────────────

#[test]
fn missing_layer_collection_is_rejected() {
    let mut arena = LayerArena::new();
    let proposal: Vec<(u32, bool)> = vec![(0, false)];

    let err = apply_entity_order_proposal(&mut arena, &proposal)
        .expect_err("absent layer_collection must produce an Err");

    assert!(
        err.contains("set-entity-order: no LayerCollectionIR staged on arena"),
        "diagnostic must explain the missing staged collection; got: {err}"
    );
}

// ── Atomicity: malformed proposal leaves ordered_entities unchanged ──────────

#[test]
fn malformed_proposal_leaves_ordered_entities_unchanged() {
    // Snapshot the pre-call state.
    let mut arena = three_entity_arena();
    let before_xs = ordered_start_xs(&arena);
    let before_topo = ordered_topo_orders(&arena);

    // Try each malformed proposal flavor and confirm the arena is untouched.
    for proposal in [
        vec![(0u32, false), (0, false), (1, false)],   // duplicate
        vec![(99u32, false), (0, false), (1, false)],  // out of range
        vec![(0u32, false), (1, false)],               // wrong length
    ] {
        let err = apply_entity_order_proposal(&mut arena, &proposal)
            .expect_err("malformed proposal must produce an Err");
        assert!(
            err.starts_with("set-entity-order: "),
            "diagnostic must be prefixed with 'set-entity-order: '; got: {err}"
        );
        assert_eq!(
            ordered_start_xs(&arena),
            before_xs,
            "ordered_entities x sequence must be unchanged after rejected proposal"
        );
        assert_eq!(
            ordered_topo_orders(&arena),
            before_topo,
            "ordered_entities topo_order sequence must be unchanged after rejected proposal"
        );
    }
}
