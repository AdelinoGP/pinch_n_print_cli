//! Charter `H-BOOL-SCAD`: scripted SCAD contour output as Boolean stimulus.
//!
//! `scripted_cases` (feature `slicer-sdk/property_testing_scad`) evaluates a
//! typed recipe through the public OpenRSCAD adapter and returns the
//! **converted** contour forest. Those contours are stimulus, never an expected
//! answer: the engine, the emitter and the adapter's own reference module are
//! not consulted here. The converted forest is passed to the same four
//! production wrappers the direct charter uses
//! (`slicer_core::polygon_ops::{union, intersection, difference, xor}`) as the
//! subject operand, together with one controlled integer rectangle operand
//! derived from the contours' own bounding box.
//!
//! Reference: #2 `point_in_region`, an exact `BigRational` point-in-region
//! predicate, on deterministic lattice samples whose Euclidean distance from
//! **every** edge of both inputs is at least [`MARGIN`] canonical units. At that
//! distance the expected membership is exact rather than merely probable:
//! Clipper rounds a new intersection vertex by at most `sqrt(1/2) < 1` unit
//! (see `super::envelopes`), so an output edge stays further than `MARGIN - 1`
//! units away from a sample that clears every input edge by [`MARGIN`]. A
//! `Boundary` classification of any sample against any output is therefore a
//! failure, not an ambiguity.
//!
//! Limitation: the sampled family proves membership only at the observed
//! points. A wrong region that contains no sample is not certified, and only
//! the rectangle family of the direct charter has a *complete* cell-partition
//! model. The populations the charter names — one sample inside the scripted
//! operand, one inside the rectangle only, one outside both, plus one material
//! witness per scripted component **and one interior witness per hole** — are
//! forced into the sample vector before the budget is filled, in the same
//! component-then-hole order and with the same over-cap error as
//! `super::boolean::sample_points`, so the check can never pass by observing
//! nothing and a strided fill can never stride a component or a hole out of
//! the population. A component or hole with no integer witness is accepted only
//! after its whole integer bounding box is enumerated (see below).
//!
//! One input class is **classified rather than asserted**: #4's catalog branch
//! emits a fraction of its contour leaves at one- to two-unit feature size, and
//! such a shape has no integer point at [`MARGIN`] distance from its own edges
//! (measured: the first pilot case was the triangle `(-1,0), (0,0), (1,1)`, whose
//! inradius is 0.215 units). No integer-grid oracle can observe it, so a
//! component with no witness is accepted only after its **entire integer
//! bounding box** is enumerated and found to hold no observable material sample.
//! That proof is bounded by [`MAX_EXHAUSTIVE_POINTS`] (a larger box raises an
//! error instead of weakening the claim), it is counted in
//! [`MembershipStats::proved_unobservable`], it is printed per case, the
//! observable components in the same case must still supply their own witnesses,
//! and the exterior/rectangle populations are still asserted. Nothing is ever
//! discarded silently.
//!
//! `max_planes` is read and validated although the contour family has no slice
//! planes: the registry schema assigns that key to both scripted records
//! (`packet.spec.md`, "Planned targets, registry and profiles"), so both
//! modules consume it rather than leaving a declared bound unread.
//!
//! ## Run-level non-vacuity
//!
//! A case whose every component is `proved_unobservable` is still a checked
//! case: it is counted in [`MembershipStats`] and it can never reject a dropped
//! body, because the integer grid cannot see one. Such a case is therefore
//! *honest but empty*, and a run made only of them would pass a SUT that ignored
//! the operand entirely. To refuse that outcome without spending rejection
//! budget (which would hide the generator scale problem behind Proptest's reject
//! accounting), the module counts the checked cases that observed at least one
//! operand component and, after the run succeeds, fails loudly if that count is
//! zero. The counter is reset at the start of [`run_property`]. Because the
//! registry profiles use fork isolation, the callbacks run in a forked child
//! whose memory the controller cannot read, so the count is also written to a
//! one-line handoff file in the run's `artifact_dir` — the same
//! cross-process channel the harness itself uses for `observations.jsonl`
//! (`docs/testing/properties.md`).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Union};
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_scad::{ScadEvalOptions, ScadGeometry, ScadLimits};
use slicer_sdk::test_support::property_testing::domains::{
    BoundaryRule, GeometryBounds, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::scripted::{
    emit_scad, evaluate_recipe, scripted_cases, Recipe2d, RecipeBounds, ScadRecipe, ScriptedCase,
    ScriptedDomain,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::boolean::{validate_general_output_rings, BoolOutputs};
use super::bounds::{at_least, in_range, read, to_i64, to_u32, to_usize, BoundsError};
use super::envelopes::{point_clears_segment, UNITS_PER_MM};
use super::rect::{edges, rings, BoolOp, Rect};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-BOOL-SCAD";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_scripted_boolean_membership";

/// Minimum Euclidean clearance, in canonical units, of every membership sample
/// from every edge of both input operands.
///
/// Clipper's output vertices move by at most `sqrt(1/2) < 1` unit from the exact
/// intersection (`super::envelopes::ROUNDED_POINT_MAX_EUCLIDEAN_SQ_NUMERATOR`),
/// so an output boundary is more than `MARGIN - 1 = 1` unit away from every
/// accepted sample and the classification is not a boundary question.
pub const MARGIN: i64 = 2;

/// Fixed adapter RNG seed, so a retained `ScriptedCase` replays exactly.
pub const SCAD_SEED: u64 = 1_000_003;

/// Scripted planar topology families this charter generates.
///
/// `Convex` is the plain rectangle family, `Holed` is a rectangle minus a
/// faceted disk and `Disconnected` is two separated rectangles; all three are
/// CSG templates of the #4 strategy, and each keeps its class witness through
/// shrinking because the class tag is fixed by the chosen branch.
pub const CLASSES: [TopologyClass; 3] = [
    TopologyClass::Convex,
    TopologyClass::Holed,
    TopologyClass::Disconnected,
];

/// Smallest accepted `max_abs_units`: `scripted_cases` rejects less than 16.
pub const MIN_ABS_UNITS: u64 = 16;
/// Smallest accepted `max_vertices`: the disconnected template needs eight.
pub const MIN_VERTICES: u64 = 8;
/// Smallest accepted `max_holes`: the holed witness needs one.
pub const MIN_HOLES: u64 = 1;
/// Smallest accepted `max_components`: the disconnected witness needs two.
pub const MIN_COMPONENTS: u64 = 2;
/// Smallest accepted `max_queries`: four mandatory populations plus slack.
pub const MIN_QUERIES: u64 = 8;

/// Safety ceiling on the count bounds; #2's catalog caps one operand at 64 rings
/// and the exact predicate is quadratic in ring size.
pub const COUNT_CEILING: u64 = 64;
/// Safety ceiling on `max_recipe_nodes`.
pub const RECIPE_NODES_CEILING: u64 = 4_096;
/// Safety ceiling on `max_recipe_depth`.
pub const RECIPE_DEPTH_CEILING: u64 = 256;
/// Safety ceiling on `max_facets`.
pub const FACETS_CEILING: u64 = 1_024;
/// Safety ceiling on `max_source_bytes`.
pub const SOURCE_BYTES_CEILING: u64 = 1 << 20;
/// Safety ceiling on `max_evaluations`.
pub const EVALUATIONS_CEILING: u64 = 1 << 20;
/// Safety ceiling on the adapter's vertex and triangle budgets.
pub const SCAD_ELEMENTS_CEILING: u64 = 10_000_000;
/// Safety ceiling on `max_queries`.
pub const QUERIES_CEILING: u64 = 4_096;
/// Safety ceiling on `max_planes`.
pub const PLANES_CEILING: u64 = 64;
/// Safety ceiling on `max_shrink_candidates`.
pub const SHRINK_CANDIDATES_CEILING: u64 = 1 << 16;

/// Raw `H-BOOL-SCAD` profile keys, before validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawScadBounds {
    /// `max_abs_units`.
    pub max_abs_units: u64,
    /// `max_vertices`.
    pub max_vertices: u64,
    /// `max_holes`.
    pub max_holes: u64,
    /// `max_components`.
    pub max_components: u64,
    /// `max_shrink_candidates`.
    pub max_shrink_candidates: u64,
    /// `max_recipe_nodes`.
    pub max_recipe_nodes: u64,
    /// `max_recipe_depth`.
    pub max_recipe_depth: u64,
    /// `max_facets`.
    pub max_facets: u64,
    /// `max_source_bytes`.
    pub max_source_bytes: u64,
    /// `max_evaluations`.
    pub max_evaluations: u64,
    /// `max_scad_vertices`.
    pub max_scad_vertices: u64,
    /// `max_scad_triangles`.
    pub max_scad_triangles: u64,
    /// `evaluation_steps`.
    pub evaluation_steps: u64,
    /// `worker_stack_bytes`.
    pub worker_stack_bytes: u64,
    /// `test_process_timeout_ms`; consumed by the xtask executor, validated here.
    pub test_process_timeout_ms: u64,
    /// `max_queries`.
    pub max_queries: u64,
    /// `max_planes`.
    pub max_planes: u64,
}

impl RawScadBounds {
    /// Read every `H-BOOL-SCAD` key from the selected registry profile.
    pub fn read(run: &PropertyRun) -> Result<Self, BoundsError> {
        Ok(Self {
            max_abs_units: read(run, "max_abs_units")?,
            max_vertices: read(run, "max_vertices")?,
            max_holes: read(run, "max_holes")?,
            max_components: read(run, "max_components")?,
            max_shrink_candidates: read(run, "max_shrink_candidates")?,
            max_recipe_nodes: read(run, "max_recipe_nodes")?,
            max_recipe_depth: read(run, "max_recipe_depth")?,
            max_facets: read(run, "max_facets")?,
            max_source_bytes: read(run, "max_source_bytes")?,
            max_evaluations: read(run, "max_evaluations")?,
            max_scad_vertices: read(run, "max_scad_vertices")?,
            max_scad_triangles: read(run, "max_scad_triangles")?,
            evaluation_steps: read(run, "evaluation_steps")?,
            worker_stack_bytes: read(run, "worker_stack_bytes")?,
            test_process_timeout_ms: read(run, "test_process_timeout_ms")?,
            max_queries: read(run, "max_queries")?,
            max_planes: read(run, "max_planes")?,
        })
    }
}

/// Validated `H-BOOL-SCAD` limits, converted without narrowing loss.
#[derive(Debug, Clone, PartialEq)]
pub struct ContourLimits {
    /// Domain bounds for the requested planar class.
    pub geometry: GeometryBounds,
    /// Recipe bounds for emission and evaluation accounting.
    pub recipe: RecipeBounds,
    /// Adapter options, derived from [`ContourLimits::recipe`].
    pub options: ScadEvalOptions,
    /// Largest sample vector per checked operand pair.
    pub max_queries: usize,
    /// Validated but unused by the contour family: see the module doc.
    pub max_planes: usize,
}

/// Reject zero, overflowing and out-of-range scripted bounds before sampling.
///
/// Every narrowing is checked; an invalid profile fails here, before a strategy
/// or a value tree exists, rather than becoming an all-rejected population or a
/// silently truncated limit.
pub fn checked_limits(raw: RawScadBounds) -> Result<ContourLimits, BoundsError> {
    at_least(
        "max_abs_units",
        raw.max_abs_units,
        "scripted contour envelope",
        MIN_ABS_UNITS,
    )?;
    let max_abs_units = to_i64(
        "max_abs_units",
        in_range(
            "max_abs_units",
            raw.max_abs_units,
            MIN_ABS_UNITS..=1_000_000,
        )?,
    )?;
    in_range("max_vertices", raw.max_vertices, 1..=COUNT_CEILING)?;
    at_least(
        "max_vertices",
        raw.max_vertices,
        "disconnected scripted operand",
        MIN_VERTICES,
    )?;
    in_range("max_holes", raw.max_holes, 1..=COUNT_CEILING)?;
    at_least(
        "max_holes",
        raw.max_holes,
        "holed scripted operand",
        MIN_HOLES,
    )?;
    in_range("max_components", raw.max_components, 1..=COUNT_CEILING)?;
    at_least(
        "max_components",
        raw.max_components,
        "disconnected scripted operand",
        MIN_COMPONENTS,
    )?;
    in_range(
        "max_shrink_candidates",
        raw.max_shrink_candidates,
        1..=SHRINK_CANDIDATES_CEILING,
    )?;
    in_range(
        "max_recipe_nodes",
        raw.max_recipe_nodes,
        1..=RECIPE_NODES_CEILING,
    )?;
    in_range(
        "max_recipe_depth",
        raw.max_recipe_depth,
        1..=RECIPE_DEPTH_CEILING,
    )?;
    // The recipe validator requires at least three facets in any faceted leaf.
    in_range("max_facets", raw.max_facets, 3..=FACETS_CEILING)?;
    // The recipe validator requires at least 32 emitted source bytes.
    in_range(
        "max_source_bytes",
        raw.max_source_bytes,
        32..=SOURCE_BYTES_CEILING,
    )?;
    in_range(
        "max_evaluations",
        raw.max_evaluations,
        1..=EVALUATIONS_CEILING,
    )?;
    in_range(
        "max_scad_vertices",
        raw.max_scad_vertices,
        3..=SCAD_ELEMENTS_CEILING,
    )?;
    in_range(
        "max_scad_triangles",
        raw.max_scad_triangles,
        1..=SCAD_ELEMENTS_CEILING,
    )?;
    // `u64::MAX` means "unbounded" to the adapter and is rejected by #4.
    in_range("evaluation_steps", raw.evaluation_steps, 1..=(u64::MAX - 1))?;
    let worker_stack_bytes = to_usize(
        "worker_stack_bytes",
        in_range(
            "worker_stack_bytes",
            raw.worker_stack_bytes,
            slicer_scad::MIN_WORKER_STACK_BYTES as u64..=slicer_scad::MAX_WORKER_STACK_BYTES as u64,
        )?,
    )?;
    // The xtask executor owns this deadline; the property only proves the
    // profile is usable, never that a supervised run happened.
    at_least(
        "test_process_timeout_ms",
        raw.test_process_timeout_ms,
        "whole-process supervision",
        1,
    )?;
    in_range("max_queries", raw.max_queries, 1..=QUERIES_CEILING)?;
    at_least(
        "max_queries",
        raw.max_queries,
        "membership sample population",
        MIN_QUERIES,
    )?;
    in_range("max_planes", raw.max_planes, 1..=PLANES_CEILING)?;

    // exhaustive: pins the registry bound mapping deliberately
    let recipe = RecipeBounds {
        max_nodes: to_usize("max_recipe_nodes", raw.max_recipe_nodes)?,
        max_depth: to_usize("max_recipe_depth", raw.max_recipe_depth)?,
        max_facets: u16::try_from(raw.max_facets).map_err(|_| BoundsError::Overflow {
            name: "max_facets".to_string(),
            value: raw.max_facets,
            target: "u16",
        })?,
        max_source_bytes: to_usize("max_source_bytes", raw.max_source_bytes)?,
        max_evaluations: to_u32("max_evaluations", raw.max_evaluations)?,
    };
    // exhaustive: pins the registry geometry bound mapping deliberately
    let geometry = GeometryBounds {
        max_abs_units,
        max_vertices: to_usize("max_vertices", raw.max_vertices)?,
        max_holes: to_usize("max_holes", raw.max_holes)?,
        max_components: to_usize("max_components", raw.max_components)?,
        max_shrink_candidates: to_u32("max_shrink_candidates", raw.max_shrink_candidates)?,
    };
    let options = adapter_options(
        &recipe,
        max_abs_units,
        to_usize("max_scad_vertices", raw.max_scad_vertices)?,
        to_usize("max_scad_triangles", raw.max_scad_triangles)?,
        raw.evaluation_steps,
        worker_stack_bytes,
    );
    Ok(ContourLimits {
        geometry,
        recipe,
        options,
        max_queries: to_usize("max_queries", raw.max_queries)?,
        max_planes: to_usize("max_planes", raw.max_planes)?,
    })
}

/// Adapter capacities derived exactly as `crates/slicer-sdk/tests/support/scripted.rs`.
///
/// `required_adapter_capacity` is `pub(crate)` in the SDK, so the same
/// derivation is repeated here: eight AST nodes per recipe node plus sixteen of
/// headroom, raised to `max_source_bytes` because a polygon leaf contributes one
/// AST node per vertex and every AST node needs at least one source byte; two
/// AST levels per recipe level plus eight; and one evaluated geometry node per
/// AST node. `scripted_cases` rejects any smaller capacity, so a profile that
/// used a smaller number fails loudly instead of truncating a recipe.
pub fn adapter_options(
    recipe: &RecipeBounds,
    max_abs_units: i64,
    max_scad_vertices: usize,
    max_scad_triangles: usize,
    evaluation_steps: u64,
    worker_stack_bytes: usize,
) -> ScadEvalOptions {
    let nodes = (8 * recipe.max_nodes + 16).max(recipe.max_source_bytes);
    let depth = 2 * recipe.max_depth + 8;
    ScadEvalOptions {
        parameters: Vec::new(),
        extrude_height_mm: None,
        // exhaustive: every derived adapter capacity is pinned from the recipe bound
        limits: ScadLimits {
            evaluation_steps,
            max_source_bytes: recipe.max_source_bytes,
            max_ast_nodes: nodes,
            max_ast_depth: depth,
            max_geometry_nodes: nodes,
            max_facets: u32::from(recipe.max_facets),
            max_vertices: max_scad_vertices,
            max_triangles: max_scad_triangles,
            // One canonical unit is 1e-4 mm; the envelope stays in millimetres.
            max_abs_coordinate_mm: max_abs_units as f64 / UNITS_PER_MM as f64,
            worker_stack_bytes,
        },
        random_seed: Some(SCAD_SEED),
    }
}

/// The scripted domain for one planar topology class.
pub fn domain(class: TopologyClass, limits: &ContourLimits) -> ScriptedDomain {
    ScriptedDomain {
        // exhaustive: pins the registry geometry capacities deliberately
        geometry: GeometryDomain {
            class,
            min_holes: 0,
            min_components: usize::from(class != TopologyClass::Empty),
            boundary: BoundaryRule::Strict,
            bounds: limits.geometry.clone(),
        },
        solid_witness: None,
        bounds: limits.recipe,
    }
}

/// Build the `H-BOOL-SCAD` strategy: one certified scripted case per sample,
/// tagged with the class whose branch produced it.
pub fn strategy(
    limits: &ContourLimits,
) -> Result<BoxedStrategy<(TopologyClass, ScriptedCase)>, BoundsError> {
    let mut branches = Vec::new();
    for class in CLASSES {
        let cases =
            scripted_cases(domain(class, limits), limits.options.clone()).map_err(|error| {
                BoundsError::Arithmetic {
                    what: format!("scripted {class:?} contour domain: {error}"),
                }
            })?;
        branches.push(cases.prop_map(move |case| (class, case)).boxed());
    }
    Ok(Union::new(branches).boxed())
}

// ─────────────────────────────────────────────────────────────────────────────
// Controlled integer rectangle operand.
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a checked `i128` coordinate back to the `i64` grid.
fn grid(value: i128, what: &str) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("{what} {value} does not fit the i64 grid"))
}

