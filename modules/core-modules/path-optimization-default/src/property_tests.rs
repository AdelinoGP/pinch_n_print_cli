//! Opt-in property tests for this module's private path-ordering kernels.
//!
//! Charter (`docs/testing/properties.md` §§ `M-PATH-NEAREST`, `M-PATH-LOCKS`,
//! `M-PATH-GROUPS`):
//!
//! * `M-PATH-NEAREST` — `nearest_neighbor_permutation` on bounded integer
//!   geometry in `1/1024 mm` quanta. The checker proves the candidate-index and
//!   original-index bijections with `oracles::is_permutation` plus a local
//!   seen-set (empty and singleton included), derives the greedy **first**
//!   choice from its own exact squared-distance arithmetic and the documented
//!   `0.001 mm` earlier-index tie window, and requires every reversal flag to be
//!   `false`. It never claims a globally shortest route and never re-runs a
//!   copied greedy model.
//! * `M-PATH-LOCKS` — `coalesce_locked_candidates` and
//!   `PathOptimizationDefault::group_then_nearest_neighbor` on ADR-0062
//!   `order_lock` blocks. The checker independently derives every maximal
//!   authored-adjacent equal-`Some(lock)` run (first start, last end, indices in
//!   authored order), compares the coalesced candidates bit-exactly, then places
//!   the output positions through `oracles::locked_groups_are_contiguous` with
//!   explicit `false` reversal flags and its own disjointness/full-membership
//!   sweep. A block may move as a unit, never split or reverse.
//! * `M-PATH-GROUPS` — `PathOptimizationDefault::group_then_nearest_neighbor`
//!   on authored role/tool mixes. The checker derives the expected segment order
//!   from its own role table (skirt, then the staged wall block, then the
//!   documented non-wall groups ascending) and verifies ascending tool clusters,
//!   exact staged-wall order, per-partition membership and the tool-change
//!   records. It never asserts that the optimizer re-sorts `InnerWall` before
//!   `OuterWall`, and it never calls `role_group` to manufacture an expectation.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! `nearest_neighbor_permutation`, `coalesce_locked_candidates`,
//! `PathOptimizationDefault::role_group`/`group_then_nearest_neighbor` and the
//! private `NearestNeighborCandidate` / `ToolChangeRecord` structs are private in
//! the crate root. This module is a crate-internal unit-test child, so all of
//! them are reachable through `super::` without widening production visibility.
//!
//! # Independent expected model
//!
//! Every fixture is built here, from authored data, as integer quanta of
//! `1/1024 mm`; the production `OrderedEntityView` is then assembled through the
//! SDK fixture `ordered_entity_view_base`. Expected values come from:
//!
//! * `nearest` — the exact integer squared distance `dx² + dy²` of each start
//!   point from the origin. Generated starts lie on one axis at multiples of
//!   `GRID_STEP = 64` quanta (`0.0625 mm`), so two distinct shells are at least
//!   `0.0625 mm` apart — 62.5× the `0.001 mm` tie window — and two equal shells
//!   are bit-identical distances. The documented tie rule therefore reduces to
//!   "lowest index among the minimum-distance group", which the checker computes
//!   without consulting the kernel. The deterministic witnesses additionally pin
//!   the window itself: candidates at `33` and `32` quanta (a `0.0009765625 mm`
//!   difference, inside the window) select the **earlier**, farther candidate.
//! * `locks` — authored adjacent-run derivation (the same rule the kernel
//!   documents), evaluated from the case's own `order_lock` values. A run's
//!   candidate start is its first member's start and its end is its last
//!   member's end, so a mutated "first end used for the last" is caught.
//! * `groups` — the authored role table below: `Skirt` → skirt segment,
//!   `InnerWall`/`OuterWall` → the staged wall segment, and every other role to
//!   its authored numeric group (the documented arms plus the five characterised
//!   wildcard arms). Segment order is `Skirt < Wall < Group(3) < Group(4) < … <
//!   Group(9)`; that is the order the module documents (skirt first, perimeter
//!   modules own the wall sequence, then role groups ascend).
//!
//! ## Characterised wildcard role group (`role_group`'s `_ => 9`)
//!
//! Reviewed on disk: `role_group` names `Skirt` 0, `InnerWall` 1, `OuterWall` 2,
//! `ThinWall` 3, `{BottomSolidInfill, TopSolidInfill, SparseInfill,
//! BridgeInfill}` 4, `Ironing` 5, `SupportMaterial` 6, `SupportInterface` 7,
//! `{WipeTower, PrimeTower}` 8, `Custom(_)` 9 and `GapFill` 4, then falls
//! through `_ => 9`. The **five** variants that reach that wildcard are
//! `InternalBridgeInfill`, `InternalSolidInfill`, `RaftInfill`,
//! `SupportBaseInterface` and `Brim`; they therefore sort *after* the towers
//! rather than with the infill they resemble. The archived, design-only
//! `docs/spec_packets/_OLD/61_path-optimization-role-sequencing.md` says a new
//! infill role such as `InternalBridgeInfill` "would belong to the same group";
//! it is not an implementable packet and no live normative document mandates
//! that grouping. This module therefore **characterises** the current wildcard
//! grouping and pins it (a later confirmed contract change must update the
//! characterisation deliberately); it does not assert the archived suggestion
//! and it does not fix `role_group`.
//!
//! ## ADR-0062 lock units across partitions
//!
//! `group_then_nearest_neighbor` first forms units (maximal authored-adjacent
//! runs of equal `Some(order_lock)`, the host's own adjacency rule in
//! `validate_entity_order_locks`; untagged entities are singleton units) and only
//! then partitions by `tool_index` and wall-versus-`role_group`, assigning each
//! whole unit to the partition of its FIRST member. A lock spanning two role
//! groups, two tools or the wall/non-wall boundary is therefore one contiguous,
//! forward, unreversed window in the output (ADR-0062: a tag within one
//! `(layer, object, region)` is an atomic sequence; the host rejects a proposal
//! that splits one). The checkers derive units and partitions independently and
//! require exactly that; a split block is rejected by the same checker (see the
//! controls).
//!
//! ## `0.001 mm` tie window
//!
//! `nearest_neighbor_permutation` compares `|dist - best_dist| < 0.001` and, in
//! that band, keeps the earlier candidate index. The checker reproduces that
//! contract by selecting the lowest index whose distance lies strictly within
//! `0.001 mm` of the minimum. Generated distances are multiples of a
//! `0.0625 mm` shell, so the band collapses to exact equality there; only the
//! deterministic `(33, 0)` / `(32, 0)` witness exercises the band itself.
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
//! * `OrderedEntityView` (`crates/slicer-sdk/src/views.rs`) — one literal site,
//!   `entity_view`, using `..ordered_entity_view_base(spec.role.clone())` (FRU)
//!   so the fixture supplies `point_count` and the unmodelled defaults. The two
//!   endpoints are then overwritten with real geometry, because the fixture's
//!   points are all zero, which would put every start at the origin and make
//!   every candidate equidistant.
//! * `Point3WithWidth` (`crates/slicer-ir/src/slice_ir.rs`) — one literal
//!   expression in `point`, used for both endpoints, carrying
//!   `..Point3WithWidth::default()`; the modelled fields are XY plus the
//!   witness's uniform width/flow, and `overhang_quartile` / `dist_to_top_mm` /
//!   `overhang_distance_mm` keep their documented `None`/`0.0` values.
//! * `RegionKey` (`crates/slicer-ir/src/slice_ir.rs`, four fields so not a
//!   watched type) — one literal site in `entity_view`, FRU over
//!   `RegionKey::default()`, pinning the one `(layer, object, region)` scope
//!   ADR-0062 talks about. `variant_chain` stays the ordered, empty identity
//!   vector — never name-sorted or normalised.
//! * `GeometryBounds`/`GeometryDomain` are not used by this module.
//!
//! Module-private types (`Limits`, `CandidateQuanta`, `NearestCase`,
//! `NearestObservation`, `LockSpec`, `LockCase`, `LockObservation`, `RunModel`,
//! `CoalescedSnapshot`, `GroupCase`, `GroupObservation` and the stats/failure
//! enums) are not watched crate-public structs, so they are exhaustive by
//! construction: their fields are the fixture.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! negative controls are mutation controls over the **same** checker each
//! property uses: every control first re-asserts that the unmodified observation
//! is accepted, then asserts the single mutated observation is rejected with a
//! specific failure class. A panic or a scaffolding-only change would not count.
//! The properties require positive per-subcase counters taken from accepted SUT
//! observations, and empty/singleton observations are asserted separately and
//! never counted as positives. Characterisation assertions name their protected
//! surface in the message; deterministic comparisons that only pin reproducibility
//! carry a `// test-quality:` note.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{ExtrusionRole, Point3WithWidth, RegionKey};
use slicer_sdk::test_support::fixtures::ordered_entity_view_base;
use slicer_sdk::test_support::property_testing::domains::DomainError;
use slicer_sdk::test_support::property_testing::oracles::{
    is_permutation, locked_groups_are_contiguous,
};
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_sdk::views::OrderedEntityView;

use super::{
    coalesce_locked_candidates, nearest_neighbor_permutation, NearestNeighborCandidate,
    PathOptimizationDefault, ToolChangeRecord,
};

const PROPERTY_ID_NEAREST: &str = "M-PATH-NEAREST";
const PROPERTY_TEST_NEAREST: &str = "property_tests::proptest_path_nearest";
const PROPERTY_ID_LOCKS: &str = "M-PATH-LOCKS";
const PROPERTY_TEST_LOCKS: &str = "property_tests::proptest_path_locks";
const PROPERTY_ID_GROUPS: &str = "M-PATH-GROUPS";
const PROPERTY_TEST_GROUPS: &str = "property_tests::proptest_path_groups";

/// Exact `PropertyRun::bound` keys of the `M-PATH-NEAREST` record.
const NEAREST_BOUND_KEYS: [&str; 2] = ["max_candidates", "max_coord_quanta"];
/// Exact `PropertyRun::bound` keys of the `M-PATH-LOCKS` / `M-PATH-GROUPS`
/// records.
const ENTITY_BOUND_KEYS: [&str; 2] = ["max_entities", "max_coord_quanta"];
/// Every key of the deterministic probe map: both record shapes plus the shared
/// coordinate ceiling. `probe_bound_map` mirrors the registry values exactly.
const BOUND_KEYS: [&str; 3] = ["max_candidates", "max_entities", "max_coord_quanta"];

/// One quantum is `1/1024 mm`, a power of two, so every quantum-grid value
/// below 8192 quanta converts to `f32` exactly.
const QUANTUM: f32 = 1.0 / 1024.0;
/// The source-documented tie window of `nearest_neighbor_permutation`.
const TIE_WINDOW_MM: f64 = 0.001;
/// Shell spacing of generated start points: 64 quanta = 0.0625 mm, 62.5x the
/// tie window, so distinct shells can never fall inside the window.
const GRID_STEP: i64 = 64;
/// Tool indices generated by the group property.
const MAX_TOOLS: u32 = 3;
/// Smallest entity/candidate bound that admits every deterministic witness
/// (the two-candidate tie pair and the multi-entity blocks).
const MIN_COUNT: u32 = 2;
/// Packet ceiling for `max_candidates` / `max_entities`.
const MAX_COUNT_CEILING: u32 = 32;
/// 2 mm: the largest witness coordinate (and the generated cap at 32 shells).
const MIN_COORD_QUANTA: u64 = WITNESS_MAX_COORD_QUANTA as u64;
/// Packet ceiling for every numeric `max_*_quanta` bound.
const MAX_COORD_QUANTA_CEILING: u64 = 65_536;
/// Step ceiling for the deterministic shrink-transition loops.
const MAX_SHRINK_STEPS: usize = 20_000;
/// Largest coordinate any deterministic witness uses.
const WITNESS_MAX_COORD_QUANTA: i64 = 2_048;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

/// The two shape parameters every generated domain here needs: a count ceiling
/// (`max_candidates` for `M-PATH-NEAREST`, `max_entities` for the two entity
/// properties) and the shared coordinate ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Limits {
    max_count: u32,
    max_coord_quanta: i64,
}

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

/// Validate every profile bound before any conversion or allocation, and reject
/// a profile that cannot satisfy the witnesses.
fn checked_limits(bounds: &BTreeMap<String, u64>, count_key: &str) -> Result<Limits, DomainError> {
    if !BOUND_KEYS.contains(&count_key) {
        return Err(DomainError::InvalidBounds(format!(
            "`{count_key}` is not a declared path-ordering bound key"
        )));
    }
    let count =
        u32::try_from(required(bounds, count_key)?).map_err(|_| DomainError::ArithmeticOverflow)?;
    let coord = i64::try_from(required(bounds, "max_coord_quanta")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;

    if !(MIN_COUNT..=MAX_COUNT_CEILING).contains(&count) {
        return Err(DomainError::InvalidBounds(format!(
            "{count_key} must be in {MIN_COUNT}..={MAX_COUNT_CEILING}: the two-candidate tie \
             witness needs two and the packet ceiling is {MAX_COUNT_CEILING}"
        )));
    }
    if !(MIN_COORD_QUANTA as i64..=MAX_COORD_QUANTA_CEILING as i64).contains(&coord) {
        return Err(DomainError::InvalidBounds(format!(
            "max_coord_quanta must be in {MIN_COORD_QUANTA}..={MAX_COORD_QUANTA_CEILING}: the \
             witnesses need 2 mm and the packet ceiling is {MAX_COORD_QUANTA_CEILING} quanta"
        )));
    }
    if coord / GRID_STEP < 2 {
        return Err(DomainError::InvalidBounds(
            "max_coord_quanta must admit at least two distinct distance shells".to_owned(),
        ));
    }
    // Derived f32 envelope: a generated coordinate is at most `coord` quanta,
    // so `dx*dx + dy*dy` stays below `2 * 65536² = 8.6e9`; the kernel's f32
    // arithmetic over at most `sqrt(2) * 64 mm ≈ 90.5 mm` accumulates a relative
    // error below 3 * 2^-24, i.e. well under 2e-5 mm. Shells are 0.0625 mm
    // apart, so no f32 rounding can reorder them or close the tie window.
    Ok(Limits {
        max_count: count,
        max_coord_quanta: coord,
    })
}

/// Read the selected profile's declared bounds. `M-PATH-NEAREST` declares
/// `max_candidates`, the two entity records declare `max_entities`; a bound the
/// selected record does not declare is absent by design and is not read.
fn live_limits(run: &PropertyRun, keys: &[&str], count_key: &str) -> Limits {
    assert!(
        keys.contains(&count_key),
        "the record's declared key set must contain `{count_key}`"
    );
    let mut bounds = BTreeMap::new();
    for key in keys {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("path-optimization bound `{key}`: {error}"));
        bounds.insert((*key).to_owned(), value);
    }
    checked_limits(&bounds, count_key).expect("path-optimization bound consistency")
}

/// The registry profiles' own values; the deterministic controls and the shrink
/// probe have no `PropertyRun` context and use these exact numbers.
fn probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_candidates".to_owned(), 16),
        ("max_entities".to_owned(), 16),
        ("max_coord_quanta".to_owned(), 8_192),
    ])
}

/// Exact `f32` value of an integer number of `1/1024 mm` quanta.
fn quantum_mm(quanta: i64) -> f32 {
    quanta as f32 * QUANTUM
}

