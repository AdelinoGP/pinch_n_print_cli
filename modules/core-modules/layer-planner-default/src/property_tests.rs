//! Opt-in property tests for this module's private layer-sequence kernels.
//!
//! Charter (`docs/testing/properties.md` §§ `M-LAYERS-SEQ`, `M-LAYERS-MERGE`):
//!
//! * `M-LAYERS-SEQ` — `generate_object_layers` and its `f64` companion on an
//!   independent event model: object-local native tops (canonical band-midpoint
//!   uniform fallback, or a verbatim explicit host schedule), the greatest
//!   final raft top added to **every** object, and the raft prefix's own
//!   sequence.
//! * `M-LAYERS-MERGE` — `merge_layer_sequences` (and the private
//!   `merge_same_height` / `merge_raft_sequences` / `raft_top` kernels) plus
//!   `DefaultLayerPlanner::run_layer_planning`: the union of native tops,
//!   per-object native/catch-up associations, first credits, catch-up
//!   envelopes, raft prefix ordering and the pushed layer sequence.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! Every kernel named above, and the `ObjectPlan` / `MergedLayer` fixtures, are
//! private in the crate root. This module is a crate-internal unit-test child,
//! so all of them are reachable through `super::` without widening production
//! visibility.
//!
//! # Independent expected model
//!
//! The checker never calls a kernel to obtain an expected value. Each fixture
//! is built by this module as an `ObjectSpec`: an object id, a height, a layer
//! height, a first-layer height, a raft count and (optionally) an explicit
//! top list. Generated fixtures are also carried in integer **quanta** of
//! 1/1024 mm, and the model derives the native sequence twice, independently:
//!
//! 1. `integer_native_tops` walks the canonical midpoint rule in integer
//!    half-quanta: with `T(n) = 2·(F + n·S) − (n == 0 ? F : S)` and all lengths
//!    in quanta, a plane is emitted iff `T(n) < 2·H`. It then cross-checks its
//!    own length against the closed form
//!    `1 + max(0, floor((2H − 1 − 2F + S) / 2S))` (zero when `F ≥ 2H`).
//! 2. `f64_native_tops` transcribes the documented rule literally
//!    (`z = first + n·layer_height`, `slice_z = z − 0.5·step`, break at
//!    `slice_z >= height`) and must agree bit-for-bit with (1) whenever the
//!    fixture is on the quantum grid.
//!
//! Both derivations come from the documented contract, not from the kernel.
//! The raft prefix, the union of native tops, the per-object catch-up from the
//! last participating Z, the first-credit rules and the emitted plane/region
//! tables are then constructed from those sequences.
//!
//! ## End rule: canonical midpoint (resolved AC-1)
//!
//! `generate_object_layers_f64` breaks when the **band midpoint** reaches the
//! object height, so a final print top may overshoot the height by up to half a
//! layer. That is canonical `generate_object_layers` (`Slicing.cpp`) as resolved
//! upstream by perf-vs-orca ticket 139 (`DEV-201` closed;
//! `docs/02_ir_schemas.md` §"Print top vs sample plane" is normative), so this
//! model pins the midpoint rule as **canonical-correct**, not as a protected
//! characterisation. Explicit non-empty `layer_zs` is forwarded verbatim; the
//! uniform formula is never reapplied.
//!
//! ## Fast f64 path vs general f32 path (characterisation)
//!
//! `merge_same_height` keeps its end test and per-object participation filter in
//! `f64`; `merge_different_heights` rounds each plane Z to `f32`, compares
//! `z as f64 − raft_top` against the object height, and computes a catch-up
//! height as one **f32 subtraction** of two f32 planes. That difference is
//! intentionally *not* re-pinned as an end rule: on the 1/1024 mm grid every
//! value is an exact `f32` (a quantum index is below 2^17, well inside the
//! 24-bit mantissa), so both paths necessarily agree there and the model
//! compares bit-exactly. Outside the grid the second rounding is observable;
//! `rounding_probe` pins the current numeric behaviour with a deliberately
//! constructed case whose f32 catch-up height differs from the single-rounded
//! `f64` difference.
//!
//! ## `1e-6` uses (none of them is an end rule)
//!
//! * `merge_layer_sequences`'s `all_same_height` comparison (`|Δ| < 1e-6`, f64);
//! * `merge_raft_sequences`'s Z dedup and per-object raft-index match;
//! * `merge_different_heights`'s global-Z dedup and native-index match;
//! * `merge_different_heights`'s catch-up gap test (`height > 1e-6`, f32);
//! * `merge_different_heights`'s catch-up end guard
//!   (`base_z_f64 > plan.height + 1e-6`, f64) — a participation rule for
//!   catch-up planes, characterised as such;
//! * `merge_different_heights`'s zero sentinel (`|last_z| < 1e-6`).
//!
//! The smallest non-zero difference on the quantum grid is one quantum
//! (`0.0009765625 mm`), roughly 977× the threshold, so every one of these
//! reduces to exact equality or ordering for generated fixtures. The only
//! non-grid fixtures are the deterministic decimal witnesses, whose expected
//! values are asserted per plane rather than inferred from the threshold.
//!
//! # Literal review
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. Watched-type sites here (a
//! watched type is a `pub` struct with at least five named fields under
//! `crates/*/src`):
//!
//! * `RegionLayerProposal` (`crates/slicer-sdk/src/prepass_types.rs`) — one
//!   literal site, `expected_region`, carrying an `// exhaustive:` note: every
//!   field is that expected region's identity and credit, and `region_id` is
//!   the planner's single region `"0"` by contract.
//! * `LayerPlanningObject` (`crates/slicer-sdk/src/traits.rs`) — one literal
//!   site, `planning_objects`, carrying an `// exhaustive:` note: all six
//!   fields are the object's planning input and `layer_zs` is empty exactly
//!   when the fixture selected the uniform fallback.
//! * `Point3WithWidth`, `ExtrusionPath3D`, `GeometryBounds`, `GeometryDomain` —
//!   not used by this module.
//!
//! Module-private types (`ObjectSpec`, `Quanta`, `LayerCase`, `Observation`,
//! `Expected`, `ExpectedPlane`, `LayerStats`, `Limits`) and the crate-private
//! `ObjectPlan` / `MergedLayer` are not watched crate-public structs; their
//! literals are exhaustive by construction because the fields are the fixture.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! negative controls are mutation controls over the **same** checker the two
//! properties use: each first re-asserts that the unmodified observation is
//! accepted, then asserts that the single mutated observation is rejected with
//! the expected `CheckFailure` class. A panic or a scaffolding-only change
//! would not count. The properties require positive per-subcase counters taken
//! from the accepted SUT observation, and empty/no-native cases are asserted
//! separately and never counted as positives. Repeat-call and mode-selector
//! comparisons are marked `// test-quality:` determinism pins naming the
//! protected surface.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_sdk::prelude::*;
use slicer_sdk::test_support::property_testing::domains::DomainError;
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_sdk::traits::LayerPlanningObject;

use super::{
    explicit_schedule_step, generate_object_layers, generate_object_layers_f64,
    merge_layer_sequences, merge_raft_sequences, merge_same_height, raft_top, DefaultLayerPlanner,
    MergedLayer, ObjectPlan,
};

const PROPERTY_ID_SEQ: &str = "M-LAYERS-SEQ";
const PROPERTY_TEST_SEQ: &str = "property_tests::proptest_layer_sequence";
const PROPERTY_ID_MERGE: &str = "M-LAYERS-MERGE";
const PROPERTY_TEST_MERGE: &str = "property_tests::proptest_layer_merge";

/// Exact `PropertyRun::bound` keys of the `M-LAYERS-SEQ` record.
const SEQ_BOUND_KEYS: [&str; 3] = ["max_layer_steps", "max_raft_layers", "max_z_quanta"];
/// Exact `PropertyRun::bound` keys of the `M-LAYERS-MERGE` record.
const MERGE_BOUND_KEYS: [&str; 4] = [
    "max_objects",
    "max_layer_steps",
    "max_raft_layers",
    "max_z_quanta",
];
/// Keys of the deterministic probe map (the merge record's full set).
const BOUND_KEYS: [&str; 4] = MERGE_BOUND_KEYS;

/// `M-LAYERS-SEQ` has no `max_objects` bound: its record fixes a two-object
/// fixture, which is still checked against `Limits::max_objects`.
const SEQ_OBJECTS: u32 = 2;

/// Step ceiling for the deterministic shrink-transition loops.
const MAX_SHRINK_STEPS: usize = 20_000;

/// The dyadic grid: 1/1024 mm per quantum.
const QUANTUM: f64 = 1.0 / 1024.0;
/// The SUT's coincidence/sentinel threshold (`1e-6`), used for exactly the
/// purposes enumerated in the module docs.
const COINCIDENCE: f64 = 1e-6;

/// Smallest `max_objects` that admits the explicit A/B and raft/raft-free pairs.
const MIN_OBJECTS: u32 = 2;
const MAX_OBJECTS_CEILING: u32 = 4;
/// Smallest `max_layer_steps` that admits the four-top explicit A witness.
const MIN_LAYER_STEPS: u32 = 4;
const MAX_LAYER_STEPS_CEILING: u32 = 32;
/// Smallest `max_raft_layers` that admits the two-raft-top witness.
const MIN_RAFT_LAYERS: u32 = 2;
const MAX_RAFT_LAYERS_CEILING: u32 = 8;
/// 1.5 mm: the raft witness's largest global plane is 1.125 mm = 1152 quanta.
const MIN_Z_QUANTA: u64 = 1536;
/// 64 mm. Below 2^17 quanta, so every quantum-grid value and every sum used
/// here is exactly representable in `f32` (the mantissa holds 24 bits).
const MAX_Z_QUANTA_CEILING: u64 = 65_536;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Limits {
    max_objects: u32,
    max_layer_steps: u32,
    max_raft_layers: u32,
    max_z_quanta: u64,
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
        max_objects: as_u32("max_objects")?,
        max_layer_steps: as_u32("max_layer_steps")?,
        max_raft_layers: as_u32("max_raft_layers")?,
        max_z_quanta: required(bounds, "max_z_quanta")?,
    };

    if !(MIN_OBJECTS..=MAX_OBJECTS_CEILING).contains(&limits.max_objects) {
        return Err(DomainError::InvalidBounds(format!(
            "max_objects must be in {MIN_OBJECTS}..={MAX_OBJECTS_CEILING}: the A/B and \
             raft/raft-free witnesses need two objects and the declared ceiling is four"
        )));
    }
    if !(MIN_LAYER_STEPS..=MAX_LAYER_STEPS_CEILING).contains(&limits.max_layer_steps) {
        return Err(DomainError::InvalidBounds(format!(
            "max_layer_steps must be in {MIN_LAYER_STEPS}..={MAX_LAYER_STEPS_CEILING}: the \
             explicit A witness has four authored tops"
        )));
    }
    if !(MIN_RAFT_LAYERS..=MAX_RAFT_LAYERS_CEILING).contains(&limits.max_raft_layers) {
        return Err(DomainError::InvalidBounds(format!(
            "max_raft_layers must be in {MIN_RAFT_LAYERS}..={MAX_RAFT_LAYERS_CEILING}: the raft \
             witness has two raft tops"
        )));
    }
    if !(MIN_Z_QUANTA..=MAX_Z_QUANTA_CEILING).contains(&limits.max_z_quanta) {
        return Err(DomainError::InvalidBounds(format!(
            "max_z_quanta must be in {MIN_Z_QUANTA}..={MAX_Z_QUANTA_CEILING}: the raft witness's \
             largest global plane is 1152 quanta, and above {MAX_Z_QUANTA_CEILING} quanta the \
             f32 exactness of the dyadic domain would no longer hold"
        )));
    }
    Ok(limits)
}