/// The controlled rectangle operand derived from the contours' own bounds.
///
/// The rectangle spans the right quarter of the operand's bounding box and
/// sticks out at least one millimetre to its right, so a
/// rectangle-only population always exists: a point with `x > max_x` is further
/// than that margin from every operand edge, because every operand vertex has
/// `x <= max_x` and a segment cannot leave the half-plane of its endpoints.
///
/// An empty operand has no bounds, so a fixed one-millimetre square is used;
/// the operand is then the empty set and only the rectangle populations are
/// required.
pub fn rectangle_operand(operand: &[ExPolygon]) -> Result<ExPolygon, String> {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    let mut any = false;
    for point in rings(operand).flat_map(|ring| ring.points.iter()) {
        any = true;
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    let (min_x, min_y, max_x, max_y) = if any {
        (min_x, min_y, max_x, max_y)
    } else {
        (0, 0, UNITS_PER_MM, UNITS_PER_MM)
    };
    let width = i128::from(max_x) - i128::from(min_x);
    let height = i128::from(max_y) - i128::from(min_y);
    // At least one millimetre of overhang on the right, and a vertical half
    // span of at least four margins so the rectangle's own edges never touch a
    // mandatory sample.
    let overhang = (width / 2).max(i128::from(UNITS_PER_MM));
    let half_height = (height / 2).max(4 * i128::from(MARGIN));
    let centre_y = (i128::from(min_y) + i128::from(max_y)) / 2;
    let rect = Rect::new(
        grid(i128::from(max_x) - width / 4, "rectangle left edge")?,
        grid(centre_y - half_height, "rectangle bottom edge")?,
        grid(i128::from(max_x) + overhang, "rectangle right edge")?,
        grid(centre_y + half_height, "rectangle top edge")?,
    )?;
    Ok(rect.to_expolygon())
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent sample reference.
// ─────────────────────────────────────────────────────────────────────────────

/// Observed populations of the membership reference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MembershipStats {
    /// Samples classified against every operation.
    pub accepted: usize,
    /// Samples inside the scripted operand (with or without the rectangle).
    pub inside_operand: usize,
    /// Samples inside the rectangle and outside the scripted operand.
    pub inside_rectangle_only: usize,
    /// Samples inside both operands.
    pub inside_both: usize,
    /// Samples outside both operands.
    pub outside_both: usize,
    /// Scripted components with an independent material witness sample.
    pub material_witnesses: usize,
    /// Scripted components whose whole integer bounding box was enumerated and
    /// found to hold no observable material sample at [`MARGIN`].
    pub proved_unobservable: usize,
    /// Scripted holes with an independent interior witness sample.
    pub hole_witnesses: usize,
    /// Scripted holes whose whole integer bounding box was enumerated and found
    /// to hold no observable interior sample at [`MARGIN`].
    pub proved_unobservable_holes: usize,
}

fn location(point: Point2, polygons: &[ExPolygon]) -> Result<PointLocation, String> {
    point_in_region(point, polygons).map_err(|error| format!("point_in_region: {error}"))
}

/// Sorted sample coordinates: quarter points of every gap between distinct
/// input coordinates, plus a uniform lattice and a padded exterior point.
fn axis_samples(values: &[i64], pad: i64, divisions: i64) -> Result<Vec<i64>, String> {
    let add = |value: i64, delta: i64| {
        value
            .checked_add(delta)
            .ok_or_else(|| String::from("sample coordinate overflow"))
    };
    let mut unique: Vec<i64> = values.to_vec();
    unique.sort_unstable();
    unique.dedup();
    let mut out = Vec::new();
    match (unique.first(), unique.last()) {
        (Some(&first), Some(&last)) => {
            out.push(add(first, -pad)?);
            out.push(add(last, pad)?);
            for pair in unique.windows(2) {
                let gap = i128::from(pair[1]) - i128::from(pair[0]);
                if gap <= 0 {
                    continue;
                }
                for numerator in [1_i128, 3] {
                    let offset = i64::try_from(gap * numerator / 4)
                        .map_err(|_| String::from("sample gap overflow"))?;
                    if offset > 0 {
                        out.push(add(pair[0], offset)?);
                    }
                }
            }
            // A uniform lattice covers far-field and exterior regions that the
            // gap quarter points do not, without depending on vertex spacing.
            let span = i128::from(last) - i128::from(first);
            for step in 1..divisions {
                let offset = i64::try_from(span * i128::from(step) / i128::from(divisions))
                    .map_err(|_| String::from("lattice overflow"))?;
                out.push(add(first, offset)?);
            }
        }
        _ => {
            out.push(-pad);
            out.push(pad);
        }
    }
    out.extend_from_slice(&unique);
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// Whether `point` clears every edge of both operands by [`MARGIN`].
fn clears_every_edge(point: Point2, all_edges: &[(Point2, Point2)]) -> Result<bool, String> {
    for &(start, end) in all_edges {
        if !point_clears_segment(point, start, end, MARGIN)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Classify `point` against both operands, rejecting any boundary hit.
fn classify(
    point: Point2,
    operand: &[ExPolygon],
    rectangle: &[ExPolygon],
) -> Result<(bool, bool), String> {
    let in_operand = match location(point, operand)? {
        PointLocation::Inside => true,
        PointLocation::Outside => false,
        PointLocation::Boundary => {
            return Err(format!(
                "sample {point:?} is on the scripted operand's boundary: the margin filter failed"
            ))
        }
    };
    let in_rectangle = match location(point, rectangle)? {
        PointLocation::Inside => true,
        PointLocation::Outside => false,
        PointLocation::Boundary => {
            return Err(format!(
                "sample {point:?} is on the rectangle operand's boundary: the margin filter failed"
            ))
        }
    };
    Ok((in_operand, in_rectangle))
}

/// Every clear candidate of both operands plus its membership in each.
fn candidates(
    all: &[ExPolygon],
    all_edges: &[(Point2, Point2)],
    operand: &[ExPolygon],
    rectangle: &[ExPolygon],
) -> Result<Vec<(Point2, bool, bool)>, String> {
    let points: Vec<Point2> = rings(all)
        .flat_map(|ring| ring.points.iter().copied())
        .collect();
    let pad = 4 * MARGIN;
    let xs = axis_samples(
        &points.iter().map(|p| p.x).collect::<Vec<_>>(),
        pad,
        LATTICE_DIVISIONS,
    )?;
    let ys = axis_samples(
        &points.iter().map(|p| p.y).collect::<Vec<_>>(),
        pad,
        LATTICE_DIVISIONS,
    )?;
    let mut out = Vec::new();
    for &y in &ys {
        for &x in &xs {
            let point = Point2 { x, y };
            if !clears_every_edge(point, all_edges)? {
                continue;
            }
            let (in_operand, in_rectangle) = classify(point, operand, rectangle)?;
            out.push((point, in_operand, in_rectangle));
        }
    }
    if out.is_empty() {
        return Err(String::from("no lattice sample clears every input edge"));
    }
    Ok(out)
}

/// Lattice divisions per axis for the uniform far-field lattice.
pub const LATTICE_DIVISIONS: i64 = 12;

/// Upper bound on the exhaustive integer-bbox scan that *proves* a scripted
/// component or hole ring has no observable interior sample.
pub const MAX_EXHAUSTIVE_POINTS: i128 = 1 << 16;

/// One interior witness per observable ring of the scripted operand.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WitnessPopulation {
    /// Components with an interior sample that clears every input edge.
    component_points: Vec<Point2>,
    /// Components whose whole integer bounding box holds no such sample.
    proved_unobservable_components: usize,
    /// Holes with an interior sample that clears every input edge.
    hole_points: Vec<Point2>,
    /// Holes whose whole integer bounding box holds no such sample.
    proved_unobservable_holes: usize,
}

/// Integer bounding box `(min_x, min_y, max_x, max_y)` of every ring of a set.
fn ring_bbox(region: &[ExPolygon]) -> Result<(i64, i64, i64, i64), String> {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for point in rings(region).flat_map(|ring| ring.points.iter()) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    if min_x > max_x || min_y > max_y {
        return Err(String::from("an empty ring set has no bounding box"));
    }
    Ok((min_x, min_y, max_x, max_y))
}

/// Find one integer sample strictly inside `region` that clears every input edge.
///
/// The region is classified with #2 `point_in_region` on its own, so a hole is
/// observed by a point inside the hole ring and a component by a point inside
/// its material. The lattice pool is searched first; only when it holds no such
/// point is the region's **entire integer bounding box** enumerated. `None`
/// means the whole box was enumerated and no observable sample exists, which is
/// a property of the generated input, not of the SUT. The scan is bounded by
/// [`MAX_EXHAUSTIVE_POINTS`], and a larger box raises an error instead of
/// weakening the claim.
fn region_witness(
    region: &[ExPolygon],
    all_edges: &[(Point2, Point2)],
    pool: &[(Point2, bool, bool)],
    label: &str,
) -> Result<Option<Point2>, String> {
    for (point, _, _) in pool {
        if matches!(location(*point, region)?, PointLocation::Inside) {
            return Ok(Some(*point));
        }
    }
    let (min_x, min_y, max_x, max_y) = ring_bbox(region)?;
    let area = (i128::from(max_x) - i128::from(min_x) + 1)
        .checked_mul(i128::from(max_y) - i128::from(min_y) + 1)
        .ok_or_else(|| format!("{label} bounding box overflow"))?;
    if area > MAX_EXHAUSTIVE_POINTS {
        return Err(format!(
            "{label} spans {area} integer points, above the {MAX_EXHAUSTIVE_POINTS}-point \
             exhaustive bound, and the lattice found no interior sample: the sample construction \
             cannot certify an observation population"
        ));
    }
    let mut found = None;
    'scan: for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = Point2 { x, y };
            if !clears_every_edge(point, all_edges)? {
                continue;
            }
            if matches!(location(point, region)?, PointLocation::Inside) {
                found = Some(point);
                break 'scan;
            }
        }
    }
    Ok(found)
}

/// Add one independent interior witness per observable component and per hole.
///
/// Returns the witness population and appends every newly found witness to
/// `pool`, classified against both operands. A ring with no witness is accepted
/// only after its **entire integer bounding box** has been enumerated and no
/// point there both clears every edge of both operands by [`MARGIN`] and lies in
/// that ring's interior — at which point no integer-grid oracle can observe the
/// ring at all. That is a property of the generated input (the #4 catalog emits
/// a fraction of its leaves at one- to two-unit feature size, where the integer
/// grid has no interior point), not of the SUT, so the ring is counted and
/// reported rather than silently skipped. The scan is bounded: a bounding box too
/// large to enumerate raises an error instead of weakening the claim, and a ring
/// whose own bbox is unobservable still leaves the exterior populations
/// asserted. Components and their holes are visited in operand order, and a
/// component's witness precedes its holes' witnesses, the ordering
/// `super::boolean::sample_points` uses.
fn add_material_witnesses(
    operand: &[ExPolygon],
    rectangle: &[ExPolygon],
    all_edges: &[(Point2, Point2)],
    pool: &mut Vec<(Point2, bool, bool)>,
) -> Result<WitnessPopulation, String> {
    let mut population = WitnessPopulation::default();
    for (index, component) in operand.iter().enumerate() {
        let component_region = std::slice::from_ref(component);
        let label = format!("scripted component {index}");
        match region_witness(component_region, all_edges, pool, &label)? {
            Some(point) => {
                let (in_operand, in_rectangle) = classify(point, operand, rectangle)?;
                if !in_operand {
                    return Err(format!(
                        "{label}: witness {point:?} is not inside the operand although it is \
                         inside the component alone"
                    ));
                }
                if !pool.iter().any(|(existing, _, _)| *existing == point) {
                    pool.push((point, in_operand, in_rectangle));
                }
                population.component_points.push(point);
            }
            None => population.proved_unobservable_components += 1,
        }
        for (hole_index, hole) in component.holes.iter().enumerate() {
            // The hole's own ring is the region: a point inside it is material
            // free, so it observes exactly the void a filled hole would occupy.
            let hole_region = [ExPolygon {
                contour: hole.clone(),
                holes: Vec::new(),
            }];
            let label = format!("scripted component {index} hole {hole_index}");
            match region_witness(&hole_region, all_edges, pool, &label)? {
                Some(point) => {
                    let (in_operand, in_rectangle) = classify(point, operand, rectangle)?;
                    if in_operand {
                        return Err(format!(
                            "{label}: witness {point:?} is inside the operand material although it \
                             is inside the hole ring alone"
                        ));
                    }
                    if !pool.iter().any(|(existing, _, _)| *existing == point) {
                        pool.push((point, in_operand, in_rectangle));
                    }
                    population.hole_points.push(point);
                }
                None => population.proved_unobservable_holes += 1,
            }
        }
    }
    Ok(population)
}

/// The full membership reference for one scripted operand and its rectangle.
pub fn check_membership(
    operand: &[ExPolygon],
    rectangle: &[ExPolygon],
    out: &BoolOutputs,
    max_queries: usize,
) -> Result<MembershipStats, String> {
    for op in BoolOp::ALL {
        validate_general_output_rings(&format!("{op:?}"), out.get(op))?;
    }
    let all: Vec<ExPolygon> = operand.iter().chain(rectangle.iter()).cloned().collect();
    let all_edges = edges(&all);
    let mut pool = candidates(&all, &all_edges, operand, rectangle)?;
    let witnesses = add_material_witnesses(operand, rectangle, &all_edges, &mut pool)?;
    let holes: usize = operand.iter().map(|polygon| polygon.holes.len()).sum();
    if witnesses.component_points.len() + witnesses.proved_unobservable_components != operand.len()
    {
        return Err(format!(
            "{} material witness(es) plus {} proved unobservable component(s) do not account for \
             all {} scripted component(s)",
            witnesses.component_points.len(),
            witnesses.proved_unobservable_components,
            operand.len()
        ));
    }
    if witnesses.hole_points.len() + witnesses.proved_unobservable_holes != holes {
        return Err(format!(
            "{} hole witness(es) plus {} proved unobservable hole(s) do not account for all {} \
             scripted hole(s)",
            witnesses.hole_points.len(),
            witnesses.proved_unobservable_holes,
            holes
        ));
    }
    let mut stats = MembershipStats {
        material_witnesses: witnesses.component_points.len(),
        proved_unobservable: witnesses.proved_unobservable_components,
        hole_witnesses: witnesses.hole_points.len(),
        proved_unobservable_holes: witnesses.proved_unobservable_holes,
        ..MembershipStats::default()
    };
    // Every observable ring is forced into the sample vector before the budget
    // is filled, then the four mandatory populations, then a deterministic
    // strided remainder. The budget can therefore never stride a component or a
    // hole out of the population, which is the failure mode the previous
    // representatives-then-stride fill had: every logged case accepted exactly
    // `max_queries` samples while a small body's only witness sat beyond the
    // stride.
    let mut chosen: Vec<(Point2, bool, bool)> = Vec::new();
    for point in witnesses
        .component_points
        .iter()
        .chain(witnesses.hole_points.iter())
    {
        let item = pool
            .iter()
            .find(|(candidate, _, _)| candidate == point)
            .copied()
            .ok_or_else(|| format!("witness {point:?} left the candidate pool"))?;
        if !chosen.iter().any(|(existing, _, _)| existing == point) {
            chosen.push(item);
        }
    }
    let forced_witnesses = chosen.len();
    for (want_operand, want_rectangle) in
        [(true, false), (false, true), (false, false), (true, true)]
    {
        if let Some(found) = pool
            .iter()
            .find(|(_, a, b)| *a == want_operand && *b == want_rectangle)
        {
            if !chosen.iter().any(|(point, _, _)| *point == found.0) {
                chosen.push(*found);
            }
        }
    }
    if chosen.len() > max_queries {
        return Err(format!(
            "{} forced witness sample(s) plus {} mandatory population sample(s) exceed \
             max_queries {max_queries}: the generator bounds admit more observable rings than the \
             profile's query budget can force, so this is a profile/generator mismatch, not a \
             sample to truncate",
            forced_witnesses,
            chosen.len() - forced_witnesses
        ));
    }
    let rest: Vec<(Point2, bool, bool)> = pool
        .iter()
        .copied()
        .filter(|item| !chosen.iter().any(|(point, _, _)| *point == item.0))
        .collect();
    let room = max_queries - chosen.len();
    if rest.len() <= room {
        chosen.extend(rest);
    } else if room > 0 {
        for index in 0..room {
            chosen.push(rest[index * rest.len() / room]);
        }
    }
    for (point, in_operand, in_rectangle) in &chosen {
        stats.accepted += 1;
        stats.inside_operand += usize::from(*in_operand);
        stats.inside_rectangle_only += usize::from(!in_operand && *in_rectangle);
        stats.inside_both += usize::from(*in_operand && *in_rectangle);
        stats.outside_both += usize::from(!in_operand && !in_rectangle);
        for op in BoolOp::ALL {
            let want = op.truth(*in_operand, *in_rectangle);
            let got = location(*point, out.get(op))?;
            let ok = matches!(
                (want, got),
                (true, PointLocation::Inside) | (false, PointLocation::Outside)
            );
            if !ok {
                return Err(format!(
                    "{op:?}: sample {point:?} (in operand: {in_operand}, in rectangle: \
                     {in_rectangle}) expected {} but the output is {got:?}; operand={operand:?} \
                     rectangle={rectangle:?} output={:?}",
                    if want { "Inside" } else { "Outside" },
                    out.get(op)
                ));
            }
        }
    }
    if stats.accepted == 0 || stats.outside_both == 0 || stats.inside_rectangle_only == 0 {
        return Err(format!(
            "vacuous membership population {stats:?} for a {} scripted component set",
            operand.len()
        ));
    }
    // The inside-operand population is required for every component the integer
    // grid can observe; a component proved unobservable at `MARGIN` cannot supply
    // one, and that classification is reported in `stats` instead of passing
    // silently. Every observable component witness is forced above, so this can
    // only fail if the witness machinery and the sample vector disagree.
    if stats.material_witnesses > 0 && stats.inside_operand == 0 {
        return Err(format!(
            "an observable scripted component was never observed inside: {stats:?}"
        ));
    }
    // The hole witnesses are forced too, so a filled hole must have been seen.
    if stats.hole_witnesses > 0 && stats.outside_both + stats.inside_rectangle_only == 0 {
        return Err(format!(
            "an observable scripted hole was never observed outside the operand: {stats:?}"
        ));
    }
    Ok(stats)
}

// ─────────────────────────────────────────────────────────────────────────────
// Run-level non-vacuity accounting.
// ─────────────────────────────────────────────────────────────────────────────

/// Checked cases, in this process, that observed at least one operand component.
static OBSERVABLE_CASES: AtomicUsize = AtomicUsize::new(0);
/// Checked cases, in this process, counted for the non-vacuity guarantee.
static CHECKED_CASES: AtomicUsize = AtomicUsize::new(0);

/// One-line handoff file, under the run's artifact directory, carrying the
/// `checked observable` pair across the fork boundary.
const OBSERVABILITY_HANDOFF: &str = "scad-boolean-observability.txt";

/// The run's artifact directory, read from the harness context.
///
/// `PropertyRun` requires `PNP_PROPERTY_CONTEXT` with a nonempty `artifact_dir`,
/// so this only fails when the harness itself is broken; the callback reports
/// that as a case failure instead of pretending the guarantee was met.
fn artifact_dir() -> Result<PathBuf, String> {
    let raw = std::env::var("PNP_PROPERTY_CONTEXT")
        .map_err(|_| String::from("run-level non-vacuity accounting needs PNP_PROPERTY_CONTEXT"))?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| format!("property context JSON: {error}"))?;
    value
        .get("artifact_dir")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| String::from("property context has no artifact_dir"))
}

