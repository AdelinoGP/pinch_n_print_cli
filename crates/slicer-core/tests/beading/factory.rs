//! TDD suite for `BeadingStrategyFactory::create_stack` (packet 111 Step 7):
//! verifies the composed decorator stack's runtime type-composition order,
//! including the conditional `OuterWallInset` wrap (AC-8a), and its
//! end-to-end numeric output against a hand-derived, multi-stage "Orca
//! reference" fixture (AC-8b).
//!
//! See `crates/slicer-core/tests/fixtures/beading/factory_orca_reference.json`
//! for the fixture's `provenance` field and the `notes` array documenting
//! the per-strategy derivation this fixture was computed from BEFORE running
//! the composed Rust code (true TDD RED-phase methodology).

use serde::Deserialize;
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::beading::Beading;

const TOLERANCE: f64 = 1e-4;

#[derive(Debug, Deserialize)]
struct BeadingFixture {
    total_thickness: f64,
    bead_widths: Vec<f64>,
    toolpath_locations: Vec<f64>,
    left_over: f64,
}

#[derive(Debug, Deserialize)]
struct FactoryOrcaReferenceFixture {
    #[allow(dead_code)]
    provenance: String,
    params: BeadingFactoryParams,
    thickness: f64,
    bead_count: usize,
    #[allow(dead_code)]
    notes: Vec<String>,
    expected_raw: BeadingFixture,
    expected_stripped: BeadingFixture,
}

fn load_fixture() -> FactoryOrcaReferenceFixture {
    let raw = include_str!("../fixtures/beading/factory_orca_reference.json");
    serde_json::from_str(raw).expect("factory_orca_reference.json must parse")
}

fn assert_beading_close(actual: &Beading, expected: &BeadingFixture, label: &str) {
    assert!(
        (actual.total_thickness - expected.total_thickness).abs() < TOLERANCE,
        "[{label}] total_thickness mismatch: actual={}, expected={}",
        actual.total_thickness,
        expected.total_thickness
    );
    assert_eq!(
        actual.bead_widths.len(),
        expected.bead_widths.len(),
        "[{label}] bead_widths length mismatch: actual={:?}, expected={:?}",
        actual.bead_widths,
        expected.bead_widths
    );
    for (i, (a, e)) in actual
        .bead_widths
        .iter()
        .zip(expected.bead_widths.iter())
        .enumerate()
    {
        assert!(
            (a - e).abs() < TOLERANCE,
            "[{label}] bead_widths[{i}] mismatch: actual={a}, expected={e}"
        );
    }
    assert_eq!(
        actual.toolpath_locations.len(),
        expected.toolpath_locations.len(),
        "[{label}] toolpath_locations length mismatch: actual={:?}, expected={:?}",
        actual.toolpath_locations,
        expected.toolpath_locations
    );
    for (i, (a, e)) in actual
        .toolpath_locations
        .iter()
        .zip(expected.toolpath_locations.iter())
        .enumerate()
    {
        assert!(
            (a - e).abs() < TOLERANCE,
            "[{label}] toolpath_locations[{i}] mismatch: actual={a}, expected={e}"
        );
    }
    assert!(
        (actual.left_over - expected.left_over).abs() < TOLERANCE,
        "[{label}] left_over mismatch: actual={}, expected={}",
        actual.left_over,
        expected.left_over
    );
}

/// AC-8(a): the returned trait object's runtime type composition must be
/// verifiably `Limited<OuterWallInset<Widening<Redistribute<Distributed>>>>`
/// in exactly that order, when `outer_wall_offset` is nonzero AND
/// `print_thin_walls` is true (both optional layers present).
#[test]
fn factory_stack_composition_order() {
    let params = BeadingFactoryParams {
        outer_wall_offset: 300.0,
        print_thin_walls: true,
        ..Default::default()
    };
    let stack = BeadingStrategyFactory::create_stack(&params);

    assert_eq!(
        stack.type_chain(),
        "Limited(OuterWallInset(Widening(Redistribute(Distributed))))"
    );
}

/// AC-8(a) conditional-skip: with `outer_wall_offset == 0.0` and
/// `print_thin_walls == false` (both the defaults), `OuterWallInsetBeadingStrategy`
/// AND `WideningBeadingStrategy` must both be literally ABSENT from the
/// composition chain — not merely runtime no-ops — matching upstream's
/// `outer_wall_offset != 0` / `print_thin_walls` gates
/// (`BeadingStrategyFactory.cpp:50-97`).
#[test]
fn factory_stack_composition_order_default_skips_both_optional_layers() {
    let params = BeadingFactoryParams::default();
    let stack = BeadingStrategyFactory::create_stack(&params);

    assert_eq!(stack.type_chain(), "Limited(Redistribute(Distributed))");
}

