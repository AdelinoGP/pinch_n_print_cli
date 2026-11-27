//! Opt-in property tests for this module's seam pick, paint and spline kernels.
//!
//! Charter (`docs/spec_packets/proptest-openrscad_15_pbt-layers-seams`):
//!
//! * `M-SEAM-PLANNER-PICK` — `comparator::pick_seam_point`, **dormant-kernel
//!   coverage**. The comparator has zero production callers on this tree: the
//!   only call sites are the `#[cfg(test)]` modules of
//!   `modules/core-modules/seam-planner-default/src/{comparator,align}.rs`, and
//!   `run_region_planning_entries` never consults it. The property therefore
//!   says nothing about live seam placement. The kernel scans a **nonempty,
//!   in-bounds** `perimeter_range` and returns an index inside that range;
//!   a strict tie retains the first index. The independently derived winner is
//!   computed per semantic case class by the test's own priority/score
//!   comparison (enforcement rank, central-enforcer flag, visibility penalty,
//!   rear-Y preference, exact ties), never by calling `is_first_better`. The
//!   empty-range behavior (`perimeter_range.start`, unchecked, not an
//!   `Option`) is asserted separately and never indexed. The dormant
//!   position-seeded sibling `comparator::pick_random_seam_point` is covered in
//!   the same property for reproducible membership and an interpolated point on
//!   the selected edge only — not for distribution or seam quality.
//! * `M-SEAM-PLANNER-PAINT` — `region_candidates` + `choose_region_candidate`,
//!   the **live** planner selection path (both are called by
//!   `run_region_planning_entries`). Exact `Custom("seam_blocker")` /
//!   `Custom("seam_enforcer")` `Flag(true)` annotations classify contour
//!   vertices: blockers are removed from the candidate list, the leading third
//!   of a contiguous enforced run (at least one vertex) scores `2`, the rest of
//!   that run `1`, and nothing else `0`. A mode chooses only among maximum-score
//!   candidates; an empty selection returns `None`; `Random` indexes the
//!   eligible set with `layer_index % eligible.len()`. Hole vertices are
//!   neutral. This is direct selection-kernel coverage, **not** evidence of
//!   end-to-end paint delivery: the production-path regression that paint moves
//!   the emitted seam is `seam_paint_moves_planner_resolved_seam` in
//!   `modules/core-modules/seam-planner-default/tests/seam_region_aware_planning_tdd.rs`.
//! * `M-SEAM-PLANNER-SPLINE` — `align::fit_cubic_bspline`, **dormant-kernel
//!   coverage**. Its only caller is `align_seam_points`, itself reachable only
//!   from its own tests, so this property is transitively dormant as well. The
//!   asserted families are a positive-span constant-data family at observed and
//!   interior parameters, the same-parameter weighted-mean degeneracy, and a
//!   single observation; output must be finite and inside a tolerance derived
//!   below. No arbitrary interpolation, affine exactness, extrapolation or
//!   universal fit-error claim is made.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! The kernels are private or `pub(crate)`, so the properties live here as a
//! crate-internal unit-test module; production visibility is not widened.
//!
//! # Independent expected model
//!
//! Three separate models, none of which calls the kernel it judges:
//!
//! 1. **Range index membership/priority (pick).** A generated case belongs to
//!    one semantic class, and each class varies exactly one dimension of the
//!    documented priority order while every other compared field is held
//!    constant. The expected index is then the first index in
//!    `perimeter_range` that attains the class's extreme value: maximum
//!    enforcement rank (`Enforced > Neutral > Blocked`), first central-enforcer
//!    index for the aligned setups, minimum `visibility` penalty (the only
//!    varying term of `base_penalty`, so the ordering needs no reimplementation
//!    of the gaussian), first maximum `position[1]` for `Rear`, or
//!    `perimeter_range.start` for an exact tie. Candidates outside the range are
//!    deliberately given the class's best value, so a scan that ignores the
//!    range returns a different index and fails. The interpolated random-sibling
//!    point is checked with an f64 cross-product/parameter computation against
//!    the two candidate positions that bound the selected edge.
//! 2. **Paint classification (paint).** Each contour vertex carries a generated
//!    *intent* (`0` neutral, `1` blocked, `2` enforced); the annotation vectors
//!    handed to the kernel are derived from that intent, and the expected
//!    candidate table is derived from the same intent, not from re-parsing the
//!    annotation. A maximal intent run of `2`s of length `L` contributes
//!    `2.0` to its first `L.div_ceil(3).max(1)` vertices and `1.0` to the rest
//!    (the documented "first third of a contiguous enforced run, at least one"
//!    convention, pinned by a concrete run-length table in the witness test);
//!    blocked vertices contribute no candidate; every other contour vertex and
//!    every hole vertex is `0.0`. Candidate order, xy/z/width, `flow_factor`
//!    and the reason tag (`enforced`/`aligned`) are compared field by field.
//!    Selection is an independent rule model over that table: maximum score,
//!    then `Nearest`/`Aligned` = first minimum `(y, x)` (`Iterator::min_by`
//!    returns the first minimum), `Rear`/`AlignedBack` = maximum `y` with the
//!    smaller `x`, last on a full `(y, x)` tie (`Iterator::max_by` returns the
//!    last maximum), `Random` = `eligible[layer_index % eligible.len()]`.
//!    All generated coordinates are finite, non-NaN and never `-0.0`, so
//!    `f32::total_cmp` and numeric order agree. The module's `0.4` mm default
//!    scoring width is modelled as the documented fallback and pinned by a
//!    witness, not read from the private constant.
//! 3. **Restricted least-squares / weighted mean (spline).** Constant data is
//!    reproduced because the clamped cubic B-spline basis rows sum to one, so
//!    the constant vector lies in the column space; the claim is asserted only
//!    on the well-conditioned sub-domain `segments + 1 <= observations.len()`
//!    where the weighted least-squares solution is unique. The degenerate
//!    same-parameter branch is an f64-accumulated weighted mean of the
//!    observations. Tolerance derivation: observations and parameters are
//!    bounded by `max_coord_quanta` (8 mm in every measured profile, 10 mm for
//!    the packet's hand-written witness), weights by `max_weight_quanta`
//!    (4.0) and floored at 1.0 so `sum(w) >= observations.len()`,
//!    `observations.len() <= 32` and `segments <= 32`. A fitted value is a sum
//!    of at most four kernel terms with `|k| <= 1` and coefficients of
//!    magnitude `<= 8` mm, so f32 rounding costs `<= 4 * 2^-24 * 8 ~= 1.9e-6`
//!    mm; the f64 solve adds `~1e-15 * kappa`. The weighted mean accumulates at
//!    most 32 terms in f32, bounding its error by
//!    `(n-1) * u * sum|w*x| / sum(w) <= 31 * 6e-8 * (32*4*8) / 32 ~= 6e-5`
//!    mm. `SPLINE_TOL_MM = 1e-3` is the resulting envelope: ~17x the derived
//!    weighted-mean bound and ~500x the derived constant-fit bound.
//!
//! # Literal review
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) does not collect
//! `modules/core-modules/*/src/**`, and the stacked `#[cfg(test)]` /
//! `#[cfg(feature = "property_testing")]` attributes do not make it scanned.
//! Manually reviewed sites:
//!
//! * `SeamPlanningRegionInput` (`crates/slicer-sdk/src/prepass_types.rs`, eight
//!   named fields, watched) — one literal in `PaintCase::region`, carrying a
//!   `..Default::default()` rest for the identity/layer-height fields the
//!   candidate and selection kernels never read.
//! * `Point3WithWidth` (`crates/slicer-ir/src/slice_ir.rs`, eight named fields,
//!   watched) — no literal: expected points are compared field by field against
//!   the kernel's own output, and mutation controls clone a real emitted
//!   candidate.
//! * `ScoredSeamCandidate` (three fields), `SeamReason` (one), `PaintValue`,
//!   `PaintSemantic`, `ExPolygon`, `Polygon`, `Point2` are not watched.
//! * `crate::comparator::SeamCandidate` is `pub(crate)` under `modules/`, so it
//!   is not a watched crate-public struct; its literals are exhaustive on
//!   purpose because that kernel's field set is the contract under test.
//! * `Limits`, `PickCase`, `PaintCase`, `SplineCase`, `ExpectedCandidate`,
//!   `PickClass`, `PaintIntent`, `SplineKind` and the failure/stats types are
//!   private module-local types.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. Every
//! negative control mutates an observation of the **same** checker the property
//! uses and asserts that checker's specific failure class, so none is a
//! success-only or decorative assertion. The properties require positive
//! per-subcase counters fed by deterministic witnesses constructed inside the
//! callback plus generated cases, and empty cases have their own exact
//! assertions and their own counters, which no positive counter can inherit.
//! Repeat-call comparisons are marked `// test-quality:` determinism pins; they
//! are not counted as independent correctness.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseResult, TestRunner};

use slicer_sdk::prelude::*;
use slicer_sdk::test_support::property_testing::PropertyRun;

use crate::align::fit_cubic_bspline;
use crate::comparator::{
    pick_random_seam_point, pick_seam_point, EnforcedBlockedSeamPoint, SeamCandidate,
    SeamComparator, SeamSetup,
};

use super::{choose_region_candidate, region_candidates, SeamPlannerMode};

const PICK_ID: &str = "M-SEAM-PLANNER-PICK";
const PICK_TEST: &str = "property_tests::proptest_planner_pick";
const PAINT_ID: &str = "M-SEAM-PLANNER-PAINT";
const PAINT_TEST: &str = "property_tests::proptest_planner_paint";
const SPLINE_ID: &str = "M-SEAM-PLANNER-SPLINE";
const SPLINE_TEST: &str = "property_tests::proptest_planner_spline";

/// The exact `PropertyRun::bound` keys each registry record carries. A profile
/// records only the keys its own SUT reads, so `live_limits` merges the live
/// values over the probe defaults (see `live_limits`).
const PICK_BOUND_KEYS: [&str; 3] = ["max_candidates", "max_coord_quanta", "max_score_quanta"];
const PAINT_BOUND_KEYS: [&str; 2] = ["max_candidates", "max_coord_quanta"];
const SPLINE_BOUND_KEYS: [&str; 4] = [
    "max_observations",
    "max_segments",
    "max_coord_quanta",
    "max_weight_quanta",
];

/// One generated numeric quantum: `1/1024` mm for float geometry and Z, and
/// `1/1024` for the dimensionless penalty/weight scalars (packet §"Design
/// notes" §"Ownership, execution and feature boundaries").
const QUANTA_PER_UNIT: f32 = 1024.0;

/// The module's documented candidate fallback width when a region supplies a
/// nonpositive or nonfinite `scoring_width` (`DEFAULT_FLOW_WIDTH_MM` in
/// `modules/core-modules/seam-planner-default/src/lib.rs`), mm. Pinned through
/// the emitted candidate, never read from the private constant at test time.
const DEFAULT_SCORING_WIDTH_MM: f32 = 0.4;

/// Fixture range for a generated `scoring_width`, quanta: `(0, 4]` mm.
const SCORING_WIDTH_QUANTA_CAP: i64 = 4096;

/// Derived constant-fit / weighted-mean envelope, mm. See the module header.
const SPLINE_TOL_MM: f32 = 1e-3;

/// Derived position envelope for the dormant random sibling's interpolated
/// point, mm. A 3D edge is at most 16 mm and the interpolation is f32, so the
/// perpendicular/parameter error costs at most `16 * 2^-23 * 4 ~= 7.6e-6` mm;
/// 1e-3 mm is a ~130x margin.
const RANDOM_EDGE_TOL_MM: f64 = 1e-3;

/// The hard fit envelope of the hand-written spline witnesses, mm. The packet's
/// weighted-mean witness uses 10 mm observations, above the routine profile's
/// 8 mm generated bound, so generated cases are additionally checked against
/// their live profile bound while witnesses use this fixed shape envelope.
const FIT_ABS_CAP_MM: f32 = 16.0;

const MAX_SHRINK_STEPS: usize = 20_000;

/// Profile ceilings and the minimum sizes the deterministic witnesses need.
const CANDIDATES_RANGE: Range<u64> = 7..33;
const OBSERVATIONS_RANGE: Range<u64> = 4..33;
const SEGMENTS_RANGE: Range<u64> = 2..33;
const COORD_QUANTA_RANGE: Range<u64> = 4096..65537;
const SCORE_QUANTA_RANGE: Range<u64> = 1024..65537;
const WEIGHT_QUANTA_RANGE: Range<u64> = 2048..65537;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

/// `1/1024` of a field's unit, as f32. Every generated magnitude is at most
/// 65536 quanta, far below `2^24`, so this conversion is exact.
fn from_quanta(quanta: i64) -> f32 {
    quanta as f32 / QUANTA_PER_UNIT
}

/// The documented central prefix of a contiguous enforced run of `length`
/// vertices: the first third, at least one vertex. This restates the annotation
/// convention ("first third of each contiguous enforced run"), and the concrete
/// run-length table in `packet15_witnesses_controls_and_shrinking` pins the SUT
/// against it.
fn central_prefix(length: usize) -> usize {
    length.div_ceil(3).max(1)
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct Limits {
    max_candidates: u32,
    max_coord_quanta: i64,
    max_score_quanta: i64,
    max_observations: u32,
    max_segments: u32,
    max_weight_quanta: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LimitsError {
    Missing(String),
    OutOfRange(String),
    Inconsistent(String),
    ArithmeticOverflow,
}

impl fmt::Display for LimitsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(m) => write!(f, "missing bound key `{m}`"),
            Self::OutOfRange(m) => write!(f, "bound out of range: {m}"),
            Self::Inconsistent(m) => write!(f, "inconsistent profile: {m}"),
            Self::ArithmeticOverflow => write!(f, "bound does not fit the conversion"),
        }
    }
}

fn checked_limits(bounds: &BTreeMap<String, u64>) -> Result<Limits, LimitsError> {
    let raw = |key: &str| -> Result<u64, LimitsError> {
        bounds
            .get(key)
            .copied()
            .ok_or_else(|| LimitsError::Missing(key.to_owned()))
    };
    let as_i64 = |key: &str| -> Result<i64, LimitsError> {
        i64::try_from(raw(key)?).map_err(|_| LimitsError::ArithmeticOverflow)
    };
    let as_u32 = |key: &str| -> Result<u32, LimitsError> {
        u32::try_from(raw(key)?).map_err(|_| LimitsError::ArithmeticOverflow)
    };

    let limits = Limits {
        max_candidates: as_u32("max_candidates")?,
        max_coord_quanta: as_i64("max_coord_quanta")?,
        max_score_quanta: as_i64("max_score_quanta")?,
        max_observations: as_u32("max_observations")?,
        max_segments: as_u32("max_segments")?,
        max_weight_quanta: as_i64("max_weight_quanta")?,
    };

    let in_range = |label: &str, value: u64, range: &Range<u64>| -> Result<(), LimitsError> {
        if range.contains(&value) {
            Ok(())
        } else {
            Err(LimitsError::OutOfRange(format!(
                "{label} = {value} is outside {}..{} (witness minimum and packet ceiling)",
                range.start, range.end
            )))
        }
    };
    in_range(
        "max_candidates",
        u64::from(limits.max_candidates),
        &CANDIDATES_RANGE,
    )?;
    in_range(
        "max_observations",
        u64::from(limits.max_observations),
        &OBSERVATIONS_RANGE,
    )?;
    in_range(
        "max_segments",
        u64::from(limits.max_segments),
        &SEGMENTS_RANGE,
    )?;
    in_range(
        "max_coord_quanta",
        u64::try_from(limits.max_coord_quanta).map_err(|_| LimitsError::ArithmeticOverflow)?,
        &COORD_QUANTA_RANGE,
    )?;
    in_range(
        "max_score_quanta",
        u64::try_from(limits.max_score_quanta).map_err(|_| LimitsError::ArithmeticOverflow)?,
        &SCORE_QUANTA_RANGE,
    )?;
    in_range(
        "max_weight_quanta",
        u64::try_from(limits.max_weight_quanta).map_err(|_| LimitsError::ArithmeticOverflow)?,
        &WEIGHT_QUANTA_RANGE,
    )?;
    if limits.max_segments > limits.max_observations {
        return Err(LimitsError::Inconsistent(format!(
            "max_segments {} exceeds max_observations {} so no generated fit could respect \
             segments <= observations",
            limits.max_segments, limits.max_observations
        )));
    }
    Ok(limits)
}

