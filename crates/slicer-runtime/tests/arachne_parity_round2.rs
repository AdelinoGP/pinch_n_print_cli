//! # Arachne parity audit round 3 — red TDD tests against `parity/arachne`.
//!
//! Three RED parity-gap tests (G12, G15, G20) for the `parity/arachne` branch.
//! Every test in this file fails on purpose on the current tree, panicking
//! with a message of the form:
//!
//! `PARITY GAP: <feature> | expected: <orcaslicer behavior> | got: <current
//! behavior> | ref: <OrcaSlicer path:line>`
//!
//! The failure message *is* the deliverable. Do not `#[ignore]`, weaken, or
//! delete these tests to get a green build — each one is closed by
//! implementing the named OrcaSlicer behavior.
//!
//! Coordinate convention (`docs/08_coordinate_system.md`): 1 unit = 100 nm =
//! 10⁻⁴ mm. All config keys are snake_case.

#![allow(dead_code)]

#[path = "fixtures/arachne_parity/mod.rs"]
mod fixtures;

use slicer_core::arachne::{run_arachne_pipeline, simplify_toolpaths, ArachneParams};
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_ir::{ExtrusionLine, Point2};

// ===========================================================================
// G12 — wall region ordering: inner (odd) region must follow enclosing even
// region (OrcaSlicer `WallToolPaths::getRegionOrder`; PerimeterGenerator's
// finalized-extrusion walk).
// ===========================================================================

/// G12: drive `run_arachne_pipeline` with two concentric square islands and
/// assert the outer-wall (`inset_idx == 0`) `ExtrusionLine`s precede the
/// inner-wall (`inset_idx >= 1`) ones in the returned `Vec`.
///
/// The direct-core fixture establishes only finalized-line ordering. Separate
/// guest-host and end-to-end tests cover three-state wall-sequence propagation.
#[test]
fn arachne_parity_wall_region_order_odd_after_enclosing() {
    let params = ArachneParams {
        outer_to_inner: true,
        ..ArachneParams::default()
    };

    let lines = match run_arachne_pipeline(
        &fixtures::ex_polygons_concentric_islands_mm(),
        &params,
        false,
    ) {
        Ok((lines, _inner_contours)) => lines,
        Err(_) => panic!(
            "PARITY GAP: wall region order odd-after-enclosing | expected: \
             emitted wall regions ordered so inner (odd) region follows its \
              enclosing even region (WallToolPaths::getRegionOrder) | got: \
              pipeline did not produce finalized region order | observed \
              first_inner=N/A last_outer=N/A"
        ),
    };

    if lines.is_empty() {
        panic!(
            "PARITY GAP: wall region order odd-after-enclosing | expected: \
             emitted wall regions ordered so inner (odd) region follows its \
              enclosing even region (WallToolPaths::getRegionOrder) | got: \
              pipeline returned no finalized lines | observed \
              first_inner=N/A last_outer=N/A"
        );
    }

    // First index of any inner wall and last index of any outer wall.
    let first_inner = lines.iter().position(|l| l.inset_idx >= 1);
    let last_outer = lines.iter().rposition(|l| l.inset_idx == 0);

    let (first_inner, last_outer) = match (first_inner, last_outer) {
        (Some(o), Some(i)) => (o, i),
        _ => panic!(
            "PARITY GAP: wall region order odd-after-enclosing | expected: \
             emitted wall regions ordered so inner (odd) region follows its \
              enclosing even region (WallToolPaths::getRegionOrder) | got: \
              pipeline omitted an outer or inner line | observed \
              first_inner={first_inner:?} last_outer={last_outer:?}"
        ),
    };

    // The first inner line must follow the last enclosing outer line.
    assert!(
        first_inner > last_outer,
        "PARITY GAP: wall region order odd-after-enclosing | expected: \
         emitted wall regions ordered so inner (odd) region follows its \
          enclosing even region (WallToolPaths::getRegionOrder) | got: \
          finalized pipeline order violates the enclosing-region precedence | observed \
          first_inner={first_inner} last_outer={last_outer}"
    );
}

// ===========================================================================
// G15 — BeadingStrategy::get_split_middle_threshold exposed and consumed by
// RedistributeBeadingStrategy.
// ===========================================================================