/// `print_thin_walls: true` alone (with `outer_wall_offset` left at its
/// default `0.0`) must wrap `Widening` but NOT `OuterWallInset`.
#[test]
fn factory_stack_composition_order_widening_only_when_thin_walls_true() {
    let params = BeadingFactoryParams {
        print_thin_walls: true,
        ..Default::default()
    };
    let stack = BeadingStrategyFactory::create_stack(&params);

    assert_eq!(
        stack.type_chain(),
        "Limited(Widening(Redistribute(Distributed)))"
    );
}

/// `max_bead_count <= 2` must select `preferred_bead_width_outer` as the
/// effective base width fed to `Distributed`, NOT `optimal_width`. Uses
/// distinct `optimal_width` (4000) / `preferred_bead_width_outer` (6000)
/// values and drives `compute` with `bead_count == 2`: for a thickness of
/// exactly `2 * preferred_bead_width_outer = 12000`, Redistribute's
/// `bead_count == 2` branch computes `actual_outer_thickness =
/// AC-8(a): with `max_bead_count = 2` and `bead_count = 2` (at the cap
/// boundary, even), the under-cap branch of `LimitedBeadingStrategy::compute`
/// inserts a single 0-width center sentinel (OrcaSlicer
/// `LimitedBeadingStrategy.cpp:73-82`); the two real beads still come out at
/// `preferred_bead_width_outer` (6000.0) — never at `optimal_width` (4000.0).
#[test]
fn factory_max_bead_count_le_2_selects_preferred_bead_width_outer() {
    let params = BeadingFactoryParams {
        optimal_width: 4000.0,
        preferred_bead_width_outer: 6000.0,
        max_bead_count: 2,
        ..Default::default()
    };
    let stack = BeadingStrategyFactory::create_stack(&params);

    let beading = stack.compute(12000.0, 2);

    // Under-cap branch with bead_count == max_bead_count (even) inserts one
    // 0-width center sentinel; total = 2 real beads + 1 sentinel = 3 entries.
    assert_eq!(
        beading.bead_widths.len(),
        3,
        "under-cap branch at cap boundary (even) inserts one center sentinel"
    );
    // The two real beads (nonzero width) should reflect
    // `preferred_bead_width_outer` (6000.0), not `optimal_width` (4000.0).
    let nonzero_widths: Vec<f64> = beading
        .bead_widths
        .iter()
        .copied()
        .filter(|&w| w > 0.0)
        .collect();
    assert_eq!(
        nonzero_widths.len(),
        2,
        "exactly 2 nonzero-width beads (1 sentinel of width 0.0)"
    );
    for (i, &width) in nonzero_widths.iter().enumerate() {
        assert!(
            (width - 6000.0).abs() < TOLERANCE,
            "bead_widths[{i}] should reflect preferred_bead_width_outer \
             (6000.0), not optimal_width (4000.0); actual={width}"
        );
    }
}

/// AC-8(b): `compute` on the fully composed stack must match the hand-derived
/// multi-stage fixture within 0.0001mm (1e-4 slicer-unit tolerance, since 1
/// slicer unit = 100nm here — see the packet digest / coordinate-system doc).
#[test]
fn factory_matches_orca_reference() {
    let fixture = load_fixture();
    let params = fixture.params;
    let stack = BeadingStrategyFactory::create_stack(&params);

    // Raw `compute` output: exercises the full stack including Limited's
    // sentinel insertion (the fixture's `expected_raw` still carries the
    // zero-width sentinel beads Limited's over-cap branch inserts).
    let raw = stack.compute(fixture.thickness, fixture.bead_count);
    assert_beading_close(&raw, &fixture.expected_raw, "raw");

    // "Production" stripped output: `Box<dyn BeadingStrategy>` only exposes
    // trait methods, and `compute_and_strip` is an inherent method on the
    // concrete `LimitedBeadingStrategy` (not part of the object-safe
    // `BeadingStrategy` trait), so it is not reachable through the trait
    // object `create_stack` returns. This replicates
    // `LimitedBeadingStrategy::compute_and_strip`'s own zero-width filter
    // directly on `raw` to verify the "production" entry point's semantics
    // propagate correctly through the full stack.
    let mut bead_widths = Vec::with_capacity(raw.bead_widths.len());
    let mut toolpath_locations = Vec::with_capacity(raw.toolpath_locations.len());
    for (&width, &location) in raw.bead_widths.iter().zip(raw.toolpath_locations.iter()) {
        if width > 0.0 {
            bead_widths.push(width);
            toolpath_locations.push(location);
        }
    }
    let stripped = Beading {
        total_thickness: raw.total_thickness,
        bead_widths,
        toolpath_locations,
        left_over: raw.left_over,
    };
    assert_beading_close(&stripped, &fixture.expected_stripped, "stripped");
}