/// Read every bound the selected profile records through `run.bound`, then fill
/// the remaining shared `Limits` fields from `probe_bound_map`. A missing or
/// zero bound is an error, never a defaulted value; the fields a profile does
/// not record are never consulted by that property's generator or checker.
fn live_limits(run: &PropertyRun, keys: &[&str]) -> Limits {
    let mut bounds = probe_bound_map();
    for key in keys {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("seam planner bound `{key}`: {error}"));
        bounds.insert((*key).to_owned(), value);
    }
    checked_limits(&bounds).expect("seam planner profile bound consistency")
}

/// The smallest profile that admits every deterministic witness and every
/// generated class: the same values the measured registry records carry. The
/// context-free controls and the shrink probe use it without a `PropertyRun`.
fn probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_candidates".to_owned(), 16),
        ("max_coord_quanta".to_owned(), 8192),
        ("max_score_quanta".to_owned(), 4096),
        ("max_observations".to_owned(), 16),
        ("max_segments".to_owned(), 8),
        ("max_weight_quanta".to_owned(), 4096),
    ])
}

// ─────────────────────────────────────────────────────────────────────────────
// The one failure vocabulary shared by every checker
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckFailure {
    /// The fixture itself is outside the declared domain (generator bug).
    Fixture(String),
    /// `pick_seam_point` returned an index outside the nonempty range.
    OutOfRange(String),
    /// `pick_seam_point` returned an in-range index that is not the first best.
    NotBest(String),
    /// The dormant random sibling's interpolated point is not on its edge.
    OffEdge(String),
    /// A blocked contour vertex survived into the candidate list.
    BlockedPresent(String),
    /// The annotation -> candidate table (order, xy/z/width, score, tag).
    Table(String),
    /// The mode's chosen candidate disagrees with the independent selection.
    Selection(String),
    /// A fit/observation value is not finite.
    NonFinite(String),
    /// A finite fit value is outside the derived tolerance envelope.
    Envelope(String),
}

impl fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(m) => write!(f, "fixture outside the domain: {m}"),
            Self::OutOfRange(m) => write!(f, "out-of-range index: {m}"),
            Self::NotBest(m) => write!(f, "not the independently expected best: {m}"),
            Self::OffEdge(m) => write!(f, "interpolated point off its edge: {m}"),
            Self::BlockedPresent(m) => write!(f, "blocked vertex retained: {m}"),
            Self::Table(m) => write!(f, "candidate table: {m}"),
            Self::Selection(m) => write!(f, "selection: {m}"),
            Self::NonFinite(m) => write!(f, "nonfinite value: {m}"),
            Self::Envelope(m) => write!(f, "outside the derived envelope: {m}"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SEAM-PLANNER-PICK: dormant comparator kernels
// ─────────────────────────────────────────────────────────────────────────────

/// One semantic pick class: each varies exactly one compared dimension, so the
/// expected index is unambiguous without reproducing `is_first_better`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickClass {
    /// Enforcement rank varies; all neutral ties on `Rear` share one Y.
    PaintNearest,
    /// Same ranking under `Rear`, proving the type gate precedes the Y gate.
    PaintRear,
    /// Central-enforcer flag varies among enforced candidates, `Aligned`.
    CentralAligned,
    /// Same, `AlignedBack`.
    CentralAlignedBack,
    /// Visibility (the only varying penalty term) varies.
    VisibilityNearest,
    /// `position[1]` varies under `Rear`.
    RearY,
    /// Every compared field is identical: a strict tie retains the first index.
    TieAligned,
}

const PICK_CLASSES: [PickClass; 7] = [
    PickClass::PaintNearest,
    PickClass::PaintRear,
    PickClass::CentralAligned,
    PickClass::CentralAlignedBack,
    PickClass::VisibilityNearest,
    PickClass::RearY,
    PickClass::TieAligned,
];

impl PickClass {
    fn label(self) -> &'static str {
        match self {
            Self::PaintNearest => "paint-nearest",
            Self::PaintRear => "paint-rear",
            Self::CentralAligned => "central-aligned",
            Self::CentralAlignedBack => "central-aligned-back",
            Self::VisibilityNearest => "visibility-nearest",
            Self::RearY => "rear-y",
            Self::TieAligned => "tie-aligned",
        }
    }

    fn setup(self) -> SeamSetup {
        match self {
            Self::PaintNearest | Self::VisibilityNearest => SeamSetup::Nearest,
            Self::PaintRear | Self::RearY => SeamSetup::Rear,
            Self::CentralAligned | Self::TieAligned => SeamSetup::Aligned,
            Self::CentralAlignedBack => SeamSetup::AlignedBack,
        }
    }

    fn is_paint(self) -> bool {
        matches!(self, Self::PaintNearest | Self::PaintRear)
    }

    fn is_central(self) -> bool {
        matches!(self, Self::CentralAligned | Self::CentralAlignedBack)
    }
}

#[derive(Debug, Clone)]
struct PickCase {
    class: PickClass,
    candidates: Vec<SeamCandidate>,
    range: Range<usize>,
}

impl PartialEq for PickCase {
    /// Value identity for the shrink probe: the class, the sliced range and
    /// every field the comparator reads. `SeamCandidate` itself has no
    /// `PartialEq`, and the decorative fields are deliberately excluded.
    fn eq(&self, other: &Self) -> bool {
        self.class == other.class
            && self.range == other.range
            && self.candidates.len() == other.candidates.len()
            && self.candidates.iter().zip(&other.candidates).all(|(a, b)| {
                a.position.map(f32::to_bits) == b.position.map(f32::to_bits)
                    && a.visibility.to_bits() == b.visibility.to_bits()
                    && a.overhang.to_bits() == b.overhang.to_bits()
                    && a.point_type == b.point_type
                    && a.central_enforcer == b.central_enforcer
            })
    }
}

const PICK_RAW_LEN: usize = 9;

fn seam_candidate(
    position: [f32; 3],
    point_type: EnforcedBlockedSeamPoint,
    central_enforcer: bool,
    visibility: f32,
) -> SeamCandidate {
    // `SeamCandidate` is `pub(crate)` under `modules/`, not a watched
    // crate-public struct, and it has no `Default`: every field is spelled.
    SeamCandidate {
        position,
        visibility,
        overhang: 0.0,
        unsupported_dist: 0.0,
        embedded_distance: 0.0,
        local_ccw_angle: -0.5,
        layer_angle: 0.0,
        central_enforcer,
        point_type,
        flow_width: 0.4,
    }
}

/// Give one out-of-range candidate the class's best value, so a scan that
/// ignores `perimeter_range` returns a different index.
fn paint_strongest(candidate: &mut SeamCandidate, class: PickClass, coord_cap: i64) {
    match class {
        PickClass::PaintNearest | PickClass::PaintRear => {
            candidate.point_type = EnforcedBlockedSeamPoint::Enforced;
        }
        PickClass::CentralAligned | PickClass::CentralAlignedBack => {
            candidate.central_enforcer = true;
        }
        PickClass::VisibilityNearest => candidate.visibility = 0.0,
        PickClass::RearY => candidate.position[1] = from_quanta(coord_cap),
        PickClass::TieAligned => {}
    }
}

fn build_pick_case(class: PickClass, raw: [u32; PICK_RAW_LEN], limits: &Limits) -> PickCase {
    let max_candidates = limits.max_candidates as usize;
    let coord_cap = limits.max_coord_quanta;
    let score_cap = limits.max_score_quanta;

    // A painted class must be able to carry both an enforced and a
    // non-enforced vertex, and the rear-Y class needs a Y spread, so both need
    // at least two candidates.
    let len = if class.is_paint() || class == PickClass::RearY {
        2 + (raw[0] as usize) % (max_candidates - 1)
    } else {
        1 + (raw[0] as usize) % max_candidates
    };
    let lo = (raw[1] as usize) % len;
    let hi = lo + 1 + (raw[2] as usize) % (len - lo);
    // Affine layout bounded so no coordinate needs clamping; a clamped layout
    // would collapse the rear-Y spread the class requires.
    let step = 1 + i64::from(raw[3]) % (coord_cap / (len as i64 + 1)).max(1);
    let max_offset = (len as i64 - 1) * step;
    let room = (coord_cap - max_offset + 1).max(1);
    let base_x = i64::from(raw[4]) % room;
    let flat_y = i64::from(raw[5]) % room;
    let flat_visibility = from_quanta(1 + i64::from(raw[6]) % score_cap);

    let mut candidates: Vec<SeamCandidate> = (0..len)
        .map(|i| {
            let offset = i as i64 * step;
            let x = from_quanta(base_x + offset);
            let y = match class {
                PickClass::RearY => from_quanta(flat_y + offset),
                _ => from_quanta(flat_y),
            };
            let visibility = match class {
                PickClass::VisibilityNearest => {
                    from_quanta(1 + i64::from(raw[6 + i % 3]) % score_cap)
                }
                _ => flat_visibility,
            };
            let (point_type, central) = match class {
                PickClass::PaintNearest | PickClass::PaintRear => {
                    let kind = match raw[6 + i % 3] % 3 {
                        0 => EnforcedBlockedSeamPoint::Blocked,
                        1 => EnforcedBlockedSeamPoint::Neutral,
                        _ => EnforcedBlockedSeamPoint::Enforced,
                    };
                    (kind, false)
                }
                PickClass::CentralAligned | PickClass::CentralAlignedBack => (
                    EnforcedBlockedSeamPoint::Enforced,
                    raw[6 + i % 3].is_multiple_of(2),
                ),
                _ => (EnforcedBlockedSeamPoint::Neutral, false),
            };
            seam_candidate([x, y, 0.0], point_type, central, visibility)
        })
        .collect();

    // Structural overlay: the slice must contain a genuine witness (an enforced
    // vertex in the painted classes, a central enforcer in the central classes,
    // a non-start extreme in the score/Y classes) and the first in-range
    // candidate must not be the answer, so a kernel returning `range.start`
    // fails. Both indices are derived from `raw`, so the overlay survives
    // shrinking.
    let has_pair = hi - lo >= 2;
    match class {
        PickClass::PaintNearest | PickClass::PaintRear => {
            candidates[lo].point_type = EnforcedBlockedSeamPoint::Neutral;
            let target = if has_pair { lo + 1 } else { lo };
            candidates[target].point_type = EnforcedBlockedSeamPoint::Enforced;
        }
        PickClass::CentralAligned | PickClass::CentralAlignedBack => {
            candidates[lo].central_enforcer = false;
            let target = if has_pair { lo + 1 } else { lo };
            candidates[target].central_enforcer = true;
        }
        PickClass::VisibilityNearest => {
            candidates[lo].visibility = from_quanta(score_cap);
            let target = if has_pair { lo + 1 } else { lo };
            candidates[target].visibility = from_quanta(1);
        }
        PickClass::RearY => {
            // One quantum of headroom below `coord_cap` is reserved for the
            // out-of-range "strongest" decoration below, so a decoration is
            // strictly better under the Rear key than every in-range candidate
            // (and the class always keeps a Y spread, even for a singleton range).
            let headroom = from_quanta((coord_cap - 1).max(0));
            for candidate in &mut candidates[lo..hi] {
                candidate.position[1] = candidate.position[1].min(headroom);
            }
            candidates[lo].position[1] = 0.0;
            let target = if has_pair { lo + 1 } else { lo };
            candidates[target].position[1] = headroom;
        }
        PickClass::TieAligned => {}
    }

    if lo > 0 {
        paint_strongest(&mut candidates[0], class, coord_cap);
    }
    if hi < len {
        paint_strongest(&mut candidates[len - 1], class, coord_cap);
    }
    // A singleton range cannot hold both classes by itself; place the missing
    // non-enforced vertex outside the range. `len >= 2` makes one of the two
    // branches available, and neither touches the in-range answer.
    if class.is_paint() && hi - lo == 1 {
        if hi < len {
            candidates[hi].point_type = EnforcedBlockedSeamPoint::Neutral;
        } else {
            candidates[0].point_type = EnforcedBlockedSeamPoint::Neutral;
        }
    }

    PickCase {
        class,
        candidates,
        range: lo..hi,
    }
}

fn pick_case_strategy(limits: &Limits) -> BoxedStrategy<PickCase> {
    let limits = *limits;
    (
        prop::sample::select(PICK_CLASSES.to_vec()).no_shrink(),
        any::<[u32; PICK_RAW_LEN]>(),
    )
        .prop_map(move |(class, raw)| build_pick_case(class, raw, &limits))
        .boxed()
}

/// Rank of a paint classification in the comparator's priority order.
fn enforcement_rank(point_type: EnforcedBlockedSeamPoint) -> u8 {
    match point_type {
        EnforcedBlockedSeamPoint::Blocked => 0,
        EnforcedBlockedSeamPoint::Neutral => 1,
        EnforcedBlockedSeamPoint::Enforced => 2,
    }
}

/// The independent expected winner: the first index in `range` attaining the
/// class's extreme. Never calls the comparator.
fn expected_pick_index(case: &PickCase) -> usize {
    let start = case.range.start;
    match case.class {
        PickClass::PaintNearest | PickClass::PaintRear => {
            (start..case.range.end).fold(start, |best, i| {
                if enforcement_rank(case.candidates[i].point_type)
                    > enforcement_rank(case.candidates[best].point_type)
                {
                    i
                } else {
                    best
                }
            })
        }
        PickClass::CentralAligned | PickClass::CentralAlignedBack => {
            (start..case.range.end).fold(start, |best, i| {
                if case.candidates[i].central_enforcer && !case.candidates[best].central_enforcer {
                    i
                } else {
                    best
                }
            })
        }
        PickClass::VisibilityNearest => (start..case.range.end).fold(start, |best, i| {
            if case.candidates[i].visibility < case.candidates[best].visibility {
                i
            } else {
                best
            }
        }),
        PickClass::RearY => (start..case.range.end).fold(start, |best, i| {
            if case.candidates[i].position[1] > case.candidates[best].position[1] {
                i
            } else {
                best
            }
        }),
        PickClass::TieAligned => start,
    }
}

#[derive(Debug, Clone, Copy)]
struct PickStats {
    /// The case carries both an enforced and a non-enforced vertex.
    paint_mix: bool,
}

