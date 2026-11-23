//! Opt-in property tests for this module's private tree-planner kernels.
//!
//! Charters (`docs/spec_packets/proptest-openrscad_14_pbt-support-modules`):
//!
//! * `M-TREE-SMOOTH` — private `smooth_nodes` over private `NodeArena` /
//!   `LayerRecord` scenes with valid parent/child/active links: generated
//!   straight (equally spaced), bent and short chains, plus deterministic
//!   witnesses for a nonminimal bent chain, a merge node (two `parents`), a
//!   `Polygon`-typed child that must not become a fixed head, the signed
//!   width-derived flag threshold and a shorter-than-three-node chain. Every
//!   node reference is inspected before and after.
//! * `M-TREE-ROLES` — `carve_emitted_regions`,
//!   `carve_emitted_regions_with_bbox_gate`, the serial `build_roles` seam and
//!   the production `build_roles_batched` kernel on bounded, non-simplifying,
//!   axis-aligned rectangle scenes, checked against an independent
//!   rectangle-cell labelling.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! The kernels are private (or hidden test seams), so the properties live here
//! as a crate-internal unit-test module; no production visibility is widened.
//!
//! # `M-TREE-SMOOTH` independent bounds
//!
//! `smooth_nodes` relaxes every interior node of a chain with the Jacobi
//! three-point mean `(p[i-1] + p[i] + p[i+1]) / 3` for 100 passes while both
//! endpoints are never written, then rounds once on commit (`DEV-143`: f64
//! relaxation, one `round()`; no bit-exact upstream truncation is asserted and
//! no second 100-pass copy is used as an expected output).
//!
//! * **Hull.** By induction over the passes every exact interior value is a
//!   convex combination of the chain's original positions, so per axis it
//!   lies in `[min_original, max_original]`. The f64 evaluation error is
//!   bounded: coordinates are at most `2^20` units, each pass does two
//!   additions and one division (relative error `<= 2^-53` each), so after
//!   100 passes the absolute error is far below `0.5` unit. The single commit
//!   `round()` moves a value by at most `0.5`; because `min_original` and
//!   `max_original` are integers and the drift is `< 0.5`, the rounded value
//!   stays inside `[min_original, max_original]`. The rounding allowance is
//!   therefore **zero** units on top of the integer hull (the generic
//!   "at most one unit" envelope collapses onto it). Radii are convex
//!   combinations of f32 originals and are checked against their
//!   `[min, max]` the same way (the f64→f32 cast is monotone).
//! * **Committed movement.** It is `round((q[i+1] - q[i-1]) / 2)` of the final
//!   pass input `q`, whose entries are inside the same hull, so
//!   `|2 * movement| <= (max - min) + 1` per axis.
//! * **Straight witness.** For `p[k] = p[0] + k * s` with integer `s`, the
//!   sum `p[k-1] + p[k] + p[k+1] = 3 * p[k]` is exact in f64 and the division
//!   by three of an exact multiple of three is exact, so every pass is an
//!   exact fixed point: committed position `== p[k]`, movement `== s`.
//!   Linear radii in sixteenths of a millimetre are exact the same way.
//! * **Flag.** `need_extra_wall` is set on an interior node when it has more
//!   than one parent, or a **signed** movement component exceeds the
//!   width-derived threshold `width_units / 2` (canonical
//!   `scale_(support_line_width / 2)`; a flag trigger, not a cap on movement
//!   or `max_move_dist`, neither of which is asserted), or the tall-branch
//!   rule fires. The generated domain keeps every chain below the canonical
//!   100 mm tall-branch height, so that disjunct is inactive. A node
//!   bracketed by two flagged nodes is then flagged; for the bottom-up
//!   in-place pass this equals the simultaneous rule over first-loop flags
//!   (if `g[i-1]` was set only by interpolation then `f[i]` was already set).
//!
//! # `M-TREE-ROLES` independent model
//!
//! Scenes are axis-aligned integer rectangles. Collision is one or two bars
//! whose y-extent spans every input region (`bar_y`), so each region minus
//! collision is a set of side-by-side rectangles; the generator guarantees a
//! **unique** widest survivor (the canonical largest part). The expected sets
//! are: per region its survivor; body = union(body survivors) minus every
//! interface union; interfaces = their unions; the batched own-territory mode
//! intersects every role with the territory rectangle. Observations are
//! compared on the open cells of the partition at **every** input and output
//! X/Y edge, by an exact integer crossing-parity test at doubled-coordinate
//! cell centres (never on an edge), never by vertex order. An optional
//! transposition swaps X and Y of the whole scene before the SUT sees it.
//! Every scene keeps `avg_node_per_layer <= 200`, for which
//! `role_simplify_tolerance` returns `None` for every role (measured in
//! `tree_planner_controls`), so no Douglas–Peucker pass runs. Segments are
//! empty: distinct-point skeleton segments are not assumed to create roles.
//! The larger production routing/aggregation is outside this kernel claim.
//!
//! # Literal review
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. Watched types (a `pub` struct
//! with at least five named fields under `crates/*/src`) are **not**
//! constructed here: `Point3WithWidth` and `ExtrusionPath3D` never appear
//! (all segment lists are empty), and the literals that do appear are
//! `Point2` (2 fields), `Polygon` (1), `ExPolygon` (2) and
//! `SupportPlanRoleRegion` (2) from `crates/slicer-ir/src/slice_ir.rs`, plus
//! the module-private `RoleBuildInput` and `LayerRecord` (not under
//! `crates/*/src`). Every local struct is private to this module.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! controls are mutation controls over the SAME checkers the properties use,
//! each asserting the specific failure class after the unmodified observation
//! is accepted; the properties require positive per-subcase counters from
//! deterministic witnesses built inside their callbacks. `clip_polygons` is
//! a pure native call (no SDK host state), so the controls need no
//! `CaseGuard`.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{mm_to_units, units_to_mm, ExPolygon, Point2, Polygon};
use slicer_ir::{SupportPlanRole, SupportPlanRoleRegion};
use slicer_sdk::test_support::property_testing::domains::DomainError;
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::{
    build_roles, build_roles_batched, carve_emitted_regions, carve_emitted_regions_with_bbox_gate,
    expolygons_bbox, role_simplify_tolerance, smooth_nodes, LayerRecord, NodeArena, NodeId,
    PlannedSupportNode, RoleBuildInput, TreeNodeType,
};

const SMOOTH_ID: &str = "M-TREE-SMOOTH";
const SMOOTH_TEST: &str = "property_tests::proptest_tree_smoothing_graph";
const SMOOTH_BOUND_KEYS: [&str; 4] = [
    "max_abs_units",
    "max_nodes",
    "max_layers",
    "max_width_units",
];

const ROLES_ID: &str = "M-TREE-ROLES";
const ROLES_TEST: &str = "property_tests::proptest_tree_carve_and_roles";
const ROLES_BOUND_KEYS: [&str; 4] = [
    "max_abs_units",
    "max_regions",
    "max_cells",
    "max_batch_items",
];

/// 1 mm in canonical units (1 unit = 100 nm).
const MM: i64 = 10_000;
/// Step ceiling for the shrink probes' transition loops.
const MAX_SHRINK_STEPS: usize = 20_000;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

fn as_i64(bounds: &BTreeMap<String, u64>, key: &str) -> Result<i64, DomainError> {
    i64::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)
}

fn as_usize(bounds: &BTreeMap<String, u64>, key: &str) -> Result<usize, DomainError> {
    let value =
        u32::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)?;
    usize::try_from(value).map_err(|_| DomainError::ArithmeticOverflow)
}

fn live_bounds(run: &PropertyRun, keys: &[&str]) -> BTreeMap<String, u64> {
    let mut bounds = BTreeMap::new();
    for key in keys {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("tree planner bound `{key}`: {error}"));
        bounds.insert((*key).to_owned(), value);
    }
    bounds
}

/// Inclusive `lo..=hi` pick; an empty range is a generator bug and panics
/// (a failure, never a rejection).
fn pick(raw: u32, lo: i64, hi: i64) -> i64 {
    assert!(lo <= hi, "generator range {lo}..={hi} is empty");
    lo + i64::from(raw) % (hi - lo + 1)
}

fn pick_usize(raw: u32, lo: usize, hi: usize) -> usize {
    usize::try_from(pick(raw, lo as i64, hi as i64)).expect("nonnegative pick")
}

/// Sequential reader over a fixed-length raw slice.
struct Raw<'a> {
    values: &'a [u32],
    at: usize,
}

impl<'a> Raw<'a> {
    fn new(values: &'a [u32]) -> Self {
        Self { values, at: 0 }
    }

    fn next(&mut self) -> u32 {
        let value = *self
            .values
            .get(self.at)
            .unwrap_or_else(|| panic!("generator read past raw length {}", self.values.len()));
        self.at += 1;
        value
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// M-TREE-SMOOTH
// ═════════════════════════════════════════════════════════════════════════════

/// Height of every fixture node; `max_layers * 0.25 mm` stays below the
/// canonical 100 mm tall-branch threshold.
const NODE_HEIGHT_MM: f32 = 0.25;
/// Canonical `thresh_tall_branch` (100 mm), restated independently; every
/// fixture chain is kept strictly below it.
const TALL_BRANCH_MM: f32 = 100.0;
/// Upper bound on `max_layers`: 256 * 0.25 mm = 64 mm < 100 mm.
const MAX_LAYERS_CEILING: usize = 256;
/// Coordinate ceiling that keeps the f64 drift argument comfortably valid.
const SMOOTH_ABS_CEILING: i64 = 1_000_000;
/// The witnesses use coordinates up to 2 mm.
const SMOOTH_WITNESS_ABS: i64 = 2 * MM;
/// The merge witness has six nodes on five layers.
const SMOOTH_WITNESS_NODES: usize = 6;
const SMOOTH_WITNESS_LAYERS: usize = 5;
/// 0.4 mm witness support line width.
const WITNESS_WIDTH_UNITS: i64 = 4_000;
/// 2 mm: the largest generated support line width.
const MAX_WIDTH_CEILING: i64 = 20_000;
/// Generated scenes hold at most this many independent chains.
const MAX_CHAINS: usize = 4;
const CHAIN_HEADER_RAW: usize = 8;
const NODE_RAW: usize = 6;

#[derive(Debug, Clone, Copy)]
struct SmoothLimits {
    max_abs_units: i64,
    max_nodes: usize,
    max_layers: usize,
    max_width_units: i64,
}

fn checked_smooth_limits(bounds: &BTreeMap<String, u64>) -> Result<SmoothLimits, DomainError> {
    let limits = SmoothLimits {
        max_abs_units: as_i64(bounds, "max_abs_units")?,
        max_nodes: as_usize(bounds, "max_nodes")?,
        max_layers: as_usize(bounds, "max_layers")?,
        max_width_units: as_i64(bounds, "max_width_units")?,
    };
    if !(SMOOTH_WITNESS_ABS..=SMOOTH_ABS_CEILING).contains(&limits.max_abs_units) {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units must be in {SMOOTH_WITNESS_ABS}..={SMOOTH_ABS_CEILING}"
        )));
    }
    if !(SMOOTH_WITNESS_LAYERS..=MAX_LAYERS_CEILING).contains(&limits.max_layers) {
        return Err(DomainError::InvalidBounds(format!(
            "max_layers must be in {SMOOTH_WITNESS_LAYERS}..={MAX_LAYERS_CEILING}: the witnesses \
             need five layers and the tall-branch rule must stay inactive"
        )));
    }
    if !(SMOOTH_WITNESS_NODES..=MAX_CHAINS * MAX_LAYERS_CEILING).contains(&limits.max_nodes) {
        return Err(DomainError::InvalidBounds(format!(
            "max_nodes must be in {SMOOTH_WITNESS_NODES}..={}",
            MAX_CHAINS * MAX_LAYERS_CEILING
        )));
    }
    if limits.max_width_units % 2 != 0
        || !(WITNESS_WIDTH_UNITS..=MAX_WIDTH_CEILING).contains(&limits.max_width_units)
    {
        return Err(DomainError::InvalidBounds(format!(
            "max_width_units must be even and in {WITNESS_WIDTH_UNITS}..={MAX_WIDTH_CEILING}"
        )));
    }
    Ok(limits)
}

fn smooth_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 20_000),
        ("max_nodes".to_owned(), 6),
        ("max_layers".to_owned(), 5),
        ("max_width_units".to_owned(), 4_000),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmoothMode {
    Straight,
    Bent,
    Short,
    Mixed,
}

