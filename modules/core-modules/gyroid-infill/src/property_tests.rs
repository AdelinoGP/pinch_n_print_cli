//! Opt-in property tests for this module's private gyroid kernels.
//!
//! Charters (`docs/spec_packets/proptest-openrscad_13_pbt-infill`):
//!
//! * `M-GYROID-PHASE` — finite `gyroid_f` branch evaluation for both reachable
//!   orientations, plus exact floor-semantics `align_to_grid` witnesses.
//! * `M-GYROID-CLIP` — raw unclipped waves from the real `fill_expolygon`,
//!   passed through the linker's own prepared batch clip primitive and checked
//!   against an independent rational expanded-rectangle/contracted-hole model.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! `gyroid_f` and `GyroidInfill::fill_expolygon` are private, so the
//! properties live here as a crate-internal unit-test module. Relocating them
//! under `tests/**` would make them integration tests, which cannot see the
//! private kernels; production visibility is deliberately not widened.
//!
//! # Step status
//!
//! Step 4 implements both properties, their independent checkers, the
//! deterministic witnesses, the mutation controls and the shrink probes. No
//! registry record is added before Step 8 measures the profiles.
//!
//! # Derived phase/axis mapping and the implicit-surface residual
//!
//! No implicit-surface residual is asserted, and the packet's open question is
//! answered explicitly: a meaningful residual subdomain is **not proved**.
//! `gyroid_f` returns the *wave offset* in the generation frame, and
//! `make_wave` then swaps axes for the vertical orientation, offsets by `y0`,
//! clamps to `[0, height]` and tiles periods across the aligned/expanded
//! bounding box. Recovering `f(x)` from an emitted world point therefore needs
//! the rotated frame, `bb_min_x`/`bb_min_y` (grid-aligned then expanded by
//! `10 × spacing_mm`), the orientation-dependent axis swap, the clamp and the
//! `f32` stamp. Several of those steps are non-injective (the clamp folds whole
//! curve ranges onto the same coordinate, the f32 stamp quantizes), so the
//! residual `sin(x)cos(y) + sin(y)cos(z) + sin(z)cos(x)` cannot be evaluated
//! on the emitted geometry without inventing a sample-to-source mapping this
//! packet has not proved. The phase property checks the branch algebra directly
//! instead, and the clip property stops at the emitter/linker seam.
//!
//! # Derived f64 envelopes
//!
//! * Branch range. In the vertical branch the SUT returns
//!   `asin(v1) + asin(v2) + PI` and in the horizontal branch
//!   `asin(v1) + asin(v2) + FRAC_PI_2`, where both `asin` results are clamped
//!   into `[-PI/2, PI/2]`. The exact range is therefore `[0, 2π]` and
//!   `[-π/2, 3π/2]`; a per-term 2-ulp rounding budget plus an at-most-2-ulp
//!   addition budget is below 2e-15, so the asserted envelope adds
//!   `PHASE_RANGE_SLACK = 1e-12` (a >500× margin, not an observed-output
//!   epsilon).
//! * Periodicity. `gyroid_f` is `2π`-periodic in `x` because every `sin`/`cos`
//!   argument contains `x` linearly. The comparison `f(x)` vs `f(x + 2π)` is
//!   only made when the evaluated branch has `r >= 0.25` and both `asin`
//!   arguments satisfy `|u| <= 0.75`: the `x + 2π` argument rounding is at most
//!   `1e-14` in absolute value at the 100 rad phase cap, `|Δu|` stays below
//!   `1.2e-13`, the `asin` derivative is at most `1/sqrt(1 - 0.75²) ≈ 1.51`,
//!   and the two branches plus the final add stay below `4e-13`. The asserted
//!   `PERIODIC_TOLERANCE = 4e-12` is a 10× margin over that budget and is only
//!   applied away from the `r < 1e-15` early return and away from the `±1`
//!   clamp plateaus.
//! * Grid floors are exact, not tolerant: generated grids and values are
//!   integers (exact dyadics). For `grid <= 16384` and `|val/grid| <= 65`, the
//!   exact quotient is either an integer (correctly-rounded division returns
//!   it exactly) or at least `1/grid >= 6.1e-5` from the next integer while
//!   the division rounding is below `1.5e-14`; `floor` is therefore exact and
//!   the product is at most `1.1e6`, exactly representable. The independent
//!   model is integer `div_euclid`/multiply, and comparison is bit-exact.
//!
//! # Clip-model allowances
//!
//! `PreparedPolylineClip::new` pre-inflates the clip universe by exactly one
//! integer unit (contours outward, holes inward — recorded in
//! `crates/slicer-core/tests/fixtures/clip_polylines_prepared_reference.json`)
//! and its documented contract puts every reported boundary coordinate within
//! `±2` units of the exact clip boundary: that 1-unit pre-inflation plus one
//! unit of integer intersection rounding. The independent model below builds
//! its boundaries **at the prepared (pre-inflated) edge**: `clip_x_boundaries`,
//! `clip_y_boundaries` and `model_kept_at` add `CLIP_INFLATION_UNITS`, and for
//! these axis-aligned integer fixtures `inflate_paths_64(delta = 1.0)` lands
//! exactly one unit out on every edge. The pre-inflation is therefore present
//! on both sides and cancels, and the only remaining deviation of a reported
//! coordinate from the **modelled** edge is the clipper's integer intersection
//! rounding: **±1 unit**. `CLIP_BAND_UNITS = 1.0` is that rounding allowance;
//! `check_returned_geometry` applies `CLIP_INFLATION_UNITS` once, only to place
//! the modelled edge on the fixture rect. There is intentionally **no**
//! `f32`-quantization term: the raw wave points are rounded to an integer
//! `Point2` subject once (`subjects_from_paths`), and the model consumes that
//! same integer subject, so any `f32` rounding is identical on both sides and
//! cancels.
//!
//! Mapping a reported point back onto the subject uses a separate attribution
//! tolerance, `SUBJECT_PROJECTION_UNITS = 2.0`: the cut is the integer-truncated
//! intersection of the subject with the modelled edge, so each coordinate moves
//! by <1 unit (Euclidean displacement < √2 ≈ 1.42) and the batch's `top_x`
//! scanline rounding adds at most 0.5, giving < 2 units. It is not a boundary
//! allowance and is not used for the interval comparison.
//!
//! That band is **perpendicular to the clip edge**, and converting it to
//! subject parameter space needs the crossing angle. The reported coordinate
//! may be anywhere within `band` of the modelled edge. Two extreme
//! displacements bound the parameter shift:
//!
//! * perpendicular to the edge, the subject's normal coordinate moves at
//!   `|Δnormal|` per unit parameter, so the shift is `band / |Δnormal|`;
//! * along the edge, a point `L` from the exact crossing projects
//!   `L·|Δparallel|/length²` further along the subject while sitting
//!   `L·|Δnormal|/length` from it, so `L ≤ band·length/|Δnormal|` gives a
//!   shift of `(band/|Δnormal|)·(|Δparallel|/length) = (band/|Δnormal|)·cos θ`.
//!
//! The perpendicular extreme dominates (`cos θ <= 1`), so the endpoint term is
//! `band / |Δnormal|` at a boundary crossing (`endpoint_parameter_term`;
//! `|Δnormal|` is `|Δx|` on a vertical edge and `|Δy|` on a horizontal one) and
//! `band / length` at a subject endpoint. For a perpendicular crossing it
//! reduces to the uniform `band / length`; for a tangential crossing it grows
//! with `1/sin θ` until it reaches **1.0 — one whole subject segment**. That cap
//! is not an arbitrary clamp: when `band / |Δnormal| >= 1` the band already
//! covers the entire segment's extent along the edge normal, so the interval
//! end carries no discriminating information. The term reports the saturation
//! (`EndpointTerm::saturated`), saturated endpoints are **counted**
//! (`saturated_endpoints`) and their interval is excluded from the coverage
//! direction instead of being silently accepted; the containment direction may
//! then reach the whole segment, which is what the band alone permits.
//! Saturation is never asserted to be zero. At this 1-unit band a **boundary**
//! endpoint cannot saturate: an integer interior crossing of an integer edge
//! has `|Δnormal| >= 2`, so the term is at most `1/2`. Only a subject endpoint
//! whose segment is at most one band unit long saturates (`1/length >= 1`),
//! which is exactly what the `gyroid_controls` saturation case exercises.
//! Measured example from the registry profiles: a wave edge with `|Δx| = 36`,
//! `|Δy| = 1877` (length 1877.3) crosses a vertical outer edge at 1.1°; the old
//! uniform term was `3 / 1877.3 = 0.0016` at the old 3-unit band, the endpoint
//! term is `1 / |Δx| = 0.0278` at the modelled 1-unit band, while the engine's
//! integer cut slid 11.5 units (0.22 units perpendicular) along the edge =
//! 0.0061 parameter units — inside the derived term, outside the old uniform
//! one. The independent `gyroid_controls` bound case shows the term is finite:
//! a near-tangential crossing with `|Δx| = 20` has term 0.05, accepts a 1-unit
//! shortfall (0.05) and rejects a 5-unit one (0.25).
//!
//! # Attribution
//!
//! Returned points are attributed to the **nearest** subject segment within
//! `SUBJECT_PROJECTION_UNITS`, not the first within that distance. A cut point sitting
//! a couple of units past a subject vertex is within the band of the incoming
//! segment (whose clamped endpoint is the vertex) while lying on the outgoing
//! one; the first-hit rule then reports a fabricated span start at the vertex,
//! which is a parameter error up to that endpoint's band term (or
//! `band / length` for a non-boundary cut) unrelated to the clip.
//! `PROJECTION_WINDOW` bounds the forward search to the adjacent segments.
//!
//! The projection window is **attribution slack only**: it decides which
//! subject segment a returned point belongs to, and it is deliberately larger
//! than the boundary band (the batch engine's `top_x` scanline rounding can move
//! a cut along its scanline). A point farther from the modelled edge than
//! `CLIP_BAND_UNITS` is still rejected by the per-interval containment and
//! coverage checks, so the slack can only produce a spurious failure, never a
//! false pass. The two tolerances therefore answer different questions and are
//! not interchangeable: `CLIP_BAND_UNITS` bounds the boundary deviation,
//! `SUBJECT_PROJECTION_UNITS` bounds point-to-subject attribution.
//!
//! # Literal review (AC-6)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. The watchlist was re-derived
//! from `docs/21_data_defaults_and_fixtures.md` §3: a watched literal is a
//! `pub` struct with at least five named fields defined under `crates/*/src/**`.
//! The only watched types reachable here are
//! `Point3WithWidth` (8 fields) and `ExtrusionPath3D` (5 fields)
//! (`crates/slicer-ir/src/slice_ir.rs`). This file constructs **neither** — the
//! SUT builds every `ExtrusionPath3D`/`Point3WithWidth` and the tests only read
//! them. `Point2` (2 fields), `Polygon` (1), `ExPolygon` (2) and
//! `GyroidInfill` (4 fields, and under `modules/`, not `crates/*/src`) are below
//! or outside the watch rule, and `ClipLimits`, `ClipCase`, `ClipPart`,
//! `PhaseLimits`, `PhaseCase`, `GridCase` and `Frac` are private module-local
//! types, not watched crate-public structs. There is therefore no watched-type
//! literal site and no `// exhaustive:` waiver in this file.
//!
//! # Test-quality review (docs/22_test_quality.md)
//!
//! Neither property is a determinism pin: `M-GYROID-PHASE` compares against
//! derived branch algebra and an integer floor model, and `M-GYROID-CLIP`
//! compares against a rational interval model that never calls the SUT's
//! clipper, offsetter or union. The controls drive the *same* checkers with
//! explicitly wrong observations, so a checker that stopped rejecting them
//! would fail. Positive counters are asserted after `check`, not inside a
//! possibly-empty loop, and no fixture is skipped silently.

#![allow(missing_docs)]

use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_core::polygon_ops::PreparedPolylineClip;
use slicer_ir::{units_to_mm, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Polygon};
use slicer_sdk::test_support::property_testing::domains::DomainError;
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::{align_to_grid, gyroid_f, GyroidInfill};

const PHASE_ID: &str = "M-GYROID-PHASE";
const PHASE_TEST: &str = "property_tests::proptest_gyroid_phase_and_grid";
const CLIP_ID: &str = "M-GYROID-CLIP";
const CLIP_TEST: &str = "property_tests::proptest_gyroid_raw_wave_clipping";
const PHASE_BOUND_KEYS: [&str; 2] = ["max_phase_milliradians", "max_grid_units"];
const CLIP_BOUND_KEYS: [&str; 9] = [
    "max_abs_units",
    "max_vertices",
    "max_holes",
    "max_components",
    "max_shrink_candidates",
    "max_width_units",
    "min_density_ppm",
    "max_phase_milliradians",
    "max_wave_points",
];

/// SUT density-adjust constant (`DENSITY_ADJUST` in `src/lib.rs`).
const DENSITY_ADJUST: f64 = 2.44;
/// Upper cap for the phase bound: keeps `sin` argument rounding below 1e-14.
const MAX_PHASE_MILLIRADIANS_CAP: u64 = 100_000;
/// The four orientation bands need at least three full quarter turns plus the
/// band margin; 3142 mrad = π rad.
const MIN_PHASE_MILLIRADIANS: u64 = 3_142;
/// Largest accepted grid magnitude (exact-dyadic floor proof bound).
const MAX_GRID_UNITS_CAP: u64 = 16_384;
/// Largest accepted `|val| / grid` ratio (exact-dyadic floor proof bound).
const GRID_RATIO: i64 = 64;
/// Whole-period translations applied to `align_to_grid` inputs.
const GRID_SHIFT: i64 = 4;
/// Derived branch-range envelope; see the module header derivation.
const PHASE_RANGE_SLACK: f64 = 1e-12;
/// Derived periodic-comparison envelope; see the module header derivation.
const PERIODIC_TOLERANCE: f64 = 4e-12;
const PERIODIC_MIN_RADIUS: f64 = 0.25;
const PERIODIC_MAX_ARGUMENT: f64 = 0.75;
/// Position of `z` inside a quarter turn, kept away from the orientation tie
/// (`|sin z| == |cos z|`) and from exactly-on-boundary sampling.
const QUARTER_BAND_MIN: f64 = 0.1;
const QUARTER_BAND_MAX: f64 = 0.9;

/// Integer intersection rounding of a reported clip coordinate relative to the
/// **modelled** (pre-inflated) clip edge. The model builds its boundaries at the
/// prepared-universe edge (`clip_x_boundaries`/`model_kept_at` add
/// `CLIP_INFLATION_UNITS`), so the documented 1-unit pre-inflation is already
/// present on both sides and cancels; only the ±1 unit of integer rounding
/// remains. See the module header.
const CLIP_BAND_UNITS: f64 = 1.0;

/// Attribution tolerance when mapping a reported point back onto the subject.
///
/// The cut is the integer-truncated intersection of the subject with the
/// modelled edge: each coordinate moves by <1 unit (Euclidean displacement
/// < √2 ≈ 1.42) and the batch's `top_x` scanline rounding adds at most 0.5, so
/// the point is < 2 units from the subject. This is not a boundary allowance;
/// it only decides which subject segment owns the point.
const SUBJECT_PROJECTION_UNITS: f64 = 2.0;
/// Pre-inflation applied by `PreparedPolylineClip::new`, in integer units.
const CLIP_INFLATION_UNITS: i64 = 1;

/// Clip geometry: outer half-extent multiples of one wave spacing.
const MIN_SIZE_MULTIPLE: i64 = 4;
const MAX_SIZE_MULTIPLE: i64 = 7;
const MIN_HOLE_MULTIPLE: i64 = 2;
const MAX_HOLE_MULTIPLE: i64 = 3;
/// Largest normalized half-extent the clip strategy can produce, in spacings:
/// two components of `MAX_SIZE_MULTIPLE` plus one spacing of separation.
const MAX_COMPONENT_SPAN_MULTIPLE: f64 = 15.0;
/// Derived wave-generation span, in normalized gyroid units: two rotated
/// half-extents (`|cos| + |sin| <= 1.37`), two 10-spacing expansions and one
/// `2π` grid cell.
const MAX_NORMALIZED_WAVE_SPAN: f64 = 2.0 * MAX_SIZE_MULTIPLE as f64 * 1.37 + 20.0 + TAU;
/// Derived adaptive-refinement ceiling per period. The generated phases keep
/// `|sin z|` and `|cos z|` at most `tan(π/4 - 0.1·π/4) ≈ 0.854` apart, so both
/// `asin` arguments stay within 0.86 and `1 - u²` stays above 0.26; a
/// conservative second-derivative bound of 139 then stops refinement at a step
/// above `0.24 × tolerance`, and the smallest admitted tolerance is
/// `0.2 / 2.46 mm ≈ 0.081`, i.e. at most 324 intervals per `2π` period.
const MAX_POINTS_PER_PERIOD: usize = 384;
/// The derived worst-case raw-point envelope for this domain.
fn derived_wave_point_envelope() -> usize {
    let periods_per_wave = MAX_NORMALIZED_WAVE_SPAN / TAU + 1.0;
    let waves = (MAX_NORMALIZED_WAVE_SPAN / PI + 2.0).ceil();
    (waves * periods_per_wave * MAX_POINTS_PER_PERIOD as f64).ceil() as usize
}
/// Largest accepted wave-point cap (allocation guard, far above
/// [`derived_wave_point_envelope`]).
const MAX_WAVE_POINTS_CAP: u64 = 1 << 20;
/// Number of shrinkable scalar leaves the clip strategy exposes.
const CLIP_SHRINK_LEAVES: u32 = 12;
const SHRINK_STEPS: usize = 64;
const DENSITY_SPLIT_PPM: u32 = 500_000;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