fn validate_pick_case(case: &PickCase, limits: &Limits) -> Result<(), CheckFailure> {
    let fixture = |m: String| Err(CheckFailure::Fixture(m));
    if case.candidates.is_empty() {
        return fixture("no candidates".to_owned());
    }
    if case.candidates.len() > limits.max_candidates as usize {
        return fixture(format!(
            "{} candidates exceed max_candidates {}",
            case.candidates.len(),
            limits.max_candidates
        ));
    }
    if case.range.start >= case.range.end || case.range.end > case.candidates.len() {
        return fixture(format!(
            "range {}..{} is empty or out of bounds for {} candidates",
            case.range.start,
            case.range.end,
            case.candidates.len()
        ));
    }
    let score_cap = from_quanta(limits.max_score_quanta);
    let coord_cap = from_quanta(limits.max_coord_quanta);
    for candidate in &case.candidates {
        if !candidate.position.iter().all(|value| value.is_finite()) {
            return fixture(format!("nonfinite position {:?}", candidate.position));
        }
        if candidate.position[0].abs() > coord_cap || candidate.position[1].abs() > coord_cap {
            return fixture(format!(
                "position {:?} exceeds the coord bound",
                candidate.position
            ));
        }
        if !candidate.visibility.is_finite() || candidate.visibility < 0.0 {
            return fixture(format!("visibility {}", candidate.visibility));
        }
        if candidate.visibility > score_cap {
            return fixture(format!(
                "visibility {} exceeds max_score_quanta",
                candidate.visibility
            ));
        }
    }
    match case.class {
        PickClass::PaintNearest | PickClass::PaintRear => {
            if case.candidates.iter().any(|c| c.central_enforcer) {
                return fixture("painted class must not set central_enforcer".to_owned());
            }
            let y = case.candidates[0].position[1];
            if case.candidates.iter().any(|c| c.position[1] != y) {
                return fixture("painted class carries a Y difference".to_owned());
            }
            if !case
                .candidates
                .iter()
                .any(|c| c.point_type == EnforcedBlockedSeamPoint::Enforced)
            {
                return fixture("painted class lost its enforced vertex".to_owned());
            }
            if case
                .candidates
                .iter()
                .all(|c| c.point_type == EnforcedBlockedSeamPoint::Enforced)
            {
                return fixture("painted class lost its non-enforced vertex".to_owned());
            }
        }
        PickClass::CentralAligned | PickClass::CentralAlignedBack => {
            if case
                .candidates
                .iter()
                .any(|c| c.point_type != EnforcedBlockedSeamPoint::Enforced)
            {
                return fixture("central class must be wholly enforced".to_owned());
            }
            if !case.candidates.iter().any(|c| c.central_enforcer) {
                return fixture("central class lost its central enforcer".to_owned());
            }
        }
        PickClass::VisibilityNearest => {
            if case
                .candidates
                .iter()
                .any(|c| c.point_type != EnforcedBlockedSeamPoint::Neutral)
            {
                return fixture("visibility class must be wholly neutral".to_owned());
            }
        }
        PickClass::RearY => {
            if case
                .candidates
                .iter()
                .any(|c| c.point_type != EnforcedBlockedSeamPoint::Neutral)
            {
                return fixture("rear-Y class must be wholly neutral".to_owned());
            }
            let y = case.candidates[0].position[1];
            if case.candidates.iter().all(|c| c.position[1] == y) {
                return fixture("rear-Y class lost its Y spread".to_owned());
            }
        }
        PickClass::TieAligned => {
            let first = &case.candidates[0];
            let same = |c: &SeamCandidate| {
                c.point_type == first.point_type
                    && c.central_enforcer == first.central_enforcer
                    && c.position[1].to_bits() == first.position[1].to_bits()
                    && c.visibility.to_bits() == first.visibility.to_bits()
                    && c.local_ccw_angle.to_bits() == first.local_ccw_angle.to_bits()
                    && c.overhang.to_bits() == first.overhang.to_bits()
                    && c.embedded_distance.to_bits() == first.embedded_distance.to_bits()
            };
            if !case.candidates.iter().all(same) {
                return fixture("tie class carries a compared-field difference".to_owned());
            }
        }
    }
    Ok(())
}

fn check_pick(case: &PickCase, limits: &Limits, chosen: usize) -> Result<PickStats, CheckFailure> {
    validate_pick_case(case, limits)?;
    if chosen < case.range.start || chosen >= case.range.end {
        return Err(CheckFailure::OutOfRange(format!(
            "chosen {chosen} outside {}..{}",
            case.range.start, case.range.end
        )));
    }
    let expected = expected_pick_index(case);
    if chosen != expected {
        return Err(CheckFailure::NotBest(format!(
            "chosen {chosen} but the first best in range is {expected}"
        )));
    }
    let paint_mix = case
        .candidates
        .iter()
        .any(|c| c.point_type == EnforcedBlockedSeamPoint::Enforced)
        && case
            .candidates
            .iter()
            .any(|c| c.point_type != EnforcedBlockedSeamPoint::Enforced);
    Ok(PickStats { paint_mix })
}

/// Call the dormant random sibling twice and pin that the observation repeats
/// (`// test-quality:`): a repeat-call comparison is a determinism pin, not
/// independent correctness, which is what `check_random` supplies.
fn observe_random(candidates: &[SeamCandidate], range: Range<usize>) -> (usize, [f32; 3]) {
    let first = pick_random_seam_point(candidates, range.clone());
    let second = pick_random_seam_point(candidates, range);
    // test-quality: determinism pin for the position-seeded dormant sibling.
    assert_eq!(
        first, second,
        "pick_random_seam_point must reproduce its position-seeded observation"
    );
    first
}

fn check_random(
    candidates: &[SeamCandidate],
    range: Range<usize>,
    observed: (usize, [f32; 3]),
) -> Result<(), CheckFailure> {
    if candidates.is_empty() || range.start >= range.end || range.end > candidates.len() {
        return Err(CheckFailure::Fixture(format!(
            "random range {}..{} for {} candidates",
            range.start,
            range.end,
            candidates.len()
        )));
    }
    let (index, point) = observed;
    if index < range.start || index >= range.end {
        return Err(CheckFailure::OutOfRange(format!(
            "random pick {index} outside {}..{}",
            range.start, range.end
        )));
    }
    if !point.iter().all(|value| value.is_finite()) {
        return Err(CheckFailure::NonFinite(format!(
            "random pick point {point:?}"
        )));
    }
    let next = if index + 1 >= range.end {
        range.start
    } else {
        index + 1
    };
    let a = candidates[index].position.map(f64::from);
    let b = candidates[next].position.map(f64::from);
    let p = point.map(f64::from);
    let edge = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let delta = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let edge_sq = edge[0] * edge[0] + edge[1] * edge[1] + edge[2] * edge[2];
    if edge_sq == 0.0 {
        let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        if distance > RANDOM_EDGE_TOL_MM {
            return Err(CheckFailure::OffEdge(format!(
                "zero-length edge {index}->{next} but the point moved {distance} mm"
            )));
        }
        return Ok(());
    }
    let dot = delta[0] * edge[0] + delta[1] * edge[1] + delta[2] * edge[2];
    let cross = [
        edge[1] * delta[2] - edge[2] * delta[1],
        edge[2] * delta[0] - edge[0] * delta[2],
        edge[0] * delta[1] - edge[1] * delta[0],
    ];
    let perpendicular =
        (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt() / edge_sq.sqrt();
    let parameter = dot / edge_sq;
    if perpendicular > RANDOM_EDGE_TOL_MM {
        return Err(CheckFailure::OffEdge(format!(
            "point is {perpendicular} mm off edge {index}->{next}"
        )));
    }
    if !(-RANDOM_EDGE_TOL_MM..=1.0 + RANDOM_EDGE_TOL_MM).contains(&parameter) {
        return Err(CheckFailure::OffEdge(format!(
            "edge {index}->{next} parameter {parameter} is outside [0, 1]"
        )));
    }
    Ok(())
}

fn pick_complexity(case: &PickCase) -> i128 {
    let mut total = case.candidates.len() as i128 + (case.range.end - case.range.start) as i128;
    for candidate in &case.candidates {
        total += i128::from((candidate.position[0].abs() * QUANTA_PER_UNIT).round() as i64);
        total += i128::from((candidate.position[1].abs() * QUANTA_PER_UNIT).round() as i64);
        total += i128::from((candidate.visibility.abs() * QUANTA_PER_UNIT).round() as i64);
    }
    total
}

// ─────────────────────────────────────────────────────────────────────────────
// Pick witnesses
// ─────────────────────────────────────────────────────────────────────────────

/// A nonzero-start singleton: only index 1 is in the range.
fn pick_singleton_witness() -> PickCase {
    let candidates = vec![
        seam_candidate(
            [0.0, 0.0, 0.0],
            EnforcedBlockedSeamPoint::Neutral,
            false,
            1.0,
        ),
        seam_candidate(
            [1.0, 0.0, 0.0],
            EnforcedBlockedSeamPoint::Neutral,
            false,
            1.0,
        ),
        seam_candidate(
            [2.0, 0.0, 0.0],
            EnforcedBlockedSeamPoint::Enforced,
            false,
            1.0,
        ),
    ];
    PickCase {
        class: PickClass::PaintNearest,
        candidates,
        range: 1..2,
    }
}

/// A nonempty mixed-paint range whose best is independently obvious: an
/// enforced vertex at index 2 inside `1..3`, with an enforced decoy at index 0
/// outside the range and a neutral at index 1 inside.
fn pick_mixed_paint_witness() -> PickCase {
    let candidates = vec![
        seam_candidate(
            [7.0, 0.0, 0.0],
            EnforcedBlockedSeamPoint::Enforced,
            false,
            1.0,
        ),
        seam_candidate(
            [0.0, 0.0, 0.0],
            EnforcedBlockedSeamPoint::Neutral,
            false,
            1.0,
        ),
        seam_candidate(
            [5.0, 0.0, 0.0],
            EnforcedBlockedSeamPoint::Enforced,
            false,
            1.0,
        ),
        seam_candidate(
            [1.0, 0.0, 0.0],
            EnforcedBlockedSeamPoint::Neutral,
            false,
            1.0,
        ),
    ];
    PickCase {
        class: PickClass::PaintNearest,
        candidates,
        range: 1..3,
    }
}

/// Every compared field identical: the strict tie must retain the first index
/// of the scanned range.
fn pick_exact_tie_witness() -> PickCase {
    let candidates = (0..4)
        .map(|i| {
            seam_candidate(
                [i as f32, 0.0, 0.0],
                EnforcedBlockedSeamPoint::Neutral,
                false,
                1.0,
            )
        })
        .collect();
    PickCase {
        class: PickClass::TieAligned,
        candidates,
        range: 2..4,
    }
}

/// Two enforced candidates straddle the range while a central enforcer sits
/// inside it: only the aligned central-enforcer preference can pick index 1.
fn pick_central_witness() -> PickCase {
    let central = [false, true, false, false];
    let candidates = (0..4)
        .map(|index| {
            seam_candidate(
                [index as f32, 0.0, 0.0],
                EnforcedBlockedSeamPoint::Enforced,
                central[index],
                1.0,
            )
        })
        .collect();
    PickCase {
        class: PickClass::CentralAligned,
        candidates,
        range: 0..3,
    }
}

/// A single varying penalty term: the smallest `visibility` is obviously best.
fn pick_visibility_witness() -> PickCase {
    let visibility = [2.0_f32, 0.5, 1.5];
    let candidates = (0..3)
        .map(|index| {
            seam_candidate(
                [index as f32, 0.0, 0.0],
                EnforcedBlockedSeamPoint::Neutral,
                false,
                visibility[index],
            )
        })
        .collect();
    PickCase {
        class: PickClass::VisibilityNearest,
        candidates,
        range: 0..3,
    }
}

/// The rear setup's Y gate: the largest Y is obviously best.
fn pick_rear_y_witness() -> PickCase {
    let y = [0.0_f32, 2.0, 1.0];
    let candidates = (0..3)
        .map(|index| {
            seam_candidate(
                [index as f32, y[index], 0.0],
                EnforcedBlockedSeamPoint::Neutral,
                false,
                1.0,
            )
        })
        .collect();
    PickCase {
        class: PickClass::RearY,
        candidates,
        range: 0..3,
    }
}

/// A finite nondegenerate loop of equal-quality candidates for the dormant
/// random sibling: all four edges differ in direction, so the interpolated
/// point must sit on exactly one of them.
fn random_loop_witness() -> (Vec<SeamCandidate>, Range<usize>) {
    let candidates = vec![
        seam_candidate(
            [0.0, 0.0, 0.0],
            EnforcedBlockedSeamPoint::Neutral,
            false,
            1.0,
        ),
        seam_candidate(
            [4.0, 0.0, 0.0],
            EnforcedBlockedSeamPoint::Neutral,
            false,
            1.0,
        ),
        seam_candidate(
            [4.0, 3.0, 0.0],
            EnforcedBlockedSeamPoint::Neutral,
            false,
            1.0,
        ),
        seam_candidate(
            [0.0, 3.0, 0.0],
            EnforcedBlockedSeamPoint::Neutral,
            false,
            1.0,
        ),
    ];
    (candidates, 0..4)
}

#[derive(Debug, Default, Clone, Copy)]
struct PickWitness {
    checked: u64,
    paint_ranges: u64,
    central_ranges: u64,
    visibility_ranges: u64,
    rear_y_ranges: u64,
    ties: u64,
    random_observations: u64,
    empty_ranges: u64,
}

/// Every deterministic pick witness through the shared checker. Used by the
/// property's callback (so the positive subcase counters cannot depend on a
/// random class appearing) and by the witness/control test.
fn pick_witness_observations(limits: &Limits) -> Result<PickWitness, String> {
    let mut out = PickWitness::default();

    let singleton = pick_singleton_witness();
    let comparator = SeamComparator::new(singleton.class.setup());
    let chosen = pick_seam_point(&singleton.candidates, singleton.range.clone(), &comparator);
    check_pick(&singleton, limits, chosen).map_err(|e| format!("singleton pick witness: {e}"))?;
    if chosen != 1 {
        return Err(format!("nonzero-start singleton chose {chosen}"));
    }
    out.checked += 1;

    let mixed = pick_mixed_paint_witness();
    let comparator = SeamComparator::new(mixed.class.setup());
    let chosen = pick_seam_point(&mixed.candidates, mixed.range.clone(), &comparator);
    let stats =
        check_pick(&mixed, limits, chosen).map_err(|e| format!("mixed paint pick witness: {e}"))?;
    if !stats.paint_mix || chosen != 2 {
        return Err(format!(
            "mixed paint witness chose {chosen} (mix={})",
            stats.paint_mix
        ));
    }
    out.checked += 1;
    out.paint_ranges += 1;

    let central = pick_central_witness();
    let comparator = SeamComparator::new(central.class.setup());
    let chosen = pick_seam_point(&central.candidates, central.range.clone(), &comparator);
    check_pick(&central, limits, chosen).map_err(|e| format!("central pick witness: {e}"))?;
    if chosen != 1 {
        return Err(format!(
            "the aligned central enforcer was not chosen: {chosen}"
        ));
    }
    out.checked += 1;
    out.central_ranges += 1;

    let visibility = pick_visibility_witness();
    let comparator = SeamComparator::new(visibility.class.setup());
    let chosen = pick_seam_point(
        &visibility.candidates,
        visibility.range.clone(),
        &comparator,
    );
    check_pick(&visibility, limits, chosen).map_err(|e| format!("visibility pick witness: {e}"))?;
    if chosen != 1 {
        return Err(format!(
            "the least-visible candidate was not chosen: {chosen}"
        ));
    }
    out.checked += 1;
    out.visibility_ranges += 1;

    let rear_y = pick_rear_y_witness();
    let comparator = SeamComparator::new(rear_y.class.setup());
    let chosen = pick_seam_point(&rear_y.candidates, rear_y.range.clone(), &comparator);
    check_pick(&rear_y, limits, chosen).map_err(|e| format!("rear-Y pick witness: {e}"))?;
    if chosen != 1 {
        return Err(format!("the rearmost candidate was not chosen: {chosen}"));
    }
    out.checked += 1;
    out.rear_y_ranges += 1;

    let tied = pick_exact_tie_witness();
    let comparator = SeamComparator::new(tied.class.setup());
    let chosen = pick_seam_point(&tied.candidates, tied.range.clone(), &comparator);
    check_pick(&tied, limits, chosen).map_err(|e| format!("tie pick witness: {e}"))?;
    if chosen != tied.range.start {
        return Err(format!(
            "strict tie chose {chosen} instead of the first index {}",
            tied.range.start
        ));
    }
    out.checked += 1;
    out.ties += 1;

    // The dormant random sibling: reproducible membership and an interpolated
    // point on the selected edge. Not a distribution or seam-quality claim.
    let (loop_candidates, loop_range) = random_loop_witness();
    let observed = observe_random(&loop_candidates, loop_range.clone());
    check_random(&loop_candidates, loop_range.clone(), observed)
        .map_err(|e| format!("random pick witness: {e}"))?;
    out.checked += 1;
    out.random_observations += 1;

    // Empty range: the kernel returns its unchecked `start`, which is not an
    // `Option` and is not a candidate. Assert the value only; never index it.
    let empty = pick_singleton_witness();
    let start = empty.candidates.len();
    let comparator = SeamComparator::new(empty.class.setup());
    let unchecked = pick_seam_point(&empty.candidates, start..start, &comparator);
    if unchecked != start {
        return Err(format!(
            "empty range returned {unchecked} instead of its unchecked start {start}"
        ));
    }
    out.empty_ranges += 1;

    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SEAM-PLANNER-PAINT: live candidate generation and selection
// ─────────────────────────────────────────────────────────────────────────────

/// Per-contour-vertex paint intent: `0` neutral, `1` blocked, `2` enforced.
type PaintIntent = u8;

const PAINT_RAW_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq)]
struct PaintCase {
    contour: Vec<Point2>,
    hole: Vec<Point2>,
    intent: Vec<PaintIntent>,
    scoring_width_mm: f32,
    z: f32,
    layer_index: u32,
    mode: SeamPlannerMode,
}

