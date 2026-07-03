//! Golden-table and invariant tests for `DistributedBeadingStrategy`.
//!
//! The golden table in `tests/fixtures/beading/distributed_10_thickness.json`
//! is ANALYTICALLY DERIVED by hand from OrcaSlicer's documented Gaussian
//! decay formula (see the fixture's `provenance` field) — no OrcaSlicer
//! gtest/unit-test fixture exists for `DistributedBeadingStrategy` to
//! transcribe from, and these values were computed independently before the
//! Rust implementation was written, not captured from running it.

use serde::Deserialize;
use slicer_core::beading::distributed::{assert_beading_invariant, DistributedBeadingStrategy};
use slicer_core::beading::{Beading, BeadingStrategy};

/// Slicer-unit tolerance for float comparisons (1e-4 units), per design.md's
/// guidance to avoid `assert_eq!` on raw f64.
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
        "{what}: length mismatch (expected {}, got {})",
        expected.len(),
        actual.len()
    );
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert_close(*a, *e, &format!("{what}[{i}]"));
    }
}

#[derive(Debug, Deserialize)]
struct FixtureParams {
    optimal_width: f64,
    default_transition_length: f64,
    transition_filter_dist: f64,
    distribution_count: usize,
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
    params: FixtureParams,
    cases: Vec<FixtureCase>,
}

fn load_fixture() -> Fixture {
    let raw = include_str!("../fixtures/beading/distributed_10_thickness.json");
    serde_json::from_str(raw).expect("fixture JSON must parse")
}

/// AC-2: `DistributedBeadingStrategy` computed against 10 recorded thickness
/// inputs; `bead_widths` and `toolpath_locations` must match the analytically
/// derived golden within 0.0001mm (1 slicer unit) tolerance.
#[test]
fn distributed_beading_strategy_orca_table() {
    let fixture = load_fixture();
    let strategy = DistributedBeadingStrategy::new(
        fixture.params.optimal_width,
        fixture.params.default_transition_length,
        fixture.params.transition_filter_dist,
        fixture.params.distribution_count,
    );

    assert_eq!(fixture.cases.len(), 10, "fixture must contain 10 cases");

    for case in &fixture.cases {
        let beading: Beading = strategy.compute(case.thickness, case.bead_count);

        assert_eq!(
            beading.bead_widths.len(),
            case.bead_count,
            "bead_widths.len() must equal requested bead_count for thickness {}",
            case.thickness
        );

        assert_close(
            beading.total_thickness,
            case.total_thickness,
            &format!("total_thickness (thickness={})", case.thickness),
        );
        assert_slice_close(
            &beading.bead_widths,
            &case.bead_widths,
            &format!("bead_widths (thickness={})", case.thickness),
        );
        assert_slice_close(
            &beading.toolpath_locations,
            &case.toolpath_locations,
            &format!("toolpath_locations (thickness={})", case.thickness),
        );
        assert_close(
            beading.left_over,
            case.left_over,
            &format!("left_over (thickness={})", case.thickness),
        );

        // design.md invariant: total_thickness == sum(bead_widths) + left_over
        let sum: f64 = beading.bead_widths.iter().sum();
        assert_close(
            beading.total_thickness,
            sum + beading.left_over,
            &format!(
                "total_thickness == sum(bead_widths) + left_over (thickness={})",
                case.thickness
            ),
        );
    }
}

/// AC-N1: the `Beading` invariant `toolpath_locations.len() ==
/// bead_widths.len()` is `debug_assert_eq!`-checked inside `compute` (via the
/// `assert_beading_invariant` helper `compute` calls on every return path).
///
/// A correct `compute` implementation can never actually produce mismatched
/// lengths, so this test exercises the exact check `compute` runs by calling
/// it directly against a manually malformed `Beading`, proving the assertion
/// genuinely fires (in a debug build) rather than being dead code.
#[test]
#[should_panic(expected = "Beading invariant violated")]
fn beading_invariant_locations_len_eq_widths_len() {
    let malformed = Beading {
        total_thickness: 4000.0,
        bead_widths: vec![4000.0],
        toolpath_locations: vec![1000.0, 3000.0],
        left_over: 0.0,
    };
    assert_beading_invariant(&malformed);
}