/// Read the selected profile's bounds, supplying `max_objects` only for a
/// record that does not declare one (`M-LAYERS-SEQ`).
fn live_limits(run: &PropertyRun, keys: &[&str], fixed_objects: Option<u32>) -> Limits {
    let mut bounds = BTreeMap::new();
    for key in keys {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("layer planner bound `{key}`: {error}"));
        bounds.insert((*key).to_owned(), value);
    }
    if let Some(objects) = fixed_objects {
        bounds.insert("max_objects".to_owned(), u64::from(objects));
    }
    checked_limits(&bounds).expect("layer planner bound consistency")
}

fn seq_limits(run: &PropertyRun) -> Limits {
    live_limits(run, &SEQ_BOUND_KEYS, Some(SEQ_OBJECTS))
}

fn merge_limits(run: &PropertyRun) -> Limits {
    live_limits(run, &MERGE_BOUND_KEYS, None)
}

/// The registry profile itself; the deterministic controls and the shrink probe
/// have no `PropertyRun` context and use these exact values.
fn probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_objects".to_owned(), 4),
        ("max_layer_steps".to_owned(), 16),
        ("max_raft_layers".to_owned(), 4),
        ("max_z_quanta".to_owned(), 8192),
    ])
}

// ─────────────────────────────────────────────────────────────────────────────
// Failure classes
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckFailure {
    /// The fixture is outside the declared domain (a generator bug).
    Fixture(String),
    /// The two independent native-sequence derivations disagree.
    Oracle(String),
    /// The SUT's `raft_top` differs from the model's.
    RaftTop(String),
    /// A per-object native (object-local f64 or raft-offset f32) sequence differs.
    NativeTops(String),
    /// The raft prefix's planes, credits or ordering differ.
    RaftPlanes(String),
    /// The pushed `LayerProposal` sequence is malformed or differs.
    PlanSequence(String),
    /// A plane's participating object set differs.
    Association(String),
    /// A model plane's region fields differ.
    Merge(String),
}

impl fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(message) => write!(f, "fixture outside the domain: {message}"),
            Self::Oracle(message) => write!(f, "oracle disagreement: {message}"),
            Self::RaftTop(message) => write!(f, "raft top: {message}"),
            Self::NativeTops(message) => write!(f, "native tops: {message}"),
            Self::RaftPlanes(message) => write!(f, "raft planes: {message}"),
            Self::PlanSequence(message) => write!(f, "plan sequence: {message}"),
            Self::Association(message) => write!(f, "association: {message}"),
            Self::Merge(message) => write!(f, "merge: {message}"),
        }
    }
}

fn is_native_tops(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::NativeTops(_))
}

fn is_raft_planes(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::RaftPlanes(_))
}

fn is_merge(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Merge(_))
}

fn is_association(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::Association(_))
}

fn is_plan_sequence(failure: &CheckFailure) -> bool {
    matches!(failure, CheckFailure::PlanSequence(_))
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

/// One object under test, in exact millimeters plus (for generated fixtures)
/// the integer quantum form the model's second derivation uses.
#[derive(Debug, Clone, PartialEq)]
struct ObjectSpec {
    object_id: String,
    height_mm: f64,
    layer_height_mm: f64,
    first_layer_height_mm: f64,
    raft_layers: u32,
    layer_zs_mm: Vec<f64>,
    quanta: Option<Quanta>,
}

/// The integer 1/1024 mm coordinates of a generated object.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Quanta {
    height: u64,
    layer_height: u64,
    first_layer_height: u64,
    layer_zs: Vec<u64>,
}

impl ObjectSpec {
    fn has_explicit(&self) -> bool {
        !self.layer_zs_mm.is_empty()
    }

    /// The crate-private fixture the kernels consume.
    fn plan(&self) -> ObjectPlan {
        ObjectPlan {
            object_id: self.object_id.clone(),
            height: self.height_mm,
            layer_height: self.layer_height_mm,
            first_layer_height: self.first_layer_height_mm,
            raft_layers: self.raft_layers,
            layer_zs: self.layer_zs_mm.clone(),
        }
    }
}

fn quanta_mm(quanta: u64) -> f64 {
    quanta as f64 * QUANTUM
}

/// Exact quantum coordinate of a whole number of 1/1024 mm.
fn quanta_of(mm: f64) -> u64 {
    let scaled = mm * 1024.0;
    assert!(
        scaled.is_finite() && scaled > 0.0 && scaled.round() == scaled,
        "{mm} mm is not on the 1/1024 mm grid"
    );
    scaled as u64
}

fn quantized_spec(id: usize, quanta: Quanta, raft_layers: u32) -> ObjectSpec {
    ObjectSpec {
        object_id: format!("obj-{id}"),
        height_mm: quanta_mm(quanta.height),
        layer_height_mm: quanta_mm(quanta.layer_height),
        first_layer_height_mm: quanta_mm(quanta.first_layer_height),
        raft_layers,
        layer_zs_mm: quanta.layer_zs.iter().copied().map(quanta_mm).collect(),
        quanta: Some(quanta),
    }
}

/// A deterministic witness on the quantum grid.
fn witness_spec(
    id: usize,
    height: u64,
    layer_height: u64,
    first_layer_height: u64,
    raft_layers: u32,
    layer_zs: &[u64],
) -> ObjectSpec {
    quantized_spec(
        id,
        Quanta {
            height,
            layer_height,
            first_layer_height,
            layer_zs: layer_zs.to_vec(),
        },
        raft_layers,
    )
}

/// A deterministic witness outside the quantum grid (decimal millimeters).
fn raw_spec(
    id: usize,
    height_mm: f64,
    layer_height_mm: f64,
    first_layer_height_mm: f64,
    raft_layers: u32,
    layer_zs_mm: &[f64],
) -> ObjectSpec {
    ObjectSpec {
        object_id: format!("obj-{id}"),
        height_mm,
        layer_height_mm,
        first_layer_height_mm,
        raft_layers,
        layer_zs_mm: layer_zs_mm.to_vec(),
        quanta: None,
    }
}

fn planning_objects(specs: &[ObjectSpec]) -> Vec<LayerPlanningObject> {
    specs
        .iter()
        // All six fields are this object's planning input, and `layer_zs` is
        // empty exactly when the fixture selected the uniform fallback (so the
        // field set is the fixture, not a default).
        // exhaustive: complete planning-object input.
        .map(|spec| LayerPlanningObject {
            object_id: spec.object_id.clone(),
            object_height: spec.height_mm,
            layer_height: spec.layer_height_mm,
            first_layer_height: spec.first_layer_height_mm,
            support_raft_layers: spec.raft_layers,
            layer_zs: spec.layer_zs_mm.clone(),
        })
        .collect()
}

