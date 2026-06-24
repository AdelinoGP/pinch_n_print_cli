#![allow(missing_docs)]

//! Standalone TDD tests for `slicer_core::perimeter_utils::wall_sequence_reorder`
//! (AC-2, packet 105).
//!
//! Tests the public reorder function via the `slicer_core::perimeter_utils` API.
//! Input WallLoops are constructed the same way as the inline `#[cfg(test)]` tests
//! inside `perimeter_utils.rs`.

use slicer_core::perimeter_utils::{wall_sequence_reorder, WallSequence};
use slicer_ir::{ExtrusionPath3D, ExtrusionRole, LoopType, WallBoundaryType, WallLoop};

fn make_wall(perimeter_index: u32, loop_type: LoopType, role: ExtrusionRole) -> WallLoop {
    WallLoop {
        perimeter_index,
        loop_type,
        path: ExtrusionPath3D {
            points: vec![],
            role,
            speed_factor: 1.0,
        },
        width_profile: Default::default(),
        feature_flags: Default::default(),
        boundary_type: WallBoundaryType::ExteriorSurface,
    }
}

/// Helper: build a 3-wall set (1 outer + 2 inner) as used in all three sequence tests.
/// Indices: 0 = outer, 1 = inner_0, 2 = inner_1.
fn three_wall_set() -> Vec<WallLoop> {
    vec![
        make_wall(0, LoopType::Outer, ExtrusionRole::OuterWall),
        make_wall(1, LoopType::Inner, ExtrusionRole::InnerWall),
        make_wall(2, LoopType::Inner, ExtrusionRole::InnerWall),
    ]
}

/// AC-2a: `InnerOuter` — canonical order is `[Outer, Inner_0, Inner_1]`.
/// Generation already produces this order; reorder must be a no-op.
#[test]
fn inner_outer_canonical_order() {
    let mut walls = three_wall_set();
    wall_sequence_reorder(&mut walls, WallSequence::InnerOuter, &[]);

    // Verify roles in order
    assert_eq!(walls[0].loop_type, LoopType::Outer, "slot 0 must be Outer");
    assert_eq!(walls[1].loop_type, LoopType::Inner, "slot 1 must be Inner");
    assert_eq!(walls[2].loop_type, LoopType::Inner, "slot 2 must be Inner");

    // Verify identity (no mutation beyond order)
    assert_eq!(walls[0].perimeter_index, 0);
    assert_eq!(walls[1].perimeter_index, 1);
    assert_eq!(walls[2].perimeter_index, 2);
}

/// AC-2b: `OuterInner` — reversed order is `[Inner_1, Inner_0, Outer]`.
/// Outer wall goes last; innermost wall goes first.
#[test]
fn outer_inner_reversed_order() {
    let mut walls = three_wall_set();
    wall_sequence_reorder(&mut walls, WallSequence::OuterInner, &[]);

    // After full reverse: [2, 1, 0]
    assert_eq!(walls[0].loop_type, LoopType::Inner, "slot 0 must be Inner");
    assert_eq!(walls[1].loop_type, LoopType::Inner, "slot 1 must be Inner");
    assert_eq!(walls[2].loop_type, LoopType::Outer, "slot 2 must be Outer");

    // Exact identity
    assert_eq!(
        walls[0].perimeter_index, 2,
        "innermost inner (index 2) goes first"
    );
    assert_eq!(
        walls[1].perimeter_index, 1,
        "first inner (index 1) goes second"
    );
    assert_eq!(walls[2].perimeter_index, 0, "outer (index 0) goes last");
}

/// AC-2c: `InnerOuterInner` (sandwich) — `[Inner_0, Outer, Inner_1]`.
/// First inner is emitted first, then the outer, then remaining inners.
#[test]
fn inner_outer_inner_sandwich_order() {
    let mut walls = three_wall_set();
    wall_sequence_reorder(&mut walls, WallSequence::InnerOuterInner, &[]);

    assert_eq!(walls[0].loop_type, LoopType::Inner, "slot 0 must be Inner");
    assert_eq!(walls[1].loop_type, LoopType::Outer, "slot 1 must be Outer");
    assert_eq!(walls[2].loop_type, LoopType::Inner, "slot 2 must be Inner");

    // Identity: [Inner_0=1, Outer=0, Inner_1=2]
    assert_eq!(walls[0].perimeter_index, 1, "first inner goes first");
    assert_eq!(walls[1].perimeter_index, 0, "outer goes second");
    assert_eq!(walls[2].perimeter_index, 2, "second inner goes last");
}

/// Edge: empty input is a no-op.
#[test]
fn empty_walls_is_noop() {
    let mut walls: Vec<WallLoop> = vec![];
    wall_sequence_reorder(&mut walls, WallSequence::InnerOuterInner, &[]);
    assert!(walls.is_empty());
}

/// Edge: single wall is unchanged for all modes.
#[test]
fn single_wall_unchanged_for_all_modes() {
    for mode in [
        WallSequence::InnerOuter,
        WallSequence::OuterInner,
        WallSequence::InnerOuterInner,
    ] {
        let mut walls = vec![make_wall(42, LoopType::Outer, ExtrusionRole::OuterWall)];
        wall_sequence_reorder(&mut walls, mode, &[]);
        assert_eq!(walls.len(), 1);
        assert_eq!(walls[0].perimeter_index, 42);
    }
}
