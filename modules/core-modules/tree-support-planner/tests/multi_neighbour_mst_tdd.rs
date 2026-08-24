//! Tests for `tree_support_planner::neighbour_direction_sum` — the Rust port
//! of canonical `TreeSupport::drop_nodes`' `move_to_neighbor_center`
//! accumulator (`sum_direction += direction * (1 / dist2)`).
//!
//! **Packet 224 step 5 (F-13) rewrite.** These tests were authored against
//! `aggregate_neighbour_targets`, which returned a 1/d^2-weighted *mean of
//! neighbour positions*; the module then stepped from the node toward that
//! mean under a fractional `max_move_xy` cap. Canonical has no such mean and
//! no such cap: it accumulates weighted *directions* and then takes a step of
//! exactly `get_max_move_dist(&node)` along the result. Only the direction is
//! ever observed.
//!
//! The two forms agree on direction — `mean - node` equals `sum / total_weight`
//! and the weight total is positive — so each assertion below is the direction
//! restatement of the one it replaces, not a weakened version of it. The one
//! genuinely retired expectation is the "single neighbour returns that
//! neighbour's position" case: a sum of directions cannot return a position.
//! It is restated as "the direction points at that neighbour".
//!
//! Weighting = `1.0 / D_j^2`, with coincident neighbours (`D_j^2` at or below
//! the f64 epsilon) skipped rather than saturating to infinity.

use tree_support_planner::neighbour_direction_sum;

/// Unit direction of a vector, for comparing directions independently of the
/// step length canonical later rescales them to.
fn unit(v: (f32, f32)) -> (f32, f32) {
    let n = (v.0 * v.0 + v.1 * v.1).sqrt();
    assert!(n > 1e-9, "expected a non-degenerate direction, got {v:?}");
    (v.0 / n, v.1 / n)
}

/// AC-2: A symmetric 3-neighbour fan (one central node at the origin, three
/// neighbours at equal distance arranged at 0°, 120°, 240°) contributes three
/// equal-weight directions that cancel. The node has nowhere to converge to,
/// so the accumulated direction is zero.
///
/// This is the direction form of the old "aggregates to the geometric
/// centroid" assertion: the centroid of those three neighbours IS the node's
/// own position, so the displacement toward it is the zero vector.
#[test]
fn symmetric_3_neighbour_fan_cancels_to_zero() {
    let r = 5.0_f32;
    let positions: [(f32, f32); 3] = [
        (r, 0.0),
        (r * (-0.5_f32), r * 0.866_025_4),
        (r * (-0.5_f32), r * (-0.866_025_4)),
    ];

    let (sx, sy) = neighbour_direction_sum((0.0, 0.0), &positions);

    // Each direction has magnitude 1/r; three of them cancel to well under a
    // thousandth of one contribution.
    let tol = 1e-3 / r;
    assert!(
        sx.abs() < tol && sy.abs() < tol,
        "symmetric fan must cancel: sum=({sx}, {sy}) tol={tol}"
    );
}

/// AC-3: An asymmetric 3-neighbour arrangement (one close neighbour at 1 mm,
/// two far neighbours at 5 mm) weights the close neighbour 25x more heavily
/// under 1/d^2, so the accumulated direction points toward it: `+x`.
#[test]
fn asymmetric_neighbours_weighted_by_reciprocal_squared() {
    let positions: [(f32, f32); 3] = [(1.0, 0.0), (-5.0, 0.5), (-5.0, -0.5)];

    let (sx, _sy) = neighbour_direction_sum((0.0, 0.0), &positions);

    // weight(1mm) = 1.0, weight(~5mm) = 0.04 each. x-component:
    // 1*1.0 + (-5)*0.04 + (-5)*0.04 ≈ 0.6 before the 1/d^2 division that is
    // already folded in — the sign is what matters.
    assert!(
        sx > 0.0,
        "close neighbour (1mm) must dominate: sum.x={sx} should be > 0"
    );
}

/// AC-N1: Single-neighbour case. The accumulated direction points exactly at
/// that neighbour. (Previously: "the aggregate target equals that neighbour's
/// position" — the same statement, expressed as a direction because canonical
/// never forms a target position at all.)
#[test]
fn single_neighbour_direction_points_at_that_neighbour() {
    let node = (1.0_f32, 2.0_f32);
    let positions: [(f32, f32); 1] = [(3.5, -7.25)];

    let sum = neighbour_direction_sum(node, &positions);
    let got = unit(sum);
    let want = unit((positions[0].0 - node.0, positions[0].1 - node.1));

    assert!(
        (got.0 - want.0).abs() < 1e-6 && (got.1 - want.1).abs() < 1e-6,
        "single-neighbour direction {got:?} must equal {want:?}"
    );
}

/// AC-N2: A coincident neighbour (zero distance) does not panic and does not
/// poison the sum with a NaN/infinity. Canonical divides by `dist2`, so a
/// coincident neighbour is skipped; the remaining neighbour decides the
/// direction.
///
/// The old assertion — "the target collapses to the coincident neighbour's
/// position" — cannot survive the rewrite: that neighbour IS the node, so
/// collapsing onto it means not moving, which in direction form is the same as
/// contributing nothing. Skipping is the faithful restatement, and it is
/// strictly more informative because it still yields a usable direction.
#[test]
fn coincident_neighbour_is_skipped_not_infinite() {
    let node = (0.0_f32, 0.0_f32);
    let positions: [(f32, f32); 2] = [(0.0, 0.0), (10.0, 10.0)];

    let (sx, sy) = neighbour_direction_sum(node, &positions);

    assert!(
        sx.is_finite() && sy.is_finite(),
        "coincident neighbour must not produce a non-finite sum: ({sx}, {sy})"
    );
    let got = unit((sx, sy));
    let want = unit((10.0, 10.0));
    assert!(
        (got.0 - want.0).abs() < 1e-6 && (got.1 - want.1).abs() < 1e-6,
        "the live neighbour must decide the direction: {got:?} vs {want:?}"
    );
}
