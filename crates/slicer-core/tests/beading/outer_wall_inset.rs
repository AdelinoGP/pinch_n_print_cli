// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Tests for `OuterWallInsetBeadingStrategy` (packet 111 Step 5).

use slicer_core::beading::outer_wall_inset::OuterWallInsetBeadingStrategy;
use slicer_core::beading::{Beading, BeadingStrategy};

const TOL: f64 = 1e-4;

/// A test-local stub inner strategy that returns a fixed, hardcoded
/// `Beading` from every `compute` call, regardless of `thickness` /
/// `bead_count`. This keeps the decorator test decoupled from upstream
/// Distributed/Redistribute/Widening behavior changes.
struct FixedBeading(Beading);

impl BeadingStrategy for FixedBeading {
    fn compute(&self, _thickness: f64, _bead_count: usize) -> Beading {
        self.0.clone()
    }

    fn optimal_bead_count(&self, _thickness: f64) -> usize {
        self.0.bead_widths.len()
    }

    fn get_transition_thickness(&self, lower_bead_count: usize) -> f64 {
        lower_bead_count as f64
    }

    fn optimal_thickness(&self, bead_count: usize) -> f64 {
        bead_count as f64
    }

    fn type_label(&self) -> &'static str {
        "FixedBeadingStub"
    }

    fn get_split_middle_threshold(&self) -> f64 {
        0.99_f64
    }

    fn get_add_middle_threshold(&self) -> f64 {
        0.99_f64
    }
}

/// 5-bead fixture: outermost to innermost widths/locations, arbitrary but
/// distinct values so any accidental index mixups are detectable.
fn five_bead_fixture() -> Beading {
    Beading {
        total_thickness: 2000.0,
        bead_widths: vec![400.0, 400.0, 400.0, 400.0, 400.0],
        toolpath_locations: vec![200.0, 600.0, 1000.0, 1400.0, 1800.0],
        left_over: 0.0,
    }
}

#[test]
fn outer_wall_inset_offset_outer_only() {
    // total_thickness / 2 == 1000.0, well above toolpath_locations[0] + 100.0
    // (== 300.0), so the clamp does not bind here — the plain `+100.0`
    // arithmetic is directly verifiable.
    let parent = Box::new(FixedBeading(five_bead_fixture()));
    let strategy = OuterWallInsetBeadingStrategy::new(parent, 100.0);

    let raw = five_bead_fixture();
    let result = strategy.compute(raw.total_thickness, 5);

    assert_eq!(result.bead_widths.len(), 5);
    assert_eq!(result.toolpath_locations.len(), 5);

    // Widths are never modified by this decorator.
    for (got, want) in result.bead_widths.iter().zip(raw.bead_widths.iter()) {
        assert!(
            (got - want).abs() < TOL,
            "bead width changed: got {got}, want {want}"
        );
    }

    // Outermost location shifted inward (+offset), unclamped since
    // 200.0 + 100.0 == 300.0 < thickness / 2 == 1000.0.
    let expected_first = raw.toolpath_locations[0] + 100.0;
    assert!(
        (result.toolpath_locations[0] - expected_first).abs() < TOL,
        "toolpath_locations[0]: got {}, want {}",
        result.toolpath_locations[0],
        expected_first
    );

    // Upstream OuterWallInsetBeadingStrategy::compute is single-sided: only
    // toolpath_locations[0] is ever written. ALL remaining indices (1..5 of
    // 5 total, including what would be the "opposite end" at index 4) stay
    // completely unchanged from the parent's raw output.
    for i in 1..5 {
        assert!(
            (result.toolpath_locations[i] - raw.toolpath_locations[i]).abs() < TOL,
            "toolpath_locations[{i}] changed: got {}, want {}",
            result.toolpath_locations[i],
            raw.toolpath_locations[i]
        );
    }

    // left_over and total_thickness are untouched by this decorator.
    assert!((result.left_over - raw.left_over).abs() < TOL);
    assert!((result.total_thickness - raw.total_thickness).abs() < TOL);

    assert_eq!(strategy.type_label(), "OuterWallInset");
}

#[test]
fn outer_wall_inset_clamps_to_half_thickness() {
    // thickness / 2 == 500.0. toolpath_locations[0] + outer_wall_offset ==
    // 480.0 + 100.0 == 580.0, which exceeds the centerline clamp, so the
    // result must be exactly 500.0, not the unclamped 580.0.
    let stub_beading = Beading {
        total_thickness: 1000.0,
        bead_widths: vec![400.0, 400.0],
        toolpath_locations: vec![480.0, 900.0],
        left_over: 0.0,
    };
    let parent = Box::new(FixedBeading(stub_beading.clone()));
    let strategy = OuterWallInsetBeadingStrategy::new(parent, 100.0);

    let result = strategy.compute(stub_beading.total_thickness, 2);

    assert!(
        (result.toolpath_locations[0] - 500.0).abs() < TOL,
        "toolpath_locations[0] not clamped: got {}, want 500.0",
        result.toolpath_locations[0]
    );

    // The opposite end is still never touched, even when the clamp binds.
    assert!(
        (result.toolpath_locations[1] - stub_beading.toolpath_locations[1]).abs() < TOL,
        "toolpath_locations[1] changed: got {}, want {}",
        result.toolpath_locations[1],
        stub_beading.toolpath_locations[1]
    );
}

#[test]
fn outer_wall_inset_noop_when_offset_zero() {
    // toolpath_locations[0] == 200.0 is safely under thickness / 2 == 1000.0,
    // so with offset == 0.0 the clamp cannot bind and this is a genuine
    // no-op end to end.
    let parent = Box::new(FixedBeading(five_bead_fixture()));
    let strategy = OuterWallInsetBeadingStrategy::new(parent, 0.0);

    let raw = five_bead_fixture();
    let result = strategy.compute(raw.total_thickness, 5);

    for (got, want) in result
        .toolpath_locations
        .iter()
        .zip(raw.toolpath_locations.iter())
    {
        assert!(
            (got - want).abs() < TOL,
            "toolpath_location changed under zero offset: got {got}, want {want}"
        );
    }
    for (got, want) in result.bead_widths.iter().zip(raw.bead_widths.iter()) {
        assert!(
            (got - want).abs() < TOL,
            "bead width changed: got {got}, want {want}"
        );
    }
}