/// G15 (TDD-red → closed): `BeadingStrategy::get_split_middle_threshold` and
/// `get_add_middle_threshold` must exist on the `BeadingStrategy` trait and be
/// observable on the `Limited` top of a fully-decorated stack built by
/// `BeadingStrategyFactory::create_stack`.
///
/// OrcaSlicer ref: `BeadingStrategy.hpp:97`
/// (`getSplitMiddleThreshold(lower_bead_count)`); `BeadingStrategy.cpp:54-57`
/// (consumed by `RedistributeBeadingStrategy` to pick the optimal bead count).
#[test]
fn arachne_parity_beading_split_middle_threshold_exposed() {
    // AC-2: G15. The factory-computed thresholds must be observable on the
    // `Limited` top of a fully-decorated stack. The previous `assert!(false)`
    // body is replaced per the test's own doc note at lines 120-132 of this
    // file.
    let params = BeadingFactoryParams {
        print_thin_walls: true,
        outer_wall_offset: 1.0,
        ..BeadingFactoryParams::default()
    };
    let stack = BeadingStrategyFactory::create_stack(&params);

    let split = stack.get_split_middle_threshold();
    let add = stack.get_add_middle_threshold();
    assert_eq!(
        split, 0.99,
        "AC-2 G15: get_split_middle_threshold on Limited top must equal factory-computed 0.99"
    );
    assert_eq!(
        add, 0.99,
        "AC-2 G15: get_add_middle_threshold on Limited top must equal factory-computed 0.99"
    );
}

// ===========================================================================
// G20 — simplify: intersection-distance gate preserves near-colinear middle
// junctions whose chord-intersection lies too far from neighbors.
// ===========================================================================

/// G20: build an `ExtrusionLine` from
/// `fixtures::simplify_input_intersection_distance_gate()` (a thin "Z" polyline
/// of four junctions) and run `simplify_toolpaths` with parameters that place
/// the middle junctions *inside* the intersection-distance gate. OrcaSlicer's
/// `ExtrusionLine::simplify` rejects removal when the intersection of the
/// extended `(prev, curr)` lines lies more than
/// `smallest_line_segment_squared` from either neighbor, so the middle
/// junctions are PRESERVED (4 junctions remain).
///
/// OrcaSlicer ref: `Arachne/utils/ExtrusionLine.cpp:163-175`.
#[test]
fn arachne_parity_simplify_intersection_distance_gate_present() {
    let line = ExtrusionLine {
        junctions: fixtures::simplify_input_intersection_distance_gate(),
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };
    let expected: Vec<(f64, f64)> = line
        .junctions
        .iter()
        .map(|j| (j.p.x as f64, j.p.y as f64))
        .collect();

    // AC-6: G20. The previous `smallest_line_segment_squared = 0.0` made the
    // tier-3 gate (`ExtrusionLine.cpp:162-164`) reduce to `length2 < 0` —
    // unsatisfiable for every input — so the intersection/`dist_greater` path
    // (`:166-220`) was dead and the old test could not have exercised the gate
    // it names. The new parameters place junction 2 inside the gate. **The
    // assertion is strengthened, never weakened.**
    let result = simplify_toolpaths(vec![line], 0.01, 1e-3, 1.0, f64::INFINITY);

    assert!(
        !result.is_empty(),
        "AC-6 G20: simplify must return at least one ExtrusionLine"
    );

    let kept = result[0].junctions.len();
    assert_eq!(
        kept, 4,
        "AC-6 G20: intersection-distance gate must preserve all 4 junctions; observed {kept}"
    );

    // Exact junction-sequence check: the four original junctions must be
    // preserved unchanged (the middle two survive the dist_greater gate).
    let got: Vec<(f64, f64)> = result[0]
        .junctions
        .iter()
        .map(|j| (j.p.x as f64, j.p.y as f64))
        .collect();
    assert_eq!(
        got, expected,
        "AC-6 G20: preserved junction sequence must exactly match the fixture input"
    );

    // Touch Point2 so the import is meaningful for coordinate hygiene.
    let _ = Point2::from_mm(0.0, 0.0);
}
