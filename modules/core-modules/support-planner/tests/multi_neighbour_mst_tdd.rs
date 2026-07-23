//! RED-phase TDD tests for `support_planner::aggregate_neighbour_targets` —
//! the Rust port of OrcaSlicer's `TreeSupport::drop_nodes` reciprocal-distance
//! (squared) weighted aggregation (TASK-287, packet 122).
//!
//! These tests are authored BEFORE `aggregate_neighbour_targets` exists, so
//! the file is expected to FAIL TO COMPILE (unresolved import). That compile
//! error is the canonical RED state.
//!
//! The function is a pure math helper that takes neighbour (x, y) positions
//! and their distances from the central node, returning the reciprocal-distance-
//! squared weighted aggregate target. Weighting = `1.0 / D_j²`, matching Orca's
//! `drop_nodes` (1/d² aggregation in the non-`is_strong` path).
//!
//! Degenerate case: when ANY `D_j < 1e-6 mm`, the target collapses to that
//! neighbour's position (dominant weight saturates; no division by zero).

/// AC-2: A symmetric 3-neighbour fan (one central node, three neighbours at
/// equal distance arranged at 0°, 120°, 240°) must aggregate to the geometric
/// centroid of the three neighbour positions (within 1e-3 mm). With equal
/// weights (`1/d²` of equal `d`s is the same weight for all three), the
/// weighted mean equals the unweighted mean.
#[test]
fn symmetric_3_neighbour_centroid() {
    // Three neighbours at 5 mm distance, arranged at 0°, 120°, 240° around
    // an origin. The centroid of the three is at the origin.
    let r = 5.0_f32;
    let positions: [(f32, f32); 3] = [
        (r, 0.0),
        (r * (-0.5_f32), r * 0.866_025_4),
        (r * (-0.5_f32), r * (-0.866_025_4)),
    ];
    let distances: [f32; 3] = [r, r, r];

    let (cx, cy) = support_planner::aggregate_neighbour_targets(&positions, &distances)
        .expect("non-empty input must produce a target");

    let expected_x = (positions[0].0 + positions[1].0 + positions[2].0) / 3.0;
    let expected_y = (positions[0].1 + positions[1].1 + positions[2].1) / 3.0;
    assert!(
        (cx - expected_x).abs() < 1e-3,
        "cx={cx} expected={expected_x} (centroid x)"
    );
    assert!(
        (cy - expected_y).abs() < 1e-3,
        "cy={cy} expected={expected_y} (centroid y)"
    );
}

/// AC-3: An asymmetric 3-neighbour arrangement (one close neighbour at 1 mm,
/// two far neighbours at 5 mm) must weight the close neighbour much more
/// heavily under 1/d². Weight(1mm) = 1.0; weight(5mm) = 0.04. The close
/// neighbour's weight dominates by a factor of 25. The aggregate must sit
/// closer to the close neighbour than to the centroid of the far cluster.
#[test]
fn asymmetric_neighbours_weighted_by_reciprocal_squared() {
    // Close neighbour at (1, 0). Two far neighbours forming a 5mm cluster
    // around (-5, 0).
    let positions: [(f32, f32); 3] = [(1.0, 0.0), (-5.0, 0.5), (-5.0, -0.5)];
    let distances: [f32; 3] = [1.0, 5.0, 5.0];

    let (cx, _cy) = support_planner::aggregate_neighbour_targets(&positions, &distances)
        .expect("non-empty input must produce a target");

    // Centroid of the far cluster is at (-5, 0). The close neighbour is at
    // (1, 0). The aggregate's x coordinate must be > 0 (closer to 1 than to
    // -5). The far-cluster midpoint is at x=-5; cx=-5 ⇒ equal weight. With
    // 1/d² weighting, the close neighbour at x=1 dominates: weighted mean x
    // = (1*1.0 + (-5)*0.04 + (-5)*0.04) / (1.0 + 0.04 + 0.04) ≈ (1 - 0.4) / 1.08
    // ≈ 0.555. So cx > 0 is the gate.
    assert!(
        cx > 0.0,
        "close neighbour (1mm) must dominate: cx={cx} should be > 0"
    );
}

/// AC-N1: Single-neighbour degenerate case. With one element, the weighted
/// aggregate equals that single neighbour's position exactly. This is the
/// behavior the old single-neighbour code path produced.
#[test]
fn single_neighbour_degenerate_case_matches_old() {
    let positions: [(f32, f32); 1] = [(3.5, -7.25)];
    let distances: [f32; 1] = [2.0];

    let (cx, cy) = support_planner::aggregate_neighbour_targets(&positions, &distances)
        .expect("single-element input must produce a target");

    assert!(
        (cx - 3.5).abs() < 1e-9,
        "single-neighbour cx={cx} expected=3.5"
    );
    assert!(
        (cy - (-7.25)).abs() < 1e-9,
        "single-neighbour cy={cy} expected=-7.25"
    );
}

/// AC-N2: A zero-distance neighbour (coincident point) does not panic. The
/// dominant weight saturates to infinity; the implementation MUST short-
/// circuit (per design.md: "D_j < 1e-6 mm → collapse to that neighbour's
/// position"). The result is the coincident neighbour's position.
#[test]
fn zero_distance_neighbour_does_not_panic() {
    let positions: [(f32, f32); 2] = [(0.0, 0.0), (10.0, 10.0)];
    let distances: [f32; 2] = [0.0, 5.0];

    let (cx, cy) = support_planner::aggregate_neighbour_targets(&positions, &distances)
        .expect("non-empty input must produce a target");

    assert!(
        (cx - 0.0).abs() < 1e-9,
        "zero-distance neighbour collapses: cx={cx} expected=0.0"
    );
    assert!(
        (cy - 0.0).abs() < 1e-9,
        "zero-distance neighbour collapses: cy={cy} expected=0.0"
    );
}
