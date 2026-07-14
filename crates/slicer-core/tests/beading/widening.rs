// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/WideningBeadingStrategy.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! TDD golden-fixture test for `WideningBeadingStrategy` (packet 111, Step 4;
//! rewritten for the TRUE OrcaSlicer 3-way `compute` parity fix).
//!
//! Case (a) (`thickness < min_input_width`) and case (b)
//! (`min_input_width <= thickness < optimal_width`) are checked against the
//! analytically derived golden values in
//! `tests/fixtures/beading/widening_thin_wedge.json`. Case (c)
//! (`thickness >= optimal_width`, full delegation) is asserted inline against
//! a live parent chain — a real
//! `RedistributeBeadingStrategy(Box::new(DistributedBeadingStrategy))` stack
//! — rather than a third fixture entry, since "delegates unchanged" is
//! naturally verified by comparing the Widening wrapper's output to the same
//! parent chain's raw output for identical inputs; see this packet's Step 4
//! (widening parity fix) follow_up.

use serde::Deserialize;
use slicer_core::beading::distributed::DistributedBeadingStrategy;
use slicer_core::beading::redistribute::RedistributeBeadingStrategy;
use slicer_core::beading::widening::WideningBeadingStrategy;
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
struct FixtureCase {
    #[allow(dead_code)]
    note: String,
    thickness: f64,
    total_thickness: f64,
    bead_widths: Vec<f64>,
    toolpath_locations: Vec<f64>,
    left_over: f64,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    #[allow(dead_code)]
    provenance: String,
    optimal_width: f64,
    min_input_width: f64,
    min_output_width: f64,
    below_threshold: FixtureCase,
    middle_regime: FixtureCase,
}

fn load_fixture() -> Fixture {
    let raw = include_str!("../fixtures/beading/widening_thin_wedge.json");
    serde_json::from_str(raw).expect("fixture JSON must parse")
}

/// Builds the Steps 2-3 decorator chain (`Redistribute` wrapping
/// `Distributed`) used as the inner strategy, matching the fixture's
/// `optimal_width = 4000.0` convention.
fn build_parent_chain() -> RedistributeBeadingStrategy {
    let distributed =
        DistributedBeadingStrategy::new(4000.0, 0.0, 0.0, 2, 0.174_532_925_199_432_95, 0.99, 0.99);
    RedistributeBeadingStrategy::new(Box::new(distributed), 4000.0, 0.5)
}

fn build_widening(fixture: &Fixture) -> WideningBeadingStrategy {
    WideningBeadingStrategy::new(
        Box::new(build_parent_chain()),
        fixture.optimal_width,
        fixture.min_input_width,
        fixture.min_output_width,
    )
}

fn assert_case(
    strategy: &WideningBeadingStrategy,
    case: &FixtureCase,
    bead_count: usize,
    tag: &str,
) {
    let beading = strategy.compute(case.thickness, bead_count);

    assert_close(
        beading.total_thickness,
        case.total_thickness,
        &format!("{tag}: total_thickness"),
    );
    assert_slice_close(
        &beading.bead_widths,
        &case.bead_widths,
        &format!("{tag}: bead_widths"),
    );
    assert_slice_close(
        &beading.toolpath_locations,
        &case.toolpath_locations,
        &format!("{tag}: toolpath_locations"),
    );
    assert_close(
        beading.left_over,
        case.left_over,
        &format!("{tag}: left_over"),
    );

    // total_thickness == sum(bead_widths) + left_over invariant.
    let sum: f64 = beading.bead_widths.iter().sum();
    assert_close(
        beading.total_thickness,
        sum + beading.left_over,
        &format!("{tag}: total_thickness == sum(bead_widths) + left_over"),
    );
}

#[test]
fn widening_below_min_input_width() {
    let fixture = load_fixture();
    let strategy = build_widening(&fixture);
    assert_eq!(strategy.type_label(), "Widening");

    // --- Case (a): thickness < min_input_width -> empty beads, left_over ==
    // thickness (entire thickness unprinted, parent NOT called). ----------
    let below = &fixture.below_threshold;
    assert_case(&strategy, below, 0, "below_threshold");
    assert!(
        below.bead_widths.is_empty(),
        "AC-4 (corrected): below min_input_width must be EMPTY, not a forced bead"
    );
    assert_eq!(
        strategy.optimal_bead_count(below.thickness),
        0,
        "optimal_bead_count below min_input_width must be 0"
    );

    // --- Case (b): min_input_width <= thickness < optimal_width -> single
    // bead at thickness.max(min_output_width), left_over == 0.0. ----------
    let middle = &fixture.middle_regime;
    assert_case(&strategy, middle, 0, "middle_regime");
    assert_eq!(
        middle.bead_widths.len(),
        1,
        "middle regime must carry exactly one bead"
    );
    assert_close(
        middle.bead_widths[0],
        middle.thickness.max(fixture.min_output_width),
        "middle regime: bead_widths[0] == thickness.max(min_output_width)",
    );
    assert_eq!(
        strategy.optimal_bead_count(middle.thickness),
        1,
        "optimal_bead_count in the middle regime must be at least 1"
    );

    // get_transition_thickness(0) == min_input_width, per the ported
    // upstream formula.
    assert_close(
        strategy.get_transition_thickness(0),
        fixture.min_input_width,
        "get_transition_thickness(0) == min_input_width",
    );

    // --- Case (c): thickness >= optimal_width delegates fully, unmodified,
    // to the wrapped parent chain. ----------------------------------------
    let thickness_above = fixture.optimal_width + 500.0; // 4500.0
    let parent_direct = build_parent_chain();
    let bead_count = parent_direct.optimal_bead_count(thickness_above);
    let expected = parent_direct.compute(thickness_above, bead_count);

    let wrapped = build_widening(&fixture);
    let actual = wrapped.compute(thickness_above, bead_count);

    assert_eq!(
        actual, expected,
        "at/above optimal_width, Widening must delegate to parent unchanged"
    );
    assert_eq!(
        wrapped.optimal_bead_count(thickness_above),
        parent_direct.optimal_bead_count(thickness_above).max(1),
        "optimal_bead_count at/above min_input_width delegates to parent (clamped to >= 1)"
    );
    assert_eq!(
        wrapped.get_transition_thickness(1),
        parent_direct.get_transition_thickness(1),
        "get_transition_thickness for lower_bead_count > 0 delegates to parent"
    );
}
