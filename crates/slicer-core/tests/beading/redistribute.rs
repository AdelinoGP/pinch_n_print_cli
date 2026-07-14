// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/RedistributeBeadingStrategy.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! TDD golden-fixture test for `RedistributeBeadingStrategy` (packet 111,
//! Step 3). See `tests/fixtures/beading/redistribute_outer_consistent.json`
//! for the fixture's analytical derivation and provenance.

use serde::Deserialize;
use slicer_core::beading::distributed::DistributedBeadingStrategy;
use slicer_core::beading::redistribute::RedistributeBeadingStrategy;
use slicer_core::beading::BeadingStrategy;

const EPS: f64 = 1e-4;

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < EPS,
        "{what}: expected {expected}, got {actual} (diff {})",
        (actual - expected).abs()
    );
}

fn assert_slice_close(actual: &[f64], expected: &[f64], what: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{what}: length mismatch (actual {actual:?}, expected {expected:?})"
    );
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert_close(*a, *e, &format!("{what}[{i}]"));
    }
}

#[derive(Debug, Deserialize)]
struct FixtureParentParams {
    optimal_width: f64,
    default_transition_length: f64,
    transition_filter_dist: f64,
    distribution_count: usize,
    wall_transition_angle: f64,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    #[allow(dead_code)]
    note: String,
    thickness: f64,
    bead_count: usize,
    total_thickness: f64,
    bead_widths: Vec<f64>,
    toolpath_locations: Vec<f64>,
    left_over: f64,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    #[allow(dead_code)]
    provenance: String,
    parent_params: FixtureParentParams,
    redistribute_optimal_width: f64,
    minimum_variable_line_ratio: f64,
    cases: Vec<FixtureCase>,
}

fn load_fixture() -> Fixture {
    let raw = include_str!("../fixtures/beading/redistribute_outer_consistent.json");
    serde_json::from_str(raw).expect("fixture JSON must parse")
}

#[test]
fn redistribute_optimal_bead_count_consults_split_middle() {
    // AC-4: the three RedistributeBeadingStrategy methods that recurse into
    // `parent` on a thickness/bead_count reduced by the two outer beads, and
    // that consult `parent.get_split_middle_threshold()` in the `case 1`
    // branch of `get_transition_thickness` (NOT `case 0`) and the 2-bead
    // branch of `optimal_bead_count`.
    const W: f64 = 4000.0;
    const MIN_VAR_RATIO: f64 = 0.5;
    const SPLIT: f64 = 0.5;

    let parent = DistributedBeadingStrategy::new(W, 0.0, 0.0, 5, f64::MAX, SPLIT, SPLIT);
    let strategy = RedistributeBeadingStrategy::new(Box::new(parent), W, MIN_VAR_RATIO);

    // optimal_bead_count sub-cases.
    assert_eq!(strategy.optimal_bead_count(0.7 * W), 1);
    assert_eq!(strategy.optimal_bead_count(1.6 * W), 2);
    assert_eq!(strategy.optimal_bead_count(0.4 * W), 0);

    // get_transition_thickness sub-cases.
    assert_close(
        strategy.get_transition_thickness(0),
        MIN_VAR_RATIO * W,
        "get_transition_thickness(0) = 0.5*W (case 0)",
    );
    // case 1 consults parent.get_split_middle_threshold() = 0.5 -> (1+0.5)*W.
    assert_close(
        strategy.get_transition_thickness(1),
        (1.0 + SPLIT) * W,
        "get_transition_thickness(1) = (1+split)*W (case 1)",
    );
    let parent_ref = DistributedBeadingStrategy::new(W, 0.0, 0.0, 5, f64::MAX, SPLIT, SPLIT);
    assert_close(
        strategy.get_transition_thickness(3),
        parent_ref.get_transition_thickness(1) + 2.0 * W,
        "get_transition_thickness(3) = parent(1) + 2*W",
    );

    // optimal_thickness(4): inner = max(0, 4-2) = 2, outer = 4-2 = 2.
    assert_close(
        strategy.optimal_thickness(4),
        parent_ref.optimal_thickness(2) + 2.0 * W,
        "optimal_thickness(4) = parent(2) + 2*W",
    );
}

#[test]
fn redistribute_outer_consistent() {
    let fixture = load_fixture();
    let parent = DistributedBeadingStrategy::new(
        fixture.parent_params.optimal_width,
        fixture.parent_params.default_transition_length,
        fixture.parent_params.transition_filter_dist,
        fixture.parent_params.distribution_count,
        fixture.parent_params.wall_transition_angle,
        0.99,
        0.99,
    );
    let strategy = RedistributeBeadingStrategy::new(
        Box::new(parent),
        fixture.redistribute_optimal_width,
        fixture.minimum_variable_line_ratio,
    );

    assert_eq!(strategy.type_label(), "Redistribute");

    for case in &fixture.cases {
        let beading = strategy.compute(case.thickness, case.bead_count);

        assert_close(
            beading.total_thickness,
            case.total_thickness,
            "total_thickness",
        );
        assert_slice_close(&beading.bead_widths, &case.bead_widths, "bead_widths");
        assert_slice_close(
            &beading.toolpath_locations,
            &case.toolpath_locations,
            "toolpath_locations",
        );
        assert_close(beading.left_over, case.left_over, "left_over");

        // AC-3: outer and inner bead widths equal optimal_width exactly.
        assert_close(
            beading.bead_widths[0],
            fixture.redistribute_optimal_width,
            "bead_widths[0] (outer)",
        );
        assert_close(
            *beading.bead_widths.last().unwrap(),
            fixture.redistribute_optimal_width,
            "bead_widths[last] (inner)",
        );

        // total_thickness == sum(bead_widths) + left_over within tolerance.
        let sum: f64 = beading.bead_widths.iter().sum();
        assert_close(
            beading.total_thickness,
            sum + beading.left_over,
            "total_thickness == sum(bead_widths) + left_over",
        );
    }
}