const SMOOTH_MODES: [SmoothMode; 4] = [
    SmoothMode::Straight,
    SmoothMode::Bent,
    SmoothMode::Short,
    SmoothMode::Mixed,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainKind {
    Straight,
    Bent,
    Short,
}

/// One generated chain, bottom node first.
#[derive(Debug, Clone, PartialEq)]
struct ChainGen {
    kind: ChainKind,
    base_level: usize,
    /// Canonical units, bottom → top.
    positions: Vec<(i64, i64)>,
    /// Radii in sixteenths of a millimetre (exact in f32).
    radii_q: Vec<i64>,
    /// Pre-smoothing `movement` of every node.
    movements: Vec<(i64, i64)>,
    /// Pre-smoothing `need_extra_wall`; interior nodes of long chains start false.
    flags: Vec<bool>,
    /// The straight chain's per-layer step.
    step: Option<(i64, i64)>,
}

#[derive(Debug, Clone, PartialEq)]
struct SmoothCase {
    mode: SmoothMode,
    width_units: i64,
    layers: usize,
    chains: Vec<ChainGen>,
}

/// A chain as `smooth_nodes` walks it: optional fixed head, then parent-ward.
struct ChainDecl {
    ids: Vec<NodeId>,
    straight: Option<Point2>,
}

struct SmoothScene {
    label: &'static str,
    arena: NodeArena,
    records: Vec<LayerRecord>,
    /// In the order the bottom-up walk processes them.
    chains: Vec<ChainDecl>,
    width_units: i64,
}

fn clone_arena(arena: &NodeArena) -> NodeArena {
    NodeArena {
        nodes: arena.nodes.clone(),
    }
}

fn radius_mm(q: i64) -> f32 {
    q as f32 / 16.0
}

/// Layer records pushed top-first, as `drop_nodes` pushes them.
fn records_top_first(by_level: Vec<Vec<NodeId>>) -> Vec<LayerRecord> {
    by_level
        .into_iter()
        .enumerate()
        .rev()
        .map(|(level, active)| LayerRecord {
            layer_rev: level,
            active,
            edges: Vec::new(),
        })
        .collect()
}

fn scene_from_case(label: &'static str, case: &SmoothCase) -> SmoothScene {
    let mut arena = NodeArena::default();
    let mut by_level: Vec<Vec<NodeId>> = vec![Vec::new(); case.layers];
    let mut chains = Vec::new();
    for chain in &case.chains {
        let n = chain.positions.len();
        let mut ids = vec![NodeId(usize::MAX); n];
        let mut parent = None;
        for k in (0..n).rev() {
            let level = chain.base_level + k;
            let (x, y) = chain.positions[k];
            let to_top = n - 1 - k;
            let id = arena.create_node(
                Point2 { x, y },
                to_top as i32,
                level,
                0,
                true,
                parent,
                level as f32 * NODE_HEIGHT_MM,
                NODE_HEIGHT_MM,
                to_top as f32 * NODE_HEIGHT_MM,
                radius_mm(chain.radii_q[k]),
            );
            let (mx, my) = chain.movements[k];
            arena[id].movement = Point2 { x: mx, y: my };
            arena[id].need_extra_wall = chain.flags[k];
            ids[k] = id;
            parent = Some(id);
        }
        for (k, id) in ids.iter().enumerate() {
            by_level[chain.base_level + k].push(*id);
        }
        chains.push(ChainDecl {
            ids,
            straight: chain.step.map(|(x, y)| Point2 { x, y }),
        });
    }
    SmoothScene {
        label,
        arena,
        records: records_top_first(by_level),
        chains,
        width_units: case.width_units,
    }
}

fn run_smooth(scene: &SmoothScene) -> NodeArena {
    let mut arena = clone_arena(&scene.arena);
    smooth_nodes(&mut arena, &scene.records, units_to_mm(scene.width_units));
    arena
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SmoothFailure {
    /// The scene itself is outside the declared domain (generator bug).
    Fixture(String),
    /// A node references an id outside the arena.
    InvalidReference(String),
    /// A parent/child/parents/merge/type/valid link changed.
    LinkChanged(String),
    /// A pinned chain endpoint changed position, radius, movement or flag.
    EndpointMoved(String),
    /// A chain shorter than three nodes changed.
    ShortChainChanged(String),
    /// A committed interior value left the independently bounded hull.
    OutsideHull(String),
    /// A straight equally spaced chain did not stay an exact fixed point.
    StraightMismatch(String),
    /// `need_extra_wall` disagrees with the source-backed trigger.
    Flag(String),
    /// `is_processed` disagrees with interior membership.
    Processed(String),
    /// A node outside every declared chain changed.
    UntouchedChanged(String),
}

impl fmt::Display for SmoothFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(m) => write!(f, "fixture outside the domain: {m}"),
            Self::InvalidReference(m) => write!(f, "invalid reference: {m}"),
            Self::LinkChanged(m) => write!(f, "link changed: {m}"),
            Self::EndpointMoved(m) => write!(f, "endpoint moved: {m}"),
            Self::ShortChainChanged(m) => write!(f, "short chain changed: {m}"),
            Self::OutsideHull(m) => write!(f, "outside hull: {m}"),
            Self::StraightMismatch(m) => write!(f, "straight chain: {m}"),
            Self::Flag(m) => write!(f, "need_extra_wall: {m}"),
            Self::Processed(m) => write!(f, "is_processed: {m}"),
            Self::UntouchedChanged(m) => write!(f, "undeclared node changed: {m}"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct SmoothStats {
    chains_long: u64,
    chains_short: u64,
    interior: u64,
    straight_interior: u64,
    bent_interior: u64,
    short_nodes: u64,
    endpoints: u64,
    merge_flags: u64,
    flagged: u64,
}

/// A node's value as the chain walk sees it: committed by an earlier chain
/// (observed) or still at its pre-smoothing value.
fn standing_node<'a>(
    committed: &BTreeSet<NodeId>,
    before: &'a NodeArena,
    observed: &'a NodeArena,
    id: NodeId,
) -> &'a PlannedSupportNode {
    if committed.contains(&id) {
        &observed[id]
    } else {
        &before[id]
    }
}

fn node_refs(node: &PlannedSupportNode) -> Vec<NodeId> {
    node.parent
        .into_iter()
        .chain(node.child)
        .chain(node.parents.iter().copied())
        .chain(node.merged_neighbours.iter().copied())
        .collect()
}

fn same_geometry(a: &PlannedSupportNode, b: &PlannedSupportNode) -> bool {
    a.position == b.position
        && a.radius.to_bits() == b.radius.to_bits()
        && a.movement == b.movement
        && a.need_extra_wall == b.need_extra_wall
}

fn describe(id: NodeId, n: &PlannedSupportNode) -> String {
    format!(
        "node {} at ({}, {}) r={} movement=({}, {}) flag={}",
        id.0, n.position.x, n.position.y, n.radius, n.movement.x, n.movement.y, n.need_extra_wall
    )
}

/// Validate the scene against the declared domain; returns the alive set.
fn validate_smooth_scene(
    scene: &SmoothScene,
    limits: &SmoothLimits,
) -> Result<BTreeSet<NodeId>, SmoothFailure> {
    let fixture = |m: String| Err(SmoothFailure::Fixture(m));
    let before = &scene.arena;
    let len = before.nodes.len();
    if len > limits.max_nodes || scene.records.len() > limits.max_layers {
        return fixture(format!(
            "{len} nodes / {} layers exceed the bounds",
            scene.records.len()
        ));
    }
    let w = scene.width_units;
    if w <= 0 || w % 2 != 0 || w > limits.max_width_units {
        return fixture(format!("width {w} units"));
    }
    // The canonical threshold is `mm_to_units(width_mm / 2)`; the independent
    // value is `w / 2`. Require that the f32 conversion round-trips so the two
    // are the same integer.
    if mm_to_units(units_to_mm(w) / 2.0) != w / 2 {
        return fixture(format!("width {w} units does not halve exactly through mm"));
    }
    for (i, node) in before.nodes.iter().enumerate() {
        if node_refs(node).iter().any(|r| r.0 >= len) {
            return fixture(format!("node {i} has an out-of-range reference"));
        }
        if node.position.x.abs() > limits.max_abs_units
            || node.position.y.abs() > limits.max_abs_units
        {
            return fixture(format!("node {i} exceeds max_abs_units"));
        }
        if !(node.radius.is_finite() && node.radius > 0.0) {
            return fixture(format!("node {i} radius {}", node.radius));
        }
    }
    let mut alive = BTreeSet::new();
    for record in &scene.records {
        for id in record
            .active
            .iter()
            .chain(record.edges.iter().flat_map(|e| [&e.0, &e.1]))
        {
            if id.0 >= len {
                return fixture(format!("record references {} of {len}", id.0));
            }
        }
        alive.extend(record.active.iter().copied());
    }
    let mut interiors = BTreeSet::new();
    for chain in &scene.chains {
        let ids = &chain.ids;
        let Some(&last_id) = ids.last() else {
            return fixture("empty chain declaration".to_owned());
        };
        let distinct: BTreeSet<NodeId> = ids.iter().copied().collect();
        if distinct.len() != ids.len() || !ids.iter().all(|id| alive.contains(id)) {
            return fixture("chain ids must be distinct alive nodes".to_owned());
        }
        if before[last_id].parent.is_some() {
            return fixture("a chain's top node must have no parent".to_owned());
        }
        for pair in ids.windows(2).skip(1) {
            if before[pair[0]].parent != Some(pair[1]) {
                return fixture("chain nodes after the head must follow `parent`".to_owned());
            }
        }
        let start = if ids.len() >= 2 && before[ids[0]].parent != Some(ids[1]) {
            // Declared fixed head: the start's child, never a Polygon node.
            if before[ids[1]].child != Some(ids[0]) || before[ids[0]].type_ == TreeNodeType::Polygon
            {
                return fixture("a declared head must be the start's Circle child".to_owned());
            }
            ids[1]
        } else {
            ids[0]
        };
        if start == ids[0] {
            if let Some(head) = before[start].child {
                if alive.contains(&head) && before[head].type_ != TreeNodeType::Polygon {
                    return fixture(
                        "an alive Circle child must be declared as the head".to_owned(),
                    );
                }
            }
        }
        let height: f32 = ids.iter().map(|id| before[*id].height).sum();
        if height >= TALL_BRANCH_MM {
            return fixture("a chain reaches the tall-branch height".to_owned());
        }
        if ids.len() >= 3 {
            for id in &ids[1..ids.len() - 1] {
                if before[*id].need_extra_wall || !interiors.insert(*id) {
                    return fixture(
                        "interior nodes start unflagged and are interior of one chain".to_owned(),
                    );
                }
            }
        }
        if let Some(step) = chain.straight {
            let first = before[ids[0]].position;
            let r0 = f64::from(before[ids[0]].radius);
            let dr = ids
                .get(1)
                .map_or(0.0, |id| f64::from(before[*id].radius) - r0);
            for (k, id) in ids.iter().enumerate() {
                let k64 = k as i64;
                let expected = Point2 {
                    x: first.x + k64 * step.x,
                    y: first.y + k64 * step.y,
                };
                let r = f64::from(before[*id].radius);
                if before[*id].position != expected || r - r0 != k as f64 * dr {
                    return fixture("a straight chain must be exactly equally spaced".to_owned());
                }
            }
        }
    }
    Ok(alive)
}

/// The one smoothing checker used by the property, witnesses, controls and
/// shrink probe. `observed` is the arena after `smooth_nodes`.
fn check_smoothing(
    scene: &SmoothScene,
    limits: &SmoothLimits,
    observed: &NodeArena,
) -> Result<SmoothStats, SmoothFailure> {
    let alive = validate_smooth_scene(scene, limits)?;
    let before = &scene.arena;
    let len = before.nodes.len();
    if observed.nodes.len() != len {
        return Err(SmoothFailure::InvalidReference(format!(
            "arena length changed from {len} to {}",
            observed.nodes.len()
        )));
    }
    // Every reference is range-checked before anything indexes through it.
    for (i, node) in observed.nodes.iter().enumerate() {
        if let Some(bad) = node_refs(node).into_iter().find(|r| r.0 >= len) {
            return Err(SmoothFailure::InvalidReference(format!(
                "node {i} references id {} in an arena of {len}",
                bad.0
            )));
        }
    }
    for (i, (b, o)) in before.nodes.iter().zip(&observed.nodes).enumerate() {
        if b.parent != o.parent
            || b.child != o.child
            || b.parents != o.parents
            || b.merged_neighbours != o.merged_neighbours
            || b.type_ != o.type_
            || b.valid != o.valid
        {
            return Err(SmoothFailure::LinkChanged(format!("node {i}")));
        }
    }

    let threshold = scene.width_units / 2;
    let mut committed: BTreeSet<NodeId> = BTreeSet::new();
    let mut declared: BTreeSet<NodeId> = BTreeSet::new();
    let mut stats = SmoothStats::default();
    for chain in &scene.chains {
        let ids = &chain.ids;
        declared.extend(ids.iter().copied());
        let standing = |id: NodeId| standing_node(&committed, before, observed, id);
        if ids.len() < 3 {
            for id in ids.iter().filter(|id| !committed.contains(id)) {
                if !same_geometry(&before[*id], &observed[*id]) {
                    return Err(SmoothFailure::ShortChainChanged(format!(
                        "{} became {}",
                        describe(*id, &before[*id]),
                        describe(*id, &observed[*id])
                    )));
                }
                stats.short_nodes += 1;
            }
            stats.chains_short += 1;
            continue;
        }
        stats.chains_long += 1;
        let last = ids.len() - 1;
        for e in [ids[0], ids[last]] {
            if committed.contains(&e) {
                continue;
            }
            if !same_geometry(&before[e], &observed[e]) {
                return Err(SmoothFailure::EndpointMoved(format!(
                    "{} became {}",
                    describe(e, &before[e]),
                    describe(e, &observed[e])
                )));
            }
            stats.endpoints += 1;
        }
        let xs: Vec<i64> = ids.iter().map(|id| standing(*id).position.x).collect();
        let ys: Vec<i64> = ids.iter().map(|id| standing(*id).position.y).collect();
        let rs: Vec<f32> = ids.iter().map(|id| standing(*id).radius).collect();
        let (min_x, max_x) = (*xs.iter().min().unwrap(), *xs.iter().max().unwrap());
        let (min_y, max_y) = (*ys.iter().min().unwrap(), *ys.iter().max().unwrap());
        let min_r = rs.iter().copied().fold(f32::INFINITY, f32::min);
        let max_r = rs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        // First-loop flags: interior from the source-backed trigger,
        // endpoints at their standing value.
        let first: Vec<bool> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                if i == 0 || i == last {
                    standing(*id).need_extra_wall
                } else {
                    let o = &observed[*id];
                    o.parents.len() > 1 || o.movement.x > threshold || o.movement.y > threshold
                }
            })
            .collect();
        for i in 1..last {
            let id = ids[i];
            let o = &observed[id];
            let p = o.position;
            if !(min_x..=max_x).contains(&p.x) || !(min_y..=max_y).contains(&p.y) {
                return Err(SmoothFailure::OutsideHull(format!(
                    "{} outside x [{min_x},{max_x}] y [{min_y},{max_y}]",
                    describe(id, o)
                )));
            }
            if !(min_r..=max_r).contains(&o.radius) {
                return Err(SmoothFailure::OutsideHull(format!(
                    "{} radius outside [{min_r},{max_r}]",
                    describe(id, o)
                )));
            }
            if (2 * o.movement.x).abs() > (max_x - min_x) + 1
                || (2 * o.movement.y).abs() > (max_y - min_y) + 1
            {
                return Err(SmoothFailure::OutsideHull(format!(
                    "{} movement exceeds the hull half-span",
                    describe(id, o)
                )));
            }
            if let Some(step) = chain.straight {
                let b = &before[id];
                if p != b.position || o.movement != step || o.radius.to_bits() != b.radius.to_bits()
                {
                    return Err(SmoothFailure::StraightMismatch(format!(
                        "{} expected position ({}, {}) movement ({}, {}) r={}",
                        describe(id, o),
                        b.position.x,
                        b.position.y,
                        step.x,
                        step.y,
                        b.radius
                    )));
                }
                stats.straight_interior += 1;
            } else {
                stats.bent_interior += 1;
            }
            let expected = first[i] || (first[i - 1] && first[i + 1]);
            if expected != o.need_extra_wall {
                return Err(SmoothFailure::Flag(format!(
                    "{} expected flag {expected} (parents={}, threshold={threshold})",
                    describe(id, o),
                    o.parents.len()
                )));
            }
            if o.need_extra_wall {
                stats.flagged += 1;
                if o.parents.len() > 1 {
                    stats.merge_flags += 1;
                }
            }
            stats.interior += 1;
        }
        committed.extend(ids[1..last].iter().copied());
    }
    for (i, (b, o)) in before.nodes.iter().zip(&observed.nodes).enumerate() {
        let id = NodeId(i);
        if !declared.contains(&id) && !same_geometry(b, o) {
            return Err(SmoothFailure::UntouchedChanged(describe(id, o)));
        }
        let expected = if alive.contains(&id) {
            committed.contains(&id)
        } else {
            b.is_processed
        };
        if o.is_processed != expected {
            return Err(SmoothFailure::Processed(format!(
                "node {i} is_processed={} expected {expected}",
                o.is_processed
            )));
        }
    }
    Ok(stats)
}