fn bits_eq(left: f32, right: f32) -> bool {
    left.to_bits() == right.to_bits()
}

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

fn checked_i64(value: u64) -> Result<i64, DomainError> {
    i64::try_from(value).map_err(|_| DomainError::ArithmeticOverflow)
}

fn checked_u32(value: u64) -> Result<u32, DomainError> {
    u32::try_from(value).map_err(|_| DomainError::ArithmeticOverflow)
}

fn checked_usize(value: u64) -> Result<usize, DomainError> {
    usize::try_from(value).map_err(|_| DomainError::ArithmeticOverflow)
}

// ─────────────────────────────────────────────────────────────────────────────
// M-GYROID-PHASE — phase limits, bands and case construction
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct PhaseLimits {
    max_phase_rad: f64,
    max_quarter: i64,
    max_grid_units: i64,
    grid_ratio_limit: i64,
}

fn checked_phase_limits(bounds: &BTreeMap<String, u64>) -> Result<PhaseLimits, DomainError> {
    let milliradians = required(bounds, "max_phase_milliradians")?;
    let max_grid_units = checked_i64(required(bounds, "max_grid_units")?)?;
    if milliradians == 0 {
        return Err(DomainError::InvalidBounds(
            "max_phase_milliradians must be positive".to_owned(),
        ));
    }
    if milliradians > MAX_PHASE_MILLIRADIANS_CAP {
        return Err(DomainError::InvalidBounds(format!(
            "max_phase_milliradians {milliradians} exceeds the derived cap \
             {MAX_PHASE_MILLIRADIANS_CAP}"
        )));
    }
    if milliradians < MIN_PHASE_MILLIRADIANS {
        return Err(DomainError::InvalidBounds(format!(
            "max_phase_milliradians {milliradians} cannot reach every orientation band \
             (needs at least {MIN_PHASE_MILLIRADIANS})"
        )));
    }
    if max_grid_units < 1 {
        return Err(DomainError::InvalidBounds(
            "max_grid_units must be positive".to_owned(),
        ));
    }
    if max_grid_units > MAX_GRID_UNITS_CAP as i64 {
        return Err(DomainError::InvalidBounds(format!(
            "max_grid_units {max_grid_units} exceeds the exact-dyadic cap \
             {MAX_GRID_UNITS_CAP}"
        )));
    }
    let max_phase_rad = milliradians as f64 / 1_000.0;
    // One quarter turn is reserved so every band's upper jitter stays inside
    // the advertised phase bound.
    let max_quarter = (max_phase_rad / FRAC_PI_4).floor() as i64 - 1;
    if max_quarter < 3 {
        return Err(DomainError::InvalidBounds(
            "max_phase_milliradians leaves no room for all four quarter-turn bands".to_owned(),
        ));
    }
    Ok(PhaseLimits {
        max_phase_rad,
        max_quarter,
        max_grid_units,
        grid_ratio_limit: GRID_RATIO + 1,
    })
}

fn live_phase_limits(run: &PropertyRun) -> PhaseLimits {
    let mut bounds = BTreeMap::new();
    for key in PHASE_BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("M-GYROID-PHASE bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_phase_limits(&bounds).expect("M-GYROID-PHASE bound consistency")
}

fn phase_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_phase_milliradians".to_owned(), 6_284),
        ("max_grid_units".to_owned(), 512),
    ])
}

fn phase_probe_limits() -> PhaseLimits {
    checked_phase_limits(&phase_probe_bounds()).expect("the phase probe bound set is valid")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaseBand {
    /// `z ∈ (0, π/4)`; `|sin z| < |cos z|` so the caller picks vertical.
    TieLower,
    /// `z ∈ (π/4, π/2)`; horizontal.
    MidLower,
    /// `z ∈ (π/2, 3π/4)`; horizontal.
    MidUpper,
    /// `z ∈ (3π/4, π)`; vertical.
    TieUpper,
}

const PHASE_BANDS: [PhaseBand; 4] = [
    PhaseBand::TieLower,
    PhaseBand::MidLower,
    PhaseBand::MidUpper,
    PhaseBand::TieUpper,
];

impl PhaseBand {
    fn residue(self) -> i64 {
        match self {
            Self::TieLower => 0,
            Self::MidLower => 1,
            Self::MidUpper => 2,
            Self::TieUpper => 3,
        }
    }

    fn vertical(self) -> bool {
        matches!(self, Self::TieLower | Self::TieUpper)
    }

    fn label(self) -> &'static str {
        match self {
            Self::TieLower => "tie-lower",
            Self::MidLower => "mid-lower",
            Self::MidUpper => "mid-upper",
            Self::TieUpper => "tie-upper",
        }
    }
}

/// `quarter` quarter-turns past zero, constrained to the band's residue class.
///
/// Subtracting four quarter turns preserves the residue class, so a shrink of
/// `j` never moves the case to another orientation band.
fn band_quarter_turn(band: PhaseBand, j: i64, max_quarter: i64) -> i64 {
    let mut quarter = band.residue() + 4 * j;
    if quarter > max_quarter && quarter >= 4 {
        quarter -= 4;
    }
    assert_eq!(quarter.rem_euclid(4), band.residue());
    assert!(quarter <= max_quarter);
    quarter
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GridCase {
    grid_units: i64,
    val_units: i64,
    shift: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PhaseCase {
    band: PhaseBand,
    phase: f64,
    x: f64,
    flip: bool,
    grid: GridCase,
}

fn phase_strategy(
    limits: &PhaseLimits,
    bands: &[PhaseBand],
    fixed_flip: Option<bool>,
) -> BoxedStrategy<PhaseCase> {
    assert!(!bands.is_empty(), "at least one phase band is required");
    let limits = limits.clone();
    let max_j = limits.max_quarter / 4;
    let max_phase = limits.max_phase_rad;
    let max_grid = limits.max_grid_units;
    let flip: BoxedStrategy<bool> = match fixed_flip {
        Some(value) => Just(value).boxed(),
        None => any::<bool>().boxed(),
    };
    (
        prop::sample::select(bands.to_vec()).no_shrink(),
        any::<bool>(),
        flip,
        -max_phase..=max_phase,
        0_i64..=max_j,
        QUARTER_BAND_MIN..=QUARTER_BAND_MAX,
        1_i64..=max_grid,
        -GRID_RATIO..=GRID_RATIO,
        any::<bool>(),
        0_i64..=max_grid - 1,
        -GRID_SHIFT..=GRID_SHIFT,
    )
        .prop_map(
            move |(
                band,
                negative,
                flip,
                x,
                j,
                within,
                grid_units,
                quotient,
                exact,
                residual,
                shift,
            )| {
                let quarter = band_quarter_turn(band, j, limits.max_quarter);
                let magnitude = (quarter as f64 + within) * FRAC_PI_4;
                let phase = if negative { -magnitude } else { magnitude };
                let remainder = if exact {
                    0
                } else if grid_units > 1 {
                    1 + residual % (grid_units - 1)
                } else {
                    0
                };
                PhaseCase {
                    band,
                    phase,
                    x,
                    flip,
                    grid: GridCase {
                        grid_units,
                        val_units: quotient * grid_units + remainder,
                        shift,
                    },
                }
            },
        )
        .boxed()
}

fn expected_vertical(z_sin: f64, z_cos: f64) -> bool {
    z_sin.abs() <= z_cos.abs()
}

fn branch_arguments(x: f64, z_sin: f64, z_cos: f64, vertical: bool, flip: bool) -> (f64, f64, f64) {
    if vertical {
        let phase_offset = if z_cos < 0.0 { PI } else { 0.0 } + PI;
        let a = (x + phase_offset).sin();
        let b = -z_cos;
        let res = z_sin * (x + phase_offset + if flip { PI } else { 0.0 }).cos();
        let r = (a * a + b * b).sqrt();
        (r, a, res)
    } else {
        let phase_offset = if z_sin < 0.0 { PI } else { 0.0 };
        let a = (x + phase_offset).cos();
        let b = -z_sin;
        let res = z_cos * (x + phase_offset + if flip { 0.0 } else { PI }).sin();
        let r = (a * a + b * b).sqrt();
        (r, a, res)
    }
}

/// Exact branch range derived from the SUT's algebra before it is asserted:
/// both `asin` terms lie in `[-π/2, π/2]`, so the vertical branch's `+ PI`
/// shift gives `[0, 2π]` and the horizontal branch's `+ FRAC_PI_2` shift gives
/// `[-π/2, 3π/2]`. `check_phase_value` widens both ends by the derived
/// `PHASE_RANGE_SLACK` only.
fn branch_range(vertical: bool) -> (f64, f64) {
    if vertical {
        (0.0, TAU)
    } else {
        (-FRAC_PI_2, 3.0 * FRAC_PI_2)
    }
}

/// Validate one observed `gyroid_f` result against the exact branch range.
fn check_phase_value(
    x: f64,
    z_sin: f64,
    z_cos: f64,
    vertical: bool,
    flip: bool,
    observed: f64,
    what: &str,
) -> Result<(), String> {
    if !observed.is_finite() {
        return Err(format!(
            "{what}: gyroid_f returned a non-finite value {observed} at x={x} \
             vertical={vertical} flip={flip}"
        ));
    }
    let (low, high) = branch_range(vertical);
    if observed < low - PHASE_RANGE_SLACK || observed > high + PHASE_RANGE_SLACK {
        return Err(format!(
            "{what}: gyroid_f returned {observed} outside the derived branch range \
             [{low}, {high}] at x={x} z_sin={z_sin} z_cos={z_cos} \
             vertical={vertical} flip={flip}"
        ));
    }
    Ok(())
}

#[derive(Debug, Default, Clone, Copy)]
struct PhaseObservation {
    vertical: bool,
    negative_phase: bool,
    flip: bool,
    periodic: bool,
}

fn check_phase_case(limits: &PhaseLimits, case: &PhaseCase) -> Result<PhaseObservation, String> {
    if !case.phase.is_finite() || case.phase.abs() > limits.max_phase_rad {
        return Err(format!(
            "sampled phase {} leaves the bound ±{}",
            case.phase, limits.max_phase_rad
        ));
    }
    if !case.x.is_finite() || case.x.abs() > limits.max_phase_rad {
        return Err(format!(
            "sampled x {} leaves the bound ±{}",
            case.x, limits.max_phase_rad
        ));
    }
    // The orientation is derived from the same phase the caller hands to the
    // kernel; the band only stratifies the draw, it never asserts the branch.
    let z_sin = case.phase.sin();
    let z_cos = case.phase.cos();
    let vertical = expected_vertical(z_sin, z_cos);
    if vertical != case.band.vertical() {
        return Err(format!(
            "band {} produced {} orientation at phase {}",
            case.band.label(),
            if vertical { "vertical" } else { "horizontal" },
            case.phase
        ));
    }
    let observed = gyroid_f(case.x, z_sin, z_cos, vertical, case.flip);
    check_phase_value(
        case.x,
        z_sin,
        z_cos,
        vertical,
        case.flip,
        observed,
        "phase case",
    )?;

    let (r, a, res) = branch_arguments(case.x, z_sin, z_cos, vertical, case.flip);
    let mut periodic = false;
    if r >= PERIODIC_MIN_RADIUS {
        let u1 = a / r;
        let u2 = res / r;
        if u1.abs() <= PERIODIC_MAX_ARGUMENT && u2.abs() <= PERIODIC_MAX_ARGUMENT {
            let shifted = gyroid_f(case.x + TAU, z_sin, z_cos, vertical, case.flip);
            check_phase_value(
                case.x + TAU,
                z_sin,
                z_cos,
                vertical,
                case.flip,
                shifted,
                "periodic observation",
            )?;
            let difference = (shifted - observed).abs();
            if difference > PERIODIC_TOLERANCE {
                return Err(format!(
                    "gyroid_f is not 2π-periodic within the derived envelope at \
                     x={} phase={}: |Δ| = {difference:e} > {PERIODIC_TOLERANCE:e}",
                    case.x, case.phase
                ));
            }
            periodic = true;
        }
    }
    Ok(PhaseObservation {
        vertical,
        negative_phase: case.phase < 0.0,
        flip: case.flip,
        periodic,
    })
}

/// Exact integer floor model for `align_to_grid`, independent of the SUT.
fn grid_expectation(grid_units: i64, val_units: i64) -> i64 {
    val_units.div_euclid(grid_units) * grid_units
}

/// Compare one observed grid result to the exact integer model.
fn check_grid_value(
    grid_units: i64,
    val_units: i64,
    observed: f64,
    what: &str,
) -> Result<(), String> {
    let expected = grid_expectation(grid_units, val_units) as f64;
    if observed.to_bits() != expected.to_bits() {
        return Err(format!(
            "{what}: align_to_grid({val_units}, {grid_units}) returned {observed}, \
             independent floor model expects {expected}"
        ));
    }
    Ok(())
}

#[derive(Debug, Default, Clone, Copy)]
struct GridObservation {
    grid_multiple: bool,
    negative_non_multiple: bool,
    translated: bool,
}

fn check_grid_case(limits: &PhaseLimits, case: &GridCase) -> Result<GridObservation, String> {
    if case.grid_units < 1 || case.grid_units > limits.max_grid_units {
        return Err(format!(
            "sampled grid {} leaves 1..={}",
            case.grid_units, limits.max_grid_units
        ));
    }
    if case.val_units.unsigned_abs() as f64
        > limits.grid_ratio_limit as f64 * case.grid_units as f64
    {
        return Err(format!(
            "sampled value {} exceeds {} grids of {}",
            case.val_units, limits.grid_ratio_limit, case.grid_units
        ));
    }
    if case.shift.abs() > GRID_SHIFT {
        return Err(format!("sampled shift {} leaves ±{GRID_SHIFT}", case.shift));
    }
    let grid = case.grid_units as f64;
    let val = case.val_units as f64;
    check_grid_value(
        case.grid_units,
        case.val_units,
        align_to_grid(val, grid),
        "grid case",
    )?;
    // Translation by whole periods is exact: both operands stay small integers.
    let shifted_units = case.val_units + case.shift * case.grid_units;
    let shifted = align_to_grid(shifted_units as f64, grid);
    check_grid_value(
        case.grid_units,
        shifted_units,
        shifted,
        "translated grid case",
    )?;
    let expected_base = grid_expectation(case.grid_units, case.val_units);
    let expected_shifted = expected_base + case.shift * case.grid_units;
    if shifted.to_bits() != (expected_shifted as f64).to_bits() {
        return Err(format!(
            "whole-period translation moved the floor: base {expected_base}, \
             shifted {shifted} vs {expected_shifted}"
        ));
    }
    Ok(GridObservation {
        grid_multiple: case.val_units % case.grid_units == 0,
        negative_non_multiple: case.val_units < 0 && case.val_units % case.grid_units != 0,
        translated: case.shift != 0,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// M-GYROID-CLIP — clip limits, case construction and the independent model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ClipLimits {
    max_abs_units: i64,
    max_vertices: usize,
    max_holes: usize,
    max_components: usize,
    max_shrink_candidates: u32,
    max_width_units: i64,
    min_density_ppm: u32,
    max_phase_rad: f64,
    max_quarter: i64,
    max_wave_points: usize,
    width_floor: i64,
}

fn checked_clip_limits(bounds: &BTreeMap<String, u64>) -> Result<ClipLimits, DomainError> {
    let max_abs_units = checked_i64(required(bounds, "max_abs_units")?)?;
    let max_vertices = checked_usize(required(bounds, "max_vertices")?)?;
    let max_holes = checked_usize(required(bounds, "max_holes")?)?;
    let max_components = checked_usize(required(bounds, "max_components")?)?;
    let max_shrink_candidates = checked_u32(required(bounds, "max_shrink_candidates")?)?;
    let max_width_units = checked_i64(required(bounds, "max_width_units")?)?;
    let min_density_ppm = checked_u32(required(bounds, "min_density_ppm")?)?;
    let milliradians = required(bounds, "max_phase_milliradians")?;
    let max_wave_points = checked_usize(required(bounds, "max_wave_points")?)?;

    if !(1..=1_000_000).contains(&max_abs_units) {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units {max_abs_units} must be in 1..=1000000"
        )));
    }
    if max_vertices < 8 {
        return Err(DomainError::InvalidBounds(
            "max_vertices must admit a rectangle with a rectangular hole (8)".to_owned(),
        ));
    }
    if max_holes < 1 {
        return Err(DomainError::InvalidBounds(
            "max_holes must admit the required rectangular hole".to_owned(),
        ));
    }
    if max_components < 1 {
        return Err(DomainError::InvalidBounds(
            "max_components must admit at least one component".to_owned(),
        ));
    }
    if max_shrink_candidates < CLIP_SHRINK_LEAVES {
        return Err(DomainError::InvalidBounds(format!(
            "max_shrink_candidates {max_shrink_candidates} must admit the \
             {CLIP_SHRINK_LEAVES} shrinkable scalar leaves"
        )));
    }
    if max_width_units < 1 {
        return Err(DomainError::InvalidBounds(
            "max_width_units must be positive".to_owned(),
        ));
    }
    if min_density_ppm == 0 || min_density_ppm > 1_000_000 {
        return Err(DomainError::InvalidBounds(format!(
            "min_density_ppm {min_density_ppm} must be in 1..=1000000"
        )));
    }
    if milliradians == 0 || milliradians > MAX_PHASE_MILLIRADIANS_CAP {
        return Err(DomainError::InvalidBounds(format!(
            "max_phase_milliradians {milliradians} must be in 1..={MAX_PHASE_MILLIRADIANS_CAP}"
        )));
    }
    if milliradians < MIN_PHASE_MILLIRADIANS {
        return Err(DomainError::InvalidBounds(format!(
            "max_phase_milliradians {milliradians} cannot reach every orientation band \
             (needs at least {MIN_PHASE_MILLIRADIANS})"
        )));
    }
    if max_wave_points as u64 > MAX_WAVE_POINTS_CAP {
        return Err(DomainError::InvalidBounds(format!(
            "max_wave_points {max_wave_points} exceeds the derived generation envelope cap \
             {MAX_WAVE_POINTS_CAP}"
        )));
    }
    let envelope = derived_wave_point_envelope();
    if max_wave_points < envelope {
        return Err(DomainError::InvalidBounds(format!(
            "max_wave_points {max_wave_points} is below the derived worst-case wave envelope \
             {envelope}; a legal case could trip the count limit"
        )));
    }
    let width_floor = (max_width_units / 4).max(1);
    let spacing_min_units = width_floor as f64 / DENSITY_ADJUST;
    if spacing_min_units < 100.0 {
        return Err(DomainError::InvalidBounds(format!(
            "max_width_units {max_width_units} leaves a {spacing_min_units:.1}-unit lower \
             wave spacing, below the 100-unit clip-band resolution floor"
        )));
    }
    let spacing_max_units =
        max_width_units as f64 * 1_000_000.0 / (DENSITY_ADJUST * min_density_ppm as f64);
    let span = MAX_COMPONENT_SPAN_MULTIPLE * spacing_max_units;
    if span > max_abs_units as f64 {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units {max_abs_units} cannot hold the largest case \
             ({MAX_COMPONENT_SPAN_MULTIPLE} spacings at {spacing_max_units:.0} units = \
             {span:.0} units)"
        )));
    }
    let max_phase_rad = milliradians as f64 / 1_000.0;
    let max_quarter = (max_phase_rad / FRAC_PI_4).floor() as i64 - 1;
    if max_quarter < 3 {
        return Err(DomainError::InvalidBounds(
            "max_phase_milliradians leaves no room for all four quarter-turn bands".to_owned(),
        ));
    }
    Ok(ClipLimits {
        max_abs_units,
        max_vertices,
        max_holes,
        max_components,
        max_shrink_candidates,
        max_width_units,
        min_density_ppm,
        max_phase_rad,
        max_quarter,
        max_wave_points,
        width_floor,
    })
}

