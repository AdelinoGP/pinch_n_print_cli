// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/LimitedBeadingStrategy.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Golden-fixture tests for `LimitedBeadingStrategy`.
//!
//! Ground truth: no OrcaSlicer fixture data exists for any BeadingStrategy
//! class in OrcaSlicerDocumented (confirmed during packet 111 Step 6). The
//! `limited_cap_boundary.json` fixture is derived analytically — see its
//! `provenance` field and `crates/slicer-core/src/beading/limited.rs`'s doc
//! comment for the sentinel-placement mapping from OrcaSlicer's mechanics.

use serde::Deserialize;
use slicer_core::beading::distributed::DistributedBeadingStrategy;
use slicer_core::beading::limited::LimitedBeadingStrategy;
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
        "{what}: length mismatch, actual {actual:?}, expected {expected:?}"
    );
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert_close(*a, *e, &format!("{what}[{i}]"));
    }
}

#[derive(Deserialize)]
struct Fixture {
    #[allow(dead_code)]
    provenance: String,
    parent_params: ParentParams,
    max_bead_count: usize,
    optimal_bead_count_probe: OptimalBeadCountProbe,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct ParentParams {
    optimal_width: f64,
    default_transition_length: f64,
    transition_filter_dist: f64,
    distribution_count: usize,
    wall_transition_angle: f64,
}

#[derive(Deserialize)]
struct OptimalBeadCountProbe {
    thickness: f64,
    expected_bead_count: usize,
}

#[derive(Deserialize)]
struct Case {
    #[allow(dead_code)]
    note: String,
    thickness: f64,
    bead_count: usize,
    sentinel_count: usize,
    total_thickness: f64,
    bead_widths: Vec<f64>,
    toolpath_locations: Vec<f64>,
    left_over: f64,
    stripped_bead_widths: Vec<f64>,
    stripped_toolpath_locations: Vec<f64>,
}

fn load_fixture() -> Fixture {
    let raw = include_str!("../fixtures/beading/limited_cap_boundary.json");
    serde_json::from_str(raw).expect("fixture JSON must parse")
}

fn build_strategy(fixture: &Fixture) -> LimitedBeadingStrategy {
    let parent = DistributedBeadingStrategy::new(
        fixture.parent_params.optimal_width,
        fixture.parent_params.default_transition_length,
        fixture.parent_params.transition_filter_dist,
        fixture.parent_params.distribution_count,
        fixture.parent_params.wall_transition_angle,
    );
    LimitedBeadingStrategy::new(Box::new(parent), fixture.max_bead_count)
}

/// AC-6: over-cap `compute` inserts `2 * sentinel_count` zero-width sentinels
/// at the cap boundary, `bead_widths.len() == max_bead_count + 2 *
/// sentinel_count`, and `optimal_bead_count` is capped end-to-end.
#[test]
fn limited_inserts_sentinels_at_cap() {
    let fixture = load_fixture();
    let strategy = build_strategy(&fixture);

    assert_eq!(strategy.type_label(), "Limited");

    let over_cap = fixture
        .cases
        .iter()
        .find(|c| c.bead_count > fixture.max_bead_count)
        .expect("fixture must contain an over-cap case");

    let beading = strategy.compute(over_cap.thickness, over_cap.bead_count);

    assert_eq!(
        beading.bead_widths.len(),
        fixture.max_bead_count + 2 * over_cap.sentinel_count,
        "bead_widths.len() == max_bead_count + 2 * sentinel_count"
    );
    assert_eq!(
        beading.toolpath_locations.len(),
        beading.bead_widths.len(),
        "toolpath_locations.len() == bead_widths.len()"
    );

    assert_close(
        beading.total_thickness,
        over_cap.total_thickness,
        "total_thickness",
    );
    assert_slice_close(&beading.bead_widths, &over_cap.bead_widths, "bead_widths");
    assert_slice_close(
        &beading.toolpath_locations,
        &over_cap.toolpath_locations,
        "toolpath_locations",
    );
    assert_close(beading.left_over, over_cap.left_over, "left_over");

    // Sentinels are exactly 0.0-width and sit at the cap boundary indices
    // (fixture encodes exactly which indices are sentinels).
    for (i, &w) in beading.bead_widths.iter().enumerate() {
        let expected_zero = over_cap.bead_widths[i] == 0.0;
        assert_eq!(
            w == 0.0,
            expected_zero,
            "bead_widths[{i}] zero-width mismatch (got {w}, fixture expected zero={expected_zero})"
        );
    }

    // optimal_bead_count is capped end-to-end.
    assert_eq!(
        strategy.optimal_bead_count(fixture.optimal_bead_count_probe.thickness),
        fixture.optimal_bead_count_probe.expected_bead_count,
        "optimal_bead_count must be capped at max_bead_count"
    );
}

/// AC-7: `compute_and_strip` removes all zero-width sentinel entries in
/// lockstep with their `toolpath_locations`, preserving the
/// `toolpath_locations.len() == bead_widths.len()` invariant and leaving no
/// zero-width beads behind.
#[test]
fn limited_compute_and_strip_no_zero_widths() {
    let fixture = load_fixture();
    let strategy = build_strategy(&fixture);

    for case in &fixture.cases {
        let stripped = strategy.compute_and_strip(case.thickness, case.bead_count);

        assert_eq!(
            stripped.toolpath_locations.len(),
            stripped.bead_widths.len(),
            "toolpath_locations.len() == bead_widths.len() ({})",
            case.note
        );
        assert!(
            stripped.bead_widths.iter().all(|&w| w > 0.0),
            "compute_and_strip must not return zero-width beads, got {:?} ({})",
            stripped.bead_widths,
            case.note
        );

        assert_slice_close(
            &stripped.bead_widths,
            &case.stripped_bead_widths,
            "stripped bead_widths",
        );
        assert_slice_close(
            &stripped.toolpath_locations,
            &case.stripped_toolpath_locations,
            "stripped toolpath_locations",
        );

        // total_thickness/left_over pass through unchanged from raw compute.
        assert_close(
            stripped.total_thickness,
            case.total_thickness,
            "total_thickness",
        );
        assert_close(stripped.left_over, case.left_over, "left_over");

        // Sum invariant still holds after stripping (removed sentinels
        // contributed exactly 0.0 width).
        let sum: f64 = stripped.bead_widths.iter().sum();
        assert_close(
            stripped.total_thickness,
            sum + stripped.left_over,
            "total_thickness == sum(bead_widths) + left_over after strip",
        );
    }
}

/// AC-N2: the raw `compute` (not `compute_and_strip`) still retains
/// zero-width sentinels in the over-cap case, and never introduces sentinels
/// in the under-cap case — guards against accidentally folding the strip
/// pass into `compute` itself.
#[test]
fn limited_raw_compute_retains_sentinels() {
    let fixture = load_fixture();
    let strategy = build_strategy(&fixture);

    let over_cap = fixture
        .cases
        .iter()
        .find(|c| c.sentinel_count > 0)
        .expect("fixture must contain a case with sentinel_count > 0");

    let raw = strategy.compute(over_cap.thickness, over_cap.bead_count);
    let zero_width_count = raw.bead_widths.iter().filter(|&&w| w == 0.0).count();

    assert_eq!(
        zero_width_count,
        2 * over_cap.sentinel_count,
        "raw compute must retain 2 * sentinel_count zero-width sentinels"
    );
    assert_eq!(
        raw.toolpath_locations.len(),
        raw.bead_widths.len(),
        "toolpath_locations.len() == bead_widths.len()"
    );

    // Under-cap case must NOT gain sentinels — compute delegates unchanged.
    let under_cap = fixture
        .cases
        .iter()
        .find(|c| c.sentinel_count == 0)
        .expect("fixture must contain an under-cap (no-sentinel) case");
    let plain = strategy.compute(under_cap.thickness, under_cap.bead_count);
    assert!(
        plain.bead_widths.iter().all(|&w| w > 0.0),
        "under-cap compute must not introduce sentinels"
    );
}