// ── Generator ───────────────────────────────────────────────────────────────

fn smooth_raw_len(l: &SmoothLimits) -> usize {
    3 + MAX_CHAINS * (CHAIN_HEADER_RAW + NODE_RAW * l.max_layers)
}

fn build_smooth_case(mode: SmoothMode, raw: &[u32], l: &SmoothLimits) -> SmoothCase {
    let m = l.max_abs_units;
    let mut header = Raw::new(&raw[..3]);
    let width_units = 2 * pick(header.next(), 1, l.max_width_units / 2);
    let min_len = if mode == SmoothMode::Short { 1 } else { 3 };
    let layers = pick_usize(header.next(), min_len, l.max_layers);
    let chain_count = pick_usize(header.next(), 1, MAX_CHAINS);
    let per_chain = CHAIN_HEADER_RAW + NODE_RAW * l.max_layers;
    let mut budget = l.max_nodes;
    let mut chains = Vec::new();
    for j in 0..chain_count {
        let start = 3 + j * per_chain;
        let mut r = Raw::new(&raw[start..start + per_chain]);
        let (kind_raw, len_raw, base_raw) = (r.next(), r.next(), r.next());
        let (sx_raw, sy_raw, px_raw, py_raw) = (r.next(), r.next(), r.next(), r.next());
        let r0_raw = r.next();
        let kind = match mode {
            SmoothMode::Straight => ChainKind::Straight,
            SmoothMode::Bent => ChainKind::Bent,
            SmoothMode::Short => ChainKind::Short,
            SmoothMode::Mixed if j == 0 => ChainKind::Bent,
            SmoothMode::Mixed => {
                [ChainKind::Straight, ChainKind::Bent, ChainKind::Short][kind_raw as usize % 3]
            }
        };
        let (lo_len, hi_len) = match kind {
            ChainKind::Short => (1, 2.min(layers)),
            _ => (3, layers),
        };
        let hi_len = hi_len.min(budget);
        if hi_len < lo_len {
            assert!(j > 0, "generator: the required first chain does not fit");
            break;
        }
        let len = pick_usize(len_raw, lo_len, hi_len);
        budget -= len;
        let base_level = pick_usize(base_raw, 0, layers - len);
        let mut node = |_: usize| [r.next(), r.next(), r.next(), r.next(), r.next(), r.next()];
        let node_raw: Vec<[u32; NODE_RAW]> = (0..len).map(&mut node).collect();
        let last = len - 1;
        let (positions, radii_q, step) = if kind == ChainKind::Straight {
            let span = (last as i64).max(1);
            let dmax = 2 * m / span;
            let sx = pick(sx_raw, -dmax, dmax);
            let sy = pick(sy_raw, -dmax, dmax);
            let reach = |s: i64| (last as i64) * s;
            let px = pick(px_raw, (-m).max(-m - reach(sx)), m.min(m - reach(sx)));
            let py = pick(py_raw, (-m).max(-m - reach(sy)), m.min(m - reach(sy)));
            let r0 = pick(r0_raw, 1, 64);
            let dr = pick(node_raw[0][2], 0, 4);
            let positions = (0..len as i64)
                .map(|k| (px + k * sx, py + k * sy))
                .collect();
            let radii = (0..len as i64).map(|k| r0 + k * dr).collect();
            (positions, radii, Some((sx, sy)))
        } else {
            let positions = node_raw
                .iter()
                .map(|n| (pick(n[0], -m, m), pick(n[1], -m, m)))
                .collect();
            let radii = node_raw.iter().map(|n| pick(n[2], 1, 64)).collect();
            (positions, radii, None)
        };
        let movements = node_raw
            .iter()
            .map(|n| (pick(n[3], -m, m), pick(n[4], -m, m)))
            .collect();
        let flags = node_raw
            .iter()
            .enumerate()
            .map(|(k, n)| {
                let interior = len >= 3 && k != 0 && k != last;
                !interior && n[5] % 2 == 1
            })
            .collect();
        chains.push(ChainGen {
            kind,
            base_level,
            positions,
            radii_q,
            movements,
            flags,
            step,
        });
    }
    SmoothCase {
        mode,
        width_units,
        layers,
        chains,
    }
}

fn smooth_strategy(limits: &SmoothLimits, modes: Vec<SmoothMode>) -> BoxedStrategy<SmoothCase> {
    let limits = *limits;
    (
        prop::sample::select(modes).no_shrink(),
        prop::collection::vec(any::<u32>(), smooth_raw_len(&limits)),
    )
        .prop_map(move |(mode, raw)| build_smooth_case(mode, &raw, &limits))
        .boxed()
}

// ── Deterministic smoothing witnesses ──────────────────────────────────────

/// One straight chain `origin + k * step` with linear radii.
fn line_scene(label: &'static str, n: usize, step: (i64, i64), width_units: i64) -> SmoothScene {
    let case = SmoothCase {
        mode: SmoothMode::Straight,
        width_units,
        layers: n,
        chains: vec![ChainGen {
            kind: ChainKind::Straight,
            base_level: 0,
            positions: (0..n as i64).map(|k| (k * step.0, k * step.1)).collect(),
            radii_q: (0..n as i64).map(|k| 16 + 2 * k).collect(),
            movements: vec![(0, 0); n],
            flags: vec![false; n],
            step: Some(step),
        }],
    };
    scene_from_case(label, &case)
}

/// Nonminimal bent five-node zig-zag (mm): (0,0) (1,0) (1,1) (2,1) (2,0).
fn bent_scene() -> SmoothScene {
    let pts = [(0, 0), (1, 0), (1, 1), (2, 1), (2, 0)];
    let case = SmoothCase {
        mode: SmoothMode::Bent,
        width_units: WITNESS_WIDTH_UNITS,
        layers: pts.len(),
        chains: vec![ChainGen {
            kind: ChainKind::Bent,
            base_level: 0,
            positions: pts.iter().map(|(x, y)| (x * MM, y * MM)).collect(),
            radii_q: vec![16, 24, 8, 32, 16],
            movements: vec![(5, -5), (0, 0), (0, 0), (0, 0), (-7, 3)],
            flags: vec![true, false, false, false, false],
            step: None,
        }],
    };
    scene_from_case("bent", &case)
}