/// Reset the counters and drop any handoff left by a previous run.
fn reset_observability() -> Result<(), String> {
    CHECKED_CASES.store(0, Ordering::SeqCst);
    OBSERVABLE_CASES.store(0, Ordering::SeqCst);
    let path = artifact_dir()?.join(OBSERVABILITY_HANDOFF);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not clear {}: {error}", path.display())),
    }
}

/// Record one checked case and hand the count across the fork boundary.
///
/// The registry profiles run their callbacks in a forked child
/// (`isolation = "fork"`), whose memory the controller cannot read, so the
/// count is mirrored to the artifact directory the harness already shares
/// between the two processes.
fn record_case_observability(observable: bool) -> Result<(), String> {
    let checked = CHECKED_CASES.fetch_add(1, Ordering::SeqCst) + 1;
    let observed = if observable {
        OBSERVABLE_CASES.fetch_add(1, Ordering::SeqCst) + 1
    } else {
        OBSERVABLE_CASES.load(Ordering::SeqCst)
    };
    let path = artifact_dir()?.join(OBSERVABILITY_HANDOFF);
    fs::write(&path, format!("{checked} {observed}\n"))
        .map_err(|error| format!("could not record {}: {error}", path.display()))
}

/// `(checked, observed)` from the last recorded case, in any process.
fn read_observability_handoff() -> Option<(usize, usize)> {
    let path = artifact_dir().ok()?.join(OBSERVABILITY_HANDOFF);
    let text = fs::read_to_string(path).ok()?;
    let mut fields = text.split_whitespace();
    Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
}