/// One expected model region.
fn expected_region(
    object_id: &str,
    effective_layer_height: f32,
    is_catchup: bool,
    catchup_z_bottom: f32,
) -> RegionLayerProposal {
    // Every field is this expected region's identity and credit; `region_id` is
    // the planner's single region "0" by contract, so a FRU over `Default`
    // would silently produce the wrong id.
    // exhaustive: complete region identity and credit.
    RegionLayerProposal {
        object_id: object_id.to_owned(),
        region_id: "0".to_owned(),
        effective_layer_height,
        is_catchup,
        catchup_z_bottom,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The independent expected model
// ─────────────────────────────────────────────────────────────────────────────

/// `T(n) = 2·(F + n·S) − (n == 0 ? F : S)` in half-quanta: a plane is emitted
/// while `T(n) < 2·H`, exactly the canonical midpoint test `slice_z < height`.
fn half_quanta_midpoint(first: u64, step: u64, n: u64) -> i128 {
    let z = i128::from(first) + i128::from(n) * i128::from(step);
    let layer_step = if n == 0 {
        i128::from(first)
    } else {
        i128::from(step)
    };
    2 * z - layer_step
}

/// The inclusive integer height range that yields exactly `k` native tops.
fn exact_k_height_range(k: u64, first: u64, step: u64) -> (u64, u64) {
    let previous = half_quanta_midpoint(first, step, k - 1);
    let next = half_quanta_midpoint(first, step, k);
    let low = u64::try_from(previous.div_euclid(2) + 1).expect("positive half-quanta");
    let high = u64::try_from(next.div_euclid(2)).expect("positive half-quanta");
    assert!(
        low <= high,
        "no height quantizes to exactly {k} tops for first={first} step={step}"
    );
    (low, high)
}

/// Closed-form native top count for the uniform fallback, derived from the
/// integer midpoint inequality (independent of the loop).
fn closed_form_native_count(quanta: &Quanta) -> u64 {
    let first = i128::from(quanta.first_layer_height);
    let step = i128::from(quanta.layer_height);
    let height = i128::from(quanta.height);
    if first >= 2 * height {
        return 0;
    }
    let numerator = 2 * height - 1 - 2 * first + step;
    let n_max = numerator.div_euclid(2 * step);
    u64::try_from(1 + n_max.max(0)).expect("bounded native count")
}

/// Derivation 1: integer half-quanta walk plus its own closed-form cross-check.
fn integer_native_tops(quanta: &Quanta, max_steps: u32) -> Result<Vec<u64>, String> {
    if !quanta.layer_zs.is_empty() {
        return Ok(quanta.layer_zs.clone());
    }
    let mut tops = Vec::new();
    let mut n = 0_u64;
    loop {
        if half_quanta_midpoint(quanta.first_layer_height, quanta.layer_height, n)
            >= 2 * i128::from(quanta.height)
        {
            break;
        }
        if tops.len() == max_steps as usize {
            return Err(format!(
                "uniform fallback would exceed max_layer_steps={max_steps}"
            ));
        }
        tops.push(quanta.first_layer_height + n * quanta.layer_height);
        n += 1;
    }
    let closed = closed_form_native_count(quanta);
    if closed != tops.len() as u64 {
        return Err(format!(
            "integer walk produced {} tops but the closed form says {closed}",
            tops.len()
        ));
    }
    Ok(tops)
}

/// Derivation 2: the documented `f64` rule, transcribed literally.
fn f64_native_tops(spec: &ObjectSpec, limits: &Limits) -> Result<Vec<f64>, CheckFailure> {
    let mut tops = Vec::new();
    let mut n = 0_u64;
    loop {
        let z = spec.first_layer_height_mm + n as f64 * spec.layer_height_mm;
        let layer_step = if n == 0 {
            spec.first_layer_height_mm
        } else {
            spec.layer_height_mm
        };
        if z - 0.5 * layer_step >= spec.height_mm {
            break;
        }
        if tops.len() == limits.max_layer_steps as usize {
            return Err(CheckFailure::Fixture(format!(
                "{}: uniform fallback exceeds max_layer_steps={}",
                spec.object_id, limits.max_layer_steps
            )));
        }
        tops.push(z);
        n += 1;
    }
    Ok(tops)
}

fn native_tops_mm(spec: &ObjectSpec, limits: &Limits) -> Result<Vec<f64>, CheckFailure> {
    if spec.has_explicit() {
        return Ok(spec.layer_zs_mm.clone());
    }
    let rule = f64_native_tops(spec, limits)?;
    match &spec.quanta {
        None => Ok(rule),
        Some(quanta) => {
            let integer =
                integer_native_tops(quanta, limits.max_layer_steps).map_err(|message| {
                    CheckFailure::Fixture(format!("{}: {message}", spec.object_id))
                })?;
            let as_mm: Vec<f64> = integer.iter().copied().map(quanta_mm).collect();
            if as_mm.len() != rule.len()
                || as_mm
                    .iter()
                    .zip(&rule)
                    .any(|(left, right)| left.to_bits() != right.to_bits())
            {
                return Err(CheckFailure::Oracle(format!(
                    "{}: the half-quanta derivation ({} tops) and the f64 rule ({} tops) disagree",
                    spec.object_id,
                    as_mm.len(),
                    rule.len()
                )));
            }
            // The integer derivation is authoritative on the grid.
            Ok(as_mm)
        }
    }
}

fn explicit_credit(spec: &ObjectSpec, index: usize) -> f32 {
    if index == 0 {
        spec.first_layer_height_mm as f32
    } else {
        (spec.layer_zs_mm[index] - spec.layer_zs_mm[index - 1]) as f32
    }
}

fn model_raft_top(specs: &[ObjectSpec]) -> f64 {
    specs
        .iter()
        .filter(|spec| spec.raft_layers > 0)
        .map(|spec| {
            spec.first_layer_height_mm + (f64::from(spec.raft_layers) - 1.0) * spec.layer_height_mm
        })
        .fold(0.0, f64::max)
}

fn is_fast_path(specs: &[ObjectSpec]) -> bool {
    let first = &specs[0];
    specs.iter().all(|spec| {
        !spec.has_explicit()
            && (spec.layer_height_mm - first.layer_height_mm).abs() < COINCIDENCE
            && (spec.first_layer_height_mm - first.first_layer_height_mm).abs() < COINCIDENCE
    })
}

/// `merge_raft_sequences`: sorted, deduped raft tops with per-object credits.
fn model_raft_planes(specs: &[ObjectSpec]) -> Vec<ExpectedPlane> {
    let raft_top = |spec: &ObjectSpec, index: u32| {
        (spec.first_layer_height_mm + f64::from(index) * spec.layer_height_mm) as f32
    };
    let mut all: Vec<f32> = specs
        .iter()
        .flat_map(|spec| (0..spec.raft_layers).map(|index| raft_top(spec, index)))
        .collect();
    all.sort_by(|left, right| left.partial_cmp(right).expect("finite raft tops"));
    all.dedup_by(|left, right| (*left - *right).abs() < COINCIDENCE as f32);

    all.into_iter()
        .map(|z| {
            let regions = specs
                .iter()
                .filter_map(|spec| {
                    (0..spec.raft_layers)
                        .find(|index| (raft_top(spec, *index) - z).abs() < COINCIDENCE as f32)
                        .map(|index| {
                            let effective = if index == 0 {
                                spec.first_layer_height_mm
                            } else {
                                spec.layer_height_mm
                            };
                            expected_region(&spec.object_id, effective as f32, false, 0.0)
                        })
                })
                .collect();
            ExpectedPlane { z, regions }
        })
        .collect()
}

/// `merge_same_height`: f64 midpoint end test and per-object midpoint filter.
fn model_fast_planes(
    specs: &[ObjectSpec],
    raft_top: f64,
    limits: &Limits,
) -> Result<Vec<ExpectedPlane>, CheckFailure> {
    let max_height = specs
        .iter()
        .map(|spec| spec.height_mm)
        .fold(0.0_f64, f64::max);
    let first = specs[0].first_layer_height_mm;
    let layer_height = specs[0].layer_height_mm;
    let mut planes: Vec<ExpectedPlane> = Vec::new();
    let mut n = 0_u64;
    loop {
        let base = first + n as f64 * layer_height;
        let layer_step = if n == 0 { first } else { layer_height };
        if base - 0.5 * layer_step >= max_height {
            break;
        }
        if n > u64::from(limits.max_layer_steps) {
            return Err(CheckFailure::Fixture(format!(
                "fast path exceeds max_layer_steps={}",
                limits.max_layer_steps
            )));
        }
        let pushed = !planes.is_empty();
        let regions: Vec<RegionLayerProposal> = specs
            .iter()
            .filter_map(|spec| {
                let own_step = if n == 0 {
                    spec.first_layer_height_mm
                } else {
                    spec.layer_height_mm
                };
                if base - 0.5 * own_step < spec.height_mm {
                    // `layers.is_empty()` is the "before this plane" state, which
                    // can only be the n == 0 plane: the n == 0 filter admits the
                    // tallest object whenever the loop did not break.
                    let effective = if pushed {
                        spec.layer_height_mm
                    } else {
                        spec.first_layer_height_mm
                    };
                    Some(expected_region(
                        &spec.object_id,
                        effective as f32,
                        false,
                        0.0,
                    ))
                } else {
                    None
                }
            })
            .collect();
        if !regions.is_empty() {
            planes.push(ExpectedPlane {
                z: (raft_top + base) as f32,
                regions,
            });
        }
        n += 1;
    }
    Ok(planes)
}

/// `merge_different_heights`: union of native tops, native planes unfiltered,
/// catch-up planes gated by the (characterised) top-based guard.
fn model_general_planes(
    specs: &[ObjectSpec],
    raft_top: f64,
    native_global: &[Vec<f32>],
) -> Vec<ExpectedPlane> {
    let mut all: Vec<f32> = native_global.iter().flatten().copied().collect();
    all.sort_by(|left, right| left.partial_cmp(right).expect("finite native tops"));
    all.dedup_by(|left, right| (*left - *right).abs() < COINCIDENCE as f32);

    let mut last_z = vec![0.0_f32; specs.len()];
    let mut planes: Vec<ExpectedPlane> = Vec::new();
    for z in all {
        let base_z_f64 = f64::from(z) - raft_top;
        let mut regions: Vec<RegionLayerProposal> = Vec::new();
        for (index, spec) in specs.iter().enumerate() {
            let native = native_global[index]
                .iter()
                .position(|candidate| (*candidate - z).abs() < COINCIDENCE as f32);
            if let Some(step_index) = native {
                let effective = if spec.has_explicit() {
                    explicit_credit(spec, step_index)
                } else if (last_z[index] - 0.0_f32).abs() < COINCIDENCE as f32 {
                    spec.first_layer_height_mm as f32
                } else {
                    spec.layer_height_mm as f32
                };
                regions.push(expected_region(&spec.object_id, effective, false, 0.0));
                last_z[index] = z;
            } else {
                if base_z_f64 > spec.height_mm + COINCIDENCE {
                    continue;
                }
                let bottom = last_z[index];
                let height = z - bottom;
                if height > COINCIDENCE as f32 {
                    regions.push(expected_region(&spec.object_id, height, true, bottom));
                    last_z[index] = z;
                }
            }
        }
        if !regions.is_empty() {
            planes.push(ExpectedPlane { z, regions });
        }
    }
    planes
}

#[derive(Debug, Clone, PartialEq)]
struct ExpectedPlane {
    z: f32,
    regions: Vec<RegionLayerProposal>,
}

#[derive(Debug, Clone)]
struct Expected {
    raft_top: f64,
    native_local: Vec<Vec<f64>>,
    native_global: Vec<Vec<f32>>,
    raft_planes: Vec<ExpectedPlane>,
    model_planes: Vec<ExpectedPlane>,
    fast: bool,
}

fn validate_specs(specs: &[ObjectSpec], limits: &Limits) -> Result<(), CheckFailure> {
    if specs.len() > limits.max_objects as usize {
        return Err(CheckFailure::Fixture(format!(
            "{} objects exceed max_objects={}",
            specs.len(),
            limits.max_objects
        )));
    }
    let cap_mm = quanta_mm(limits.max_z_quanta);
    let mut ids = BTreeSet::new();
    for spec in specs {
        if spec.object_id.is_empty() || !ids.insert(spec.object_id.as_str()) {
            return Err(CheckFailure::Fixture(format!(
                "object id {:?} is empty or duplicated",
                spec.object_id
            )));
        }
        for (label, value) in [
            ("height", spec.height_mm),
            ("layer_height", spec.layer_height_mm),
            ("first_layer_height", spec.first_layer_height_mm),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(CheckFailure::Fixture(format!(
                    "{}: {label} {value} is not finite and positive",
                    spec.object_id
                )));
            }
            if value > cap_mm {
                return Err(CheckFailure::Fixture(format!(
                    "{}: {label} {value} mm exceeds max_z_quanta ({cap_mm} mm)",
                    spec.object_id
                )));
            }
        }
        if spec.raft_layers > limits.max_raft_layers {
            return Err(CheckFailure::Fixture(format!(
                "{}: {} raft layers exceed max_raft_layers={}",
                spec.object_id, spec.raft_layers, limits.max_raft_layers
            )));
        }
        let mut previous = 0.0_f64;
        for z in &spec.layer_zs_mm {
            if !(z.is_finite() && *z > 0.0) || *z <= previous {
                return Err(CheckFailure::Fixture(format!(
                    "{}: explicit tops must be finite, positive and strictly increasing",
                    spec.object_id
                )));
            }
            if *z > cap_mm {
                return Err(CheckFailure::Fixture(format!(
                    "{}: explicit top {z} mm exceeds max_z_quanta",
                    spec.object_id
                )));
            }
            previous = *z;
        }
        if let Some(quanta) = &spec.quanta {
            let consistent = quanta.layer_zs.len() == spec.layer_zs_mm.len()
                && quanta.height == quanta_of(spec.height_mm)
                && quanta.layer_height == quanta_of(spec.layer_height_mm)
                && quanta.first_layer_height == quanta_of(spec.first_layer_height_mm)
                && quanta
                    .layer_zs
                    .iter()
                    .zip(&spec.layer_zs_mm)
                    .all(|(left, right)| quanta_mm(*left).to_bits() == right.to_bits());
            if !consistent {
                return Err(CheckFailure::Fixture(format!(
                    "{}: the millimeter fields do not match the quantum form",
                    spec.object_id
                )));
            }
        }
    }
    Ok(())
}

fn expected_table(specs: &[ObjectSpec], limits: &Limits) -> Result<Expected, CheckFailure> {
    if specs.is_empty() {
        return Err(CheckFailure::Fixture(
            "a layer fixture needs at least one object".to_owned(),
        ));
    }
    validate_specs(specs, limits)?;
    let raft_top = model_raft_top(specs);
    let cap_mm = quanta_mm(limits.max_z_quanta);
    if raft_top > cap_mm {
        return Err(CheckFailure::Fixture(format!(
            "combined raft top {raft_top} mm exceeds max_z_quanta ({cap_mm} mm)"
        )));
    }

    let mut native_local = Vec::with_capacity(specs.len());
    for spec in specs {
        native_local.push(native_tops_mm(spec, limits)?);
    }
    for (spec, tops) in specs.iter().zip(&native_local) {
        if let Some(last) = tops.last() {
            let global = raft_top + *last;
            if !global.is_finite() || global > cap_mm {
                return Err(CheckFailure::Fixture(format!(
                    "{}: combined global top {global} mm exceeds max_z_quanta ({cap_mm} mm)",
                    spec.object_id
                )));
            }
        }
    }

    let native_global: Vec<Vec<f32>> = native_local
        .iter()
        .map(|tops| tops.iter().map(|z| (raft_top + *z) as f32).collect())
        .collect();
    let raft_planes = model_raft_planes(specs);
    let fast = is_fast_path(specs);
    let model_planes = if fast {
        model_fast_planes(specs, raft_top, limits)?
    } else {
        model_general_planes(specs, raft_top, &native_global)
    };

    Ok(Expected {
        raft_top,
        native_local,
        native_global,
        raft_planes,
        model_planes,
        fast,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Observation and the one checker
// ─────────────────────────────────────────────────────────────────────────────

/// The complete kernel observation for one fixture.
#[derive(Debug, Clone)]
struct Observation {
    sut_raft_top: f64,
    local_f64: Vec<Vec<f64>>,
    local_f32: Vec<Vec<f32>>,
    raft_planes: Vec<MergedLayer>,
    merged: Vec<MergedLayer>,
    planned: Vec<LayerProposal>,
}

fn observe(specs: &[ObjectSpec]) -> Observation {
    let plans: Vec<ObjectPlan> = specs.iter().map(ObjectSpec::plan).collect();
    let sut_raft_top = raft_top(&plans);
    let local_f64 = plans.iter().map(generate_object_layers_f64).collect();
    let local_f32 = plans
        .iter()
        .map(|plan| generate_object_layers(plan, sut_raft_top))
        .collect();
    let raft_planes = merge_raft_sequences(&plans);
    let merged = merge_layer_sequences(&plans, sut_raft_top);

    let config = ConfigView::from_map(HashMap::new());
    let module =
        DefaultLayerPlanner::from_config(&config).expect("DefaultLayerPlanner ignores config");
    let objects = planning_objects(specs);
    let mut output = LayerPlanOutput::new();
    module
        .run_layer_planning(&objects, &mut output, &config)
        .expect("the generated planning objects are valid");
    Observation {
        sut_raft_top,
        local_f64,
        local_f32,
        raft_planes,
        merged,
        planned: output.layers().to_vec(),
    }
}

fn planes_of_merged(planes: &[MergedLayer]) -> Vec<ExpectedPlane> {
    planes
        .iter()
        .map(|plane| ExpectedPlane {
            z: plane.z,
            regions: plane.regions.clone(),
        })
        .collect()
}

fn planes_of_planned(planes: &[LayerProposal]) -> Vec<ExpectedPlane> {
    planes
        .iter()
        .map(|plane| ExpectedPlane {
            z: plane.z,
            regions: plane.active_regions.clone(),
        })
        .collect()
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct LayerStats {
    planes: u64,
    raft_planes: u64,
    native_events: u64,
    catchups: u64,
    raft_entries: u64,
    participating: BTreeSet<String>,
    fast: bool,
    general: bool,
    explicit: bool,
    uniform: bool,
}

#[derive(Clone, Copy)]
enum PlaneClass {
    Raft,
    Merge,
    Plan,
}

impl PlaneClass {
    fn failure(self, message: String) -> CheckFailure {
        match self {
            Self::Raft => CheckFailure::RaftPlanes(message),
            Self::Merge => CheckFailure::Merge(message),
            Self::Plan => CheckFailure::PlanSequence(message),
        }
    }
}

fn compare_planes(
    label: &str,
    expected: &[ExpectedPlane],
    actual: &[ExpectedPlane],
    class: PlaneClass,
) -> Result<(), CheckFailure> {
    if expected.len() != actual.len() {
        return Err(class.failure(format!(
            "{label}: expected {} plane(s), observed {}",
            expected.len(),
            actual.len()
        )));
    }
    for (index, (want, got)) in expected.iter().zip(actual).enumerate() {
        if want.z.to_bits() != got.z.to_bits() {
            return Err(class.failure(format!(
                "{label}: plane {index} z is {} but expected {}",
                got.z, want.z
            )));
        }
        if got.regions.is_empty() {
            return Err(CheckFailure::PlanSequence(format!(
                "{label}: plane {index} (z {}) carries no regions",
                got.z
            )));
        }
        let want_ids: Vec<&str> = want
            .regions
            .iter()
            .map(|region| region.object_id.as_str())
            .collect();
        let got_ids: Vec<&str> = got
            .regions
            .iter()
            .map(|region| region.object_id.as_str())
            .collect();
        if want_ids != got_ids {
            return Err(CheckFailure::Association(format!(
                "{label}: plane {index} (z {}) object association {got_ids:?} != expected {want_ids:?}",
                got.z
            )));
        }
        if want.regions != got.regions {
            let (slot, (want_region, got_region)) = want
                .regions
                .iter()
                .zip(&got.regions)
                .enumerate()
                .find(|(_, (left, right))| left != right)
                .expect("a differing region exists");
            return Err(class.failure(format!(
                "{label}: plane {index} (z {}) region {slot} ({}) expected {want_region:?}, \
                 observed {got_region:?}",
                got.z, want_region.object_id
            )));
        }
    }
    Ok(())
}

/// Structural invariants of the observed plan, checked without the model.
fn check_observed_structure(observation: &Observation) -> Result<(), CheckFailure> {
    for (index, plane) in observation.merged.iter().enumerate() {
        if plane.regions.is_empty() {
            return Err(CheckFailure::PlanSequence(format!(
                "merged plane {index} (z {}) carries no regions",
                plane.z
            )));
        }
        if index > 0 && plane.z <= observation.merged[index - 1].z {
            return Err(CheckFailure::PlanSequence(format!(
                "merged z is not strictly ascending at plane {index}: {} after {}",
                plane.z,
                observation.merged[index - 1].z
            )));
        }
    }

    let mut in_raft_band = true;
    for (index, plane) in observation.planned.iter().enumerate() {
        if plane.active_regions.is_empty() {
            return Err(CheckFailure::PlanSequence(format!(
                "planned plane {index} (z {}) carries no regions",
                plane.z
            )));
        }
        if index > 0 && plane.z <= observation.planned[index - 1].z {
            return Err(CheckFailure::PlanSequence(format!(
                "planned z is not strictly ascending at plane {index}: {} after {}",
                plane.z,
                observation.planned[index - 1].z
            )));
        }
        if plane.is_raft && !in_raft_band {
            return Err(CheckFailure::PlanSequence(format!(
                "raft plane {index} (z {}) appears after a model plane",
                plane.z
            )));
        }
        if !plane.is_raft {
            in_raft_band = false;
        }
        for region in &plane.active_regions {
            if region.region_id != "0" {
                return Err(CheckFailure::PlanSequence(format!(
                    "plane {index}: region id {:?} is not the planner's \"0\"",
                    region.region_id
                )));
            }
            if !(region.effective_layer_height.is_finite() && region.effective_layer_height > 0.0) {
                return Err(CheckFailure::PlanSequence(format!(
                    "plane {index}: effective layer height {} is not finite and positive",
                    region.effective_layer_height
                )));
            }
            if !region.catchup_z_bottom.is_finite() {
                return Err(CheckFailure::PlanSequence(format!(
                    "plane {index}: catch-up bottom {} is not finite",
                    region.catchup_z_bottom
                )));
            }
            if region.is_catchup {
                if !(region.catchup_z_bottom < plane.z) {
                    return Err(CheckFailure::PlanSequence(format!(
                        "plane {index}: catch-up bottom {} is not below the plane z {}",
                        region.catchup_z_bottom, plane.z
                    )));
                }
            } else if region.catchup_z_bottom.to_bits() != 0.0_f32.to_bits() {
                return Err(CheckFailure::PlanSequence(format!(
                    "plane {index}: a native region carries catch-up bottom {}",
                    region.catchup_z_bottom
                )));
            }
        }
    }
    Ok(())
}

/// The **one** checker: both properties, the deterministic witnesses, the
/// mutation controls and the shrink-state revalidation all call it.
fn check_layer_sequence(
    specs: &[ObjectSpec],
    limits: &Limits,
    observation: &Observation,
) -> Result<LayerStats, CheckFailure> {
    let expected = expected_table(specs, limits)?;

    if observation.sut_raft_top.to_bits() != expected.raft_top.to_bits() {
        return Err(CheckFailure::RaftTop(format!(
            "raft_top is {} but the model says {}",
            observation.sut_raft_top, expected.raft_top
        )));
    }

    check_native_lists(specs, &expected, observation)?;
    check_observed_structure(observation)?;

    // An ended object must not acquire later catch-up events.
    for plane in &observation.merged {
        let base = f64::from(plane.z) - expected.raft_top;
        for region in &plane.regions {
            if !region.is_catchup {
                continue;
            }
            let spec = specs
                .iter()
                .find(|spec| spec.object_id == region.object_id)
                .expect("association is checked below");
            if base > spec.height_mm + COINCIDENCE {
                return Err(CheckFailure::PlanSequence(format!(
                    "{}: catch-up plane z {} is above the object's print top {}",
                    spec.object_id, plane.z, spec.height_mm
                )));
            }
        }
    }

    compare_planes(
        "raft prefix",
        &expected.raft_planes,
        &planes_of_merged(&observation.raft_planes),
        PlaneClass::Raft,
    )?;
    compare_planes(
        "merged model planes",
        &expected.model_planes,
        &planes_of_merged(&observation.merged),
        PlaneClass::Merge,
    )?;

    let pushed = planes_of_planned(&observation.planned);
    let raft_len = expected.raft_planes.len();
    if pushed.len() != raft_len + expected.model_planes.len() {
        return Err(CheckFailure::PlanSequence(format!(
            "the pushed sequence has {} plane(s); the model expects {raft_len} raft + {} model",
            pushed.len(),
            expected.model_planes.len()
        )));
    }
    if observation.planned[..raft_len]
        .iter()
        .any(|plane| !plane.is_raft)
    {
        return Err(CheckFailure::PlanSequence(
            "a plane inside the raft prefix is not marked is_raft".to_owned(),
        ));
    }
    if observation.planned[raft_len..]
        .iter()
        .any(|plane| plane.is_raft)
    {
        return Err(CheckFailure::PlanSequence(
            "a model plane is marked is_raft".to_owned(),
        ));
    }
    compare_planes(
        "raft prefix push",
        &expected.raft_planes,
        &pushed[..raft_len],
        PlaneClass::Raft,
    )?;
    compare_planes(
        "model plane push",
        &expected.model_planes,
        &pushed[raft_len..],
        PlaneClass::Plan,
    )?;

    Ok(stats_of(&expected, specs))
}

fn check_native_lists(
    specs: &[ObjectSpec],
    expected: &Expected,
    observation: &Observation,
) -> Result<(), CheckFailure> {
    if observation.local_f64.len() != specs.len() || observation.local_f32.len() != specs.len() {
        return Err(CheckFailure::NativeTops(format!(
            "the SUT returned {} f64 and {} f32 sequence(s) for {} objects",
            observation.local_f64.len(),
            observation.local_f32.len(),
            specs.len()
        )));
    }
    let plans: Vec<ObjectPlan> = specs.iter().map(ObjectSpec::plan).collect();
    for (index, spec) in specs.iter().enumerate() {
        let want_f64 = &expected.native_local[index];
        let got_f64 = &observation.local_f64[index];
        if want_f64.len() != got_f64.len()
            || want_f64
                .iter()
                .zip(got_f64)
                .any(|(left, right)| left.to_bits() != right.to_bits())
        {
            return Err(CheckFailure::NativeTops(format!(
                "{}: object-local f64 tops {got_f64:?} != expected {want_f64:?}",
                spec.object_id
            )));
        }
        let want_f32 = &expected.native_global[index];
        let got_f32 = &observation.local_f32[index];
        if want_f32.len() != got_f32.len()
            || want_f32
                .iter()
                .zip(got_f32)
                .any(|(left, right)| left.to_bits() != right.to_bits())
        {
            return Err(CheckFailure::NativeTops(format!(
                "{}: raft-offset f32 tops {got_f32:?} != expected {want_f32:?}",
                spec.object_id
            )));
        }
        if spec.has_explicit() {
            for step_index in 0..spec.layer_zs_mm.len() {
                let sut = explicit_schedule_step(&plans[index], step_index);
                let model = explicit_credit(spec, step_index);
                if sut.to_bits() != model.to_bits() {
                    return Err(CheckFailure::NativeTops(format!(
                        "{}: explicit_schedule_step({step_index}) is {sut} but the model says {model}",
                        spec.object_id
                    )));
                }
            }
        }
    }

    // Derived f32 conversion envelope: on the quantum grid a global top is an
    // integer multiple of 1/1024 mm below 64 mm, i.e. below 2^17 quanta, so the
    // `as f32` cast at the kernel boundary is exact and every comparison above
    // is an exact dyadic comparison rather than a tolerance.
    if specs.iter().all(|spec| spec.quanta.is_some()) {
        for (spec, tops) in specs.iter().zip(&expected.native_local) {
            for z in tops {
                let global = expected.raft_top + *z;
                let narrowed = global as f32;
                if f64::from(narrowed).to_bits() != global.to_bits() {
                    return Err(CheckFailure::Oracle(format!(
                        "{}: global top {global} mm is not f32-exact; the profile's quantum \
                         ceiling no longer supports exact comparison",
                        spec.object_id
                    )));
                }
            }
        }
    }
    Ok(())
}

fn stats_of(expected: &Expected, specs: &[ObjectSpec]) -> LayerStats {
    let mut stats = LayerStats {
        fast: expected.fast,
        general: !expected.fast,
        explicit: specs.iter().any(ObjectSpec::has_explicit),
        uniform: specs.iter().any(|spec| !spec.has_explicit()),
        ..LayerStats::default()
    };
    for plane in &expected.model_planes {
        stats.planes += 1;
        for region in &plane.regions {
            if region.is_catchup {
                stats.catchups += 1;
            } else {
                stats.native_events += 1;
            }
            stats.participating.insert(region.object_id.clone());
        }
    }
    for plane in &expected.raft_planes {
        stats.raft_planes += 1;
        stats.raft_entries += plane.regions.len() as u64;
    }
    stats
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathClass {
    /// All objects share one first height and one layer height, and none carries
    /// an explicit schedule: `merge_same_height`.
    Fast,
    /// Object 0 carries an explicit schedule (which alone forces the general
    /// path); every other object is uniform.
    General,
}

const PATH_CLASSES: [PathClass; 2] = [PathClass::Fast, PathClass::General];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RaftClass {
    WithRaft,
    WithoutRaft,
}

const RAFT_CLASSES: [RaftClass; 2] = [RaftClass::WithRaft, RaftClass::WithoutRaft];

#[derive(Debug, Clone, PartialEq)]
struct LayerCase {
    path: PathClass,
    raft: RaftClass,
    specs: Vec<ObjectSpec>,
}

/// `raw[0]` object count, `raw[1]` raft layers, four four-field object blocks
/// from `raw[2]`, `raw[18]` the explicit first credit.
const RAW_LEN: usize = 19;
const CREDIT_SLOT: usize = 18;

fn raw_block(index: usize) -> usize {
    2 + 4 * index
}

/// Inclusive `lo..=hi` pick; an empty range is a generator bug and panics
/// (a failure, never a rejection).
fn pick(raw: u32, lo: u64, hi: u64) -> u64 {
    assert!(lo <= hi, "generator range {lo}..={hi} is empty");
    let span = hi - lo + 1;
    assert!(
        span <= u64::from(u32::MAX),
        "generator range {lo}..={hi} is too wide"
    );
    lo + u64::from(raw) % span
}

/// Height picker that deliberately reaches both band-midpoint boundaries: raw
/// `0` is the just-emitted boundary, raw `1` the exact-break boundary.
fn pick_height(raw: u32, lo: u64, hi: u64) -> u64 {
    match raw % 3 {
        0 => lo,
        1 => hi,
        _ => pick(raw, lo, hi),
    }
}

fn uniform_dims(
    k: u64,
    span: u64,
    first_raw: u32,
    step_raw: u32,
    first_high: u64,
    raft_bound: Option<(u32, u64)>,
) -> (u64, u64) {
    let first_cap = (span / 8).max(1).min(first_high.max(1));
    let first = pick(first_raw, 1, first_cap);
    let mut step_cap = ((span - first) / k).max(1);
    if let Some((layers, raft_cap)) = raft_bound {
        if layers > 1 {
            step_cap =
                step_cap.min(((raft_cap.saturating_sub(first)) / u64::from(layers - 1)).max(1));
        }
    }
    let step = pick(step_raw, 1, step_cap);
    (first, step)
}

fn uniform_spec(
    index: usize,
    k: u64,
    first: u64,
    step: u64,
    height_raw: u32,
    raft_layers: u32,
) -> ObjectSpec {
    let (low, high) = exact_k_height_range(k, first, step);
    quantized_spec(
        index,
        Quanta {
            height: pick_height(height_raw, low, high),
            layer_height: step,
            first_layer_height: first,
            layer_zs: Vec::new(),
        },
        raft_layers,
    )
}

fn explicit_spec(
    index: usize,
    k: u64,
    first_top: u64,
    gap: u64,
    credit: u64,
    height_raw: u32,
) -> ObjectSpec {
    let layer_zs: Vec<u64> = (0..k)
        .map(|step_index| first_top + step_index * gap)
        .collect();
    let last = *layer_zs.last().expect("k >= 1");
    // The height variants reach the catch-up gate's exact boundary: equal to the
    // last authored top, one quantum below the first, or anywhere between.
    let low = (first_top - 1).max(1);
    let height = match height_raw % 3 {
        0 => last,
        1 => low,
        _ => pick(height_raw, low, last.max(low)),
    };
    quantized_spec(
        index,
        Quanta {
            height,
            layer_height: gap,
            first_layer_height: credit,
            layer_zs,
        },
        0,
    )
}

fn build_case(path: PathClass, raft: RaftClass, raw: [u32; RAW_LEN], limits: &Limits) -> LayerCase {
    let local_cap = (limits.max_z_quanta / 2).max(2);
    let raft_cap = (local_cap / 2).max(1);
    let min_objects: u32 = match (path, raft) {
        (PathClass::Fast, RaftClass::WithoutRaft) => 1,
        _ => 2,
    };
    assert!(
        u64::from(limits.max_objects) >= u64::from(min_objects),
        "the profile needs at least {min_objects} objects"
    );
    let count = pick(
        raw[0],
        u64::from(min_objects),
        u64::from(limits.max_objects),
    ) as usize;
    let raft_layers = match raft {
        RaftClass::WithoutRaft => 0,
        RaftClass::WithRaft => u32::try_from(pick(raw[1], 1, u64::from(limits.max_raft_layers)))
            .expect("raft layer count"),
    };
    let steps: Vec<u64> = (0..count)
        .map(|index| {
            pick(
                raw[raw_block(index) + 2],
                1,
                u64::from(limits.max_layer_steps),
            )
        })
        .collect();

    let specs = match path {
        PathClass::Fast => {
            let first = pick(raw[raw_block(0)], 1, (local_cap / 8).max(1));
            let widest = steps.iter().copied().max().expect("at least one object");
            let mut step_cap = ((local_cap - first) / widest).max(1);
            if raft_layers > 1 {
                step_cap = step_cap
                    .min(((raft_cap.saturating_sub(first)) / u64::from(raft_layers - 1)).max(1));
            }
            let step = pick(raw[raw_block(0) + 1], 1, step_cap);
            (0..count)
                .map(|index| {
                    let carrier = if index == 1 { raft_layers } else { 0 };
                    uniform_spec(
                        index,
                        steps[index],
                        first,
                        step,
                        raw[raw_block(index) + 3],
                        carrier,
                    )
                })
                .collect()
        }
        PathClass::General => {
            let first_top = pick(raw[raw_block(0)], 2, (local_cap / 4).max(2));
            let gap_cap = ((local_cap - first_top) / steps[0]).max(1);
            let gap = pick(raw[raw_block(0) + 1], 1, gap_cap);
            let credit = pick(raw[CREDIT_SLOT], 1, (first_top - 1).max(1));
            let mut out = Vec::with_capacity(count);
            out.push(explicit_spec(
                0,
                steps[0],
                first_top,
                gap,
                credit,
                raw[raw_block(0) + 3],
            ));
            for index in 1..count {
                // Object 1's first plane always precedes object 0's first
                // authored top, so object 0 is guaranteed a catch-up there.
                let first_high = if index == 1 {
                    first_top - 1
                } else {
                    (local_cap / 8).max(1)
                };
                let raft_bound = if index == 1 && raft_layers > 0 {
                    Some((raft_layers, raft_cap))
                } else {
                    None
                };
                let (first, step) = uniform_dims(
                    steps[index],
                    local_cap,
                    raw[raw_block(index)],
                    raw[raw_block(index) + 1],
                    first_high,
                    raft_bound,
                );
                let carrier = if index == 1 { raft_layers } else { 0 };
                out.push(uniform_spec(
                    index,
                    steps[index],
                    first,
                    step,
                    raw[raw_block(index) + 3],
                    carrier,
                ));
            }
            out
        }
    };

    LayerCase { path, raft, specs }
}

fn case_strategy(
    limits: &Limits,
    paths: Vec<PathClass>,
    rafts: Vec<RaftClass>,
) -> BoxedStrategy<LayerCase> {
    let limits = *limits;
    (
        prop::sample::select(paths).no_shrink(),
        prop::sample::select(rafts).no_shrink(),
        any::<[u32; RAW_LEN]>(),
    )
        .prop_map(move |(path, raft, raw)| build_case(path, raft, raw, &limits))
        .boxed()
}

fn layer_case_strategy(limits: &Limits) -> BoxedStrategy<LayerCase> {
    case_strategy(limits, PATH_CLASSES.to_vec(), RAFT_CLASSES.to_vec())
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic witnesses
// ─────────────────────────────────────────────────────────────────────────────

fn checked_witness(specs: &[ObjectSpec], limits: &Limits) -> (Observation, LayerStats) {
    let observation = observe(specs);
    let stats = check_layer_sequence(specs, limits, &observation).unwrap_or_else(|error| {
        let ids: Vec<&str> = specs.iter().map(|spec| spec.object_id.as_str()).collect();
        panic!("witness {ids:?} rejected: {error}")
    });
    (observation, stats)
}

fn region_at<'a>(plane: &'a ExpectedPlane, object_id: &str) -> &'a RegionLayerProposal {
    plane
        .regions
        .iter()
        .find(|region| region.object_id == object_id)
        .unwrap_or_else(|| panic!("plane z={} has no region for {object_id}", plane.z))
}

fn explicit_ab_witness() -> Vec<ObjectSpec> {
    vec![
        witness_spec(
            0,
            quanta_of(1.0),
            quanta_of(0.25),
            quanta_of(0.25),
            0,
            &[
                quanta_of(0.25),
                quanta_of(0.5),
                quanta_of(0.75),
                quanta_of(1.0),
            ],
        ),
        witness_spec(
            1,
            quanta_of(0.75),
            quanta_of(0.375),
            quanta_of(0.375),
            0,
            &[quanta_of(0.375), quanta_of(0.75)],
        ),
    ]
}

fn raft_witness() -> Vec<ObjectSpec> {
    vec![
        witness_spec(
            0,
            quanta_of(0.625),
            quanta_of(0.125),
            quanta_of(0.25),
            2,
            &[],
        ),
        witness_spec(1, quanta_of(0.75), quanta_of(0.25), quanta_of(0.25), 0, &[]),
    ]
}

fn one_object_witness() -> Vec<ObjectSpec> {
    vec![witness_spec(
        0,
        quanta_of(1.375),
        quanta_of(0.25),
        quanta_of(0.25),
        0,
        &[],
    )]
}

fn equal_height_fast_witness() -> Vec<ObjectSpec> {
    vec![
        witness_spec(
            0,
            quanta_of(0.875),
            quanta_of(0.25),
            quanta_of(0.25),
            0,
            &[],
        ),
        witness_spec(
            1,
            quanta_of(0.4375),
            quanta_of(0.25),
            quanta_of(0.25),
            0,
            &[],
        ),
    ]
}

fn mixed_witness() -> Vec<ObjectSpec> {
    vec![
        witness_spec(
            0,
            quanta_of(0.75),
            quanta_of(0.25),
            quanta_of(0.25),
            0,
            &[quanta_of(0.375), quanta_of(0.75)],
        ),
        witness_spec(
            1,
            quanta_of(0.875),
            quanta_of(0.25),
            quanta_of(0.25),
            0,
            &[],
        ),
    ]
}

fn exact_end_specs(height_quanta: u64) -> Vec<ObjectSpec> {
    vec![witness_spec(
        0,
        height_quanta,
        quanta_of(0.25),
        quanta_of(0.25),
        0,
        &[],
    )]
}

fn fractional_height_specs() -> Vec<ObjectSpec> {
    vec![raw_spec(0, 0.55, 0.2, 0.2, 0, &[])]
}

fn no_native_specs() -> Vec<ObjectSpec> {
    vec![raw_spec(0, 0.2, 0.2, 0.5, 0, &[])]
}

fn ended_object_specs() -> Vec<ObjectSpec> {
    vec![
        raw_spec(0, 0.2, 0.2, 0.5, 0, &[]),
        raw_spec(1, 0.75, 0.25, 0.5, 0, &[0.5, 0.75]),
    ]
}

fn explicit_first_credit_witness() -> Vec<ObjectSpec> {
    vec![
        witness_spec(
            0,
            quanta_of(1.0),
            quanta_of(0.25),
            quanta_of(0.375),
            0,
            &[quanta_of(0.5), quanta_of(0.75), quanta_of(1.0)],
        ),
        witness_spec(1, quanta_of(0.25), quanta_of(0.25), quanta_of(0.25), 0, &[]),
    ]
}

/// Non-dyadic probe: the general merge's catch-up height is one `f32`
/// subtraction of two `f32` planes, so it differs from the `f64` difference
/// rounded once. This is a characterisation of current numeric behaviour, not a
/// parity claim.
fn rounding_probe_specs() -> Vec<ObjectSpec> {
    vec![
        raw_spec(0, 0.5, 0.2, 0.25, 0, &[0.25, 0.45]),
        raw_spec(1, 0.4, 0.1, 0.2, 0, &[0.3]),
    ]
}

#[derive(Debug, Default)]
struct WitnessStats {
    explicit_planes: u64,
    raft_planes: u64,
    raft_free_offset_planes: u64,
    fast_planes: u64,
    general_planes: u64,
    empty_cases: u64,
    catchups: u64,
    native_events: u64,
    probe_catchup_height: u64,
}

fn plane_z(observation: &Observation) -> Vec<f32> {
    observation.merged.iter().map(|plane| plane.z).collect()
}

fn witness_observations(limits: &Limits) -> WitnessStats {
    let mut out = WitnessStats::default();

    // ── Explicit A/B: first credits, catch-up from zero, unfiltered native tops
    let ab = explicit_ab_witness();
    let (observation, stats) = checked_witness(&ab, limits);
    assert_eq!(
        plane_z(&observation),
        vec![0.25_f32, 0.375, 0.5, 0.75, 1.0],
        "A/B global planes"
    );
    let expected = expected_table(&ab, limits).expect("A/B model");
    let first = &expected.model_planes[0];
    assert_eq!(region_at(first, "obj-0").effective_layer_height, 0.25_f32);
    assert!(!region_at(first, "obj-0").is_catchup);
    let b_early = region_at(first, "obj-1");
    assert!(b_early.is_catchup, "B's 0.25 plane is a catch-up");
    assert_eq!(b_early.catchup_z_bottom, 0.0_f32);
    assert_eq!(b_early.effective_layer_height, 0.25_f32);
    let second = &expected.model_planes[1];
    assert!(!region_at(second, "obj-1").is_catchup);
    assert_eq!(
        region_at(second, "obj-1").effective_layer_height,
        0.375_f32,
        "B's 0.375 native plane keeps the first-layer credit after its catch-up"
    );
    assert_eq!(
        region_at(second, "obj-0").effective_layer_height,
        0.125_f32,
        "A's 0.375 plane is a catch-up from 0.25"
    );
    assert!(
        region_at(&expected.model_planes[4], "obj-0").effective_layer_height > 0.0
            && expected.model_planes[4].regions.len() == 1,
        "A's 1.0 plane is its last native plane and B has ended"
    );
    assert_eq!(stats.catchups, 3, "A/B catch-up regions");
    out.explicit_planes += stats.planes;
    out.catchups += stats.catchups;
    out.native_events += stats.native_events;

    // ── Raft prefix, common offset, raft-free object
    let raft = raft_witness();
    let (observation, stats) = checked_witness(&raft, limits);
    let expected = expected_table(&raft, limits).expect("raft model");
    assert_eq!(expected.raft_top, 0.375_f64, "greatest final raft top");
    assert_eq!(
        plane_z(&observation),
        vec![0.625_f32, 0.75, 0.875, 1.0, 1.125],
        "raft offset is added to both objects' model tops"
    );
    assert_eq!(expected.raft_planes.len(), 2);
    assert_eq!(expected.raft_planes[0].z, 0.25_f32);
    assert_eq!(expected.raft_planes[1].z, 0.375_f32);
    for plane in &expected.raft_planes {
        assert_eq!(plane.regions.len(), 1, "only the raft-bearing object");
        assert_eq!(plane.regions[0].object_id, "obj-0");
    }
    assert_eq!(
        expected.raft_planes[0].regions[0].effective_layer_height,
        0.25_f32
    );
    assert_eq!(
        expected.raft_planes[1].regions[0].effective_layer_height,
        0.125_f32
    );
    assert_eq!(
        observation.local_f32[1][0], 0.625_f32,
        "the raft-free object's first model top carries the global raft offset"
    );
    assert_eq!(
        region_at(&expected.model_planes[0], "obj-1").effective_layer_height,
        0.25_f32
    );
    assert_eq!(stats.raft_entries, 2);
    assert_eq!(
        observation.planned[..2]
            .iter()
            .filter(|p| p.is_raft)
            .count(),
        2
    );
    assert_eq!(
        observation.planned[2..]
            .iter()
            .filter(|p| p.is_raft)
            .count(),
        0
    );
    out.raft_planes += stats.raft_planes;
    out.raft_free_offset_planes += 1;
    out.native_events += stats.native_events;

    // ── One object, uniform fallback, fast path
    let single = one_object_witness();
    let (observation, stats) = checked_witness(&single, limits);
    assert!(
        stats.fast && !stats.general,
        "a lone uniform object is the fast path"
    );
    assert_eq!(
        plane_z(&observation),
        vec![0.25_f32, 0.5, 0.75, 1.0, 1.25],
        "the uniform fallback keeps every band whose midpoint is below 1.375 mm"
    );
    let plans: Vec<ObjectPlan> = single.iter().map(ObjectSpec::plan).collect();
    // test-quality: determinism pin -- `merge_layer_sequences` must dispatch an
    // all-same-height, schedule-free plan to `merge_same_height`; this compares
    // two SUT entry points (the protected dispatch surface), not the model.
    assert_eq!(
        planes_of_merged(&merge_same_height(&plans, raft_top(&plans))),
        planes_of_merged(&observation.merged),
        "the fast-path dispatch and its kernel disagree"
    );
    out.fast_planes += stats.planes;

    // ── Equal-height fast path with a shorter object
    let equal = equal_height_fast_witness();
    let (observation, stats) = checked_witness(&equal, limits);
    assert!(stats.fast);
    let table: Vec<(f32, Vec<&str>)> = observation
        .merged
        .iter()
        .map(|plane| {
            (
                plane.z,
                plane
                    .regions
                    .iter()
                    .map(|region| region.object_id.as_str())
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        table,
        vec![
            (0.25_f32, vec!["obj-0", "obj-1"]),
            (0.5_f32, vec!["obj-0", "obj-1"]),
            (0.75_f32, vec!["obj-0"]),
        ],
        "the fast path filters each object by its own band midpoint"
    );
    out.fast_planes += stats.planes;

    // ── Mixed explicit/uniform: the general path
    let mixed = mixed_witness();
    let (observation, stats) = checked_witness(&mixed, limits);
    assert!(stats.general && !stats.fast);
    let shape: Vec<(f32, Vec<(&str, bool)>)> = observation
        .merged
        .iter()
        .map(|plane| {
            (
                plane.z,
                plane
                    .regions
                    .iter()
                    .map(|region| (region.object_id.as_str(), region.is_catchup))
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            (0.25_f32, vec![("obj-0", true), ("obj-1", false)]),
            (0.375_f32, vec![("obj-0", false), ("obj-1", true)]),
            (0.5_f32, vec![("obj-0", true), ("obj-1", false)]),
            (0.75_f32, vec![("obj-0", false), ("obj-1", false)]),
        ],
        "the mixed schedule's native/catch-up table"
    );
    assert_eq!(stats.catchups, 3);
    out.general_planes += stats.planes;
    out.catchups += stats.catchups;
    out.native_events += stats.native_events;

    // ── Exact end and its neighbour one quantum above
    let (at_boundary, at_stats) = checked_witness(&exact_end_specs(quanta_of(0.625)), limits);
    assert_eq!(
        plane_z(&at_boundary),
        vec![0.25_f32, 0.5],
        "a band whose midpoint equals the height is excluded"
    );
    out.native_events += at_stats.native_events;
    let above = expected_table(&exact_end_specs(quanta_of(0.625) + 1), limits)
        .expect("boundary neighbour model");
    assert_eq!(
        above.model_planes.len(),
        3,
        "one quantum above the boundary admits the next band"
    );
    let (above_observed, above_stats) =
        checked_witness(&exact_end_specs(quanta_of(0.625) + 1), limits);
    assert_eq!(plane_z(&above_observed), vec![0.25_f32, 0.5, 0.75]);
    out.native_events += above_stats.native_events;

    // ── Fractional height outside the grid: 0.55 mm / 0.2 mm
    let fractional = fractional_height_specs();
    let (observation, stats) = checked_witness(&fractional, limits);
    assert_eq!(observation.local_f64[0].len(), 3);
    for (index, want) in [0.2_f64, 0.4, 0.6].into_iter().enumerate() {
        assert!(
            (observation.local_f64[0][index] - want).abs() < 1e-12,
            "0.55 mm / 0.2 mm top {index} is {} not {want}",
            observation.local_f64[0][index]
        );
    }
    out.native_events += stats.native_events;

    // ── Empty / no-native cases: separate exact assertions
    let empty = no_native_specs();
    let (observation, stats) = checked_witness(&empty, limits);
    assert!(observation.local_f64[0].is_empty());
    assert!(observation.merged.is_empty());
    assert!(observation.planned.is_empty());
    assert!(observation.raft_planes.is_empty());
    assert_eq!(stats.planes, 0);
    assert_eq!(stats.native_events, 0);
    out.empty_cases += 1;

    let ended = ended_object_specs();
    let (observation, stats) = checked_witness(&ended, limits);
    assert_eq!(
        stats.participating.len(),
        1,
        "the ended object never participates"
    );
    assert!(stats.participating.contains("obj-1"));
    assert!(
        observation.merged.iter().all(|plane| plane
            .regions
            .iter()
            .all(|region| region.object_id != "obj-0")),
        "an object with no native plane acquires no catch-up above its height"
    );
    out.empty_cases += 1;
    out.native_events += stats.native_events;

    // ── Explicit first credit after a catch-up
    let credits = explicit_first_credit_witness();
    let (observation, stats) = checked_witness(&credits, limits);
    assert_eq!(plane_z(&observation), vec![0.25_f32, 0.5, 0.75, 1.0]);
    let expected = expected_table(&credits, limits).expect("first-credit model");
    assert!(region_at(&expected.model_planes[0], "obj-0").is_catchup);
    let first_native = &expected.model_planes[1];
    assert!(!region_at(first_native, "obj-0").is_catchup);
    assert_eq!(
        region_at(first_native, "obj-0").effective_layer_height,
        0.375_f32,
        "the first explicit native plane keeps first_layer_height after a catch-up"
    );
    assert_eq!(
        region_at(&expected.model_planes[2], "obj-0").effective_layer_height,
        0.25_f32,
        "later explicit credits are adjacent authored-top differences"
    );
    out.explicit_planes += stats.planes;
    out.catchups += stats.catchups;
    out.native_events += stats.native_events;

    // ── Rounding-boundary characterisation
    let probe = rounding_probe_specs();
    let (observation, stats) = checked_witness(&probe, limits);
    let expected = expected_table(&probe, limits).expect("rounding probe model");
    assert_eq!(plane_z(&observation), vec![0.25_f32, 0.3, 0.45]);
    let catchup = region_at(&expected.model_planes[1], "obj-0");
    assert!(catchup.is_catchup);
    assert_eq!(
        catchup.catchup_z_bottom.to_bits(),
        (0.25_f64 as f32).to_bits()
    );
    let two_roundings = (0.3_f64 as f32) - (0.25_f64 as f32);
    let single_rounding = (0.3_f64 - 0.25_f64) as f32;
    assert_eq!(
        catchup.effective_layer_height.to_bits(),
        two_roundings.to_bits(),
        "the general merge subtracts two f32 planes"
    );
    assert_ne!(
        two_roundings.to_bits(),
        single_rounding.to_bits(),
        "the probe must make the two roundings distinguishable"
    );
    out.probe_catchup_height += 1;
    out.catchups += stats.catchups;
    out.native_events += stats.native_events;

    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic mutation controls
// ─────────────────────────────────────────────────────────────────────────────

fn expect_failure(
    specs: &[ObjectSpec],
    limits: &Limits,
    baseline: &Observation,
    mutant: &Observation,
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    check_layer_sequence(specs, limits, baseline)
        .unwrap_or_else(|error| panic!("{what}: the unmodified observation was rejected: {error}"));
    match check_layer_sequence(specs, limits, mutant) {
        Ok(_) => panic!("{what}: the mutation was accepted"),
        Err(error) => assert!(
            matches_class(&error),
            "{what}: rejected for the wrong reason: {error}"
        ),
    }
}

fn witness_controls(limits: &Limits) {
    // ── A/B: dropped native top, association swap, catch-up edits ─────────
    let ab = explicit_ab_witness();
    let baseline = observe(&ab);
    check_layer_sequence(&ab, limits, &baseline).expect("unmodified A/B");

    let dropped_at = baseline
        .merged
        .iter()
        .position(|plane| plane.z.to_bits() == 1.0_f32.to_bits())
        .expect("A's final native plane");
    let mut dropped = baseline.clone();
    dropped.merged.remove(dropped_at);
    expect_failure(
        &ab,
        limits,
        &baseline,
        &dropped,
        "dropped native top z=1.0",
        is_merge,
    );

    let two_region = baseline
        .merged
        .iter()
        .position(|plane| plane.regions.len() == 2)
        .expect("a two-object plane");
    let mut swapped = baseline.clone();
    swapped.merged[two_region].regions.swap(0, 1);
    expect_failure(
        &ab,
        limits,
        &baseline,
        &swapped,
        "swapped object association",
        is_association,
    );

    let catchup_slot = baseline.merged[0]
        .regions
        .iter()
        .position(|region| region.is_catchup)
        .expect("B's catch-up at z=0.25");
    let mut moved_bottom = baseline.clone();
    moved_bottom.merged[0].regions[catchup_slot].catchup_z_bottom = 0.1_f32;
    expect_failure(
        &ab,
        limits,
        &baseline,
        &moved_bottom,
        "replaced catch-up bottom",
        is_merge,
    );

    let mut replaced_height = baseline.clone();
    replaced_height.merged[0].regions[catchup_slot].effective_layer_height = 0.2_f32;
    expect_failure(
        &ab,
        limits,
        &baseline,
        &replaced_height,
        "replaced catch-up height",
        is_merge,
    );

    // ── Raft: raft-free object's omitted global offset ────────────────────
    let raft = raft_witness();
    let raft_baseline = observe(&raft);
    check_layer_sequence(&raft, limits, &raft_baseline).expect("unmodified raft witness");
    let raft_top = raft_baseline.sut_raft_top;
    assert_eq!(raft_top, 0.375_f64);
    let mut unoffset = raft_baseline.clone();
    for z in &mut unoffset.local_f32[1] {
        *z = (*z as f64 - raft_top) as f32;
    }
    expect_failure(
        &raft,
        limits,
        &raft_baseline,
        &unoffset,
        "raft-free object without the global raft offset",
        is_native_tops,
    );

    // ── Raft prefix: a later raft plane's credit replaced by the first ────
    let mut raft_credit = raft_baseline.clone();
    raft_credit.raft_planes[1].regions[0].effective_layer_height = 0.25_f32;
    expect_failure(
        &raft,
        limits,
        &raft_baseline,
        &raft_credit,
        "raft prefix credit replaced",
        is_raft_planes,
    );

    // ── Plan sequence: empty-region plane and a raft plane after a model ──
    let mut empty_plane = raft_baseline.clone();
    empty_plane.planned.insert(
        empty_plane.planned.len() / 2,
        LayerProposal {
            z: 9.0,
            active_regions: Vec::new(),
            is_raft: false,
        },
    );
    expect_failure(
        &raft,
        limits,
        &raft_baseline,
        &empty_plane,
        "inserted empty-region plane",
        is_plan_sequence,
    );

    let first_model = raft_baseline
        .planned
        .iter()
        .position(|plane| !plane.is_raft)
        .expect("a model plane");
    let mut reordered = raft_baseline.clone();
    let moved = reordered.planned.remove(0);
    assert!(moved.is_raft, "index 0 is a raft plane");
    reordered.planned.insert(first_model, moved);
    expect_failure(
        &raft,
        limits,
        &raft_baseline,
        &reordered,
        "raft plane moved after a model plane",
        is_plan_sequence,
    );

    // ── Explicit first credit after a catch-up replaced by the later step ─
    let credits = explicit_first_credit_witness();
    let credit_baseline = observe(&credits);
    check_layer_sequence(&credits, limits, &credit_baseline)
        .expect("unmodified first-credit witness");
    let native_slot = credit_baseline.merged[1]
        .regions
        .iter()
        .position(|region| region.object_id == "obj-0" && !region.is_catchup)
        .expect("object 0's first native plane");
    let mut demoted = credit_baseline.clone();
    demoted.merged[1].regions[native_slot].effective_layer_height = 0.25_f32;
    expect_failure(
        &credits,
        limits,
        &credit_baseline,
        &demoted,
        "first explicit credit replaced by the later step",
        is_merge,
    );

    println!("layer_controls: every named mutation rejected by the shared checker");
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink probe
// ─────────────────────────────────────────────────────────────────────────────

fn complexity(case: &LayerCase) -> i128 {
    let mut total = case.specs.len() as i128;
    for spec in &case.specs {
        let quanta = spec
            .quanta
            .as_ref()
            .expect("generated fixtures are on the quanta grid");
        total += i128::from(quanta.height)
            + i128::from(quanta.layer_height)
            + i128::from(quanta.first_layer_height)
            + i128::from(spec.raft_layers);
        for z in &quanta.layer_zs {
            total += i128::from(*z);
        }
    }
    total
}

fn check_shrink_state(case: &LayerCase, path: PathClass, raft: RaftClass, limits: &Limits) {
    assert_eq!(case.path, path, "the path mode must survive shrinking");
    assert_eq!(case.raft, raft, "the raft mode must survive shrinking");
    assert!(!case.specs.is_empty(), "a shrunk case lost every object");
    assert!(
        case.specs.len() <= limits.max_objects as usize,
        "{} objects exceed max_objects",
        case.specs.len()
    );
    let observation = observe(&case.specs);
    let stats = check_layer_sequence(&case.specs, limits, &observation)
        .unwrap_or_else(|error| panic!("shrunk {path:?}/{raft:?} fixture: {error}"));
    assert!(
        stats.native_events > 0,
        "{path:?}/{raft:?}: the required native witness vanished"
    );
    assert_eq!(
        stats.fast,
        path == PathClass::Fast,
        "{path:?}/{raft:?}: the path dispatch changed"
    );
    assert_eq!(stats.general, path == PathClass::General);
    if path == PathClass::General {
        assert!(
            stats.explicit,
            "the general path is forced by an explicit schedule"
        );
    }
    if raft == RaftClass::WithRaft {
        assert!(
            stats.raft_entries > 0 && stats.raft_planes > 0,
            "the required raft witness vanished"
        );
    } else {
        assert_eq!(stats.raft_entries, 0);
        assert_eq!(observation.sut_raft_top.to_bits(), 0.0_f64.to_bits());
    }
}

fn shrink_probe(limits: &Limits) {
    for path in PATH_CLASSES {
        for raft in RAFT_CLASSES {
            let strategy = case_strategy(limits, vec![path], vec![raft]);

            // Phase 1: accept every simplification (as if each still failed) and
            // observe a real reduction from a non-minimal start.
            let mut runner = TestRunner::deterministic();
            let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
            let initial = tree.current();
            check_shrink_state(&initial, path, raft, limits);
            let initial_complexity = complexity(&initial);
            let mut accepted = 0_usize;
            while tree.simplify() {
                accepted += 1;
                assert!(
                    accepted < MAX_SHRINK_STEPS,
                    "{path:?}/{raft:?}: simplify never exhausted"
                );
                check_shrink_state(&tree.current(), path, raft, limits);
            }
            let minimal_complexity = complexity(&tree.current());
            assert!(accepted > 0, "{path:?}/{raft:?}: the tree never simplified");
            assert!(
                minimal_complexity < initial_complexity,
                "{path:?}/{raft:?}: no permitted reduction ({initial_complexity} -> {minimal_complexity})"
            );

            // Phase 2: alternate simplify/complicate, then settle.
            let mut runner = TestRunner::deterministic();
            let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
            // test-quality: determinism pin -- the same seeded runner must start
            // the shrink probe from the same generated fixture (protected
            // surface: the strategy's raw-to-fixture mapping).
            assert_eq!(
                tree.current(),
                initial,
                "the deterministic start is not stable"
            );
            let mut simplified = 0_usize;
            let mut complicated = 0_usize;
            let mut changed = false;
            while tree.simplify() {
                simplified += 1;
                assert!(
                    simplified < MAX_SHRINK_STEPS,
                    "{path:?}/{raft:?}: the alternation never ended"
                );
                let current = tree.current();
                check_shrink_state(&current, path, raft, limits);
                changed |= current != initial;
                if tree.complicate() {
                    complicated += 1;
                }
                check_shrink_state(&tree.current(), path, raft, limits);
            }
            assert!(
                simplified > 0 && changed,
                "{path:?}/{raft:?}: no real transition"
            );
            let exhausted = tree.current();
            assert!(
                !tree.simplify(),
                "an exhausted {path:?}/{raft:?} tree simplified again"
            );
            assert_eq!(
                tree.current(),
                exhausted,
                "a false simplify moved the value"
            );
            let mut restores = 0_usize;
            while tree.complicate() {
                restores += 1;
                assert!(
                    restores < MAX_SHRINK_STEPS,
                    "{path:?}/{raft:?}: complicate never ended"
                );
                check_shrink_state(&tree.current(), path, raft, limits);
            }
            let settled = tree.current();
            assert!(
                !tree.complicate(),
                "complicate after exhaustion must be false"
            );
            assert_eq!(
                tree.current(),
                settled,
                "a false complicate moved the value"
            );
            println!(
                "{path:?}/{raft:?}: phase1_simplify={accepted} complexity \
                 {initial_complexity}->{minimal_complexity} phase2_simplify={simplified} \
                 complicate={complicated} restores={restores}"
            );
        }
    }

    // The semantic selectors never shrink: a shrinking mode could switch a
    // witness's class mid-search.
    let mut runner = TestRunner::deterministic();
    let mut tree = prop::sample::select(PATH_CLASSES.to_vec())
        .no_shrink()
        .new_tree(&mut runner)
        .expect("path mode tree");
    let before = tree.current();
    // test-quality: determinism pin -- a persistent semantic mode must not
    // shrink (protected surface: fast/general path classification).
    assert!(!tree.simplify(), "the path mode selector simplified");
    assert!(!tree.complicate(), "the path mode selector complicated");
    assert_eq!(tree.current(), before);
    let mut tree = prop::sample::select(RAFT_CLASSES.to_vec())
        .no_shrink()
        .new_tree(&mut runner)
        .expect("raft mode tree");
    let before = tree.current();
    // test-quality: determinism pin -- a persistent semantic mode must not
    // shrink (protected surface: raft/no-raft witness retention).
    assert!(!tree.simplify(), "the raft mode selector simplified");
    assert!(!tree.complicate(), "the raft mode selector complicated");
    assert_eq!(tree.current(), before);
}

// ─────────────────────────────────────────────────────────────────────────────
// Properties
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_layer_sequence() {
    let run = PropertyRun::new(PROPERTY_ID_SEQ, PROPERTY_TEST_SEQ).expect("M-LAYERS-SEQ context");
    let limits = seq_limits(&run);
    let strategy = layer_case_strategy(&limits);

    let calls = Cell::new(0_u64);
    let native_events = Cell::new(0_u64);
    let catchups = Cell::new(0_u64);
    let raft_entries = Cell::new(0_u64);
    let participating = Cell::new(0_u64);
    let fast = Cell::new(0_u64);
    let general = Cell::new(0_u64);
    let explicit = Cell::new(0_u64);
    let uniform = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let observation = observe(&case.specs);
        let stats = match check_layer_sequence(&case.specs, &limits, &observation) {
            Ok(stats) => stats,
            Err(error) => return fail(format!("{:?}/{:?} case: {error}", case.path, case.raft)),
        };
        if stats.native_events == 0 {
            return fail(format!(
                "{:?}/{:?} case produced no native event",
                case.path, case.raft
            ));
        }
        native_events.set(native_events.get() + stats.native_events);
        catchups.set(catchups.get() + stats.catchups);
        raft_entries.set(raft_entries.get() + stats.raft_entries);
        participating.set(participating.get() + stats.participating.len() as u64);
        if stats.fast {
            fast.set(fast.get() + 1);
        }
        if stats.general {
            general.set(general.get() + 1);
        }
        if stats.explicit {
            explicit.set(explicit.get() + 1);
        }
        if stats.uniform {
            uniform.set(uniform.get() + 1);
        }
        Ok(())
    })
    .expect("M-LAYERS-SEQ native sequences, raft prefix and credits");

    assert!(calls.get() > 0, "the property callback never ran");
    for (name, counter) in [
        ("native events", &native_events),
        ("catch-ups", &catchups),
        ("raft entries", &raft_entries),
        ("participating objects", &participating),
        ("fast-path cases", &fast),
        ("general-path cases", &general),
        ("explicit cases", &explicit),
        ("uniform cases", &uniform),
    ] {
        assert!(counter.get() > 0, "the {name} counter was never observed");
    }
    println!(
        "M-LAYERS-SEQ accepted_calls={} native_events={} catchups={} raft_entries={} \
         participating={} fast={} general={} explicit={} uniform={}",
        calls.get(),
        native_events.get(),
        catchups.get(),
        raft_entries.get(),
        participating.get(),
        fast.get(),
        general.get(),
        explicit.get(),
        uniform.get()
    );
}

#[test]
fn proptest_layer_merge() {
    let run =
        PropertyRun::new(PROPERTY_ID_MERGE, PROPERTY_TEST_MERGE).expect("M-LAYERS-MERGE context");
    let limits = merge_limits(&run);
    let strategy = layer_case_strategy(&limits);

    let calls = Cell::new(0_u64);
    let native_events = Cell::new(0_u64);
    let catchups = Cell::new(0_u64);
    let raft_entries = Cell::new(0_u64);
    let participating = Cell::new(0_u64);
    let fast = Cell::new(0_u64);
    let general = Cell::new(0_u64);
    let explicit = Cell::new(0_u64);
    let uniform = Cell::new(0_u64);
    let empty = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let observation = observe(&case.specs);
        let stats = match check_layer_sequence(&case.specs, &limits, &observation) {
            Ok(stats) => stats,
            Err(error) => return fail(format!("{:?}/{:?} case: {error}", case.path, case.raft)),
        };
        native_events.set(native_events.get() + stats.native_events);
        catchups.set(catchups.get() + stats.catchups);
        raft_entries.set(raft_entries.get() + stats.raft_entries);
        participating.set(participating.get() + stats.participating.len() as u64);
        if stats.fast {
            fast.set(fast.get() + 1);
        }
        if stats.general {
            general.set(general.get() + 1);
        }
        if stats.explicit {
            explicit.set(explicit.get() + 1);
        }
        if stats.uniform {
            uniform.set(uniform.get() + 1);
        }
        if stats.planes == 0 {
            empty.set(empty.get() + 1);
            return fail("a generated merge case produced no model plane");
        }
        Ok(())
    })
    .expect("M-LAYERS-MERGE union, associations, catch-ups and raft prefix");

    assert!(calls.get() > 0, "the property callback never ran");
    for (name, counter) in [
        ("native events", &native_events),
        ("catch-ups", &catchups),
        ("raft entries", &raft_entries),
        ("participating objects", &participating),
        ("fast-path cases", &fast),
        ("general-path cases", &general),
        ("explicit cases", &explicit),
        ("uniform cases", &uniform),
    ] {
        assert!(counter.get() > 0, "the {name} counter was never observed");
    }
    assert_eq!(
        empty.get(),
        0,
        "every generated merge case is non-empty by construction"
    );
    println!(
        "M-LAYERS-MERGE accepted_calls={} native_events={} catchups={} raft_entries={} \
         participating={} fast={} general={} explicit={} uniform={}",
        calls.get(),
        native_events.get(),
        catchups.get(),
        raft_entries.get(),
        participating.get(),
        fast.get(),
        general.get(),
        explicit.get(),
        uniform.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Combined deterministic witness / control / shrink test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn packet15_witnesses_controls_and_shrinking() {
    let limits = checked_limits(&probe_bound_map()).expect("probe bounds");
    let stats = witness_observations(&limits);
    assert!(
        stats.explicit_planes > 0,
        "explicit witness planes unobserved"
    );
    assert!(stats.raft_planes > 0, "raft prefix planes unobserved");
    assert!(
        stats.raft_free_offset_planes > 0,
        "raft-free global offset unobserved"
    );
    assert!(stats.fast_planes > 0, "fast-path planes unobserved");
    assert!(stats.general_planes > 0, "general-path planes unobserved");
    assert!(
        stats.empty_cases > 0,
        "empty/no-native witnesses unobserved"
    );
    assert!(stats.catchups > 0, "catch-up regions unobserved");
    assert!(stats.native_events > 0, "native events unobserved");
    assert!(
        stats.probe_catchup_height > 0,
        "rounding probe catch-up unobserved"
    );

    witness_controls(&limits);
    invalid_profile_matrix();
    shrink_probe(&limits);

    println!(
        "packet15_witnesses_controls_and_shrinking: witnesses={:?} controls=rejected shrinking=green",
        stats
    );
}

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
        ("max_objects", 1),
        ("max_objects", 5),
        ("max_layer_steps", 3),
        ("max_layer_steps", 33),
        ("max_raft_layers", 1),
        ("max_raft_layers", 9),
        ("max_z_quanta", 1535),
        ("max_z_quanta", 65_537),
        ("max_z_quanta", 0),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        invalid(&format!("invalid {key}={value}"), bounds);
    }
    for key in ["max_objects", "max_layer_steps", "max_raft_layers"] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), u64::from(u32::MAX) + 1);
        assert!(matches!(
            checked_limits(&bounds),
            Err(DomainError::ArithmeticOverflow)
        ));
    }

    // The seq record's bound shape (three keys plus the fixed object count) is
    // accepted, and its own profile rejects the merge ceiling.
    let mut seq_profile = probe_bound_map();
    seq_profile.insert("max_objects".to_owned(), u64::from(SEQ_OBJECTS));
    checked_limits(&seq_profile).expect("the M-LAYERS-SEQ profile shape is accepted");

    // The generated fixtures themselves sit inside the registry bounds.
    let limits = checked_limits(&valid).expect("probe bounds");
    witness_observations(&limits);
}