/// A two-node and a one-node chain with nonzero pre-smoothing state.
fn short_scene() -> SmoothScene {
    let case = SmoothCase {
        mode: SmoothMode::Short,
        width_units: WITNESS_WIDTH_UNITS,
        layers: 2,
        chains: vec![
            ChainGen {
                kind: ChainKind::Short,
                base_level: 0,
                positions: vec![(0, 0), (3 * MM / 2, 2 * MM)],
                radii_q: vec![16, 20],
                movements: vec![(7, -3), (-11, 13)],
                flags: vec![true, false],
                step: None,
            },
            ChainGen {
                kind: ChainKind::Short,
                base_level: 1,
                positions: vec![(-MM, MM)],
                radii_q: vec![12],
                movements: vec![(4, 4)],
                flags: vec![true],
                step: None,
            },
        ],
    };
    scene_from_case("short", &case)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MergeVariant {
    /// `s` absorbed `m`; `c` (below `s`) has parents `[s, m]`.
    Merge,
    /// Same, but `c` is `Polygon`-typed, so it is not `m`'s fixed head.
    PolygonHead,
    /// No merge link at all (contrast for the parent-count trigger).
    NoMerge,
}

struct MergeIds {
    m: NodeId,
    mp: NodeId,
    c: NodeId,
}

/// Merge witness. Levels 4..0: `mpp`, `mp`, `{s, m}`, `c`, `d`. `s`, `c`,
/// `d` sit at x = 0; `m`, `mp`, `mpp` at x = 2 mm.
fn merge_scene(variant: MergeVariant) -> (SmoothScene, MergeIds) {
    let mut arena = NodeArena::default();
    let mk = |arena: &mut NodeArena, x: i64, level: usize, parent: Option<NodeId>| {
        arena.create_node(
            Point2 { x, y: 0 },
            3,
            level,
            0,
            true,
            parent,
            level as f32 * NODE_HEIGHT_MM,
            NODE_HEIGHT_MM,
            0.6,
            1.0,
        )
    };
    let mpp = mk(&mut arena, 2 * MM, 4, None);
    let mp = mk(&mut arena, 2 * MM, 3, Some(mpp));
    let m = mk(&mut arena, 2 * MM, 2, Some(mp));
    let s = mk(&mut arena, 0, 2, None);
    if variant != MergeVariant::NoMerge {
        arena[s].merged_neighbours.push(m);
        arena[m].valid = false;
    }
    let c = mk(&mut arena, 0, 1, Some(s));
    if variant == MergeVariant::PolygonHead {
        arena[c].type_ = TreeNodeType::Polygon;
    }
    let d = mk(&mut arena, 0, 0, Some(c));
    let mut by_level = vec![Vec::new(); 5];
    by_level[4].push(mpp);
    by_level[3].push(mp);
    by_level[2].extend([s, m]);
    by_level[1].push(c);
    by_level[0].push(d);
    let side = if variant == MergeVariant::Merge {
        vec![c, m, mp, mpp]
    } else {
        vec![m, mp, mpp]
    };
    let label = match variant {
        MergeVariant::Merge => "merge",
        MergeVariant::PolygonHead => "polygon-head",
        MergeVariant::NoMerge => "no-merge",
    };
    let scene = SmoothScene {
        label,
        arena,
        records: records_top_first(by_level),
        chains: vec![
            ChainDecl {
                ids: vec![d, c, s],
                straight: Some(Point2 { x: 0, y: 0 }),
            },
            ChainDecl {
                ids: side,
                straight: None,
            },
        ],
        width_units: WITNESS_WIDTH_UNITS,
    };
    (scene, MergeIds { m, mp, c })
}

#[derive(Debug, Default)]
struct SmoothWitness {
    straight_interior: u64,
    bent_interior: u64,
    merge_flags: u64,
    polygon_head: u64,
    short_nodes: u64,
    signed_threshold: u64,
}

fn smooth_witness_observations(limits: &SmoothLimits) -> Result<SmoothWitness, String> {
    let mut out = SmoothWitness::default();
    let checked = |scene: &SmoothScene| -> Result<(NodeArena, SmoothStats), String> {
        let observed = run_smooth(scene);
        let stats = check_smoothing(scene, limits, &observed)
            .map_err(|e| format!("{} witness: {e}", scene.label))?;
        Ok((observed, stats))
    };

    let (_, stats) = checked(&line_scene(
        "straight",
        5,
        (1_000, -500),
        WITNESS_WIDTH_UNITS,
    ))?;
    if stats.straight_interior != 3 {
        return Err("straight witness lost its interior nodes".to_owned());
    }
    out.straight_interior += stats.straight_interior;

    let bent = bent_scene();
    let (observed, stats) = checked(&bent)?;
    let moved = bent.chains[0].ids[1..4]
        .iter()
        .filter(|id| observed[**id].position != bent.arena[**id].position)
        .count();
    if stats.bent_interior != 3 || moved == 0 {
        return Err("bent witness interior did not relax".to_owned());
    }
    out.bent_interior += stats.bent_interior;

    // Merge: `c` is a committed interior node with two parents.
    let (scene, ids) = merge_scene(MergeVariant::Merge);
    let (observed, stats) = checked(&scene)?;
    let c = &observed[ids.c];
    if !(c.is_processed && c.need_extra_wall && c.parents.len() == 2) || stats.merge_flags == 0 {
        return Err("merge witness: the two-parent interior node is not flagged".to_owned());
    }
    if observed[ids.m].position.x >= 2 * MM || !observed[ids.m].is_processed {
        return Err("merge witness: the absorbed node was not pulled toward the column".to_owned());
    }
    out.merge_flags += stats.merge_flags;
    // Contrast: the same positions without the merge link leave `c` unflagged.
    let (scene, ids) = merge_scene(MergeVariant::NoMerge);
    let (observed, _) = checked(&scene)?;
    if observed[ids.c].need_extra_wall {
        return Err("no-merge contrast flagged the single-parent node".to_owned());
    }

    // Polygon-typed child is not a fixed head: `m` becomes a pinned endpoint.
    let (scene, ids) = merge_scene(MergeVariant::PolygonHead);
    let (observed, _) = checked(&scene)?;
    let m = &observed[ids.m];
    if m.position != (Point2 { x: 2 * MM, y: 0 })
        || m.movement != (Point2 { x: 0, y: 0 })
        || m.is_processed
        || !observed[ids.mp].is_processed
    {
        return Err("polygon-head witness: the Polygon child became a fixed head".to_owned());
    }
    out.polygon_head += 1;

    let (_, stats) = checked(&short_scene())?;
    if stats.short_nodes != 3 || stats.interior != 0 {
        return Err("short witness did not observe three unchanged nodes".to_owned());
    }
    out.short_nodes += stats.short_nodes;

    // Signed component threshold w/2 = 2000 units: strictly greater flags.
    for (step, flagged) in [
        ((3_000, 0), true),
        ((-3_000, 0), false),
        ((0, 2_500), true),
        ((0, -2_500), false),
        ((2_000, 0), false),
    ] {
        let scene = line_scene("signed-threshold", 4, step, WITNESS_WIDTH_UNITS);
        let (observed, stats) = checked(&scene)?;
        let interior = &scene.chains[0].ids[1..3];
        if !interior
            .iter()
            .all(|id| observed[*id].need_extra_wall == flagged)
        {
            return Err(format!(
                "threshold witness step {step:?} expected flag {flagged}"
            ));
        }
        out.signed_threshold += stats.interior;
    }
    Ok(out)
}

#[test]
fn proptest_tree_smoothing_graph() {
    let run = PropertyRun::new(SMOOTH_ID, SMOOTH_TEST).expect("M-TREE-SMOOTH context");
    let limits = checked_smooth_limits(&live_bounds(&run, &SMOOTH_BOUND_KEYS))
        .expect("tree smoothing bound consistency");
    let strategy = smooth_strategy(&limits, SMOOTH_MODES.to_vec());

    let calls = Cell::new(0_u64);
    let w_straight = Cell::new(0_u64);
    let w_bent = Cell::new(0_u64);
    let w_merge = Cell::new(0_u64);
    let w_polygon = Cell::new(0_u64);
    let w_short = Cell::new(0_u64);
    let w_signed = Cell::new(0_u64);
    let interior = Cell::new(0_u64);
    let straight_interior = Cell::new(0_u64);
    let short_nodes = Cell::new(0_u64);
    let flagged = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 4] = Default::default();
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let w = match smooth_witness_observations(&limits) {
            Ok(w) => w,
            Err(message) => return fail(message),
        };
        w_straight.set(w_straight.get() + w.straight_interior);
        w_bent.set(w_bent.get() + w.bent_interior);
        w_merge.set(w_merge.get() + w.merge_flags);
        w_polygon.set(w_polygon.get() + w.polygon_head);
        w_short.set(w_short.get() + w.short_nodes);
        w_signed.set(w_signed.get() + w.signed_threshold);

        let scene = scene_from_case("generated", &case);
        let observed = run_smooth(&scene);
        let stats = match check_smoothing(&scene, &limits, &observed) {
            Ok(stats) => stats,
            Err(e) => return fail(format!("{:?} case failed: {e}", case.mode)),
        };
        match case.mode {
            SmoothMode::Straight if stats.straight_interior == 0 => {
                return fail("straight case checked no interior node");
            }
            SmoothMode::Bent | SmoothMode::Mixed if stats.bent_interior == 0 => {
                return fail("bent case checked no interior node");
            }
            SmoothMode::Short if stats.short_nodes == 0 || stats.interior != 0 => {
                return fail("short case did not stay a short-chain scene");
            }
            _ => {}
        }
        let slot = SMOOTH_MODES
            .iter()
            .position(|m| *m == case.mode)
            .expect("mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        interior.set(interior.get() + stats.interior);
        straight_interior.set(straight_interior.get() + stats.straight_interior);
        short_nodes.set(short_nodes.get() + stats.short_nodes);
        flagged.set(flagged.get() + stats.flagged);
        Ok(())
    })
    .expect("M-TREE-SMOOTH graph references, pinned endpoints, hull and flags");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(w_straight.get() > 0, "straight witness unobserved");
    assert!(w_bent.get() > 0, "bent witness unobserved");
    assert!(w_merge.get() > 0, "merge flag witness unobserved");
    assert!(w_polygon.get() > 0, "polygon-head witness unobserved");
    assert!(w_short.get() > 0, "short-chain witness unobserved");
    assert!(w_signed.get() > 0, "signed-threshold witness unobserved");
    println!(
        "M-TREE-SMOOTH accepted_calls={} straight={} bent={} short={} mixed={} interior={} \
         straight_interior={} short_nodes={} flagged={} witness_straight={} witness_bent={} \
         witness_merge_flags={} witness_polygon_head={} witness_short={} witness_signed={}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        interior.get(),
        straight_interior.get(),
        short_nodes.get(),
        flagged.get(),
        w_straight.get(),
        w_bent.get(),
        w_merge.get(),
        w_polygon.get(),
        w_short.get(),
        w_signed.get()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// M-TREE-ROLES
// ═════════════════════════════════════════════════════════════════════════════

/// `[x0, x1, y0, y1]` in canonical units, model coordinates.
type Rect = [i64; 4];

/// Role slots in `build_roles` order.
const ROLE_ORDER: [SupportPlanRole; 4] = [
    SupportPlanRole::SupportBody,
    SupportPlanRole::TopInterface,
    SupportPlanRole::BaseInterface,
    SupportPlanRole::BottomInterface,
];
/// Canonical `COARSE_CIRCLE_NODE_THRESHOLD`: at or below it no role simplifies.
const NON_SIMPLIFYING_MAX_AVG: usize = 200;
/// The second witness is translated 20 mm, reaching x = 30 mm.
const ROLES_WITNESS_ABS: i64 = 30 * MM;
const ROLES_ABS_CEILING: i64 = 1_000_000;
/// The swallowed-region witness has three regions.
const ROLES_WITNESS_REGIONS: usize = 3;
const MAX_REGIONS_CEILING: usize = 32;
const MAX_BATCH_CEILING: usize = 16;
const MAX_BARS: usize = 2;

#[derive(Debug, Clone, Copy)]
struct RolesLimits {
    max_abs_units: i64,
    max_regions: usize,
    max_cells: usize,
    max_batch_items: usize,
}

/// Partition cells the input alone can create: at most `2 * (regions +
/// bars + territory)` distinct coordinates per axis.
fn input_cell_ceiling(max_regions: usize) -> usize {
    let per_axis = 2 * (max_regions + MAX_BARS + 1);
    (per_axis - 1) * (per_axis - 1)
}

fn checked_roles_limits(bounds: &BTreeMap<String, u64>) -> Result<RolesLimits, DomainError> {
    let limits = RolesLimits {
        max_abs_units: as_i64(bounds, "max_abs_units")?,
        max_regions: as_usize(bounds, "max_regions")?,
        max_cells: as_usize(bounds, "max_cells")?,
        max_batch_items: as_usize(bounds, "max_batch_items")?,
    };
    if !(ROLES_WITNESS_ABS..=ROLES_ABS_CEILING).contains(&limits.max_abs_units) {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units must be in {ROLES_WITNESS_ABS}..={ROLES_ABS_CEILING}"
        )));
    }
    if !(ROLES_WITNESS_REGIONS..=MAX_REGIONS_CEILING).contains(&limits.max_regions) {
        return Err(DomainError::InvalidBounds(format!(
            "max_regions must be in {ROLES_WITNESS_REGIONS}..={MAX_REGIONS_CEILING}"
        )));
    }
    let need = input_cell_ceiling(limits.max_regions);
    if !(need..=1 << 20).contains(&limits.max_cells) {
        return Err(DomainError::InvalidBounds(format!(
            "max_cells must be in {need}..={} for max_regions {}",
            1 << 20,
            limits.max_regions
        )));
    }
    if !(2..=MAX_BATCH_CEILING).contains(&limits.max_batch_items) {
        return Err(DomainError::InvalidBounds(format!(
            "max_batch_items must be in 2..={MAX_BATCH_CEILING}: association needs two items"
        )));
    }
    Ok(limits)
}

fn roles_probe_bounds() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 300_000),
        ("max_regions".to_owned(), 3),
        ("max_cells".to_owned(), 121),
        ("max_batch_items".to_owned(), 2),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RolesMode {
    /// Item region 0 is split by a collision bar into two unequal sides.
    Split,
    /// Item region 0 lies wholly inside a collision bar.
    Swallow,
    /// No collision at all.
    NoCollision,
    /// `Split` plus an own territory on every item (batched only).
    Territory,
}

const ROLES_MODES: [RolesMode; 4] = [
    RolesMode::Split,
    RolesMode::Swallow,
    RolesMode::NoCollision,
    RolesMode::Territory,
];

/// One `build_roles` / `RoleBuildInput` item in model coordinates.
#[derive(Debug, Clone, PartialEq)]
struct RoleItem {
    /// Swap X and Y of every rectangle before the SUT sees it.
    transpose: bool,
    /// Collision bars `[x0, x1]` spanning `bar_y`.
    bars: Vec<[i64; 2]>,
    bar_y: [i64; 2],
    /// Body, roof (top), base and floor (bottom) area rectangles.
    regions: [Vec<Rect>; 4],
    territory: Option<Rect>,
    avg_node_per_layer: usize,
    line_width_mm: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct RolesCase {
    mode: RolesMode,
    items: Vec<RoleItem>,
}

impl RoleItem {
    fn collision_rects(&self) -> Vec<Rect> {
        self.bars
            .iter()
            .map(|b| [b[0], b[1], self.bar_y[0], self.bar_y[1]])
            .collect()
    }

    fn sut_rect(&self, r: Rect) -> ExPolygon {
        let [x0, x1, y0, y1] = if self.transpose {
            [r[2], r[3], r[0], r[1]]
        } else {
            r
        };
        // Counter-clockwise in SUT coordinates (canonical contour winding).
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: x0, y: y0 },
                    Point2 { x: x1, y: y0 },
                    Point2 { x: x1, y: y1 },
                    Point2 { x: x0, y: y1 },
                ],
            },
            holes: Vec::new(),
        }
    }

    fn sut_list(&self, rects: &[Rect]) -> Vec<ExPolygon> {
        rects.iter().map(|r| self.sut_rect(*r)).collect()
    }

    fn sut_collision(&self) -> Vec<ExPolygon> {
        self.sut_list(&self.collision_rects())
    }

    /// Every region, flattened in role order.
    fn all_regions(&self) -> Vec<Rect> {
        self.regions.iter().flatten().copied().collect()
    }

    fn build_input(&self) -> RoleBuildInput {
        RoleBuildInput {
            branch_segments: Vec::new(),
            interface_segments: Vec::new(),
            base_segments: Vec::new(),
            floor_segments: Vec::new(),
            branch_areas: self.sut_list(&self.regions[0]),
            interface_areas: self.sut_list(&self.regions[1]),
            base_areas: self.sut_list(&self.regions[2]),
            floor_areas: self.sut_list(&self.regions[3]),
            branch_radius: 1.0,
            collision_polys: self.sut_collision(),
            avg_node_per_layer: self.avg_node_per_layer,
            line_width_mm: self.line_width_mm,
            own_territory: self.territory.map(|t| vec![self.sut_rect(t)]),
        }
    }

    fn run_serial(&self) -> Vec<SupportPlanRoleRegion> {
        build_roles(
            &[],
            &[],
            &[],
            &[],
            &self.sut_list(&self.regions[0]),
            &self.sut_list(&self.regions[1]),
            &self.sut_list(&self.regions[2]),
            &self.sut_list(&self.regions[3]),
            1.0,
            &self.sut_collision(),
            self.avg_node_per_layer,
            self.line_width_mm,
        )
    }

    fn run_carve(&self) -> Vec<ExPolygon> {
        carve_emitted_regions(&self.sut_list(&self.all_regions()), &self.sut_collision())
    }

    fn run_gated(&self, simplify_active: bool) -> Vec<ExPolygon> {
        let collision = self.sut_collision();
        carve_emitted_regions_with_bbox_gate(
            &self.sut_list(&self.all_regions()),
            &collision,
            expolygons_bbox(&collision),
            simplify_active,
        )
    }

    /// Back to model coordinates (an observation transform only).
    fn to_model(&self, poly: &ExPolygon) -> ExPolygon {
        if !self.transpose {
            return poly.clone();
        }
        let swap = |ring: &Polygon| Polygon {
            points: ring
                .points
                .iter()
                .map(|p| Point2 { x: p.y, y: p.x })
                .collect(),
        };
        ExPolygon {
            contour: swap(&poly.contour),
            holes: poly.holes.iter().map(swap).collect(),
        }
    }
}

fn run_batched(items: &[RoleItem]) -> Vec<Vec<SupportPlanRoleRegion>> {
    let inputs: Vec<RoleBuildInput> = items.iter().map(RoleItem::build_input).collect();
    let refs: Vec<&RoleBuildInput> = inputs.iter().collect();
    build_roles_batched(&refs)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RoleFailure {
    /// The scene is outside the declared domain (generator bug).
    Fixture(String),
    /// Output count/order does not match the inputs.
    Association(String),
    /// Role order, duplication, an empty role vector or a missing/extra role.
    RoleOrder(String),
    /// A ring is short or has a non-axis-aligned edge.
    Shape(String),
    /// Output material inside collision.
    Collision(String),
    /// Body overlaps an interface role.
    Overlap(String),
    /// A scene with expected material produced no roles at all.
    FalseEmpty,
    /// Material outside the own territory that only territory clipping removes.
    Territory(String),
    /// Any other missing or extra cell.
    Membership(String),
}

impl fmt::Display for RoleFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(m) => write!(f, "fixture outside the domain: {m}"),
            Self::Association(m) => write!(f, "association: {m}"),
            Self::RoleOrder(m) => write!(f, "role order: {m}"),
            Self::Shape(m) => write!(f, "shape: {m}"),
            Self::Collision(m) => write!(f, "collision material: {m}"),
            Self::Overlap(m) => write!(f, "body/interface overlap: {m}"),
            Self::FalseEmpty => write!(f, "a scene with material produced no roles"),
            Self::Territory(m) => write!(f, "own territory: {m}"),
            Self::Membership(m) => write!(f, "membership: {m}"),
        }
    }
}