/// Fail the run when every checked case was fully unobservable.
fn assert_run_non_vacuity() -> Result<(), String> {
    let mut checked = CHECKED_CASES.load(Ordering::SeqCst);
    let mut observed = OBSERVABLE_CASES.load(Ordering::SeqCst);
    if let Some((handoff_checked, handoff_observed)) = read_observability_handoff() {
        checked = checked.max(handoff_checked);
        observed = observed.max(handoff_observed);
    }
    if observed == 0 {
        return Err(format!(
            "run-level non-vacuity: none of the {checked} checked case(s) observed a single operand \
             component at MARGIN {MARGIN}, so a SUT that ignored the operand would have passed. \
             This is a generator/profile mismatch: raise max_abs_units (or the query budget) for \
             this profile rather than accepting a vacuous run"
        ));
    }
    println!(
        "H-BOOL-SCAD run-level non-vacuity: {observed} of {checked} checked case(s) observed at \
         least one operand component"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic witnesses and the property body.
// ─────────────────────────────────────────────────────────────────────────────

/// Check one certified scripted case end to end.
pub fn check_case(
    class: TopologyClass,
    case: &ScriptedCase,
    limits: &ContourLimits,
) -> TestCaseResult {
    // A retained case must reproduce its own source, so the witness survives
    // shrinking as re-emittable input rather than as a remembered string.
    match emit_scad(&case.recipe, &case.recipe_bounds) {
        Ok(source) if source == case.source => {}
        Ok(_) => {
            return Err(TestCaseError::fail(
                "a retained scripted case no longer re-emits its own source",
            ))
        }
        Err(error) => {
            return Err(TestCaseError::fail(format!(
                "a retained scripted case no longer re-emits: {error}"
            )))
        }
    }
    let operand = match &case.output.geometry {
        ScadGeometry::Contours(contours) => contours.clone(),
        ScadGeometry::Empty => {
            return Err(TestCaseError::fail(format!(
                "a certified {class:?} contour case evaluated to no geometry"
            )))
        }
        ScadGeometry::Mesh(mesh) => {
            return Err(TestCaseError::fail(format!(
                "a certified {class:?} contour case converted to a mesh with {} triangles",
                mesh.indices.len() / 3
            )))
        }
    };
    if operand.is_empty() {
        return Err(TestCaseError::fail(format!(
            "a certified {class:?} contour case has no components"
        )));
    }
    let rectangle = match rectangle_operand(&operand) {
        Ok(rectangle) => rectangle,
        Err(error) => return Err(TestCaseError::fail(error)),
    };
    let out = BoolOutputs::compute(&operand, std::slice::from_ref(&rectangle));
    match check_membership(
        &operand,
        std::slice::from_ref(&rectangle),
        &out,
        limits.max_queries,
    ) {
        Ok(stats) => {
            if let Err(error) = record_case_observability(stats.material_witnesses > 0) {
                return Err(TestCaseError::fail(format!(
                    "H-BOOL-SCAD non-vacuity accounting: {error}"
                )));
            }
            println!(
                "H-BOOL-SCAD {class:?} case: {} component(s), {stats:?}",
                operand.len()
            );
            Ok(())
        }
        Err(error) => Err(TestCaseError::fail(error)),
    }
}

/// A deterministic, engine-evaluated witness recipe in canonical units.
///
/// Both recipes are built from `max_abs_units` alone, so the witness does not
/// depend on RNG draws, and every dimension is a quarter or eighth of the
/// profile envelope, i.e. at least four canonical units at the minimum accepted
/// envelope.
fn witness_recipes(max_abs_units: i64) -> [(ScadRecipe, &'static str); 2] {
    let side = (max_abs_units / 2).max(8);
    let half = (side / 2).max(4);
    let quarter = (side / 4).max(2);
    let small = (side / 4).max(2);
    let side_units = u32::try_from(side).unwrap_or(u32::MAX);
    let half_units = u32::try_from(half).unwrap_or(u32::MAX);
    let quarter_units = u32::try_from(quarter).unwrap_or(u32::MAX);
    let small_units = u32::try_from(small).unwrap_or(u32::MAX);
    [
        (
            // A square annulus: the cut is strictly inside its base.
            ScadRecipe::Contours(Recipe2d::Difference {
                base: Box::new(Recipe2d::Rectangle {
                    size_units: [side_units, side_units],
                }),
                cuts: vec![Recipe2d::Translate {
                    offset_units: [i64::from(quarter_units), i64::from(quarter_units)],
                    child: Box::new(Recipe2d::Rectangle {
                        size_units: [half_units, half_units],
                    }),
                }],
            }),
            "holed",
        ),
        (
            // Two bodies separated by a positive gap.
            ScadRecipe::Contours(Recipe2d::Union(vec![
                Recipe2d::Rectangle {
                    size_units: [small_units, small_units],
                },
                Recipe2d::Translate {
                    offset_units: [i64::from(side_units), 0],
                    child: Box::new(Recipe2d::Rectangle {
                        size_units: [small_units, small_units],
                    }),
                },
            ])),
            "disconnected",
        ),
    ]
}

/// Canonical-unit axis-aligned rectangle as a counter-clockwise `ExPolygon`.
fn rect_units(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
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

/// Control arm for the forced-witness ordering: a component the lattice cannot see.
///
/// The needle is four units tall and one hundred wide, placed in the gap between
/// the disconnected witness's two bodies. Its only integer points at [`MARGIN`]
/// from its own edges are on `y = y0 + 2`, and no sampled `Y` is that value (the
/// needle's own coords contribute the quarter points `y0 + 1` and `y0 + 3`), so
/// the lattice pool holds no point inside it and its witness can only come from
/// the bounded exhaustive scan. Before the fix that witness was appended at the
/// *tail* of the pool, which the strided fill provably cannot reach: with
/// `rest.len() > room` the last element is never selected by
/// `index * rest.len() / room`. Every sample outside the needle classifies
/// identically for the unmutated and the mutated output, so the rejection below
/// is possible only if a sample inside the needle was forced into the vector.
fn unseen_component_arm(operand: &[ExPolygon], max_queries: usize) -> Result<(), String> {
    let (min_x, min_y, max_x, _max_y) = ring_bbox(operand)?;
    let span = max_x - min_x;
    if span < 1_000 {
        return Err(format!(
            "H-BOOL-SCAD arm 'dropped-unseen-component': the witness envelope spans {span} units, \
             too small to host a component the lattice cannot see, so the arm cannot be \
             non-vacuous at this profile"
        ));
    }
    let needle_x = min_x + span / 2;
    let needle = rect_units(needle_x, min_y, needle_x + 100, min_y + 4);
    let mut augmented: Vec<ExPolygon> = operand.to_vec();
    augmented.push(needle.clone());
    let rectangle = rectangle_operand(&augmented)?;
    let mut all: Vec<ExPolygon> = augmented.clone();
    all.push(rectangle.clone());
    let all_edges = edges(&all);
    let pool = candidates(
        &all,
        &all_edges,
        &augmented,
        std::slice::from_ref(&rectangle),
    )?;
    let needle_only = std::slice::from_ref(&needle);
    for (point, _, _) in &pool {
        if matches!(location(*point, needle_only)?, PointLocation::Inside) {
            return Err(format!(
                "H-BOOL-SCAD arm 'dropped-unseen-component': the lattice already samples \
                 {point:?} inside the needle, so the arm would not exercise the forced witnesses"
            ));
        }
    }
    let out = BoolOutputs::compute(&augmented, std::slice::from_ref(&rectangle));
    let stats = check_membership(
        &augmented,
        std::slice::from_ref(&rectangle),
        &out,
        max_queries,
    )
    .map_err(|error| format!("H-BOOL-SCAD arm 'dropped-unseen-component' unmutated: {error}"))?;
    if stats.material_witnesses != augmented.len() || stats.proved_unobservable != 0 {
        return Err(format!(
            "H-BOOL-SCAD arm 'dropped-unseen-component': the unmutated set reports {stats:?}, so \
             the needle is not observable and the arm would be vacuous"
        ));
    }
    // The SUT answer a dropped needle would produce: the original two bodies,
    // through the same real wrappers, against the same rectangle.
    let mutant = BoolOutputs::compute(operand, std::slice::from_ref(&rectangle));
    expect_rejected(
        "dropped-unseen-component",
        "dropped-unseen-component",
        check_membership(
            &augmented,
            std::slice::from_ref(&rectangle),
            &mutant,
            max_queries,
        ),
    )
}

/// Every polygon of `output` with its holes removed, plus the number cleared.
fn filled_holes(output: &[ExPolygon]) -> (Vec<ExPolygon>, usize) {
    let mut cleared = 0;
    let filled = output
        .iter()
        .map(|polygon| {
            cleared += polygon.holes.len();
            ExPolygon {
                contour: polygon.contour.clone(),
                holes: Vec::new(),
            }
        })
        .collect();
    (filled, cleared)
}

/// Require the membership reference to reject a named mutant.
///
/// The rejection must come from the membership classification itself ("expected
/// … but the output is …"), never from ring validation or a profile error, so
/// the arm cannot pass because of an unrelated fault. The unmutated output is
/// checked against the same reference in the same witness, which is what makes
/// the arm non-vacuous.
fn expect_rejected(
    arm: &str,
    what: &str,
    result: Result<MembershipStats, String>,
) -> Result<(), String> {
    match result {
        Ok(stats) => Err(format!(
            "H-BOOL-SCAD arm '{arm}': the {what} mutant passed the membership reference: {stats:?}"
        )),
        Err(reason) => {
            if !(reason.contains("expected") && reason.contains("but the output is")) {
                return Err(format!(
                    "H-BOOL-SCAD arm '{arm}': the {what} mutant was rejected for the wrong reason, \
                     so the arm does not exercise the classification: {reason}"
                ));
            }
            println!("H-BOOL-SCAD arm '{arm}' rejected the {what} mutant: {reason}");
            Ok(())
        }
    }
}

/// Reject-arm for one operation whose real output has holes filled.
fn expect_filled_holes_rejected(
    op: BoolOp,
    out: &BoolOutputs,
    operand: &[ExPolygon],
    rectangle: &[ExPolygon],
    max_queries: usize,
) -> Result<bool, String> {
    let (filled, cleared) = filled_holes(out.get(op));
    if cleared == 0 {
        return Ok(false);
    }
    let mutant = match op {
        BoolOp::Union => BoolOutputs {
            union: filled,
            ..out.clone()
        },
        BoolOp::Intersection => BoolOutputs {
            intersection: filled,
            ..out.clone()
        },
        BoolOp::Difference => BoolOutputs {
            difference: filled,
            ..out.clone()
        },
        BoolOp::Xor => BoolOutputs {
            xor: filled,
            ..out.clone()
        },
    };
    expect_rejected(
        &format!("filled-hole/{op:?}"),
        &format!("{op:?} filled-hole"),
        check_membership(operand, rectangle, &mutant, max_queries),
    )?;
    Ok(true)
}

/// Deterministic, RNG-independent witnesses evaluated through the real adapter.
///
/// The converted forest is then passed through the same membership reference the
/// generated cases use, so a hole or a separated body is observed by the oracle
/// rather than assumed from the recipe. Each witness also runs the mutation arms
/// below, so the reference is proved to *detect* the faults the charter names
/// rather than merely to accept the correct output.
pub fn deterministic_witnesses(limits: &ContourLimits) -> Result<(), String> {
    for (recipe, label) in witness_recipes(limits.geometry.max_abs_units) {
        let output = evaluate_recipe(&recipe, &limits.options, &limits.recipe)
            .map_err(|error| format!("{label} witness evaluation: {error}"))?;
        let operand = match output.geometry {
            ScadGeometry::Contours(contours) => contours,
            other => {
                return Err(format!(
                    "{label} witness converted to {other:?}, not a contour forest"
                ))
            }
        };
        let holes: usize = operand.iter().map(|polygon| polygon.holes.len()).sum();
        match label {
            "holed" => {
                if operand.is_empty() || holes == 0 {
                    return Err(format!(
                        "the holed witness must convert to at least one component with a hole, \
                         got {} component(s) and {holes} hole(s)",
                        operand.len()
                    ));
                }
            }
            _ => {
                if operand.len() < 2 {
                    return Err(format!(
                        "the disconnected witness must convert to at least two components, got \
                         {}",
                        operand.len()
                    ));
                }
            }
        }
        let rectangle = rectangle_operand(&operand)?;
        let out = BoolOutputs::compute(&operand, std::slice::from_ref(&rectangle));
        let stats = check_membership(
            &operand,
            std::slice::from_ref(&rectangle),
            &out,
            limits.max_queries,
        )?;
        println!(
            "H-BOOL-SCAD {label} witness: {} component(s), {holes} hole(s), {stats:?}",
            operand.len()
        );
        // Fault detection arms. The unmutated output above already passed this
        // exact reference; every mutant below must be rejected by it.
        match label {
            "holed" => {
                let mut filled_arms = 0;
                for op in BoolOp::ALL {
                    if expect_filled_holes_rejected(
                        op,
                        &out,
                        &operand,
                        std::slice::from_ref(&rectangle),
                        limits.max_queries,
                    )? {
                        filled_arms += 1;
                    }
                }
                if filled_arms == 0 {
                    return Err(String::from(
                        "H-BOOL-SCAD arm 'filled-hole': no operation of the holed witness has a \
                         hole to fill, so the arm would be vacuous",
                    ));
                }
                let all_outside = BoolOutputs {
                    union: Vec::new(),
                    intersection: Vec::new(),
                    difference: Vec::new(),
                    xor: Vec::new(),
                };
                expect_rejected(
                    "all-outside",
                    "all-outside",
                    check_membership(
                        &operand,
                        std::slice::from_ref(&rectangle),
                        &all_outside,
                        limits.max_queries,
                    ),
                )?;
            }
            _ => {
                // The SUT answer a dropped body would produce: the same operands
                // with one component's material removed, through the same real
                // wrappers.
                let dropped = operand[1..].to_vec();
                let mutant = BoolOutputs::compute(&dropped, std::slice::from_ref(&rectangle));
                expect_rejected(
                    "dropped-body",
                    "dropped-body",
                    check_membership(
                        &operand,
                        std::slice::from_ref(&rectangle),
                        &mutant,
                        limits.max_queries,
                    ),
                )?;
                unseen_component_arm(&operand, limits.max_queries)?;
            }
        }
    }
    Ok(())
}

/// Run the registry-selected `H-BOOL-SCAD` property.
pub fn run_property() {
    let run = PropertyRun::new(PROPERTY_ID, TEST_NAME)
        .expect("H-BOOL-SCAD requires the registry-backed property command");
    let raw = RawScadBounds::read(&run).expect("H-BOOL-SCAD bound keys");
    let limits = checked_limits(raw).expect("H-BOOL-SCAD checked profile bounds");
    deterministic_witnesses(&limits).expect("H-BOOL-SCAD deterministic scripted witnesses");
    let strategy = strategy(&limits).expect("H-BOOL-SCAD strategy");
    reset_observability().expect("H-BOOL-SCAD non-vacuity accounting");
    run.check(&strategy, |(class, case), _guard| {
        check_case(class, &case, &limits)
    })
    .expect("H-BOOL-SCAD scripted Boolean membership");
    if let Err(reason) = assert_run_non_vacuity() {
        panic!("H-BOOL-SCAD {reason}");
    }
}