const PLANNER_MODES: [SeamPlannerMode; 5] = [
    SeamPlannerMode::Nearest,
    SeamPlannerMode::Rear,
    SeamPlannerMode::Random,
    SeamPlannerMode::Aligned,
    SeamPlannerMode::AlignedBack,
];

#[derive(Debug, Clone, Copy, PartialEq)]
struct ExpectedCandidate {
    score: f32,
    enforced: bool,
}

impl PaintCase {
    fn vertex_points(&self) -> Vec<(f32, f32)> {
        self.contour
            .iter()
            .chain(self.hole.iter())
            .map(|point| (units_to_mm(point.x), units_to_mm(point.y)))
            .collect()
    }

    fn effective_width(&self) -> f32 {
        if self.scoring_width_mm.is_finite() && self.scoring_width_mm > 0.0 {
            self.scoring_width_mm
        } else {
            DEFAULT_SCORING_WIDTH_MM
        }
    }

    /// `Some` per vertex in contour-then-hole order; `None` for a blocked
    /// contour vertex, which contributes no candidate.
    fn expected_slots(&self) -> Vec<Option<ExpectedCandidate>> {
        let mut slots = Vec::with_capacity(self.contour.len() + self.hole.len());
        let mut index = 0;
        while index < self.intent.len() {
            match self.intent[index] {
                1 => {
                    slots.push(None);
                    index += 1;
                }
                2 => {
                    let start = index;
                    while index < self.intent.len() && self.intent[index] == 2 {
                        index += 1;
                    }
                    let run = index - start;
                    let central = central_prefix(run);
                    for offset in 0..run {
                        slots.push(Some(ExpectedCandidate {
                            score: if offset < central { 2.0 } else { 1.0 },
                            enforced: true,
                        }));
                    }
                }
                _ => {
                    slots.push(Some(ExpectedCandidate {
                        score: 0.0,
                        enforced: false,
                    }));
                    index += 1;
                }
            }
        }
        for _ in &self.hole {
            slots.push(Some(ExpectedCandidate {
                score: 0.0,
                enforced: false,
            }));
        }
        slots
    }

    fn expected(&self) -> Vec<ExpectedCandidate> {
        self.expected_slots().into_iter().flatten().collect()
    }

    fn expected_winner(&self) -> Option<usize> {
        let expected = self.expected();
        if expected.is_empty() {
            return None;
        }
        let max_score = expected
            .iter()
            .map(|slot| slot.score)
            .fold(f32::NEG_INFINITY, f32::max);
        let eligible: Vec<usize> = expected
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| (slot.score == max_score).then_some(index))
            .collect();
        let points = self.candidate_points();
        let min_by = |best: usize, i: usize| {
            let (bx, by) = points[best];
            let (ix, iy) = points[i];
            if iy < by || (iy == by && ix < bx) {
                i
            } else {
                best
            }
        };
        let max_by = |best: usize, i: usize| {
            let (bx, by) = points[best];
            let (ix, iy) = points[i];
            if iy > by || (iy == by && ix <= bx) {
                i
            } else {
                best
            }
        };
        match self.mode {
            SeamPlannerMode::Nearest | SeamPlannerMode::Aligned => {
                Some(eligible.iter().copied().reduce(min_by).expect("eligible"))
            }
            SeamPlannerMode::Rear | SeamPlannerMode::AlignedBack => {
                Some(eligible.iter().copied().reduce(max_by).expect("eligible"))
            }
            SeamPlannerMode::Random => Some(eligible[self.layer_index as usize % eligible.len()]),
        }
    }

    /// Candidate positions in candidate order (blocked vertices removed).
    fn candidate_points(&self) -> Vec<(f32, f32)> {
        let mut points = Vec::with_capacity(self.contour.len() + self.hole.len());
        for (index, point) in self.contour.iter().enumerate() {
            if self.intent[index] != 1 {
                points.push((units_to_mm(point.x), units_to_mm(point.y)));
            }
        }
        for point in &self.hole {
            points.push((units_to_mm(point.x), units_to_mm(point.y)));
        }
        points
    }

    fn annotations(&self) -> Vec<(PaintSemantic, Vec<Vec<Option<PaintValue>>>)> {
        let blocked: Vec<Option<PaintValue>> = self
            .intent
            .iter()
            .map(|intent| (*intent == 1).then_some(PaintValue::Flag(true)))
            .collect();
        let enforced: Vec<Option<PaintValue>> = self
            .intent
            .iter()
            .map(|intent| (*intent == 2).then_some(PaintValue::Flag(true)))
            .collect();
        let mut annotations = Vec::new();
        if blocked.iter().any(Option::is_some) {
            annotations.push((
                PaintSemantic::Custom("seam_blocker".to_owned()),
                vec![blocked],
            ));
        }
        if enforced.iter().any(Option::is_some) {
            annotations.push((
                PaintSemantic::Custom("seam_enforcer".to_owned()),
                vec![enforced],
            ));
        }
        annotations
    }

    fn region(&self) -> SeamPlanningRegionInput {
        let polygons = if self.contour.is_empty() && self.hole.is_empty() {
            Vec::new()
        } else {
            vec![ExPolygon {
                contour: Polygon {
                    points: self.contour.clone(),
                },
                holes: if self.hole.is_empty() {
                    Vec::new()
                } else {
                    vec![Polygon {
                        points: self.hole.clone(),
                    }]
                },
            }]
        };
        // The candidate and selection kernels read only `z`, the supplied
        // polygons, the paint annotations and `scoring_width`; the region
        // identity and layer height stay at their defaults.
        SeamPlanningRegionInput {
            global_layer_index: self.layer_index,
            z: self.z,
            ex_polygons: polygons,
            segment_annotations: self.annotations(),
            scoring_width: self.scoring_width_mm,
            ..Default::default()
        }
    }
}

fn build_paint_case(
    mode: SeamPlannerMode,
    raw: [u32; PAINT_RAW_LEN],
    limits: &Limits,
) -> PaintCase {
    let max_candidates = limits.max_candidates as usize;
    let coord_cap = limits.max_coord_quanta;

    let contour_len = 1 + (raw[0] as usize) % max_candidates;
    let hole_slot = max_candidates - contour_len;
    let hole_len = if hole_slot >= 3 && raw[1] % 2 == 1 {
        3 + (raw[1] as usize / 2) % (hole_slot - 2)
    } else {
        0
    };
    let total = contour_len + hole_len;

    // Affine layout with a strictly increasing x, so every generated position
    // is distinct and inside the coordinate bound without clamping.
    let step = 1 + i64::from(raw[2]) % (coord_cap / (total as i64 + 1)).max(1);
    let max_offset = (total as i64 - 1) * step;
    let room = (coord_cap - max_offset + 1).max(1);
    let base_x = i64::from(raw[3]) % room;
    let base_y = i64::from(raw[4]) % room;
    let point = |index: usize| -> Point2 {
        let offset = index as i64 * step;
        Point2::from_mm(from_quanta(base_x + offset), from_quanta(base_y + offset))
    };

    let contour: Vec<Point2> = (0..contour_len).map(point).collect();
    let hole: Vec<Point2> = (contour_len..total).map(point).collect();
    let intent: Vec<PaintIntent> = (0..contour_len)
        .map(|index| (raw[5 + index % 4] % 3) as PaintIntent)
        .collect();

    PaintCase {
        contour,
        hole,
        intent,
        scoring_width_mm: from_quanta(1 + i64::from(raw[10]) % SCORING_WIDTH_QUANTA_CAP),
        z: 0.05 * (1 + raw[9] % 200) as f32,
        layer_index: raw[11] % 256,
        mode,
    }
}

fn paint_case_strategy(limits: &Limits) -> BoxedStrategy<PaintCase> {
    let limits = *limits;
    (
        prop::sample::select(PLANNER_MODES.to_vec()).no_shrink(),
        any::<[u32; PAINT_RAW_LEN]>(),
    )
        .prop_map(move |(mode, raw)| build_paint_case(mode, raw, &limits))
        .boxed()
}

#[derive(Debug, Default, Clone, Copy)]
struct PaintStats {
    candidates: usize,
    enforced: usize,
    hole_neutrals: usize,
    eligible: usize,
    max_score: f32,
    picked_score: f32,
}

fn validate_paint_case(case: &PaintCase, limits: &Limits) -> Result<(), CheckFailure> {
    let fixture = |m: String| Err(CheckFailure::Fixture(m));
    if case.intent.len() != case.contour.len() {
        return fixture(format!(
            "{} intents for {} contour vertices",
            case.intent.len(),
            case.contour.len()
        ));
    }
    let total = case.contour.len() + case.hole.len();
    if total > limits.max_candidates as usize {
        return fixture(format!(
            "{total} vertices exceed max_candidates {}",
            limits.max_candidates
        ));
    }
    if !case.hole.is_empty() && case.hole.len() < 3 {
        return fixture(format!("{} hole vertices", case.hole.len()));
    }
    if case.intent.iter().any(|intent| *intent > 2) {
        return fixture("intent outside 0..=2".to_owned());
    }
    let coord_cap = from_quanta(limits.max_coord_quanta);
    let points = case.vertex_points();
    let mut seen = Vec::with_capacity(points.len());
    for candidate in &points {
        if candidate.0.abs() > coord_cap || candidate.1.abs() > coord_cap {
            return fixture(format!("vertex {candidate:?} exceeds the coord bound"));
        }
        if seen.contains(candidate) {
            return fixture(format!("duplicate vertex {candidate:?}"));
        }
        seen.push(*candidate);
    }
    if !case.z.is_finite() || case.z < 0.0 {
        return fixture(format!("z {}", case.z));
    }
    if !case.scoring_width_mm.is_finite() || case.scoring_width_mm > 4.0 {
        return fixture(format!("scoring width {}", case.scoring_width_mm));
    }
    Ok(())
}

fn check_paint(
    case: &PaintCase,
    limits: &Limits,
    candidates: &[ScoredSeamCandidate],
    chosen: Option<&ScoredSeamCandidate>,
) -> Result<PaintStats, CheckFailure> {
    validate_paint_case(case, limits)?;

    let slots = case.expected_slots();
    let vertex_points = case.vertex_points();
    // Blocked vertices must be *removed*, not merely deprioritised: a surviving
    // candidate at a blocked vertex's own coordinate is a distinct failure from
    // a shifted table.
    for candidate in candidates {
        for (slot, point) in slots.iter().zip(&vertex_points) {
            let at_vertex = candidate.position.x.to_bits() == point.0.to_bits()
                && candidate.position.y.to_bits() == point.1.to_bits();
            if at_vertex && slot.is_none() {
                return Err(CheckFailure::BlockedPresent(format!(
                    "candidate at ({}, {}) mm is a blocked contour vertex",
                    point.0, point.1
                )));
            }
        }
    }

    let expected: Vec<ExpectedCandidate> = slots.into_iter().flatten().collect();
    if candidates.len() != expected.len() {
        return Err(CheckFailure::Table(format!(
            "{} candidates for {} expected (blocked vertices already removed)",
            candidates.len(),
            expected.len()
        )));
    }

    // Candidate order is the same contour-then-hole vertex order with the
    // blocked vertices dropped.
    let candidate_points = case.candidate_points();
    let width = case.effective_width();
    let mut stats = PaintStats {
        candidates: candidates.len(),
        hole_neutrals: case.hole.len(),
        max_score: f32::NEG_INFINITY,
        ..PaintStats::default()
    };
    for (index, (candidate, want)) in candidates.iter().zip(&expected).enumerate() {
        let (x, y) = candidate_points[index];
        if candidate.position.x.to_bits() != x.to_bits()
            || candidate.position.y.to_bits() != y.to_bits()
        {
            return Err(CheckFailure::Table(format!(
                "candidate {index} at ({}, {}) mm, expected ({x}, {y}) mm",
                candidate.position.x, candidate.position.y
            )));
        }
        if candidate.position.z.to_bits() != case.z.to_bits() {
            return Err(CheckFailure::Table(format!(
                "candidate {index} z {} differs from the supplied {}",
                candidate.position.z, case.z
            )));
        }
        if candidate.position.width.to_bits() != width.to_bits() {
            return Err(CheckFailure::Table(format!(
                "candidate {index} width {} differs from the expected {width}",
                candidate.position.width
            )));
        }
        if candidate.position.flow_factor.to_bits() != 1.0_f32.to_bits()
            || candidate.position.overhang_quartile.is_some()
            || candidate.position.overhang_distance_mm.is_some()
            || candidate.position.dist_to_top_mm.to_bits() != 0.0_f32.to_bits()
        {
            return Err(CheckFailure::Table(format!(
                "candidate {index} carries an unspecified metadata field"
            )));
        }
        if candidate.score.to_bits() != want.score.to_bits() {
            return Err(CheckFailure::Table(format!(
                "candidate {index} score {} differs from the expected {}",
                candidate.score, want.score
            )));
        }
        let tag = if want.enforced { "enforced" } else { "aligned" };
        if candidate.reason.tag != tag {
            return Err(CheckFailure::Table(format!(
                "candidate {index} tag `{}` differs from the expected `{tag}`",
                candidate.reason.tag
            )));
        }
        stats.max_score = stats.max_score.max(want.score);
        if want.enforced {
            stats.enforced += 1;
        }
    }

    let expected_choice = case.expected_winner();
    match (expected_choice, chosen) {
        (None, None) => {}
        (None, Some(_)) => {
            return Err(CheckFailure::Selection(
                "an empty eligible set produced a candidate".to_owned(),
            ))
        }
        (Some(expected_index), None) => {
            return Err(CheckFailure::Selection(format!(
                "eligible candidate {expected_index} was dropped"
            )))
        }
        (Some(expected_index), Some(actual)) => {
            if actual != &candidates[expected_index] {
                return Err(CheckFailure::Selection(format!(
                    "mode {:?} chose ({}, {}) instead of eligible index {expected_index}",
                    case.mode, actual.position.x, actual.position.y
                )));
            }
            stats.picked_score = actual.score;
        }
    }
    stats.eligible = expected
        .iter()
        .filter(|slot| slot.score == stats.max_score)
        .count();
    Ok(stats)
}

/// Observe the live paint path: candidate generation plus the mode's choice,
/// with the `Random` repeat-call determinism pin. Nothing here supplies the
/// checker's expected answer.
fn observe_paint(case: &PaintCase) -> (Vec<ScoredSeamCandidate>, Option<ScoredSeamCandidate>) {
    let candidates = region_candidates(&case.region());
    let chosen = choose_region_candidate(&candidates, case.mode, case.layer_index);
    if case.mode == SeamPlannerMode::Random {
        // test-quality: determinism pin for the layer-index-seeded modulo
        // choice; independent correctness is the expected index in `check_paint`.
        let repeat = choose_region_candidate(&candidates, case.mode, case.layer_index);
        assert_eq!(
            repeat, chosen,
            "Random choice at layer {} must repeat",
            case.layer_index
        );
    }
    (candidates, chosen)
}