/// Open intervals of `[x0, x1]` not covered by any bar.
fn surviving_intervals(x0: i64, x1: i64, bars: &[[i64; 2]]) -> Vec<[i64; 2]> {
    let mut cuts = bars.to_vec();
    cuts.sort_unstable();
    let mut out = Vec::new();
    let mut cursor = x0;
    for [b0, b1] in cuts {
        if b1 <= cursor {
            continue;
        }
        if b0 >= x1 {
            break;
        }
        if b0 > cursor {
            out.push([cursor, b0]);
        }
        cursor = cursor.max(b1);
        if cursor >= x1 {
            break;
        }
    }
    if cursor < x1 {
        out.push([cursor, x1]);
    }
    out
}

/// The unique widest survivor of `r` minus the bars (the canonical largest
/// part: every survivor shares `r`'s y-extent, so area order is width
/// order). `Err` on a tie, which the generator never produces.
fn survivor(r: Rect, bars: &[[i64; 2]]) -> Result<(Option<Rect>, usize), String> {
    let pieces = surviving_intervals(r[0], r[1], bars);
    let Some(widest) = pieces.iter().map(|p| p[1] - p[0]).max() else {
        return Ok((None, 0));
    };
    let tied: Vec<&[i64; 2]> = pieces.iter().filter(|p| p[1] - p[0] == widest).collect();
    if tied.len() != 1 {
        return Err(format!(
            "region {r:?} has {} tied widest survivors",
            tied.len()
        ));
    }
    Ok((Some([tied[0][0], tied[0][1], r[2], r[3]]), pieces.len()))
}

/// Doubled-coordinate open-rectangle membership.
fn rect_contains(r: Rect, p: (i64, i64)) -> bool {
    2 * r[0] < p.0 && p.0 < 2 * r[1] && 2 * r[2] < p.1 && p.1 < 2 * r[3]
}

/// Exact crossing parity of an axis-aligned ring at a doubled-coordinate
/// point that is never on an edge.
fn ring_parity(ring: &Polygon, p: (i64, i64)) -> bool {
    let pts = &ring.points;
    let mut inside = false;
    for (a, b) in pts.iter().zip(pts.iter().cycle().skip(1)) {
        if a.x == b.x && 2 * a.x > p.0 {
            let (lo, hi) = (2 * a.y.min(b.y), 2 * a.y.max(b.y));
            if lo < p.1 && p.1 < hi {
                inside = !inside;
            }
        }
    }
    inside
}

fn expoly_contains(e: &ExPolygon, p: (i64, i64)) -> bool {
    e.holes
        .iter()
        .fold(ring_parity(&e.contour, p), |v, h| v ^ ring_parity(h, p))
}

fn any_contains(polys: &[ExPolygon], p: (i64, i64)) -> bool {
    polys.iter().any(|e| expoly_contains(e, p))
}

fn check_shape(polys: &[ExPolygon], what: &str) -> Result<(), RoleFailure> {
    for e in polys {
        for ring in std::iter::once(&e.contour).chain(&e.holes) {
            let pts = &ring.points;
            if pts.len() < 3 {
                return Err(RoleFailure::Shape(format!(
                    "{what}: a ring has {} points",
                    pts.len()
                )));
            }
            for (a, b) in pts.iter().zip(pts.iter().cycle().skip(1)) {
                if a.x != b.x && a.y != b.y {
                    return Err(RoleFailure::Shape(format!(
                        "{what}: edge ({},{})->({},{}) is not axis-aligned",
                        a.x, a.y, b.x, b.y
                    )));
                }
            }
        }
    }
    Ok(())
}

/// The independent per-item model.
struct ItemModel {
    /// Surviving pieces per role.
    pieces: [Vec<Rect>; 4],
    /// Survivor of every region in flattened role order.
    region_survivors: Vec<Option<Rect>>,
    collision: Vec<Rect>,
    territory: Option<Rect>,
    xs: BTreeSet<i64>,
    ys: BTreeSet<i64>,
    split_regions: u64,
    swallowed_regions: u64,
}

impl ItemModel {
    fn raw(&self, role: usize, p: (i64, i64)) -> bool {
        self.pieces[role].iter().any(|r| rect_contains(*r, p))
    }

    /// Expected membership of `role` at `p`, optionally territory-clipped.
    fn expected(&self, role: usize, p: (i64, i64), with_territory: bool) -> bool {
        let v = if role == 0 {
            self.raw(0, p) && !(1..4).any(|k| self.raw(k, p))
        } else {
            self.raw(role, p)
        };
        v && (!with_territory || self.territory.is_none_or(|t| rect_contains(t, p)))
    }

    fn in_collision(&self, p: (i64, i64)) -> bool {
        self.collision.iter().any(|r| rect_contains(*r, p))
    }
}

fn model_item(item: &RoleItem, limits: &RolesLimits) -> Result<ItemModel, RoleFailure> {
    let fixture = |m: String| Err(RoleFailure::Fixture(m));
    let m = limits.max_abs_units;
    let ok_rect = |r: &Rect| r[0] < r[1] && r[2] < r[3] && r.iter().all(|v| v.abs() <= m);
    let regions = item.all_regions();
    if regions.len() > limits.max_regions || item.bars.len() > MAX_BARS {
        return fixture(format!(
            "{} regions / {} bars exceed the bounds",
            regions.len(),
            item.bars.len()
        ));
    }
    if !regions.iter().all(ok_rect) || !item.territory.iter().all(ok_rect) {
        return fixture("a rectangle is empty or exceeds max_abs_units".to_owned());
    }
    let bar_y = item.bar_y;
    if !item.bars.is_empty() {
        if !item
            .bars
            .iter()
            .all(|b| b[0] < b[1] && b[0].abs() <= m && b[1].abs() <= m)
            || !(bar_y[0] < bar_y[1] && bar_y[0].abs() <= m && bar_y[1].abs() <= m)
        {
            return fixture("a collision bar is empty or exceeds max_abs_units".to_owned());
        }
        if !regions.iter().all(|r| bar_y[0] <= r[2] && r[3] <= bar_y[1]) {
            return fixture("collision bars must span every region's y-extent".to_owned());
        }
    }
    if item.avg_node_per_layer > NON_SIMPLIFYING_MAX_AVG
        || !(item.line_width_mm.is_finite() && item.line_width_mm > 0.0)
        || role_simplify_tolerance(true, item.avg_node_per_layer, item.line_width_mm).is_some()
    {
        return fixture("the scene must stay in the non-simplifying domain".to_owned());
    }
    let mut pieces: [Vec<Rect>; 4] = Default::default();
    let mut region_survivors = Vec::new();
    let (mut split_regions, mut swallowed_regions) = (0, 0);
    for (role, rects) in item.regions.iter().enumerate() {
        for r in rects {
            let (piece, count) = survivor(*r, &item.bars).map_err(RoleFailure::Fixture)?;
            if count >= 2 {
                split_regions += 1;
            }
            if piece.is_none() {
                swallowed_regions += 1;
            }
            pieces[role].extend(piece);
            region_survivors.push(piece);
        }
    }
    let collision = item.collision_rects();
    let mut xs = BTreeSet::new();
    let mut ys = BTreeSet::new();
    for r in regions
        .iter()
        .chain(&collision)
        .chain(item.territory.iter())
    {
        xs.extend([r[0], r[1]]);
        ys.extend([r[2], r[3]]);
    }
    let cells = xs.len().saturating_sub(1) * ys.len().saturating_sub(1);
    if cells > limits.max_cells {
        return fixture(format!(
            "{cells} input cells exceed max_cells {}",
            limits.max_cells
        ));
    }
    Ok(ItemModel {
        pieces,
        region_survivors,
        collision,
        territory: item.territory,
        xs,
        ys,
        split_regions,
        swallowed_regions,
    })
}

/// Doubled-coordinate centres of the open cells of the partition at every
/// model and observed coordinate.
fn cell_centres(
    model: &ItemModel,
    observed: &[&ExPolygon],
    limits: &RolesLimits,
) -> Result<Vec<(i64, i64)>, RoleFailure> {
    let mut xs = model.xs.clone();
    let mut ys = model.ys.clone();
    for e in observed {
        for ring in std::iter::once(&e.contour).chain(&e.holes) {
            for p in &ring.points {
                xs.insert(p.x);
                ys.insert(p.y);
            }
        }
    }
    let cells = xs.len().saturating_sub(1) * ys.len().saturating_sub(1);
    if cells > 4 * limits.max_cells {
        return Err(RoleFailure::Shape(format!(
            "observed vertices expand the partition to {cells} cells"
        )));
    }
    let xs: Vec<i64> = xs.into_iter().collect();
    let ys: Vec<i64> = ys.into_iter().collect();
    let mut out = Vec::with_capacity(cells);
    for x in xs.windows(2) {
        for y in ys.windows(2) {
            out.push((x[0] + x[1], y[0] + y[1]));
        }
    }
    Ok(out)
}

fn mm_point(p: (i64, i64)) -> String {
    format!("({}, {}) mm", p.0 as f64 / 20_000.0, p.1 as f64 / 20_000.0)
}

#[derive(Debug, Default, Clone, Copy)]
struct RoleStats {
    roles: u64,
    body_cells: u64,
    interface_cells: u64,
    collision_cells: u64,
    territory_removed: u64,
}

/// The one roles checker (serial `build_roles` with `with_territory =
/// false`; batched items with `with_territory = true`).
fn check_roles(
    item: &RoleItem,
    limits: &RolesLimits,
    roles: &[SupportPlanRoleRegion],
    with_territory: bool,
) -> Result<RoleStats, RoleFailure> {
    let model = model_item(item, limits)?;
    // Structure: canonical order, no duplicates, no Raft, no empty vectors.
    let mut slots = Vec::new();
    for role in roles {
        let Some(slot) = ROLE_ORDER.iter().position(|r| *r == role.role) else {
            return Err(RoleFailure::RoleOrder(format!(
                "unexpected role {:?}",
                role.role
            )));
        };
        if slots.last().is_some_and(|prev| *prev >= slot) {
            return Err(RoleFailure::RoleOrder(format!(
                "{:?} out of canonical order or duplicated",
                role.role
            )));
        }
        if role.regions.is_empty() {
            return Err(RoleFailure::RoleOrder(format!(
                "{:?} has no regions",
                role.role
            )));
        }
        slots.push(slot);
    }
    let mut observed: [Vec<ExPolygon>; 4] = Default::default();
    for (role, slot) in roles.iter().zip(&slots) {
        check_shape(&role.regions, &format!("{:?}", role.role))?;
        observed[*slot] = role.regions.iter().map(|e| item.to_model(e)).collect();
    }
    let all: Vec<&ExPolygon> = observed.iter().flatten().collect();
    let centres = cell_centres(&model, &all, limits)?;
    let obs = |k: usize, p: (i64, i64)| any_contains(&observed[k], p);

    for &p in &centres {
        if model.in_collision(p) && (0..4).any(|k| obs(k, p)) {
            return Err(RoleFailure::Collision(format!(
                "material at {}",
                mm_point(p)
            )));
        }
    }
    for &p in &centres {
        if obs(0, p) && (1..4).any(|k| obs(k, p)) {
            return Err(RoleFailure::Overlap(format!(
                "body and interface at {}",
                mm_point(p)
            )));
        }
    }
    let expected_present: Vec<usize> = (0..4)
        .filter(|k| {
            centres
                .iter()
                .any(|p| model.expected(*k, *p, with_territory))
        })
        .collect();
    if slots.is_empty() && !expected_present.is_empty() {
        return Err(RoleFailure::FalseEmpty);
    }
    if slots != expected_present {
        return Err(RoleFailure::RoleOrder(format!(
            "observed role slots {slots:?}, expected {expected_present:?}"
        )));
    }
    let mut stats = RoleStats {
        roles: slots.len() as u64,
        ..RoleStats::default()
    };
    let inputs = item.all_regions();
    for &p in &centres {
        if model.in_collision(p) && inputs.iter().any(|r| rect_contains(*r, p)) {
            stats.collision_cells += 1;
        }
        for k in 0..4 {
            let want = model.expected(k, p, with_territory);
            let without = model.expected(k, p, false);
            let got = obs(k, p);
            if got && !want {
                return Err(if without {
                    RoleFailure::Territory(format!(
                        "{:?} keeps {} outside the own territory",
                        ROLE_ORDER[k],
                        mm_point(p)
                    ))
                } else {
                    RoleFailure::Membership(format!(
                        "{:?} has extra material at {}",
                        ROLE_ORDER[k],
                        mm_point(p)
                    ))
                });
            }
            if want && !got {
                return Err(RoleFailure::Membership(format!(
                    "{:?} misses {}",
                    ROLE_ORDER[k],
                    mm_point(p)
                )));
            }
            if want {
                if k == 0 {
                    stats.body_cells += 1;
                } else {
                    stats.interface_cells += 1;
                }
            }
            if without && !want {
                stats.territory_removed += 1;
            }
        }
    }
    Ok(stats)
}

fn check_batch(
    items: &[RoleItem],
    limits: &RolesLimits,
    outputs: &[Vec<SupportPlanRoleRegion>],
) -> Result<Vec<RoleStats>, RoleFailure> {
    if items.len() > limits.max_batch_items {
        return Err(RoleFailure::Fixture(format!("{} batch items", items.len())));
    }
    if outputs.len() != items.len() {
        return Err(RoleFailure::Association(format!(
            "{} outputs for {} inputs",
            outputs.len(),
            items.len()
        )));
    }
    items
        .iter()
        .zip(outputs)
        .enumerate()
        .map(|(i, (item, roles))| {
            check_roles(item, limits, roles, true).map_err(|e| match e {
                RoleFailure::Fixture(m) => RoleFailure::Fixture(format!("item {i}: {m}")),
                other => other,
            })
        })
        .collect()
}

#[derive(Debug, Default, Clone, Copy)]
struct CarveStats {
    outputs: u64,
    swallowed: u64,
    split: u64,
    cells: u64,
}

