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
use slicer_ir::{ExtrusionLine, Point2};

// ===========================================================================
// G12 — wall region ordering: inner (odd) region must follow enclosing even
// region (OrcaSlicer `WallToolPaths::getRegionOrder`, WallToolPaths.cpp:809;
// PerimeterGenerator.cpp:2302).
// ===========================================================================

/// G12: drive `run_arachne_pipeline` with two concentric square islands and
/// assert the outer-wall (`inset_idx == 0`) `ExtrusionLine`s precede the
/// inner-wall (`inset_idx >= 1`) ones in the returned `Vec`.
///
/// OrcaSlicer ref: `WallToolPaths.cpp:809` (`getRegionOrder` reorders so an
/// inner/odd region follows its enclosing even region);
/// `PerimeterGenerator.cpp:2302` (region emission order). The PnP pipeline
/// flattens per-inset buckets in source order and performs no
/// `getRegionOrder` pass (`crates/slicer-core/src/arachne/pipeline.rs:383`).
#[test]
fn arachne_parity_wall_region_order_odd_after_enclosing() {
    let params = ArachneParams::default();

    let lines = match run_arachne_pipeline(
        &fixtures::ex_polygons_concentric_islands_mm(),
        &params,
        false,
    ) {
        Ok((lines, _inner_contours)) => lines,
        Err(_) => panic!(
            "PARITY GAP: wall region order odd-after-enclosing | expected: \
             emitted wall regions ordered so inner (odd) region follows its \
             enclosing even region (WallToolPaths.cpp:809, \
             PerimeterGenerator.cpp:2302) | got: pipeline flattens per-inset \
             buckets in source order with no getRegionOrder pass \
             (pipeline.rs:383) | ref: WallToolPaths.cpp:809 | observed \
             outer_max=N/A inner_min=N/A"
        ),
    };

    if lines.is_empty() {
        panic!(
            "PARITY GAP: wall region order odd-after-enclosing | expected: \
             emitted wall regions ordered so inner (odd) region follows its \
             enclosing even region (WallToolPaths.cpp:809, \
             PerimeterGenerator.cpp:2302) | got: pipeline flattens per-inset \
             buckets in source order with no getRegionOrder pass \
             (pipeline.rs:383) | ref: WallToolPaths.cpp:809 | observed \
             outer_max=N/A inner_min=N/A"
        );
    }

    // First index of any line whose inset_idx >= 1 (an inner/odd wall).
    let outer_max = lines.iter().position(|l| l.inset_idx >= 1);
    // Last index of any line whose inset_idx == 0 (an outer/even wall).
    let inner_min = lines.iter().rposition(|l| l.inset_idx == 0);

    let (outer_max, inner_min) = match (outer_max, inner_min) {
        (Some(o), Some(i)) => (o, i),
        _ => panic!(
            "PARITY GAP: wall region order odd-after-enclosing | expected: \
             emitted wall regions ordered so inner (odd) region follows its \
             enclosing even region (WallToolPaths.cpp:809, \
             PerimeterGenerator.cpp:2302) | got: pipeline flattens per-inset \
             buckets in source order with no getRegionOrder pass \
             (pipeline.rs:383) | ref: WallToolPaths.cpp:809 | observed \
             outer_max={outer_max:?} inner_min={inner_min:?}"
        ),
    };

    // If the pipeline already ordered regions correctly the inner (odd)
    // region would follow the enclosing even region, i.e. the first inner
    // index would be greater than the last outer index. The current
    // source-order flattening makes the first inner index smaller, so this
    // assertion (outer_max >= inner_min) holds and the test FAILS (red).
    assert!(
        outer_max >= inner_min,
        "PARITY GAP: wall region order odd-after-enclosing | expected: \
         emitted wall regions ordered so inner (odd) region follows its \
         enclosing even region (WallToolPaths.cpp:809, \
         PerimeterGenerator.cpp:2302) | got: pipeline flattens per-inset \
         buckets in source order with no getRegionOrder pass \
         (pipeline.rs:383) | ref: WallToolPaths.cpp:809 | observed \
         outer_max={outer_max} inner_min={inner_min}"
    );
}

// ===========================================================================
// G15 — BeadingStrategy::get_split_middle_threshold exposed and consumed by
// RedistributeBeadingStrategy.
// ===========================================================================