fn paint_complexity(case: &PaintCase) -> i128 {
    let mut total = (case.contour.len() + case.hole.len()) as i128;
    for point in case.contour.iter().chain(&case.hole) {
        total += i128::from(point.x.abs() + point.y.abs());
    }
    total += case
        .intent
        .iter()
        .map(|intent| i128::from(*intent))
        .sum::<i128>();
    total += i128::from((case.scoring_width_mm * QUANTA_PER_UNIT).round() as i64);
    total += i128::from((case.z * QUANTA_PER_UNIT).round() as i64);
    total + i128::from(case.layer_index)
}

// ─────────────────────────────────────────────────────────────────────────────
// Paint witnesses
// ─────────────────────────────────────────────────────────────────────────────

fn paint_case(
    mode: SeamPlannerMode,
    contour_mm: &[(f32, f32)],
    hole_mm: &[(f32, f32)],
    intent: &[PaintIntent],
    scoring_width_mm: f32,
    layer_index: u32,
) -> PaintCase {
    assert_eq!(
        contour_mm.len(),
        intent.len(),
        "a paint witness needs one intent per contour vertex"
    );
    PaintCase {
        contour: contour_mm
            .iter()
            .map(|(x, y)| Point2::from_mm(*x, *y))
            .collect(),
        hole: hole_mm
            .iter()
            .map(|(x, y)| Point2::from_mm(*x, *y))
            .collect(),
        intent: intent.to_vec(),
        scoring_width_mm,
        z: 0.2,
        layer_index,
        mode,
    }
}

/// No paint at all: every vertex is a `0.0` "aligned" candidate.
fn paint_no_paint_witness() -> PaintCase {
    paint_case(
        SeamPlannerMode::Nearest,
        &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)],
        &[],
        &[0, 0, 0, 0],
        0.6,
        0,
    )
}

/// One lone enforcer at the geometrically worst corner: paint outranks the
/// aligned geometric preference.
fn paint_enforcer_vs_neutral_witness() -> PaintCase {
    paint_case(
        SeamPlannerMode::Aligned,
        &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (5.0, 5.0)],
        &[],
        &[0, 0, 0, 2],
        0.6,
        0,
    )
}

/// A blocked vertex at the geometric winner and one enforced vertex elsewhere:
/// blocking removes the vertex, enforcement then wins the remainder.
fn paint_blocked_vs_enforced_witness() -> PaintCase {
    paint_case(
        SeamPlannerMode::Aligned,
        &[(0.0, 0.0), (5.0, 5.0), (1.0, 0.0), (2.0, 0.0)],
        &[],
        &[1, 2, 0, 0],
        0.6,
        0,
    )
}

/// A four-vertex enforced run: its first two vertices are central (`2.0`), the
/// remaining two score `1.0`.
fn paint_enforcer_run_witness() -> PaintCase {
    let contour: Vec<(f32, f32)> = (0..7).map(|i| (i as f32, 0.0)).collect();
    paint_case(
        SeamPlannerMode::Aligned,
        &contour,
        &[],
        &[0, 2, 2, 2, 2, 0, 0],
        0.6,
        0,
    )
}

/// A single-vertex contour: exactly one candidate for every mode.
fn paint_singleton_witness(mode: SeamPlannerMode) -> PaintCase {
    paint_case(mode, &[(3.0, 3.0)], &[], &[0], 0.6, 7)
}

/// A contour plus a hole: the hole's vertices are neutral candidates.
fn paint_hole_neutral_witness() -> PaintCase {
    paint_case(
        SeamPlannerMode::Aligned,
        &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (0.0, 2.0)],
        &[(6.0, 6.0), (7.0, 6.0), (7.0, 7.0)],
        &[2, 0, 0, 0],
        0.6,
        0,
    )
}

#[derive(Debug, Default, Clone, Copy)]
struct PaintWitness {
    checked: u64,
    painted_selections: u64,
    blocked_removals: u64,
    no_paint_selections: u64,
    singleton_selections: u64,
    hole_neutrals: u64,
    empty_selections: u64,
    random_cycles: u64,
    run_lengths: u64,
    width_fallbacks: u64,
}

fn paint_witness_observations(limits: &Limits) -> Result<PaintWitness, String> {
    let mut out = PaintWitness::default();
    let checked = |case: &PaintCase| -> Result<PaintStats, String> {
        let (candidates, chosen) = observe_paint(case);
        check_paint(case, limits, &candidates, chosen.as_ref())
            .map_err(|e| format!("{:?} paint witness: {e}", case.mode))
    };

    let stats = checked(&paint_no_paint_witness())?;
    if stats.candidates != 4 || stats.enforced != 0 || stats.picked_score != 0.0 {
        return Err(format!("no-paint witness: {stats:?}"));
    }
    out.checked += 1;
    out.no_paint_selections += 1;

    let stats = checked(&paint_enforcer_vs_neutral_witness())?;
    if stats.enforced != 1 || stats.max_score != 2.0 || stats.picked_score != 2.0 {
        return Err(format!("enforcer-vs-neutral witness: {stats:?}"));
    }
    out.checked += 1;
    out.painted_selections += 1;

    let blocked = paint_blocked_vs_enforced_witness();
    let (candidates, chosen) = observe_paint(&blocked);
    let stats = check_paint(&blocked, limits, &candidates, chosen.as_ref())
        .map_err(|e| format!("blocked-vs-enforced witness: {e}"))?;
    if stats.candidates != 3 {
        return Err(format!("blocked vertex was not removed: {stats:?}"));
    }
    if stats.picked_score != 2.0 {
        return Err(format!("the enforced vertex did not win: {stats:?}"));
    }
    out.checked += 1;
    out.blocked_removals += 1;
    out.painted_selections += 1;

    let stats = checked(&paint_enforcer_run_witness())?;
    if stats.enforced != 4 || stats.picked_score != 2.0 {
        return Err(format!("enforcer-run witness: {stats:?}"));
    }
    out.checked += 1;
    out.painted_selections += 1;

    for mode in PLANNER_MODES {
        let stats = checked(&paint_singleton_witness(mode))?;
        if stats.candidates != 1 || stats.eligible != 1 || stats.picked_score != 0.0 {
            return Err(format!("singleton witness for {mode:?}: {stats:?}"));
        }
        out.checked += 1;
        out.singleton_selections += 1;
    }

    let stats = checked(&paint_hole_neutral_witness())?;
    if stats.hole_neutrals != 3 || stats.candidates != 7 {
        return Err(format!("hole-neutral witness: {stats:?}"));
    }
    out.checked += 1;
    out.hole_neutrals += stats.hole_neutrals as u64;

    // Empty region and empty candidate list are exact, separate cases.
    let empty_region = PaintCase {
        contour: Vec::new(),
        hole: Vec::new(),
        intent: Vec::new(),
        scoring_width_mm: 0.4,
        z: 0.2,
        layer_index: 0,
        mode: SeamPlannerMode::Aligned,
    };
    let (candidates, chosen) = observe_paint(&empty_region);
    let stats = check_paint(&empty_region, limits, &candidates, chosen.as_ref())
        .map_err(|e| format!("empty region witness: {e}"))?;
    if stats.candidates != 0 {
        return Err(format!("an empty region produced {stats:?}"));
    }
    for mode in PLANNER_MODES {
        if choose_region_candidate(&[], mode, 3).is_some() {
            return Err(format!("empty selection returned Some for {mode:?}"));
        }
        out.empty_selections += 1;
    }

    // Layer-index-seeded modulo choice over the eligible set only.
    let run = paint_enforcer_run_witness();
    let candidates = region_candidates(&run.region());
    for layer_index in 0..4_u32 {
        let first = choose_region_candidate(&candidates, SeamPlannerMode::Random, layer_index)
            .ok_or_else(|| format!("Random returned None at layer {layer_index}"))?;
        // test-quality: determinism pin for the layer-index-seeded choice.
        let second = choose_region_candidate(&candidates, SeamPlannerMode::Random, layer_index)
            .expect("Random repeats");
        if first != second {
            return Err(format!("Random is not repeatable at layer {layer_index}"));
        }
        if first.score != 2.0 {
            return Err(format!(
                "Random left the enforced set at layer {layer_index}: score {}",
                first.score
            ));
        }
        out.random_cycles += 1;
    }

    // The annotation -> score table across contiguous enforced run lengths.
    for run_len in 1..=7_usize {
        let contour: Vec<(f32, f32)> = (0..run_len).map(|i| (i as f32, 0.0)).collect();
        let intent = vec![2 as PaintIntent; run_len];
        let case = paint_case(SeamPlannerMode::Aligned, &contour, &[], &intent, 0.6, 0);
        let (candidates, chosen) = observe_paint(&case);
        let stats = check_paint(&case, limits, &candidates, chosen.as_ref())
            .map_err(|e| format!("run length {run_len}: {e}"))?;
        let central = central_prefix(run_len);
        let expected: Vec<f32> = (0..run_len)
            .map(|offset| if offset < central { 2.0 } else { 1.0 })
            .collect();
        let observed: Vec<f32> = candidates.iter().map(|candidate| candidate.score).collect();
        if observed != expected {
            return Err(format!(
                "run length {run_len}: scores {observed:?} instead of {expected:?}"
            ));
        }
        if stats.enforced != run_len || stats.picked_score != 2.0 {
            return Err(format!("run length {run_len}: {stats:?}"));
        }
        out.run_lengths += 1;
    }

    // Nonpositive scoring width falls back to the module's documented 0.4 mm.
    let mut zero_width = paint_no_paint_witness();
    zero_width.scoring_width_mm = 0.0;
    let (candidates, chosen) = observe_paint(&zero_width);
    check_paint(&zero_width, limits, &candidates, chosen.as_ref())
        .map_err(|e| format!("zero-width witness: {e}"))?;
    if candidates
        .iter()
        .any(|candidate| candidate.position.width.to_bits() != DEFAULT_SCORING_WIDTH_MM.to_bits())
    {
        return Err("a nonpositive scoring width did not fall back to 0.4 mm".to_owned());
    }
    out.width_fallbacks += 1;

    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SEAM-PLANNER-SPLINE: dormant restricted least-squares fit
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplineKind {
    /// Positive, strictly increasing span with constant observations.
    ConstantSpan,
    /// Equal parameters: the degenerate weighted-mean branch.
    WeightedMean,
    /// A single observation: the degenerate branch with one term.
    Single,
}

const SPLINE_KINDS: [SplineKind; 3] = [
    SplineKind::ConstantSpan,
    SplineKind::WeightedMean,
    SplineKind::Single,
];

const SPLINE_RAW_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq)]
struct SplineCase {
    kind: SplineKind,
    observation_points: Vec<f32>,
    observations: Vec<[f32; 2]>,
    weights: Vec<f32>,
    segments: usize,
    queries: Vec<f32>,
}

fn build_spline_case(kind: SplineKind, raw: [u32; SPLINE_RAW_LEN], limits: &Limits) -> SplineCase {
    let max_observations = limits.max_observations as usize;
    let max_segments = limits.max_segments as usize;
    let coord_cap = limits.max_coord_quanta;
    let weight_cap = limits.max_weight_quanta;

    let count = match kind {
        SplineKind::Single => 1,
        _ => 2 + (raw[0] as usize) % (max_observations - 1),
    };
    let step = 1 + i64::from(raw[1]) % (coord_cap / count as i64).max(1);
    let room = (coord_cap - (count as i64 - 1) * step + 1).max(1);
    let base = i64::from(raw[2]) % room;

    let observation_points: Vec<f32> = match kind {
        SplineKind::WeightedMean => vec![from_quanta(base); count],
        _ => (0..count)
            .map(|index| from_quanta(base + index as i64 * step))
            .collect(),
    };
    let observations: Vec<[f32; 2]> = match kind {
        SplineKind::ConstantSpan => {
            let value = [
                from_quanta(i64::from(raw[3]) % coord_cap),
                from_quanta(i64::from(raw[4]) % coord_cap),
            ];
            vec![value; count]
        }
        _ => (0..count)
            .map(|index| {
                [
                    from_quanta(i64::from(raw[3 + index % 3]) % coord_cap),
                    from_quanta(i64::from(raw[6 + index % 3]) % coord_cap),
                ]
            })
            .collect(),
    };
    // Weights are confined to [1, weight_cap] quanta so the derived weighted-mean
    // error bound (module header) holds with `sum(w) >= observations.len()`.
    let weight_floor = 1024;
    let weights: Vec<f32> = (0..count)
        .map(|index| {
            from_quanta(
                weight_floor + i64::from(raw[9 + index % 3]) % (weight_cap - weight_floor + 1),
            )
        })
        .collect();
    let segments = match kind {
        SplineKind::Single => 1,
        SplineKind::ConstantSpan => 1 + (raw[10] as usize) % (count - 1).min(max_segments).max(1),
        SplineKind::WeightedMean => 1 + (raw[10] as usize) % count.min(max_segments).max(1),
    };
    let queries: Vec<f32> = match kind {
        SplineKind::ConstantSpan => {
            let mut queries = observation_points.clone();
            for pair in observation_points.windows(2) {
                queries.push(0.5 * (pair[0] + pair[1]));
            }
            queries
        }
        _ => vec![observation_points[0]],
    };

    SplineCase {
        kind,
        observation_points,
        observations,
        weights,
        segments,
        queries,
    }
}

fn spline_case_strategy(limits: &Limits) -> BoxedStrategy<SplineCase> {
    let limits = *limits;
    (
        prop::sample::select(SPLINE_KINDS.to_vec()).no_shrink(),
        any::<[u32; SPLINE_RAW_LEN]>(),
    )
        .prop_map(move |(kind, raw)| build_spline_case(kind, raw, &limits))
        .boxed()
}

/// The independent weighted mean, accumulated in f64 from the f32 inputs.
fn weighted_mean(observations: &[[f32; 2]], weights: &[f32]) -> [f64; 2] {
    let mut sum = [0.0_f64; 2];
    let mut weight_sum = 0.0_f64;
    for (observation, weight) in observations.iter().zip(weights) {
        let weight = f64::from(*weight);
        weight_sum += weight;
        sum[0] += f64::from(observation[0]) * weight;
        sum[1] += f64::from(observation[1]) * weight;
    }
    [sum[0] / weight_sum, sum[1] / weight_sum]
}

/// The expected fitted value for every query of a generated/witness case.
fn spline_expected(case: &SplineCase) -> [f64; 2] {
    match case.kind {
        SplineKind::ConstantSpan => [
            f64::from(case.observations[0][0]),
            f64::from(case.observations[0][1]),
        ],
        SplineKind::WeightedMean | SplineKind::Single => {
            weighted_mean(&case.observations, &case.weights)
        }
    }
}