/// Per-region carve association: the i-th output is the i-th surviving
/// region's largest survivor (or the region itself without collision), as a
/// geometric set.
fn check_carve(
    item: &RoleItem,
    limits: &RolesLimits,
    outputs: &[ExPolygon],
) -> Result<CarveStats, RoleFailure> {
    let model = model_item(item, limits)?;
    let expected: Vec<Rect> = model.region_survivors.iter().flatten().copied().collect();
    if outputs.len() != expected.len() {
        return Err(RoleFailure::Association(format!(
            "{} carve outputs for {} surviving regions",
            outputs.len(),
            expected.len()
        )));
    }
    check_shape(outputs, "carve")?;
    let mut stats = CarveStats {
        outputs: outputs.len() as u64,
        swallowed: model.swallowed_regions,
        split: model.split_regions,
        cells: 0,
    };
    for (j, (out, want)) in outputs.iter().zip(&expected).enumerate() {
        let out = item.to_model(out);
        for p in cell_centres(&model, &[&out], limits)? {
            let got = expoly_contains(&out, p);
            if got && model.in_collision(p) {
                return Err(RoleFailure::Collision(format!(
                    "carve output {j} keeps {}",
                    mm_point(p)
                )));
            }
            if got != rect_contains(*want, p) {
                return Err(RoleFailure::Membership(format!(
                    "carve output {j} {} {}",
                    if got { "has extra" } else { "misses" },
                    mm_point(p)
                )));
            }
            stats.cells += u64::from(got);
        }
    }
    Ok(stats)
}

// ── Generator ───────────────────────────────────────────────────────────────

fn item_raw_len(l: &RolesLimits) -> usize {
    19 + 5 * l.max_regions
}

fn rect_from(r: &mut Raw<'_>, lo: i64, hi: i64) -> Rect {
    let x0 = pick(r.next(), lo, hi - 1);
    let x1 = pick(r.next(), x0 + 1, hi);
    let y0 = pick(r.next(), lo, hi - 1);
    let y1 = pick(r.next(), y0 + 1, hi);
    [x0, x1, y0, y1]
}

/// Resolve a widest-survivor tie by growing one free end by one unit.
fn untie(r: Rect, bars: &[[i64; 2]]) -> Rect {
    if survivor(r, bars).is_ok() {
        return r;
    }
    for candidate in [[r[0] - 1, r[1], r[2], r[3]], [r[0], r[1] + 1, r[2], r[3]]] {
        if survivor(candidate, bars).is_ok() {
            return candidate;
        }
    }
    panic!("generator could not untie region {r:?} against bars {bars:?}");
}

fn build_item(mode: RolesMode, raw: &[u32], l: &RolesLimits) -> RoleItem {
    let m = l.max_abs_units;
    // Regions keep a one-unit margin so `untie` stays inside `max_abs_units`.
    let (lo, hi) = (-m + 1, m - 1);
    let mut r = Raw::new(raw);
    let transpose = r.next() % 2 == 1;
    let (a, b, c, d) = (r.next(), r.next(), r.next(), r.next());
    let (e, f) = (r.next(), r.next());
    let (mut bars, region0) = match mode {
        RolesMode::Split | RolesMode::Territory => {
            let x0 = pick(a, lo, hi - 3);
            let x1 = pick(b, x0 + 3, hi);
            let y0 = pick(c, lo, hi - 1);
            let y1 = pick(d, y0 + 1, hi);
            let bx0 = pick(e, x0 + 1, x1 - 2);
            let bx1 = pick(f, bx0 + 1, x1 - 1);
            (vec![[bx0, bx1]], [x0, x1, y0, y1])
        }
        RolesMode::Swallow => {
            let bx0 = pick(e, lo, hi - 1);
            let bx1 = pick(f, bx0 + 1, hi);
            let x0 = pick(a, bx0, bx1 - 1);
            let x1 = pick(b, x0 + 1, bx1);
            let y0 = pick(c, lo, hi - 1);
            let y1 = pick(d, y0 + 1, hi);
            (vec![[bx0, bx1]], [x0, x1, y0, y1])
        }
        RolesMode::NoCollision => {
            let x0 = pick(a, lo, hi - 1);
            let x1 = pick(b, x0 + 1, hi);
            let y0 = pick(c, lo, hi - 1);
            let y1 = pick(d, y0 + 1, hi);
            (Vec::new(), [x0, x1, y0, y1])
        }
    };
    let (second, sx0, sx1) = (r.next(), r.next(), r.next());
    if mode != RolesMode::NoCollision && second % 2 == 1 {
        let bx0 = pick(sx0, lo, hi - 1);
        let extra = [bx0, pick(sx1, bx0 + 1, hi)];
        let mut with = bars.clone();
        with.push(extra);
        // Keep the Split/Territory witness: region 0 must stay split.
        let keeps = match mode {
            RolesMode::Split | RolesMode::Territory => {
                surviving_intervals(region0[0], region0[1], &with).len() >= 2
            }
            _ => true,
        };
        if keeps {
            bars = with;
        }
    }
    let territory_rect = rect_from(&mut r, -m, m);
    let total = pick_usize(r.next(), 1, l.max_regions);
    let mut regions: [Vec<Rect>; 4] = Default::default();
    regions[0].push(untie(region0, &bars));
    for k in 1..l.max_regions {
        let role = r.next() as usize % 4;
        let rect = rect_from(&mut r, lo, hi);
        if k < total {
            regions[role].push(untie(rect, &bars));
        }
    }
    let avg_node_per_layer = pick_usize(r.next(), 0, NON_SIMPLIFYING_MAX_AVG);
    let line_width_mm = pick(r.next(), 1, 128) as f32 / 64.0;
    RoleItem {
        transpose,
        bars,
        bar_y: [-m, m],
        regions,
        territory: (mode == RolesMode::Territory).then_some(territory_rect),
        avg_node_per_layer,
        line_width_mm,
    }
}

fn build_roles_case(mode: RolesMode, raw: &[u32], l: &RolesLimits) -> RolesCase {
    let per = item_raw_len(l);
    let count = pick_usize(raw[0], 1, l.max_batch_items);
    let items = (0..count)
        .map(|i| build_item(mode, &raw[1 + i * per..1 + (i + 1) * per], l))
        .collect();
    RolesCase { mode, items }
}

fn roles_strategy(limits: &RolesLimits, modes: Vec<RolesMode>) -> BoxedStrategy<RolesCase> {
    let limits = *limits;
    let len = 1 + limits.max_batch_items * item_raw_len(&limits);
    (
        prop::sample::select(modes).no_shrink(),
        prop::collection::vec(any::<u32>(), len),
    )
        .prop_map(move |(mode, raw)| build_roles_case(mode, &raw, &limits))
        .boxed()
}

// ── Deterministic role witnesses ───────────────────────────────────────────

fn mm_rect(x0: i64, x1: i64, y0: i64, y1: i64) -> Rect {
    [x0 * MM, x1 * MM, y0 * MM, y1 * MM]
}

fn shift(r: Rect, dx: i64) -> Rect {
    [r[0] + dx, r[1] + dx, r[2], r[3]]
}

/// Packet witness translated by `dx` units: body `[0,10]²`, collision
/// `[7,8] × [0,10]`, roof `[3,5]²`, optional territory `[0,6] × [0,10]` mm.
fn witness_item(dx: i64, territory: bool) -> RoleItem {
    RoleItem {
        transpose: false,
        bars: vec![[7 * MM + dx, 8 * MM + dx]],
        bar_y: [0, 10 * MM],
        regions: [
            vec![shift(mm_rect(0, 10, 0, 10), dx)],
            vec![shift(mm_rect(3, 5, 3, 5), dx)],
            Vec::new(),
            Vec::new(),
        ],
        territory: territory.then(|| shift(mm_rect(0, 6, 0, 10), dx)),
        avg_node_per_layer: 1,
        line_width_mm: 0.4,
    }
}

fn role_regions_of(roles: &[SupportPlanRoleRegion], role: SupportPlanRole) -> &[ExPolygon] {
    roles
        .iter()
        .find(|r| r.role == role)
        .map_or(&[], |r| r.regions.as_slice())
}

/// SDK exact point oracle at a named millimetre point.
fn located(polys: &[ExPolygon], x: f32, y: f32) -> PointLocation {
    point_in_region(Point2::from_mm(x, y), polys).expect("point oracle")
}

#[derive(Debug, Default)]
struct RolesWitness {
    body_cells: u64,
    roof_cells: u64,
    collision_cells: u64,
    territory_removed: u64,
    swallowed: u64,
    split: u64,
    no_collision_identity: u64,
    gated_identity: u64,
    batch_items: u64,
}

fn roles_witness_observations(limits: &RolesLimits) -> Result<RolesWitness, String> {
    let mut out = RolesWitness::default();
    let a = witness_item(0, false);

    // Serial build_roles.
    let roles = a.run_serial();
    let stats = check_roles(&a, limits, &roles, false).map_err(|e| format!("serial: {e}"))?;
    let body = role_regions_of(&roles, SupportPlanRole::SupportBody);
    let roof = role_regions_of(&roles, SupportPlanRole::TopInterface);
    let named = [
        (body, 1.0, 1.0, PointLocation::Inside),
        (body, 7.5, 1.0, PointLocation::Outside),
        (body, 8.5, 1.0, PointLocation::Outside),
        (body, 4.0, 4.0, PointLocation::Outside),
        (roof, 4.0, 4.0, PointLocation::Inside),
    ];
    for (polys, x, y, want) in named {
        if located(polys, x, y) != want {
            return Err(format!("serial witness: ({x},{y}) mm is not {want:?}"));
        }
    }
    if stats.body_cells == 0 || stats.interface_cells == 0 || stats.collision_cells == 0 {
        return Err("serial witness lacks body, roof or collision cells".to_owned());
    }
    out.body_cells += stats.body_cells;
    out.roof_cells += stats.interface_cells;
    out.collision_cells += stats.collision_cells;

    // Carve: the unique larger side [0,7] survives.
    let carve = check_carve(&a, limits, &a.run_carve()).map_err(|e| format!("carve: {e}"))?;
    if carve.split == 0 {
        return Err("carve witness was not split".to_owned());
    }
    out.split += carve.split;

    // Fully swallowed region [7.2,7.8] × [2,3] mm inside the collision bar.
    let mut swallow = a.clone();
    swallow.regions[0].push([72_000, 78_000, 2 * MM, 3 * MM]);
    let carve =
        check_carve(&swallow, limits, &swallow.run_carve()).map_err(|e| format!("swallow: {e}"))?;
    if carve.swallowed != 1 || carve.outputs != 2 {
        return Err("swallowed witness did not drop exactly one region".to_owned());
    }
    check_roles(&swallow, limits, &swallow.run_serial(), false)
        .map_err(|e| format!("swallow roles: {e}"))?;
    out.swallowed += carve.swallowed;

    // No collision: identity carve and a plain body-minus-roof.
    let mut free = a.clone();
    free.bars.clear();
    let outputs = free.run_carve();
    check_carve(&free, limits, &outputs).map_err(|e| format!("no-collision carve: {e}"))?;
    if outputs != free.sut_list(&free.all_regions()) {
        return Err("no-collision carve is not the input".to_owned());
    }
    let stats = check_roles(&free, limits, &free.run_serial(), false)
        .map_err(|e| format!("no-collision roles: {e}"))?;
    if stats.collision_cells != 0 || stats.body_cells == 0 {
        return Err("no-collision witness is not collision-free with a body".to_owned());
    }
    out.no_collision_identity += 1;

    // A bbox-gated region [12,14] × [0,2] mm, disjoint from the collision
    // bbox, kept as a geometric set; also the lone-gated repair path.
    let mut gated = a.clone();
    gated.regions[0].push(mm_rect(12, 14, 0, 2));
    let mut lone = gated.clone();
    lone.regions = [
        vec![mm_rect(12, 14, 0, 2)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];
    for (item, label) in [(&gated, "gated"), (&lone, "lone gated")] {
        for simplify_active in [false, true] {
            check_carve(item, limits, &item.run_gated(simplify_active))
                .map_err(|e| format!("{label} (simplify_active={simplify_active}): {e}"))?;
        }
        check_roles(item, limits, &item.run_serial(), false)
            .map_err(|e| format!("{label} roles: {e}"))?;
        out.gated_identity += 1;
    }

    // Production batched kernel: two distinguishable items with territory.
    let items = [witness_item(0, true), witness_item(20 * MM, true)];
    let outputs = run_batched(&items);
    let stats = check_batch(&items, limits, &outputs).map_err(|e| format!("batched: {e}"))?;
    let body = role_regions_of(&outputs[0], SupportPlanRole::SupportBody);
    if located(body, 6.5, 1.0) != PointLocation::Outside
        || located(body, 1.0, 1.0) != PointLocation::Inside
    {
        return Err("batched witness did not clip to the own territory".to_owned());
    }
    if stats
        .iter()
        .any(|s| s.territory_removed == 0 || s.body_cells == 0)
    {
        return Err("batched witness lacks territory-removed or body cells".to_owned());
    }
    out.territory_removed += stats.iter().map(|s| s.territory_removed).sum::<u64>();
    out.batch_items += stats.len() as u64;
    Ok(out)
}

#[derive(Debug, Default)]
struct RolesCaseStats {
    carve: CarveStats,
    serial: RoleStats,
    batched: RoleStats,
}

fn check_roles_case(case: &RolesCase, limits: &RolesLimits) -> Result<RolesCaseStats, String> {
    let mut out = RolesCaseStats::default();
    for (i, item) in case.items.iter().enumerate() {
        let carve = check_carve(item, limits, &item.run_carve())
            .map_err(|e| format!("item {i} carve: {e}"))?;
        for simplify_active in [false, true] {
            check_carve(item, limits, &item.run_gated(simplify_active)).map_err(|e| {
                format!("item {i} gated carve (simplify_active={simplify_active}): {e}")
            })?;
        }
        let serial = check_roles(item, limits, &item.run_serial(), false)
            .map_err(|e| format!("item {i} serial: {e}"))?;
        out.carve.outputs += carve.outputs;
        out.carve.swallowed += carve.swallowed;
        out.carve.split += carve.split;
        out.carve.cells += carve.cells;
        out.serial.body_cells += serial.body_cells;
        out.serial.interface_cells += serial.interface_cells;
        out.serial.collision_cells += serial.collision_cells;
    }
    let batched = check_batch(&case.items, limits, &run_batched(&case.items))
        .map_err(|e| format!("batched: {e}"))?;
    for s in batched {
        out.batched.roles += s.roles;
        out.batched.body_cells += s.body_cells;
        out.batched.interface_cells += s.interface_cells;
        out.batched.territory_removed += s.territory_removed;
    }
    Ok(out)
}

#[test]
fn proptest_tree_carve_and_roles() {
    let run = PropertyRun::new(ROLES_ID, ROLES_TEST).expect("M-TREE-ROLES context");
    let limits = checked_roles_limits(&live_bounds(&run, &ROLES_BOUND_KEYS))
        .expect("tree roles bound consistency");
    let strategy = roles_strategy(&limits, ROLES_MODES.to_vec());

    let calls = Cell::new(0_u64);
    let w_body = Cell::new(0_u64);
    let w_roof = Cell::new(0_u64);
    let w_collision = Cell::new(0_u64);
    let w_territory = Cell::new(0_u64);
    let w_swallowed = Cell::new(0_u64);
    let w_split = Cell::new(0_u64);
    let w_free = Cell::new(0_u64);
    let w_gated = Cell::new(0_u64);
    let w_batch = Cell::new(0_u64);
    let items = Cell::new(0_u64);
    let body_cells = Cell::new(0_u64);
    let interface_cells = Cell::new(0_u64);
    let territory_removed = Cell::new(0_u64);
    let swallowed = Cell::new(0_u64);
    let split = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 4] = Default::default();
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let w = match roles_witness_observations(&limits) {
            Ok(w) => w,
            Err(message) => return fail(message),
        };
        w_body.set(w_body.get() + w.body_cells);
        w_roof.set(w_roof.get() + w.roof_cells);
        w_collision.set(w_collision.get() + w.collision_cells);
        w_territory.set(w_territory.get() + w.territory_removed);
        w_swallowed.set(w_swallowed.get() + w.swallowed);
        w_split.set(w_split.get() + w.split);
        w_free.set(w_free.get() + w.no_collision_identity);
        w_gated.set(w_gated.get() + w.gated_identity);
        w_batch.set(w_batch.get() + w.batch_items);

        let stats = match check_roles_case(&case, &limits) {
            Ok(stats) => stats,
            Err(e) => return fail(format!("{:?} case failed: {e}", case.mode)),
        };
        match case.mode {
            RolesMode::Split | RolesMode::Territory if stats.carve.split == 0 => {
                return fail("split case lost its split region");
            }
            RolesMode::Swallow if stats.carve.swallowed == 0 => {
                return fail("swallow case lost its swallowed region");
            }
            RolesMode::NoCollision if stats.serial.collision_cells != 0 => {
                return fail("no-collision case has collision cells");
            }
            _ => {}
        }
        let slot = ROLES_MODES
            .iter()
            .position(|m| *m == case.mode)
            .expect("mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        items.set(items.get() + case.items.len() as u64);
        body_cells.set(body_cells.get() + stats.batched.body_cells);
        interface_cells.set(interface_cells.get() + stats.batched.interface_cells);
        territory_removed.set(territory_removed.get() + stats.batched.territory_removed);
        swallowed.set(swallowed.get() + stats.carve.swallowed);
        split.set(split.get() + stats.carve.split);
        Ok(())
    })
    .expect("M-TREE-ROLES carve, serial and batched role cells");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(w_body.get() > 0, "witness body cells unobserved");
    assert!(w_roof.get() > 0, "witness interface cells unobserved");
    assert!(
        w_collision.get() > 0,
        "witness collision exclusion unobserved"
    );
    assert!(
        w_territory.get() > 0,
        "witness own-territory clipping unobserved"
    );
    assert!(w_swallowed.get() > 0, "witness swallowed region unobserved");
    assert!(w_split.get() > 0, "witness split region unobserved");
    assert!(w_free.get() > 0, "witness no-collision identity unobserved");
    assert!(w_gated.get() > 0, "witness bbox-gated identity unobserved");
    assert!(w_batch.get() > 0, "witness batch association unobserved");
    println!(
        "M-TREE-ROLES accepted_calls={} split={} swallow={} no_collision={} territory={} \
         items={} body_cells={} interface_cells={} territory_removed={} swallowed={} \
         split_regions={} witness_body={} witness_roof={} witness_collision={} \
         witness_territory={} witness_swallowed={} witness_split={} witness_no_collision={} \
         witness_gated={} witness_batch_items={}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        items.get(),
        body_cells.get(),
        interface_cells.get(),
        territory_removed.get(),
        swallowed.get(),
        split.get(),
        w_body.get(),
        w_roof.get(),
        w_collision.get(),
        w_territory.get(),
        w_swallowed.get(),
        w_split.get(),
        w_free.get(),
        w_gated.get(),
        w_batch.get()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Deterministic mutation controls (both checkers)
// ═════════════════════════════════════════════════════════════════════════════

fn expect_smooth_failure(
    scene: &SmoothScene,
    limits: &SmoothLimits,
    observed: &NodeArena,
    what: &str,
    matches_class: impl Fn(&SmoothFailure) -> bool,
) {
    match check_smoothing(scene, limits, observed) {
        Ok(_) => panic!("{what}: the mutation was accepted"),
        Err(e) => assert!(
            matches_class(&e),
            "{what}: rejected for the wrong reason: {e}"
        ),
    }
}

fn expect_roles_failure(
    result: Result<impl fmt::Debug, RoleFailure>,
    what: &str,
    matches_class: impl Fn(&RoleFailure) -> bool,
) {
    match result {
        Ok(stats) => panic!("{what}: the mutation was accepted ({stats:?})"),
        Err(e) => assert!(
            matches_class(&e),
            "{what}: rejected for the wrong reason: {e}"
        ),
    }
}

/// A 0.2 mm square centred on a millimetre point (model coordinates).
fn speck(x_mm: f32, y_mm: f32) -> ExPolygon {
    let c = Point2::from_mm(x_mm, y_mm);
    let h = 1_000;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: c.x - h,
                    y: c.y - h,
                },
                Point2 {
                    x: c.x + h,
                    y: c.y - h,
                },
                Point2 {
                    x: c.x + h,
                    y: c.y + h,
                },
                Point2 {
                    x: c.x - h,
                    y: c.y + h,
                },
            ],
        },
        holes: Vec::new(),
    }
}

