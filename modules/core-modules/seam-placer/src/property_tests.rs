//! Opt-in property tests for this module's private seam-placer kernels.
//!
//! Charter (`docs/testing/properties.md` §§ `M-SEAM-PLACER-SELECT`,
//! `M-SEAM-PLACER-PROJECT`, `M-SEAM-PLACER-ROTATE`):
//!
//! * `M-SEAM-PLACER-SELECT` — `select_seam_candidate` on finite nonempty
//!   candidate lists: real membership plus the mode's own local choice
//!   (`Nearest`/the defensive aligned arms minimize `score + reason_bonus`
//!   with Y-then-X tie-breaks, `Rear` maximizes Y with a lower-score-then-higher-X
//!   tie-break, `Random` takes `layer_index % len`). The empty list must produce
//!   no candidate for **every** mode, `Random` included.
//! * `M-SEAM-PLACER-PROJECT` — `project_onto_wall_segment` onto the nearest
//!   **actual** segment (an open polyline's last-to-first pair is not one),
//!   and `insert_projected_point`'s interior insertion: linear continuous
//!   fields, a start/end discrete-flag choice at `t <= 0.5`, independently
//!   interpolated `width_profile.widths`, and parallel per-point arrays that
//!   keep an open path open.
//! * `M-SEAM-PLACER-ROTATE` — `rotate_wall_loop`'s cyclic zipped
//!   `(point, flag, width)` shift for closed loops plus exactly one closing
//!   repeat, and byte-for-byte preservation of open polylines.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! Every kernel named above is private in the crate root. This module is a
//! crate-internal unit-test child, so all of them are reachable through
//! `super::` without widening production visibility.
//!
//! # Independent expected model
//!
//! No checker calls a kernel to obtain an expected value.
//!
//! * Selection: the winner is recomputed from the candidate list with this
//!   module's own reason-bonus table and its own tie-break walk, including the
//!   standard-library container semantics each arm relies on
//!   (`Iterator::min_by` keeps the **first** minimum, `Iterator::max_by` keeps
//!   the **last** maximum). `Random`'s index is `layer_index % len` computed
//!   here, never read back from the SUT.
//! * Projection: an independent clamped dot-product scan in `f64` over the
//!   **actual** segments (`n - 1` of them for an open polyline, `n - 1` plus
//!   the closing wrap for a closed loop), with the `1e-5` endpoint snap, `t = 0`
//!   for a zero-length segment, and the documented equal-distance tie-break
//!   (lower `wall_index`, then lower `segment_start`). Insertion metadata is
//!   recomputed field by field from the documented contract — six linearly
//!   interpolated continuous fields, two discrete fields chosen at
//!   `t <= 0.5`, and `width_profile.widths` interpolated from the profile
//!   values rather than from the points' own `width` fields.
//! * Rotation: a cyclic zipped `(point, flag, width)` shift built here, then
//!   one closing repeat copied from the new first effective vertex.
//!
//! ## Numeric envelope (derived, not borrowed from the SUT)
//!
//! Generated coordinates sit on a **quarter-millimetre** grid inside
//! `max_coord_quanta` (1/1024 mm quanta). A quarter-millimetre coordinate is
//! `k · 2⁻²` mm with `k` below 2¹², so the coordinate is exact in `f32`; a
//! difference of two such values is at most 64 mm = 2⁶ and is also exact, and
//! its square is `j · 2⁻⁴` with `j` below 2¹⁸ — still inside the 24-bit
//! mantissa, so squared distances are exact too. That is why the ordinary
//! oracle can compare exactly-equal distances (a genuine tie) and still order
//! non-equal ones. The only non-dyadic parameter is `t` on the projection
//! segment; it is compared with `PROJECTION_TOL = 1e-5` mm/parameter, roughly
//! two orders of magnitude above the one-ulp error of a single `f32` rounding
//! at 8 mm. Discrete fields and retained flags are compared exactly.
//!
//! ## Closure convention (representation witness)
//!
//! `ExtrusionPath3D::is_closed` (`crates/slicer-ir/src/slice_ir.rs`) compares
//! XY only, while the pre-fix projection/insertion pair compared whole
//! `Point3WithWidth` records. The kernels under test now use the XY-only
//! predicate, matching `rotate_wall_loop`, so a closing repeat whose metadata
//! differs from the first record is classified as the closing repeat rather
//! than as an extra independent vertex. A dedicated witness pins that
//! representation; the generator emits canonical repeats (whole-record copies).
//!
//! # Literal review
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. Watched-type sites here (a
//! watched type is a `pub` struct with at least five named fields under
//! `crates/*/src`), each reviewed by hand against
//! `docs/21_data_defaults_and_fixtures.md`:
//!
//! * `Point3WithWidth` (eight named fields) — every partial site
//!   (`plain_point`, `rich_point`, `default_point`) carries
//!   `..Default::default()`. The one full literal, `metadata_point`, carries an
//!   adjacent `// exhaustive:` note because a FRU on a fully specified literal
//!   would trip `clippy::needless_update`.
//! * `ExtrusionPath3D` (five named fields) — one site, `wall_from_records`,
//!   uses `..extrusion_path3d_base(ExtrusionRole::OuterWall)`.
//! * `WallLoop` (six named fields) — one site, `wall_from_records`, uses
//!   `..wall_loop_base(LoopType::Outer, WallBoundaryType::ExteriorSurface)`.
//! * `WallFeatureFlags` (six named fields) — `distinct_flags` uses
//!   `..Default::default()` (`tool_index` and `fuzzy_skin` are the identity
//!   being varied).
//!
//! Module-private types (`Limits`, `SelectCase`, `ProjectCase`, `RotateCase`,
//! the `*Observation`/`*Stats` structs, `ModelProjection`) are not watched
//! crate-public structs; their literals are exhaustive by construction because
//! the fields are the fixture or the measurement.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! negative controls are mutation controls over the **same** checker the three
//! properties use: each first re-asserts that the unmodified observation is
//! accepted, then asserts that the single mutated observation is rejected with
//! the expected `CheckFailure` class. The generated properties require positive
//! per-subcase counters taken from the accepted SUT observation; the empty
//! candidate list, the empty wall list and the degenerate single-point path are
//! asserted separately and never counted as positives. Repeat calls and the
//! nonshrinking semantic selectors are marked `// test-quality:` determinism
//! pins naming the protected surface.

#![allow(missing_docs)]

use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{
    ExtrusionRole, LoopType, Point3WithWidth, SeamCandidate, SeamReason, WallBoundaryType,
    WallFeatureFlags, WallLoop, WidthProfile,
};
use slicer_sdk::test_support::fixtures::{extrusion_path3d_base, wall_loop_base};
use slicer_sdk::test_support::property_testing::domains::DomainError;
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::{
    insert_projected_point, project_onto_wall_segment, rotate_wall_loop, select_seam_candidate,
    SeamMode, WallSegmentProjection,
};

const PROPERTY_ID_SELECT: &str = "M-SEAM-PLACER-SELECT";
const PROPERTY_TEST_SELECT: &str = "property_tests::proptest_placer_select";
const PROPERTY_ID_PROJECT: &str = "M-SEAM-PLACER-PROJECT";
const PROPERTY_TEST_PROJECT: &str = "property_tests::proptest_placer_project";
const PROPERTY_ID_ROTATE: &str = "M-SEAM-PLACER-ROTATE";
const PROPERTY_TEST_ROTATE: &str = "property_tests::proptest_placer_rotate";

/// Exact `PropertyRun::bound` keys of the `M-SEAM-PLACER-SELECT` record.
const SELECT_BOUND_KEYS: [&str; 3] = ["max_candidates", "max_coord_quanta", "max_score_quanta"];
/// Exact `PropertyRun::bound` keys of the `M-SEAM-PLACER-PROJECT` record.
const PROJECT_BOUND_KEYS: [&str; 3] = ["max_wall_points", "max_walls", "max_coord_quanta"];
/// Exact `PropertyRun::bound` keys of the `M-SEAM-PLACER-ROTATE` record.
const ROTATE_BOUND_KEYS: [&str; 2] = ["max_wall_points", "max_coord_quanta"];
/// Every key of the deterministic probe map (the union of the three records).
const BOUND_KEYS: [&str; 5] = [
    "max_candidates",
    "max_wall_points",
    "max_walls",
    "max_coord_quanta",
    "max_score_quanta",
];

/// 1/1024 mm per quantum (`docs/08_coordinate_system.md`).
const QUANTUM: f32 = 1.0 / 1024.0;
/// Generated coordinates use a quarter-millimetre grid; see the module docs for
/// why that keeps every square exact in `f32`.
const COORD_GRID_QUANTA: i64 = 256;
/// Endpoint snap tolerance of `project_onto_wall_segment`, restated here so the
/// oracle can exclude the snap boundary from the ordinary domain.
const SNAP_TOLERANCE: f64 = 1e-5;
/// Derived comparison tolerance for interpolated coordinates, widths and `t`.
const PROJECTION_TOL: f64 = 1e-5;
/// Step ceiling for the deterministic shrink-transition loops.
const MAX_SHRINK_STEPS: usize = 20_000;
/// Longest polygon the rotate/project generators build (bounded by
/// `max_wall_points` and by `max_coord_quanta` through the quarter-mm grid).
const MAX_POLYGON_VERTICES: usize = 10;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

/// 8 mm: the largest fixture coordinate in either geometry generator is 7 mm,
/// so a smaller ceiling cannot carry the required positive witnesses.
const MIN_COORD_QUANTA: u64 = 8192;
/// 1.0 in score quanta: the exact-tie witness needs at least one representable
/// score step, and 1024 keeps `score + bonus` well inside `f32` precision.
const MIN_SCORE_QUANTA: u64 = 1024;
/// Two candidates are the smallest list that can express an exact tie.
const MIN_CANDIDATES: u32 = 2;
/// A closed rectangle needs four effective vertices plus its closing repeat.
const MIN_WALL_POINTS: u32 = 5;
/// One near wall plus at least one far wall.
const MIN_WALLS: u32 = 2;
/// Packet ceiling for every count bound.
const MAX_COUNT_CEILING: u32 = 32;
/// Packet ceiling for every numeric quantum bound.
const MAX_QUANTA_CEILING: u64 = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Limits {
    max_candidates: u32,
    max_wall_points: u32,
    max_walls: u32,
    max_coord_quanta: u64,
    max_score_quanta: u64,
}

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

/// Validate every profile bound before any conversion or allocation.
fn checked_limits(bounds: &BTreeMap<String, u64>) -> Result<Limits, DomainError> {
    let as_u32 = |key: &str| -> Result<u32, DomainError> {
        u32::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)
    };
    let limits = Limits {
        max_candidates: as_u32("max_candidates")?,
        max_wall_points: as_u32("max_wall_points")?,
        max_walls: as_u32("max_walls")?,
        max_coord_quanta: required(bounds, "max_coord_quanta")?,
        max_score_quanta: required(bounds, "max_score_quanta")?,
    };

    for (name, value, min) in [
        ("max_candidates", limits.max_candidates, MIN_CANDIDATES),
        ("max_wall_points", limits.max_wall_points, MIN_WALL_POINTS),
        ("max_walls", limits.max_walls, MIN_WALLS),
    ] {
        if !(min..=MAX_COUNT_CEILING).contains(&value) {
            return Err(DomainError::InvalidBounds(format!(
                "{name} must be in {min}..={MAX_COUNT_CEILING}: the required positive \
                 witnesses need at least {min} and the declared ceiling is {MAX_COUNT_CEILING}"
            )));
        }
    }
    if !(MIN_COORD_QUANTA..=MAX_QUANTA_CEILING).contains(&limits.max_coord_quanta) {
        return Err(DomainError::InvalidBounds(format!(
            "max_coord_quanta must be in {MIN_COORD_QUANTA}..={MAX_QUANTA_CEILING}: the \
             farthest fixture coordinate is 7 mm = 7168 quanta"
        )));
    }
    if !(MIN_SCORE_QUANTA..=MAX_QUANTA_CEILING).contains(&limits.max_score_quanta) {
        return Err(DomainError::InvalidBounds(format!(
            "max_score_quanta must be in {MIN_SCORE_QUANTA}..={MAX_QUANTA_CEILING}"
        )));
    }
    if limits.max_wall_points <= limits.max_walls {
        return Err(DomainError::InvalidBounds(format!(
            "max_wall_points ({}) must exceed max_walls ({}): every generated wall needs \
             at least one real segment",
            limits.max_wall_points, limits.max_walls
        )));
    }
    // The quarter-millimetre grid must still fit inside the coordinate ceiling.
    if (limits.max_coord_quanta as i64) < 2 * COORD_GRID_QUANTA {
        return Err(DomainError::InvalidBounds(
            "max_coord_quanta cannot express one quarter-millimetre grid step".to_owned(),
        ));
    }
    Ok(limits)
}

/// The registry profiles themselves; the deterministic controls and the shrink
/// probes have no `PropertyRun` context and use these exact values.
fn probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_candidates".to_owned(), 16),
        ("max_wall_points".to_owned(), 16),
        ("max_walls".to_owned(), 4),
        ("max_coord_quanta".to_owned(), 8192),
        ("max_score_quanta".to_owned(), 4096),
    ])
}

/// Read the selected profile's own bounds and fill the keys its record does not
/// declare from the probe map (each record only carries the keys its contract
/// table lists; the missing ones are unused by that property).
fn live_limits(run: &PropertyRun, keys: &[&str]) -> Limits {
    let mut bounds = probe_bound_map();
    for key in keys {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("seam placer bound `{key}`: {error}"));
        bounds.insert((*key).to_owned(), value);
    }
    checked_limits(&bounds).expect("seam placer bound consistency")
}