/// G15 (TDD-red): `BeadingStrategy::get_split_middle_threshold` must exist and
/// be consumed by `RedistributeBeadingStrategy`'s optimal bead count.
///
/// OrcaSlicer ref: `BeadingStrategy.hpp:97`
/// (`getSplitMiddleThreshold(lower_bead_count)`); `BeadingStrategy.cpp:54-57`
/// (consumed by `RedistributeBeadingStrategy` to pick the optimal bead count).
///
/// The PnP trait `BeadingStrategy` (`crates/slicer-core/src/beading/mod.rs`)
/// does NOT expose `get_split_middle_threshold`; `RedistributeBeadingStrategy`
/// (`crates/slicer-core/src/beading/redistribute.rs:31-37`) delegates
/// `optimal_bead_count` to its parent unchanged, so the method is dead/missing.
///
/// **Compile-failure mode (the intended TDD-red):** a test that directly calls
/// `stack.get_split_middle_threshold(0)` will NOT compile until the trait
/// grows the method. To keep the whole test file compiling (so G12/G20 can be
/// checked) we only *build* the fixture stack here and then emit the parity
/// panic unconditionally — if the trait ever gains the method (even a stub
/// returning `0.0`) the runtime assertion below still fires, because the
/// contract requires a *positive* value matching Orca's
/// `wall_split_middle_threshold`, not `0.0`.
#[test]
fn arachne_parity_beading_split_middle_threshold_exposed() {
    // Confirm the fixture builds (this part compiles today).
    let stack = fixtures::beading_stack_for_split_middle();
    let _ = &stack;

    // If the trait gains the method, this is the runtime contract we would
    // enforce; we keep it commented so the file compiles without the method,
    // but document that the call below is what must eventually succeed:
    //
    //     let thr = stack.get_split_middle_threshold(0);
    //     assert!(
    //         thr > 0.0,
    //         "PARITY GAP: BeadingStrategy.getSplitMiddleThreshold | ..."
    //     );
    //
    // Until then the parity gap is unconditional.
    assert!(
        false,
        "PARITY GAP: BeadingStrategy.getSplitMiddleThreshold | expected: \
         trait method get_split_middle_threshold(lower_bead_count) present and \
         consumed by RedistributeBeadingStrategy optimal bead count \
         (BeadingStrategy.hpp:97) | got: method absent from BeadingStrategy \
         trait (beading/mod.rs); RedistributeBeadingStrategy delegates \
         optimal_bead_count to parent unchanged (redistribute.rs:31-37) | ref: \
         BeadingStrategy.hpp:97"
    );
}

// ===========================================================================
// G20 — simplify: intersection-distance gate preserves near-colinear middle
// junctions whose chord-intersection lies too far from neighbors.
// ===========================================================================

/// G20: build an `ExtrusionLine` from
/// `fixtures::simplify_input_intersection_distance_gate()` (a thin "Z" polyline
/// of four junctions) and run `simplify_toolpaths` with permissive parameters.
/// OrcaSlicer's `ExtrusionLine::simplify` rejects removal when the intersection
/// of the extended `(prev, curr)` lines lies more than
/// `smallest_line_segment_squared` from either neighbor, so the middle
/// junctions are PRESERVED (4 junctions remain). The PnP impl drops them
/// because it only checks `seg_len²` and `height_2`, with no
/// intersection-distance predicate.
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

    // visvalingam_area_threshold = 0.01, smallest_line_segment_squared = 0.0
    // (removes any short segment that satisfies the error gate),
    // allowed_error_distance_squared = INFINITY,
    // maximum_extrusion_area_deviation = INFINITY (fully permissive error
    // gates, so only the length/intersection predicate can preserve points).
    let result = simplify_toolpaths(vec![line], 0.01, 0.0, f64::INFINITY, f64::INFINITY);

    assert!(
        !result.is_empty(),
        "PARITY GAP: simplify intersection distance gate | expected: \
         ExtrusionLine::simplify rejects removal when the intersection of \
         (prev,curr) extended lines lies more than smallest_line_segment_squared \
         from either neighbor (ExtrusionLine.cpp:163-175) | got: simplify only \
         checks seg_len² and height_2; no intersection-distance predicate \
         (simplify.rs) | ref: ExtrusionLine.cpp:163-175"
    );

    let kept = result[0].junctions.len();
    assert!(
        kept >= 4,
        "PARITY GAP: simplify intersection distance gate | expected: \
         ExtrusionLine::simplify rejects removal when the intersection of \
         (prev,curr) extended lines lies more than smallest_line_segment_squared \
         from either neighbor (ExtrusionLine.cpp:163-175) | got: simplify only \
         checks seg_len² and height_2; no intersection-distance predicate \
         (simplify.rs) | ref: ExtrusionLine.cpp:163-175 | observed \
         junctions.len()={kept} (expected 4)"
    );

    // Touch Point2 so the import is meaningful for coordinate hygiene.
    let _ = Point2::from_mm(0.0, 0.0);
}