fn with_speck(
    roles: &[SupportPlanRoleRegion],
    role: SupportPlanRole,
    x_mm: f32,
    y_mm: f32,
) -> Vec<SupportPlanRoleRegion> {
    let mut out = roles.to_vec();
    out.iter_mut()
        .find(|r| r.role == role)
        .expect("named role present")
        .regions
        .push(speck(x_mm, y_mm));
    out
}

fn invalid_bounds<T: fmt::Debug>(label: &str, result: Result<T, DomainError>) {
    match result {
        Err(DomainError::InvalidBounds(message)) => assert!(!message.is_empty(), "{label}"),
        other => panic!("{label}: expected InvalidBounds, got {other:?}"),
    }
}

#[test]
fn tree_planner_controls() {
    // ── Smoothing ──────────────────────────────────────────────────────────
    let sl = checked_smooth_limits(&smooth_probe_bounds()).expect("smooth probe bounds");

    // A straight valid three-node chain is accepted before any mutation.
    let line = line_scene("control-line", 3, (1_000, 0), WITNESS_WIDTH_UNITS);
    let observed = run_smooth(&line);
    let stats = check_smoothing(&line, &sl, &observed).expect("unmodified straight chain");
    assert_eq!(stats.straight_interior, 1);
    let [bottom, mid, top] = [
        line.chains[0].ids[0],
        line.chains[0].ids[1],
        line.chains[0].ids[2],
    ];
    assert_eq!(observed[mid].position, Point2 { x: 1_000, y: 0 });
    assert_eq!(observed[mid].movement, Point2 { x: 1_000, y: 0 });

    // Pinned endpoint moved to a different grid point.
    for end in [bottom, top] {
        let mut mutant = clone_arena(&observed);
        mutant[end].position.x += 1;
        expect_smooth_failure(&line, &sl, &mutant, "moved endpoint", |e| {
            matches!(e, SmoothFailure::EndpointMoved(_))
        });
    }
    // One inspected parent index set to the arena length.
    let len = observed.nodes.len();
    let mut mutant = clone_arena(&observed);
    mutant[bottom].parent = Some(NodeId(len));
    expect_smooth_failure(&line, &sl, &mutant, "parent = arena length", |e| {
        matches!(e, SmoothFailure::InvalidReference(_))
    });
    let mut mutant = clone_arena(&observed);
    mutant[bottom].parents[0] = NodeId(len);
    expect_smooth_failure(&line, &sl, &mutant, "parents[0] = arena length", |e| {
        matches!(e, SmoothFailure::InvalidReference(_))
    });
    // A rewired (but in-range) link is a separate class.
    let mut mutant = clone_arena(&observed);
    mutant[bottom].parent = Some(top);
    expect_smooth_failure(&line, &sl, &mutant, "rewired parent", |e| {
        matches!(e, SmoothFailure::LinkChanged(_))
    });
    // An interior node pushed out of the hull.
    let mut mutant = clone_arena(&observed);
    mutant[mid].position.x = 2_001;
    expect_smooth_failure(&line, &sl, &mutant, "interior outside hull", |e| {
        matches!(e, SmoothFailure::OutsideHull(_))
    });
    let mut mutant = clone_arena(&observed);
    mutant[mid].position.x = 999;
    expect_smooth_failure(
        &line,
        &sl,
        &mutant,
        "straight chain not a fixed point",
        |e| matches!(e, SmoothFailure::StraightMismatch(_)),
    );

    // Merge: clear need_extra_wall on the committed two-parent interior node.
    let (merge, ids) = merge_scene(MergeVariant::Merge);
    let observed = run_smooth(&merge);
    let stats = check_smoothing(&merge, &sl, &observed).expect("unmodified merge scene");
    assert!(stats.merge_flags > 0);
    assert!(observed[ids.c].need_extra_wall && observed[ids.c].parents.len() == 2);
    assert!(
        observed[ids.c].is_processed,
        "c is a committed interior node"
    );
    let mut mutant = clone_arena(&observed);
    mutant[ids.c].need_extra_wall = false;
    expect_smooth_failure(&merge, &sl, &mutant, "cleared merge flag", |e| {
        matches!(e, SmoothFailure::Flag(_))
    });
    let mut mutant = clone_arena(&observed);
    mutant[ids.c].parents.push(NodeId(len + 100));
    expect_smooth_failure(&merge, &sl, &mutant, "merge parents out of range", |e| {
        matches!(e, SmoothFailure::InvalidReference(_))
    });

    // Signed threshold: a spurious flag on a negative large movement fails.
    let neg = line_scene("control-negative", 4, (-3_000, 0), WITNESS_WIDTH_UNITS);
    let observed = run_smooth(&neg);
    check_smoothing(&neg, &sl, &observed).expect("unmodified negative-step chain");
    let mut mutant = clone_arena(&observed);
    mutant[neg.chains[0].ids[1]].need_extra_wall = true;
    expect_smooth_failure(&neg, &sl, &mutant, "flag on a negative component", |e| {
        matches!(e, SmoothFailure::Flag(_))
    });

    // Short chain: any change fails.
    let short = short_scene();
    let observed = run_smooth(&short);
    check_smoothing(&short, &sl, &observed).expect("unmodified short scene");
    let mut mutant = clone_arena(&observed);
    mutant[short.chains[0].ids[1]].movement.y += 1;
    expect_smooth_failure(&short, &sl, &mutant, "short chain movement", |e| {
        matches!(e, SmoothFailure::ShortChainChanged(_))
    });

    // Smoothing bound matrix.
    let valid = smooth_probe_bounds();
    for key in SMOOTH_BOUND_KEYS {
        let mut bounds = valid.clone();
        bounds.remove(key);
        invalid_bounds(&format!("missing {key}"), checked_smooth_limits(&bounds));
    }
    for (key, value) in [
        ("max_abs_units", 19_999),
        ("max_abs_units", 1_000_001),
        ("max_nodes", 5),
        ("max_layers", 4),
        ("max_layers", 257),
        ("max_width_units", 3_998),
        ("max_width_units", 4_001),
        ("max_width_units", 20_002),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        invalid_bounds(&format!("{key}={value}"), checked_smooth_limits(&bounds));
    }
    let mut bounds = valid.clone();
    bounds.insert("max_abs_units".to_owned(), i64::MAX as u64 + 1);
    assert!(matches!(
        checked_smooth_limits(&bounds),
        Err(DomainError::ArithmeticOverflow)
    ));
    let mut bounds = valid;
    bounds.insert("max_nodes".to_owned(), u64::from(u32::MAX) + 1);
    assert!(matches!(
        checked_smooth_limits(&bounds),
        Err(DomainError::ArithmeticOverflow)
    ));
    smooth_witness_observations(&sl).expect("smoothing witnesses inside the probe bounds");

    // ── Carve and roles ────────────────────────────────────────────────────
    let rl = checked_roles_limits(&roles_probe_bounds()).expect("roles probe bounds");

    // Non-simplifying domain, measured on the production function.
    for avg in 0..=NON_SIMPLIFYING_MAX_AVG {
        for width in [1.0 / 64.0, 0.4, 2.0_f32] {
            for is_base in [true, false] {
                assert_eq!(
                    role_simplify_tolerance(is_base, avg, width),
                    None,
                    "avg={avg}"
                );
            }
        }
    }
    assert_eq!(role_simplify_tolerance(true, 201, 0.4), Some(2_000.0));
    assert_eq!(role_simplify_tolerance(false, 201, 0.4), None);

    let a = witness_item(0, false);
    let roles = a.run_serial();
    check_roles(&a, &rl, &roles, false).expect("unmodified serial witness");
    let body = SupportPlanRole::SupportBody;
    // Missing carve, wrong retained fragment, missing body subtraction.
    expect_roles_failure(
        check_roles(&a, &rl, &with_speck(&roles, body, 7.5, 1.0), false),
        "body at (7.5,1)",
        |e| matches!(e, RoleFailure::Collision(_)),
    );
    expect_roles_failure(
        check_roles(&a, &rl, &with_speck(&roles, body, 8.5, 1.0), false),
        "body at (8.5,1)",
        |e| matches!(e, RoleFailure::Membership(_)),
    );
    expect_roles_failure(
        check_roles(&a, &rl, &with_speck(&roles, body, 4.0, 4.0), false),
        "body at (4,4)",
        |e| matches!(e, RoleFailure::Overlap(_)),
    );
    // The roof really does retain (4,4).
    assert_eq!(
        located(
            role_regions_of(&roles, SupportPlanRole::TopInterface),
            4.0,
            4.0
        ),
        PointLocation::Inside
    );
    // Role order and absent-empty-role structure.
    let mut swapped = roles.clone();
    swapped.swap(0, 1);
    expect_roles_failure(
        check_roles(&a, &rl, &swapped, false),
        "swapped role order",
        |e| matches!(e, RoleFailure::RoleOrder(_)),
    );
    let mut padded = roles.clone();
    padded.push(SupportPlanRoleRegion {
        role: SupportPlanRole::BottomInterface,
        regions: Vec::new(),
    });
    expect_roles_failure(
        check_roles(&a, &rl, &padded, false),
        "empty role present",
        |e| matches!(e, RoleFailure::RoleOrder(_)),
    );
    let roof_only: Vec<SupportPlanRoleRegion> = roles[1..].to_vec();
    expect_roles_failure(
        check_roles(&a, &rl, &roof_only, false),
        "body omitted",
        |e| matches!(e, RoleFailure::RoleOrder(_)),
    );
    expect_roles_failure(check_roles(&a, &rl, &[], false), "all-empty serial", |e| {
        *e == RoleFailure::FalseEmpty
    });

    // Carve output mutations.
    let carve = a.run_carve();
    check_carve(&a, &rl, &carve).expect("unmodified carve");
    let mut wrong = carve.clone();
    wrong[0] = carve_emitted_regions(
        &[a.sut_rect(mm_rect(0, 10, 0, 10))],
        &[a.sut_rect(mm_rect(1, 8, 0, 10))],
    )
    .remove(0);
    expect_roles_failure(
        check_carve(&a, &rl, &wrong),
        "smaller fragment retained",
        |e| matches!(e, RoleFailure::Membership(_)),
    );
    let mut uncarved = carve.clone();
    uncarved[0] = a.sut_rect(mm_rect(0, 10, 0, 10));
    expect_roles_failure(check_carve(&a, &rl, &uncarved), "missing carve", |e| {
        matches!(e, RoleFailure::Collision(_))
    });
    expect_roles_failure(
        check_carve(&a, &rl, &carve[..1]),
        "dropped carve output",
        |e| matches!(e, RoleFailure::Association(_)),
    );

    // Batched own territory and association.
    let items = [witness_item(0, true), witness_item(20 * MM, true)];
    let outputs = run_batched(&items);
    check_batch(&items, &rl, &outputs).expect("unmodified batched witness");
    let mut reinserted = outputs.clone();
    reinserted[0] = with_speck(&outputs[0], body, 6.5, 1.0);
    expect_roles_failure(
        check_batch(&items, &rl, &reinserted),
        "body at (6.5,1)",
        |e| matches!(e, RoleFailure::Territory(_)),
    );
    let mut swapped = outputs.clone();
    swapped.swap(0, 1);
    expect_roles_failure(
        check_batch(&items, &rl, &swapped),
        "swapped batch items",
        |e| matches!(e, RoleFailure::Membership(_)),
    );
    let empty = vec![Vec::new(); items.len()];
    expect_roles_failure(check_batch(&items, &rl, &empty), "all-empty batch", |e| {
        *e == RoleFailure::FalseEmpty
    });
    expect_roles_failure(
        check_batch(&items, &rl, &outputs[..1]),
        "dropped batch item",
        |e| matches!(e, RoleFailure::Association(_)),
    );
    // The serial seam has no territory argument: its unclipped body would be
    // rejected as a batched own-territory observation.
    let serial_as_batched = vec![items[0].run_serial(), items[1].run_serial()];
    expect_roles_failure(
        check_batch(&items, &rl, &serial_as_batched),
        "territory clipping omitted",
        |e| matches!(e, RoleFailure::Territory(_)),
    );

    // Roles bound matrix.
    let valid = roles_probe_bounds();
    for key in ROLES_BOUND_KEYS {
        let mut bounds = valid.clone();
        bounds.remove(key);
        invalid_bounds(&format!("missing {key}"), checked_roles_limits(&bounds));
    }
    for (key, value) in [
        ("max_abs_units", 299_999),
        ("max_abs_units", 1_000_001),
        ("max_regions", 2),
        ("max_regions", 33),
        ("max_cells", 120),
        ("max_batch_items", 1),
        ("max_batch_items", 17),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        invalid_bounds(&format!("{key}={value}"), checked_roles_limits(&bounds));
    }
    let mut bounds = valid;
    bounds.insert("max_cells".to_owned(), u64::from(u32::MAX) + 1);
    assert!(matches!(
        checked_roles_limits(&bounds),
        Err(DomainError::ArithmeticOverflow)
    ));
    roles_witness_observations(&rl).expect("role witnesses inside the probe bounds");
    println!("tree_planner_controls: every mutation rejected by its shared checker");
}