/// Structural validity of a fit input, independent of any profile bound.
fn validate_spline_shape(case: &SplineCase) -> Result<(), CheckFailure> {
    let fixture = |m: String| Err(CheckFailure::Fixture(m));
    let count = case.observation_points.len();
    if count == 0 {
        return fixture("the fit domain has no empty-array input".to_owned());
    }
    if case.observations.len() != count || case.weights.len() != count {
        return fixture(format!(
            "unequal fit arrays: {} parameters, {} observations, {} weights",
            count,
            case.observations.len(),
            case.weights.len()
        ));
    }
    if case.segments == 0 || case.segments > count {
        return fixture(format!(
            "{} segments for {count} observations is outside 1..=len",
            case.segments
        ));
    }
    if case.queries.is_empty() {
        return fixture("no query parameters".to_owned());
    }
    if !case
        .observation_points
        .iter()
        .all(|value| value.is_finite())
        || !case.queries.iter().all(|value| value.is_finite())
    {
        return fixture("nonfinite parameter".to_owned());
    }
    // Strictly increasing for a positive-span family; the weighted-mean family
    // is the equal-parameter degeneracy and is checked as all-equal below.
    if case.kind != SplineKind::WeightedMean {
        for pair in case.observation_points.windows(2) {
            if pair[1] <= pair[0] {
                return fixture(format!(
                    "observation parameters are not sorted ascending: {} then {}",
                    pair[0], pair[1]
                ));
            }
        }
    }
    for observation in &case.observations {
        if !observation.iter().all(|value| value.is_finite()) {
            return fixture(format!("nonfinite observation {observation:?}"));
        }
        if observation[0].abs() > FIT_ABS_CAP_MM || observation[1].abs() > FIT_ABS_CAP_MM {
            return fixture(format!(
                "observation {observation:?} outside the fit envelope"
            ));
        }
    }
    for weight in &case.weights {
        if !weight.is_finite() || *weight <= 0.0 {
            return fixture(format!("nonpositive/nonfinite weight {weight}"));
        }
    }
    match case.kind {
        SplineKind::ConstantSpan => {
            if count < 2 {
                return fixture("a positive-span family needs two observations".to_owned());
            }
            // Unique least squares needs at most `count - 1` segments; the
            // SUT's wider `segments <= count` domain is not claimed here.
            if case.segments + 1 > count {
                return fixture(format!(
                    "{} segments is not well conditioned for {count} observations",
                    case.segments
                ));
            }
            let first = case.observations[0];
            if case
                .observations
                .iter()
                .any(|observation| observation != &first)
            {
                return fixture("a constant-data family needs one observation value".to_owned());
            }
        }
        SplineKind::WeightedMean => {
            if count < 2 {
                return fixture("the weighted mean needs two observations".to_owned());
            }
            let first = case.observation_points[0];
            if case
                .observation_points
                .iter()
                .any(|parameter| parameter != &first)
            {
                return fixture("the weighted-mean family needs equal parameters".to_owned());
            }
        }
        SplineKind::Single => {
            if count != 1 || case.segments != 1 {
                return fixture(format!(
                    "a single-observation fit needs len 1 and segments 1, got {count}/{}",
                    case.segments
                ));
            }
        }
    }
    Ok(())
}

/// The live-profile magnitude envelope, checked for generated cases only (the
/// packet's 10 mm weighted-mean witness is above the 8 mm generated bound).
fn validate_spline_bounds(case: &SplineCase, limits: &Limits) -> Result<(), CheckFailure> {
    let fixture = |m: String| Err(CheckFailure::Fixture(m));
    if case.observation_points.len() > limits.max_observations as usize {
        return fixture(format!(
            "{} observations exceed max_observations {}",
            case.observation_points.len(),
            limits.max_observations
        ));
    }
    if case.segments > limits.max_segments as usize {
        return fixture(format!(
            "{} segments exceed max_segments {}",
            case.segments, limits.max_segments
        ));
    }
    let coord_cap = from_quanta(limits.max_coord_quanta);
    let weight_cap = from_quanta(limits.max_weight_quanta);
    for value in case
        .observation_points
        .iter()
        .chain(case.observations.iter().flat_map(|pair| pair.iter()))
    {
        if value.abs() > coord_cap {
            return fixture(format!("magnitude {value} exceeds max_coord_quanta"));
        }
    }
    if case.weights.iter().any(|weight| *weight > weight_cap) {
        return fixture("a weight exceeds max_weight_quanta".to_owned());
    }
    Ok(())
}

fn check_spline(case: &SplineCase, observed: &[[f32; 2]]) -> Result<(), CheckFailure> {
    validate_spline_shape(case)?;
    if observed.len() != case.queries.len() {
        return Err(CheckFailure::Fixture(format!(
            "{} observed values for {} queries",
            observed.len(),
            case.queries.len()
        )));
    }
    let expected = spline_expected(case);
    for (index, value) in observed.iter().enumerate() {
        if !value.iter().all(|component| component.is_finite()) {
            return Err(CheckFailure::NonFinite(format!(
                "query {} observed {value:?}",
                case.queries[index]
            )));
        }
        for axis in 0..2 {
            let error = (f64::from(value[axis]) - expected[axis]).abs();
            if error > f64::from(SPLINE_TOL_MM) {
                return Err(CheckFailure::Envelope(format!(
                    "query {} axis {axis} observed {} vs expected {} (error {error} > {SPLINE_TOL_MM})",
                    case.queries[index], value[axis], expected[axis]
                )));
            }
        }
    }
    Ok(())
}

/// One SUT fit over every query parameter of the case.
fn observe_spline(case: &SplineCase) -> Vec<[f32; 2]> {
    let fit = fit_cubic_bspline(
        &case.observation_points,
        &case.observations,
        &case.weights,
        case.segments,
    );
    case.queries
        .iter()
        .map(|parameter| fit.get_fitted_value(*parameter))
        .collect()
}

fn spline_complexity(case: &SplineCase) -> i128 {
    let mut total = case.observation_points.len() as i128 + case.segments as i128;
    for value in case
        .observation_points
        .iter()
        .chain(case.queries.iter())
        .chain(case.observations.iter().flat_map(|pair| pair.iter()))
    {
        total += i128::from((value.abs() * QUANTA_PER_UNIT).round() as i64);
    }
    for weight in &case.weights {
        total += i128::from((weight.abs() * QUANTA_PER_UNIT).round() as i64);
    }
    total
}

// ─────────────────────────────────────────────────────────────────────────────
// Spline witnesses
// ─────────────────────────────────────────────────────────────────────────────

/// Assert a fitted value against an independently derived expectation using the
/// derived `SPLINE_TOL_MM` envelope (never bit equality: `get_fitted_value`
/// re-applies the basis in f32 after an f64 solve).
fn assert_fitted_near(actual: &[f32; 2], expected: [f32; 2], what: &str) {
    for axis in 0..2 {
        assert!(
            (actual[axis] - expected[axis]).abs() <= SPLINE_TOL_MM,
            "{what} axis {axis}: observed {actual:?} vs expected {expected:?}"
        );
    }
}

/// Positive, well-conditioned constant-data family (4 observations, 2 segments,
/// so the weighted least-squares solution is unique), queried at every observed
/// parameter and at the interior midpoints.
fn spline_constant_witness() -> SplineCase {
    SplineCase {
        kind: SplineKind::ConstantSpan,
        observation_points: vec![0.0, 0.5, 1.0, 1.5],
        observations: vec![[2.0, -3.0]; 4],
        weights: vec![1.0, 3.0, 1.0, 2.0],
        segments: 2,
        queries: vec![0.0, 0.25, 0.5, 0.75, 1.0, 1.25, 1.5],
    }
}

/// The packet's same-parameter weighted-mean degeneracy: equal parameters,
/// observations `[[0,0],[10,0],[0,10]]`, weights `[1,2,1]` mean `[5,2.5]`.
fn spline_weighted_mean_witness() -> SplineCase {
    SplineCase {
        kind: SplineKind::WeightedMean,
        observation_points: vec![0.0, 0.0, 0.0],
        observations: vec![[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]],
        weights: vec![1.0, 2.0, 1.0],
        segments: 1,
        queries: vec![0.0],
    }
}

/// A single observation: the degenerate branch reduces to that observation.
fn spline_single_witness() -> SplineCase {
    SplineCase {
        kind: SplineKind::Single,
        observation_points: vec![0.75],
        observations: vec![[4.5, -2.25]],
        weights: vec![1.0],
        segments: 1,
        queries: vec![0.75],
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct SplineWitness {
    checked: u64,
    constant_observations: u64,
    interior_queries: u64,
    weighted_means: u64,
    single_observations: u64,
}

fn spline_witness_observations() -> Result<SplineWitness, String> {
    let mut out = SplineWitness::default();

    let constant = spline_constant_witness();
    let observed = observe_spline(&constant);
    check_spline(&constant, &observed).map_err(|e| format!("constant witness: {e}"))?;
    out.checked += 1;
    out.constant_observations += constant.observation_points.len() as u64;
    out.interior_queries += (constant.queries.len() - constant.observation_points.len()) as u64;

    let mean = spline_weighted_mean_witness();
    let observed = observe_spline(&mean);
    check_spline(&mean, &observed).map_err(|e| format!("weighted-mean witness: {e}"))?;
    if (observed[0][0] - 5.0).abs() > SPLINE_TOL_MM || (observed[0][1] - 2.5).abs() > SPLINE_TOL_MM
    {
        return Err(format!(
            "the [1,2,1] weighted mean is {:?} instead of [5.0, 2.5]",
            observed[0]
        ));
    }
    out.checked += 1;
    out.weighted_means += 1;

    let single = spline_single_witness();
    let observed = observe_spline(&single);
    check_spline(&single, &observed).map_err(|e| format!("single-observation witness: {e}"))?;
    if (observed[0][0] - 4.5).abs() > SPLINE_TOL_MM || (observed[0][1] + 2.25).abs() > SPLINE_TOL_MM
    {
        return Err(format!(
            "a single observation fitted to {:?} instead of itself",
            observed[0]
        ));
    }
    out.checked += 1;
    out.single_observations += 1;

    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// Negative controls: each mutation must be rejected by the same checker
// ─────────────────────────────────────────────────────────────────────────────

fn expect_pick_failure(
    case: &PickCase,
    limits: &Limits,
    chosen: usize,
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    match check_pick(case, limits, chosen) {
        Ok(_) => panic!("{what}: the mutation was accepted"),
        Err(failure) => assert!(
            matches_class(&failure),
            "{what}: rejected for the wrong reason: {failure}"
        ),
    }
}

fn expect_paint_failure(
    case: &PaintCase,
    limits: &Limits,
    candidates: &[ScoredSeamCandidate],
    chosen: Option<&ScoredSeamCandidate>,
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    match check_paint(case, limits, candidates, chosen) {
        Ok(_) => panic!("{what}: the mutation was accepted"),
        Err(failure) => assert!(
            matches_class(&failure),
            "{what}: rejected for the wrong reason: {failure}"
        ),
    }
}

fn expect_spline_failure(
    case: &SplineCase,
    observed: &[[f32; 2]],
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    match check_spline(case, observed) {
        Ok(_) => panic!("{what}: the mutation was accepted"),
        Err(failure) => assert!(
            matches_class(&failure),
            "{what}: rejected for the wrong reason: {failure}"
        ),
    }
}

fn expect_random_failure(
    candidates: &[SeamCandidate],
    range: Range<usize>,
    observed: (usize, [f32; 3]),
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    match check_random(candidates, range, observed) {
        Ok(()) => panic!("{what}: the mutation was accepted"),
        Err(failure) => assert!(
            matches_class(&failure),
            "{what}: rejected for the wrong reason: {failure}"
        ),
    }
}

fn is_out_of_range(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::OutOfRange(_))
}

fn is_not_best(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::NotBest(_))
}

fn is_off_edge(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::OffEdge(_))
}

fn is_blocked_present(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::BlockedPresent(_))
}

fn is_table(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Table(_))
}

fn is_selection(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Selection(_))
}

fn is_non_finite(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::NonFinite(_))
}

fn is_envelope(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Envelope(_))
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink probe
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct ShrinkReport {
    simplified: usize,
    complicated: usize,
    restores: usize,
    initial_complexity: i128,
    minimal_complexity: i128,
}

/// Drive a real strategy's `ValueTree` through both transitions. Every current
/// value is re-validated by the caller's closure against the initial value
/// (selected mode, domain validity and the class witness), then phase 1 accepts
/// every simplification and phase 2 alternates simplify/complicate.
fn shrink_probe<S, C, V>(label: &str, strategy: &S, complexity: C, mut validate: V) -> ShrinkReport
where
    S: Strategy,
    S::Value: Clone + fmt::Debug + PartialEq,
    C: Fn(&S::Value) -> i128,
    V: FnMut(&S::Value, &S::Value),
{
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    let initial = tree.current();
    validate(&initial, &initial);
    let initial_complexity = complexity(&initial);

    let mut simplified = 0_usize;
    while tree.simplify() {
        simplified += 1;
        assert!(
            simplified < MAX_SHRINK_STEPS,
            "{label}: simplify never exhausted"
        );
        validate(&tree.current(), &initial);
    }
    assert!(simplified > 0, "{label}: false-only shrink probe");
    let minimal_complexity = complexity(&tree.current());
    assert!(
        minimal_complexity < initial_complexity,
        "{label}: no permitted reduction ({initial_complexity} -> {minimal_complexity})"
    );
    let exhausted = tree.current();
    assert!(
        !tree.simplify(),
        "{label}: an exhausted tree simplified again"
    );
    assert_eq!(
        tree.current(),
        exhausted,
        "{label}: a false simplify moved the value"
    );

    let mut second_runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut second_runner).expect("shrink tree");
    assert_eq!(
        tree.current(),
        initial,
        "{label}: the deterministic start is stable"
    );
    let mut alternated = 0_usize;
    let mut complicated = 0_usize;
    let mut changed = false;
    while tree.simplify() {
        alternated += 1;
        assert!(
            alternated < MAX_SHRINK_STEPS,
            "{label}: alternation never ended"
        );
        let current = tree.current();
        validate(&current, &initial);
        changed |= current != initial;
        if tree.complicate() {
            complicated += 1;
        }
        validate(&tree.current(), &initial);
    }
    assert!(alternated > 0 && changed, "{label}: no real transition");
    let mut restores = 0_usize;
    while tree.complicate() {
        restores += 1;
        assert!(
            restores < MAX_SHRINK_STEPS,
            "{label}: complicate never ended"
        );
        validate(&tree.current(), &initial);
    }
    let settled = tree.current();
    assert!(
        !tree.complicate(),
        "{label}: complicate after exhaustion must be false"
    );
    assert_eq!(
        tree.current(),
        settled,
        "{label}: a false complicate moved the value"
    );

    ShrinkReport {
        simplified,
        complicated,
        restores,
        initial_complexity,
        minimal_complexity,
    }
}