/// AC-5: `BeadingFactoryParams::default()` seeds both middle thresholds at the
/// `0.99` sentinel. With the shipped defaults (`min_output_width = 4000`,
/// `optimal_width = 4000`, `preferred_bead_width_outer = 4000`), both clamp
/// formulas evaluate to `1.0` and saturate at `0.99` — but this test pins the
/// *seeded* `Default` value, independent of `create_stack`'s recomputation.
#[test]
fn beading_factory_passes_split_middle_thresholds() {
    let params = BeadingFactoryParams::default();

    assert!(
        (params.wall_split_middle_threshold - 0.99).abs() < TOLERANCE,
        "default wall_split_middle_threshold must be 0.99; actual={}",
        params.wall_split_middle_threshold
    );
    assert!(
        (params.wall_add_middle_threshold - 0.99).abs() < TOLERANCE,
        "default wall_add_middle_threshold must be 0.99; actual={}",
        params.wall_add_middle_threshold
    );
}

/// AC-1 stack-forwarding lock: with every optional decorator present
/// (`print_thin_walls = true`, `outer_wall_offset != 0.0`), the two thresholds
/// computed by `create_stack` must forward unchanged through the full
/// `Limited` top of the stack (`Limited → OuterWallInset → Widening →
/// Redistribute → Distributed`).
#[test]
fn beading_factory_threshold_propagates_through_full_stack() {
    let params = BeadingFactoryParams {
        outer_wall_offset: 300.0,
        print_thin_walls: true,
        ..Default::default()
    };
    let stack = BeadingStrategyFactory::create_stack(&params);

    assert!(
        (stack.get_split_middle_threshold() - 0.99).abs() < TOLERANCE,
        "split threshold must forward through the full stack as 0.99; actual={}",
        stack.get_split_middle_threshold()
    );
    assert!(
        (stack.get_add_middle_threshold() - 0.99).abs() < TOLERANCE,
        "add threshold must forward through the full stack as 0.99; actual={}",
        stack.get_add_middle_threshold()
    );
}

/// AC-N1: the canonical `[0.01, 0.99]` clamp bounds must hold exactly. With
/// `min_output_width = 100.0`, the split formula `2*100/4000 - 1 = -0.95`
/// clamps to the LOWER bound `0.01`, and the add formula `100/4000 = 0.025`
/// stays unclamped inside the band. With `min_output_width = 100_000.0`, the
/// split formula `2*100000/4000 - 1 = 49.0` clamps to the UPPER bound `0.99`.
#[test]
fn beading_factory_threshold_clamp_bounds_are_canonical() {
    let params_lo = BeadingFactoryParams {
        min_output_width: 100.0,
        ..Default::default()
    };
    let stack_lo = BeadingStrategyFactory::create_stack(&params_lo);
    assert!(
        (stack_lo.get_split_middle_threshold() - 0.01).abs() < TOLERANCE,
        "split threshold clamps to LOWER bound 0.01 for min_output_width=100; actual={}",
        stack_lo.get_split_middle_threshold()
    );
    assert!(
        (stack_lo.get_add_middle_threshold() - 0.025).abs() < TOLERANCE,
        "add threshold unclamped inside band = 0.025 for min_output_width=100; actual={}",
        stack_lo.get_add_middle_threshold()
    );

    let params_hi = BeadingFactoryParams {
        min_output_width: 100_000.0,
        ..Default::default()
    };
    let stack_hi = BeadingStrategyFactory::create_stack(&params_hi);
    assert!(
        (stack_hi.get_split_middle_threshold() - 0.99).abs() < TOLERANCE,
        "split threshold clamps to UPPER bound 0.99 for min_output_width=100000; actual={}",
        stack_hi.get_split_middle_threshold()
    );
}