fn coord_in_bounds(quanta: i64, limits: &Limits) -> bool {
    quanta.abs() <= limits.max_coord_quanta
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared fixtures: one `(layer, object, region)` scope of ordered entities
// ─────────────────────────────────────────────────────────────────────────────

/// One authored entity: its staged identity plus the integer geometry every
/// expectation is derived from.
#[derive(Debug, Clone, PartialEq)]
struct LockSpec {
    /// Staged position. `group_then_nearest_neighbor` reads
    /// `entities[original_index]` for its tool-change records, so the production
    /// identity convention is `index == authored position`.
    index: u32,
    tool: u32,
    role: ExtrusionRole,
    start: (i64, i64),
    end: (i64, i64),
    lock: Option<u64>,
}

fn lock_spec(
    index: usize,
    tool: u32,
    role: ExtrusionRole,
    start: (i64, i64),
    end: (i64, i64),
    lock: Option<u64>,
) -> LockSpec {
    LockSpec {
        index: index as u32,
        tool,
        role,
        start,
        end,
        lock,
    }
}

/// A `Point3WithWidth` on the quantum grid. FRU keeps the unmodelled overhang
/// and top-distance fields at their documented defaults.
fn point(x: i64, y: i64) -> Point3WithWidth {
    Point3WithWidth {
        x: quantum_mm(x),
        y: quantum_mm(y),
        z: 0.0,
        width: 0.4,
        flow_factor: 1.0,
        ..Point3WithWidth::default()
    }
}

/// The production `OrderedEntityView` for one authored entity. FRU over the SDK
/// fixture supplies `point_count` and every field this module does not model;
/// the two endpoints are overwritten because the fixture's points are all zero.
fn entity_view(spec: &LockSpec) -> OrderedEntityView {
    let mut view = OrderedEntityView {
        original_index: spec.index,
        tool_index: spec.tool,
        order_lock: spec.lock,
        region_key: RegionKey {
            object_id: "path-object".to_owned(),
            ..RegionKey::default()
        },
        ..ordered_entity_view_base(spec.role.clone())
    };
    view.start_point = point(spec.start.0, spec.start.1);
    view.end_point = point(spec.end.0, spec.end.1);
    view
}

fn validate_specs(specs: &[LockSpec], limits: &Limits) -> Result<(), String> {
    if specs.len() > limits.max_count as usize {
        return Err(format!(
            "{} entities exceed the profile count ceiling {}",
            specs.len(),
            limits.max_count
        ));
    }
    for (slot, spec) in specs.iter().enumerate() {
        if spec.index as usize != slot {
            return Err(format!(
                "authored position {slot} carries original_index {}; the production view \
                 identity convention is position",
                spec.index
            ));
        }
        for (label, value) in [
            ("start.x", spec.start.0),
            ("start.y", spec.start.1),
            ("end.x", spec.end.0),
            ("end.y", spec.end.1),
        ] {
            if !coord_in_bounds(value, limits) {
                return Err(format!(
                    "entity {slot} {label} = {value} quanta exceeds max_coord_quanta {}",
                    limits.max_coord_quanta
                ));
            }
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// The authored role table (never `role_group`)
// ─────────────────────────────────────────────────────────────────────────────

/// One output segment inside a tool cluster, ordered exactly as the module
/// documents it: skirt first, the staged wall block next, then role groups
/// ascending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SegmentKey {
    Skirt,
    Wall,
    Group(u32),
}

/// The independently authored expectation for one `ExtrusionRole`.
///
/// `documented` is `false` exactly for the five variants that the live
/// `role_group` swallows through `_ => 9`; those are characterised, not claimed.
fn characterized_group(role: &ExtrusionRole) -> u32 {
    match role {
        ExtrusionRole::Skirt => 0,
        ExtrusionRole::InnerWall => 1,
        ExtrusionRole::OuterWall => 2,
        ExtrusionRole::ThinWall => 3,
        ExtrusionRole::BottomSolidInfill
        | ExtrusionRole::TopSolidInfill
        | ExtrusionRole::SparseInfill
        | ExtrusionRole::BridgeInfill
        | ExtrusionRole::GapFill => 4,
        ExtrusionRole::Ironing => 5,
        ExtrusionRole::SupportMaterial => 6,
        ExtrusionRole::SupportInterface => 7,
        ExtrusionRole::WipeTower | ExtrusionRole::PrimeTower => 8,
        ExtrusionRole::Custom(_) => 9,
        // Characterised wildcard arms: current `_ => 9` behaviour. The archived
        // packet-61 digest suggested `InternalBridgeInfill` belongs with infill;
        // no live normative document mandates that grouping, so this table pins
        // the characterisation rather than the suggestion.
        ExtrusionRole::InternalBridgeInfill
        | ExtrusionRole::InternalSolidInfill
        | ExtrusionRole::RaftInfill
        | ExtrusionRole::SupportBaseInterface
        | ExtrusionRole::Brim => 9,
    }
}

fn role_group_is_documented(role: &ExtrusionRole) -> bool {
    !matches!(
        role,
        ExtrusionRole::InternalBridgeInfill
            | ExtrusionRole::InternalSolidInfill
            | ExtrusionRole::RaftInfill
            | ExtrusionRole::SupportBaseInterface
            | ExtrusionRole::Brim
    )
}

fn segment_key(role: &ExtrusionRole) -> SegmentKey {
    match role {
        ExtrusionRole::Skirt => SegmentKey::Skirt,
        ExtrusionRole::InnerWall | ExtrusionRole::OuterWall => SegmentKey::Wall,
        other => SegmentKey::Group(characterized_group(other)),
    }
}

/// The five variants the current wildcard takes, authored once so the
/// characterisation cannot rot silently.
const WILDCARD_ROLES: [ExtrusionRole; 5] = [
    ExtrusionRole::InternalBridgeInfill,
    ExtrusionRole::InternalSolidInfill,
    ExtrusionRole::RaftInfill,
    ExtrusionRole::SupportBaseInterface,
    ExtrusionRole::Brim,
];

fn ordinary_roles() -> Vec<ExtrusionRole> {
    vec![
        ExtrusionRole::Skirt,
        ExtrusionRole::InnerWall,
        ExtrusionRole::OuterWall,
        ExtrusionRole::ThinWall,
        ExtrusionRole::SparseInfill,
        ExtrusionRole::TopSolidInfill,
        ExtrusionRole::BottomSolidInfill,
        ExtrusionRole::BridgeInfill,
        ExtrusionRole::GapFill,
        ExtrusionRole::Ironing,
        ExtrusionRole::SupportMaterial,
        ExtrusionRole::SupportInterface,
        ExtrusionRole::WipeTower,
        ExtrusionRole::PrimeTower,
        ExtrusionRole::Custom("variant-a".to_owned()),
    ]
}

fn wildcard_mix_roles() -> Vec<ExtrusionRole> {
    vec![
        ExtrusionRole::InternalBridgeInfill,
        ExtrusionRole::InternalSolidInfill,
        ExtrusionRole::RaftInfill,
        ExtrusionRole::SupportBaseInterface,
        ExtrusionRole::Brim,
        ExtrusionRole::SupportMaterial,
        ExtrusionRole::WipeTower,
        ExtrusionRole::SparseInfill,
        ExtrusionRole::GapFill,
        ExtrusionRole::Ironing,
        ExtrusionRole::Skirt,
        ExtrusionRole::ThinWall,
        ExtrusionRole::Custom("variant-b".to_owned()),
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared run derivation
// ─────────────────────────────────────────────────────────────────────────────

/// One candidate as the documented coalescing rule sees it: the maximal
/// authored-adjacent run of equal `Some(lock)` tags, or one untagged entity.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RunModel {
    indices: Vec<usize>,
    lock: Option<u64>,
}

fn runs_of(specs: &[LockSpec], members: &[usize]) -> Vec<RunModel> {
    let mut runs = Vec::new();
    let mut slot = 0;
    while slot < members.len() {
        let first = members[slot];
        let lock = specs[first].lock;
        let mut indices = vec![first];
        slot += 1;
        if let Some(tag) = lock {
            while slot < members.len() && specs[members[slot]].lock == Some(tag) {
                indices.push(members[slot]);
                slot += 1;
            }
        }
        runs.push(RunModel { indices, lock });
    }
    runs
}

fn authored_indices(specs: &[LockSpec]) -> Vec<usize> {
    (0..specs.len()).collect()
}

fn tag_partition_map(specs: &[LockSpec]) -> BTreeMap<u64, BTreeSet<(u32, SegmentKey)>> {
    let mut map: BTreeMap<u64, BTreeSet<(u32, SegmentKey)>> = BTreeMap::new();
    for spec in specs {
        if let Some(tag) = spec.lock {
            map.entry(tag)
                .or_default()
                .insert((spec.tool, segment_key(&spec.role)));
        }
    }
    map
}

fn tag_run_counts(specs: &[LockSpec]) -> BTreeMap<u64, usize> {
    let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
    for run in runs_of(specs, &authored_indices(specs)) {
        if let Some(tag) = run.lock {
            *counts.entry(tag).or_default() += 1;
        }
    }
    counts
}

/// Tags with at least one authored-adjacent locked run (a unit) that is not one
/// contiguous authored-order window in `order`. ADR-0062 requires this set to be
/// empty for every case, cross-partition tags included. A tag authored as two
/// separated runs is two independent units, so its two windows need not touch.
fn split_tags(specs: &[LockSpec], order: &[usize]) -> BTreeSet<u64> {
    runs_of(specs, &authored_indices(specs))
        .into_iter()
        .filter(|run| run.indices.len() >= 2 && window_start(order, &run.indices).is_none())
        .filter_map(|run| run.lock)
        .collect()
}

/// First-member tool of every unit (maximal authored-adjacent equal-`Some(lock)`
/// run, or an untagged singleton) in output order. Each unit must appear as one
/// consecutive run of `order`; a unit that does not is reported by the caller's
/// contiguity checks, here it simply contributes its first-seen position.
fn unit_first_tools(specs: &[LockSpec], order: &[usize]) -> Vec<u32> {
    let mut unit_of = vec![0_usize; specs.len()];
    let mut first_tool = Vec::new();
    for (unit, run) in runs_of(specs, &authored_indices(specs)).iter().enumerate() {
        first_tool.push(specs[run.indices[0]].tool);
        for &slot in &run.indices {
            unit_of[slot] = unit;
        }
    }
    let mut seen = BTreeSet::new();
    order
        .iter()
        .filter(|&&slot| seen.insert(unit_of[slot]))
        .map(|&slot| first_tool[unit_of[slot]])
        .collect()
}

fn window_start(order: &[usize], group: &[usize]) -> Option<usize> {
    if group.is_empty() {
        return None;
    }
    order
        .windows(group.len())
        .position(|window| window == group)
}

fn same_set(left: &[usize], right: &[usize]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let a: BTreeSet<usize> = left.iter().copied().collect();
    let b: BTreeSet<usize> = right.iter().copied().collect();
    a == b && a.len() == left.len()
}

// ─────────────────────────────────────────────────────────────────────────────
// M-PATH-NEAREST
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NearestMode {
    /// No candidate: the kernel must return an empty permutation.
    Empty,
    /// One candidate: the kernel must return `[(0, false)]`.
    Singleton,
    /// An exact two-candidate tie plus strictly farther extras.
    Tie,
    /// Distinct, well-separated distances with the nearest candidate not first.
    Separated,
    /// Mixed shells and axes; exact ties still resolve to the lower index.
    General,
}

const NEAREST_MODES: [NearestMode; 5] = [
    NearestMode::Empty,
    NearestMode::Singleton,
    NearestMode::Tie,
    NearestMode::Separated,
    NearestMode::General,
];

impl NearestMode {
    fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Singleton => "singleton",
            Self::Tie => "tie",
            Self::Separated => "separated",
            Self::General => "general",
        }
    }
}

/// One candidate in integer `1/1024 mm` quanta. The kernel candidate is built on
/// demand so the case itself stays cheap to clone, shrink and validate.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CandidateQuanta {
    /// The candidate's original-index payload (`indices` in the kernel struct).
    payload: u32,
    start: (i64, i64),
    end: (i64, i64),
}

impl CandidateQuanta {
    fn kernel(&self) -> NearestNeighborCandidate {
        NearestNeighborCandidate {
            indices: vec![self.payload],
            start_x: quantum_mm(self.start.0),
            start_y: quantum_mm(self.start.1),
            end_x: quantum_mm(self.end.0),
            end_y: quantum_mm(self.end.1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NearestCase {
    mode: NearestMode,
    candidates: Vec<CandidateQuanta>,
}

fn max_shells(limits: &Limits) -> i64 {
    (limits.max_coord_quanta / GRID_STEP).max(2)
}

fn shell_coord(raw: u32, limits: &Limits) -> i64 {
    let shells = max_shells(limits);
    GRID_STEP * (1 + i64::from(raw % shells as u32))
}

fn offset_coord(base: i64, raw: u32, limits: &Limits) -> i64 {
    (base + GRID_STEP * i64::from(raw % 4)).min(limits.max_coord_quanta)
}

fn build_nearest_case(mode: NearestMode, raw: &[u32], limits: &Limits) -> NearestCase {
    let max = limits.max_count as usize;
    let candidates = match mode {
        NearestMode::Empty => Vec::new(),
        NearestMode::Singleton => {
            let value = raw.first().copied().unwrap_or(0);
            let q = shell_coord(value, limits);
            vec![CandidateQuanta {
                payload: 0,
                start: (q, 0),
                end: (offset_coord(q, value >> 8, limits), GRID_STEP),
            }]
        }
        NearestMode::Tie => {
            // Candidates 0 and 1 sit on the two axes at the same shell, so their
            // squared distances are bit-identical; every extra candidate is at
            // least one shell farther.
            let shells = max_shells(limits);
            let base =
                1 + i64::from(raw.first().copied().unwrap_or(0) % (shells as u32 / 2).max(1));
            let base_q = GRID_STEP * base;
            let extras = raw.len().min(max.saturating_sub(2));
            let mut out = vec![
                CandidateQuanta {
                    payload: 0,
                    start: (base_q, 0),
                    end: (base_q, GRID_STEP),
                },
                CandidateQuanta {
                    payload: 1,
                    start: (0, base_q),
                    end: (GRID_STEP, base_q),
                },
            ];
            let room = (shells - base).max(1) as u32;
            for slot in 0..extras {
                let value = raw.get(slot + 1).copied().unwrap_or(0);
                let q = base_q + GRID_STEP * (1 + i64::from(value % room));
                out.push(CandidateQuanta {
                    payload: (slot + 2) as u32,
                    start: (q, 0),
                    end: (offset_coord(q, value >> 8, limits), GRID_STEP),
                });
            }
            out
        }
        NearestMode::Separated => {
            let count = raw.len().min(max);
            if count == 0 {
                return NearestCase {
                    mode,
                    candidates: Vec::new(),
                };
            }
            let shift = (raw[0] as usize) % count;
            (0..count)
                .map(|slot| {
                    let shell = 1 + ((slot + shift) % count) as i64;
                    let q = GRID_STEP * shell;
                    CandidateQuanta {
                        payload: slot as u32,
                        start: (q, 0),
                        end: (offset_coord(q, raw[slot] >> 8, limits), GRID_STEP),
                    }
                })
                .collect()
        }
        NearestMode::General => {
            let count = raw.len().min(max);
            (0..count)
                .map(|slot| {
                    let value = raw[slot];
                    let q = shell_coord(value, limits);
                    let start = if slot % 2 == 0 { (q, 0) } else { (0, q) };
                    CandidateQuanta {
                        payload: slot as u32,
                        start,
                        end: (
                            offset_coord(start.0, value >> 8, limits),
                            offset_coord(start.1, value >> 16, limits),
                        ),
                    }
                })
                .collect()
        }
    };
    NearestCase { mode, candidates }
}

fn nearest_strategy(limits: &Limits, modes: Vec<NearestMode>) -> BoxedStrategy<NearestCase> {
    let limits = *limits;
    (
        prop::sample::select(modes).no_shrink(),
        prop::collection::vec(any::<u32>(), 0..=limits.max_count as usize),
    )
        .prop_map(move |(mode, raw)| build_nearest_case(mode, &raw, &limits))
        .boxed()
}

#[derive(Debug, Clone, PartialEq)]
struct NearestObservation {
    permutation: Vec<(usize, bool)>,
}

fn observe_nearest(case: &NearestCase) -> NearestObservation {
    let kernels: Vec<NearestNeighborCandidate> = case
        .candidates
        .iter()
        .map(CandidateQuanta::kernel)
        .collect();
    NearestObservation {
        permutation: nearest_neighbor_permutation(&kernels),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NearestFailure {
    /// The fixture is outside the declared domain (a generator bug).
    Fixture(String),
    /// The tie-mode witness was lost.
    Oracle(String),
    /// The permutation is not a bijection over the candidate set.
    Permutation(String),
    /// The first choice violates the documented nearest/tie rule.
    TieWinner(String),
    /// A reversal flag was set.
    Reversal(String),
}

impl fmt::Display for NearestFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(message) => write!(f, "fixture outside the domain: {message}"),
            Self::Oracle(message) => write!(f, "oracle: {message}"),
            Self::Permutation(message) => write!(f, "permutation: {message}"),
            Self::TieWinner(message) => write!(f, "tie winner: {message}"),
            Self::Reversal(message) => write!(f, "reversal: {message}"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct NearestStats {
    candidates: u64,
    permutations: u64,
    tie_minima: u64,
    first: usize,
}

/// Exact distance of a start point from the origin, in millimetres, from integer
/// squared quanta. The independent arithmetic never consults the kernel.
fn distance_mm(start: (i64, i64)) -> f64 {
    let squared =
        i128::from(start.0) * i128::from(start.0) + i128::from(start.1) * i128::from(start.1);
    (squared as f64).sqrt() * f64::from(QUANTUM)
}

/// The **one** nearest-neighbour checker: the property, the deterministic
/// witnesses, the mutation controls and the shrink-state revalidation all call
/// it.
fn check_nearest(
    case: &NearestCase,
    limits: &Limits,
    observation: &NearestObservation,
) -> Result<NearestStats, NearestFailure> {
    let n = case.candidates.len();
    if n > limits.max_count as usize {
        return Err(NearestFailure::Fixture(format!(
            "{n} candidates exceed the profile count ceiling {}",
            limits.max_count
        )));
    }
    for (slot, candidate) in case.candidates.iter().enumerate() {
        if candidate.payload as usize != slot {
            return Err(NearestFailure::Fixture(format!(
                "candidate {slot} carries payload {}",
                candidate.payload
            )));
        }
        for (label, value) in [
            ("start.x", candidate.start.0),
            ("start.y", candidate.start.1),
            ("end.x", candidate.end.0),
            ("end.y", candidate.end.1),
        ] {
            if !coord_in_bounds(value, limits) {
                return Err(NearestFailure::Fixture(format!(
                    "candidate {slot} {label} = {value} quanta exceeds max_coord_quanta {}",
                    limits.max_coord_quanta
                )));
            }
        }
    }

    if observation.permutation.len() != n {
        return Err(NearestFailure::Permutation(format!(
            "the kernel returned {} entries for {n} candidates",
            observation.permutation.len()
        )));
    }
    let order: Vec<usize> = observation
        .permutation
        .iter()
        .map(|&(index, _)| index)
        .collect();
    if !is_permutation(&order, n) {
        return Err(NearestFailure::Permutation(format!(
            "candidate indices {order:?} are not a bijection over 0..{n}"
        )));
    }
    // Original-index bijection with a local seen-set: every payload appears once.
    let mut payloads = BTreeSet::new();
    for &slot in &order {
        if !payloads.insert(case.candidates[slot].payload) {
            return Err(NearestFailure::Permutation(format!(
                "original index {} was selected more than once",
                case.candidates[slot].payload
            )));
        }
    }
    if payloads.len() != n {
        return Err(NearestFailure::Permutation(format!(
            "{} distinct original indices were selected for {n} candidates",
            payloads.len()
        )));
    }
    if observation
        .permutation
        .iter()
        .any(|&(_, reversal)| reversal)
    {
        return Err(NearestFailure::Reversal(
            "the kernel set a reversal flag; it is documented to always be false".to_owned(),
        ));
    }

    if n == 0 {
        return Ok(NearestStats::default());
    }

    let distances: Vec<f64> = case
        .candidates
        .iter()
        .map(|candidate| distance_mm(candidate.start))
        .collect();
    let minimum = distances.iter().copied().fold(f64::INFINITY, f64::min);
    let expected_first = distances
        .iter()
        .enumerate()
        .filter(|(_, distance)| **distance - minimum < TIE_WINDOW_MM)
        .map(|(slot, _)| slot)
        .min()
        .expect("the minimum-distance candidate is in the band");
    if observation.permutation[0].0 != expected_first {
        return Err(NearestFailure::TieWinner(format!(
            "the kernel selected candidate {} first; the independent distance calculation and \
             the {TIE_WINDOW_MM} mm earlier-index window select {expected_first} \
             (distances {distances:?})",
            observation.permutation[0].0
        )));
    }
    let minima = distances
        .iter()
        .filter(|distance| **distance == minimum)
        .count();
    if case.mode == NearestMode::Tie && minima < 2 {
        return Err(NearestFailure::Oracle(
            "the tie mode lost its exact two-candidate tie".to_owned(),
        ));
    }
    Ok(NearestStats {
        candidates: n as u64,
        permutations: 1,
        tie_minima: u64::from(minima >= 2),
        first: expected_first,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// M-PATH-LOCKS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockMode {
    Empty,
    Singleton,
    /// Only untagged entities: every item is a singleton candidate.
    Untagged,
    /// One locked block plus untagged singletons.
    MixedBlocks,
    /// One locked block spanning several non-wall roles of the same group.
    MultiRoleBlock,
    /// One tag spanning two tools: one ADR-0062 unit, placed whole.
    CrossToolBlock,
    /// The same tag on two authored runs separated by an untagged entity.
    SeparatedTag,
}

const LOCK_MODES: [LockMode; 7] = [
    LockMode::Empty,
    LockMode::Singleton,
    LockMode::Untagged,
    LockMode::MixedBlocks,
    LockMode::MultiRoleBlock,
    LockMode::CrossToolBlock,
    LockMode::SeparatedTag,
];

impl LockMode {
    fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Singleton => "singleton",
            Self::Untagged => "untagged",
            Self::MixedBlocks => "mixed-blocks",
            Self::MultiRoleBlock => "multi-role-block",
            Self::CrossToolBlock => "cross-tool-block",
            Self::SeparatedTag => "separated-tag",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LockCase {
    mode: LockMode,
    specs: Vec<LockSpec>,
}

fn raw_coord(value: u32) -> i64 {
    // 1..=32 shells of 64 quanta = at most 2048 quanta (2 mm).
    GRID_STEP * i64::from(1 + (value & 0x1F))
}

fn raw_point(raw: &[u32; 4]) -> ((i64, i64), (i64, i64)) {
    (
        (raw_coord(raw[1]), raw_coord(raw[1] >> 8)),
        (raw_coord(raw[2]), raw_coord(raw[2] >> 8)),
    )
}

fn append_extras(specs: &mut Vec<LockSpec>, raw: &[[u32; 4]], extra_count: usize) {
    for extra in 0..extra_count {
        let slot = specs.len();
        let value = raw.get(extra).copied().unwrap_or([0; 4]);
        let role = ExtrusionRole::SparseInfill;
        let (start, end) = raw_point(&value);
        specs.push(lock_spec(
            slot,
            value[0] % MAX_TOOLS,
            role,
            start,
            end,
            None,
        ));
    }
}

fn build_lock_case(mode: LockMode, raw: &[[u32; 4]], limits: &Limits) -> LockCase {
    let max = limits.max_count as usize;
    let specs = match mode {
        LockMode::Empty => Vec::new(),
        LockMode::Singleton => vec![lock_spec(
            0,
            0,
            ExtrusionRole::SparseInfill,
            (256, 0),
            (512, 0),
            None,
        )],
        LockMode::Untagged => {
            let count = raw.len().min(max);
            (0..count)
                .map(|slot| {
                    let value = raw[slot];
                    let (start, end) = raw_point(&value);
                    lock_spec(
                        slot,
                        value[0] % MAX_TOOLS,
                        ExtrusionRole::SparseInfill,
                        start,
                        end,
                        None,
                    )
                })
                .collect()
        }
        LockMode::MixedBlocks => {
            // A fixed three-entity block survives shrinking by construction; the
            // raw extras (all untagged) are what simplifies.
            let mut specs = vec![
                lock_spec(
                    0,
                    0,
                    ExtrusionRole::SparseInfill,
                    (256, 0),
                    (512, 0),
                    Some(7),
                ),
                lock_spec(
                    1,
                    0,
                    ExtrusionRole::SparseInfill,
                    (768, 0),
                    (1024, 0),
                    Some(7),
                ),
                lock_spec(
                    2,
                    0,
                    ExtrusionRole::SparseInfill,
                    (1280, 0),
                    (1536, 0),
                    Some(7),
                ),
            ];
            let room = max.saturating_sub(specs.len());
            append_extras(&mut specs, raw, raw.len().min(room));
            specs
        }
        LockMode::MultiRoleBlock => {
            let mut specs = vec![
                lock_spec(
                    0,
                    0,
                    ExtrusionRole::SparseInfill,
                    (512, 0),
                    (768, 0),
                    Some(9),
                ),
                lock_spec(
                    1,
                    0,
                    ExtrusionRole::TopSolidInfill,
                    (1024, 0),
                    (1280, 0),
                    Some(9),
                ),
                lock_spec(
                    2,
                    0,
                    ExtrusionRole::BottomSolidInfill,
                    (1536, 0),
                    (1792, 0),
                    Some(9),
                ),
            ];
            let room = max.saturating_sub(specs.len());
            append_extras(&mut specs, raw, raw.len().min(room));
            specs
        }
        LockMode::CrossToolBlock => {
            // The direct kernel coalesces across tools; the caller partitions.
            let mut specs = vec![
                lock_spec(
                    0,
                    0,
                    ExtrusionRole::SparseInfill,
                    (256, 0),
                    (512, 0),
                    Some(9),
                ),
                lock_spec(
                    1,
                    1,
                    ExtrusionRole::SparseInfill,
                    (768, 0),
                    (1024, 0),
                    Some(9),
                ),
                lock_spec(2, 0, ExtrusionRole::Ironing, (1280, 0), (1536, 0), None),
                lock_spec(
                    3,
                    1,
                    ExtrusionRole::TopSolidInfill,
                    (1792, 0),
                    (2048, 0),
                    None,
                ),
            ];
            let room = max.saturating_sub(specs.len());
            append_extras(&mut specs, raw, raw.len().min(room));
            specs
        }
        LockMode::SeparatedTag => {
            let mut specs = vec![
                lock_spec(
                    0,
                    0,
                    ExtrusionRole::SparseInfill,
                    (256, 0),
                    (512, 0),
                    Some(7),
                ),
                lock_spec(1, 0, ExtrusionRole::SparseInfill, (768, 0), (1024, 0), None),
                lock_spec(
                    2,
                    0,
                    ExtrusionRole::SparseInfill,
                    (1280, 0),
                    (1536, 0),
                    Some(7),
                ),
            ];
            let room = max.saturating_sub(specs.len());
            append_extras(&mut specs, raw, raw.len().min(room));
            specs
        }
    };
    LockCase { mode, specs }
}

fn lock_strategy(limits: &Limits, modes: Vec<LockMode>) -> BoxedStrategy<LockCase> {
    let limits = *limits;
    (
        prop::sample::select(modes).no_shrink(),
        prop::collection::vec(any::<[u32; 4]>(), 0..=limits.max_count as usize),
    )
        .prop_map(move |(mode, raw)| build_lock_case(mode, &raw, &limits))
        .boxed()
}

#[derive(Debug, Clone, PartialEq)]
struct CoalescedSnapshot {
    indices: Vec<u32>,
    start: (f32, f32),
    end: (f32, f32),
}

#[derive(Debug, Clone, PartialEq)]
struct LockObservation {
    candidates: Vec<CoalescedSnapshot>,
    permutation: Vec<(u32, bool)>,
    tool_changes: Vec<(u32, u32, u32)>,
}

fn snapshot_of(specs: &[LockSpec], run: &RunModel) -> CoalescedSnapshot {
    let first = run.indices.first().copied().expect("a run is nonempty");
    let last = run.indices.last().copied().expect("a run is nonempty");
    CoalescedSnapshot {
        indices: run.indices.iter().map(|&slot| specs[slot].index).collect(),
        start: (
            quantum_mm(specs[first].start.0),
            quantum_mm(specs[first].start.1),
        ),
        end: (quantum_mm(specs[last].end.0), quantum_mm(specs[last].end.1)),
    }
}

fn tool_changes_of(records: &[ToolChangeRecord]) -> Vec<(u32, u32, u32)> {
    records
        .iter()
        .map(|record| (record.after_entity_index, record.from_tool, record.to_tool))
        .collect()
}

fn observe_locks(case: &LockCase) -> LockObservation {
    let views: Vec<OrderedEntityView> = case.specs.iter().map(entity_view).collect();
    let refs: Vec<&OrderedEntityView> = views.iter().collect();
    let candidates = coalesce_locked_candidates(&refs)
        .into_iter()
        .map(|candidate| CoalescedSnapshot {
            indices: candidate.indices,
            start: (candidate.start_x, candidate.start_y),
            end: (candidate.end_x, candidate.end_y),
        })
        .collect();
    let (permutation, tool_changes) =
        PathOptimizationDefault::default().group_then_nearest_neighbor(&views);
    LockObservation {
        candidates,
        permutation,
        tool_changes: tool_changes_of(&tool_changes),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LockFailure {
    Fixture(String),
    /// The coalesced candidate list differs from the authored-run derivation.
    Coalescing(String),
    /// The permutation is not a bijection.
    Permutation(String),
    /// A reversal flag was set.
    Reversal(String),
    /// A locked block was split, interleaved, reordered or reversed (ADR-0062).
    Block(String),
    /// A generator witness lost the property it exists to exercise.
    Witness(String),
}

impl fmt::Display for LockFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(message) => write!(f, "fixture outside the domain: {message}"),
            Self::Coalescing(message) => write!(f, "coalescing: {message}"),
            Self::Permutation(message) => write!(f, "permutation: {message}"),
            Self::Reversal(message) => write!(f, "reversal: {message}"),
            Self::Block(message) => write!(f, "block: {message}"),
            Self::Witness(message) => write!(f, "witness: {message}"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct LockStats {
    candidates: u64,
    blocks: u64,
    moved_blocks: u64,
    permutations: u64,
    separated_tags: u64,
    cross_partition_blocks: u64,
    empty: u64,
    singleton: u64,
}

/// The **one** lock checker: the property, the deterministic witnesses, the
/// mutation controls and the shrink-state revalidation all call it.
fn check_locks(
    case: &LockCase,
    limits: &Limits,
    observation: &LockObservation,
) -> Result<LockStats, LockFailure> {
    validate_specs(&case.specs, limits).map_err(LockFailure::Fixture)?;
    let n = case.specs.len();
    let order: Vec<usize> = observation
        .permutation
        .iter()
        .map(|&(index, _)| index as usize)
        .collect();

    // 1. Direct coalescing against the authored adjacent-run derivation.
    let expected: Vec<CoalescedSnapshot> = runs_of(&case.specs, &authored_indices(&case.specs))
        .iter()
        .map(|run| snapshot_of(&case.specs, run))
        .collect();
    if expected.len() != observation.candidates.len() {
        return Err(LockFailure::Coalescing(format!(
            "the kernel returned {} candidate(s) for {} authored run(s)",
            observation.candidates.len(),
            expected.len()
        )));
    }
    for (slot, (want, got)) in expected.iter().zip(&observation.candidates).enumerate() {
        if want.indices != got.indices {
            return Err(LockFailure::Coalescing(format!(
                "candidate {slot} indices {:?} != authored run {:?}",
                got.indices, want.indices
            )));
        }
        for (label, want, got) in [
            ("start.x", want.start.0, got.start.0),
            ("start.y", want.start.1, got.start.1),
            ("end.x", want.end.0, got.end.0),
            ("end.y", want.end.1, got.end.1),
        ] {
            if want.to_bits() != got.to_bits() {
                return Err(LockFailure::Coalescing(format!(
                    "candidate {slot} {label} is {got} but the authored run expects {want} \
                     (the last member's end, not the first's)"
                )));
            }
        }
    }

    let blocks: Vec<RunModel> = expected
        .iter()
        .map(|snapshot| RunModel {
            indices: snapshot
                .indices
                .iter()
                .map(|&index| index as usize)
                .collect(),
            lock: case.specs[snapshot.indices[0] as usize].lock,
        })
        .filter(|run| run.lock.is_some() && run.indices.len() >= 2)
        .collect();

    if n == 0 {
        if !order.is_empty() || !observation.tool_changes.is_empty() {
            return Err(LockFailure::Permutation(
                "an empty case produced output".to_owned(),
            ));
        }
        return Ok(LockStats {
            empty: 1,
            ..LockStats::default()
        });
    }

    // 2. Bijection and direction of the module proposal.
    if !is_permutation(&order, n) {
        return Err(LockFailure::Permutation(format!(
            "entity indices {order:?} are not a bijection over 0..{n}"
        )));
    }
    if observation
        .permutation
        .iter()
        .any(|&(_, reversal)| reversal)
    {
        return Err(LockFailure::Reversal(
            "the module set a reversal flag; the kernel is documented to always return false"
                .to_owned(),
        ));
    }
    // 3. ADR-0062: every locked run -- including one spanning tools, role groups
    //    or the wall/non-wall boundary -- is one contiguous window in authored
    //    order, never reversed. The same independent oracle sweeps every block.
    for run in &blocks {
        if window_start(&order, &run.indices).is_none() {
            return Err(LockFailure::Block(format!(
                "locked block {:?} (tag {:?}) is not one contiguous authored-order window in {order:?}",
                run.indices, run.lock
            )));
        }
    }
    let all_blocks: Vec<Vec<usize>> = blocks.iter().map(|run| run.indices.clone()).collect();
    if !locked_groups_are_contiguous(&order, &all_blocks) {
        return Err(LockFailure::Block(format!(
            "the independent contiguity oracle rejects {all_blocks:?} in {order:?}"
        )));
    }
    // Tool clusters ascend by the tool of each unit's FIRST member: a block is
    // assigned whole to the first member's partition.
    let unit_tools = unit_first_tools(&case.specs, &order);
    if unit_tools.windows(2).any(|pair| pair[0] > pair[1]) {
        return Err(LockFailure::Block(format!(
            "unit tool clusters (first-member tool) are not ascending: {unit_tools:?}"
        )));
    }

    // 4. The module's stated role ordering still holds for the tool blocks.
    let expected_changes = expected_tool_changes(&case.specs, &order);
    if observation.tool_changes != expected_changes {
        return Err(LockFailure::Block(format!(
            "tool changes {:?} != expected {:?}",
            observation.tool_changes, expected_changes
        )));
    }

    let partition_tags = tag_partition_map(&case.specs);
    let split = split_tags(&case.specs, &order);
    if !split.is_empty() {
        return Err(LockFailure::Block(format!(
            "ADR-0062 tag(s) {split:?} are not one contiguous window in {order:?}"
        )));
    }
    let separated = tag_run_counts(&case.specs)
        .values()
        .filter(|count| **count > 1)
        .count() as u64;
    let cross_partition = partition_tags
        .values()
        .filter(|partitions| partitions.len() > 1)
        .count() as u64;
    let moved = blocks
        .iter()
        .filter(|run| window_start(&order, &run.indices) != Some(run.indices[0]))
        .count() as u64;

    // Witness post-conditions: the cross-tool witness must really span two
    // partitions (and, above, stay one contiguous forward block), and a
    // separated tag must remain two authored runs.
    match case.mode {
        LockMode::CrossToolBlock => {
            if cross_partition == 0 {
                return Err(LockFailure::Witness(
                    "the cross-tool witness lost its cross-partition tag".to_owned(),
                ));
            }
        }
        LockMode::SeparatedTag => {
            if separated == 0 {
                return Err(LockFailure::Witness(
                    "the separated-tag witness lost its two authored runs".to_owned(),
                ));
            }
        }
        LockMode::MixedBlocks | LockMode::MultiRoleBlock => {
            if blocks.is_empty() {
                return Err(LockFailure::Witness(
                    "the locked-block witness vanished".to_owned(),
                ));
            }
        }
        LockMode::Empty | LockMode::Singleton | LockMode::Untagged => {}
    }

    Ok(LockStats {
        candidates: expected.len() as u64,
        blocks: blocks.len() as u64,
        moved_blocks: moved,
        permutations: 1,
        separated_tags: separated,
        cross_partition_blocks: cross_partition,
        empty: 0,
        singleton: u64::from(n == 1),
    })
}

fn expected_tool_changes(specs: &[LockSpec], order: &[usize]) -> Vec<(u32, u32, u32)> {
    let mut changes = Vec::new();
    for (position, pair) in order.windows(2).enumerate() {
        let from = specs[pair[0]].tool;
        let to = specs[pair[1]].tool;
        if from != to {
            changes.push((position as u32, from, to));
        }
    }
    changes
}

// ─────────────────────────────────────────────────────────────────────────────
// M-PATH-GROUPS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupMode {
    Empty,
    /// Documented roles, ordinary placement, partition-confined locks only.
    Ordinary,
    /// One same-region block spanning the wall group and two infill groups.
    CrossRoleLock,
    /// One same-region block spanning two tool clusters.
    CrossToolLock,
    /// Wildcard roles next to the roles they resemble.
    WildcardMix,
}

const GROUP_MODES: [GroupMode; 5] = [
    GroupMode::Empty,
    GroupMode::Ordinary,
    GroupMode::CrossRoleLock,
    GroupMode::CrossToolLock,
    GroupMode::WildcardMix,
];

impl GroupMode {
    fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Ordinary => "ordinary",
            Self::CrossRoleLock => "cross-role-lock",
            Self::CrossToolLock => "cross-tool-lock",
            Self::WildcardMix => "wildcard-mix",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct GroupCase {
    mode: GroupMode,
    specs: Vec<LockSpec>,
}

/// Tag authored-adjacent runs that stay inside one partition, so the ordinary
/// domain never accidentally claims ADR-0062 protection for a cross-group block.
fn tag_partition_runs(specs: &mut [LockSpec], raw: &[[u32; 4]]) {
    let mut slot = 0;
    let mut tag = 1_u64;
    while slot < specs.len() {
        let key = segment_key(&specs[slot].role);
        let start_bit = raw.get(slot).map_or(0, |value| value[3]) % 3 == 0;
        if key != SegmentKey::Wall && start_bit {
            let mut end = slot;
            while end + 1 < specs.len()
                && specs[end + 1].tool == specs[slot].tool
                && segment_key(&specs[end + 1].role) == key
                && raw
                    .get(end + 1)
                    .map_or(0, |value| value[3])
                    .is_multiple_of(2)
            {
                end += 1;
            }
            if end > slot {
                for item in &mut specs[slot..=end] {
                    item.lock = Some(tag);
                }
                tag += 1;
                slot = end + 1;
                continue;
            }
        }
        slot += 1;
    }
}

fn build_group_entity(slot: usize, role: ExtrusionRole, tool: u32, raw: &[u32; 4]) -> LockSpec {
    let (start, end) = raw_point(raw);
    lock_spec(slot, tool, role, start, end, None)
}

fn build_group_case(mode: GroupMode, raw: &[[u32; 4]], limits: &Limits) -> GroupCase {
    let max = limits.max_count as usize;
    let mut specs = match mode {
        GroupMode::Empty => Vec::new(),
        GroupMode::Ordinary => {
            let roles = ordinary_roles();
            let count = raw.len().min(max);
            (0..count)
                .map(|slot| {
                    let value = raw[slot];
                    let role = roles[((value[0] / MAX_TOOLS) as usize) % roles.len()].clone();
                    build_group_entity(slot, role, value[0] % MAX_TOOLS, &value)
                })
                .collect()
        }
        GroupMode::CrossRoleLock => {
            // A fixed wall + group-4 + group-5 block (one tag) followed by one
            // untagged entity in each of those partitions: the untagged wall
            // always follows the tagged wall inside the wall segment, so the
            // tagged block can never form one contiguous window.
            let mut specs = vec![
                lock_spec(0, 0, ExtrusionRole::InnerWall, (128, 0), (256, 0), Some(9)),
                lock_spec(
                    1,
                    0,
                    ExtrusionRole::SparseInfill,
                    (256, 0),
                    (512, 0),
                    Some(9),
                ),
                lock_spec(
                    2,
                    0,
                    ExtrusionRole::SparseInfill,
                    (512, 0),
                    (768, 0),
                    Some(9),
                ),
                lock_spec(3, 0, ExtrusionRole::Ironing, (768, 0), (1024, 0), Some(9)),
                lock_spec(4, 0, ExtrusionRole::InnerWall, (1024, 0), (1280, 0), None),
                lock_spec(
                    5,
                    0,
                    ExtrusionRole::SparseInfill,
                    (1280, 0),
                    (1536, 0),
                    None,
                ),
                lock_spec(6, 0, ExtrusionRole::Ironing, (1536, 0), (1792, 0), None),
            ];
            let room = max.saturating_sub(specs.len());
            let roles = ordinary_roles();
            for extra in 0..raw.len().min(room) {
                let value = raw[extra];
                let slot = specs.len();
                let role = roles[((value[0] / MAX_TOOLS) as usize) % roles.len()].clone();
                specs.push(build_group_entity(slot, role, 0, &value));
            }
            specs
        }
        GroupMode::CrossToolLock => {
            let mut specs = vec![
                lock_spec(
                    0,
                    0,
                    ExtrusionRole::SparseInfill,
                    (256, 0),
                    (512, 0),
                    Some(9),
                ),
                lock_spec(
                    1,
                    1,
                    ExtrusionRole::SparseInfill,
                    (768, 0),
                    (1024, 0),
                    Some(9),
                ),
                lock_spec(2, 0, ExtrusionRole::Ironing, (1280, 0), (1536, 0), None),
                lock_spec(
                    3,
                    1,
                    ExtrusionRole::TopSolidInfill,
                    (1792, 0),
                    (2048, 0),
                    None,
                ),
            ];
            let room = max.saturating_sub(specs.len());
            for extra in 0..raw.len().min(room) {
                let value = raw[extra];
                let slot = specs.len();
                specs.push(build_group_entity(
                    slot,
                    ExtrusionRole::SparseInfill,
                    value[0] % 2,
                    &value,
                ));
            }
            specs
        }
        GroupMode::WildcardMix => {
            // A fixed wildcard/support pair keeps the witness alive under shrink.
            let roles = wildcard_mix_roles();
            let mut specs = vec![
                lock_spec(
                    0,
                    0,
                    ExtrusionRole::InternalBridgeInfill,
                    (512, 0),
                    (768, 0),
                    None,
                ),
                lock_spec(
                    1,
                    0,
                    ExtrusionRole::SupportMaterial,
                    (1024, 0),
                    (1280, 0),
                    None,
                ),
            ];
            let room = max.saturating_sub(specs.len());
            for extra in 0..raw.len().min(room) {
                let value = raw[extra];
                let slot = specs.len();
                let role = roles[((value[0] / MAX_TOOLS) as usize) % roles.len()].clone();
                specs.push(build_group_entity(slot, role, value[0] % MAX_TOOLS, &value));
            }
            specs
        }
    };
    if mode == GroupMode::Ordinary {
        tag_partition_runs(&mut specs, raw);
    }
    GroupCase { mode, specs }
}

fn group_strategy(limits: &Limits, modes: Vec<GroupMode>) -> BoxedStrategy<GroupCase> {
    let limits = *limits;
    (
        prop::sample::select(modes).no_shrink(),
        prop::collection::vec(any::<[u32; 4]>(), 0..=limits.max_count as usize),
    )
        .prop_map(move |(mode, raw)| build_group_case(mode, &raw, &limits))
        .boxed()
}

#[derive(Debug, Clone, PartialEq)]
struct GroupObservation {
    permutation: Vec<(u32, bool)>,
    tool_changes: Vec<(u32, u32, u32)>,
}

fn observe_groups(case: &GroupCase) -> GroupObservation {
    let views: Vec<OrderedEntityView> = case.specs.iter().map(entity_view).collect();
    let (permutation, tool_changes) =
        PathOptimizationDefault::default().group_then_nearest_neighbor(&views);
    GroupObservation {
        permutation,
        tool_changes: tool_changes_of(&tool_changes),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GroupFailure {
    Fixture(String),
    Permutation(String),
    SkirtOrder(String),
    WallOrder(String),
    GroupOrder(String),
    Membership(String),
    Block(String),
    Reversal(String),
    ToolChanges(String),
    Witness(String),
}

impl fmt::Display for GroupFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(message) => write!(f, "fixture outside the domain: {message}"),
            Self::Permutation(message) => write!(f, "permutation: {message}"),
            Self::SkirtOrder(message) => write!(f, "skirt order: {message}"),
            Self::WallOrder(message) => write!(f, "wall order: {message}"),
            Self::GroupOrder(message) => write!(f, "group order: {message}"),
            Self::Membership(message) => write!(f, "membership: {message}"),
            Self::Block(message) => write!(f, "block: {message}"),
            Self::Reversal(message) => write!(f, "reversal: {message}"),
            Self::ToolChanges(message) => write!(f, "tool changes: {message}"),
            Self::Witness(message) => write!(f, "witness: {message}"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct GroupStats {
    permutations: u64,
    tool_blocks: u64,
    skirt_segments: u64,
    wall_segments: u64,
    group_segments: u64,
    locked_runs: u64,
    cross_partition_tags: u64,
    split_tags: u64,
    tool_changes: u64,
    empty: u64,
}

/// Walk the output as units. Every unit (maximal authored-adjacent equal-
/// `Some(lock)` run, or untagged singleton) must appear as exactly its authored
/// indices, consecutive and forward (ADR-0062), and the units are returned in
/// output order as authored-unit numbers.
fn observed_units(specs: &[LockSpec], order: &[usize]) -> Result<Vec<usize>, GroupFailure> {
    let units = runs_of(specs, &authored_indices(specs));
    let mut unit_of = vec![0_usize; specs.len()];
    for (unit, run) in units.iter().enumerate() {
        for &slot in &run.indices {
            unit_of[slot] = unit;
        }
    }
    let mut out = Vec::new();
    let mut position = 0;
    while position < order.len() {
        let unit = unit_of[order[position]];
        let members = &units[unit].indices;
        if order.get(position..position + members.len()) != Some(members.as_slice()) {
            return Err(GroupFailure::Block(format!(
                "unit {members:?} (tag {:?}) is not one contiguous authored-order window at                  output position {position} of {order:?}",
                units[unit].lock
            )));
        }
        out.push(unit);
        position += members.len();
    }
    Ok(out)
}

/// Partition-by-first-member check over the observed unit sequence: tool
/// clusters ascend, within a tool the segments run skirt, staged walls, then
/// documented groups ascending, and every partition holds exactly its units.
fn check_group_units(
    specs: &[LockSpec],
    order: &[usize],
    stats: &mut GroupStats,
) -> Result<(), GroupFailure> {
    let units = runs_of(specs, &authored_indices(specs));
    let observed = observed_units(specs, order)?;

    let mut by_partition: BTreeMap<(u32, SegmentKey), Vec<usize>> = BTreeMap::new();
    for (unit, run) in units.iter().enumerate() {
        let first = run.indices[0];
        by_partition
            .entry((specs[first].tool, segment_key(&specs[first].role)))
            .or_default()
            .push(unit);
    }
    stats.tool_blocks = by_partition
        .keys()
        .map(|(tool, _)| *tool)
        .collect::<BTreeSet<_>>()
        .len() as u64;

    let mut offset = 0;
    for ((tool, key), members) in &by_partition {
        let len = members.len();
        if offset + len > observed.len() {
            return Err(GroupFailure::Membership(format!(
                "tool {tool} partition {key:?} needs {len} unit(s) but only {} remain",
                observed.len().saturating_sub(offset)
            )));
        }
        let got = &observed[offset..offset + len];
        offset += len;
        match key {
            SegmentKey::Skirt => {
                if !same_set(got, members) {
                    return Err(GroupFailure::SkirtOrder(format!(
                        "tool {tool}: skirt units {got:?} != authored skirt units {members:?}"
                    )));
                }
                stats.skirt_segments += 1;
            }
            SegmentKey::Wall => {
                if got != members.as_slice() {
                    return Err(GroupFailure::WallOrder(format!(
                        "tool {tool}: staged wall units {got:?} != authored wall order {members:?}"
                    )));
                }
                if members.len() >= 2 {
                    stats.wall_segments += 1;
                }
            }
            SegmentKey::Group(group) => {
                let observed_groups: BTreeSet<u32> = got
                    .iter()
                    .map(|&unit| characterized_group(&specs[units[unit].indices[0]].role))
                    .collect();
                if observed_groups != BTreeSet::from([*group]) {
                    return Err(GroupFailure::GroupOrder(format!(
                        "tool {tool}: expected group {group} units at unit position {} but                          observed {got:?} (first-member groups {observed_groups:?})",
                        offset - len
                    )));
                }
                if !same_set(got, members) {
                    return Err(GroupFailure::Membership(format!(
                        "tool {tool}: group {group} units {got:?} != authored members {members:?}"
                    )));
                }
                stats.group_segments += 1;
            }
        }
    }
    if offset != observed.len() {
        return Err(GroupFailure::Membership(format!(
            "{} unit(s) of {observed:?} are outside every authored partition",
            observed.len() - offset
        )));
    }
    Ok(())
}

/// The **one** group checker: the property, the deterministic witnesses, the
/// mutation controls and the shrink-state revalidation all call it.
fn check_groups(
    case: &GroupCase,
    limits: &Limits,
    observation: &GroupObservation,
) -> Result<GroupStats, GroupFailure> {
    validate_specs(&case.specs, limits).map_err(GroupFailure::Fixture)?;
    let n = case.specs.len();
    let order: Vec<usize> = observation
        .permutation
        .iter()
        .map(|&(index, _)| index as usize)
        .collect();

    if n == 0 {
        if !order.is_empty() || !observation.tool_changes.is_empty() {
            return Err(GroupFailure::Permutation(
                "an empty case produced output".to_owned(),
            ));
        }
        return Ok(GroupStats {
            empty: 1,
            ..GroupStats::default()
        });
    }
    if !is_permutation(&order, n) {
        return Err(GroupFailure::Permutation(format!(
            "entity indices {order:?} are not a bijection over 0..{n}"
        )));
    }
    if observation
        .permutation
        .iter()
        .any(|&(_, reversal)| reversal)
    {
        return Err(GroupFailure::Reversal(
            "the module set a reversal flag; the kernel is documented to always return false"
                .to_owned(),
        ));
    }

    // Partition by each unit's FIRST member (ADR-0062: a locked block is one
    // atomic unit, placed whole), tool clusters ascending.
    let mut stats = GroupStats {
        permutations: 1,
        ..GroupStats::default()
    };
    check_group_units(&case.specs, &order, &mut stats)?;

    let expected_changes = expected_tool_changes(&case.specs, &order);
    if observation.tool_changes != expected_changes {
        return Err(GroupFailure::ToolChanges(format!(
            "{:?} != expected {:?}",
            observation.tool_changes, expected_changes
        )));
    }
    stats.tool_changes = expected_changes.len() as u64;

    let partitions = tag_partition_map(&case.specs);
    stats.cross_partition_tags = partitions.values().filter(|keys| keys.len() > 1).count() as u64;
    stats.split_tags = split_tags(&case.specs, &order).len() as u64;
    stats.locked_runs = runs_of(&case.specs, &authored_indices(&case.specs))
        .iter()
        .filter(|run| run.lock.is_some() && run.indices.len() >= 2)
        .count() as u64;
    // ADR-0062: a block that spans tools, role groups or the wall/non-wall
    // boundary stays one contiguous forward window (checked by
    // `check_group_units`); no tag may be split.
    if stats.split_tags != 0 {
        return Err(GroupFailure::Block(format!(
            "{} ADR-0062 tag(s) are not one contiguous window in {order:?}",
            stats.split_tags
        )));
    }
    match case.mode {
        GroupMode::CrossRoleLock | GroupMode::CrossToolLock => {
            if stats.cross_partition_tags == 0 {
                return Err(GroupFailure::Witness(
                    "the cross-partition witness lost its spanning tag".to_owned(),
                ));
            }
        }
        GroupMode::WildcardMix => {
            if !case
                .specs
                .iter()
                .any(|spec| !role_group_is_documented(&spec.role))
            {
                return Err(GroupFailure::Witness(
                    "the wildcard witness lost its wildcard role".to_owned(),
                ));
            }
        }
        GroupMode::Empty | GroupMode::Ordinary => {}
    }
    Ok(stats)
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic witnesses
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct NearestWitnessStats {
    exact_ties: u64,
    window_wins: u64,
    separated_first: u64,
    empty_cases: u64,
    singleton_cases: u64,
}

fn nearest_witness_observations(limits: &Limits) -> NearestWitnessStats {
    let mut out = NearestWitnessStats::default();
    let checked = |case: &NearestCase| {
        let observation = observe_nearest(case);
        check_nearest(case, limits, &observation)
            .unwrap_or_else(|error| panic!("{} witness rejected: {error}", case.mode.label()))
    };

    // Exact two-candidate tie at 0.0625 mm: the lower index must win.
    let tie = build_nearest_case(NearestMode::Tie, &[0], limits);
    let stats = checked(&tie);
    assert_eq!(
        stats.first, 0,
        "the exact two-candidate tie must select the lower candidate index"
    );
    assert_eq!(
        stats.tie_minima, 1,
        "the tie witness must expose two minimum-distance candidates"
    );
    out.exact_ties += 1;

    // The documented window itself: candidate 0 is one quantum farther than
    // candidate 1 (0.0009765625 mm), which is inside the 0.001 mm window, so the
    // earlier, farther candidate wins.
    let window = NearestCase {
        mode: NearestMode::General,
        candidates: vec![
            CandidateQuanta {
                payload: 0,
                start: (33, 0),
                end: (64, 0),
            },
            CandidateQuanta {
                payload: 1,
                start: (32, 0),
                end: (96, 0),
            },
        ],
    };
    let stats = checked(&window);
    assert_eq!(
        stats.first, 0,
        "a candidate inside the 0.001 mm window must not displace the earlier index"
    );
    out.window_wins += 1;

    // Separated distances: candidate 1 is nearest, so it is selected first.
    let separated = NearestCase {
        mode: NearestMode::Separated,
        candidates: vec![
            CandidateQuanta {
                payload: 0,
                start: (256, 0),
                end: (320, 0),
            },
            CandidateQuanta {
                payload: 1,
                start: (64, 0),
                end: (128, 0),
            },
            CandidateQuanta {
                payload: 2,
                start: (128, 0),
                end: (192, 0),
            },
        ],
    };
    let stats = checked(&separated);
    assert_eq!(
        stats.first, 1,
        "the strictly nearest candidate must be first"
    );
    out.separated_first += 1;

    // Empty and singleton are separate exact assertions, never positives.
    let empty = build_nearest_case(NearestMode::Empty, &[], limits);
    let observation = observe_nearest(&empty);
    assert!(
        observation.permutation.is_empty(),
        "the empty case must return no entries"
    );
    checked(&empty);
    out.empty_cases += 1;

    let singleton = build_nearest_case(NearestMode::Singleton, &[3], limits);
    let observation = observe_nearest(&singleton);
    assert_eq!(
        observation.permutation,
        vec![(0, false)],
        "the singleton must return its only candidate with no reversal"
    );
    checked(&singleton);
    out.singleton_cases += 1;

    out
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct LockWitnessStats {
    block_units: u64,
    moved_blocks: u64,
    endpoint_wins: u64,
    separated_tags: u64,
    cross_tool_merges: u64,
    empty_cases: u64,
    singleton_cases: u64,
}

fn lock_witness_observations(limits: &Limits) -> LockWitnessStats {
    let mut out = LockWitnessStats::default();
    let checked = |case: &LockCase| {
        let observation = observe_locks(case);
        check_locks(case, limits, &observation)
            .unwrap_or_else(|error| panic!("{} witness rejected: {error}", case.mode.label()))
    };

    // ── A locked block moves as a unit past a nearer unrelated singleton ────
    let unit = LockCase {
        mode: LockMode::MixedBlocks,
        specs: vec![
            lock_spec(
                0,
                0,
                ExtrusionRole::SparseInfill,
                (1024, 0),
                (1536, 0),
                Some(7),
            ),
            lock_spec(
                1,
                0,
                ExtrusionRole::SparseInfill,
                (2048, 0),
                (2048, 0),
                Some(7),
            ),
            lock_spec(2, 0, ExtrusionRole::SparseInfill, (256, 0), (256, 0), None),
        ],
    };
    let observation = observe_locks(&unit);
    let stats = checked(&unit);
    assert_eq!(
        observation.permutation,
        vec![(2, false), (0, false), (1, false)],
        "the nearer singleton prints first, then the block in authored order"
    );
    assert!(stats.blocks >= 1 && stats.moved_blocks >= 1);
    out.block_units += stats.blocks;
    out.moved_blocks += stats.moved_blocks;

    // ── First-start/last-end: the cursor after the block is its LAST end ────
    let endpoint = LockCase {
        mode: LockMode::MixedBlocks,
        specs: vec![
            lock_spec(
                0,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(7),
            ),
            lock_spec(
                1,
                0,
                ExtrusionRole::SparseInfill,
                (768, 0),
                (1024, 0),
                Some(7),
            ),
            lock_spec(
                2,
                0,
                ExtrusionRole::SparseInfill,
                (512, 512),
                (512, 512),
                None,
            ),
            lock_spec(
                3,
                0,
                ExtrusionRole::SparseInfill,
                (1536, 0),
                (1536, 0),
                None,
            ),
        ],
    };
    let observation = observe_locks(&endpoint);
    checked(&endpoint);
    assert_eq!(
        observation.permutation,
        vec![(0, false), (1, false), (3, false), (2, false)],
        "from the block's last end (1.0, 0) the singleton at 1.5 mm is nearer than the one at \
         (0.5, 0.5) mm; from the first end (0.5, 0) the order would reverse"
    );
    out.endpoint_wins += 1;

    // ── Separated tags: two authored runs of one tag (each stays a unit) ────
    let separated = LockCase {
        mode: LockMode::SeparatedTag,
        specs: vec![
            lock_spec(
                0,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(7),
            ),
            lock_spec(
                1,
                0,
                ExtrusionRole::SparseInfill,
                (1024, 0),
                (1280, 0),
                None,
            ),
            lock_spec(
                2,
                0,
                ExtrusionRole::SparseInfill,
                (2048, 0),
                (2048, 0),
                Some(7),
            ),
        ],
    };
    let stats = checked(&separated);
    assert_eq!(
        stats.separated_tags, 1,
        "the tag must be observed as two runs"
    );
    out.separated_tags += stats.separated_tags;

    // ── A cross-tool block is one unit (ADR-0062): placed whole in its first
    //    member's tool cluster, adjacent, forward ─────────────────────────────
    let cross_tool = LockCase {
        mode: LockMode::CrossToolBlock,
        specs: vec![
            lock_spec(
                0,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(9),
            ),
            lock_spec(
                1,
                1,
                ExtrusionRole::SparseInfill,
                (768, 0),
                (1024, 0),
                Some(9),
            ),
            lock_spec(2, 0, ExtrusionRole::Ironing, (1280, 0), (1536, 0), None),
            lock_spec(
                3,
                1,
                ExtrusionRole::TopSolidInfill,
                (1792, 0),
                (2048, 0),
                None,
            ),
        ],
    };
    let observation = observe_locks(&cross_tool);
    let stats = checked(&cross_tool);
    assert_eq!(
        observation.candidates.len(),
        3,
        "the direct kernel coalesces the cross-tool tag into one candidate"
    );
    assert_eq!(
        observation.candidates[0].indices,
        vec![0, 1],
        "the merged candidate keeps both authored indices in order"
    );
    assert_eq!(
        observation.permutation,
        vec![(0, false), (1, false), (2, false), (3, false)],
        "the tag-9 block [0, 1] spans tools 0 and 1 yet stays adjacent, forward and unreversed"
    );
    assert_eq!(stats.cross_partition_blocks, 1);
    out.cross_tool_merges += 1;

    // ── Empty and singleton, separately ─────────────────────────────────────
    let empty = LockCase {
        mode: LockMode::Empty,
        specs: Vec::new(),
    };
    let observation = observe_locks(&empty);
    assert!(observation.candidates.is_empty() && observation.permutation.is_empty());
    checked(&empty);
    out.empty_cases += 1;

    let singleton = LockCase {
        mode: LockMode::Singleton,
        specs: vec![lock_spec(
            0,
            0,
            ExtrusionRole::SparseInfill,
            (256, 0),
            (512, 0),
            None,
        )],
    };
    let observation = observe_locks(&singleton);
    assert_eq!(observation.permutation, vec![(0, false)]);
    checked(&singleton);
    out.singleton_cases += 1;

    out
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct GroupWitnessStats {
    wildcard_last: u64,
    support_after_infill: u64,
    staged_walls: u64,
    skirt_first: u64,
    tool_clusters: u64,
    tool_change_records: u64,
    cross_role_fragments: u64,
    cross_tool_fragments: u64,
    empty_cases: u64,
    singleton_cases: u64,
}

fn group_witness_observations(limits: &Limits) -> GroupWitnessStats {
    let mut out = GroupWitnessStats::default();
    let checked = |case: &GroupCase| {
        let observation = observe_groups(case);
        check_groups(case, limits, &observation)
            .unwrap_or_else(|error| panic!("{} witness rejected: {error}", case.mode.label()))
    };

    // ── InternalBridgeInfill / SupportMaterial wildcard witness ─────────────
    let wildcard = GroupCase {
        mode: GroupMode::WildcardMix,
        specs: vec![
            lock_spec(0, 0, ExtrusionRole::SparseInfill, (256, 0), (512, 0), None),
            lock_spec(
                1,
                0,
                ExtrusionRole::TopSolidInfill,
                (512, 0),
                (768, 0),
                None,
            ),
            lock_spec(2, 0, ExtrusionRole::Ironing, (1024, 0), (1280, 0), None),
            lock_spec(
                3,
                0,
                ExtrusionRole::SupportMaterial,
                (1280, 0),
                (1536, 0),
                None,
            ),
            lock_spec(4, 0, ExtrusionRole::WipeTower, (1536, 0), (1792, 0), None),
            lock_spec(
                5,
                0,
                ExtrusionRole::InternalBridgeInfill,
                (1792, 0),
                (2048, 0),
                None,
            ),
        ],
    };
    let observation = observe_groups(&wildcard);
    let stats = checked(&wildcard);
    assert_eq!(
        observation.permutation,
        vec![
            (0, false),
            (1, false),
            (2, false),
            (3, false),
            (4, false),
            (5, false)
        ],
        "infill group, ironing, support, tower, then the wildcard group-9 role"
    );
    let groups: Vec<u32> = observation
        .permutation
        .iter()
        .map(|&(slot, _)| characterized_group(&wildcard.specs[slot as usize].role))
        .collect();
    assert_eq!(
        groups,
        vec![4, 4, 5, 6, 8, 9],
        "the authored group order, with InternalBridgeInfill last as group 9"
    );
    assert!(
        !role_group_is_documented(&ExtrusionRole::InternalBridgeInfill),
        "InternalBridgeInfill reaches the characterised wildcard"
    );
    out.wildcard_last += 1;
    assert!(stats.group_segments >= 4 && stats.wall_segments == 0);
    out.support_after_infill += 1;

    // ── Staged walls preserved in authored order ───────────────────────────
    let walls = GroupCase {
        mode: GroupMode::Ordinary,
        specs: vec![
            lock_spec(0, 0, ExtrusionRole::OuterWall, (1024, 0), (1280, 0), None),
            lock_spec(1, 0, ExtrusionRole::InnerWall, (256, 0), (512, 0), None),
            lock_spec(2, 0, ExtrusionRole::InnerWall, (512, 0), (768, 0), None),
            lock_spec(3, 0, ExtrusionRole::SparseInfill, (128, 0), (256, 0), None),
        ],
    };
    let observation = observe_groups(&walls);
    let stats = checked(&walls);
    assert_eq!(
        observation.permutation,
        vec![(0, false), (1, false), (2, false), (3, false)],
        "OuterWall stays before the staged InnerWalls; the optimizer never re-sorts them"
    );
    assert!(stats.wall_segments >= 1);
    out.staged_walls += stats.wall_segments;

    // ── Skirt before the staged wall block ─────────────────────────────────
    let skirt = GroupCase {
        mode: GroupMode::Ordinary,
        specs: vec![
            lock_spec(0, 0, ExtrusionRole::InnerWall, (128, 0), (256, 0), None),
            lock_spec(1, 0, ExtrusionRole::Skirt, (1024, 0), (1280, 0), None),
            lock_spec(2, 0, ExtrusionRole::Skirt, (2048, 0), (2048, 0), None),
        ],
    };
    let observation = observe_groups(&skirt);
    let stats = checked(&skirt);
    assert_eq!(
        observation.permutation,
        vec![(1, false), (2, false), (0, false)],
        "skirts are the cluster prefix, then the wall"
    );
    assert!(stats.skirt_segments >= 1);
    out.skirt_first += 1;

    // ── Ascending tool clusters and the exact tool-change records ──────────
    let tools = GroupCase {
        mode: GroupMode::Ordinary,
        specs: vec![
            lock_spec(0, 2, ExtrusionRole::SparseInfill, (256, 0), (512, 0), None),
            lock_spec(1, 0, ExtrusionRole::SparseInfill, (512, 0), (768, 0), None),
            lock_spec(2, 1, ExtrusionRole::SparseInfill, (768, 0), (1024, 0), None),
        ],
    };
    let observation = observe_groups(&tools);
    let stats = checked(&tools);
    assert_eq!(
        observation.permutation,
        vec![(1, false), (2, false), (0, false)],
        "tool clusters ascend regardless of authored order"
    );
    assert_eq!(
        observation.tool_changes,
        vec![(0, 0, 1), (1, 1, 2)],
        "one record at each cluster boundary, anchored after the previous entity"
    );
    out.tool_clusters += stats.tool_blocks;
    out.tool_change_records += stats.tool_changes;

    // ── Cross-role lock: one ADR-0062 unit spanning wall + two infill groups ─
    let cross_role = GroupCase {
        mode: GroupMode::CrossRoleLock,
        specs: vec![
            lock_spec(0, 0, ExtrusionRole::InnerWall, (128, 0), (256, 0), Some(9)),
            lock_spec(
                1,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(9),
            ),
            lock_spec(
                2,
                0,
                ExtrusionRole::SparseInfill,
                (512, 0),
                (768, 0),
                Some(9),
            ),
            lock_spec(3, 0, ExtrusionRole::Ironing, (768, 0), (1024, 0), Some(9)),
            lock_spec(4, 0, ExtrusionRole::InnerWall, (1024, 0), (1280, 0), None),
            lock_spec(
                5,
                0,
                ExtrusionRole::SparseInfill,
                (1280, 0),
                (1536, 0),
                None,
            ),
            lock_spec(6, 0, ExtrusionRole::Ironing, (1536, 0), (1792, 0), None),
        ],
    };
    let observation = observe_groups(&cross_role);
    let stats = checked(&cross_role);
    assert_eq!(
        observation.permutation,
        vec![
            (0, false),
            (1, false),
            (2, false),
            (3, false),
            (4, false),
            (5, false),
            (6, false)
        ],
        "the tag-9 block [0, 1, 2, 3] is led by a wall, so it is placed whole in the wall          segment (adjacent, forward, unreversed), then the untagged wall, group 4, group 5"
    );
    assert_eq!(stats.cross_partition_tags, 1);
    assert_eq!(stats.split_tags, 0);
    out.cross_role_fragments += 1;

    // ── Cross-tool lock: one unit placed whole in its first member's tool ───
    let cross_tool = GroupCase {
        mode: GroupMode::CrossToolLock,
        specs: vec![
            lock_spec(
                0,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(9),
            ),
            lock_spec(
                1,
                1,
                ExtrusionRole::SparseInfill,
                (768, 0),
                (1024, 0),
                Some(9),
            ),
            lock_spec(2, 0, ExtrusionRole::Ironing, (1280, 0), (1536, 0), None),
            lock_spec(
                3,
                1,
                ExtrusionRole::TopSolidInfill,
                (1792, 0),
                (2048, 0),
                None,
            ),
        ],
    };
    let observation = observe_groups(&cross_tool);
    let stats = checked(&cross_tool);
    assert_eq!(
        observation.permutation,
        vec![(0, false), (1, false), (2, false), (3, false)],
        "the tag-9 block [0, 1] is led by tool 0 so it stays adjacent in the tool-0 cluster"
    );
    assert_eq!(stats.cross_partition_tags, 1);
    assert_eq!(stats.split_tags, 0);
    out.cross_tool_fragments += 1;

    // ── Empty and singleton, separately ─────────────────────────────────────
    let empty = GroupCase {
        mode: GroupMode::Empty,
        specs: Vec::new(),
    };
    let observation = observe_groups(&empty);
    assert!(observation.permutation.is_empty() && observation.tool_changes.is_empty());
    checked(&empty);
    out.empty_cases += 1;

    let singleton = GroupCase {
        mode: GroupMode::Ordinary,
        specs: vec![lock_spec(
            0,
            0,
            ExtrusionRole::OuterWall,
            (256, 0),
            (512, 0),
            None,
        )],
    };
    let observation = observe_groups(&singleton);
    assert_eq!(observation.permutation, vec![(0, false)]);
    checked(&singleton);
    out.singleton_cases += 1;

    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic mutation controls
// ─────────────────────────────────────────────────────────────────────────────

fn expect_nearest_failure(
    case: &NearestCase,
    limits: &Limits,
    baseline: &NearestObservation,
    mutant: &NearestObservation,
    what: &str,
    matches_class: impl Fn(&NearestFailure) -> bool,
) {
    check_nearest(case, limits, baseline)
        .unwrap_or_else(|error| panic!("{what}: the unmodified observation was rejected: {error}"));
    match check_nearest(case, limits, mutant) {
        Ok(_) => panic!("{what}: the mutation was accepted"),
        Err(error) => assert!(
            matches_class(&error),
            "{what}: rejected for the wrong reason: {error}"
        ),
    }
}

fn is_nearest_permutation(error: &NearestFailure) -> bool {
    matches!(error, NearestFailure::Permutation(_))
}

fn is_nearest_tie(error: &NearestFailure) -> bool {
    matches!(error, NearestFailure::TieWinner(_))
}

fn is_nearest_reversal(error: &NearestFailure) -> bool {
    matches!(error, NearestFailure::Reversal(_))
}

fn nearest_controls(limits: &Limits) {
    // Exact tie pair: [0.0625 mm on x] vs [0.0625 mm on y].
    let case = build_nearest_case(NearestMode::Tie, &[0], limits);
    let baseline = observe_nearest(&case);
    check_nearest(&case, limits, &baseline).expect("unmodified tie case");

    let mut tie_lost = baseline.clone();
    tie_lost.permutation.swap(0, 1);
    expect_nearest_failure(
        &case,
        limits,
        &baseline,
        &tie_lost,
        "violated exact tie winner",
        is_nearest_tie,
    );

    let mut duplicated = baseline.clone();
    duplicated.permutation.push(duplicated.permutation[0]);
    expect_nearest_failure(
        &case,
        limits,
        &baseline,
        &duplicated,
        "duplicated candidate index",
        is_nearest_permutation,
    );

    let mut dropped = baseline.clone();
    dropped.permutation.pop();
    expect_nearest_failure(
        &case,
        limits,
        &baseline,
        &dropped,
        "dropped candidate index",
        is_nearest_permutation,
    );

    let mut reversed = baseline.clone();
    reversed.permutation[0].1 = true;
    expect_nearest_failure(
        &case,
        limits,
        &baseline,
        &reversed,
        "set reversal flag",
        is_nearest_reversal,
    );

    // The window witness must reject a route that prefers the strictly nearer
    // candidate, because the documented window keeps the earlier index.
    let window = NearestCase {
        mode: NearestMode::General,
        candidates: vec![
            CandidateQuanta {
                payload: 0,
                start: (33, 0),
                end: (64, 0),
            },
            CandidateQuanta {
                payload: 1,
                start: (32, 0),
                end: (96, 0),
            },
        ],
    };
    let window_baseline = observe_nearest(&window);
    check_nearest(&window, limits, &window_baseline).expect("unmodified window case");
    let mut window_lost = window_baseline.clone();
    window_lost.permutation = vec![(1, false), (0, false)];
    expect_nearest_failure(
        &window,
        limits,
        &window_baseline,
        &window_lost,
        "ignored the 0.001 mm earlier-index window",
        is_nearest_tie,
    );

    println!("path_nearest_controls: every named mutation rejected by the shared checker");
}

fn expect_lock_failure(
    case: &LockCase,
    limits: &Limits,
    baseline: &LockObservation,
    mutant: &LockObservation,
    what: &str,
    matches_class: impl Fn(&LockFailure) -> bool,
) {
    check_locks(case, limits, baseline)
        .unwrap_or_else(|error| panic!("{what}: the unmodified observation was rejected: {error}"));
    match check_locks(case, limits, mutant) {
        Ok(_) => panic!("{what}: the mutation was accepted"),
        Err(error) => assert!(
            matches_class(&error),
            "{what}: rejected for the wrong reason: {error}"
        ),
    }
}

fn is_lock_coalescing(error: &LockFailure) -> bool {
    matches!(error, LockFailure::Coalescing(_))
}

fn is_lock_permutation(error: &LockFailure) -> bool {
    matches!(error, LockFailure::Permutation(_))
}

fn is_lock_block(error: &LockFailure) -> bool {
    matches!(error, LockFailure::Block(_))
}

fn is_lock_reversal(error: &LockFailure) -> bool {
    matches!(error, LockFailure::Reversal(_))
}

fn lock_controls(limits: &Limits) {
    let case = LockCase {
        mode: LockMode::MixedBlocks,
        specs: vec![
            lock_spec(
                0,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(7),
            ),
            lock_spec(
                1,
                0,
                ExtrusionRole::SparseInfill,
                (768, 0),
                (1024, 0),
                Some(7),
            ),
            lock_spec(
                2,
                0,
                ExtrusionRole::SparseInfill,
                (1280, 0),
                (1536, 0),
                None,
            ),
        ],
    };
    let baseline = observe_locks(&case);
    check_locks(&case, limits, &baseline).expect("unmodified locked block");
    assert_eq!(
        baseline.permutation,
        vec![(0, false), (1, false), (2, false)]
    );

    let mut split = baseline.clone();
    split.permutation = vec![(0, false), (2, false), (1, false)];
    expect_lock_failure(
        &case,
        limits,
        &baseline,
        &split,
        "split locked block",
        is_lock_block,
    );

    let mut reversed = baseline.clone();
    reversed.permutation = vec![(1, false), (0, false), (2, false)];
    expect_lock_failure(
        &case,
        limits,
        &baseline,
        &reversed,
        "reversed locked block",
        is_lock_block,
    );

    let mut flag = baseline.clone();
    flag.permutation[0].1 = true;
    expect_lock_failure(
        &case,
        limits,
        &baseline,
        &flag,
        "set one reversal flag",
        is_lock_reversal,
    );

    let mut duplicated = baseline.clone();
    duplicated.permutation.push((1, false));
    expect_lock_failure(
        &case,
        limits,
        &baseline,
        &duplicated,
        "duplicated index",
        is_lock_permutation,
    );

    let mut dropped = baseline.clone();
    dropped.permutation.pop();
    expect_lock_failure(
        &case,
        limits,
        &baseline,
        &dropped,
        "dropped index",
        is_lock_permutation,
    );

    // The candidate's end must be the LAST member's end. Substituting the first
    // member's end is exactly the documented first-versus-last mistake.
    let mut first_end = baseline.clone();
    first_end.candidates[0].end = (quantum_mm(512), quantum_mm(0));
    expect_lock_failure(
        &case,
        limits,
        &baseline,
        &first_end,
        "first block end substituted for the last",
        is_lock_coalescing,
    );

    let mut merged = baseline.clone();
    let tail = merged.candidates.remove(1);
    merged.candidates[0].indices.extend(tail.indices);
    merged.candidates[0].end = tail.end;
    expect_lock_failure(
        &case,
        limits,
        &baseline,
        &merged,
        "merged two candidates",
        is_lock_coalescing,
    );

    let mut split_candidate = baseline.clone();
    let removed = split_candidate.candidates.remove(0);
    let mut first = removed.clone();
    first.indices = vec![0];
    first.end = (quantum_mm(512), quantum_mm(0));
    let mut second = removed;
    second.indices = vec![1];
    second.start = (quantum_mm(768), quantum_mm(0));
    split_candidate.candidates.insert(0, second);
    split_candidate.candidates.insert(0, first);
    expect_lock_failure(
        &case,
        limits,
        &baseline,
        &split_candidate,
        "split one candidate into two",
        is_lock_coalescing,
    );

    // ── Cross-tool block through the shared lock checker ───────────────────
    let cross_tool = LockCase {
        mode: LockMode::CrossToolBlock,
        specs: vec![
            lock_spec(
                0,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(9),
            ),
            lock_spec(
                1,
                1,
                ExtrusionRole::SparseInfill,
                (768, 0),
                (1024, 0),
                Some(9),
            ),
            lock_spec(2, 0, ExtrusionRole::Ironing, (1280, 0), (1536, 0), None),
            lock_spec(
                3,
                1,
                ExtrusionRole::TopSolidInfill,
                (1792, 0),
                (2048, 0),
                None,
            ),
        ],
    };
    let cross_baseline = observe_locks(&cross_tool);
    check_locks(&cross_tool, limits, &cross_baseline)
        .expect("unmodified cross-tool block must be accepted");
    assert_eq!(
        cross_baseline.permutation,
        vec![(0, false), (1, false), (2, false), (3, false)],
        "a contiguous cross-tool block is the required output"
    );
    let mut cross_split = cross_baseline.clone();
    cross_split.permutation = vec![(0, false), (2, false), (1, false), (3, false)];
    expect_lock_failure(
        &cross_tool,
        limits,
        &cross_baseline,
        &cross_split,
        "the former tool-partition split of a cross-tool block",
        is_lock_block,
    );

    // A separated tag is not one ADR-0062 block: its two runs are independent
    // candidates.
    let separated = LockCase {
        mode: LockMode::SeparatedTag,
        specs: vec![
            lock_spec(
                0,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(7),
            ),
            lock_spec(1, 0, ExtrusionRole::SparseInfill, (768, 0), (1024, 0), None),
            lock_spec(
                2,
                0,
                ExtrusionRole::SparseInfill,
                (1280, 0),
                (1536, 0),
                Some(7),
            ),
        ],
    };
    let separated_baseline = observe_locks(&separated);
    check_locks(&separated, limits, &separated_baseline).expect("unmodified separated-tag witness");
    assert_eq!(separated_baseline.candidates.len(), 3);

    println!("path_lock_controls: every named mutation rejected by the shared checker");
}

fn expect_group_failure(
    case: &GroupCase,
    limits: &Limits,
    baseline: &GroupObservation,
    mutant: &GroupObservation,
    what: &str,
    matches_class: impl Fn(&GroupFailure) -> bool,
) {
    check_groups(case, limits, baseline)
        .unwrap_or_else(|error| panic!("{what}: the unmodified observation was rejected: {error}"));
    match check_groups(case, limits, mutant) {
        Ok(_) => panic!("{what}: the mutation was accepted"),
        Err(error) => assert!(
            matches_class(&error),
            "{what}: rejected for the wrong reason: {error}"
        ),
    }
}

fn is_group_wall(error: &GroupFailure) -> bool {
    matches!(error, GroupFailure::WallOrder(_))
}

fn is_group_order(error: &GroupFailure) -> bool {
    matches!(error, GroupFailure::GroupOrder(_))
}

fn is_group_block(error: &GroupFailure) -> bool {
    matches!(error, GroupFailure::Block(_))
}

fn is_group_permutation(error: &GroupFailure) -> bool {
    matches!(error, GroupFailure::Permutation(_))
}

fn is_group_reversal(error: &GroupFailure) -> bool {
    matches!(error, GroupFailure::Reversal(_))
}

fn is_group_tool_changes(error: &GroupFailure) -> bool {
    matches!(error, GroupFailure::ToolChanges(_))
}

fn is_group_membership(error: &GroupFailure) -> bool {
    matches!(error, GroupFailure::Membership(_))
}

fn group_controls(limits: &Limits) {
    // ── Staged wall order ──────────────────────────────────────────────────
    let walls = GroupCase {
        mode: GroupMode::Ordinary,
        specs: vec![
            lock_spec(0, 0, ExtrusionRole::OuterWall, (1024, 0), (1280, 0), None),
            lock_spec(1, 0, ExtrusionRole::InnerWall, (256, 0), (512, 0), None),
            lock_spec(2, 0, ExtrusionRole::InnerWall, (512, 0), (768, 0), None),
        ],
    };
    let wall_baseline = observe_groups(&walls);
    check_groups(&walls, limits, &wall_baseline).expect("unmodified staged walls");
    let mut reordered = wall_baseline.clone();
    reordered.permutation = vec![(1, false), (0, false), (2, false)];
    expect_group_failure(
        &walls,
        limits,
        &wall_baseline,
        &reordered,
        "reordered staged walls",
        is_group_wall,
    );

    // ── Support before the documented infill witness ───────────────────────
    let wildcard = GroupCase {
        mode: GroupMode::WildcardMix,
        specs: vec![
            lock_spec(0, 0, ExtrusionRole::SparseInfill, (256, 0), (512, 0), None),
            lock_spec(
                1,
                0,
                ExtrusionRole::TopSolidInfill,
                (512, 0),
                (768, 0),
                None,
            ),
            lock_spec(2, 0, ExtrusionRole::Ironing, (1024, 0), (1280, 0), None),
            lock_spec(
                3,
                0,
                ExtrusionRole::SupportMaterial,
                (1280, 0),
                (1536, 0),
                None,
            ),
            lock_spec(4, 0, ExtrusionRole::WipeTower, (1536, 0), (1792, 0), None),
            lock_spec(
                5,
                0,
                ExtrusionRole::InternalBridgeInfill,
                (1792, 0),
                (2048, 0),
                None,
            ),
        ],
    };
    let wildcard_baseline = observe_groups(&wildcard);
    check_groups(&wildcard, limits, &wildcard_baseline).expect("unmodified wildcard witness");
    let mut support_first = wildcard_baseline.clone();
    support_first.permutation = vec![
        (3, false),
        (0, false),
        (1, false),
        (2, false),
        (4, false),
        (5, false),
    ];
    expect_group_failure(
        &wildcard,
        limits,
        &wildcard_baseline,
        &support_first,
        "support before the documented infill witness",
        is_group_order,
    );

    // ── Cross-role lock (ADR-0062): the block must stay one forward window ──
    let cross_role = GroupCase {
        mode: GroupMode::CrossRoleLock,
        specs: vec![
            lock_spec(0, 0, ExtrusionRole::InnerWall, (128, 0), (256, 0), Some(9)),
            lock_spec(
                1,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(9),
            ),
            lock_spec(
                2,
                0,
                ExtrusionRole::SparseInfill,
                (512, 0),
                (768, 0),
                Some(9),
            ),
            lock_spec(3, 0, ExtrusionRole::Ironing, (768, 0), (1024, 0), Some(9)),
            lock_spec(4, 0, ExtrusionRole::InnerWall, (1024, 0), (1280, 0), None),
            lock_spec(
                5,
                0,
                ExtrusionRole::SparseInfill,
                (1280, 0),
                (1536, 0),
                None,
            ),
            lock_spec(6, 0, ExtrusionRole::Ironing, (1536, 0), (1792, 0), None),
        ],
    };
    let cross_baseline = observe_groups(&cross_role);
    check_groups(&cross_role, limits, &cross_baseline)
        .expect("unmodified cross-role ADR-0062 block");
    assert_eq!(
        cross_baseline.permutation,
        (0..7).map(|slot| (slot, false)).collect::<Vec<_>>(),
        "the block leads with a wall and is placed whole in the wall segment"
    );
    // The former partition-first output: the per-partition fragments of the
    // tag-9 block with the untagged wall in the middle. It is exactly what
    // ADR-0062 forbids and the checker must reject it.
    let perm_of = |slots: [u32; 7]| slots.map(|slot| (slot, false)).to_vec();
    let mut legacy_split = cross_baseline.clone();
    legacy_split.permutation = perm_of([0, 4, 1, 2, 5, 3, 6]);
    expect_group_failure(
        &cross_role,
        limits,
        &cross_baseline,
        &legacy_split,
        "the former partition-first split of a cross-role block",
        is_group_block,
    );
    let mut interleaved = cross_baseline.clone();
    interleaved.permutation = perm_of([0, 1, 5, 2, 3, 4, 6]);
    expect_group_failure(
        &cross_role,
        limits,
        &cross_baseline,
        &interleaved,
        "an unrelated entity interleaved inside a cross-role block",
        is_group_block,
    );
    let mut block_reversed = cross_baseline.clone();
    block_reversed.permutation = perm_of([3, 2, 1, 0, 4, 5, 6]);
    expect_group_failure(
        &cross_role,
        limits,
        &cross_baseline,
        &block_reversed,
        "a cross-role block emitted in reverse member order",
        is_group_block,
    );
    let mut block_flag = cross_baseline.clone();
    block_flag.permutation[1].1 = true;
    expect_group_failure(
        &cross_role,
        limits,
        &cross_baseline,
        &block_flag,
        "a reversal flag on a cross-role block member",
        is_group_reversal,
    );

    // ── Cross-tool lock (ADR-0062) ─────────────────────────────────────────
    let cross_tool = GroupCase {
        mode: GroupMode::CrossToolLock,
        specs: vec![
            lock_spec(
                0,
                0,
                ExtrusionRole::SparseInfill,
                (256, 0),
                (512, 0),
                Some(9),
            ),
            lock_spec(
                1,
                1,
                ExtrusionRole::SparseInfill,
                (768, 0),
                (1024, 0),
                Some(9),
            ),
            lock_spec(2, 0, ExtrusionRole::Ironing, (1280, 0), (1536, 0), None),
            lock_spec(
                3,
                1,
                ExtrusionRole::TopSolidInfill,
                (1792, 0),
                (2048, 0),
                None,
            ),
        ],
    };
    let tool_cross_baseline = observe_groups(&cross_tool);
    check_groups(&cross_tool, limits, &tool_cross_baseline)
        .expect("unmodified cross-tool ADR-0062 block");
    let mut tool_split = tool_cross_baseline.clone();
    tool_split.permutation = vec![(0, false), (2, false), (1, false), (3, false)];
    expect_group_failure(
        &cross_tool,
        limits,
        &tool_cross_baseline,
        &tool_split,
        "the former tool-partition split of a cross-tool block",
        is_group_block,
    );

    // ── Permutation, reversal and tool-change mutations ────────────────────
    let tools = GroupCase {
        mode: GroupMode::Ordinary,
        specs: vec![
            lock_spec(0, 2, ExtrusionRole::SparseInfill, (256, 0), (512, 0), None),
            lock_spec(1, 0, ExtrusionRole::SparseInfill, (512, 0), (768, 0), None),
            lock_spec(2, 1, ExtrusionRole::SparseInfill, (768, 0), (1024, 0), None),
        ],
    };
    let tool_baseline = observe_groups(&tools);
    check_groups(&tools, limits, &tool_baseline).expect("unmodified tool clusters");

    let mut duplicated = tool_baseline.clone();
    duplicated.permutation.push((1, false));
    expect_group_failure(
        &tools,
        limits,
        &tool_baseline,
        &duplicated,
        "duplicated index",
        is_group_permutation,
    );

    let mut dropped = tool_baseline.clone();
    dropped.permutation.pop();
    expect_group_failure(
        &tools,
        limits,
        &tool_baseline,
        &dropped,
        "dropped index",
        is_group_permutation,
    );

    let mut flag = tool_baseline.clone();
    flag.permutation[0].1 = true;
    expect_group_failure(
        &tools,
        limits,
        &tool_baseline,
        &flag,
        "set one reversal flag",
        is_group_reversal,
    );

    let mut cluster = tool_baseline.clone();
    cluster.permutation = vec![(1, false), (0, false), (2, false)];
    expect_group_failure(
        &tools,
        limits,
        &tool_baseline,
        &cluster,
        "tool clusters out of order",
        is_group_membership,
    );

    let mut change = tool_baseline.clone();
    change.tool_changes = vec![(0, 0, 1)];
    expect_group_failure(
        &tools,
        limits,
        &tool_baseline,
        &change,
        "missing tool-change record",
        is_group_tool_changes,
    );

    println!("path_group_controls: every named mutation rejected by the shared checker");
}

// ─────────────────────────────────────────────────────────────────────────────
// Profile rejection matrix
// ─────────────────────────────────────────────────────────────────────────────

fn invalid_profile_matrix() {
    let valid = probe_bound_map();
    for count_key in ["max_candidates", "max_entities"] {
        let invalid =
            |label: &str, bounds: BTreeMap<String, u64>| match checked_limits(&bounds, count_key) {
                Err(DomainError::InvalidBounds(message)) => {
                    assert!(!message.is_empty(), "{label}: empty reason")
                }
                other => panic!("{label}: expected InvalidBounds, got {other:?}"),
            };

        for key in BOUND_KEYS {
            let mut bounds = valid.clone();
            bounds.remove(key);
            if key == "max_candidates" && count_key == "max_entities" {
                // Not consumed by this record shape; removing it is harmless.
                checked_limits(&bounds, count_key).expect("an unconsumed key is not required");
                continue;
            }
            if key == "max_entities" && count_key == "max_candidates" {
                checked_limits(&bounds, count_key).expect("an unconsumed key is not required");
                continue;
            }
            invalid(&format!("missing {key}"), bounds);
        }
        for (key, value) in [
            (count_key, 1),
            (count_key, 33),
            ("max_coord_quanta", 2_047),
            ("max_coord_quanta", 65_537),
            ("max_coord_quanta", 0),
        ] {
            let mut bounds = valid.clone();
            bounds.insert(key.to_owned(), value);
            invalid(&format!("invalid {key}={value}"), bounds);
        }
        let mut overflow = valid.clone();
        overflow.insert(count_key.to_owned(), u64::from(u32::MAX) + 1);
        assert!(matches!(
            checked_limits(&overflow, count_key),
            Err(DomainError::ArithmeticOverflow)
        ));
        let mut coord_overflow = valid.clone();
        coord_overflow.insert("max_coord_quanta".to_owned(), i64::MAX as u64 + 1);
        assert!(matches!(
            checked_limits(&coord_overflow, count_key),
            Err(DomainError::ArithmeticOverflow)
        ));
    }
    assert!(matches!(
        checked_limits(&valid, "max_vertices"),
        Err(DomainError::InvalidBounds(_))
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink probe
// ─────────────────────────────────────────────────────────────────────────────

fn shrink_transitions<T: Clone + fmt::Debug + PartialEq>(
    strategy: &BoxedStrategy<T>,
    label: &str,
    require_reduction: bool,
    complexity: impl Fn(&T) -> i128,
    revalidate: impl Fn(&T),
) {
    // Phase 1: accept every simplification (as if each still failed) and show a
    // real reduction from the deterministic start.
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    let initial = tree.current();
    revalidate(&initial);
    let initial_complexity = complexity(&initial);
    let mut accepted = 0_usize;
    while tree.simplify() {
        accepted += 1;
        assert!(
            accepted < MAX_SHRINK_STEPS,
            "{label}: simplify never exhausted"
        );
        revalidate(&tree.current());
    }
    let minimal_complexity = complexity(&tree.current());
    if require_reduction {
        assert!(accepted > 0, "{label}: the tree never simplified");
        assert!(
            minimal_complexity < initial_complexity,
            "{label}: no permitted reduction ({initial_complexity} -> {minimal_complexity})"
        );
    }

    // Phase 2: alternate simplify/complicate, revalidating every current value.
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    // test-quality: determinism pin -- the same seeded runner must start the
    // shrink probe from the same generated fixture (protected surface: the
    // strategy's raw-to-fixture mapping).
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
        revalidate(&current);
        changed |= current != initial;
        if tree.complicate() {
            complicated += 1;
        }
        revalidate(&tree.current());
    }
    if require_reduction {
        assert!(
            simplified > 0 && changed,
            "{label}: no real simplify transition"
        );
    }
    let exhausted = tree.current();
    assert!(
        !tree.simplify(),
        "an exhausted {label} tree simplified again"
    );
    assert_eq!(
        tree.current(),
        exhausted,
        "a false simplify moved the {label} value"
    );
    let mut restores = 0_usize;
    while tree.complicate() {
        restores += 1;
        assert!(
            restores < MAX_SHRINK_STEPS,
            "{label}: complicate never ended"
        );
        revalidate(&tree.current());
    }
    let settled = tree.current();
    assert!(
        !tree.complicate(),
        "complicate after exhaustion must be false for {label}"
    );
    assert_eq!(
        tree.current(),
        settled,
        "a false complicate moved the {label} value"
    );
    println!(
        "{label}: phase1_simplify={accepted} complexity {initial_complexity}->\
         {minimal_complexity} phase2_simplify={simplified} complicate={complicated} \
         restores={restores}"
    );
}

fn revalidate_nearest(case: &NearestCase, mode: NearestMode, limits: &Limits) {
    assert_eq!(case.mode, mode, "the nearest mode must survive shrinking");
    let observation = observe_nearest(case);
    let stats = check_nearest(case, limits, &observation)
        .unwrap_or_else(|error| panic!("shrunk {} case: {error}", mode.label()));
    if mode == NearestMode::Tie {
        assert!(
            stats.tie_minima > 0,
            "the required exact tie witness vanished"
        );
    }
}

fn nearest_complexity(case: &NearestCase) -> i128 {
    let mut total = case.candidates.len() as i128;
    for candidate in &case.candidates {
        total += i128::from(candidate.start.0.abs())
            + i128::from(candidate.start.1.abs())
            + i128::from(candidate.end.0.abs())
            + i128::from(candidate.end.1.abs());
    }
    total
}

fn revalidate_locks(case: &LockCase, mode: LockMode, limits: &Limits) {
    assert_eq!(case.mode, mode, "the lock mode must survive shrinking");
    let observation = observe_locks(case);
    let stats = check_locks(case, limits, &observation)
        .unwrap_or_else(|error| panic!("shrunk {} case: {error}", mode.label()));
    if matches!(mode, LockMode::MixedBlocks | LockMode::MultiRoleBlock) {
        assert!(stats.blocks > 0, "the required locked block vanished");
    }
    if mode == LockMode::CrossToolBlock {
        assert!(
            stats.cross_partition_blocks > 0,
            "the required cross-tool tag vanished"
        );
    }
}

fn lock_complexity(case: &LockCase) -> i128 {
    let mut total = case.specs.len() as i128;
    for spec in &case.specs {
        total += i128::from(spec.start.0.abs())
            + i128::from(spec.start.1.abs())
            + i128::from(spec.end.0.abs())
            + i128::from(spec.end.1.abs());
    }
    total
}

fn revalidate_groups(case: &GroupCase, mode: GroupMode, limits: &Limits) {
    assert_eq!(case.mode, mode, "the group mode must survive shrinking");
    let observation = observe_groups(case);
    let stats = check_groups(case, limits, &observation)
        .unwrap_or_else(|error| panic!("shrunk {} case: {error}", mode.label()));
    match mode {
        GroupMode::CrossRoleLock | GroupMode::CrossToolLock => assert!(
            stats.cross_partition_tags > 0 && stats.split_tags == 0,
            "the required cross-partition tag vanished or was split (ADR-0062)"
        ),
        GroupMode::WildcardMix => assert!(
            case.specs
                .iter()
                .any(|spec| !role_group_is_documented(&spec.role)),
            "the required wildcard role vanished"
        ),
        GroupMode::Empty | GroupMode::Ordinary => {}
    }
}

fn group_complexity(case: &GroupCase) -> i128 {
    let mut total = case.specs.len() as i128;
    for spec in &case.specs {
        total += i128::from(spec.start.0.abs())
            + i128::from(spec.start.1.abs())
            + i128::from(spec.end.0.abs())
            + i128::from(spec.end.1.abs());
    }
    total
}

fn shrink_probe(limits: &Limits) {
    for mode in NEAREST_MODES {
        let strategy = nearest_strategy(limits, vec![mode]);
        // The empty mode ignores the generated payload entirely, so it has
        // nothing to reduce; every other mode must show a real reduction.
        let reducible = mode != NearestMode::Empty;
        shrink_transitions(
            &strategy,
            &format!("nearest/{}", mode.label()),
            reducible,
            nearest_complexity,
            |case| revalidate_nearest(case, mode, limits),
        );
    }
    for mode in LOCK_MODES {
        let strategy = lock_strategy(limits, vec![mode]);
        let reducible = !matches!(mode, LockMode::Empty | LockMode::Singleton);
        shrink_transitions(
            &strategy,
            &format!("locks/{}", mode.label()),
            reducible,
            lock_complexity,
            |case| revalidate_locks(case, mode, limits),
        );
    }
    for mode in GROUP_MODES {
        let strategy = group_strategy(limits, vec![mode]);
        let reducible = mode != GroupMode::Empty;
        shrink_transitions(
            &strategy,
            &format!("groups/{}", mode.label()),
            reducible,
            group_complexity,
            |case| revalidate_groups(case, mode, limits),
        );
    }

    // The semantic selectors never shrink: a shrinking mode could switch a
    // witness's class mid-search.
    let mut runner = TestRunner::deterministic();
    let mut tree = prop::sample::select(NEAREST_MODES.to_vec())
        .no_shrink()
        .new_tree(&mut runner)
        .expect("nearest mode tree");
    let before = tree.current();
    // test-quality: determinism pin -- a persistent semantic mode must not
    // shrink (protected surface: nearest class selection).
    assert!(!tree.simplify(), "the nearest mode selector simplified");
    assert!(!tree.complicate(), "the nearest mode selector complicated");
    assert_eq!(tree.current(), before);

    let mut tree = prop::sample::select(LOCK_MODES.to_vec())
        .no_shrink()
        .new_tree(&mut runner)
        .expect("lock mode tree");
    let before = tree.current();
    // test-quality: determinism pin -- a persistent semantic mode must not
    // shrink (protected surface: lock pattern selection).
    assert!(!tree.simplify(), "the lock mode selector simplified");
    assert!(!tree.complicate(), "the lock mode selector complicated");
    assert_eq!(tree.current(), before);

    let mut tree = prop::sample::select(GROUP_MODES.to_vec())
        .no_shrink()
        .new_tree(&mut runner)
        .expect("group mode tree");
    let before = tree.current();
    // test-quality: determinism pin -- a persistent semantic mode must not
    // shrink (protected surface: role/tool mix selection).
    assert!(!tree.simplify(), "the group mode selector simplified");
    assert!(!tree.complicate(), "the group mode selector complicated");
    assert_eq!(tree.current(), before);
}

// ─────────────────────────────────────────────────────────────────────────────
// M-PATH-NEAREST property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_path_nearest() {
    let run = PropertyRun::new(PROPERTY_ID_NEAREST, PROPERTY_TEST_NEAREST)
        .expect("M-PATH-NEAREST context");
    let limits = live_limits(&run, &NEAREST_BOUND_KEYS, "max_candidates");
    let strategy = nearest_strategy(&limits, NEAREST_MODES.to_vec());

    let calls = Cell::new(0_u64);
    let generated_candidates = Cell::new(0_u64);
    let generated_permutations = Cell::new(0_u64);
    let generated_ties = Cell::new(0_u64);
    let generated_empty = Cell::new(0_u64);
    let generated_singleton = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 5] = Default::default();
    let witness_ties = Cell::new(0_u64);
    let witness_window = Cell::new(0_u64);
    let witness_separated = Cell::new(0_u64);
    let witness_empty = Cell::new(0_u64);
    let witness_singleton = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);

        // Deterministic positives, constructed inside the callback so the
        // required counters never depend on the sampler.
        let witnesses = nearest_witness_observations(&limits);
        witness_ties.set(witness_ties.get() + witnesses.exact_ties);
        witness_window.set(witness_window.get() + witnesses.window_wins);
        witness_separated.set(witness_separated.get() + witnesses.separated_first);
        witness_empty.set(witness_empty.get() + witnesses.empty_cases);
        witness_singleton.set(witness_singleton.get() + witnesses.singleton_cases);

        let observation = observe_nearest(&case);
        let stats = match check_nearest(&case, &limits, &observation) {
            Ok(stats) => stats,
            Err(error) => return fail(format!("{} case: {error}", case.mode.label())),
        };
        generated_permutations.set(generated_permutations.get() + stats.permutations);
        generated_candidates.set(generated_candidates.get() + stats.candidates);
        generated_ties.set(generated_ties.get() + stats.tie_minima);
        match case.candidates.len() {
            0 => generated_empty.set(generated_empty.get() + 1),
            1 => generated_singleton.set(generated_singleton.get() + 1),
            _ => {}
        }
        let slot = NEAREST_MODES
            .iter()
            .position(|mode| *mode == case.mode)
            .expect("generated mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        Ok(())
    })
    .expect("M-PATH-NEAREST bijections, local tie choice and direction");

    assert!(calls.get() > 0, "the property callback never ran");
    for (name, counter) in [
        ("generated candidates", &generated_candidates),
        ("generated index permutations", &generated_permutations),
        ("generated exact ties", &generated_ties),
        ("generated empty cases", &generated_empty),
        ("generated singleton cases", &generated_singleton),
        ("witness exact ties", &witness_ties),
        ("witness tie-window wins", &witness_window),
        ("witness separated first choices", &witness_separated),
        ("witness empty cases", &witness_empty),
        ("witness singleton cases", &witness_singleton),
    ] {
        assert!(counter.get() > 0, "the {name} counter was never observed");
    }
    for (mode, counter) in NEAREST_MODES.iter().zip(&per_mode) {
        assert!(
            counter.get() > 0,
            "the {} mode was never generated",
            mode.label()
        );
    }
    println!(
        "M-PATH-NEAREST accepted_calls={} permutations={} ties={} empty={} singleton={} \
         modes=[{}, {}, {}, {}, {}] witness_ties={} witness_window={} witness_separated={}",
        calls.get(),
        generated_permutations.get(),
        generated_ties.get(),
        generated_empty.get(),
        generated_singleton.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        per_mode[4].get(),
        witness_ties.get(),
        witness_window.get(),
        witness_separated.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-PATH-LOCKS property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_path_locks() {
    let run =
        PropertyRun::new(PROPERTY_ID_LOCKS, PROPERTY_TEST_LOCKS).expect("M-PATH-LOCKS context");
    let limits = live_limits(&run, &ENTITY_BOUND_KEYS, "max_entities");
    let strategy = lock_strategy(&limits, LOCK_MODES.to_vec());

    let calls = Cell::new(0_u64);
    let generated_candidates = Cell::new(0_u64);
    let generated_permutations = Cell::new(0_u64);
    let generated_blocks = Cell::new(0_u64);
    let generated_empty = Cell::new(0_u64);
    let generated_singletons = Cell::new(0_u64);
    let generated_separated = Cell::new(0_u64);
    let generated_cross_tool = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 7] = Default::default();
    let witness_units = Cell::new(0_u64);
    let witness_moved = Cell::new(0_u64);
    let witness_endpoint = Cell::new(0_u64);
    let witness_separated = Cell::new(0_u64);
    let witness_cross_tool = Cell::new(0_u64);
    let witness_empty = Cell::new(0_u64);
    let witness_singleton = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);

        let witnesses = lock_witness_observations(&limits);
        witness_units.set(witness_units.get() + witnesses.block_units);
        witness_moved.set(witness_moved.get() + witnesses.moved_blocks);
        witness_endpoint.set(witness_endpoint.get() + witnesses.endpoint_wins);
        witness_separated.set(witness_separated.get() + witnesses.separated_tags);
        witness_cross_tool.set(witness_cross_tool.get() + witnesses.cross_tool_merges);
        witness_empty.set(witness_empty.get() + witnesses.empty_cases);
        witness_singleton.set(witness_singleton.get() + witnesses.singleton_cases);

        let observation = observe_locks(&case);
        let stats = match check_locks(&case, &limits, &observation) {
            Ok(stats) => stats,
            Err(error) => return fail(format!("{} case: {error}", case.mode.label())),
        };
        generated_permutations.set(generated_permutations.get() + stats.permutations);
        generated_candidates.set(generated_candidates.get() + stats.candidates);
        generated_blocks.set(generated_blocks.get() + stats.blocks);
        generated_empty.set(generated_empty.get() + stats.empty);
        generated_singletons.set(generated_singletons.get() + stats.singleton);
        generated_separated.set(generated_separated.get() + stats.separated_tags);
        generated_cross_tool.set(generated_cross_tool.get() + stats.cross_partition_blocks);
        let slot = LOCK_MODES
            .iter()
            .position(|mode| *mode == case.mode)
            .expect("generated mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        Ok(())
    })
    .expect("M-PATH-LOCKS coalescing, block preservation and direction");

    assert!(calls.get() > 0, "the property callback never ran");
    for (name, counter) in [
        ("generated candidates", &generated_candidates),
        ("generated permutations", &generated_permutations),
        ("generated locked blocks", &generated_blocks),
        ("generated empty cases", &generated_empty),
        ("generated singleton cases", &generated_singletons),
        ("witness block units", &witness_units),
        ("witness moved blocks", &witness_moved),
        ("witness endpoint wins", &witness_endpoint),
        ("witness separated tags", &witness_separated),
        (
            "witness cross-tool blocks (lock property)",
            &witness_cross_tool,
        ),
        ("witness empty cases", &witness_empty),
        ("witness singleton cases", &witness_singleton),
    ] {
        assert!(counter.get() > 0, "the {name} counter was never observed");
    }
    for (mode, counter) in LOCK_MODES.iter().zip(&per_mode) {
        assert!(
            counter.get() > 0,
            "the {} mode was never generated",
            mode.label()
        );
    }
    println!(
        "M-PATH-LOCKS accepted_calls={} permutations={} blocks={} empty={} singleton={} \
         separated={} cross_tool={} witness_units={} witness_moved={} witness_endpoint={}",
        calls.get(),
        generated_permutations.get(),
        generated_blocks.get(),
        generated_empty.get(),
        generated_singletons.get(),
        generated_separated.get(),
        generated_cross_tool.get(),
        witness_units.get(),
        witness_moved.get(),
        witness_endpoint.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// M-PATH-GROUPS property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_path_groups() {
    let run =
        PropertyRun::new(PROPERTY_ID_GROUPS, PROPERTY_TEST_GROUPS).expect("M-PATH-GROUPS context");
    let limits = live_limits(&run, &ENTITY_BOUND_KEYS, "max_entities");
    let strategy = group_strategy(&limits, GROUP_MODES.to_vec());

    let calls = Cell::new(0_u64);
    let generated_permutations = Cell::new(0_u64);
    let generated_tool_blocks = Cell::new(0_u64);
    let generated_group_segments = Cell::new(0_u64);
    let generated_wall_segments = Cell::new(0_u64);
    let generated_locked_runs = Cell::new(0_u64);
    let generated_tool_changes = Cell::new(0_u64);
    let generated_cross_partition = Cell::new(0_u64);
    let generated_split = Cell::new(0_u64);
    let generated_empty = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 5] = Default::default();
    let witness_wildcard = Cell::new(0_u64);
    let witness_support = Cell::new(0_u64);
    let witness_walls = Cell::new(0_u64);
    let witness_skirt = Cell::new(0_u64);
    let witness_tools = Cell::new(0_u64);
    let witness_changes = Cell::new(0_u64);
    let witness_cross_role = Cell::new(0_u64);
    let witness_cross_tool = Cell::new(0_u64);
    let witness_empty = Cell::new(0_u64);
    let witness_singleton = Cell::new(0_u64);

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);

        let witnesses = group_witness_observations(&limits);
        witness_wildcard.set(witness_wildcard.get() + witnesses.wildcard_last);
        witness_support.set(witness_support.get() + witnesses.support_after_infill);
        witness_walls.set(witness_walls.get() + witnesses.staged_walls);
        witness_skirt.set(witness_skirt.get() + witnesses.skirt_first);
        witness_tools.set(witness_tools.get() + witnesses.tool_clusters);
        witness_changes.set(witness_changes.get() + witnesses.tool_change_records);
        witness_cross_role.set(witness_cross_role.get() + witnesses.cross_role_fragments);
        witness_cross_tool.set(witness_cross_tool.get() + witnesses.cross_tool_fragments);
        witness_empty.set(witness_empty.get() + witnesses.empty_cases);
        witness_singleton.set(witness_singleton.get() + witnesses.singleton_cases);

        let observation = observe_groups(&case);
        let stats = match check_groups(&case, &limits, &observation) {
            Ok(stats) => stats,
            Err(error) => return fail(format!("{} case: {error}", case.mode.label())),
        };
        generated_permutations.set(generated_permutations.get() + stats.permutations);
        generated_tool_blocks.set(generated_tool_blocks.get() + stats.tool_blocks);
        generated_group_segments.set(generated_group_segments.get() + stats.group_segments);
        generated_wall_segments.set(generated_wall_segments.get() + stats.wall_segments);
        generated_locked_runs.set(generated_locked_runs.get() + stats.locked_runs);
        generated_tool_changes.set(generated_tool_changes.get() + stats.tool_changes);
        generated_cross_partition.set(generated_cross_partition.get() + stats.cross_partition_tags);
        generated_split.set(generated_split.get() + stats.split_tags);
        generated_empty.set(generated_empty.get() + stats.empty);
        let slot = GROUP_MODES
            .iter()
            .position(|mode| *mode == case.mode)
            .expect("generated mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        Ok(())
    })
    .expect("M-PATH-GROUPS tool clusters, staged walls and role groups");

    assert!(calls.get() > 0, "the property callback never ran");
    for (name, counter) in [
        ("generated permutations", &generated_permutations),
        ("generated tool blocks", &generated_tool_blocks),
        ("generated group segments", &generated_group_segments),
        ("generated wall segments", &generated_wall_segments),
        ("generated locked runs", &generated_locked_runs),
        ("generated tool changes", &generated_tool_changes),
        ("generated cross-partition tags", &generated_cross_partition),
        ("generated empty cases", &generated_empty),
        ("witness wildcard placement", &witness_wildcard),
        ("witness support-after-infill", &witness_support),
        ("witness staged walls", &witness_walls),
        ("witness skirt prefix", &witness_skirt),
        ("witness tool clusters", &witness_tools),
        ("witness tool changes", &witness_changes),
        ("witness cross-role blocks", &witness_cross_role),
        ("witness cross-tool blocks", &witness_cross_tool),
        ("witness empty cases", &witness_empty),
        ("witness singleton cases", &witness_singleton),
    ] {
        assert!(counter.get() > 0, "the {name} counter was never observed");
    }
    assert_eq!(
        generated_split.get(),
        0,
        "ADR-0062: no generated tag may be split across output positions"
    );
    for (mode, counter) in GROUP_MODES.iter().zip(&per_mode) {
        assert!(
            counter.get() > 0,
            "the {} mode was never generated",
            mode.label()
        );
    }
    println!(
        "M-PATH-GROUPS accepted_calls={} permutations={} tool_blocks={} group_segments={} \
         wall_segments={} tool_changes={} cross_partition={} split={} empty={} \
         witness_wildcard={} witness_support={} witness_cross_role={} witness_cross_tool={}",
        calls.get(),
        generated_permutations.get(),
        generated_tool_blocks.get(),
        generated_group_segments.get(),
        generated_wall_segments.get(),
        generated_tool_changes.get(),
        generated_cross_partition.get(),
        generated_split.get(),
        generated_empty.get(),
        witness_wildcard.get(),
        witness_support.get(),
        witness_cross_role.get(),
        witness_cross_tool.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Combined deterministic witness / control / shrink test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn packet15_witnesses_controls_and_shrinking() {
    let limits = checked_limits(&probe_bound_map(), "max_entities").expect("probe bounds");
    let nearest_limits =
        checked_limits(&probe_bound_map(), "max_candidates").expect("nearest probe bounds");

    // The probe map must mirror the registry records exactly.
    let probe = probe_bound_map();
    assert_eq!(probe["max_candidates"], 16, "M-PATH-NEAREST max_candidates");
    assert_eq!(
        probe["max_entities"], 16,
        "M-PATH-LOCKS/GROUPS max_entities"
    );
    assert_eq!(probe["max_coord_quanta"], 8_192, "max_coord_quanta");
    for key in BOUND_KEYS {
        assert!(probe.contains_key(key), "the probe map lost `{key}`");
    }

    // ── Every required positive witness, through its property's checker ────
    let nearest = nearest_witness_observations(&nearest_limits);
    assert!(nearest.exact_ties > 0, "exact-tie witness unobserved");
    assert!(nearest.window_wins > 0, "tie-window witness unobserved");
    assert!(
        nearest.separated_first > 0,
        "separated-first witness unobserved"
    );
    assert!(nearest.empty_cases > 0, "empty witness unobserved");
    assert!(nearest.singleton_cases > 0, "singleton witness unobserved");

    let locks = lock_witness_observations(&limits);
    assert!(locks.block_units > 0, "locked block witness unobserved");
    assert!(locks.moved_blocks > 0, "moved-block witness unobserved");
    assert!(locks.endpoint_wins > 0, "last-endpoint witness unobserved");
    assert!(locks.separated_tags > 0, "separated-tag witness unobserved");
    assert!(
        locks.cross_tool_merges > 0,
        "cross-tool block witness unobserved"
    );
    assert!(locks.empty_cases > 0, "lock empty witness unobserved");
    assert!(
        locks.singleton_cases > 0,
        "lock singleton witness unobserved"
    );

    let groups = group_witness_observations(&limits);
    assert!(groups.wildcard_last > 0, "wildcard witness unobserved");
    assert!(
        groups.support_after_infill > 0,
        "support ordering unobserved"
    );
    assert!(groups.staged_walls > 0, "staged wall witness unobserved");
    assert!(groups.skirt_first > 0, "skirt prefix witness unobserved");
    assert!(groups.tool_clusters > 0, "tool cluster witness unobserved");
    assert!(
        groups.tool_change_records > 0,
        "tool change witness unobserved"
    );
    assert!(
        groups.cross_role_fragments > 0,
        "cross-role witness unobserved"
    );
    assert!(
        groups.cross_tool_fragments > 0,
        "cross-tool witness unobserved"
    );
    assert!(groups.empty_cases > 0, "group empty witness unobserved");
    assert!(
        groups.singleton_cases > 0,
        "group singleton witness unobserved"
    );

    // ── The wildcard table itself (authored, never `role_group`) ───────────
    for role in WILDCARD_ROLES {
        assert!(
            !role_group_is_documented(&role),
            "{role:?} must be characterised, not documented"
        );
        assert_eq!(
            characterized_group(&role),
            9,
            "{role:?} currently takes the `_ => 9` wildcard"
        );
    }
    assert!(role_group_is_documented(&ExtrusionRole::GapFill));
    assert_eq!(characterized_group(&ExtrusionRole::GapFill), 4);
    assert!(role_group_is_documented(&ExtrusionRole::Custom(
        "any".to_owned()
    )));
    assert_eq!(
        characterized_group(&ExtrusionRole::Custom("any".to_owned())),
        9
    );
    assert_eq!(characterized_group(&ExtrusionRole::Skirt), 0);
    assert_eq!(characterized_group(&ExtrusionRole::InnerWall), 1);
    assert_eq!(characterized_group(&ExtrusionRole::OuterWall), 2);
    assert_eq!(characterized_group(&ExtrusionRole::ThinWall), 3);
    for role in [
        ExtrusionRole::BottomSolidInfill,
        ExtrusionRole::TopSolidInfill,
        ExtrusionRole::SparseInfill,
        ExtrusionRole::BridgeInfill,
    ] {
        assert_eq!(characterized_group(&role), 4);
    }
    assert_eq!(characterized_group(&ExtrusionRole::Ironing), 5);
    assert_eq!(characterized_group(&ExtrusionRole::SupportMaterial), 6);
    assert_eq!(characterized_group(&ExtrusionRole::SupportInterface), 7);
    for role in [ExtrusionRole::WipeTower, ExtrusionRole::PrimeTower] {
        assert_eq!(characterized_group(&role), 8);
    }

    // ── Mutation controls over the same checkers ───────────────────────────
    nearest_controls(&nearest_limits);
    lock_controls(&limits);
    group_controls(&limits);
    invalid_profile_matrix();

    // ── Real ValueTree simplify/complicate, revalidated at every state ─────
    shrink_probe(&limits);

    println!(
        "packet15_witnesses_controls_and_shrinking: nearest={nearest:?} locks={locks:?} \
         groups={groups:?} controls=rejected shrinking=green"
    );
}