/// A persistent semantic mode selector must never shrink: a shrinking selector
/// could switch a case's class mid-search.
fn assert_selector_does_not_shrink<T: Clone + fmt::Debug + PartialEq + 'static>(
    label: &str,
    values: Vec<T>,
) {
    let mut runner = TestRunner::deterministic();
    let strategy = prop::sample::select(values).no_shrink();
    let mut tree = strategy.new_tree(&mut runner).expect("selector tree");
    let before = tree.current();
    assert!(!tree.simplify(), "{label}: the selector simplified");
    assert!(!tree.complicate(), "{label}: the selector complicated");
    assert_eq!(tree.current(), before, "{label}: the selector moved");
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SEAM-PLANNER-PICK property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_planner_pick() {
    let run = PropertyRun::new(PICK_ID, PICK_TEST).expect("M-SEAM-PLANNER-PICK context");
    let limits = live_limits(&run, &PICK_BOUND_KEYS);
    let strategy = pick_case_strategy(&limits);

    let calls = Cell::new(0_u64);
    let paint = Cell::new(0_u64);
    let central = Cell::new(0_u64);
    let score = Cell::new(0_u64);
    let rear_y = Cell::new(0_u64);
    let ties = Cell::new(0_u64);
    let sliced = Cell::new(0_u64);
    let witness_checks = Cell::new(0_u64);
    let witness_paint = Cell::new(0_u64);
    let witness_central = Cell::new(0_u64);
    let witness_visibility = Cell::new(0_u64);
    let witness_rear_y = Cell::new(0_u64);
    let witness_ties = Cell::new(0_u64);
    let random_observations = Cell::new(0_u64);
    let empty_ranges = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);

        match pick_witness_observations(&limits) {
            Ok(witness) => {
                witness_checks.set(witness_checks.get() + witness.checked);
                witness_paint.set(witness_paint.get() + witness.paint_ranges);
                witness_central.set(witness_central.get() + witness.central_ranges);
                witness_visibility.set(witness_visibility.get() + witness.visibility_ranges);
                witness_rear_y.set(witness_rear_y.get() + witness.rear_y_ranges);
                witness_ties.set(witness_ties.get() + witness.ties);
                random_observations.set(random_observations.get() + witness.random_observations);
                empty_ranges.set(empty_ranges.get() + witness.empty_ranges);
            }
            Err(message) => return fail(message),
        }

        let comparator = SeamComparator::new(case.class.setup());
        let chosen = pick_seam_point(&case.candidates, case.range.clone(), &comparator);
        let stats = match check_pick(&case, &limits, chosen) {
            Ok(stats) => stats,
            Err(failure) => return fail(format!("{} case: {failure}", case.class.label())),
        };
        if case.range.start > 0 || case.range.end < case.candidates.len() {
            sliced.set(sliced.get() + 1);
        }
        if case.class.is_paint() {
            if !stats.paint_mix {
                return fail(format!("{} case lost its paint mix", case.class.label()));
            }
            paint.set(paint.get() + 1);
        } else if case.class.is_central() {
            central.set(central.get() + 1);
        } else if case.class == PickClass::VisibilityNearest {
            score.set(score.get() + 1);
        } else if case.class == PickClass::RearY {
            rear_y.set(rear_y.get() + 1);
        } else {
            ties.set(ties.get() + 1);
        }
        Ok(())
    })
    .expect("M-SEAM-PLANNER-PICK range membership, tie order and priority");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(sliced.get() > 0, "no generated case sliced a sub-range");
    // Every required positive subcase is guaranteed by construction inside the
    // callback; the generated per-class counts below are evidence, not the
    // obligation.
    assert!(witness_checks.get() > 0, "pick witnesses unobserved");
    assert!(witness_paint.get() > 0, "mixed-paint witness unobserved");
    assert!(
        witness_central.get() > 0,
        "central-enforcer witness unobserved"
    );
    assert!(
        witness_visibility.get() > 0,
        "visibility witness unobserved"
    );
    assert!(witness_rear_y.get() > 0, "rear-Y witness unobserved");
    assert!(witness_ties.get() > 0, "tie witness unobserved");
    assert!(random_observations.get() > 0, "random sibling unobserved");
    assert!(empty_ranges.get() > 0, "empty-range case unobserved");
    println!(
        "M-SEAM-PLANNER-PICK accepted_calls={} paint={} central={} visibility={} rear_y={} \
         ties={} sub_ranges={} witnesses={} witness_paint={} witness_central={} \
         witness_visibility={} witness_rear_y={} witness_ties={} random_picks={} \
         empty_ranges={}",
        calls.get(),
        paint.get(),
        central.get(),
        score.get(),
        rear_y.get(),
        ties.get(),
        sliced.get(),
        witness_checks.get(),
        witness_paint.get(),
        witness_central.get(),
        witness_visibility.get(),
        witness_rear_y.get(),
        witness_ties.get(),
        random_observations.get(),
        empty_ranges.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SEAM-PLANNER-PAINT property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_planner_paint() {
    let run = PropertyRun::new(PAINT_ID, PAINT_TEST).expect("M-SEAM-PLANNER-PAINT context");
    let limits = live_limits(&run, &PAINT_BOUND_KEYS);
    let strategy = paint_case_strategy(&limits);

    let calls = Cell::new(0_u64);
    let painted = Cell::new(0_u64);
    let blocked = Cell::new(0_u64);
    let holed = Cell::new(0_u64);
    let random = Cell::new(0_u64);
    let singleton = Cell::new(0_u64);
    let candidates_seen = Cell::new(0_u64);
    let witness_checks = Cell::new(0_u64);
    let witness_painted = Cell::new(0_u64);
    let witness_blocked = Cell::new(0_u64);
    let witness_singletons = Cell::new(0_u64);
    let witness_holes = Cell::new(0_u64);
    let witness_empty = Cell::new(0_u64);
    let witness_random = Cell::new(0_u64);
    let witness_runs = Cell::new(0_u64);
    let witness_widths = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);

        match paint_witness_observations(&limits) {
            Ok(witness) => {
                witness_checks.set(witness_checks.get() + witness.checked);
                witness_painted.set(witness_painted.get() + witness.painted_selections);
                witness_blocked.set(witness_blocked.get() + witness.blocked_removals);
                witness_singletons.set(witness_singletons.get() + witness.singleton_selections);
                witness_holes.set(witness_holes.get() + witness.hole_neutrals);
                witness_empty.set(witness_empty.get() + witness.empty_selections);
                witness_random.set(witness_random.get() + witness.random_cycles);
                witness_runs.set(witness_runs.get() + witness.run_lengths);
                witness_widths.set(witness_widths.get() + witness.width_fallbacks);
            }
            Err(message) => return fail(message),
        }

        let (candidates, chosen) = observe_paint(&case);
        let stats = match check_paint(&case, &limits, &candidates, chosen.as_ref()) {
            Ok(stats) => stats,
            Err(failure) => return fail(format!("{:?} painted case: {failure}", case.mode)),
        };
        candidates_seen.set(candidates_seen.get() + stats.candidates as u64);
        if stats.enforced > 0 {
            painted.set(painted.get() + 1);
        }
        if case.intent.contains(&1) {
            blocked.set(blocked.get() + 1);
        }
        if !case.hole.is_empty() {
            holed.set(holed.get() + 1);
        }
        if case.mode == SeamPlannerMode::Random {
            if stats.picked_score < stats.max_score {
                return fail("Random left the maximum-score set");
            }
            random.set(random.get() + 1);
        }
        if stats.candidates == 1 {
            if stats.eligible != 1 {
                return fail("a singleton selection was not eligible");
            }
            singleton.set(singleton.get() + 1);
        }
        Ok(())
    })
    .expect("M-SEAM-PLANNER-PAINT annotation classification and mode selection");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(painted.get() > 0, "no generated painted selection");
    assert!(blocked.get() > 0, "no generated blocked vertex");
    assert!(holed.get() > 0, "no generated hole");
    assert!(random.get() > 0, "no generated Random selection");
    assert!(candidates_seen.get() > 0, "no generated candidates");
    assert!(witness_checks.get() > 0, "paint witnesses unobserved");
    assert!(witness_painted.get() > 0, "enforcer witness unobserved");
    assert!(witness_blocked.get() > 0, "blocked witness unobserved");
    assert!(witness_singletons.get() > 0, "singleton witness unobserved");
    assert!(witness_holes.get() > 0, "hole-neutral witness unobserved");
    assert!(witness_empty.get() > 0, "empty selection unobserved");
    assert!(witness_random.get() > 0, "layer-indexed Random unobserved");
    assert!(witness_runs.get() > 0, "run-length table unobserved");
    assert!(witness_widths.get() > 0, "width fallback unobserved");
    println!(
        "M-SEAM-PLANNER-PAINT accepted_calls={} painted={} blocked={} holed={} \
         random={} singleton={} candidates={} witnesses={} witness_painted={} \
         witness_blocked={} witness_singletons={} witness_holes={} witness_empty={} \
         witness_random={} witness_runs={} witness_widths={}",
        calls.get(),
        painted.get(),
        blocked.get(),
        holed.get(),
        random.get(),
        singleton.get(),
        candidates_seen.get(),
        witness_checks.get(),
        witness_painted.get(),
        witness_blocked.get(),
        witness_singletons.get(),
        witness_holes.get(),
        witness_empty.get(),
        witness_random.get(),
        witness_runs.get(),
        witness_widths.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SEAM-PLANNER-SPLINE property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_planner_spline() {
    let run = PropertyRun::new(SPLINE_ID, SPLINE_TEST).expect("M-SEAM-PLANNER-SPLINE context");
    let limits = live_limits(&run, &SPLINE_BOUND_KEYS);
    let strategy = spline_case_strategy(&limits);

    let calls = Cell::new(0_u64);
    let constant = Cell::new(0_u64);
    let weighted = Cell::new(0_u64);
    let single = Cell::new(0_u64);
    let interior = Cell::new(0_u64);
    let queries = Cell::new(0_u64);
    let witness_checks = Cell::new(0_u64);
    let witness_constant = Cell::new(0_u64);
    let witness_interior = Cell::new(0_u64);
    let witness_means = Cell::new(0_u64);
    let witness_singles = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);

        match spline_witness_observations() {
            Ok(witness) => {
                witness_checks.set(witness_checks.get() + witness.checked);
                witness_constant.set(witness_constant.get() + witness.constant_observations);
                witness_interior.set(witness_interior.get() + witness.interior_queries);
                witness_means.set(witness_means.get() + witness.weighted_means);
                witness_singles.set(witness_singles.get() + witness.single_observations);
            }
            Err(message) => return fail(message),
        }

        if let Err(failure) = validate_spline_bounds(&case, &limits) {
            return fail(format!("{:?} generated case: {failure}", case.kind));
        }
        // The fit domain has no empty-array or zero-segment input: the shape is
        // validated, never exercised with an invalid array.
        let observed = observe_spline(&case);
        if let Err(failure) = check_spline(&case, &observed) {
            return fail(format!("{:?} fitted case: {failure}", case.kind));
        }
        queries.set(queries.get() + case.queries.len() as u64);
        match case.kind {
            SplineKind::ConstantSpan => {
                constant.set(constant.get() + 1);
                interior.set(
                    interior.get() + (case.queries.len() - case.observation_points.len()) as u64,
                );
            }
            SplineKind::WeightedMean => weighted.set(weighted.get() + 1),
            SplineKind::Single => single.set(single.get() + 1),
        }
        Ok(())
    })
    .expect("M-SEAM-PLANNER-SPLINE finite constant and weighted-mean fits");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(constant.get() > 0, "no generated constant fit");
    assert!(weighted.get() > 0, "no generated weighted mean");
    assert!(single.get() > 0, "no generated single observation");
    assert!(interior.get() > 0, "no generated interior query");
    assert!(queries.get() > 0, "no fit value observed");
    assert!(witness_checks.get() > 0, "spline witnesses unobserved");
    assert!(witness_constant.get() > 0, "constant witness unobserved");
    assert!(
        witness_interior.get() > 0,
        "interior query witness unobserved"
    );
    assert!(witness_means.get() > 0, "weighted-mean witness unobserved");
    assert!(
        witness_singles.get() > 0,
        "single-observation witness unobserved"
    );
    println!(
        "M-SEAM-PLANNER-SPLINE accepted_calls={} constant={} weighted_mean={} single={} \
         interior_queries={} queries={} witnesses={} witness_constant={} \
         witness_interior={} witness_means={} witness_singles={}",
        calls.get(),
        constant.get(),
        weighted.get(),
        single.get(),
        interior.get(),
        queries.get(),
        witness_checks.get(),
        witness_constant.get(),
        witness_interior.get(),
        witness_means.get(),
        witness_singles.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// C: deterministic witnesses, negative controls and shrink transitions
// ─────────────────────────────────────────────────────────────────────────────

/// Deterministic witness for the `RearY` fixture defect found by the extended
/// tier (seed 6119898662842419507): with `len = 2`, `lo = 1`, `hi = 2` the old
/// overlay gave the in-range target and the out-of-range decoration the same Y
/// (`coord_cap`), so the class lost its Y spread and the fixture guard fired.
/// Asserts, for that exact shape and for every `(len, lo, hi)` the generator can
/// emit under two raw layouts, that the case passes the shared validator, keeps
/// a Y spread, every out-of-range decoration slot is strictly better than every
/// in-range candidate, and the real kernel's pick is accepted by `check_pick`
/// while a range-ignoring scan is rejected.
fn assert_rear_y_decoration_witness(limits: &Limits) {
    let max_len = limits.max_candidates as usize;
    let layouts: [[u32; PICK_RAW_LEN]; 2] = [
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, u32::MAX / 3, u32::MAX / 5, u32::MAX / 7, 3, 5, 7],
    ];
    for layout in layouts {
        for len in 2..=max_len {
            for lo in 0..len {
                for hi in (lo + 1)..=len {
                    let mut raw = layout;
                    raw[0] = (len - 2) as u32;
                    raw[1] = lo as u32;
                    raw[2] = (hi - lo - 1) as u32;
                    let case = build_pick_case(PickClass::RearY, raw, limits);
                    assert_eq!(
                        (case.candidates.len(), case.range.clone()),
                        (len, lo..hi),
                        "the raw vector must reproduce the requested shape"
                    );
                    validate_pick_case(&case, limits)
                        .unwrap_or_else(|e| panic!("rear-y {len}/{lo}..{hi}: {e}"));
                    let in_range_best = case.candidates[lo..hi]
                        .iter()
                        .map(|c| c.position[1])
                        .fold(f32::MIN, f32::max);
                    // Only the two decoration slots (index 0 below the range,
                    // `len - 1` above it) receive the strongest value; other
                    // out-of-range candidates keep the plain layout.
                    let decorations = [(lo > 0).then_some(0), (hi < len).then_some(len - 1)];
                    for i in decorations.into_iter().flatten() {
                        assert!(
                            case.candidates[i].position[1] > in_range_best,
                            "rear-y {len}/{lo}..{hi}: decoration {i} must be strictly better"
                        );
                    }
                    let comparator = SeamComparator::new(case.class.setup());
                    let chosen = pick_seam_point(&case.candidates, case.range.clone(), &comparator);
                    check_pick(&case, limits, chosen)
                        .unwrap_or_else(|e| panic!("rear-y {len}/{lo}..{hi}: {e}"));
                    if lo > 0 || hi < len {
                        // A scan over the whole slice (ignoring the range) picks
                        // a decoration, which the same checker rejects.
                        let ignoring = (0..len).fold(0, |best, i| {
                            if case.candidates[i].position[1] > case.candidates[best].position[1] {
                                i
                            } else {
                                best
                            }
                        });
                        assert!(ignoring < lo || ignoring >= hi);
                        expect_pick_failure(
                            &case,
                            limits,
                            ignoring,
                            "range-ignoring rear-y scan",
                            is_out_of_range,
                        );
                    }
                }
            }
        }
    }
    // The exact failing shape, named explicitly.
    let mut raw = layouts[0];
    raw[1] = 1;
    let case = build_pick_case(PickClass::RearY, raw, limits);
    assert_eq!((case.candidates.len(), case.range.clone()), (2, 1..2));
    assert_ne!(
        case.candidates[0].position[1], case.candidates[1].position[1],
        "the len=2, range 1..2 RearY case must keep its Y spread"
    );
}