// ─────────────────────────────────────────────────────────────────────────────
// Failure classes
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckFailure {
    /// The fixture is outside the declared domain (a generator bug).
    Fixture(String),
    /// The selected candidate is not a member, or is the wrong member.
    Selection(String),
    /// The projected segment/parameter is not the actual nearest one.
    Projection(String),
    /// Insertion cardinality, closure or neighbour sequence is wrong.
    Insertion(String),
    /// An interpolated continuous field, profile width or discrete flag is wrong.
    Metadata(String),
    /// The rotated sequence, its paired arrays or an open path is wrong.
    Rotation(String),
}

impl fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(message) => write!(f, "fixture outside the domain: {message}"),
            Self::Selection(message) => write!(f, "selection: {message}"),
            Self::Projection(message) => write!(f, "projection: {message}"),
            Self::Insertion(message) => write!(f, "insertion: {message}"),
            Self::Metadata(message) => write!(f, "metadata: {message}"),
            Self::Rotation(message) => write!(f, "rotation: {message}"),
        }
    }
}

fn is_selection(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Selection(_))
}

fn is_projection(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Projection(_))
}

fn is_insertion(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Insertion(_))
}

fn is_metadata(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Metadata(_))
}

fn is_rotation(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Rotation(_))
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared construction helpers
// ─────────────────────────────────────────────────────────────────────────────

fn quanta_mm(quanta: i64) -> f32 {
    quanta as f32 * QUANTUM
}

/// Inclusive `lo..=hi` pick; an empty range is a generator bug and panics
/// (a failure, never a rejection — the profiles declare zero reject budgets).
fn pick(raw: u32, lo: u64, hi: u64) -> u64 {
    assert!(lo <= hi, "generator range {lo}..={hi} is empty");
    let span = hi - lo + 1;
    assert!(
        span <= u64::from(u32::MAX),
        "generator range {lo}..={hi} is too wide"
    );
    lo + u64::from(raw) % span
}

/// One point on the quarter-millimetre grid with the default metadata.
fn plain_point(x_quanta: i64, y_quanta: i64, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x: quanta_mm(x_quanta),
        y: quanta_mm(y_quanta),
        z,
        width: 0.4,
        ..Default::default()
    }
}

/// One point whose eight fields are all distinct per `slot`, so a linear
/// interpolation and a discrete start/end choice are distinguishable from each
/// other and from the neighbouring vertices.
fn rich_point(x_quanta: i64, y_quanta: i64, z: f32, slot: usize) -> Point3WithWidth {
    let step = slot as f32;
    // exhaustive: the fixture is exactly this per-point metadata; every field
    // is set from `slot`, so a `..Default::default()` rest would be unreachable
    // (and `clippy::needless_update` rejects it).
    Point3WithWidth {
        x: quanta_mm(x_quanta),
        y: quanta_mm(y_quanta),
        z,
        width: 0.4 + 0.0625 * step,
        flow_factor: 1.0 + 0.125 * step,
        overhang_quartile: Some((slot % 4) as u8),
        dist_to_top_mm: 0.25 * step,
        overhang_distance_mm: Some(0.5 * step - 1.0),
    }
}

/// Per-vertex flags whose `tool_index` makes each vertex individually
/// identifiable through a rotation.
fn distinct_flags(count: usize) -> Vec<WallFeatureFlags> {
    (0..count)
        .map(|index| WallFeatureFlags {
            tool_index: Some(index as u32),
            fuzzy_skin: index % 2 == 1,
            ..Default::default()
        })
        .collect()
}

/// Build a wall loop whose three per-point arrays are parallel by construction.
/// A closed loop's closing record repeats the first record exactly, including
/// its width and flags — the canonical wall-loop representation.
fn wall_from_records(records: Vec<Point3WithWidth>) -> WallLoop {
    let closed = model_is_closed(&records);
    let effective = model_effective_len(&records);
    if effective == 0 {
        return WallLoop {
            path: slicer_ir::ExtrusionPath3D {
                points: records,
                ..extrusion_path3d_base(ExtrusionRole::OuterWall)
            },
            width_profile: WidthProfile { widths: Vec::new() },
            feature_flags: Vec::new(),
            ..wall_loop_base(LoopType::Outer, WallBoundaryType::ExteriorSurface)
        };
    }
    let widths: Vec<f32> = (0..records.len())
        .map(|index| {
            let slot = if closed { index % effective } else { index };
            0.4 + 0.0625 * slot as f32
        })
        .collect();
    let mut flags = distinct_flags(effective);
    if closed {
        let first = flags[0].clone();
        flags.push(first);
    }
    WallLoop {
        path: slicer_ir::ExtrusionPath3D {
            points: records,
            ..extrusion_path3d_base(ExtrusionRole::OuterWall)
        },
        width_profile: WidthProfile { widths },
        feature_flags: flags,
        ..wall_loop_base(LoopType::Outer, WallBoundaryType::ExteriorSurface)
    }
}

/// Axis-aligned rectangle corners in counter-clockwise order.
fn rectangle_corners(
    cx_quanta: i64,
    cy_quanta: i64,
    half_w_quanta: i64,
    half_h_quanta: i64,
) -> [(i64, i64); 4] {
    [
        (cx_quanta - half_w_quanta, cy_quanta - half_h_quanta),
        (cx_quanta + half_w_quanta, cy_quanta - half_h_quanta),
        (cx_quanta + half_w_quanta, cy_quanta + half_h_quanta),
        (cx_quanta - half_w_quanta, cy_quanta + half_h_quanta),
    ]
}

/// The list starts at corner `start` and, when `closed`, repeats that corner as
/// the closing record.
fn rectangle_xy(corners: &[(i64, i64); 4], start: usize, closed: bool) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = (0..4).map(|i| corners[(start + i) % 4]).collect();
    if closed {
        let first = out[0];
        out.push(first);
    }
    out
}

/// XY-only closure, restated independently of `ExtrusionPath3D::is_closed`.
fn model_is_closed(points: &[Point3WithWidth]) -> bool {
    match (points.first(), points.last()) {
        (Some(first), Some(last)) if points.len() >= 2 => first.x == last.x && first.y == last.y,
        _ => false,
    }
}

fn model_effective_len(points: &[Point3WithWidth]) -> usize {
    if model_is_closed(points) {
        points.len() - 1
    } else {
        points.len()
    }
}

fn model_segment_count(points: &[Point3WithWidth]) -> usize {
    let effective = model_effective_len(points);
    if model_is_closed(points) {
        effective
    } else {
        effective.saturating_sub(1)
    }
}