fn live_clip_limits(run: &PropertyRun) -> ClipLimits {
    let mut bounds = BTreeMap::new();
    for key in CLIP_BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("M-GYROID-CLIP bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_clip_limits(&bounds).expect("M-GYROID-CLIP bound consistency")
}

fn clip_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 524_288),
        ("max_vertices".to_owned(), 32),
        ("max_holes".to_owned(), 4),
        ("max_components".to_owned(), 2),
        ("max_shrink_candidates".to_owned(), 32),
        ("max_width_units".to_owned(), 6_000),
        ("min_density_ppm".to_owned(), 100_000),
        ("max_phase_milliradians".to_owned(), 6_284),
        ("max_wave_points".to_owned(), 65_536),
    ])
}

fn clip_probe_limits() -> ClipLimits {
    checked_clip_limits(&clip_probe_bounds()).expect("the clip probe bound set is valid")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipTopology {
    PlainRect,
    HoledRect,
    TwoComponents,
}

const CLIP_TOPOLOGIES: [ClipTopology; 3] = [
    ClipTopology::PlainRect,
    ClipTopology::HoledRect,
    ClipTopology::TwoComponents,
];

impl ClipTopology {
    fn label(self) -> &'static str {
        match self {
            Self::PlainRect => "plain-rect",
            Self::HoledRect => "holed-rect",
            Self::TwoComponents => "two-components",
        }
    }

    fn vertices(self) -> usize {
        match self {
            Self::PlainRect => 4,
            Self::HoledRect => 8,
            Self::TwoComponents => 16,
        }
    }

    fn holes(self) -> usize {
        match self {
            Self::PlainRect => 0,
            Self::HoledRect => 1,
            Self::TwoComponents => 2,
        }
    }

    fn components(self) -> usize {
        match self {
            Self::PlainRect => 1,
            Self::HoledRect => 1,
            Self::TwoComponents => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipAngle {
    /// `base_angle = 45°` cancels the −45° correction, so waves run axis-aligned.
    Axis,
    /// `base_angle = 15°` leaves a −30° generation rotation.
    Rotated,
}

const CLIP_ANGLES: [ClipAngle; 2] = [ClipAngle::Axis, ClipAngle::Rotated];

impl ClipAngle {
    fn base_angle(self) -> f32 {
        match self {
            Self::Axis => 45.0,
            Self::Rotated => 15.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DensityClass {
    Low,
    High,
}

impl DensityClass {
    fn of(ppm: u32) -> Self {
        if ppm <= DENSITY_SPLIT_PPM {
            Self::Low
        } else {
            Self::High
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    min_x: i64,
    min_y: i64,
    max_x: i64,
    max_y: i64,
}

impl Rect {
    fn width(self) -> i64 {
        self.max_x - self.min_x
    }

    fn height(self) -> i64 {
        self.max_y - self.min_y
    }

    fn contains_strictly(self, other: Rect, margin: i64) -> bool {
        other.min_x - self.min_x >= margin
            && self.max_x - other.max_x >= margin
            && other.min_y - self.min_y >= margin
            && self.max_y - other.max_y >= margin
    }
}

#[derive(Debug, Clone)]
struct ClipPart {
    outer: Rect,
    holes: Vec<Rect>,
}

#[derive(Debug, Clone)]
struct ClipCase {
    parts: Vec<ClipPart>,
    generation: Rect,
    density: f32,
    spacing_width_mm: f32,
    stamped_width_mm: f32,
    speed_factor: f32,
    z: f32,
    z_phase: f64,
    base_angle: f32,
    topology: ClipTopology,
    angle: ClipAngle,
    class: DensityClass,
    spacing_units: i64,
}

/// One raw draw of the clip strategy before bound/geometry construction.
#[derive(Debug, Clone, Copy)]
struct ClipSample {
    topology: ClipTopology,
    angle: ClipAngle,
    density_ppm: u32,
    width_units: i64,
    size_x_extra: i64,
    size_y_extra: i64,
    hole_x_multiple: i64,
    hole_y_multiple: i64,
    offset_x: i64,
    offset_y: i64,
    phase_quarter: i64,
    phase_within: f64,
    phase_negative: bool,
    speed_milli: i32,
}

fn clip_strategy(
    limits: &ClipLimits,
    topologies: &[ClipTopology],
    angles: &[ClipAngle],
    density_range: (u32, u32),
    width_range: (i64, i64),
) -> BoxedStrategy<ClipSample> {
    assert!(
        !topologies.is_empty(),
        "at least one clip topology is required"
    );
    assert!(!angles.is_empty(), "at least one clip angle is required");
    assert!(density_range.0 >= 1 && density_range.0 <= density_range.1);
    assert!(width_range.0 >= 1 && width_range.0 <= width_range.1);
    let (low_density, high_density) = density_range;
    let (low_width, high_width) = width_range;
    (
        (
            prop::sample::select(topologies.to_vec()).no_shrink(),
            prop::sample::select(angles.to_vec()).no_shrink(),
            low_density..=high_density,
            low_width..=high_width,
            0_i64..=2,
            0_i64..=2,
        ),
        (
            MIN_HOLE_MULTIPLE..=MAX_HOLE_MULTIPLE,
            MIN_HOLE_MULTIPLE..=MAX_HOLE_MULTIPLE,
            -(MAX_SIZE_MULTIPLE - MIN_HOLE_MULTIPLE - 1)
                ..=(MAX_SIZE_MULTIPLE - MIN_HOLE_MULTIPLE - 1),
            -(MAX_SIZE_MULTIPLE - MIN_HOLE_MULTIPLE - 1)
                ..=(MAX_SIZE_MULTIPLE - MIN_HOLE_MULTIPLE - 1),
            0_i64..=limits.max_quarter,
            QUARTER_BAND_MIN..=QUARTER_BAND_MAX,
            any::<bool>(),
            250_i32..=1_500,
        ),
    )
        .prop_map(
            move |(
                (topology, angle, density_ppm, width_units, sx_extra, sy_extra),
                (
                    hole_x_multiple,
                    hole_y_multiple,
                    offset_x,
                    offset_y,
                    phase_quarter,
                    phase_within,
                    phase_negative,
                    speed_milli,
                ),
            )| ClipSample {
                topology,
                angle,
                density_ppm,
                width_units,
                size_x_extra: sx_extra,
                size_y_extra: sy_extra,
                hole_x_multiple,
                hole_y_multiple,
                offset_x,
                offset_y,
                phase_quarter,
                phase_within,
                phase_negative,
                speed_milli,
            },
        )
        .boxed()
}

fn spacing_units(density_ppm: u32, width_units: i64) -> i64 {
    let density = density_ppm as f64 / 1_000_000.0;
    let width_mm = units_to_mm(width_units) as f64;
    let spacing_mm = width_mm / (density * DENSITY_ADJUST);
    (spacing_mm * 10_000.0).round() as i64
}

fn build_clip_case(limits: &ClipLimits, sample: &ClipSample) -> Result<ClipCase, DomainError> {
    if sample.density_ppm == 0 || sample.density_ppm > 1_000_000 {
        return Err(DomainError::InvalidInput(format!(
            "density {} ppm is outside (0, 1]",
            sample.density_ppm
        )));
    }
    if sample.width_units < limits.width_floor || sample.width_units > limits.max_width_units {
        return Err(DomainError::InvalidInput(format!(
            "width {} units leaves {}..={}",
            sample.width_units, limits.width_floor, limits.max_width_units
        )));
    }
    if sample.topology.vertices() > limits.max_vertices
        || sample.topology.holes() > limits.max_holes
        || sample.topology.components() > limits.max_components
    {
        return Err(DomainError::InvalidBounds(format!(
            "bounds cannot hold the {} topology",
            sample.topology.label()
        )));
    }
    let density = sample.density_ppm as f32 / 1_000_000.0;
    let width_mm = units_to_mm(sample.width_units);
    let spacing_mm = width_mm as f64 / (density as f64 * DENSITY_ADJUST);
    if !spacing_mm.is_finite() || spacing_mm <= 0.0 {
        return Err(DomainError::InvalidInput(
            "constructed spacing is not finite and positive".to_owned(),
        ));
    }
    let spacing = spacing_units(sample.density_ppm, sample.width_units);
    if spacing < 1 {
        return Err(DomainError::InvalidInput(
            "constructed spacing rounds below one canonical unit".to_owned(),
        ));
    }
    let size_x = sample.hole_x_multiple + 2 + sample.size_x_extra;
    let size_y = sample.hole_y_multiple + 2 + sample.size_y_extra;
    if !(MIN_SIZE_MULTIPLE..=MAX_SIZE_MULTIPLE).contains(&size_x)
        || !(MIN_SIZE_MULTIPLE..=MAX_SIZE_MULTIPLE).contains(&size_y)
    {
        return Err(DomainError::InvalidInput(format!(
            "constructed half-extents {size_x}x{size_y} leave the derived span"
        )));
    }
    let half_x = size_x * spacing;
    let half_y = size_y * spacing;
    let hole_x = sample.hole_x_multiple * spacing;
    let hole_y = sample.hole_y_multiple * spacing;
    let offset_x = sample.offset_x.clamp(
        -(size_x - sample.hole_x_multiple - 1),
        size_x - sample.hole_x_multiple - 1,
    ) * spacing;
    let offset_y = sample.offset_y.clamp(
        -(size_y - sample.hole_y_multiple - 1),
        size_y - sample.hole_y_multiple - 1,
    ) * spacing;

    let hole = |cx: i64, cy: i64| Rect {
        min_x: cx - hole_x,
        min_y: cy - hole_y,
        max_x: cx + hole_x,
        max_y: cy + hole_y,
    };
    let parts = match sample.topology {
        ClipTopology::PlainRect => vec![ClipPart {
            outer: Rect {
                min_x: -half_x,
                min_y: -half_y,
                max_x: half_x,
                max_y: half_y,
            },
            holes: Vec::new(),
        }],
        ClipTopology::HoledRect => {
            let outer = Rect {
                min_x: -half_x,
                min_y: -half_y,
                max_x: half_x,
                max_y: half_y,
            };
            let ring = hole(offset_x, offset_y);
            if !outer.contains_strictly(ring, spacing) {
                return Err(DomainError::InvalidInput(
                    "constructed hole is not strictly inside its contour".to_owned(),
                ));
            }
            vec![ClipPart {
                outer,
                holes: vec![ring],
            }]
        }
        ClipTopology::TwoComponents => {
            let gap = spacing;
            let left = Rect {
                min_x: -2 * half_x - gap,
                min_y: -half_y,
                max_x: -gap,
                max_y: half_y,
            };
            let right = Rect {
                min_x: gap,
                min_y: -half_y,
                max_x: 2 * half_x + gap,
                max_y: half_y,
            };
            let left_hole = hole(-half_x - gap, 0);
            let right_hole = hole(half_x + gap, 0);
            if !left.contains_strictly(left_hole, spacing)
                || !right.contains_strictly(right_hole, spacing)
            {
                return Err(DomainError::InvalidInput(
                    "constructed component hole is not strictly inside its contour".to_owned(),
                ));
            }
            vec![
                ClipPart {
                    outer: left,
                    holes: vec![left_hole],
                },
                ClipPart {
                    outer: right,
                    holes: vec![right_hole],
                },
            ]
        }
    };
    let generation = Rect {
        min_x: parts.iter().map(|part| part.outer.min_x).min().unwrap_or(0),
        min_y: parts.iter().map(|part| part.outer.min_y).min().unwrap_or(0),
        max_x: parts.iter().map(|part| part.outer.max_x).max().unwrap_or(0),
        max_y: parts.iter().map(|part| part.outer.max_y).max().unwrap_or(0),
    };
    let magnitude = (sample.phase_quarter as f64 + sample.phase_within) * FRAC_PI_4;
    let z_phase = if sample.phase_negative {
        -magnitude
    } else {
        magnitude
    };
    let z = (z_phase * spacing_mm) as f32;
    let stamped_width_mm = (width_mm as f64 * 1.125) as f32;
    Ok(ClipCase {
        parts,
        generation,
        density,
        spacing_width_mm: width_mm,
        stamped_width_mm,
        speed_factor: sample.speed_milli as f32 / 1_000.0,
        z,
        z_phase,
        base_angle: sample.angle.base_angle(),
        topology: sample.topology,
        angle: sample.angle,
        class: DensityClass::of(sample.density_ppm),
        spacing_units: spacing,
    })
}

fn rect_points(rect: Rect, clockwise: bool) -> Polygon {
    let points = if clockwise {
        vec![
            Point2 {
                x: rect.min_x,
                y: rect.min_y,
            },
            Point2 {
                x: rect.min_x,
                y: rect.max_y,
            },
            Point2 {
                x: rect.max_x,
                y: rect.max_y,
            },
            Point2 {
                x: rect.max_x,
                y: rect.min_y,
            },
        ]
    } else {
        vec![
            Point2 {
                x: rect.min_x,
                y: rect.min_y,
            },
            Point2 {
                x: rect.max_x,
                y: rect.min_y,
            },
            Point2 {
                x: rect.max_x,
                y: rect.max_y,
            },
            Point2 {
                x: rect.min_x,
                y: rect.max_y,
            },
        ]
    };
    Polygon { points }
}

fn plain_expolygon(rect: Rect) -> ExPolygon {
    ExPolygon {
        contour: rect_points(rect, false),
        holes: Vec::new(),
    }
}

fn boundary_expolygons(case: &ClipCase) -> Vec<ExPolygon> {
    case.parts
        .iter()
        .map(|part| ExPolygon {
            contour: rect_points(part.outer, false),
            holes: part
                .holes
                .iter()
                .map(|hole| rect_points(*hole, true))
                .collect(),
        })
        .collect()
}

fn validate_clip_case(case: &ClipCase, limits: &ClipLimits) -> Result<(), String> {
    if !case.density.is_finite() || case.density <= 0.0 || case.density > 1.0 {
        return Err(format!("density {} leaves (0, 1]", case.density));
    }
    if ((case.density * 1_000_000.0).round() as u32) < limits.min_density_ppm {
        return Err(format!(
            "density {} is below the profile minimum {} ppm",
            case.density, limits.min_density_ppm
        ));
    }
    if !case.spacing_width_mm.is_finite() || case.spacing_width_mm <= 0.0 {
        return Err("spacing width is not finite and positive".to_owned());
    }
    let width_units = (case.spacing_width_mm as f64 * 10_000.0).round() as i64;
    if width_units < limits.width_floor || width_units > limits.max_width_units {
        return Err(format!(
            "width {width_units} units leaves {}..={}",
            limits.width_floor, limits.max_width_units
        ));
    }
    if !case.stamped_width_mm.is_finite() || case.stamped_width_mm <= 0.0 {
        return Err("stamped width is not finite and positive".to_owned());
    }
    if bits_eq(case.stamped_width_mm, case.spacing_width_mm) {
        return Err(
            "the stamped width equals the per-call spacing width; the metadata check would \
             not distinguish the two"
                .to_owned(),
        );
    }
    if !case.speed_factor.is_finite() || case.speed_factor <= 0.0 {
        return Err("speed factor is not finite and positive".to_owned());
    }
    if !case.z.is_finite() {
        return Err("z is not finite".to_owned());
    }
    let spacing_mm = case.spacing_width_mm as f64 / (case.density as f64 * DENSITY_ADJUST);
    let derived_phase = case.z as f64 / spacing_mm;
    if !case.z_phase.is_finite() || case.z_phase.abs() > limits.max_phase_rad {
        return Err(format!("z phase {} leaves the bound", case.z_phase));
    }
    if (derived_phase - case.z_phase).abs() > 1e-5 * (1.0 + case.z_phase.abs()) {
        return Err(format!(
            "stamped z re-derives phase {derived_phase} instead of {}",
            case.z_phase
        ));
    }
    if case.parts.is_empty() || case.parts.len() > limits.max_components {
        return Err(format!("{} components leave the bound", case.parts.len()));
    }
    let mut vertices = 0_usize;
    let mut holes = 0_usize;
    for part in &case.parts {
        vertices += 4 + 4 * part.holes.len();
        holes += part.holes.len();
        if part.outer.width() <= 0 || part.outer.height() <= 0 {
            return Err("a clip contour is not a positive rectangle".to_owned());
        }
        for hole in &part.holes {
            if hole.width() <= 0 || hole.height() <= 0 {
                return Err("a clip hole is not a positive rectangle".to_owned());
            }
            if !part.outer.contains_strictly(*hole, 1) {
                return Err("a clip hole is not strictly inside its contour".to_owned());
            }
        }
    }
    if vertices > limits.max_vertices {
        return Err(format!("{vertices} vertices exceed the bound"));
    }
    if holes > limits.max_holes {
        return Err(format!("{holes} holes exceed the bound"));
    }
    let max_coord = case
        .parts
        .iter()
        .flat_map(|part| {
            let mut bounds = vec![
                part.outer.min_x,
                part.outer.min_y,
                part.outer.max_x,
                part.outer.max_y,
            ];
            for hole in &part.holes {
                bounds.extend([hole.min_x, hole.min_y, hole.max_x, hole.max_y]);
            }
            bounds
        })
        .map(i64::abs)
        .max()
        .unwrap_or(0);
    if max_coord > limits.max_abs_units {
        return Err(format!(
            "clip coordinate {max_coord} exceeds max_abs_units {}",
            limits.max_abs_units
        ));
    }
    if case.spacing_units < 1 {
        return Err("spacing rounds below one canonical unit".to_owned());
    }
    Ok(())
}

impl ClipCase {
    fn module(&self) -> GyroidInfill {
        GyroidInfill {
            density: self.density,
            base_angle: self.base_angle,
            line_width: self.stamped_width_mm,
        }
    }

    fn raw_paths(&self) -> Vec<ExtrusionPath3D> {
        let expoly = plain_expolygon(self.generation);
        self.module().fill_expolygon(
            &expoly,
            self.z,
            self.speed_factor,
            self.density,
            self.spacing_width_mm,
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct RawStats {
    paths: usize,
    segments: usize,
    points: usize,
    outside_points: usize,
}

fn check_raw_paths(
    case: &ClipCase,
    paths: &[ExtrusionPath3D],
    limits: &ClipLimits,
) -> Result<RawStats, String> {
    let mut stats = RawStats {
        paths: paths.len(),
        ..RawStats::default()
    };
    let outers: Vec<Rect> = case.parts.iter().map(|part| part.outer).collect();
    for (index, path) in paths.iter().enumerate() {
        if path.role != ExtrusionRole::SparseInfill {
            return Err(format!("raw path {index} has role {:?}", path.role));
        }
        if !bits_eq(path.speed_factor, case.speed_factor) {
            return Err(format!(
                "raw path {index} copies speed {} instead of {}",
                path.speed_factor, case.speed_factor
            ));
        }
        if path.tool_index.is_some() || path.order_lock.is_some() {
            return Err(format!("raw path {index} sets a tool or order lock"));
        }
        if path.points.len() < 2 {
            return Err(format!(
                "raw path {index} has {} points but fill_expolygon drops shorter paths",
                path.points.len()
            ));
        }
        stats.segments += path.points.len() - 1;
        for (point_index, point) in path.points.iter().enumerate() {
            stats.points += 1;
            if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
                return Err(format!(
                    "raw path {index} point {point_index} has a non-finite coordinate"
                ));
            }
            if !point.width.is_finite() || !point.flow_factor.is_finite() {
                return Err(format!(
                    "raw path {index} point {point_index} has a non-finite width/flow"
                ));
            }
            if !bits_eq(point.z, case.z) {
                return Err(format!(
                    "raw path {index} point {point_index} stamps z {} instead of {}",
                    point.z, case.z
                ));
            }
            if !bits_eq(point.width, case.stamped_width_mm) {
                return Err(format!(
                    "raw path {index} point {point_index} stamps width {} instead of the \
                     module-global {}",
                    point.width, case.stamped_width_mm
                ));
            }
            if !bits_eq(point.flow_factor, 1.0) {
                return Err(format!(
                    "raw path {index} point {point_index} stamps flow {} instead of 1.0",
                    point.flow_factor
                ));
            }
            if point.overhang_quartile.is_some()
                || point.overhang_distance_mm.is_some()
                || !bits_eq(point.dist_to_top_mm, 0.0)
            {
                return Err(format!(
                    "raw path {index} point {point_index} carries overhang metadata"
                ));
            }
            let unit = Point2::from_mm(point.x, point.y);
            let inside = outers.iter().any(|rect| {
                unit.x >= rect.min_x
                    && unit.x <= rect.max_x
                    && unit.y >= rect.min_y
                    && unit.y <= rect.max_y
            });
            if !inside {
                stats.outside_points += 1;
            }
        }
    }
    if stats.points > limits.max_wave_points {
        return Err(format!(
            "raw waves emitted {} points, above the derived envelope {}",
            stats.points, limits.max_wave_points
        ));
    }
    Ok(stats)
}

// ── exact rational interval model ───────────────────────────────────────────

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

/// Exact rational with a positive denominator, kept in lowest terms.
#[derive(Debug, Clone, Copy)]
struct Frac {
    num: i128,
    den: i128,
}

impl Frac {
    fn new(num: i128, den: i128) -> Self {
        assert!(den != 0, "rational denominator must be nonzero");
        let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
        let divisor = gcd_u128(num.unsigned_abs(), den as u128) as i128;
        Self {
            num: num / divisor,
            den: den / divisor,
        }
    }

    fn zero() -> Self {
        Self { num: 0, den: 1 }
    }

    fn one() -> Self {
        Self { num: 1, den: 1 }
    }

    fn cmp_frac(&self, other: &Self) -> Ordering {
        (self.num * other.den).cmp(&(other.num * self.den))
    }

    fn midpoint(&self, other: &Self) -> Self {
        Self {
            num: self.num * other.den + other.num * self.den,
            den: 2 * self.den * other.den,
        }
    }

    fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    fn le_int(&self, value: i64) -> bool {
        self.num <= value as i128 * self.den
    }

    fn ge_int(&self, value: i64) -> bool {
        self.num >= value as i128 * self.den
    }

    fn lt_int(&self, value: i64) -> bool {
        self.num < value as i128 * self.den
    }

    fn gt_int(&self, value: i64) -> bool {
        self.num > value as i128 * self.den
    }
}

fn coordinate_at(origin: i64, target: i64, t: Frac) -> Frac {
    Frac::new(
        origin as i128 * t.den + (target as i128 - origin as i128) * t.num,
        t.den,
    )
}

fn clip_x_boundaries(case: &ClipCase) -> Vec<i64> {
    let mut out = Vec::new();
    for part in &case.parts {
        out.push(part.outer.min_x - CLIP_INFLATION_UNITS);
        out.push(part.outer.max_x + CLIP_INFLATION_UNITS);
        for hole in &part.holes {
            out.push(hole.min_x + CLIP_INFLATION_UNITS);
            out.push(hole.max_x - CLIP_INFLATION_UNITS);
        }
    }
    out
}

fn clip_y_boundaries(case: &ClipCase) -> Vec<i64> {
    let mut out = Vec::new();
    for part in &case.parts {
        out.push(part.outer.min_y - CLIP_INFLATION_UNITS);
        out.push(part.outer.max_y + CLIP_INFLATION_UNITS);
        for hole in &part.holes {
            out.push(hole.min_y + CLIP_INFLATION_UNITS);
            out.push(hole.max_y - CLIP_INFLATION_UNITS);
        }
    }
    out
}

/// Model membership: inside an inflated contour and not strictly inside any
/// contracted hole. `x`/`y` are exact rational canonical units.
fn model_kept_at(case: &ClipCase, x: Frac, y: Frac) -> bool {
    let mut inside = false;
    for part in &case.parts {
        let outer = part.outer;
        if x.ge_int(outer.min_x - CLIP_INFLATION_UNITS)
            && x.le_int(outer.max_x + CLIP_INFLATION_UNITS)
            && y.ge_int(outer.min_y - CLIP_INFLATION_UNITS)
            && y.le_int(outer.max_y + CLIP_INFLATION_UNITS)
        {
            inside = true;
        }
        for hole in &part.holes {
            if x.gt_int(hole.min_x + CLIP_INFLATION_UNITS)
                && x.lt_int(hole.max_x - CLIP_INFLATION_UNITS)
                && y.gt_int(hole.min_y + CLIP_INFLATION_UNITS)
                && y.lt_int(hole.max_y - CLIP_INFLATION_UNITS)
            {
                return false;
            }
        }
    }
    inside
}

/// Orientation of the clip-universe edge a kept-interval endpoint sits on.
///
/// A `Vertical` edge is a line of constant `x`, so the subject segment's
/// *normal* component at that crossing is `|Δx|`; a `Horizontal` edge is a line
/// of constant `y`, so the normal component is `|Δy|`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundaryAxis {
    Vertical,
    Horizontal,
}

/// One exact kept parameter interval plus the boundary class of each end.
#[derive(Debug, Clone, Copy)]
struct ExpectedInterval {
    low: Frac,
    high: Frac,
    low_axis: Option<BoundaryAxis>,
    high_axis: Option<BoundaryAxis>,
}

/// Boundary class of a subject vertex that lies exactly on a modelled clip
/// edge line, or `None` for a vertex on no edge.
///
/// Clipper decides inclusion per integer scanline row, so a subject that only
/// *touches* an edge at a vertex (zero-length exact contact) or grazes it at a
/// shallow angle can still legitimately return a span starting up to
/// `band / |Δnormal|` of subject parameter before the contact. Treating such a
/// vertex as a plain subject endpoint (`band / length`) would forbid that. When
/// the vertex lies on both a vertical and a horizontal edge line the axis with
/// the smaller normal delta wins, because it has the larger (worse) term.
fn vertex_boundary_axis(
    point: Point2,
    x_boundaries: &[i64],
    y_boundaries: &[i64],
    delta_x: f64,
    delta_y: f64,
) -> Option<BoundaryAxis> {
    let on_vertical = x_boundaries.contains(&point.x);
    let on_horizontal = y_boundaries.contains(&point.y);
    match (on_vertical, on_horizontal) {
        (true, false) => Some(BoundaryAxis::Vertical),
        (false, true) => Some(BoundaryAxis::Horizontal),
        (true, true) if delta_x <= delta_y => Some(BoundaryAxis::Vertical),
        (true, true) => Some(BoundaryAxis::Horizontal),
        (false, false) => None,
    }
}

/// Exact kept parameter intervals of one straight subject segment.
fn expected_segment_intervals(
    case: &ClipCase,
    start: Point2,
    end: Point2,
    x_boundaries: &[i64],
    y_boundaries: &[i64],
) -> Vec<ExpectedInterval> {
    if start == end {
        return Vec::new();
    }
    // A segment end that sits exactly on a modelled clip edge is a boundary
    // contact, not a plain subject endpoint: the reported cut there carries the
    // edge's tangential ill-conditioning, so it takes that edge's axis. See
    // `vertex_boundary_axis`.
    let delta_x = (end.x - start.x).unsigned_abs() as f64;
    let delta_y = (end.y - start.y).unsigned_abs() as f64;
    let mut criticals: Vec<(Frac, Option<BoundaryAxis>)> = vec![
        (
            Frac::zero(),
            vertex_boundary_axis(start, x_boundaries, y_boundaries, delta_x, delta_y),
        ),
        (
            Frac::one(),
            vertex_boundary_axis(end, x_boundaries, y_boundaries, delta_x, delta_y),
        ),
    ];
    let add_roots = |origin: i64,
                     target: i64,
                     boundaries: &[i64],
                     axis: BoundaryAxis,
                     criticals: &mut Vec<(Frac, Option<BoundaryAxis>)>| {
        let delta = target - origin;
        if delta == 0 {
            return;
        }
        for &boundary in boundaries {
            let t = Frac::new((boundary - origin) as i128, delta as i128);
            if t.cmp_frac(&Frac::zero()) == Ordering::Greater
                && t.cmp_frac(&Frac::one()) == Ordering::Less
            {
                criticals.push((t, Some(axis)));
            }
        }
    };
    add_roots(
        start.x,
        end.x,
        x_boundaries,
        BoundaryAxis::Vertical,
        &mut criticals,
    );
    add_roots(
        start.y,
        end.y,
        y_boundaries,
        BoundaryAxis::Horizontal,
        &mut criticals,
    );
    criticals.sort_by(|left, right| left.0.cmp_frac(&right.0));
    criticals.dedup_by(|current, kept| {
        if current.0.cmp_frac(&kept.0) == Ordering::Equal {
            kept.1 = kept.1.or(current.1);
            true
        } else {
            false
        }
    });

    let mut intervals: Vec<ExpectedInterval> = Vec::new();
    for window in criticals.windows(2) {
        let (low, low_axis) = window[0];
        let (high, high_axis) = window[1];
        let middle = low.midpoint(&high);
        let x = coordinate_at(start.x, end.x, middle);
        let y = coordinate_at(start.y, end.y, middle);
        if model_kept_at(case, x, y) {
            match intervals.last_mut() {
                Some(last) if last.high.cmp_frac(&low) == Ordering::Equal => {
                    last.high = high;
                    last.high_axis = high_axis;
                }
                _ => intervals.push(ExpectedInterval {
                    low,
                    high,
                    low_axis,
                    high_axis,
                }),
            }
        }
    }
    intervals
}

fn expected_subject_intervals(case: &ClipCase, subject: &[Point2]) -> Vec<Vec<ExpectedInterval>> {
    let x_boundaries = clip_x_boundaries(case);
    let y_boundaries = clip_y_boundaries(case);
    subject
        .windows(2)
        .map(|window| {
            expected_segment_intervals(case, window[0], window[1], &x_boundaries, &y_boundaries)
        })
        .collect()
}

/// One endpoint's band extent in subject parameter space, with its saturation
/// state.
#[derive(Debug, Clone, Copy, PartialEq)]
struct EndpointTerm {
    term: f64,
    /// `true` when the uncapped term reached one whole subject segment.
    saturated: bool,
}

/// Parameter-space extent of the clip band at one expected-interval endpoint.
///
/// The reported coordinate is the integer rounding of the exact subject/clip
/// intersection on the modelled edge, so it lies within `band` of that edge in
/// any direction. Two extreme displacements bound the subject-parameter shift:
///
/// * **perpendicular to the edge**: the subject's normal coordinate moves at
///   `|Δnormal|` per unit parameter, so a `band`-sized perpendicular
///   displacement is `band / |Δnormal|` parameter units;
/// * **along the edge**: a boundary point `L` along the edge from the exact
///   crossing projects `L·|Δparallel|/length²` further along the subject while
///   sitting `L·|Δnormal|/length` from it, so `L ≤ band·length/|Δnormal|`
///   gives a shift of `(band/|Δnormal|)·(|Δparallel|/length) =
///   (band/|Δnormal|)·cos θ`.
///
/// The perpendicular extreme dominates (`cos θ <= 1`), so the endpoint term is
/// `band / |Δnormal|` at a boundary crossing — `|Δx|` on a vertical edge,
/// `|Δy|` on a horizontal one — and `band / length` at a subject endpoint. For
/// a perpendicular crossing this reduces to the uniform `band / length`; for a
/// tangential crossing it grows with `1/sin θ` until it reaches **1.0**, at
/// which point the band already accounts for the whole segment's extent along
/// the edge normal and the endpoint carries no discriminating information. The
/// term saturates there (one full segment) and reports `saturated` so the
/// caller can count it and drop the interval's coverage requirement instead of
/// silently accepting it; containment may still use the full-segment window,
/// which is exactly what the band permits. A subject endpoint (no clip
/// boundary) saturates only when the segment is at most one band unit long.
fn endpoint_parameter_term(
    axis: Option<BoundaryAxis>,
    delta_x: f64,
    delta_y: f64,
    length: f64,
) -> EndpointTerm {
    let scale = match axis {
        Some(BoundaryAxis::Vertical) => delta_x.max(1.0),
        Some(BoundaryAxis::Horizontal) => delta_y.max(1.0),
        None => length.max(1.0),
    };
    // The derivation above assumes the reported point lies ON the subject. It
    // does not: the cut is an integer point, up to `SUBJECT_PROJECTION_UNITS`
    // (the same bound `project_point` uses to attribute it to a segment) off the
    // subject line. The oracle recovers a parameter by projecting that point onto
    // the subject, so the off-subject displacement shifts the recovered
    // parameter by at most `SUBJECT_PROJECTION_UNITS / length`. A subject
    // endpoint (`None`) is an exact subject vertex and carries no such shift.
    let projection = match axis {
        Some(_) => SUBJECT_PROJECTION_UNITS / length.max(1.0),
        None => 0.0,
    };
    let uncapped = CLIP_BAND_UNITS / scale + projection;
    if uncapped >= 1.0 {
        EndpointTerm {
            term: 1.0,
            saturated: true,
        }
    } else {
        EndpointTerm {
            term: uncapped,
            saturated: false,
        }
    }
}

fn merge_intervals(mut intervals: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    intervals.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(Ordering::Equal));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (low, high) in intervals {
        if high <= low {
            continue;
        }
        match merged.last_mut() {
            Some(last) if low <= last.1 + 1e-9 => last.1 = last.1.max(high),
            _ => merged.push((low, high)),
        }
    }
    merged
}

fn dedupe_points(points: &[Point2]) -> Vec<Point2> {
    let mut out: Vec<Point2> = Vec::with_capacity(points.len());
    for &point in points {
        if out.last() != Some(&point) {
            out.push(point);
        }
    }
    out
}

fn segment_length(start: Point2, end: Point2) -> f64 {
    let dx = (end.x - start.x) as f64;
    let dy = (end.y - start.y) as f64;
    dx.hypot(dy)
}

fn nearest_subject_distance(subject: &[Point2], x: f64, y: f64) -> (usize, f64) {
    let mut best = (0_usize, f64::INFINITY);
    for index in 0..subject.len().saturating_sub(1) {
        let start = subject[index];
        let end = subject[index + 1];
        let dx = (end.x - start.x) as f64;
        let dy = (end.y - start.y) as f64;
        let length_squared = dx * dx + dy * dy;
        if length_squared <= 0.0 {
            continue;
        }
        let t = (((x - start.x as f64) * dx + (y - start.y as f64) * dy) / length_squared)
            .clamp(0.0, 1.0);
        let distance = (x - (start.x as f64 + dx * t)).hypot(y - (start.y as f64 + dy * t));
        if distance < best.1 {
            best = (index, distance);
        }
    }
    best
}

/// Attributing a returned point to a subject segment: the first segment within
/// the band is not necessarily the nearest one.
///
/// A clip cut point sits on the clip boundary. When the subject crosses that
/// boundary a couple of units past one of its own vertices, the cut is within
/// the band of the *incoming* segment (whose clamped endpoint is the vertex)
/// while lying on the *outgoing* segment. Taking the first hit attributes the
/// cut to the vertex and reports a fabricated span start at that vertex.
/// Instead, keep scanning a short forward window after the first hit and take
/// the minimum-distance segment. Waves are simple polylines, so the true
/// segment is adjacent to the hit; the window bounds the cost and the
/// tie-break keeps the earliest parameter.
const PROJECTION_WINDOW: usize = 4;

fn project_point(subject: &[Point2], from: usize, x: f64, y: f64, band: f64) -> Option<(f64, f64)> {
    if subject.len() < 2 {
        return None;
    }
    let last = subject.len() - 1;
    let mut hit: Option<usize> = None;
    let mut best: Option<(f64, f64)> = None;
    for index in from.min(last - 1)..last {
        let start = subject[index];
        let end = subject[index + 1];
        let dx = (end.x - start.x) as f64;
        let dy = (end.y - start.y) as f64;
        let length_squared = dx * dx + dy * dy;
        if length_squared <= 0.0 {
            continue;
        }
        let t = (((x - start.x as f64) * dx + (y - start.y as f64) * dy) / length_squared)
            .clamp(0.0, 1.0);
        let closest_x = start.x as f64 + dx * t;
        let closest_y = start.y as f64 + dy * t;
        let distance = (x - closest_x).hypot(y - closest_y);
        match hit {
            None => {
                if distance <= band {
                    hit = Some(index);
                    best = Some((index as f64 + t, distance));
                }
            }
            Some(first) => {
                if let Some((_, best_distance)) = best {
                    if distance < best_distance {
                        best = Some((index as f64 + t, distance));
                    }
                }
                if index + 1 >= first + PROJECTION_WINDOW {
                    break;
                }
            }
        }
    }
    best
}

fn point_at_s(subject: &[Point2], s: f64) -> (f64, f64) {
    let last = subject.len() - 1;
    let index = (s.floor().max(0.0) as usize).min(last - 1);
    let t = (s - index as f64).clamp(0.0, 1.0);
    let start = subject[index];
    let end = subject[index + 1];
    (
        start.x as f64 + (end.x - start.x) as f64 * t,
        start.y as f64 + (end.y - start.y) as f64 * t,
    )
}

fn span_length(subject: &[Point2], s0: f64, s1: f64) -> f64 {
    if s1 <= s0 {
        return 0.0;
    }
    let last = subject.len() - 1;
    let mut total = 0.0;
    let mut s = s0.max(0.0);
    while s < s1 {
        let index = s.floor() as usize;
        if index >= last {
            break;
        }
        let segment_end = (index + 1) as f64;
        let high = s1.min(segment_end);
        total += (high - s) * segment_length(subject[index], subject[index + 1]);
        s = high;
    }
    total
}

fn in_any_hole(case: &ClipCase, x: f64, y: f64) -> bool {
    case.parts.iter().any(|part| {
        part.holes.iter().any(|hole| {
            x > hole.min_x as f64
                && x < hole.max_x as f64
                && y > hole.min_y as f64
                && y < hole.max_y as f64
        })
    })
}

fn boundary_distance_units(case: &ClipCase, x: f64, y: f64) -> f64 {
    let mut best = f64::INFINITY;
    for part in &case.parts {
        for rect in std::iter::once(part.outer).chain(part.holes.iter().copied()) {
            best = best
                .min((x - rect.min_x as f64).abs())
                .min((x - rect.max_x as f64).abs())
                .min((y - rect.min_y as f64).abs())
                .min((y - rect.max_y as f64).abs());
        }
    }
    best
}

/// Total length of `left` not covered by `right` grown by `delta`.
fn uncovered_length(left: &[(f64, f64)], right: &[(f64, f64)], delta: f64) -> f64 {
    let mut total = 0.0;
    for &(low, high) in left {
        if high <= low {
            continue;
        }
        let mut position = low;
        for &(right_low, right_high) in right {
            let grown_low = right_low - delta;
            let grown_high = right_high + delta;
            if grown_high <= position {
                continue;
            }
            if grown_low >= high {
                break;
            }
            if grown_low > position {
                total += grown_low.min(high) - position;
                position = grown_low.min(high);
            }
            position = position.max(grown_high.min(high));
            if position >= high {
                break;
            }
        }
        if position < high {
            total += high - position;
        }
    }
    total
}

#[derive(Debug, Clone, Copy)]
struct CoveredSpan {
    low: f64,
    high: f64,
}

#[derive(Debug, Default, Clone, Copy)]
struct ClipObservation {
    expected_intervals: usize,
    returned_spans: usize,
    returned_segments: usize,
    retained_subjects: usize,
    interior_observations: usize,
    hole_split_witnesses: usize,
    /// Expected-interval ends whose band term reached one whole segment. Their
    /// interval has no discriminating core and is excluded from coverage.
    saturated_endpoints: usize,
}

/// Map every returned polyline of one subject back onto subject parameter
/// space and require it to follow the subject, not a fabricated chord.
fn map_returned_spans(
    subject: &[Point2],
    spans: &[Vec<Point2>],
    subject_index: usize,
) -> Result<Vec<CoveredSpan>, String> {
    let mut mapped = Vec::with_capacity(spans.len());
    for (span_index, span) in spans.iter().enumerate() {
        if span.len() < 2 {
            return Err(format!(
                "subject {subject_index} span {span_index} has {} points",
                span.len()
            ));
        }
        // `clip_batched` does not promise cross-span order, so each span
        // restarts the subject search; order is only required inside a span.
        let mut cursor = 0_usize;
        let mut previous: Option<f64> = None;
        let mut first = 0.0;
        let mut last = 0.0;
        for (point_index, point) in span.iter().enumerate() {
            let x = point.x as f64;
            let y = point.y as f64;
            let (s, _) =
                project_point(subject, cursor, x, y, SUBJECT_PROJECTION_UNITS).ok_or_else(|| {
                    let (nearest_index, nearest) = nearest_subject_distance(subject, x, y);
                    format!(
                        "subject {subject_index} span {span_index} point {point_index} \
                     ({}, {}) is not on the subject polyline within {SUBJECT_PROJECTION_UNITS} units \
                     (nearest segment {nearest_index} is {nearest:.3} units away)",
                        point.x, point.y
                    )
                })?;
            if let Some(previous_s) = previous {
                if s + 1e-9 < previous_s {
                    return Err(format!(
                        "subject {subject_index} span {span_index} point {point_index} \
                         moved backwards in subject parameter space ({previous_s} -> {s})"
                    ));
                }
                let previous_point = span[point_index - 1];
                let middle = (
                    (previous_point.x as f64 + x) / 2.0,
                    (previous_point.y as f64 + y) / 2.0,
                );
                let search_from = (previous_s.floor().max(0.0) as usize).min(subject.len() - 2);
                let (middle_s, _) = project_point(
                    subject,
                    search_from,
                    middle.0,
                    middle.1,
                    SUBJECT_PROJECTION_UNITS,
                )
                .ok_or_else(|| {
                    format!(
                        "subject {subject_index} span {span_index} point {point_index} \
                                 chord midpoint is not on the subject polyline; a clipped span \
                                 must follow the subject, not shortcut it"
                    )
                })?;
                if middle_s + 1e-9 < previous_s || middle_s > s + 1e-9 {
                    return Err(format!(
                        "subject {subject_index} span {span_index} point {point_index} \
                         chord midpoint parameter {middle_s} leaves [{previous_s}, {s}]"
                    ));
                }
                last = s;
            } else {
                first = s;
                last = s;
            }
            previous = Some(s);
            cursor = (s.floor().max(0.0) as usize).min(subject.len() - 2);
        }
        mapped.push(CoveredSpan {
            low: first.min(last),
            high: first.max(last),
        });
    }
    Ok(mapped)
}

fn covered_ranges_on_segment(covered: &[CoveredSpan], index: usize) -> Vec<(f64, f64)> {
    let base = index as f64;
    let mut ranges = Vec::new();
    for span in covered {
        let low = span.low.max(base);
        let high = span.high.min(base + 1.0);
        if high > low {
            ranges.push((low - base, high - base));
        }
    }
    merge_intervals(ranges)
}

fn check_clip_results(
    case: &ClipCase,
    subjects: &[Vec<Point2>],
    results: &[Vec<Vec<Point2>>],
) -> Result<ClipObservation, String> {
    if results.len() != subjects.len() {
        return Err(format!(
            "clip_batched returned {} subject slots for {} subjects",
            results.len(),
            subjects.len()
        ));
    }
    let mut observation = ClipObservation::default();
    for (subject_index, (subject, spans)) in subjects.iter().zip(results).enumerate() {
        let clean = dedupe_points(subject);
        if clean.len() < 2 {
            if !spans.is_empty() {
                return Err(format!(
                    "subject {subject_index} has fewer than two distinct points but returned \
                     {} spans",
                    spans.len()
                ));
            }
            continue;
        }
        let expected = expected_subject_intervals(case, &clean);
        let mut expected_parameter: Vec<(f64, f64)> = Vec::new();
        for (index, intervals) in expected.iter().enumerate() {
            for interval in intervals {
                expected_parameter.push((
                    index as f64 + interval.low.to_f64(),
                    index as f64 + interval.high.to_f64(),
                ));
            }
        }
        let expected_parameter = merge_intervals(expected_parameter);
        observation.expected_intervals += expected_parameter.len();
        observation.returned_spans += spans.len();
        if !spans.is_empty() {
            observation.retained_subjects += 1;
        }
        observation.returned_segments += spans.iter().map(|span| span.len() - 1).sum::<usize>();

        check_returned_geometry(case, spans)?;
        let covered = map_returned_spans(&clean, spans, subject_index)?;
        // Per-interval retained cores that still carry discriminating power
        // (an endpoint whose band term saturated removes the whole core).
        let mut discriminating: Vec<(usize, f64, f64)> = Vec::new();
        for (index, intervals) in expected.iter().enumerate() {
            let expected_segment: Vec<(f64, f64)> = intervals
                .iter()
                .map(|interval| (interval.low.to_f64(), interval.high.to_f64()))
                .collect();
            let covered_segment = covered_ranges_on_segment(&covered, index);
            let start = clean[index];
            let end = clean[index + 1];
            let delta_x = (end.x - start.x).unsigned_abs() as f64;
            let delta_y = (end.y - start.y).unsigned_abs() as f64;
            let length = segment_length(start, end);
            // Per-endpoint band terms, not a uniform `band / length`: any
            // displacement within the band of the modelled edge moves the
            // subject parameter by at most `band / |normal|`, so a boundary
            // crossing uses that perpendicular worst case (see
            // `endpoint_parameter_term`). Coverage is required only outside the
            // intervals eroded by their own endpoint terms, and returned spans
            // may reach only into the dilated intervals. See the module header
            // derivation.
            let mut eroded: Vec<(f64, f64)> = Vec::new();
            let mut dilated: Vec<(f64, f64)> = Vec::new();
            for interval in intervals {
                let low = interval.low.to_f64();
                let high = interval.high.to_f64();
                let low_term = endpoint_parameter_term(interval.low_axis, delta_x, delta_y, length);
                let high_term =
                    endpoint_parameter_term(interval.high_axis, delta_x, delta_y, length);
                for term in [low_term, high_term] {
                    if term.saturated {
                        observation.saturated_endpoints += 1;
                    }
                }
                let core_low = low + low_term.term;
                let core_high = high - high_term.term;
                if core_high > core_low {
                    eroded.push((core_low, core_high));
                    discriminating.push((index, core_low, core_high));
                }
                // A saturated end has no discriminating core: the band alone
                // covers the segment's extent there, so the interval is
                // excluded from coverage rather than silently accepted. The
                // dilated window may still reach the whole segment, which is
                // exactly what the band permits.
                dilated.push((
                    (low - low_term.term).max(0.0),
                    (high + high_term.term).min(1.0),
                ));
            }
            // A kept interval of a NEIGHBOURING segment that begins (ends) at the
            // shared vertex pulls the band window across that vertex: the cut
            // sits in this segment's tail (head), up to this segment's own band
            // term before (after) the vertex. Without this, a subject that only
            // touches a clip edge at a vertex has an empty expected set here and
            // no window at all for a cut the integer rounding legitimately puts
            // a few units earlier.
            if let Some(next) = expected.get(index + 1) {
                for interval in next
                    .iter()
                    .filter(|interval| interval.low.cmp_frac(&Frac::zero()) == Ordering::Equal)
                {
                    let term = endpoint_parameter_term(interval.low_axis, delta_x, delta_y, length);
                    dilated.push(((1.0 - term.term).max(0.0), 1.0));
                }
            }
            if index > 0 {
                for interval in expected[index - 1]
                    .iter()
                    .filter(|interval| interval.high.cmp_frac(&Frac::one()) == Ordering::Equal)
                {
                    let term =
                        endpoint_parameter_term(interval.high_axis, delta_x, delta_y, length);
                    dilated.push((0.0, term.term.min(1.0)));
                }
            }
            let eroded = merge_intervals(eroded);
            let dilated = merge_intervals(dilated);
            let missing = uncovered_length(&eroded, &covered_segment, 1e-9);
            if missing > 1e-9 {
                return Err(format!(
                    "subject {subject_index} segment {index}: {missing:.6} parameter units of \
                     expected retained span are missing (expected {expected_segment:?}, \
                     eroded {eroded:?}, returned {covered_segment:?})"
                ));
            }
            let extra = uncovered_length(&covered_segment, &dilated, 1e-9);
            if extra > 1e-9 {
                return Err(format!(
                    "subject {subject_index} segment {index}: {extra:.6} parameter units of \
                     returned span lie outside the model (expected {expected_segment:?}, \
                     dilated {dilated:?}, returned {covered_segment:?})"
                ));
            }
        }

        // Interior observations: a **discriminating** (non-saturated) retained
        // core long enough to contain a sample at least two clip bands from any
        // boundary, and that sample is actually covered by a returned span.
        // Counting only discriminating cores is what keeps saturation from
        // hiding the case: it removes coverage requirements, never observations.
        for &(index, core_low, core_high) in &discriminating {
            let low = index as f64 + core_low;
            let high = index as f64 + core_high;
            let length = span_length(&clean, low, high);
            if length < 8.0 * CLIP_BAND_UNITS {
                continue;
            }
            let middle = (low + high) / 2.0;
            let (x, y) = point_at_s(&clean, middle);
            if boundary_distance_units(case, x, y) < 2.0 * CLIP_BAND_UNITS {
                continue;
            }
            let local = middle - index as f64;
            let covered_segment = covered_ranges_on_segment(&covered, index);
            if covered_segment
                .iter()
                .any(|&(low, high)| local > low && local < high)
            {
                observation.interior_observations += 1;
            }
        }
        // Hole-crossing split witness: the model removes a long span from the
        // middle of one subject polyline, and that removed span lies inside a
        // hole (not merely in the gap between two components).
        if expected_parameter.len() >= 2 {
            for window in expected_parameter.windows(2) {
                if span_length(&clean, window[0].1, window[1].0) < 4.0 * CLIP_BAND_UNITS {
                    continue;
                }
                let middle = (window[0].1 + window[1].0) / 2.0;
                let (x, y) = point_at_s(&clean, middle);
                if in_any_hole(case, x, y) {
                    observation.hole_split_witnesses += 1;
                    break;
                }
            }
        }
    }
    Ok(observation)
}

fn subjects_from_paths(paths: &[ExtrusionPath3D]) -> Vec<Vec<Point2>> {
    paths
        .iter()
        .map(|path| {
            path.points
                .iter()
                .map(|point| Point2::from_mm(point.x, point.y))
                .collect()
        })
        .collect()
}

/// Exact f64 length of the part of a segment inside an axis-aligned rectangle.
fn segment_length_inside_rect(start: Point2, end: Point2, rect: (f64, f64, f64, f64)) -> f64 {
    let (min_x, min_y, max_x, max_y) = rect;
    let dx = (end.x - start.x) as f64;
    let dy = (end.y - start.y) as f64;
    let length = dx.hypot(dy);
    if length <= 0.0 {
        return 0.0;
    }
    let mut low = 0.0_f64;
    let mut high = 1.0_f64;
    for (direction, offset) in [
        (-dx, start.x as f64 - min_x),
        (dx, max_x - start.x as f64),
        (-dy, start.y as f64 - min_y),
        (dy, max_y - start.y as f64),
    ] {
        if direction == 0.0 {
            if offset < 0.0 {
                return 0.0;
            }
        } else {
            let t = offset / direction;
            if direction < 0.0 {
                low = low.max(t);
            } else {
                high = high.min(t);
            }
        }
    }
    if high > low {
        (high - low) * length
    } else {
        0.0
    }
}

/// Direct geometric checks on every surviving segment: a returned point must
/// sit in an inflated contour, and no returned segment may reach deeper than
/// the proved clip band into a contracted hole. This is independent of the
/// parameter-space comparison and does not rely on vertices alone.
fn check_returned_geometry(case: &ClipCase, spans: &[Vec<Point2>]) -> Result<(), String> {
    // The model's boundary is the prepared (pre-inflated) edge: on these
    // axis-aligned integer fixtures `inflate_paths_64(delta = 1.0)` lands
    // exactly one unit out, so the inflation is applied once here to place that
    // edge on the fixture rect and the band is the integer intersection
    // rounding around it.
    let inflation = CLIP_INFLATION_UNITS as f64;
    let band = CLIP_BAND_UNITS;
    for (span_index, span) in spans.iter().enumerate() {
        for (point_index, point) in span.iter().enumerate() {
            let inside = case.parts.iter().any(|part| {
                let outer = part.outer;
                point.x as f64 >= outer.min_x as f64 - inflation - band
                    && point.x as f64 <= outer.max_x as f64 + inflation + band
                    && point.y as f64 >= outer.min_y as f64 - inflation - band
                    && point.y as f64 <= outer.max_y as f64 + inflation + band
            });
            if !inside {
                return Err(format!(
                    "returned span {span_index} point {point_index} ({}, {}) lies outside \
                     the modelled (pre-inflated) contour by more than the {CLIP_BAND_UNITS}-unit \
                     rounding band",
                    point.x, point.y
                ));
            }
        }
        for window in span.windows(2) {
            for part in &case.parts {
                for hole in &part.holes {
                    let forbidden = (
                        hole.min_x as f64 + inflation + band,
                        hole.min_y as f64 + inflation + band,
                        hole.max_x as f64 - inflation - band,
                        hole.max_y as f64 - inflation - band,
                    );
                    let inside = segment_length_inside_rect(window[0], window[1], forbidden);
                    if inside > 1e-6 {
                        return Err(format!(
                            "returned segment ({}, {})-({}, {}) runs {inside:.3} units inside \
                             the modelled hole boundary beyond the {CLIP_BAND_UNITS}-unit \
                             rounding band",
                            window[0].x, window[0].y, window[1].x, window[1].y
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// The shared clip composition used by the property, the control and the
/// shrink probe: real raw waves, the linker's own batch clip seam
/// (`link_paths_without_offset`'s `PreparedPolylineClip::clip_batched`), and
/// the independent model. This is the primitive emitter/clip composition, not a
/// claim of end-to-end linker dispatch; `M-LINKER-CONNECT`/`M-LINKER-CHAIN`
/// own the orchestration seam.
fn run_clip_observation(
    case: &ClipCase,
    limits: &ClipLimits,
) -> Result<(RawStats, ClipObservation), String> {
    validate_clip_case(case, limits)?;
    let paths = case.raw_paths();
    let raw = check_raw_paths(case, &paths, limits)?;
    let subjects = subjects_from_paths(&paths);
    let boundary = boundary_expolygons(case);
    let results = PreparedPolylineClip::new(&boundary).clip_batched(&subjects);
    let observation = check_clip_results(case, &subjects, &results)?;
    Ok((raw, observation))
}

/// Replays one shrunk `ClipSample` through exactly the path the property uses
/// (`build_clip_case` → `run_clip_observation`), so a seed-dependent oracle
/// failure becomes a deterministic regression.
fn replay_clip_sample(sample: &ClipSample) -> Result<(), String> {
    let limits = checked_clip_limits(&clip_probe_bounds()).expect("M-GYROID-CLIP probe bounds");
    let case = build_clip_case(&limits, sample).map_err(|error| format!("{error}"))?;
    run_clip_observation(&case, &limits).map(|_| ())
}

/// Seed 30 (hole): subject segment 21 runs `(1769,-805)→(1785,-4)`, almost
/// parallel to the hole's right edge (modelled at x = 1785) and touching it only
/// at the vertex where segment 22 begins. The cut is returned 23 units earlier,
/// 0.46 units from the edge, inside the band. The oracle used to give that tail
/// an empty window (`expected [] dilated []`).
#[test]
fn clip_oracle_allows_band_cut_before_a_grazing_vertex_contact_hole() {
    let sample = ClipSample {
        topology: ClipTopology::HoledRect,
        angle: ClipAngle::Rotated,
        density_ppm: 913_898,
        width_units: 1_992,
        size_x_extra: 0,
        size_y_extra: 0,
        hole_x_multiple: 3,
        hole_y_multiple: 2,
        offset_x: -1,
        offset_y: 0,
        phase_quarter: 3,
        phase_within: 0.447_899_144_262_894_3,
        phase_negative: true,
        speed_milli: 250,
    };
    replay_clip_sample(&sample).expect("grazing vertex contact within the band must be accepted");
}

/// Seed 40 (plain rectangle): segment 34 `(9441,3514)→(9386,5248)` ends exactly
/// on the modelled right edge x = 9386; the cut is returned 2 units earlier.
#[test]
fn clip_oracle_allows_band_cut_before_a_grazing_vertex_contact_plain() {
    let sample = ClipSample {
        topology: ClipTopology::PlainRect,
        angle: ClipAngle::Rotated,
        density_ppm: 774_669,
        width_units: 3_547,
        size_x_extra: 1,
        size_y_extra: 0,
        hole_x_multiple: 2,
        hole_y_multiple: 3,
        offset_x: 0,
        offset_y: 0,
        phase_quarter: 3,
        phase_within: 0.345_561_874_740_964_7,
        phase_negative: false,
        speed_milli: 250,
    };
    replay_clip_sample(&sample).expect("grazing vertex contact within the band must be accepted");
}

/// Seed 60: segment 27 `(-940,4513)→(893,4142)` crosses the modelled hole edge
/// x = -1. The integer cut `(0,4322)` is 0.8 units off the subject; projecting
/// it back shifts the recovered parameter by 0.000079 more than `band /
/// |Δnormal|` allowed (the off-subject displacement, bounded by
/// `SUBJECT_PROJECTION_UNITS / length`, was not part of the term).
#[test]
fn clip_oracle_counts_off_subject_projection_at_a_boundary_crossing() {
    let sample = ClipSample {
        topology: ClipTopology::HoledRect,
        angle: ClipAngle::Rotated,
        density_ppm: 911_190,
        width_units: 5_017,
        size_x_extra: 2,
        size_y_extra: 0,
        hole_x_multiple: 3,
        hole_y_multiple: 2,
        offset_x: -3,
        offset_y: 0,
        phase_quarter: 5,
        phase_within: 0.795_626_958_549_58,
        phase_negative: false,
        speed_milli: 250,
    };
    replay_clip_sample(&sample)
        .expect("off-subject cut projection must be within the endpoint term");
}

// ─────────────────────────────────────────────────────────────────────────────
// M-GYROID-PHASE property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_gyroid_phase_and_grid() {
    let run = PropertyRun::new(PHASE_ID, PHASE_TEST).expect("M-GYROID-PHASE registry context");
    let limits = live_phase_limits(&run);
    let strategy = phase_strategy(&limits, &PHASE_BANDS, None);
    let calls = Cell::new(0_u64);
    let vertical_cases = Cell::new(0_u64);
    let horizontal_cases = Cell::new(0_u64);
    let positive_phases = Cell::new(0_u64);
    let negative_phases = Cell::new(0_u64);
    let flip_cases = Cell::new(0_u64);
    let unflipped_cases = Cell::new(0_u64);
    let periodic_cases = Cell::new(0_u64);
    let grid_multiple_cases = Cell::new(0_u64);
    let negative_non_multiple_cases = Cell::new(0_u64);
    let translated_cases = Cell::new(0_u64);
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let phase = match check_phase_case(&limits, &case) {
            Ok(observation) => observation,
            Err(message) => return fail(message),
        };
        if phase.vertical {
            vertical_cases.set(vertical_cases.get() + 1);
        } else {
            horizontal_cases.set(horizontal_cases.get() + 1);
        }
        if phase.negative_phase {
            negative_phases.set(negative_phases.get() + 1);
        } else {
            positive_phases.set(positive_phases.get() + 1);
        }
        if phase.flip {
            flip_cases.set(flip_cases.get() + 1);
        } else {
            unflipped_cases.set(unflipped_cases.get() + 1);
        }
        if phase.periodic {
            periodic_cases.set(periodic_cases.get() + 1);
        }
        let grid = match check_grid_case(&limits, &case.grid) {
            Ok(observation) => observation,
            Err(message) => return fail(message),
        };
        if grid.grid_multiple {
            grid_multiple_cases.set(grid_multiple_cases.get() + 1);
        }
        if grid.negative_non_multiple {
            negative_non_multiple_cases.set(negative_non_multiple_cases.get() + 1);
        }
        if grid.translated {
            translated_cases.set(translated_cases.get() + 1);
        }
        Ok(())
    })
    .expect("M-GYROID-PHASE branch algebra and floor semantics");
    assert!(
        vertical_cases.get() > 0,
        "no vertical orientation was sampled"
    );
    assert!(
        horizontal_cases.get() > 0,
        "no horizontal orientation was sampled"
    );
    assert!(positive_phases.get() > 0, "no positive phase was sampled");
    assert!(negative_phases.get() > 0, "no negative phase was sampled");
    assert!(flip_cases.get() > 0, "no flipped branch was sampled");
    assert!(unflipped_cases.get() > 0, "no unflipped branch was sampled");
    assert!(
        periodic_cases.get() > 0,
        "no phase case left the periodic guard satisfied, so the periodic envelope was never \
         exercised"
    );
    assert!(
        grid_multiple_cases.get() > 0,
        "no exact grid multiple was sampled"
    );
    assert!(
        negative_non_multiple_cases.get() > 0,
        "no negative non-multiple value was sampled, so floor-versus-truncation was never \
         distinguished"
    );
    assert!(
        translated_cases.get() > 0,
        "no whole-period translation was sampled"
    );
    println!(
        "M-GYROID-PHASE accepted_calls={} vertical={} horizontal={} positive_phase={} \
         negative_phase={} flip={} unflipped={} periodic={} grid_multiples={} \
         negative_non_multiples={} translated={}",
        calls.get(),
        vertical_cases.get(),
        horizontal_cases.get(),
        positive_phases.get(),
        negative_phases.get(),
        flip_cases.get(),
        unflipped_cases.get(),
        periodic_cases.get(),
        grid_multiple_cases.get(),
        negative_non_multiple_cases.get(),
        translated_cases.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-GYROID-CLIP property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_gyroid_raw_wave_clipping() {
    let run = PropertyRun::new(CLIP_ID, CLIP_TEST).expect("M-GYROID-CLIP registry context");
    let limits = live_clip_limits(&run);
    let strategy = clip_strategy(
        &limits,
        &CLIP_TOPOLOGIES,
        &CLIP_ANGLES,
        (limits.min_density_ppm, 1_000_000),
        (limits.width_floor, limits.max_width_units),
    );
    let calls = Cell::new(0_u64);
    let raw_paths = Cell::new(0_u64);
    let raw_segments = Cell::new(0_u64);
    let raw_points = Cell::new(0_u64);
    let outside_points = Cell::new(0_u64);
    let expected_intervals = Cell::new(0_u64);
    let returned_segments = Cell::new(0_u64);
    let retained_subjects = Cell::new(0_u64);
    let interior_observations = Cell::new(0_u64);
    let saturated_endpoints = Cell::new(0_u64);
    let hole_splits = Cell::new(0_u64);
    let holed_cases = Cell::new(0_u64);
    let rotated_cases = Cell::new(0_u64);
    let dense_cases = Cell::new(0_u64);
    run.check(&strategy, |sample, _guard| {
        calls.set(calls.get() + 1);
        let case = match build_clip_case(&limits, &sample) {
            Ok(case) => case,
            Err(error) => return fail(format!("clip case construction: {error}")),
        };
        let (raw, observation) = match run_clip_observation(&case, &limits) {
            Ok(result) => result,
            Err(message) => return fail(message),
        };
        if raw.paths == 0 {
            return fail(format!(
                "{} case emitted no raw wave path for a valid {}×{} clip",
                case.topology.label(),
                case.parts[0].outer.width(),
                case.parts[0].outer.height()
            ));
        }
        raw_paths.set(raw_paths.get() + raw.paths as u64);
        raw_segments.set(raw_segments.get() + raw.segments as u64);
        raw_points.set(raw_points.get() + raw.points as u64);
        outside_points.set(outside_points.get() + raw.outside_points as u64);
        expected_intervals.set(expected_intervals.get() + observation.expected_intervals as u64);
        returned_segments.set(returned_segments.get() + observation.returned_segments as u64);
        retained_subjects.set(retained_subjects.get() + observation.retained_subjects as u64);
        interior_observations
            .set(interior_observations.get() + observation.interior_observations as u64);
        saturated_endpoints.set(saturated_endpoints.get() + observation.saturated_endpoints as u64);
        hole_splits.set(hole_splits.get() + observation.hole_split_witnesses as u64);
        if case.topology.holes() > 0 {
            holed_cases.set(holed_cases.get() + 1);
        }
        if case.angle == ClipAngle::Rotated {
            rotated_cases.set(rotated_cases.get() + 1);
        }
        if case.class == DensityClass::High {
            dense_cases.set(dense_cases.get() + 1);
        }
        Ok(())
    })
    .expect("M-GYROID-CLIP raw waves and prepared clip");
    assert!(raw_paths.get() > 0, "no raw wave path was generated");
    assert!(raw_segments.get() > 0, "no raw wave segment was inspected");
    assert!(
        expected_intervals.get() > 0,
        "the model never expected retained geometry"
    );
    assert!(
        retained_subjects.get() > 0,
        "the clip retained no subject span in any case"
    );
    assert!(
        interior_observations.get() > 0,
        "no discriminating retained span had a genuinely interior observed sample \
         (saturated_endpoints={})",
        saturated_endpoints.get()
    );
    assert!(
        hole_splits.get() > 0,
        "no wave was split by a hole, so the split witness was never exercised"
    );
    assert!(holed_cases.get() > 0, "no holed clip case was sampled");
    assert!(
        rotated_cases.get() > 0,
        "no rotated generation case was sampled"
    );
    assert!(
        dense_cases.get() > 0,
        "no high-density generation case was sampled"
    );
    println!(
        "M-GYROID-CLIP accepted_calls={} raw_paths={} raw_segments={} raw_points={} \
         outside_points={} expected_intervals={} returned_segments={} retained_subjects={} \
         interior_observations={} saturated_endpoints={} hole_splits={} holed_cases={} \
         rotated_cases={} dense_cases={}",
        calls.get(),
        raw_paths.get(),
        raw_segments.get(),
        raw_points.get(),
        outside_points.get(),
        expected_intervals.get(),
        returned_segments.get(),
        retained_subjects.get(),
        interior_observations.get(),
        saturated_endpoints.get(),
        hole_splits.get(),
        holed_cases.get(),
        rotated_cases.get(),
        dense_cases.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic controls
// ─────────────────────────────────────────────────────────────────────────────

fn witness_clip_case() -> ClipCase {
    let spacing = 8_197_i64;
    ClipCase {
        parts: vec![ClipPart {
            outer: Rect {
                min_x: -50_000,
                min_y: -50_000,
                max_x: 50_000,
                max_y: 50_000,
            },
            holes: vec![Rect {
                min_x: -20_000,
                min_y: -20_000,
                max_x: 20_000,
                max_y: 20_000,
            }],
        }],
        generation: Rect {
            min_x: -50_000,
            min_y: -50_000,
            max_x: 50_000,
            max_y: 50_000,
        },
        density: 0.2,
        spacing_width_mm: 0.4,
        stamped_width_mm: 0.45,
        speed_factor: 1.2,
        z: 0.2,
        z_phase: 0.2 / (0.4 / (0.2 * 2.44)),
        base_angle: 45.0,
        topology: ClipTopology::HoledRect,
        angle: ClipAngle::Axis,
        class: DensityClass::Low,
        spacing_units: spacing,
    }
}

fn plane_rect_clip_case() -> ClipCase {
    let mut case = witness_clip_case();
    case.parts = vec![ClipPart {
        outer: Rect {
            min_x: -50_000,
            min_y: -50_000,
            max_x: 50_000,
            max_y: 50_000,
        },
        holes: Vec::new(),
    }];
    case.topology = ClipTopology::PlainRect;
    case
}

fn truncating_grid_model(grid_units: i64, val_units: i64) -> f64 {
    // The deliberate mutant the charter names: truncate the quotient toward
    // zero the way a `%`-based remainder would, instead of flooring it.
    let quotient = (val_units as f64 / grid_units as f64) as i64;
    (quotient * grid_units) as f64
}

// test-quality: mutation control — every deliberate bad observation (out-of-range phase,
// truncating grid model, conflated width, all-empty/dropped/shortcut clip) is compared with the
// same checker the properties use; the positive witnesses are independent model observations.
#[test]
fn gyroid_controls() {
    // (a) Both orientation branches, both flip branches, positive and negative
    // phases, and explicit near-singular inputs.
    let mut seen = (false, false, false, false);
    for phase in [-2.6_f64, -1.2, -0.1, 0.1, 1.2, 2.6] {
        let z_sin = phase.sin();
        let z_cos = phase.cos();
        let vertical = expected_vertical(z_sin, z_cos);
        for x in [-2.0_f64, -0.5, 0.0, 0.5, 2.0] {
            for flip in [false, true] {
                let value = gyroid_f(x, z_sin, z_cos, vertical, flip);
                check_phase_value(x, z_sin, z_cos, vertical, flip, value, "control")
                    .expect("both-phase control witness");
                seen.0 |= vertical && !flip;
                seen.1 |= vertical && flip;
                seen.2 |= !vertical && !flip;
                seen.3 |= !vertical && flip;
            }
        }
    }
    assert_eq!(seen, (true, true, true, true), "control branch matrix");

    // The singular early return is unreachable from a consistent phase: the
    // caller's orientation rule guarantees `|unused component| >= 1/√2`. These
    // are deliberate inconsistent (z_sin, z_cos) pairs, which is exactly why
    // they are explicit controls rather than generated cases.
    let singular_vertical = gyroid_f(0.0, 1.0, 0.0, true, false);
    assert!(
        (singular_vertical - PI).abs() <= PHASE_RANGE_SLACK,
        "vertical r<1e-15 control returned {singular_vertical}"
    );
    let singular_horizontal = gyroid_f(FRAC_PI_2, 0.0, 1.0, false, false);
    assert!(
        (singular_horizontal - FRAC_PI_2).abs() <= PHASE_RANGE_SLACK,
        "horizontal r<1e-15 control returned {singular_horizontal}"
    );
    let near_singular = gyroid_f(0.0, 1.0, 1e-9, true, false);
    check_phase_value(0.0, 1.0, 1e-9, true, false, near_singular, "near-singular")
        .expect("near-singular control");
    // Exact math: `asin(sin(π)/r) + asin(-1) + π`, with `sin(π) = 1.22e-16` and
    // `r = 1e-9`, so the first term is `asin(1.22e-7) ≈ 1.22e-7` and the result
    // is `π/2 + 1.22e-7`, not exactly `π/2`.
    assert!(
        (near_singular - FRAC_PI_2).abs() <= 1e-6,
        "near-singular control should sit at the clamped asin limit, got {near_singular}"
    );
    let near_singular_flipped = gyroid_f(0.0, 1.0, 1e-9, true, true);
    check_phase_value(
        0.0,
        1.0,
        1e-9,
        true,
        true,
        near_singular_flipped,
        "near-singular flipped",
    )
    .expect("near-singular flipped control");

    // The same checker must reject fabricated out-of-range and NaN results.
    assert!(
        check_phase_value(0.0, 0.0, 1.0, true, false, 3.0 * PI, "mutant").is_err(),
        "an out-of-range vertical result was accepted"
    );
    assert!(
        check_phase_value(0.0, 0.0, 1.0, false, false, f64::NAN, "mutant").is_err(),
        "a NaN result was accepted"
    );

    // (b) Exact dyadic floor witnesses on both sides of zero, plus the
    // truncation-toward-zero mutant rejected by the same checker.
    let limits = phase_probe_limits();
    for (grid, value, expected) in [
        (2_i64, -3_i64, -4_i64),
        (2, -2, -2),
        (2, 3, 2),
        (4, -9, -12),
        (4, -8, -8),
        (4, 7, 4),
        (1, -3, -3),
    ] {
        let observed = align_to_grid(value as f64, grid as f64);
        assert_eq!(
            observed, expected as f64,
            "align_to_grid({value}, {grid}) must floor to {expected}"
        );
        check_grid_value(grid, value, observed, "dyadic control").expect("dyadic grid control");
        let case = GridCase {
            grid_units: grid,
            val_units: value,
            shift: 0,
        };
        check_grid_case(&limits, &case).expect("dyadic grid case");
    }
    let mutant = truncating_grid_model(2, -3);
    assert_ne!(
        mutant,
        grid_expectation(2, -3) as f64,
        "the truncation mutant must differ from the floor model on -3 / 2"
    );
    assert!(
        check_grid_value(2, -3, mutant, "truncation mutant").is_err(),
        "the grid checker accepted the truncation-toward-zero mutant"
    );
    // Adjacent negative values around a grid multiple stay distinct.
    assert_eq!(align_to_grid(-3.0, 2.0), -4.0);
    assert_eq!(align_to_grid(-2.0, 2.0), -2.0);
    assert_eq!(align_to_grid(-1.0, 2.0), -2.0);

    // (c) Deterministic nonempty raw-wave witness with full metadata.
    let plain = plane_rect_clip_case();
    let paths = plain.raw_paths();
    let raw = check_raw_paths(&plain, &paths, &clip_probe_limits()).expect("raw wave witness");
    assert!(raw.paths > 0, "the raw witness emitted no path");
    assert!(raw.segments > 0, "the raw witness emitted no segment");
    assert!(
        raw.outside_points > 0,
        "the raw witness emitted nothing outside the outline, so it does not demonstrate \
         unclipped emission"
    );
    println!(
        "raw witness: paths={} segments={} points={} outside={} stamped={} spacing={}",
        raw.paths,
        raw.segments,
        raw.points,
        raw.outside_points,
        plain.stamped_width_mm,
        plain.spacing_width_mm
    );

    // The same metadata check must reject a width conflated with the spacing
    // argument and a non-finite coordinate.
    let mut wrong_width = paths.clone();
    wrong_width[0].points[0].width = plain.spacing_width_mm;
    assert!(
        check_raw_paths(&plain, &wrong_width, &clip_probe_limits()).is_err(),
        "the metadata check accepted the per-call spacing width as the stamped width"
    );
    let mut non_finite = paths;
    non_finite[0].points[0].x = f32::INFINITY;
    assert!(
        check_raw_paths(&plain, &non_finite, &clip_probe_limits()).is_err(),
        "the metadata check accepted a non-finite coordinate"
    );

    // (d) The full clip check on a real clipped witness.
    let holed = witness_clip_case();
    let limits = clip_probe_limits();
    let (raw, observation) = run_clip_observation(&holed, &limits).expect("real clipped witness");
    assert!(raw.paths > 0);
    assert!(observation.expected_intervals > 0, "model expected nothing");
    assert!(
        observation.retained_subjects > 0,
        "the real clip retained nothing"
    );
    assert!(
        observation.interior_observations > 0,
        "the real clip produced no interior observation"
    );
    assert!(
        observation.hole_split_witnesses > 0,
        "the real clip produced no hole-crossing split"
    );
    println!(
        "clip witness: expected_intervals={} returned_spans={} retained={} \
         interior={} hole_splits={}",
        observation.expected_intervals,
        observation.returned_spans,
        observation.retained_subjects,
        observation.interior_observations,
        observation.hole_split_witnesses
    );

    // (e) Mutations rejected by the same checker.
    let subjects = subjects_from_paths(&holed.raw_paths());
    let boundary = boundary_expolygons(&holed);
    let results = PreparedPolylineClip::new(&boundary).clip_batched(&subjects);
    check_clip_results(&holed, &subjects, &results).expect("unmutated witness");
    let total_spans: usize = results.iter().map(Vec::len).sum();
    assert!(total_spans > 0, "the witness produced no clipped span");

    let empty = vec![Vec::new(); results.len()];
    let error = check_clip_results(&holed, &subjects, &empty)
        .expect_err("an all-empty clip must be rejected despite expected retained geometry");
    assert!(
        error.contains("missing"),
        "the all-empty clip was rejected by something other than the coverage model: {error}"
    );

    let mut omitted = results.clone();
    let mut removed: Option<(usize, usize, f64)> = None;
    for (subject_index, spans) in results.iter().enumerate() {
        if spans.is_empty() {
            continue;
        }
        let clean = dedupe_points(&subjects[subject_index]);
        if clean.len() < 2 {
            continue;
        }
        let mapped = map_returned_spans(&clean, spans, subject_index)
            .expect("witness span mapping before mutation");
        for (span_index, span) in mapped.iter().enumerate() {
            let length = span_length(&clean, span.low, span.high);
            if removed.is_none_or(|(_, _, best)| length > best) {
                removed = Some((subject_index, span_index, length));
            }
        }
    }
    let (subject_index, span_index, removed_length) =
        removed.expect("a long interior retained span");
    assert!(
        removed_length >= 8.0 * CLIP_BAND_UNITS,
        "the omitted-span mutation needs a retained span longer than eight clip bands, got \
         {removed_length}"
    );
    omitted[subject_index].remove(span_index);
    let error = check_clip_results(&holed, &subjects, &omitted)
        .expect_err("an omitted interior span must be rejected");
    assert!(
        error.contains("missing"),
        "the omitted span was rejected by something other than the coverage model: {error}"
    );

    // Shortcut: join two real retained points on opposite sides of the hole
    // with one segment. Both endpoints ride the subject, but the segment cuts
    // straight through the hole far beyond the boundary band.
    let mut split = None;
    for (subject_index, spans) in results.iter().enumerate() {
        if spans.len() < 2 {
            continue;
        }
        let clean = dedupe_points(&subjects[subject_index]);
        let expected = expected_subject_intervals(&holed, &clean);
        let mut flat: Vec<(f64, f64)> = Vec::new();
        for (index, intervals) in expected.iter().enumerate() {
            for interval in intervals {
                flat.push((
                    index as f64 + interval.low.to_f64(),
                    index as f64 + interval.high.to_f64(),
                ));
            }
        }
        let flat = merge_intervals(flat);
        if flat.len() >= 2 {
            let first = point_at_s(&clean, (flat[0].0 + flat[0].1) / 2.0);
            let second = point_at_s(&clean, (flat[1].0 + flat[1].1) / 2.0);
            split = Some((
                subject_index,
                Point2 {
                    x: first.0.round() as i64,
                    y: first.1.round() as i64,
                },
                Point2 {
                    x: second.0.round() as i64,
                    y: second.1.round() as i64,
                },
            ));
            break;
        }
    }
    let (subject_index, start, end) = split.expect("a hole-split subject for the shortcut");
    let mut shortcut = results.clone();
    shortcut[subject_index] = vec![vec![start, end]];
    let error = check_clip_results(&holed, &subjects, &shortcut)
        .expect_err("a shortcut through the hole must be rejected");
    assert!(
        error.contains("hole")
            || error.contains("outside the model")
            || error.contains("chord midpoint")
            || error.contains("not on the subject"),
        "the shortcut was rejected for an unexpected reason: {error}"
    );

    // (e2) The derived boundary-normal term is bounded, not a blanket pass.
    // A subject that grazes a vertical outer edge almost tangentially
    // (|Δx| = 20 over a 1200-unit segment) has `band / |Δnormal| = 1 / 20`
    // parameter units, i.e. 0.05 — 60 times the straight-ahead `band / length`
    // term. A cut one unit short of the modelled edge (parameter 0.50) sits
    // exactly on that term and must be accepted; a cut five units short (0.70)
    // is outside it and must still be reported as missing coverage.
    let shallow = ClipCase {
        parts: vec![ClipPart {
            outer: Rect {
                min_x: 0,
                min_y: 0,
                max_x: 1_000,
                max_y: 1_000,
            },
            holes: Vec::new(),
        }],
        generation: Rect {
            min_x: 0,
            min_y: 0,
            max_x: 1_000,
            max_y: 1_000,
        },
        topology: ClipTopology::PlainRect,
        ..holed.clone()
    };
    let shallow_subject = vec![vec![Point2 { x: -10, y: -100 }, Point2 { x: 10, y: 1_100 }]];
    // Exact model endpoints: the inflated outer edges are x = -1 and x = 1001;
    // the subject crosses x = -1 at t = 9/20 = 0.45 and stops at t = 1.
    let accepted_start = Point2 { x: 0, y: 500 };
    let accepted_end = Point2 { x: 8, y: 1_001 };
    let accepted = vec![vec![vec![accepted_start, accepted_end]]];
    let observation = check_clip_results(&shallow, &shallow_subject, &accepted)
        .expect("a cut inside the derived boundary-normal term must be accepted");
    assert_eq!(observation.expected_intervals, 1);
    let rejected_start = Point2 { x: 4, y: 740 };
    let rejected = vec![vec![vec![rejected_start, accepted_end]]];
    let error = check_clip_results(&shallow, &shallow_subject, &rejected)
        .expect_err("a cut beyond the derived boundary-normal term must be rejected");
    assert!(
        error.contains("missing"),
        "the over-short cut must fail the coverage direction, got: {error}"
    );
    // The same shallow geometry bounds the other direction: a cut further
    // outside the edge than the term exceeds the geometric band as well.
    let beyond_start = Point2 { x: -6, y: 140 };
    let beyond = vec![vec![vec![beyond_start, accepted_end]]];
    assert!(
        check_clip_results(&shallow, &shallow_subject, &beyond).is_err(),
        "a cut outside the derived boundary-normal term was accepted"
    );
    println!(
        "boundary-normal term control: shallow |Δx|=20 term=0.05; accepted shortfall 0.05, \
         rejected shortfall 0.25; saturated_endpoints={}",
        observation.saturated_endpoints
    );

    // (e3) Saturation is recorded, not silent. At the modelled 1-unit band a
    // *boundary* term cannot saturate (an integer crossing has |Δnormal| >= 2,
    // so 1/|Δnormal| <= 1/2), but a subject segment at most one band unit long
    // saturates both endpoint terms at 1.0. Such an interval carries no
    // discriminating core, so coverage is deliberately not required and the
    // whole segment lies inside the band window; the counter must record it.
    let tiny_subject = vec![vec![Point2 { x: 500, y: 500 }, Point2 { x: 501, y: 500 }]];
    let tiny_results: Vec<Vec<Vec<Point2>>> = vec![Vec::new()];
    let tiny = check_clip_results(&shallow, &tiny_subject, &tiny_results)
        .expect("a sub-band segment carries no discriminating coverage requirement");
    assert!(
        tiny.saturated_endpoints >= 1,
        "a one-unit segment against a one-unit band must record saturated endpoints, got {}",
        tiny.saturated_endpoints
    );
    println!(
        "saturation control: one-unit segment saturated_endpoints={} \
         interior_observations={}",
        tiny.saturated_endpoints, tiny.interior_observations
    );

    // (f) Legitimate empty modes, observed separately from failure.
    let empty_contour = ExPolygon {
        contour: Polygon { points: Vec::new() },
        holes: Vec::new(),
    };
    let module = holed.module();
    assert!(
        module
            .fill_expolygon(&empty_contour, 0.2, 1.0, 0.2, 0.4)
            .is_empty(),
        "an empty contour must emit no wave"
    );
    for (density, width) in [(0.0_f32, 0.4_f32), (-0.5, 0.4), (0.2, 0.0), (0.2, -0.4)] {
        assert!(
            module
                .fill_expolygon(&plain.generation_expolygon(), 0.2, 1.0, density, width)
                .is_empty(),
            "density {density} / width {width} must emit no wave"
        );
    }
    // A legitimate empty *clipped* result: real raw waves clipped against a
    // boundary that contains none of them. The model expects nothing and the
    // empty per-subject output is accepted.
    let far = ClipCase {
        parts: vec![ClipPart {
            outer: Rect {
                min_x: 400_000,
                min_y: 400_000,
                max_x: 440_000,
                max_y: 440_000,
            },
            holes: Vec::new(),
        }],
        generation: plain.generation,
        topology: ClipTopology::PlainRect,
        ..plain.clone()
    };
    let subjects = subjects_from_paths(&far.raw_paths());
    let boundary = boundary_expolygons(&far);
    let results = PreparedPolylineClip::new(&boundary).clip_batched(&subjects);
    let observation = check_clip_results(&far, &subjects, &results)
        .expect("a legitimate empty clipped result is not a failure");
    assert_eq!(observation.expected_intervals, 0);
    assert_eq!(observation.retained_subjects, 0);
    println!(
        "empty modes: contour/density/width suppressed; legitimate empty clip accepted \
         (expected_intervals=0)"
    );
}

impl ClipCase {
    fn generation_expolygon(&self) -> ExPolygon {
        plain_expolygon(self.generation)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink probes and bound matrices
// ─────────────────────────────────────────────────────────────────────────────

fn assert_no_shrink<S>(runner: &mut TestRunner, strategy: &S, label: &str)
where
    S: Strategy,
    S::Value: PartialEq + std::fmt::Debug,
{
    let mut tree = strategy
        .new_tree(runner)
        .unwrap_or_else(|reason| panic!("{label}: no-shrink tree construction failed: {reason}"));
    let before = tree.current();
    assert!(!tree.simplify(), "{label}: a no-shrink tree simplified");
    assert!(!tree.complicate(), "{label}: a no-shrink tree complicated");
    assert_eq!(tree.current(), before, "{label}: a no-shrink tree changed");
}

fn phase_bits(case: &PhaseCase) -> Vec<u64> {
    let mut bits = vec![
        case.phase.to_bits(),
        case.x.to_bits(),
        u64::from(case.flip),
        case.grid.grid_units as u64,
        case.grid.val_units as u64,
        case.grid.shift as u64,
    ];
    bits.push(case.band.residue() as u64);
    bits
}

fn clip_bits(case: &ClipCase) -> Vec<u64> {
    let mut bits = vec![
        u64::from(case.density.to_bits()),
        u64::from(case.spacing_width_mm.to_bits()),
        u64::from(case.z.to_bits()),
        u64::from(case.speed_factor.to_bits()),
        u64::from(case.base_angle.to_bits()),
    ];
    for part in &case.parts {
        bits.extend([
            part.outer.min_x as u64,
            part.outer.min_y as u64,
            part.outer.max_x as u64,
            part.outer.max_y as u64,
        ]);
        for hole in &part.holes {
            bits.extend([
                hole.min_x as u64,
                hole.min_y as u64,
                hole.max_x as u64,
                hole.max_y as u64,
            ]);
        }
    }
    bits
}

// test-quality: shrink probe — bound-construction rejection plus real simplify/complicate over
// every phase band, density/width class and clip topology, revalidating each exposed input and
// retained witness with the property checkers rather than the shrink bookkeeping.
#[test]
fn gyroid_shrink_bounds() {
    // ── bound-construction rejection matrix ────────────────────────────────
    fn expect_invalid(label: &str, bounds: BTreeMap<String, u64>, phase: bool) {
        let result = if phase {
            checked_phase_limits(&bounds).map(|_| ())
        } else {
            checked_clip_limits(&bounds).map(|_| ())
        };
        match result {
            Err(DomainError::InvalidBounds(message)) => {
                assert!(!message.is_empty(), "{label}: empty InvalidBounds payload")
            }
            other => panic!("{label}: expected InvalidBounds, got {other:?}"),
        }
    }

    let phase_bounds = phase_probe_bounds();
    checked_phase_limits(&phase_bounds).expect("valid phase control bounds");
    for key in PHASE_BOUND_KEYS {
        let mut missing = phase_bounds.clone();
        missing.remove(key);
        expect_invalid(&format!("phase missing `{key}`"), missing, true);
    }
    for (label, key, value) in [
        ("zero phase", "max_phase_milliradians", 0),
        (
            "phase below the band minimum",
            "max_phase_milliradians",
            1_000,
        ),
        (
            "phase above the derived cap",
            "max_phase_milliradians",
            MAX_PHASE_MILLIRADIANS_CAP + 1,
        ),
        ("zero grid", "max_grid_units", 0),
        (
            "grid above the exact-dyadic cap",
            "max_grid_units",
            MAX_GRID_UNITS_CAP + 1,
        ),
    ] {
        let mut bounds = phase_bounds.clone();
        bounds.insert(key.to_owned(), value);
        expect_invalid(label, bounds, true);
    }
    let mut oversized_grid = phase_bounds.clone();
    oversized_grid.insert("max_grid_units".to_owned(), u64::MAX);
    assert!(matches!(
        checked_phase_limits(&oversized_grid),
        Err(DomainError::ArithmeticOverflow)
    ));

    let clip_bounds = clip_probe_bounds();
    checked_clip_limits(&clip_bounds).expect("valid clip control bounds");
    for key in CLIP_BOUND_KEYS {
        let mut missing = clip_bounds.clone();
        missing.remove(key);
        expect_invalid(&format!("clip missing `{key}`"), missing, false);
    }
    for (label, key, value) in [
        ("zero coordinate bound", "max_abs_units", 0),
        ("too few vertices", "max_vertices", 7),
        ("no hole room", "max_holes", 0),
        ("no component room", "max_components", 0),
        ("zero shrink candidates", "max_shrink_candidates", 0),
        (
            "too few shrink leaves",
            "max_shrink_candidates",
            u64::from(CLIP_SHRINK_LEAVES - 1),
        ),
        ("zero width", "max_width_units", 0),
        ("zero density", "min_density_ppm", 0),
        ("density above 100%", "min_density_ppm", 1_000_001),
        ("zero phase", "max_phase_milliradians", 0),
        (
            "phase below the band minimum",
            "max_phase_milliradians",
            MIN_PHASE_MILLIRADIANS - 1,
        ),
        ("no wave points", "max_wave_points", 0),
        (
            "wave points below the derived envelope",
            "max_wave_points",
            1_024,
        ),
        (
            "wave points above the derived cap",
            "max_wave_points",
            MAX_WAVE_POINTS_CAP + 1,
        ),
    ] {
        let mut bounds = clip_bounds.clone();
        bounds.insert(key.to_owned(), value);
        expect_invalid(label, bounds, false);
    }
    // Impossible combinations, not just individual values.
    let mut no_resolution = clip_bounds.clone();
    no_resolution.insert("max_width_units".to_owned(), 400);
    expect_invalid(
        "width below the clip-band resolution floor",
        no_resolution,
        false,
    );
    let mut no_room = clip_bounds.clone();
    no_room.insert("min_density_ppm".to_owned(), 1_000);
    expect_invalid(
        "coordinate bound cannot hold the largest case",
        no_room,
        false,
    );
    let mut no_band_room = clip_bounds.clone();
    no_band_room.insert("max_phase_milliradians".to_owned(), 2_000);
    expect_invalid("phase cannot reach every band", no_band_room, false);
    let mut oversized_abs = clip_bounds.clone();
    oversized_abs.insert("max_abs_units".to_owned(), i64::MAX as u64 + 1);
    assert!(matches!(
        checked_clip_limits(&oversized_abs),
        Err(DomainError::ArithmeticOverflow)
    ));
    for (label, key) in [
        ("density above u32", "min_density_ppm"),
        ("shrink candidates above u32", "max_shrink_candidates"),
    ] {
        let mut bounds = clip_bounds.clone();
        bounds.insert(key.to_owned(), i64::MAX as u64 + 1);
        assert!(
            matches!(
                checked_clip_limits(&bounds),
                Err(DomainError::ArithmeticOverflow)
            ),
            "{label}: expected ArithmeticOverflow"
        );
    }
    let mut oversized_points = clip_bounds.clone();
    oversized_points.insert("max_wave_points".to_owned(), u64::MAX);
    match checked_clip_limits(&oversized_points) {
        Err(DomainError::InvalidBounds(message)) => {
            assert!(message.contains("cap"), "unexpected message: {message}");
        }
        other => panic!("oversized max_wave_points expected InvalidBounds, got {other:?}"),
    }

    // ── real strategies through simplify/complicate ────────────────────────
    let phase_limits = phase_probe_limits();
    let mut runner = TestRunner::deterministic();
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(PHASE_BANDS.to_vec()).no_shrink(),
        "phase band selector",
    );
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(CLIP_TOPOLOGIES.to_vec()).no_shrink(),
        "clip topology selector",
    );
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(CLIP_ANGLES.to_vec()).no_shrink(),
        "clip angle selector",
    );

    let mut phase_probes = 0_usize;
    for band in PHASE_BANDS {
        for flip in [false, true] {
            let strategy = phase_strategy(&phase_limits, &[band], Some(flip));
            let mut tree = strategy
                .new_tree(&mut runner)
                .expect("phase shrink-probe tree construction");
            let mut previous = tree.current();
            check_phase_case(&phase_limits, &previous).expect("initial phase case");
            check_grid_case(&phase_limits, &previous.grid).expect("initial grid case");
            assert_eq!(previous.band, band, "the band restriction was lost");
            assert_eq!(previous.flip, flip, "the flip restriction was lost");
            let initial_bits = phase_bits(&previous);
            let mut changed = false;
            let mut simplified = 0_usize;
            let mut complicated = 0_usize;
            let mut repeated = 0_usize;
            let mut false_simplify_stable = false;
            for _ in 0..SHRINK_STEPS {
                let before_simplify = format!("{:?}", tree.current());
                if !tree.simplify() {
                    false_simplify_stable = format!("{:?}", tree.current()) == before_simplify;
                    break;
                }
                simplified += 1;
                let current = tree.current();
                check_phase_case(&phase_limits, &current).expect("simplified phase case");
                check_grid_case(&phase_limits, &current.grid).expect("simplified grid case");
                assert_eq!(current.band, band, "shrinking moved the orientation band");
                assert_eq!(current.flip, flip, "shrinking flipped the flip branch");
                changed |= phase_bits(&current) != initial_bits;
                previous = current;
                if !tree.complicate() {
                    break;
                }
                complicated += 1;
                let restored = tree.current();
                check_phase_case(&phase_limits, &restored).expect("complicated phase case");
                check_grid_case(&phase_limits, &restored.grid).expect("complicated grid case");
                changed |= phase_bits(&restored) != initial_bits;
                previous = restored;
                if tree.complicate() {
                    repeated += 1;
                    let repeated_case = tree.current();
                    check_phase_case(&phase_limits, &repeated_case)
                        .expect("repeated-complicate phase case");
                    check_grid_case(&phase_limits, &repeated_case.grid)
                        .expect("repeated-complicate grid case");
                    changed |= phase_bits(&repeated_case) != initial_bits;
                }
            }
            assert!(
                simplified > 0 && complicated > 0,
                "{} flip={flip}: simplify={simplified} complicate={complicated}; both \
                 directions must be exercised",
                band.label()
            );
            assert!(changed, "{} flip={flip}: no scalar moved", band.label());
            assert!(
                repeated > 0,
                "{} flip={flip}: repeated complicate never ran",
                band.label()
            );
            assert!(
                false_simplify_stable,
                "{} flip={flip}: a false simplify moved the phase case",
                band.label()
            );
            let settled = format!("{:?}", tree.current());
            assert!(
                !tree.complicate(),
                "{} flip={flip}: complicate after exhaustion must be false",
                band.label()
            );
            assert_eq!(
                format!("{:?}", tree.current()),
                settled,
                "{} flip={flip}: complicated after exhaustion moved the phase case",
                band.label()
            );
            let _ = previous;
            phase_probes += 1;
            println!(
                "phase band {} flip={flip} simplify={simplified} complicate={complicated} \
                 repeated_complicate={repeated}",
                band.label()
            );
        }
    }
    assert_eq!(phase_probes, PHASE_BANDS.len() * 2);

    let clip_limits = clip_probe_limits();
    let mid_density = DENSITY_SPLIT_PPM;
    let width_mid = (clip_limits.width_floor + clip_limits.max_width_units) / 2;
    let probes: [(ClipTopology, ClipAngle, (u32, u32), (i64, i64), &str); 5] = [
        (
            ClipTopology::HoledRect,
            ClipAngle::Axis,
            (clip_limits.min_density_ppm, mid_density),
            (clip_limits.width_floor, width_mid),
            "holed/axis/low-density/narrow",
        ),
        (
            ClipTopology::HoledRect,
            ClipAngle::Axis,
            (mid_density + 1, 1_000_000),
            (width_mid + 1, clip_limits.max_width_units),
            "holed/axis/high-density/wide",
        ),
        (
            ClipTopology::HoledRect,
            ClipAngle::Rotated,
            (clip_limits.min_density_ppm, 1_000_000),
            (clip_limits.width_floor, clip_limits.max_width_units),
            "holed/rotated",
        ),
        (
            ClipTopology::PlainRect,
            ClipAngle::Rotated,
            (clip_limits.min_density_ppm, 1_000_000),
            (clip_limits.width_floor, clip_limits.max_width_units),
            "plain/rotated",
        ),
        (
            ClipTopology::TwoComponents,
            ClipAngle::Axis,
            (clip_limits.min_density_ppm, 1_000_000),
            (clip_limits.width_floor, clip_limits.max_width_units),
            "two-components/axis",
        ),
    ];
    let mut clip_probes = 0_usize;
    for (topology, angle, density_range, width_range, label) in probes {
        if topology.vertices() > clip_limits.max_vertices
            || topology.holes() > clip_limits.max_holes
            || topology.components() > clip_limits.max_components
        {
            continue;
        }
        let strategy = clip_strategy(
            &clip_limits,
            &[topology],
            &[angle],
            density_range,
            width_range,
        );
        let mut tree = strategy
            .new_tree(&mut runner)
            .expect("clip shrink-probe tree construction");
        let mut previous = tree.current();
        let initial_bits = {
            let case = build_clip_case(&clip_limits, &previous).expect("initial clip case");
            run_clip_observation(&case, &clip_limits).expect("initial clip observation");
            clip_bits(&case)
        };
        assert_eq!(previous.topology, topology, "{label}: topology lost");
        assert_eq!(previous.angle, angle, "{label}: angle lost");
        let mut changed = false;
        let mut simplified = 0_usize;
        let mut complicated = 0_usize;
        let mut repeated = 0_usize;
        let steps = SHRINK_STEPS.min(clip_limits.max_shrink_candidates as usize);
        let mut false_simplify_stable = false;
        for _ in 0..steps {
            let before_simplify = format!("{:?}", tree.current());
            if !tree.simplify() {
                false_simplify_stable = format!("{:?}", tree.current()) == before_simplify;
                break;
            }
            simplified += 1;
            let current = tree.current();
            let case = build_clip_case(&clip_limits, &current).expect("simplified clip case");
            run_clip_observation(&case, &clip_limits).expect("simplified clip observation");
            changed |= clip_bits(&case) != initial_bits;
            previous = current;
            if !tree.complicate() {
                break;
            }
            complicated += 1;
            let restored = tree.current();
            let case = build_clip_case(&clip_limits, &restored).expect("complicated clip case");
            run_clip_observation(&case, &clip_limits).expect("complicated clip observation");
            changed |= clip_bits(&case) != initial_bits;
            previous = restored;
            if tree.complicate() {
                repeated += 1;
                let repeated_case = tree.current();
                let case =
                    build_clip_case(&clip_limits, &repeated_case).expect("repeated clip case");
                run_clip_observation(&case, &clip_limits).expect("repeated clip observation");
                changed |= clip_bits(&case) != initial_bits;
            }
        }
        assert!(
            simplified > 0 && complicated > 0,
            "{label}: simplify={simplified} complicate={complicated}; both directions must be \
             exercised"
        );
        assert!(changed, "{label}: no exposed scalar moved");
        assert!(repeated > 0, "{label}: repeated complicate never ran");
        assert!(
            false_simplify_stable,
            "{label}: a false simplify moved the clip case"
        );
        let settled = format!("{:?}", tree.current());
        assert!(
            !tree.complicate(),
            "{label}: complicate after exhaustion must be false"
        );
        assert_eq!(
            format!("{:?}", tree.current()),
            settled,
            "{label}: complicate after exhaustion moved the clip case"
        );
        let _ = previous;
        clip_probes += 1;
        println!(
            "clip {label}: simplify={simplified} complicate={complicated} \
             repeated_complicate={repeated}"
        );
    }
    assert!(clip_probes > 0, "the clip shrink probe covered no mode");
}