#[test]
fn packet15_witnesses_controls_and_shrinking() {
    let limits = checked_limits(&probe_bound_map()).expect("probe bounds");
    assert_eq!(
        probe_bound_map().len(),
        6,
        "the probe map must carry exactly the six owner bounds"
    );

    // ── Pick witnesses, accepted by the shared checker ──────────────────────
    let singleton = pick_singleton_witness();
    let comparator = SeamComparator::new(singleton.class.setup());
    let chosen = pick_seam_point(&singleton.candidates, singleton.range.clone(), &comparator);
    check_pick(&singleton, &limits, chosen).expect("unmodified nonzero-start singleton");
    assert_eq!(
        chosen, 1,
        "a nonzero-start singleton returns its only index"
    );

    let mixed = pick_mixed_paint_witness();
    let comparator = SeamComparator::new(mixed.class.setup());
    // The enforced decoy at index 0 is outside the range and the enforced
    // vertex at index 2 is inside it, so only a range-respecting scan finds 2.
    let chosen = pick_seam_point(&mixed.candidates, mixed.range.clone(), &comparator);
    let stats = check_pick(&mixed, &limits, chosen).expect("unmodified mixed paint range");
    assert!(
        stats.paint_mix,
        "the mixed witness must contain a paint mix"
    );
    assert_eq!(chosen, 2, "the in-range enforced vertex wins");
    assert_eq!(
        expected_pick_index(&mixed),
        2,
        "the independent model agrees without the comparator"
    );

    let tied = pick_exact_tie_witness();
    let comparator = SeamComparator::new(tied.class.setup());
    let chosen = pick_seam_point(&tied.candidates, tied.range.clone(), &comparator);
    check_pick(&tied, &limits, chosen).expect("unmodified exact tie");
    assert_eq!(
        chosen, tied.range.start,
        "a strict tie retains the first index"
    );

    assert_rear_y_decoration_witness(&limits);

    // Empty range: the unchecked `start` is asserted only; it is never indexed
    // and never advertised as a valid candidate.
    let start = singleton.candidates.len();
    let unchecked = pick_seam_point(&singleton.candidates, start..start, &comparator);
    assert_eq!(
        unchecked, start,
        "an empty range returns its unchecked start"
    );

    // Dormant random sibling: reproducible membership and an on-edge point.
    let (loop_candidates, loop_range) = random_loop_witness();
    let random_observation = observe_random(&loop_candidates, loop_range.clone());
    check_random(&loop_candidates, loop_range.clone(), random_observation)
        .expect("unmodified random-sibling observation");

    // ── Paint witnesses, accepted by the shared checker ─────────────────────
    let no_paint = paint_no_paint_witness();
    let (no_paint_candidates, no_paint_chosen) = observe_paint(&no_paint);
    let stats = check_paint(
        &no_paint,
        &limits,
        &no_paint_candidates,
        no_paint_chosen.as_ref(),
    )
    .expect("unmodified no-paint region");
    assert_eq!(stats.candidates, 4, "one candidate per unpainted vertex");
    assert_eq!(stats.picked_score, 0.0, "an unpainted selection is neutral");

    let enforced = paint_enforcer_vs_neutral_witness();
    let (enforced_candidates, enforced_chosen) = observe_paint(&enforced);
    let stats = check_paint(
        &enforced,
        &limits,
        &enforced_candidates,
        enforced_chosen.as_ref(),
    )
    .expect("unmodified enforcer-vs-neutral region");
    assert_eq!(stats.max_score, 2.0, "a lone enforcer is central (score 2)");
    assert_eq!(
        stats.picked_score, 2.0,
        "paint outranks the aligned geometric preference"
    );

    let blocked = paint_blocked_vs_enforced_witness();
    let (blocked_candidates, blocked_chosen) = observe_paint(&blocked);
    let stats = check_paint(
        &blocked,
        &limits,
        &blocked_candidates,
        blocked_chosen.as_ref(),
    )
    .expect("unmodified blocked-vs-enforced region");
    assert_eq!(stats.candidates, 3, "the blocked vertex is removed");
    assert_eq!(
        stats.picked_score, 2.0,
        "the enforced vertex takes the region"
    );

    let run_witness = paint_enforcer_run_witness();
    let (run_candidates, run_chosen) = observe_paint(&run_witness);
    check_paint(&run_witness, &limits, &run_candidates, run_chosen.as_ref())
        .expect("unmodified enforcer run");
    let run_scores: Vec<f32> = run_candidates.iter().map(|c| c.score).collect();
    assert_eq!(
        run_scores,
        vec![0.0, 2.0, 2.0, 1.0, 1.0, 0.0, 0.0],
        "a four-vertex enforced run scores 2,2,1,1"
    );

    for mode in PLANNER_MODES {
        let case = paint_singleton_witness(mode);
        let (candidates, chosen) = observe_paint(&case);
        let stats = check_paint(&case, &limits, &candidates, chosen.as_ref())
            .expect("unmodified singleton region");
        assert_eq!(stats.candidates, 1, "{mode:?}: one singleton candidate");
        assert_eq!(stats.picked_score, 0.0, "{mode:?}: the singleton is picked");
    }

    let holed = paint_hole_neutral_witness();
    let (holed_candidates, holed_chosen) = observe_paint(&holed);
    let stats = check_paint(&holed, &limits, &holed_candidates, holed_chosen.as_ref())
        .expect("unmodified hole-neutral region");
    assert_eq!(stats.candidates, 7, "four contour plus three hole vertices");
    assert!(
        holed_candidates[4..]
            .iter()
            .all(|candidate| candidate.score == 0.0 && candidate.reason.tag == "aligned"),
        "hole vertices are neutral"
    );

    // Empty selection is exact and separate for every mode.
    for mode in PLANNER_MODES {
        assert!(
            choose_region_candidate(&[], mode, 0).is_none(),
            "{mode:?}: an empty selection returns None"
        );
    }
    assert!(
        region_candidates(
            &PaintCase {
                contour: Vec::new(),
                hole: Vec::new(),
                intent: Vec::new(),
                scoring_width_mm: 0.4,
                z: 0.2,
                layer_index: 0,
                mode: SeamPlannerMode::Aligned,
            }
            .region()
        )
        .is_empty(),
        "an empty region generates no candidates"
    );

    // ── Spline witnesses, accepted by the shared checker ────────────────────
    let constant = spline_constant_witness();
    let observed = observe_spline(&constant);
    check_spline(&constant, &observed).expect("unmodified constant-data family");
    for value in &observed {
        // The derived f32 envelope, not bit equality: the clamped basis sums to
        // one but the returned coefficients are f32, so one-ulp drift is
        // expected (~1.2e-7 mm here, far inside `SPLINE_TOL_MM`).
        for axis in 0..2 {
            assert!(
                (value[axis] - [2.0_f32, -3.0][axis]).abs() <= SPLINE_TOL_MM,
                "constant data drifted at axis {axis}: {value:?}"
            );
        }
    }

    let mean = spline_weighted_mean_witness();
    let mean_observed = observe_spline(&mean);
    check_spline(&mean, &mean_observed).expect("unmodified weighted-mean degeneracy");
    // [1,2,1] over [[0,0],[10,0],[0,10]]: wsum 4, x = (0*1 + 10*2 + 0*1)/4 = 5,
    // y = (0*1 + 0*2 + 10*1)/4 = 2.5. The fit *coefficients* are that mean; the
    // evaluated value re-applies the basis in f32, so one-ulp drift (4.8e-7 at
    // 5 mm) is expected and `SPLINE_TOL_MM` covers it.
    assert_fitted_near(&mean_observed[0], [5.0, 2.5], "weighted mean [5,2.5]");

    let single = spline_single_witness();
    let single_observed = observe_spline(&single);
    check_spline(&single, &single_observed).expect("unmodified single observation");
    assert_fitted_near(&single_observed[0], [4.5, -2.25], "single observation");

    // ── Mutation controls: each rejected by the same checker ────────────────
    // Pick: an index outside the scanned range.
    expect_pick_failure(
        &singleton,
        &limits,
        0,
        "pick index before the range",
        is_out_of_range,
    );
    expect_pick_failure(
        &mixed,
        &limits,
        mixed.range.end,
        "pick index at range.end",
        is_out_of_range,
    );
    // Pick: an in-range lower-priority candidate instead of the enforced one.
    expect_pick_failure(
        &mixed,
        &limits,
        1,
        "lower-priority in-range candidate",
        is_not_best,
    );
    // Random sibling: an interpolated point off the selected edge.
    let (index, point) = random_observation;
    let next = if index + 1 >= loop_range.end {
        loop_range.start
    } else {
        index + 1
    };
    let a = loop_candidates[index].position;
    let b = loop_candidates[next].position;
    let overshoot = [
        a[0] + (b[0] - a[0]) * 2.0,
        a[1] + (b[1] - a[1]) * 2.0,
        a[2] + (b[2] - a[2]) * 2.0,
    ];
    expect_random_failure(
        &loop_candidates,
        loop_range.clone(),
        (index, overshoot),
        "random point past the edge end",
        is_off_edge,
    );
    expect_random_failure(
        &loop_candidates,
        loop_range.clone(),
        (loop_range.end, point),
        "random index at range.end",
        is_out_of_range,
    );
    assert_ne!(overshoot, point, "the off-edge control must move the point");

    // Paint: the blocked vertex is retained as a candidate. The mutant clones a
    // real emitted candidate, so only the retained vertex is the defect.
    let mut retained = blocked_candidates.clone();
    let mut injected = blocked_candidates[0].clone();
    injected.position.x = units_to_mm(blocked.contour[0].x);
    injected.position.y = units_to_mm(blocked.contour[0].y);
    injected.score = 0.0;
    injected.reason.tag = "aligned".to_owned();
    retained.insert(0, injected);
    expect_paint_failure(
        &blocked,
        &limits,
        &retained,
        blocked_chosen.as_ref(),
        "retained blocked vertex",
        is_blocked_present,
    );

    // Paint: the only enforcer is demoted from 2.0 to 1.0.
    let mut demoted = enforced_candidates.clone();
    let enforcer_index = demoted
        .iter()
        .position(|candidate| candidate.score == 2.0)
        .expect("the witness has exactly one enforcer");
    demoted[enforcer_index].score = 1.0;
    expect_paint_failure(
        &enforced,
        &limits,
        &demoted,
        enforced_chosen.as_ref(),
        "demoted sole enforcer",
        is_table,
    );
    // A silent reordering of the table is a distinct table failure.
    let mut swapped = no_paint_candidates.clone();
    swapped.swap(0, 3);
    expect_paint_failure(
        &no_paint,
        &limits,
        &swapped,
        no_paint_chosen.as_ref(),
        "reordered candidate table",
        is_table,
    );
    // Paint: a selection observation that is not the model's winner.
    expect_paint_failure(
        &no_paint,
        &limits,
        &no_paint_candidates,
        Some(&no_paint_candidates[1]),
        "wrong tie/geometry winner",
        is_selection,
    );
    // Paint: an eligible candidate silently dropped.
    expect_paint_failure(
        &no_paint,
        &limits,
        &no_paint_candidates,
        None,
        "dropped selection",
        is_selection,
    );

    // Spline: the weighted mean replaced by the unweighted mean.
    let unweighted = [10.0_f32 / 3.0, 10.0_f32 / 3.0];
    expect_spline_failure(
        &mean,
        &[unweighted],
        "unweighted mean substituted for [5,2.5]",
        is_envelope,
    );
    // Spline: a non-finite fit.
    expect_spline_failure(
        &mean,
        &[[f32::NAN, 2.5]],
        "nonfinite fitted value",
        is_non_finite,
    );
    // Spline: a constant family whose fit drifts is caught by the envelope.
    expect_spline_failure(
        &constant,
        &[[2.5_f32, -3.0]; 7],
        "constant family drifted by 0.5 mm",
        is_envelope,
    );

    // ── Bound-construction rejection matrix ────────────────────────────────
    let valid = probe_bound_map();
    for key in [
        "max_candidates",
        "max_coord_quanta",
        "max_score_quanta",
        "max_observations",
        "max_segments",
        "max_weight_quanta",
    ] {
        let mut bounds = valid.clone();
        bounds.remove(key);
        assert!(
            matches!(
                checked_limits(&bounds),
                Err(LimitsError::Missing(missing)) if missing == key
            ),
            "a missing `{key}` must be rejected as missing"
        );
    }
    for (key, value) in [
        ("max_candidates", 6),
        ("max_candidates", 33),
        ("max_observations", 3),
        ("max_observations", 33),
        ("max_segments", 1),
        ("max_segments", 33),
        ("max_coord_quanta", 4095),
        ("max_coord_quanta", 65_537),
        ("max_score_quanta", 1023),
        ("max_weight_quanta", 2047),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        assert!(
            matches!(checked_limits(&bounds), Err(LimitsError::OutOfRange(_))),
            "`{key} = {value}` must be rejected as out of range"
        );
    }
    let mut inconsistent = valid.clone();
    inconsistent.insert("max_observations".to_owned(), 4);
    inconsistent.insert("max_segments".to_owned(), 5);
    assert!(matches!(
        checked_limits(&inconsistent),
        Err(LimitsError::Inconsistent(_))
    ));
    let mut wide = valid.clone();
    wide.insert("max_observations".to_owned(), u64::from(u32::MAX) + 1);
    assert!(matches!(
        checked_limits(&wide),
        Err(LimitsError::ArithmeticOverflow)
    ));

    // Re-run the aggregate witness batteries the properties call, so this test
    // covers every deterministic witness through the same checkers and reports
    // their real counts rather than a hand-maintained tally.
    let pick_witnesses = pick_witness_observations(&limits).expect("aggregate pick witnesses");
    let paint_witnesses = paint_witness_observations(&limits).expect("aggregate paint witnesses");
    let spline_witnesses = spline_witness_observations().expect("aggregate spline witnesses");
    assert!(pick_witnesses.checked > 0 && paint_witnesses.checked > 0);
    assert!(spline_witnesses.checked > 0);

    // ── Shrink transitions: pick ────────────────────────────────────────────
    let pick_report = shrink_probe(
        PICK_ID,
        &pick_case_strategy(&limits),
        pick_complexity,
        |current, initial| {
            assert_eq!(
                current.class, initial.class,
                "the pick class must survive shrinking"
            );
            validate_pick_case(current, &limits)
                .unwrap_or_else(|failure| panic!("shrunk pick fixture: {failure}"));
            let comparator = SeamComparator::new(current.class.setup());
            let chosen = pick_seam_point(&current.candidates, current.range.clone(), &comparator);
            check_pick(current, &limits, chosen)
                .unwrap_or_else(|failure| panic!("shrunk pick case: {failure}"));
        },
    );
    assert_selector_does_not_shrink("pick classes", PICK_CLASSES.to_vec());

    // ── Shrink transitions: paint ───────────────────────────────────────────
    let paint_report = shrink_probe(
        PAINT_ID,
        &paint_case_strategy(&limits),
        paint_complexity,
        |current, initial| {
            assert_eq!(
                current.mode, initial.mode,
                "the planner mode must survive shrinking"
            );
            validate_paint_case(current, &limits)
                .unwrap_or_else(|failure| panic!("shrunk paint fixture: {failure}"));
            let (candidates, chosen) = observe_paint(current);
            check_paint(current, &limits, &candidates, chosen.as_ref())
                .unwrap_or_else(|failure| panic!("shrunk paint case: {failure}"));
        },
    );
    assert_selector_does_not_shrink("planner modes", PLANNER_MODES.to_vec());

    // ── Shrink transitions: spline ──────────────────────────────────────────
    let spline_report = shrink_probe(
        SPLINE_ID,
        &spline_case_strategy(&limits),
        spline_complexity,
        |current, initial| {
            assert_eq!(
                current.kind, initial.kind,
                "the fit family must survive shrinking"
            );
            validate_spline_shape(current)
                .unwrap_or_else(|failure| panic!("shrunk spline fixture: {failure}"));
            validate_spline_bounds(current, &limits)
                .unwrap_or_else(|failure| panic!("shrunk spline bound: {failure}"));
            let observed = observe_spline(current);
            check_spline(current, &observed)
                .unwrap_or_else(|failure| panic!("shrunk spline case: {failure}"));
        },
    );
    assert_selector_does_not_shrink("spline kinds", SPLINE_KINDS.to_vec());

    println!(
        "packet15 witnesses: pick={} paint={} spline={} \
         pick_shrink=({} -> {}) simplify={} complicate={} restores={} \
         paint_shrink=({} -> {}) simplify={} complicate={} restores={} \
         spline_shrink=({} -> {}) simplify={} complicate={} restores={}",
        pick_witnesses.checked,
        paint_witnesses.checked,
        spline_witnesses.checked,
        pick_report.initial_complexity,
        pick_report.minimal_complexity,
        pick_report.simplified,
        pick_report.complicated,
        pick_report.restores,
        paint_report.initial_complexity,
        paint_report.minimal_complexity,
        paint_report.simplified,
        paint_report.complicated,
        paint_report.restores,
        spline_report.initial_complexity,
        spline_report.minimal_complexity,
        spline_report.simplified,
        spline_report.complicated,
        spline_report.restores
    );
}