fn model_segment_end(segment_start: usize, effective: usize) -> usize {
    if segment_start + 1 < effective {
        segment_start + 1
    } else {
        0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SEAM-PLACER-SELECT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Nearest,
    Rear,
    Random,
    Aligned,
    AlignedBack,
}

const MODES: [Mode; 5] = [
    Mode::Nearest,
    Mode::Rear,
    Mode::Random,
    Mode::Aligned,
    Mode::AlignedBack,
];

fn mode_index(mode: Mode) -> usize {
    match mode {
        Mode::Nearest => 0,
        Mode::Rear => 1,
        Mode::Random => 2,
        Mode::Aligned => 3,
        Mode::AlignedBack => 4,
    }
}

impl Mode {
    fn sut(self) -> SeamMode {
        match self {
            Self::Nearest => SeamMode::Nearest,
            Self::Rear => SeamMode::Rear,
            Self::Random => SeamMode::Random,
            Self::Aligned => SeamMode::Aligned,
            Self::AlignedBack => SeamMode::AlignedBack,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Nearest => "nearest",
            Self::Rear => "rear",
            Self::Random => "random",
            Self::Aligned => "aligned",
            Self::AlignedBack => "aligned_back",
        }
    }
}

const REASONS: [SeamReason; 4] = [
    SeamReason::Concave,
    SeamReason::Sharp,
    SeamReason::UserForced,
    SeamReason::Aligned,
];

/// The documented priority bonuses, restated here rather than read from
/// `reason_bonus`.
fn model_bonus(reason: SeamReason) -> f32 {
    match reason {
        SeamReason::Concave => -0.5,
        SeamReason::Sharp => -0.2,
        SeamReason::UserForced => -1.0,
        SeamReason::Aligned => 0.0,
    }
}

fn model_effective_score(candidate: &SeamCandidate) -> f32 {
    candidate.score + model_bonus(candidate.reason)
}

/// The winner index, recomputed from the documented rule.
fn oracle_select(case: &SelectCase) -> Option<usize> {
    let candidates = &case.candidates;
    if candidates.is_empty() {
        return None;
    }
    match case.mode {
        Mode::Nearest | Mode::Aligned | Mode::AlignedBack => {
            // `Iterator::min_by` keeps the accumulator when the comparison is
            // not `Greater`, so an exact tie retains the FIRST index.
            let mut best = 0_usize;
            for index in 1..candidates.len() {
                let acc = &candidates[best];
                let next = &candidates[index];
                let ordering = model_effective_score(acc)
                    .total_cmp(&model_effective_score(next))
                    .then_with(|| acc.position.y.total_cmp(&next.position.y))
                    .then_with(|| acc.position.x.total_cmp(&next.position.x));
                if ordering == Ordering::Greater {
                    best = index;
                }
            }
            Some(best)
        }
        Mode::Rear => {
            // `Iterator::max_by` keeps the accumulator only on `Greater`, so an
            // exact tie moves to the LAST index.
            let mut best = 0_usize;
            for index in 1..candidates.len() {
                let acc = &candidates[best];
                let next = &candidates[index];
                let ordering = acc
                    .position
                    .y
                    .total_cmp(&next.position.y)
                    .then_with(|| {
                        model_effective_score(next).total_cmp(&model_effective_score(acc))
                    })
                    .then_with(|| acc.position.x.total_cmp(&next.position.x));
                if ordering != Ordering::Greater {
                    best = index;
                }
            }
            Some(best)
        }
        Mode::Random => Some((case.layer_index as usize) % candidates.len()),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SelectCase {
    mode: Mode,
    layer_index: u32,
    candidates: Vec<SeamCandidate>,
    /// The fixture was built with two identical candidates, so an exact tie is
    /// guaranteed rather than hoped for.
    tie: bool,
}

/// What the SUT returned: the member index (identity-checked at observation
/// time) plus the returned value.
#[derive(Debug, Clone)]
struct SelectObservation {
    index: Option<usize>,
    candidate: Option<SeamCandidate>,
}

fn observe_select(case: &SelectCase) -> SelectObservation {
    let selected = select_seam_candidate(case.mode.sut(), case.layer_index, &case.candidates);
    let index = selected.map(|chosen| {
        case.candidates
            .iter()
            .position(|candidate| std::ptr::eq(candidate, chosen))
            .expect("select_seam_candidate returned a candidate outside its input list")
    });
    SelectObservation {
        index,
        candidate: selected.cloned(),
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct SelectStats {
    nonempty: u64,
    per_mode: [u64; 5],
    ties: u64,
    random_cycles: u64,
    empty: u64,
}

fn validate_select_case(case: &SelectCase, limits: &Limits) -> Result<(), CheckFailure> {
    if case.candidates.len() > limits.max_candidates as usize {
        return Err(CheckFailure::Fixture(format!(
            "{} candidates exceed max_candidates={}",
            case.candidates.len(),
            limits.max_candidates
        )));
    }
    let cap_mm = quanta_mm(limits.max_coord_quanta as i64).abs();
    let cap_score = limits.max_score_quanta as f32 * QUANTUM;
    for (index, candidate) in case.candidates.iter().enumerate() {
        for (label, value) in [
            ("x", candidate.position.x),
            ("y", candidate.position.y),
            ("z", candidate.position.z),
        ] {
            if !(value.is_finite() && value.abs() <= cap_mm + 1.0) {
                return Err(CheckFailure::Fixture(format!(
                    "candidate {index}: {label} {value} is not finite and inside max_coord_quanta"
                )));
            }
        }
        if !(candidate.score.is_finite() && candidate.score.abs() <= cap_score + 1.0) {
            return Err(CheckFailure::Fixture(format!(
                "candidate {index}: score {} is not finite and inside max_score_quanta",
                candidate.score
            )));
        }
    }
    if case.tie && case.candidates.len() < 2 {
        return Err(CheckFailure::Fixture(
            "a tie case needs at least two candidates".to_owned(),
        ));
    }
    Ok(())
}

/// The **one** selection checker: the property, the deterministic witnesses,
/// the mutation controls and the shrink-state revalidation all call it.
fn check_selection(
    case: &SelectCase,
    limits: &Limits,
    observation: &SelectObservation,
) -> Result<SelectStats, CheckFailure> {
    validate_select_case(case, limits)?;
    let expected = oracle_select(case);
    let mut stats = SelectStats {
        empty: u64::from(case.candidates.is_empty()),
        ..SelectStats::default()
    };

    match (expected, observation.index) {
        (None, None) => {
            if observation.candidate.is_some() {
                return Err(CheckFailure::Selection(
                    "an empty candidate list must not produce a candidate".to_owned(),
                ));
            }
        }
        (None, Some(index)) => {
            return Err(CheckFailure::Selection(format!(
                "the SUT selected index {index} from an empty candidate list"
            )))
        }
        (Some(_), None) => {
            return Err(CheckFailure::Selection(
                "the SUT returned no candidate for a nonempty list".to_owned(),
            ))
        }
        (Some(expected), Some(index)) => {
            let member = case.candidates.get(index).ok_or_else(|| {
                CheckFailure::Selection(format!(
                    "index {index} is not a member of a {}-candidate list",
                    case.candidates.len()
                ))
            })?;
            let returned = observation.candidate.as_ref().ok_or_else(|| {
                CheckFailure::Selection("the observed candidate value is missing".to_owned())
            })?;
            if member != returned {
                return Err(CheckFailure::Selection(format!(
                    "the returned candidate {returned:?} is not the list's element {index}"
                )));
            }
            if index != expected {
                return Err(CheckFailure::Selection(format!(
                    "{} selected index {index}, but the independent model selects {expected}",
                    case.mode.label()
                )));
            }
            stats.nonempty = 1;
            stats.per_mode[mode_index(case.mode)] = 1;
            let winner = &case.candidates[index];
            if case
                .candidates
                .iter()
                .filter(|other| {
                    model_effective_score(other).total_cmp(&model_effective_score(winner))
                        == Ordering::Equal
                        && other.position.y.total_cmp(&winner.position.y) == Ordering::Equal
                        && other.position.x.total_cmp(&winner.position.x) == Ordering::Equal
                })
                .count()
                > 1
            {
                stats.ties = 1;
            }
            if case.mode == Mode::Random && case.candidates.len() > 1 && index != 0 {
                stats.random_cycles = 1;
            }
        }
    }
    Ok(stats)
}

const SELECT_RAW_LEN: usize = 3 + 3 * 16;
const SELECT_CANDIDATE_SLOTS: usize = 16;

fn build_select_case(
    mode: Mode,
    tie: bool,
    raw: [u32; SELECT_RAW_LEN],
    limits: &Limits,
) -> SelectCase {
    let slots = (limits.max_candidates as usize).min(SELECT_CANDIDATE_SLOTS);
    let lo = if tie { 2 } else { 1 };
    assert!(
        slots >= lo,
        "max_candidates={} cannot carry a {lo}-candidate fixture",
        limits.max_candidates
    );
    let count = pick(raw[0], lo as u64, slots as u64) as usize;

    let coord_steps = (limits.max_coord_quanta as i64 / COORD_GRID_QUANTA).min(32);
    let score_steps = limits.max_score_quanta as i64;
    let mut candidates = Vec::with_capacity(count);
    for slot in 0..count {
        let block = 3 + 3 * slot;
        let x = (i64::from(raw[block] % ((2 * coord_steps + 1) as u32)) - coord_steps)
            * COORD_GRID_QUANTA;
        let y = (i64::from(raw[block + 1] % ((2 * coord_steps + 1) as u32)) - coord_steps)
            * COORD_GRID_QUANTA;
        let score_quanta = i64::from(raw[block + 2] % ((2 * score_steps + 1) as u32)) - score_steps;
        candidates.push(SeamCandidate {
            position: plain_point(x, y, 0.2),
            score: quanta_mm(score_quanta),
            reason: REASONS[(raw[block] as usize) % REASONS.len()],
        });
    }
    if tie {
        // An exact tie: identical score, reason and position. `min_by` retains
        // the first record and `max_by` the last, so the two containers disagree
        // on the winner while the score-level comparison agrees.
        candidates[1] = candidates[0].clone();
    }

    SelectCase {
        mode,
        layer_index: raw[1],
        candidates,
        tie,
    }
}

fn select_strategy(
    limits: &Limits,
    modes: Vec<Mode>,
    ties: Vec<bool>,
) -> BoxedStrategy<SelectCase> {
    let limits = *limits;
    (
        prop::sample::select(modes).no_shrink(),
        prop::sample::select(ties).no_shrink(),
        any::<[u32; SELECT_RAW_LEN]>(),
    )
        .prop_map(move |(mode, tie, raw)| build_select_case(mode, tie, raw, &limits))
        .boxed()
}

fn select_case_strategy(limits: &Limits) -> BoxedStrategy<SelectCase> {
    select_strategy(limits, MODES.to_vec(), vec![false, true])
}

fn select_complexity(case: &SelectCase) -> i128 {
    let mut total = case.candidates.len() as i128 + i128::from(u64::from(case.layer_index % 1024));
    for candidate in &case.candidates {
        total += i128::from((candidate.position.x.abs() * 4.0) as u64)
            + i128::from((candidate.position.y.abs() * 4.0) as u64)
            + i128::from((candidate.score.abs() * 1024.0) as u64);
    }
    total
}

fn check_select_shrink_state(case: &SelectCase, mode: Mode, tie: bool, limits: &Limits) {
    assert_eq!(case.mode, mode, "the selection mode must survive shrinking");
    assert_eq!(case.tie, tie, "the tie class must survive shrinking");
    if tie {
        assert!(
            case.candidates.len() >= 2 && case.candidates[0] == case.candidates[1],
            "the exact-tie witness vanished"
        );
    }
    let observation = observe_select(case);
    let stats = check_selection(case, limits, &observation)
        .unwrap_or_else(|error| panic!("shrunk {mode:?}/tie={tie} fixture: {error}"));
    assert_eq!(
        stats.nonempty, 1,
        "{mode:?}/tie={tie}: the required nonempty selection vanished"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SEAM-PLACER-PROJECT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Topology {
    Closed,
    Open,
}

const TOPOLOGIES: [Topology; 2] = [Topology::Closed, Topology::Open];

impl Topology {
    fn label(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ProjectCase {
    topology: Topology,
    walls: Vec<WallLoop>,
    target: Point3WithWidth,
    /// Slot index of the near (strictly nearest) wall.
    near_wall: usize,
}

#[derive(Debug, Clone)]
struct ProjectObservation {
    projection: Option<WallSegmentProjection>,
    inserted: Option<WallLoop>,
}

fn observe_project(case: &ProjectCase) -> ProjectObservation {
    let projection = project_onto_wall_segment(&case.target, &case.walls);
    let inserted = projection.and_then(|projected| {
        // The production caller only inserts a genuinely interior projection;
        // an endpoint resolves to the existing vertex.
        if projected.t > 0.0 && projected.t < 1.0 {
            let wall = case
                .walls
                .get(projected.wall_index)
                .expect("the kernel returned an out-of-range wall index");
            Some(insert_projected_point(wall, projected))
        } else {
            None
        }
    });
    ProjectObservation {
        projection,
        inserted,
    }
}

/// The documented interpolation contract, restated here field by field.
fn model_interpolate(start: &Point3WithWidth, end: &Point3WithWidth, t: f32) -> Point3WithWidth {
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    // exhaustive: the oracle restates the documented interpolation rule for each
    // of the eight fields; a `..Default::default()` rest would let a newly added
    // field silently default instead of being given an explicit rule (and
    // `clippy::needless_update` rejects a rest over a full literal).
    Point3WithWidth {
        x: lerp(start.x, end.x),
        y: lerp(start.y, end.y),
        z: lerp(start.z, end.z),
        width: lerp(start.width, end.width),
        flow_factor: lerp(start.flow_factor, end.flow_factor),
        overhang_quartile: if t <= 0.5 {
            start.overhang_quartile
        } else {
            end.overhang_quartile
        },
        dist_to_top_mm: lerp(start.dist_to_top_mm, end.dist_to_top_mm),
        overhang_distance_mm: if t <= 0.5 {
            start.overhang_distance_mm
        } else {
            end.overhang_distance_mm
        },
    }
}

#[derive(Debug, Clone)]
struct ModelProjection {
    point: Point3WithWidth,
    t: f64,
    wall_index: usize,
    segment_start: usize,
    distance2: f64,
}

/// The independent clamped dot-product scan over **actual** segments.
fn model_project(target: &Point3WithWidth, walls: &[WallLoop]) -> Option<ModelProjection> {
    let tx = f64::from(target.x);
    let ty = f64::from(target.y);
    let mut best: Option<ModelProjection> = None;

    for (wall_index, wall) in walls.iter().enumerate() {
        let points = &wall.path.points;
        let effective = model_effective_len(points);
        if effective == 0 {
            continue;
        }
        for segment_start in 0..model_segment_count(points) {
            let end_index = model_segment_end(segment_start, effective);
            let start = points[segment_start];
            let end = points[end_index];
            let dx = f64::from(end.x) - f64::from(start.x);
            let dy = f64::from(end.y) - f64::from(start.y);
            let length2 = dx * dx + dy * dy;
            let raw_t = if length2 > 0.0 {
                ((tx - f64::from(start.x)) * dx + (ty - f64::from(start.y)) * dy) / length2
            } else {
                0.0
            };
            let clamped = raw_t.clamp(0.0, 1.0);
            let t = if clamped.abs() <= SNAP_TOLERANCE {
                0.0
            } else if (1.0 - clamped).abs() <= SNAP_TOLERANCE {
                1.0
            } else {
                clamped
            };
            let point = if t == 0.0 {
                start
            } else if t == 1.0 {
                end
            } else {
                model_interpolate(&start, &end, t as f32)
            };
            let distance2 = (f64::from(point.x) - tx).powi(2) + (f64::from(point.y) - ty).powi(2);
            let candidate = ModelProjection {
                point,
                t,
                wall_index,
                segment_start,
                distance2,
            };
            let replace = match &best {
                None => true,
                Some(current) => match candidate
                    .distance2
                    .partial_cmp(&current.distance2)
                    .expect("finite projected distances")
                {
                    Ordering::Less => true,
                    Ordering::Greater => false,
                    // Equal distance: the lower wall index, then the lower
                    // segment start, wins.
                    Ordering::Equal => {
                        (candidate.wall_index, candidate.segment_start)
                            < (current.wall_index, current.segment_start)
                    }
                },
            };
            if replace {
                best = Some(candidate);
            }
        }
    }
    best
}

/// Distance from `point` to the infinite line through `start`/`end`, plus a
/// check that the clamped foot stays on the closed segment. Used as a geometric
/// on-segment assertion independent of the kernel's own `t`.
fn on_actual_segment(
    point: &Point3WithWidth,
    start: &Point3WithWidth,
    end: &Point3WithWidth,
) -> f64 {
    let dx = f64::from(end.x) - f64::from(start.x);
    let dy = f64::from(end.y) - f64::from(start.y);
    let length2 = dx * dx + dy * dy;
    if length2 == 0.0 {
        return ((f64::from(point.x) - f64::from(start.x)).powi(2)
            + (f64::from(point.y) - f64::from(start.y)).powi(2))
        .sqrt();
    }
    let raw = ((f64::from(point.x) - f64::from(start.x)) * dx
        + (f64::from(point.y) - f64::from(start.y)) * dy)
        / length2;
    let foot_t = raw.clamp(0.0, 1.0);
    let foot_x = f64::from(start.x) + dx * foot_t;
    let foot_y = f64::from(start.y) + dy * foot_t;
    ((f64::from(point.x) - foot_x).powi(2) + (f64::from(point.y) - foot_y).powi(2)).sqrt()
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ProjectStats {
    interior: u64,
    closed_insertions: u64,
    open_insertions: u64,
    endpoints: u64,
    empty_walls: u64,
}

fn validate_project_case(case: &ProjectCase, limits: &Limits) -> Result<(), CheckFailure> {
    if case.walls.len() > limits.max_walls as usize {
        return Err(CheckFailure::Fixture(format!(
            "{} walls exceed max_walls={}",
            case.walls.len(),
            limits.max_walls
        )));
    }
    let cap_mm = quanta_mm(limits.max_coord_quanta as i64).abs();
    for (index, wall) in case.walls.iter().enumerate() {
        if wall.path.points.len() > limits.max_wall_points as usize {
            return Err(CheckFailure::Fixture(format!(
                "wall {index}: {} records exceed max_wall_points={}",
                wall.path.points.len(),
                limits.max_wall_points
            )));
        }
        if wall.path.points.len() != wall.width_profile.widths.len()
            || wall.path.points.len() != wall.feature_flags.len()
        {
            return Err(CheckFailure::Fixture(format!(
                "wall {index}: per-point arrays are not parallel ({} / {} / {})",
                wall.path.points.len(),
                wall.width_profile.widths.len(),
                wall.feature_flags.len()
            )));
        }
        for point in &wall.path.points {
            if !(point.x.is_finite() && point.y.is_finite())
                || point.x.abs() > cap_mm + 1.0
                || point.y.abs() > cap_mm + 1.0
            {
                return Err(CheckFailure::Fixture(format!(
                    "wall {index}: point ({}, {}) is not finite and inside max_coord_quanta",
                    point.x, point.y
                )));
            }
        }
    }
    Ok(())
}

fn compare_f32(label: &str, observed: f32, expected: f32) -> Result<(), CheckFailure> {
    if (f64::from(observed) - f64::from(expected)).abs() > PROJECTION_TOL {
        return Err(CheckFailure::Metadata(format!(
            "{label}: observed {observed} but the model interpolates {expected}"
        )));
    }
    Ok(())
}

fn check_projection_point(
    model: &ModelProjection,
    observed: &WallSegmentProjection,
    case: &ProjectCase,
) -> Result<(), CheckFailure> {
    if observed.wall_index != model.wall_index || observed.segment_start != model.segment_start {
        return Err(CheckFailure::Projection(format!(
            "{} case: the kernel projected onto wall {} segment {}, but the nearest actual \
             segment is wall {} segment {}",
            case.topology.label(),
            observed.wall_index,
            observed.segment_start,
            model.wall_index,
            model.segment_start
        )));
    }
    let wall = &case.walls[observed.wall_index];
    let effective = model_effective_len(&wall.path.points);
    if observed.segment_start >= model_segment_count(&wall.path.points) || effective == 0 {
        return Err(CheckFailure::Projection(format!(
            "wall {} has no segment {} (an open path enumerates only its real segments)",
            observed.wall_index, observed.segment_start
        )));
    }
    let end_index = model_segment_end(observed.segment_start, effective);
    let start = &wall.path.points[observed.segment_start];
    let end = &wall.path.points[end_index];
    let residual = on_actual_segment(&observed.point, start, end);
    if residual > PROJECTION_TOL {
        return Err(CheckFailure::Projection(format!(
            "the projected point ({}, {}) is {residual} mm off the actual segment from \
             ({}, {}) to ({}, {})",
            observed.point.x, observed.point.y, start.x, start.y, end.x, end.y
        )));
    }
    if (f64::from(observed.t) - model.t).abs() > PROJECTION_TOL {
        return Err(CheckFailure::Projection(format!(
            "the parameter is {} but the independent clamped dot product gives {}",
            observed.t, model.t
        )));
    }
    compare_f32("projected x", observed.point.x, model.point.x)?;
    compare_f32("projected y", observed.point.y, model.point.y)?;
    compare_f32("projected z", observed.point.z, model.point.z)?;
    compare_f32("projected width", observed.point.width, model.point.width)?;
    compare_f32(
        "projected flow_factor",
        observed.point.flow_factor,
        model.point.flow_factor,
    )?;
    compare_f32(
        "projected dist_to_top_mm",
        observed.point.dist_to_top_mm,
        model.point.dist_to_top_mm,
    )?;
    if observed.point.overhang_quartile != model.point.overhang_quartile {
        return Err(CheckFailure::Metadata(format!(
            "projected overhang_quartile {:?} != discrete model {:?}",
            observed.point.overhang_quartile, model.point.overhang_quartile
        )));
    }
    if observed.point.overhang_distance_mm != model.point.overhang_distance_mm {
        return Err(CheckFailure::Metadata(format!(
            "projected overhang_distance_mm {:?} != discrete model {:?}",
            observed.point.overhang_distance_mm, model.point.overhang_distance_mm
        )));
    }
    Ok(())
}

fn check_insertion(
    case: &ProjectCase,
    observed: &WallSegmentProjection,
    inserted: &WallLoop,
) -> Result<ProjectStats, CheckFailure> {
    let wall = &case.walls[observed.wall_index];
    let points = &wall.path.points;
    let closed = model_is_closed(points);
    let effective = model_effective_len(points);
    let end_index = model_segment_end(observed.segment_start, effective);
    let insert_at = observed.segment_start + 1;

    // Parallel arrays first: this is the invariant a fabricated closing record
    // broke for open paths.
    if inserted.path.points.len() != inserted.width_profile.widths.len()
        || inserted.path.points.len() != inserted.feature_flags.len()
    {
        return Err(CheckFailure::Insertion(format!(
            "insertion broke parallel per-point arrays: points={} widths={} flags={}",
            inserted.path.points.len(),
            inserted.width_profile.widths.len(),
            inserted.feature_flags.len()
        )));
    }
    let expected_len = points.len() + 1;
    if inserted.path.points.len() != expected_len {
        return Err(CheckFailure::Insertion(format!(
            "the inserted {} loop has {} records; one interior insert yields {expected_len}",
            case.topology.label(),
            inserted.path.points.len()
        )));
    }
    if inserted.path.points.get(insert_at) != Some(&observed.point) {
        return Err(CheckFailure::Insertion(format!(
            "the inserted record at {insert_at} is not the projected point"
        )));
    }

    let inserted_closed = model_is_closed(&inserted.path.points);
    if inserted_closed != closed {
        return Err(CheckFailure::Insertion(format!(
            "closure changed: the input was closed={closed} but the output is \
             closed={inserted_closed}"
        )));
    }
    if closed {
        let last = inserted.path.points.len() - 1;
        if inserted.path.points[last] != inserted.path.points[0] {
            return Err(CheckFailure::Insertion(
                "the closed output lost its explicit closing repeat".to_owned(),
            ));
        }
        if inserted.width_profile.widths[last] != inserted.width_profile.widths[0]
            || inserted.feature_flags[last] != inserted.feature_flags[0]
        {
            return Err(CheckFailure::Insertion(
                "the closing repeat is not mirrored in widths and flags".to_owned(),
            ));
        }
    }

    // Every surviving original record stays in its authored cyclic order.
    for index in 0..effective {
        let observed_index = if index < insert_at { index } else { index + 1 };
        if inserted.path.points[observed_index] != points[index] {
            return Err(CheckFailure::Insertion(format!(
                "original vertex {index} moved or changed at output index {observed_index}"
            )));
        }
        if inserted.width_profile.widths[observed_index] != wall.width_profile.widths[index] {
            return Err(CheckFailure::Insertion(format!(
                "the width of original vertex {index} changed"
            )));
        }
        if inserted.feature_flags[observed_index] != wall.feature_flags[index] {
            return Err(CheckFailure::Insertion(format!(
                "the flags of original vertex {index} changed"
            )));
        }
    }

    // Discrete metadata: the whole start flag at `t <= 0.5`, otherwise the end
    // flag. No boolean interpolation.
    let expected_flag = if observed.t <= 0.5 {
        wall.feature_flags[observed.segment_start].clone()
    } else {
        wall.feature_flags[end_index].clone()
    };
    if inserted.feature_flags[insert_at] != expected_flag {
        return Err(CheckFailure::Metadata(format!(
            "the inserted flags {:?} are not the {} endpoint's {:?}",
            inserted.feature_flags[insert_at],
            if observed.t <= 0.5 { "start" } else { "end" },
            expected_flag
        )));
    }

    // `width_profile.widths` interpolates from the profile, independently of the
    // point's own `width` field.
    let start_width = wall.width_profile.widths[observed.segment_start];
    let end_width = wall.width_profile.widths[end_index];
    let expected_width = f64::from(start_width)
        + (f64::from(end_width) - f64::from(start_width)) * f64::from(observed.t);
    let observed_width = f64::from(inserted.width_profile.widths[insert_at]);
    if (observed_width - expected_width).abs() > PROJECTION_TOL {
        return Err(CheckFailure::Metadata(format!(
            "the inserted profile width {observed_width} is not the linear interpolation \
             {expected_width} of {start_width}..{end_width} at t={}",
            observed.t
        )));
    }

    // The inserted record's own fields follow the same documented contract.
    let start = points[observed.segment_start];
    let end = points[end_index];
    let model_point = model_interpolate(&start, &end, observed.t);
    compare_f32(
        "inserted x",
        inserted.path.points[insert_at].x,
        model_point.x,
    )?;
    compare_f32(
        "inserted y",
        inserted.path.points[insert_at].y,
        model_point.y,
    )?;
    compare_f32(
        "inserted z",
        inserted.path.points[insert_at].z,
        model_point.z,
    )?;
    compare_f32(
        "inserted point width",
        inserted.path.points[insert_at].width,
        model_point.width,
    )?;
    compare_f32(
        "inserted flow_factor",
        inserted.path.points[insert_at].flow_factor,
        model_point.flow_factor,
    )?;
    compare_f32(
        "inserted dist_to_top_mm",
        inserted.path.points[insert_at].dist_to_top_mm,
        model_point.dist_to_top_mm,
    )?;
    if inserted.path.points[insert_at].overhang_quartile != model_point.overhang_quartile {
        return Err(CheckFailure::Metadata(format!(
            "inserted overhang_quartile {:?} != discrete model {:?}",
            inserted.path.points[insert_at].overhang_quartile, model_point.overhang_quartile
        )));
    }
    if inserted.path.points[insert_at].overhang_distance_mm != model_point.overhang_distance_mm {
        return Err(CheckFailure::Metadata(format!(
            "inserted overhang_distance_mm {:?} != discrete model {:?}",
            inserted.path.points[insert_at].overhang_distance_mm, model_point.overhang_distance_mm
        )));
    }

    Ok(ProjectStats {
        interior: 1,
        closed_insertions: u64::from(closed),
        open_insertions: u64::from(!closed),
        ..ProjectStats::default()
    })
}

/// The **one** projection checker: the property, the deterministic witnesses,
/// the mutation controls and the shrink-state revalidation all call it.
fn check_projection(
    case: &ProjectCase,
    limits: &Limits,
    observation: &ProjectObservation,
) -> Result<ProjectStats, CheckFailure> {
    validate_project_case(case, limits)?;
    let model = model_project(&case.target, &case.walls);
    let mut stats = ProjectStats {
        empty_walls: u64::from(case.walls.is_empty()),
        ..ProjectStats::default()
    };

    match (model, &observation.projection) {
        (None, None) => {
            if observation.inserted.is_some() {
                return Err(CheckFailure::Insertion(
                    "a projection that does not exist must not insert a point".to_owned(),
                ));
            }
        }
        (None, Some(projected)) => {
            return Err(CheckFailure::Projection(format!(
                "the kernel returned a projection onto wall {} segment {} for a fixture \
                 with no actual segment",
                projected.wall_index, projected.segment_start
            )))
        }
        (Some(_), None) => {
            return Err(CheckFailure::Projection(
                "the kernel found no projection although the fixture has a real segment".to_owned(),
            ))
        }
        (Some(model), Some(projected)) => {
            if projected.wall_index >= case.walls.len() {
                return Err(CheckFailure::Projection(format!(
                    "wall index {} is outside a {}-wall fixture",
                    projected.wall_index,
                    case.walls.len()
                )));
            }
            check_projection_point(&model, projected, case)?;
            let interior = projected.t > 0.0 && projected.t < 1.0;
            match (&observation.inserted, interior) {
                (None, false) => {
                    stats.endpoints = 1;
                }
                (None, true) => {
                    return Err(CheckFailure::Insertion(format!(
                        "an interior projection at t={} did not insert a point",
                        projected.t
                    )))
                }
                (Some(_), false) => {
                    return Err(CheckFailure::Insertion(format!(
                        "an endpoint projection at t={} inserted a point",
                        projected.t
                    )))
                }
                (Some(inserted), true) => {
                    let insertion = check_insertion(case, projected, inserted)?;
                    stats.interior = insertion.interior;
                    stats.closed_insertions = insertion.closed_insertions;
                    stats.open_insertions = insertion.open_insertions;
                }
            }
        }
    }
    Ok(stats)
}

const PROJECT_RAW_LEN: usize = 6;

/// The near wall's bottom edge sits at `y = 0` with the target `h` below it, so
/// the bottom edge is strictly nearest; the far walls are translated far enough
/// that their nearest corner is at least 2 mm farther than `h <= 1 mm`.
fn build_project_case(
    topology: Topology,
    raw: [u32; PROJECT_RAW_LEN],
    limits: &Limits,
) -> ProjectCase {
    let wall_cap = (limits.max_walls as usize).min(4);
    assert!(wall_cap >= 1, "max_walls must admit one wall");
    let count = pick(raw[0], 1, wall_cap as u64) as usize;
    let near_wall = (raw[1] as usize) % count;
    let start = (raw[2] as usize) % 4;
    let h_quanta = COORD_GRID_QUANTA * (1 + i64::from(raw[3] % 4));
    let tx_quanta = COORD_GRID_QUANTA * (i64::from(raw[4] % 5) - 2);

    let near_corners = rectangle_corners(0, 1024, 1024, 1024);
    let far_corners = [
        rectangle_corners(5120, 6144, 1024, 1024),
        rectangle_corners(-5120, 6144, 1024, 1024),
        rectangle_corners(5120, -5120, 1024, 1024),
    ];

    let mut walls = Vec::with_capacity(count);
    let mut far_slot = 0_usize;
    for slot in 0..count {
        if slot == near_wall {
            let xy = rectangle_xy(&near_corners, start, topology == Topology::Closed);
            let records = xy
                .iter()
                .enumerate()
                .map(|(index, (x, y))| rich_point(*x, *y, 0.2, index))
                .collect();
            walls.push(wall_from_records(records));
        } else {
            let corners = far_corners[far_slot.min(far_corners.len() - 1)];
            far_slot += 1;
            let xy = rectangle_xy(&corners, 0, true);
            let records = xy
                .iter()
                .enumerate()
                .map(|(index, (x, y))| rich_point(*x, *y, 0.2, index))
                .collect();
            walls.push(wall_from_records(records));
        }
    }

    ProjectCase {
        topology,
        walls,
        target: plain_point(tx_quanta, -h_quanta, 0.2),
        near_wall,
    }
}

fn project_strategy(limits: &Limits, topologies: Vec<Topology>) -> BoxedStrategy<ProjectCase> {
    let limits = *limits;
    (
        prop::sample::select(topologies).no_shrink(),
        any::<[u32; PROJECT_RAW_LEN]>(),
    )
        .prop_map(move |(topology, raw)| build_project_case(topology, raw, &limits))
        .boxed()
}

fn project_case_strategy(limits: &Limits) -> BoxedStrategy<ProjectCase> {
    project_strategy(limits, TOPOLOGIES.to_vec())
}

fn project_complexity(case: &ProjectCase) -> i128 {
    let mut total = case.walls.len() as i128;
    for wall in &case.walls {
        total += wall.path.points.len() as i128;
        for point in &wall.path.points {
            total += i128::from((point.x.abs() * 4.0) as u64);
        }
    }
    total += i128::from((case.target.y.abs() * 4.0) as u64);
    total
}

fn check_project_shrink_state(case: &ProjectCase, topology: Topology, limits: &Limits) {
    assert_eq!(
        case.topology, topology,
        "the projection topology must survive shrinking"
    );
    let observation = observe_project(case);
    let stats = check_projection(case, limits, &observation)
        .unwrap_or_else(|error| panic!("shrunk {} fixture: {error}", topology.label()));
    assert!(
        stats.interior + stats.endpoints > 0,
        "{}: the required projection vanished",
        topology.label()
    );
    assert!(
        case.near_wall < case.walls.len(),
        "the wall association vanished"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SEAM-PLACER-ROTATE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
struct RotateCase {
    topology: Topology,
    start_idx: usize,
    wall: WallLoop,
}

#[derive(Debug, Clone)]
struct RotateObservation {
    rotated: WallLoop,
}

fn observe_rotate(case: &RotateCase) -> RotateObservation {
    RotateObservation {
        rotated: rotate_wall_loop(&case.wall, case.start_idx),
    }
}

/// The documented rotation contract, restated here: a cyclic zipped shift of
/// the effective `(point, flag, width)` triples plus exactly one closing
/// repeat; open paths are returned unchanged.
fn model_rotate(wall: &WallLoop, start_idx: usize) -> WallLoop {
    let points = &wall.path.points;
    let mut expected = wall.clone();
    if !model_is_closed(points) {
        return expected;
    }
    let effective = points.len() - 1;
    if effective == 0 {
        return expected;
    }
    let start = start_idx % effective;
    let mut rotated_points = Vec::with_capacity(points.len());
    let mut rotated_widths = Vec::with_capacity(points.len());
    let mut rotated_flags = Vec::with_capacity(points.len());
    for offset in 0..effective {
        let index = (start + offset) % effective;
        rotated_points.push(points[index]);
        rotated_widths.push(wall.width_profile.widths[index]);
        rotated_flags.push(wall.feature_flags[index].clone());
    }
    rotated_points.push(rotated_points[0]);
    rotated_widths.push(rotated_widths[0]);
    rotated_flags.push(rotated_flags[0].clone());
    expected.path.points = rotated_points;
    expected.width_profile.widths = rotated_widths;
    expected.feature_flags = rotated_flags;
    expected
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RotateStats {
    closed_rotations: u64,
    open_unchanged: u64,
    shifted: u64,
    degenerate: u64,
}

fn validate_rotate_case(case: &RotateCase, limits: &Limits) -> Result<(), CheckFailure> {
    let points = &case.wall.path.points;
    if points.len() > limits.max_wall_points as usize {
        return Err(CheckFailure::Fixture(format!(
            "{} records exceed max_wall_points={}",
            points.len(),
            limits.max_wall_points
        )));
    }
    if points.len() != case.wall.width_profile.widths.len()
        || points.len() != case.wall.feature_flags.len()
    {
        return Err(CheckFailure::Fixture(format!(
            "per-point arrays are not parallel ({} / {} / {})",
            points.len(),
            case.wall.width_profile.widths.len(),
            case.wall.feature_flags.len()
        )));
    }
    let cap_mm = quanta_mm(limits.max_coord_quanta as i64).abs();
    for point in points {
        if !(point.x.is_finite() && point.y.is_finite())
            || point.x.abs() > cap_mm + 1.0
            || point.y.abs() > cap_mm + 1.0
        {
            return Err(CheckFailure::Fixture(format!(
                "point ({}, {}) is not finite and inside max_coord_quanta",
                point.x, point.y
            )));
        }
    }
    Ok(())
}

/// The **one** rotation checker: the property, the deterministic witnesses, the
/// mutation controls and the shrink-state revalidation all call it.
fn check_rotation(
    case: &RotateCase,
    limits: &Limits,
    observation: &RotateObservation,
) -> Result<RotateStats, CheckFailure> {
    validate_rotate_case(case, limits)?;
    let points = &case.wall.path.points;
    let rotated = &observation.rotated;
    let closed = model_is_closed(points);
    let mut stats = RotateStats::default();

    // Every field the rotation does not own must be untouched.
    if rotated.perimeter_index != case.wall.perimeter_index
        || rotated.loop_type != case.wall.loop_type
        || rotated.boundary_type != case.wall.boundary_type
        || rotated.path.role != case.wall.path.role
        || rotated.path.speed_factor.to_bits() != case.wall.path.speed_factor.to_bits()
        || rotated.path.tool_index != case.wall.path.tool_index
        || rotated.path.order_lock != case.wall.path.order_lock
    {
        return Err(CheckFailure::Rotation(
            "a non-geometry field of the wall loop changed".to_owned(),
        ));
    }

    if !closed {
        if rotated != &case.wall {
            return Err(CheckFailure::Rotation(
                "an open path must be returned unchanged in every field".to_owned(),
            ));
        }
        stats.open_unchanged = 1;
        return Ok(stats);
    }

    let effective = points.len() - 1;
    if effective == 0 {
        if rotated != &case.wall {
            return Err(CheckFailure::Rotation(
                "a degenerate one-record loop must be returned unchanged".to_owned(),
            ));
        }
        stats.degenerate = 1;
        return Ok(stats);
    }

    if rotated.path.points.len() != points.len() {
        return Err(CheckFailure::Rotation(format!(
            "rotation changed the record count from {} to {}",
            points.len(),
            rotated.path.points.len()
        )));
    }
    if rotated.width_profile.widths.len() != points.len()
        || rotated.feature_flags.len() != points.len()
    {
        return Err(CheckFailure::Rotation(format!(
            "rotation broke the parallel arrays: points={} widths={} flags={}",
            rotated.path.points.len(),
            rotated.width_profile.widths.len(),
            rotated.feature_flags.len()
        )));
    }

    let start = case.start_idx % effective;
    for offset in 0..effective {
        let source = (start + offset) % effective;
        if rotated.path.points[offset] != points[source] {
            return Err(CheckFailure::Rotation(format!(
                "effective vertex {offset} is {:?}, but the cyclic shift of vertex {source} \
                 is {:?}",
                rotated.path.points[offset], points[source]
            )));
        }
        if rotated.width_profile.widths[offset] != case.wall.width_profile.widths[source] {
            return Err(CheckFailure::Rotation(format!(
                "effective vertex {offset} carries width {} but vertex {source} carries {}",
                rotated.width_profile.widths[offset], case.wall.width_profile.widths[source]
            )));
        }
        if rotated.feature_flags[offset] != case.wall.feature_flags[source] {
            return Err(CheckFailure::Rotation(format!(
                "effective vertex {offset} carries the wrong feature flags for vertex {source}"
            )));
        }
    }
    let closing = points.len() - 1;
    if rotated.path.points[closing] != rotated.path.points[0] {
        return Err(CheckFailure::Rotation(
            "the rotated loop lost its closing repeat".to_owned(),
        ));
    }
    if rotated.width_profile.widths[closing] != rotated.width_profile.widths[0]
        || rotated.feature_flags[closing] != rotated.feature_flags[0]
    {
        return Err(CheckFailure::Rotation(
            "the closing repeat is not mirrored in widths and flags".to_owned(),
        ));
    }
    // Geometry and orientation are preserved: the effective multiset is the
    // same set of vertices, so no vertex was dropped, duplicated or reflected.
    for point in &points[..effective] {
        if !rotated.path.points[..effective].contains(point) {
            return Err(CheckFailure::Rotation(format!(
                "vertex ({}, {}) disappeared from the rotated loop",
                point.x, point.y
            )));
        }
    }
    // The independent oracle built the whole expected loop; comparing it here
    // pins every remaining field (speed factor, role, tool and order lock) and
    // makes the effective-sequence checks above diagnostics rather than the
    // only evidence.
    let expected = model_rotate(&case.wall, case.start_idx);
    if rotated != &expected {
        return Err(CheckFailure::Rotation(format!(
            "the rotated loop is not the oracle's cyclic zipped shift: expected {:?}, \
             observed {:?}",
            expected.path.points, rotated.path.points
        )));
    }
    if start != 0 {
        stats.shifted = 1;
    }
    stats.closed_rotations = 1;
    Ok(stats)
}

const ROTATE_RAW_LEN: usize = 4;

fn build_rotate_case(
    topology: Topology,
    raw: [u32; ROTATE_RAW_LEN],
    limits: &Limits,
) -> RotateCase {
    let closed_overhead = usize::from(topology == Topology::Closed);
    let cap = (limits.max_wall_points as usize - closed_overhead).clamp(1, MAX_POLYGON_VERTICES);
    let vertices = pick(raw[0], 1, cap as u64) as usize;
    let records: Vec<Point3WithWidth> = (0..vertices)
        .map(|index| {
            rich_point(
                i64::from(index as u32) * 2 * COORD_GRID_QUANTA,
                i64::from(index as u32 % 3) * COORD_GRID_QUANTA,
                0.2,
                index,
            )
        })
        .collect();
    let mut records = records;
    if topology == Topology::Closed {
        let first = records[0];
        records.push(first);
    }
    RotateCase {
        topology,
        start_idx: (raw[1] % 64) as usize,
        wall: wall_from_records(records),
    }
}

fn rotate_strategy(limits: &Limits, topologies: Vec<Topology>) -> BoxedStrategy<RotateCase> {
    let limits = *limits;
    (
        prop::sample::select(topologies).no_shrink(),
        any::<[u32; ROTATE_RAW_LEN]>(),
    )
        .prop_map(move |(topology, raw)| build_rotate_case(topology, raw, &limits))
        .boxed()
}

fn rotate_case_strategy(limits: &Limits) -> BoxedStrategy<RotateCase> {
    rotate_strategy(limits, TOPOLOGIES.to_vec())
}

fn rotate_complexity(case: &RotateCase) -> i128 {
    let mut total = case.wall.path.points.len() as i128 + case.start_idx as i128;
    for point in &case.wall.path.points {
        total += i128::from((point.x.abs() * 4.0) as u64);
    }
    total
}

fn check_rotate_shrink_state(case: &RotateCase, topology: Topology, limits: &Limits) {
    assert_eq!(
        case.topology, topology,
        "the rotation topology must survive shrinking"
    );
    let observation = observe_rotate(case);
    let stats = check_rotation(case, limits, &observation)
        .unwrap_or_else(|error| panic!("shrunk {} fixture: {error}", topology.label()));
    assert!(
        stats.closed_rotations + stats.open_unchanged + stats.degenerate > 0,
        "{}: the required rotation witness vanished",
        topology.label()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic selection witnesses
// ─────────────────────────────────────────────────────────────────────────────

fn candidate(x_quanta: i64, y_quanta: i64, score: f32, reason: SeamReason) -> SeamCandidate {
    SeamCandidate {
        position: plain_point(x_quanta, y_quanta, 0.2),
        score,
        reason,
    }
}

/// Four candidates whose winning index differs per mode: effective scores are
/// `1.0, 0.4, 0.2, 0.4` in list order.
fn select_witness_candidates() -> Vec<SeamCandidate> {
    vec![
        candidate(0, 0, 1.0, SeamReason::Aligned),
        candidate(COORD_GRID_QUANTA, 0, 0.4, SeamReason::Aligned),
        candidate(2 * COORD_GRID_QUANTA, 0, 0.4, SeamReason::Sharp),
        candidate(0, 3 * COORD_GRID_QUANTA, 0.9, SeamReason::Concave),
    ]
}

fn select_witness(case: &SelectCase, limits: &Limits) -> (SelectObservation, SelectStats) {
    let observation = observe_select(case);
    let stats = check_selection(case, limits, &observation)
        .unwrap_or_else(|error| panic!("selection witness {}: {error}", case.mode.label()));
    (observation, stats)
}

fn selection_witnesses(limits: &Limits) -> SelectStats {
    let mut totals = SelectStats::default();
    let candidates = select_witness_candidates();

    let mut absorb = |stats: SelectStats| {
        totals.nonempty += stats.nonempty;
        for (index, count) in stats.per_mode.iter().enumerate() {
            totals.per_mode[index] += count;
        }
        totals.ties += stats.ties;
        totals.random_cycles += stats.random_cycles;
        totals.empty += stats.empty;
    };

    // ── Every mode on the same list
    for (mode, expected) in [
        (Mode::Nearest, 2_usize),
        (Mode::Aligned, 2),
        (Mode::AlignedBack, 2),
        (Mode::Rear, 3),
        (Mode::Random, 1),
    ] {
        let case = SelectCase {
            mode,
            layer_index: 5,
            candidates: candidates.clone(),
            tie: false,
        };
        let (observation, stats) = select_witness(&case, limits);
        assert_eq!(
            observation.index,
            Some(expected),
            "{} must select index {expected}",
            mode.label()
        );
        assert_eq!(
            observation.candidate.as_ref(),
            candidates.get(expected),
            "{} must return the list's own element",
            mode.label()
        );
        absorb(stats);
    }

    // ── Singleton: every mode returns the only member
    let singleton = SelectCase {
        mode: Mode::Nearest,
        layer_index: 9,
        candidates: vec![candidate(0, 0, 0.3, SeamReason::Sharp)],
        tie: false,
    };
    for mode in MODES {
        let case = SelectCase {
            mode,
            ..singleton.clone()
        };
        let (observation, stats) = select_witness(&case, limits);
        assert_eq!(observation.index, Some(0), "{} singleton", mode.label());
        absorb(stats);
    }

    // ── Exact tie: identical records; `min_by` keeps the first, `max_by` the last
    let tied = candidate(0, 0, 0.5, SeamReason::Concave);
    let tie_case = SelectCase {
        mode: Mode::Nearest,
        layer_index: 0,
        candidates: vec![tied.clone(), tied],
        tie: true,
    };
    let (nearest, stats) = select_witness(&tie_case, limits);
    assert_eq!(nearest.index, Some(0), "Nearest keeps the first tie");
    assert_eq!(stats.ties, 1, "the exact tie must be observed as a tie");
    absorb(stats);
    let rear_case = SelectCase {
        mode: Mode::Rear,
        ..tie_case.clone()
    };
    let (rear, stats) = select_witness(&rear_case, limits);
    assert_eq!(rear.index, Some(1), "Rear keeps the last tie");
    absorb(stats);

    // ── Y-equal tie broken by X: Nearest takes the lower X, Rear the higher
    let x_tie = SelectCase {
        mode: Mode::Nearest,
        layer_index: 0,
        candidates: vec![
            candidate(0, 2 * COORD_GRID_QUANTA, 1.0, SeamReason::Aligned),
            candidate(
                3 * COORD_GRID_QUANTA,
                2 * COORD_GRID_QUANTA,
                1.0,
                SeamReason::Aligned,
            ),
        ],
        tie: false,
    };
    let (low, stats) = select_witness(&x_tie, limits);
    assert_eq!(
        low.index,
        Some(0),
        "Nearest breaks an equal-score tie by lower X"
    );
    absorb(stats);
    let x_rear = SelectCase {
        mode: Mode::Rear,
        ..x_tie
    };
    let (high, stats) = select_witness(&x_rear, limits);
    assert_eq!(
        high.index,
        Some(1),
        "Rear breaks an equal-score tie by higher X"
    );
    absorb(stats);

    // ── Random is deterministic cycling, not sampling
    let cycle = SelectCase {
        mode: Mode::Random,
        layer_index: 7,
        candidates: candidates.clone(),
        tie: false,
    };
    let (seventh, stats) = select_witness(&cycle, limits);
    assert_eq!(
        seventh.index,
        Some(7 % candidates.len()),
        "layer_index % len"
    );
    absorb(stats);
    // test-quality: determinism pin -- the same layer index must select the same
    // candidate twice (protected surface: the deterministic modulo choice).
    let (again, _) = select_witness(&cycle, limits);
    assert_eq!(again.index, seventh.index, "random selection is repeatable");

    // ── Empty list: no candidate for ANY mode, asserted separately
    for mode in MODES {
        let case = SelectCase {
            mode,
            layer_index: 3,
            candidates: Vec::new(),
            tie: false,
        };
        let observation = observe_select(&case);
        assert_eq!(
            observation.index,
            None,
            "{} must return no candidate for an empty list",
            mode.label()
        );
        let stats = check_selection(&case, limits, &observation)
            .unwrap_or_else(|error| panic!("{} empty list: {error}", mode.label()));
        assert_eq!(stats.empty, 1);
        assert_eq!(stats.nonempty, 0, "an empty case is never a positive");
        totals.empty += stats.empty;
    }

    totals
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic projection witnesses
// ─────────────────────────────────────────────────────────────────────────────

fn witness_rectangle(closed: bool, start: usize) -> WallLoop {
    let corners = rectangle_corners(2048, 2048, 2048, 2048);
    let xy = rectangle_xy(&corners, start, closed);
    let effective = if closed { xy.len() - 1 } else { xy.len() };
    wall_from_records(
        xy.iter()
            .enumerate()
            .map(|(index, (x, y))| rich_point(*x, *y, 0.2, index % effective))
            .collect(),
    )
}

fn project_witness(case: &ProjectCase, limits: &Limits) -> (ProjectObservation, ProjectStats) {
    let observation = observe_project(case);
    let stats = check_projection(case, limits, &observation)
        .unwrap_or_else(|error| panic!("projection witness {}: {error}", case.topology.label()));
    (observation, stats)
}

fn project_case(topology: Topology, walls: Vec<WallLoop>, target: (i64, i64)) -> ProjectCase {
    ProjectCase {
        topology,
        walls,
        target: plain_point(target.0, target.1, 0.2),
        near_wall: 0,
    }
}

fn projection_witnesses(limits: &Limits) -> ProjectStats {
    let mut totals = ProjectStats::default();

    // ── Closed loop, insertion exactly at the half-way discrete boundary
    let half = project_case(
        Topology::Closed,
        vec![witness_rectangle(true, 0)],
        (2048, -1024),
    );
    let (observation, stats) = project_witness(&half, limits);
    let projected = observation.projection.expect("half-way projection");
    assert_eq!(projected.wall_index, 0);
    assert_eq!(projected.segment_start, 0, "the bottom edge is segment 0");
    assert_eq!(projected.t, 0.5, "the half-way parameter must be exact");
    assert_eq!(
        projected.point.x, 2.0,
        "the foot of the half-way projection"
    );
    let inserted = observation.inserted.as_ref().expect("interior insertion");
    assert_eq!(inserted.path.points.len(), 6);
    assert_eq!(inserted.width_profile.widths.len(), 6);
    assert_eq!(inserted.feature_flags.len(), 6);
    assert_eq!(
        inserted.feature_flags[1], inserted.feature_flags[0],
        "t <= 0.5 takes the START flag"
    );
    assert_eq!(inserted.width_profile.widths[1], 0.4 + 0.0625 * 0.5);
    totals.interior += stats.interior;
    totals.closed_insertions += stats.closed_insertions;

    // ── Just above the boundary: the END flag
    let above = project_case(
        Topology::Closed,
        vec![witness_rectangle(true, 0)],
        (2560, -1024),
    );
    let (observation, stats) = project_witness(&above, limits);
    let projected = observation.projection.expect("above-half projection");
    assert_eq!(projected.t, 0.625);
    let inserted = observation.inserted.as_ref().expect("interior insertion");
    assert_eq!(
        inserted.feature_flags[1], inserted.feature_flags[2],
        "t > 0.5 takes the END flag"
    );
    totals.interior += stats.interior;
    totals.closed_insertions += stats.closed_insertions;

    // ── Just below the boundary: the START flag
    let below = project_case(
        Topology::Closed,
        vec![witness_rectangle(true, 0)],
        (1536, -1024),
    );
    let (observation, stats) = project_witness(&below, limits);
    let projected = observation.projection.expect("below-half projection");
    assert_eq!(projected.t, 0.375);
    let inserted = observation.inserted.as_ref().expect("interior insertion");
    assert_eq!(
        inserted.feature_flags[1], inserted.feature_flags[0],
        "t < 0.5 takes the START flag"
    );
    assert_ne!(
        inserted.path.points[1].overhang_quartile, inserted.path.points[2].overhang_quartile,
        "the fixture must make the discrete overhang fields distinguishable"
    );
    totals.interior += stats.interior;
    totals.closed_insertions += stats.closed_insertions;

    // ── Open polyline: insertion keeps it open and keeps its arrays parallel
    let open = project_case(
        Topology::Open,
        vec![witness_rectangle(false, 0)],
        (1536, -1024),
    );
    let (observation, stats) = project_witness(&open, limits);
    let projected = observation.projection.expect("open projection");
    assert_eq!(
        projected.segment_start, 0,
        "the bottom edge is a real segment"
    );
    let inserted = observation
        .inserted
        .as_ref()
        .expect("open interior insertion");
    assert_eq!(
        inserted.path.points.len(),
        5,
        "one interior insert, no closing repeat"
    );
    assert_eq!(inserted.width_profile.widths.len(), 5);
    assert_eq!(inserted.feature_flags.len(), 5);
    assert!(
        !model_is_closed(&inserted.path.points),
        "an open polyline must stay open"
    );
    assert_eq!(
        inserted
            .path
            .points
            .last()
            .copied()
            .expect("a last record")
            .y,
        4.0,
        "no fabricated closing chord may replace the real last vertex"
    );
    totals.interior += stats.interior;
    totals.open_insertions += stats.open_insertions;

    // ── Endpoint: t = 0 resolves to the existing vertex and inserts nothing
    let endpoint = project_case(
        Topology::Closed,
        vec![witness_rectangle(true, 0)],
        (0, -1024),
    );
    let (observation, stats) = project_witness(&endpoint, limits);
    let projected = observation.projection.expect("endpoint projection");
    assert_eq!(projected.t, 0.0, "an exact endpoint snaps to t = 0");
    assert_eq!(projected.point.x, 0.0);
    assert!(observation.inserted.is_none(), "endpoints add no record");
    assert_eq!(stats.endpoints, 1);
    totals.endpoints += stats.endpoints;

    // ── Zero-length segment: t = 0 is accounted, never divided by
    let mut zero_length = witness_rectangle(true, 0);
    let duplicate = zero_length.path.points[0];
    zero_length.path.points.insert(1, duplicate);
    zero_length.width_profile.widths.insert(1, 0.9);
    zero_length
        .feature_flags
        .insert(1, WallFeatureFlags::default());
    assert_eq!(
        model_segment_count(&zero_length.path.points),
        5,
        "the duplicated record is an effective vertex, not closure"
    );
    let zero = project_case(Topology::Closed, vec![zero_length], (0, 0));
    let (observation, stats) = project_witness(&zero, limits);
    let projected = observation.projection.expect("zero-length projection");
    assert_eq!(projected.t, 0.0, "a zero-length segment pins t = 0");
    assert_eq!(
        projected.segment_start, 0,
        "a full tie falls back to the lowest segment start"
    );
    assert!(observation.inserted.is_none());
    assert_eq!(stats.endpoints, 1);
    totals.endpoints += stats.endpoints;

    // ── Equal distance across walls: the lower wall index wins first
    let mut second = witness_rectangle(true, 0);
    second.perimeter_index = 1;
    let tie = project_case(
        Topology::Closed,
        vec![witness_rectangle(true, 1), second],
        (2048, -1024),
    );
    let (observation, stats) = project_witness(&tie, limits);
    let projected = observation.projection.expect("cross-wall tie projection");
    assert_eq!(
        (projected.wall_index, projected.segment_start),
        (0, 3),
        "equal distances resolve to wall 0 segment 3, not wall 1 segment 0"
    );
    totals.interior += stats.interior;
    totals.closed_insertions += stats.closed_insertions;

    // ── Same XY, different metadata closing record: classified as the repeat
    let mut representation = witness_rectangle(true, 0);
    let last = representation.path.points.len() - 1;
    representation.path.points[last].width = 9.0;
    representation.path.points[last].flow_factor = 9.0;
    representation.feature_flags[last].tool_index = Some(99);
    representation.width_profile.widths[last] = 9.0;
    assert!(
        representation.path.points[last] != representation.path.points[0],
        "the representation witness needs differing metadata"
    );
    assert!(
        model_is_closed(&representation.path.points),
        "XY equality classifies the record as the closing repeat"
    );
    assert_eq!(
        model_effective_len(&representation.path.points),
        4,
        "the differing-metadata repeat is not an extra vertex"
    );
    let same_xy = project_case(Topology::Closed, vec![representation], (2048, -1024));
    let (observation, stats) = project_witness(&same_xy, limits);
    let projected = observation.projection.expect("representation projection");
    assert_eq!(
        (projected.wall_index, projected.segment_start, projected.t),
        (0, 0, 0.5)
    );
    let inserted = observation
        .inserted
        .as_ref()
        .expect("interior insertion into the representation loop");
    assert_eq!(inserted.path.points.len(), 6);
    assert_eq!(inserted.width_profile.widths.len(), 6);
    assert_eq!(inserted.feature_flags.len(), 6);
    assert_eq!(
        inserted.path.points[5].x, inserted.path.points[0].x,
        "the closing repeat follows the new first vertex"
    );
    assert_eq!(
        inserted.path.points[5].y, inserted.path.points[0].y,
        "the closing repeat follows the new first vertex"
    );
    totals.interior += stats.interior;
    totals.closed_insertions += stats.closed_insertions;

    // ── Empty wall list: nothing to project onto, nothing to insert
    let empty = project_case(Topology::Open, Vec::new(), (0, 0));
    let (observation, stats) = project_witness(&empty, limits);
    assert!(observation.projection.is_none());
    assert!(observation.inserted.is_none());
    assert_eq!(stats.empty_walls, 1);
    totals.empty_walls += stats.empty_walls;

    totals
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic rotation witnesses
// ─────────────────────────────────────────────────────────────────────────────

fn rotate_witness(case: &RotateCase, limits: &Limits) -> (RotateObservation, RotateStats) {
    let observation = observe_rotate(case);
    let stats = check_rotation(case, limits, &observation)
        .unwrap_or_else(|error| panic!("rotation witness {}: {error}", case.topology.label()));
    (observation, stats)
}

fn rotation_witnesses(limits: &Limits) -> RotateStats {
    let mut totals = RotateStats::default();

    // ── Closed loop, start index 0: identical sequence
    let start_zero = RotateCase {
        topology: Topology::Closed,
        start_idx: 0,
        wall: witness_rectangle(true, 0),
    };
    let (observation, stats) = rotate_witness(&start_zero, limits);
    // test-quality: determinism pin -- rotation by 0 must be the identity
    // (protected surface: the cyclic shift's anchor).
    assert_eq!(observation.rotated, start_zero.wall);
    totals.closed_rotations += stats.closed_rotations;

    // ── Closed loop, start index 3: the cyclic shift and one closing repeat
    let shifted = RotateCase {
        topology: Topology::Closed,
        start_idx: 3,
        wall: witness_rectangle(true, 0),
    };
    let (observation, stats) = rotate_witness(&shifted, limits);
    let rotated = &observation.rotated;
    assert_eq!(
        rotated.path.points.len(),
        5,
        "rotation keeps the record count including the repeat"
    );
    assert_eq!(rotated.path.points[0], shifted.wall.path.points[3]);
    assert_eq!(rotated.path.points[1], shifted.wall.path.points[0]);
    assert_eq!(
        rotated.width_profile.widths[0], shifted.wall.width_profile.widths[3],
        "the width travels with its vertex"
    );
    assert_eq!(
        rotated.feature_flags[0], shifted.wall.feature_flags[3],
        "the flag travels with its vertex"
    );
    assert_eq!(rotated.path.points[4], rotated.path.points[0]);
    assert_eq!(stats.shifted, 1);
    totals.closed_rotations += stats.closed_rotations;
    totals.shifted += stats.shifted;

    // ── Start index beyond the effective count wraps modulo that count
    let wrapped = RotateCase {
        topology: Topology::Closed,
        start_idx: 7,
        wall: witness_rectangle(true, 0),
    };
    let (wrap_observation, _) = rotate_witness(&wrapped, limits);
    // test-quality: determinism pin -- start 7 on a four-vertex loop must equal
    // start 3 (protected surface: the modulo anchor).
    let (three_observation, _) = rotate_witness(&shifted, limits);
    assert_eq!(
        wrap_observation.rotated, three_observation.rotated,
        "an out-of-range start index wraps modulo the effective count"
    );

    // ── Open polyline: unchanged in every field
    let open = RotateCase {
        topology: Topology::Open,
        start_idx: 2,
        wall: witness_rectangle(false, 0),
    };
    let (open_observation, stats) = rotate_witness(&open, limits);
    assert_eq!(open_observation.rotated, open.wall);
    totals.open_unchanged += stats.open_unchanged;

    // ── Singleton and empty paths are degenerate, never rotated
    let mut singleton = wall_from_records(vec![plain_point(0, 0, 0.2)]);
    singleton.feature_flags = distinct_flags(1);
    let single = RotateCase {
        topology: Topology::Open,
        start_idx: 3,
        wall: singleton,
    };
    let (single_observation, stats) = rotate_witness(&single, limits);
    assert_eq!(single_observation.rotated, single.wall);
    totals.open_unchanged += stats.open_unchanged;

    let empty = RotateCase {
        topology: Topology::Open,
        start_idx: 1,
        wall: wall_from_records(Vec::new()),
    };
    let (empty_observation, stats) = rotate_witness(&empty, limits);
    assert!(empty_observation.rotated.path.points.is_empty());
    assert!(empty_observation.rotated.feature_flags.is_empty());
    totals.open_unchanged += stats.open_unchanged;

    totals
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic mutation controls
// ─────────────────────────────────────────────────────────────────────────────

fn expect_select_failure(
    case: &SelectCase,
    limits: &Limits,
    baseline: &SelectObservation,
    mutant: &SelectObservation,
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    check_selection(case, limits, baseline)
        .unwrap_or_else(|error| panic!("{what}: the unmodified selection was rejected: {error}"));
    match check_selection(case, limits, mutant) {
        Ok(_) => panic!("{what}: the selection mutation was accepted"),
        Err(error) => assert!(
            matches_class(&error),
            "{what}: rejected for the wrong reason: {error}"
        ),
    }
}

fn expect_project_failure(
    case: &ProjectCase,
    limits: &Limits,
    baseline: &ProjectObservation,
    mutant: &ProjectObservation,
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    check_projection(case, limits, baseline)
        .unwrap_or_else(|error| panic!("{what}: the unmodified projection was rejected: {error}"));
    match check_projection(case, limits, mutant) {
        Ok(_) => panic!("{what}: the projection mutation was accepted"),
        Err(error) => assert!(
            matches_class(&error),
            "{what}: rejected for the wrong reason: {error}"
        ),
    }
}

fn expect_rotate_failure(
    case: &RotateCase,
    limits: &Limits,
    baseline: &RotateObservation,
    mutant: &RotateObservation,
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    check_rotation(case, limits, baseline)
        .unwrap_or_else(|error| panic!("{what}: the unmodified rotation was rejected: {error}"));
    match check_rotation(case, limits, mutant) {
        Ok(_) => panic!("{what}: the rotation mutation was accepted"),
        Err(error) => assert!(
            matches_class(&error),
            "{what}: rejected for the wrong reason: {error}"
        ),
    }
}

fn selection_controls(limits: &Limits) {
    // ── A returned candidate that is not a member of the list
    let case = SelectCase {
        mode: Mode::Nearest,
        layer_index: 2,
        candidates: select_witness_candidates(),
        tie: false,
    };
    let baseline = observe_select(&case);
    check_selection(&case, limits, &baseline).expect("unmodified selection");
    let foreign = candidate(
        9 * COORD_GRID_QUANTA,
        9 * COORD_GRID_QUANTA,
        0.0,
        SeamReason::Aligned,
    );
    let nonmember = SelectObservation {
        index: Some(case.candidates.len()),
        candidate: Some(foreign),
    };
    expect_select_failure(
        &case,
        limits,
        &baseline,
        &nonmember,
        "returned a non-member candidate",
        is_selection,
    );

    // ── The wrong winner of an exact tie
    let tied = candidate(0, 0, 0.5, SeamReason::Concave);
    let tie_case = SelectCase {
        mode: Mode::Nearest,
        layer_index: 0,
        candidates: vec![tied.clone(), tied],
        tie: true,
    };
    let tie_baseline = observe_select(&tie_case);
    assert_eq!(tie_baseline.index, Some(0));
    let wrong_tie = SelectObservation {
        index: Some(1),
        candidate: tie_case.candidates.get(1).cloned(),
    };
    expect_select_failure(
        &tie_case,
        limits,
        &tie_baseline,
        &wrong_tie,
        "selected the wrong tie winner",
        is_selection,
    );

    println!("placer_selection_controls: every named mutation rejected by the shared checker");
}

fn projection_controls(limits: &Limits) {
    // ── A projection point that is off the actual segment
    let case = project_case(
        Topology::Closed,
        vec![witness_rectangle(true, 0)],
        (1536, -1024),
    );
    let baseline = observe_project(&case);
    check_projection(&case, limits, &baseline).expect("unmodified projection");
    let mut off_segment = baseline.clone();
    let mut shifted = off_segment.projection.expect("a projection exists");
    shifted.point.y += 0.5;
    off_segment.projection = Some(shifted);
    off_segment.inserted = None;
    expect_project_failure(
        &case,
        limits,
        &baseline,
        &off_segment,
        "projection put off the actual segment",
        is_projection,
    );

    // ── An altered interpolated profile width
    let baseline = observe_project(&case);
    let mut altered_width = baseline.clone();
    let mut inserted = altered_width.inserted.clone().expect("an insertion exists");
    inserted.width_profile.widths[1] += 0.0625;
    altered_width.inserted = Some(inserted);
    expect_project_failure(
        &case,
        limits,
        &baseline,
        &altered_width,
        "altered interpolated width",
        is_metadata,
    );

    // ── An altered discrete flag
    let baseline = observe_project(&case);
    let mut altered_flag = baseline.clone();
    let mut inserted = altered_flag.inserted.clone().expect("an insertion exists");
    inserted.feature_flags[1].is_bridge = !inserted.feature_flags[1].is_bridge;
    altered_flag.inserted = Some(inserted);
    expect_project_failure(
        &case,
        limits,
        &baseline,
        &altered_flag,
        "altered discrete flag",
        is_metadata,
    );

    // ── An open path that gets force-closed by the insertion
    let open_case = project_case(
        Topology::Open,
        vec![witness_rectangle(false, 0)],
        (1536, -1024),
    );
    let open_baseline = observe_project(&open_case);
    check_projection(&open_case, limits, &open_baseline).expect("unmodified open projection");
    let mut closed_mutant = open_baseline.clone();
    let mut inserted = closed_mutant.inserted.clone().expect("an insertion exists");
    let first_point = inserted.path.points[0];
    let first_width = inserted.width_profile.widths[0];
    let first_flag = inserted.feature_flags[0].clone();
    inserted.path.points.push(first_point);
    inserted.width_profile.widths.push(first_width);
    inserted.feature_flags.push(first_flag);
    closed_mutant.inserted = Some(inserted);
    expect_project_failure(
        &open_case,
        limits,
        &open_baseline,
        &closed_mutant,
        "closed an open path",
        is_insertion,
    );

    println!("placer_projection_controls: every named mutation rejected by the shared checker");
}

fn rotation_controls(limits: &Limits) {
    let case = RotateCase {
        topology: Topology::Closed,
        start_idx: 2,
        wall: witness_rectangle(true, 0),
    };
    let baseline = observe_rotate(&case);
    check_rotation(&case, limits, &baseline).expect("unmodified rotation");

    // ── Points rotated without their matching widths
    let mut unmatched_width = baseline.clone();
    unmatched_width.rotated.width_profile.widths.swap(0, 1);
    expect_rotate_failure(
        &case,
        limits,
        &baseline,
        &unmatched_width,
        "rotated points without their matching width",
        is_rotation,
    );

    // ── Points rotated without their matching flags
    let mut unmatched_flag = baseline.clone();
    unmatched_flag.rotated.feature_flags[0].tool_index = Some(4242);
    expect_rotate_failure(
        &case,
        limits,
        &baseline,
        &unmatched_flag,
        "rotated points without their matching flag",
        is_rotation,
    );

    // ── A lost closing repeat
    let mut lost_repeat = baseline.clone();
    lost_repeat.rotated.path.points.pop();
    lost_repeat.rotated.width_profile.widths.pop();
    lost_repeat.rotated.feature_flags.pop();
    expect_rotate_failure(
        &case,
        limits,
        &baseline,
        &lost_repeat,
        "lost the closing repeat",
        is_rotation,
    );

    // ── A reversed orientation with matched metadata
    let mut reversed = baseline.clone();
    let effective = reversed.rotated.path.points.len() - 1;
    reversed.rotated.path.points[..effective].reverse();
    reversed.rotated.width_profile.widths[..effective].reverse();
    reversed.rotated.feature_flags[..effective].reverse();
    let first = reversed.rotated.path.points[0];
    let first_width = reversed.rotated.width_profile.widths[0];
    let first_flag = reversed.rotated.feature_flags[0].clone();
    reversed.rotated.path.points[effective] = first;
    reversed.rotated.width_profile.widths[effective] = first_width;
    reversed.rotated.feature_flags[effective] = first_flag;
    expect_rotate_failure(
        &case,
        limits,
        &baseline,
        &reversed,
        "reversed the effective orientation",
        is_rotation,
    );

    // ── An open path that gets closed
    let open_case = RotateCase {
        topology: Topology::Open,
        start_idx: 1,
        wall: witness_rectangle(false, 0),
    };
    let open_baseline = observe_rotate(&open_case);
    check_rotation(&open_case, limits, &open_baseline).expect("unmodified open rotation");
    let mut closed_mutant = open_baseline.clone();
    let first_point = closed_mutant.rotated.path.points[0];
    let first_width = closed_mutant.rotated.width_profile.widths[0];
    let first_flag = closed_mutant.rotated.feature_flags[0].clone();
    closed_mutant.rotated.path.points.push(first_point);
    closed_mutant.rotated.width_profile.widths.push(first_width);
    closed_mutant.rotated.feature_flags.push(first_flag);
    expect_rotate_failure(
        &open_case,
        limits,
        &open_baseline,
        &closed_mutant,
        "closed an open path",
        is_rotation,
    );

    println!("placer_rotation_controls: every named mutation rejected by the shared checker");
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink probes
// ─────────────────────────────────────────────────────────────────────────────

/// Drive one strategy through a real `simplify`/`complicate` chain, revalidating
/// the class and the required witness at every intermediate value.
fn drive_shrink_probe<S, T>(
    strategy: S,
    label: &str,
    check: impl Fn(&T),
    complexity: impl Fn(&T) -> i128,
) where
    S: Strategy<Value = T>,
    T: Clone + fmt::Debug + PartialEq,
{
    // Phase 1: accept every simplification and observe a real reduction.
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    let initial = tree.current();
    check(&initial);
    let initial_complexity = complexity(&initial);
    let mut accepted = 0_usize;
    while tree.simplify() {
        accepted += 1;
        assert!(
            accepted < MAX_SHRINK_STEPS,
            "{label}: simplify never exhausted"
        );
        check(&tree.current());
    }
    let minimal = tree.current();
    assert!(accepted > 0, "{label}: the tree never simplified");
    let minimal_complexity = complexity(&minimal);
    assert!(
        minimal_complexity < initial_complexity,
        "{label}: no permitted reduction ({initial_complexity} -> {minimal_complexity})"
    );

    // Phase 2: alternate simplify/complicate, then settle.
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    // test-quality: determinism pin -- the same seeded runner must start from the
    // same generated fixture (protected surface: the strategy's raw-to-fixture
    // mapping).
    assert_eq!(
        tree.current(),
        initial,
        "{label}: the deterministic start is not stable"
    );
    let mut simplified = 0_usize;
    let mut complicated = 0_usize;
    let mut changed = false;
    while tree.simplify() {
        simplified += 1;
        assert!(
            simplified < MAX_SHRINK_STEPS,
            "{label}: the alternation never ended"
        );
        let current = tree.current();
        check(&current);
        changed |= current != initial;
        if tree.complicate() {
            complicated += 1;
        }
        check(&tree.current());
    }
    assert!(
        simplified > 0 && changed,
        "{label}: no real transition was observed"
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
    let mut restores = 0_usize;
    while tree.complicate() {
        restores += 1;
        assert!(
            restores < MAX_SHRINK_STEPS,
            "{label}: complicate never ended"
        );
        check(&tree.current());
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
    println!(
        "{label}: phase1_simplify={accepted} complexity {initial_complexity}->\
         {minimal_complexity} phase2_simplify={simplified} complicate={complicated} \
         restores={restores}"
    );
}

fn select_shrink_probe(limits: &Limits) {
    for mode in MODES {
        for tie in [false, true] {
            let label = format!("select/{}/{tie}", mode.label());
            let strategy = select_strategy(limits, vec![mode], vec![tie]);
            drive_shrink_probe(
                strategy.clone(),
                &label,
                |case| check_select_shrink_state(case, mode, tie, limits),
                select_complexity,
            );
            // The semantic selectors never shrink: a shrinking mode or tie class
            // would switch a witness's class mid-search.
            let mut runner = TestRunner::deterministic();
            let tree = strategy
                .clone()
                .new_tree(&mut runner)
                .expect("select case tree");
            assert_eq!(tree.current().mode, mode);
        }
    }
    let mut runner = TestRunner::deterministic();
    let mut tree = prop::sample::select(MODES.to_vec())
        .no_shrink()
        .new_tree(&mut runner)
        .expect("mode tree");
    let before = tree.current();
    // test-quality: determinism pin -- a persistent semantic mode must not shrink
    // (protected surface: per-mode selection witnesses).
    assert!(!tree.simplify(), "the mode selector simplified");
    assert!(!tree.complicate(), "the mode selector complicated");
    assert_eq!(tree.current(), before);
    let mut tree = prop::sample::select(vec![false, true])
        .no_shrink()
        .new_tree(&mut runner)
        .expect("tie tree");
    let before = tree.current();
    // test-quality: determinism pin -- the tie class must not shrink (protected
    // surface: the exact-tie witness).
    assert!(!tree.simplify(), "the tie selector simplified");
    assert_eq!(tree.current(), before);
}

fn project_shrink_probe(limits: &Limits) {
    for topology in TOPOLOGIES {
        let label = format!("project/{}", topology.label());
        let strategy = project_strategy(limits, vec![topology]);
        drive_shrink_probe(
            strategy,
            &label,
            |case| check_project_shrink_state(case, topology, limits),
            project_complexity,
        );
    }
    let mut runner = TestRunner::deterministic();
    let mut tree = prop::sample::select(TOPOLOGIES.to_vec())
        .no_shrink()
        .new_tree(&mut runner)
        .expect("topology tree");
    let before = tree.current();
    // test-quality: determinism pin -- a persistent topology must not shrink
    // (protected surface: open/closed projection witnesses).
    assert!(!tree.simplify(), "the topology selector simplified");
    assert!(!tree.complicate(), "the topology selector complicated");
    assert_eq!(tree.current(), before);
}

fn rotate_shrink_probe(limits: &Limits) {
    for topology in TOPOLOGIES {
        let label = format!("rotate/{}", topology.label());
        let strategy = rotate_strategy(limits, vec![topology]);
        drive_shrink_probe(
            strategy,
            &label,
            |case| check_rotate_shrink_state(case, topology, limits),
            rotate_complexity,
        );
    }
    let mut runner = TestRunner::deterministic();
    let mut tree = prop::sample::select(TOPOLOGIES.to_vec())
        .no_shrink()
        .new_tree(&mut runner)
        .expect("topology tree");
    let before = tree.current();
    // test-quality: determinism pin -- a persistent topology must not shrink
    // (protected surface: open-path preservation).
    assert!(
        !tree.simplify(),
        "the rotation topology selector simplified"
    );
    assert_eq!(tree.current(), before);
}

// ─────────────────────────────────────────────────────────────────────────────
// Properties
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_placer_select() {
    let run = PropertyRun::new(PROPERTY_ID_SELECT, PROPERTY_TEST_SELECT)
        .expect("M-SEAM-PLACER-SELECT context");
    let limits = live_limits(&run, &SELECT_BOUND_KEYS);
    let strategy = select_case_strategy(&limits);

    let calls = Cell::new(0_u64);
    let nonempty = Cell::new(0_u64);
    let ties = Cell::new(0_u64);
    let random_cycles = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 5] = std::array::from_fn(|_| Cell::new(0));

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let observation = observe_select(&case);
        let stats = match check_selection(&case, &limits, &observation) {
            Ok(stats) => stats,
            Err(error) => return fail(format!("{} case: {error}", case.mode.label())),
        };
        if stats.nonempty == 0 {
            return fail(format!(
                "{} case produced no nonempty selection",
                case.mode.label()
            ));
        }
        nonempty.set(nonempty.get() + stats.nonempty);
        ties.set(ties.get() + stats.ties);
        random_cycles.set(random_cycles.get() + stats.random_cycles);
        for (index, count) in stats.per_mode.iter().enumerate() {
            per_mode[index].set(per_mode[index].get() + count);
        }
        Ok(())
    })
    .expect("M-SEAM-PLACER-SELECT membership and local choice");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(
        nonempty.get() > 0,
        "the nonempty counter was never observed"
    );
    assert!(ties.get() > 0, "the exact-tie counter was never observed");
    assert!(
        random_cycles.get() > 0,
        "the random-cycling counter was never observed"
    );
    for (index, mode) in MODES.iter().enumerate() {
        assert!(
            per_mode[index].get() > 0,
            "the {} selection counter was never observed",
            mode.label()
        );
    }

    // Empty lists are asserted separately and never counted as positives.
    for mode in MODES {
        let empty = SelectCase {
            mode,
            layer_index: 0,
            candidates: Vec::new(),
            tie: false,
        };
        let observation = observe_select(&empty);
        assert!(
            observation.index.is_none(),
            "{} must not select from an empty list",
            mode.label()
        );
    }

    println!(
        "M-SEAM-PLACER-SELECT accepted_calls={} nonempty={} ties={} random_cycles={} \
         per_mode={:?}",
        calls.get(),
        nonempty.get(),
        ties.get(),
        random_cycles.get(),
        per_mode.each_ref().map(Cell::get)
    );
}

#[test]
fn proptest_placer_project() {
    let run = PropertyRun::new(PROPERTY_ID_PROJECT, PROPERTY_TEST_PROJECT)
        .expect("M-SEAM-PLACER-PROJECT context");
    let limits = live_limits(&run, &PROJECT_BOUND_KEYS);
    let strategy = project_case_strategy(&limits);

    let calls = Cell::new(0_u64);
    let interior = Cell::new(0_u64);
    let closed_insertions = Cell::new(0_u64);
    let open_insertions = Cell::new(0_u64);
    let endpoints = Cell::new(0_u64);
    let multi_wall = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let observation = observe_project(&case);
        let stats = match check_projection(&case, &limits, &observation) {
            Ok(stats) => stats,
            Err(error) => return fail(format!("{} case: {error}", case.topology.label())),
        };
        if stats.interior == 0 && stats.endpoints == 0 {
            return fail(format!(
                "{} case produced no projection at all",
                case.topology.label()
            ));
        }
        if stats.empty_walls > 0 {
            return fail("a generated projection case had no wall".to_owned());
        }
        interior.set(interior.get() + stats.interior);
        closed_insertions.set(closed_insertions.get() + stats.closed_insertions);
        open_insertions.set(open_insertions.get() + stats.open_insertions);
        endpoints.set(endpoints.get() + stats.endpoints);
        if case.walls.len() > 1 {
            multi_wall.set(multi_wall.get() + 1);
        }
        Ok(())
    })
    .expect("M-SEAM-PLACER-PROJECT actual-segment projection and insertion metadata");

    assert!(calls.get() > 0, "the property callback never ran");
    for (name, counter) in [
        ("interior insertions", &interior),
        ("closed insertions", &closed_insertions),
        ("open insertions", &open_insertions),
        ("endpoint projections", &endpoints),
        ("multi-wall fixtures", &multi_wall),
    ] {
        assert!(counter.get() > 0, "the {name} counter was never observed");
    }

    // The empty wall list is asserted separately and never counted as a positive.
    let empty = project_case(Topology::Open, Vec::new(), (0, 0));
    let observation = observe_project(&empty);
    assert!(observation.projection.is_none());
    assert!(observation.inserted.is_none());

    println!(
        "M-SEAM-PLACER-PROJECT accepted_calls={} interior={} closed={} open={} \
         endpoints={} multi_wall={}",
        calls.get(),
        interior.get(),
        closed_insertions.get(),
        open_insertions.get(),
        endpoints.get(),
        multi_wall.get()
    );
}

#[test]
fn proptest_placer_rotate() {
    let run = PropertyRun::new(PROPERTY_ID_ROTATE, PROPERTY_TEST_ROTATE)
        .expect("M-SEAM-PLACER-ROTATE context");
    let limits = live_limits(&run, &ROTATE_BOUND_KEYS);
    let strategy = rotate_case_strategy(&limits);

    let calls = Cell::new(0_u64);
    let closed_rotations = Cell::new(0_u64);
    let open_unchanged = Cell::new(0_u64);
    let shifted = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let observation = observe_rotate(&case);
        let stats = match check_rotation(&case, &limits, &observation) {
            Ok(stats) => stats,
            Err(error) => return fail(format!("{} case: {error}", case.topology.label())),
        };
        if stats.closed_rotations == 0 && stats.open_unchanged == 0 {
            return fail(format!(
                "{} case produced no rotation witness",
                case.topology.label()
            ));
        }
        closed_rotations.set(closed_rotations.get() + stats.closed_rotations);
        open_unchanged.set(open_unchanged.get() + stats.open_unchanged);
        shifted.set(shifted.get() + stats.shifted);
        Ok(())
    })
    .expect("M-SEAM-PLACER-ROTATE cyclic correspondence and open preservation");

    assert!(calls.get() > 0, "the property callback never ran");
    for (name, counter) in [
        ("closed rotations", &closed_rotations),
        ("open-path preservations", &open_unchanged),
        ("non-zero shifts", &shifted),
    ] {
        assert!(counter.get() > 0, "the {name} counter was never observed");
    }

    // The degenerate empty path is asserted separately.
    let empty = RotateCase {
        topology: Topology::Open,
        start_idx: 5,
        wall: wall_from_records(Vec::new()),
    };
    let observation = observe_rotate(&empty);
    assert!(observation.rotated.path.points.is_empty());

    println!(
        "M-SEAM-PLACER-ROTATE accepted_calls={} closed={} open_unchanged={} shifted={}",
        calls.get(),
        closed_rotations.get(),
        open_unchanged.get(),
        shifted.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Combined deterministic witness / control / shrink test
// ─────────────────────────────────────────────────────────────────────────────

/// Bound-construction rejection matrix: every inconsistent profile is refused
/// before conversion or allocation.
fn invalid_profile_matrix() {
    let valid = probe_bound_map();
    let invalid = |label: &str, bounds: BTreeMap<String, u64>| match checked_limits(&bounds) {
        Err(DomainError::InvalidBounds(message)) => {
            assert!(!message.is_empty(), "{label}: empty reason")
        }
        other => panic!("{label}: expected InvalidBounds, got {other:?}"),
    };

    for key in BOUND_KEYS {
        let mut bounds = valid.clone();
        bounds.remove(key);
        invalid(&format!("missing {key}"), bounds);
    }
    for (key, value) in [
        ("max_candidates", 1),
        ("max_candidates", 33),
        ("max_wall_points", 4),
        ("max_wall_points", 33),
        ("max_walls", 1),
        ("max_walls", 33),
        ("max_coord_quanta", 8191),
        ("max_coord_quanta", 65_537),
        ("max_coord_quanta", 0),
        ("max_score_quanta", 1023),
        ("max_score_quanta", 65_537),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        invalid(&format!("invalid {key}={value}"), bounds);
    }
    // A profile whose wall ceiling cannot hold even one real segment is
    // inconsistent rather than silently narrowed.
    let mut inconsistent = valid.clone();
    inconsistent.insert("max_walls".to_owned(), 16);
    inconsistent.insert("max_wall_points".to_owned(), 16);
    invalid("max_wall_points <= max_walls", inconsistent);

    for key in ["max_candidates", "max_wall_points", "max_walls"] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), u64::from(u32::MAX) + 1);
        assert!(matches!(
            checked_limits(&bounds),
            Err(DomainError::ArithmeticOverflow)
        ));
    }
}

#[test]
fn packet15_witnesses_controls_and_shrinking() {
    let limits = checked_limits(&probe_bound_map()).expect("probe bounds");

    let selection = selection_witnesses(&limits);
    assert!(
        selection.nonempty > 0,
        "the nonempty selection witness was unobserved"
    );
    assert!(
        selection.per_mode.iter().all(|count| *count > 0),
        "a per-mode selection witness was unobserved: {:?}",
        selection.per_mode
    );
    assert!(selection.ties > 0, "the exact-tie witness was unobserved");
    assert!(
        selection.random_cycles > 0,
        "the random-cycling witness was unobserved"
    );
    assert!(
        selection.empty > 0,
        "the empty-candidate witness was unobserved"
    );

    let projection = projection_witnesses(&limits);
    assert!(
        projection.interior > 0,
        "the interior insertion witness was unobserved"
    );
    assert!(
        projection.closed_insertions > 0,
        "the closed-loop insertion witness was unobserved"
    );
    assert!(
        projection.open_insertions > 0,
        "the open-polyline insertion witness was unobserved"
    );
    assert!(
        projection.endpoints > 0,
        "the endpoint witness was unobserved"
    );
    assert!(
        projection.empty_walls > 0,
        "the empty-wall witness was unobserved"
    );

    let rotation = rotation_witnesses(&limits);
    assert!(
        rotation.closed_rotations > 0,
        "the closed rotation witness was unobserved"
    );
    assert!(
        rotation.open_unchanged > 0,
        "the open-path preservation witness was unobserved"
    );
    assert!(rotation.shifted > 0, "the non-zero shift was unobserved");

    selection_controls(&limits);
    projection_controls(&limits);
    rotation_controls(&limits);
    invalid_profile_matrix();
    select_shrink_probe(&limits);
    project_shrink_probe(&limits);
    rotate_shrink_probe(&limits);

    println!(
        "packet15_witnesses_controls_and_shrinking: selection={selection:?} \
         projection={projection:?} rotation={rotation:?} controls=rejected shrinking=green"
    );
}