// ═════════════════════════════════════════════════════════════════════════════
// Shrink probe
// ═════════════════════════════════════════════════════════════════════════════

fn check_smooth_shrink_state(case: &SmoothCase, mode: SmoothMode, l: &SmoothLimits) {
    assert_eq!(case.mode, mode, "the smoothing mode must survive shrinking");
    assert!(!case.chains.is_empty() && case.chains.len() <= MAX_CHAINS);
    assert!(case.layers >= 1 && case.layers <= l.max_layers);
    assert!(case.width_units >= 2 && case.width_units <= l.max_width_units);
    let total: usize = case.chains.iter().map(|c| c.positions.len()).sum();
    assert!(total <= l.max_nodes, "{total} nodes exceed max_nodes");
    for (j, chain) in case.chains.iter().enumerate() {
        let n = chain.positions.len();
        let want = match (mode, j) {
            (SmoothMode::Straight, _) => Some(ChainKind::Straight),
            (SmoothMode::Bent, _) | (SmoothMode::Mixed, 0) => Some(ChainKind::Bent),
            (SmoothMode::Short, _) => Some(ChainKind::Short),
            (SmoothMode::Mixed, _) => None,
        };
        if let Some(kind) = want {
            assert_eq!(chain.kind, kind, "chain {j} kind changed");
        }
        match chain.kind {
            ChainKind::Short => assert!((1..=2).contains(&n)),
            _ => assert!(n >= 3, "long chain {j} has {n} nodes"),
        }
        assert!(chain.base_level + n <= case.layers);
        assert_eq!(chain.step.is_some(), chain.kind == ChainKind::Straight);
        assert!(chain
            .positions
            .iter()
            .chain(&chain.movements)
            .all(|(x, y)| x.abs() <= l.max_abs_units && y.abs() <= l.max_abs_units));
        assert!(chain.radii_q.iter().all(|q| *q >= 1));
        assert_eq!(chain.radii_q.len(), n);
        assert_eq!(chain.flags.len(), n);
    }
    let scene = scene_from_case("shrink", case);
    let observed = run_smooth(&scene);
    let stats = check_smoothing(&scene, l, &observed)
        .unwrap_or_else(|e| panic!("shrunk {mode:?} smoothing scene: {e}"));
    match mode {
        SmoothMode::Straight => assert!(stats.straight_interior > 0),
        SmoothMode::Bent | SmoothMode::Mixed => assert!(stats.bent_interior > 0),
        SmoothMode::Short => assert!(stats.short_nodes > 0 && stats.interior == 0),
    }
}

fn smooth_complexity(case: &SmoothCase, l: &SmoothLimits) -> i128 {
    let m = l.max_abs_units;
    let mut total = i128::from(case.width_units) + case.layers as i128;
    for chain in &case.chains {
        total += (chain.positions.len() + chain.base_level) as i128;
        for (x, y) in &chain.movements {
            total += i128::from(x + m) + i128::from(y + m);
        }
        total += chain.radii_q.iter().map(|q| i128::from(*q)).sum::<i128>();
        total += chain.flags.iter().filter(|f| **f).count() as i128;
        match chain.step {
            Some((sx, sy)) => total += i128::from(sx + 2 * m) + i128::from(sy + 2 * m),
            None => {
                for (x, y) in &chain.positions {
                    total += i128::from(x + m) + i128::from(y + m);
                }
            }
        }
    }
    total
}

fn check_roles_shrink_state(case: &RolesCase, mode: RolesMode, l: &RolesLimits) {
    assert_eq!(case.mode, mode, "the roles mode must survive shrinking");
    assert!(!case.items.is_empty() && case.items.len() <= l.max_batch_items);
    for (i, item) in case.items.iter().enumerate() {
        let model = model_item(item, l).unwrap_or_else(|e| panic!("item {i}: {e}"));
        assert!(!item.regions[0].is_empty(), "item {i} lost region 0");
        let r0 = item.regions[0][0];
        let pieces = surviving_intervals(r0[0], r0[1], &item.bars);
        match mode {
            RolesMode::Split | RolesMode::Territory => {
                assert!(pieces.len() >= 2, "item {i} region 0 is no longer split");
            }
            RolesMode::Swallow => assert!(pieces.is_empty(), "item {i} region 0 survives"),
            RolesMode::NoCollision => assert!(item.bars.is_empty()),
        }
        assert_eq!(item.territory.is_some(), mode == RolesMode::Territory);
        assert!(model.swallowed_regions <= item.all_regions().len() as u64);
    }
    let stats = check_roles_case(case, l).unwrap_or_else(|e| panic!("shrunk {mode:?}: {e}"));
    match mode {
        RolesMode::Split | RolesMode::Territory => assert!(stats.carve.split > 0),
        RolesMode::Swallow => assert!(stats.carve.swallowed > 0),
        RolesMode::NoCollision => assert_eq!(stats.serial.collision_cells, 0),
    }
}

fn roles_complexity(case: &RolesCase, l: &RolesLimits) -> i128 {
    let m = i128::from(l.max_abs_units);
    let mut total = case.items.len() as i128;
    for item in &case.items {
        total += i128::from(item.transpose) + item.avg_node_per_layer as i128;
        total += (item.line_width_mm * 64.0) as i128;
        for r in item.all_regions().iter().chain(item.territory.iter()) {
            total += 1 + r.iter().map(|v| i128::from(*v) + m).sum::<i128>();
        }
        for b in &item.bars {
            total += 1 + i128::from(b[0]) + i128::from(b[1]) + 2 * m;
        }
    }
    total
}

/// Real `simplify`/`complicate` transitions on one nonshrinking-mode tree.
fn probe_tree<T, S>(
    label: &str,
    strategy: &S,
    validate: impl Fn(&T),
    complexity: impl Fn(&T) -> i128,
) where
    T: Clone + PartialEq + fmt::Debug,
    S: Strategy<Value = T>,
{
    // Phase 1: accept every simplification (as if each still failed).
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    let initial = tree.current();
    validate(&initial);
    let initial_complexity = complexity(&initial);
    let mut accepted = 0_usize;
    while tree.simplify() {
        accepted += 1;
        assert!(
            accepted < MAX_SHRINK_STEPS,
            "{label}: simplify never exhausted"
        );
        validate(&tree.current());
    }
    let minimal_complexity = complexity(&tree.current());
    assert!(accepted > 0, "{label}: false-only shrink probe");
    assert!(
        minimal_complexity < initial_complexity,
        "{label}: no permitted reduction ({initial_complexity} -> {minimal_complexity})"
    );

    // Phase 2: alternate simplify/complicate, including failed transitions
    // and repeated complicate.
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    assert_eq!(
        tree.current(),
        initial,
        "{label}: the deterministic start is stable"
    );
    let (mut simplified, mut complicated, mut changed) = (0_usize, 0_usize, false);
    while tree.simplify() {
        simplified += 1;
        assert!(
            simplified < MAX_SHRINK_STEPS,
            "{label}: alternation never ended"
        );
        let current = tree.current();
        validate(&current);
        changed |= current != initial;
        if tree.complicate() {
            complicated += 1;
        }
        validate(&tree.current());
    }
    assert!(simplified > 0 && changed, "{label}: no real transition");
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
        validate(&tree.current());
    }
    let settled = tree.current();
    assert!(!tree.complicate(), "{label}: complicate after exhaustion");
    assert_eq!(
        tree.current(),
        settled,
        "{label}: a false complicate moved the value"
    );
    println!(
        "{label} phase1_simplify={accepted} complexity {initial_complexity}->{minimal_complexity} \
         phase2_simplify={simplified} complicate={complicated} restores={restores}"
    );
}

#[test]
fn tree_planner_shrink_bounds() {
    let sl = checked_smooth_limits(&smooth_probe_bounds()).expect("smooth shrink bounds");
    for mode in SMOOTH_MODES {
        let strategy = smooth_strategy(&sl, vec![mode]);
        probe_tree(
            &format!("smooth {mode:?}"),
            &strategy,
            |case: &SmoothCase| check_smooth_shrink_state(case, mode, &sl),
            |case: &SmoothCase| smooth_complexity(case, &sl),
        );
    }
    let rl = checked_roles_limits(&roles_probe_bounds()).expect("roles shrink bounds");
    for mode in ROLES_MODES {
        let strategy = roles_strategy(&rl, vec![mode]);
        probe_tree(
            &format!("roles {mode:?}"),
            &strategy,
            |case: &RolesCase| check_roles_shrink_state(case, mode, &rl),
            |case: &RolesCase| roles_complexity(case, &rl),
        );
    }

    // The mode selectors themselves never shrink.
    let mut runner = TestRunner::deterministic();
    let modes = prop::sample::select(SMOOTH_MODES.to_vec()).no_shrink();
    let mut tree = modes.new_tree(&mut runner).expect("smooth mode tree");
    let before = tree.current();
    assert!(
        !tree.simplify() && !tree.complicate(),
        "the smoothing mode selector moved"
    );
    assert_eq!(tree.current(), before);
    let modes = prop::sample::select(ROLES_MODES.to_vec()).no_shrink();
    let mut tree = modes.new_tree(&mut runner).expect("roles mode tree");
    let before = tree.current();
    assert!(
        !tree.simplify() && !tree.complicate(),
        "the roles mode selector moved"
    );
    assert_eq!(tree.current(), before);
}
