//! Charter `H-ARACHNE-PIPELINE`: real printable/contour output structure.
//!
//! # What is called
//!
//! Every generated case and every deterministic witness calls the real
//! `slicer_core::arachne::pipeline::run_arachne_pipeline` on realized geometry
//! from the shared #2 catalog model in [`super::graph`]. Parameters are built
//! by FRU from [`ArachneParams::default`] with bounded variation:
//!
//! * `max_bead_count` — an even value in `2..=max_beads` (canonical caps are
//!   even; the factory also rounds, but the strategy only emits even values);
//! * `distribution_count` — `1..=3`;
//! * `wall_transition_length` — `0.2`, `0.4` or `0.8` mm;
//! * `print_thin_walls` — both boolean branches;
//! * `is_initial_layer` — both boolean branches.
//!
//! Everything else is the audited default. The 10 mm square, the tapered
//! wedge, the thin strip and the collapsed square are deterministic witnesses
//! that run before the generated campaign, each under a fresh [`CaseGuard`].
//! Their measured declared outcomes are: square and wedge `Ok` with nonempty
//! printable lines (every measured closed line repeats its endpoint); thin
//! strip `Ok` with one nonempty widened line; collapsed square
//! `Err(EmptyAfterPreprocess)`. No witness is credited as an OrcaSlicer output
//! golden.
//!
//! # Declared-outcome model
//!
//! A generated case is [`OutcomeMode::Guaranteed`] only when its topology
//! class is `Convex` **and** every component's axis-aligned bounding box is at
//! least [`ROBUST_EXTENT_UNITS`] (3 mm) on both axes **and** every ring edge
//! is at least [`ROBUST_EDGE_UNITS`] (0.5 mm) long. In the sampled catalog the
//! surviving members are plain rectangles and the dedicated 4..=8 mm
//! rectangle branch: the triangle template is only reachable at
//! `max_vertices == 3` (below this profile's 8-vertex minimum), and the
//! chamfered/unit-split layouts fail the edge-length condition. A plain
//! rectangle at least 3 mm on both axes has an inradius of at least 1.5 mm,
//! comfortably above the 0.2 mm half-width the default strategy stack needs
//! for a bead and above the 0.5 mm stage-8 small-area side the preprocessing
//! pipeline removes. That class must return `Ok` with **nonempty printable
//! lines**. Every other generated case (tiny convex shapes,
//! chamfered/unit-split convex shapes, concave, holed, thin, disconnected) has
//! the declared outcome `Ok` with possibly empty printable lines, or
//! `Err(ArachnePipelineError::EmptyAfterPreprocess)` when the preprocessing
//! stages erase the whole outline. Any other error is a failure, never an
//! accepted class.
//!
//! # Output relations checked
//!
//! For printable lines (the first return value):
//!
//! * nonempty junction vectors;
//! * finite coordinates and widths, bounded from the live profile extent and
//!   the constructor's 5%-of-diagonal clamp margin;
//! * first junction width `> 0` (the first-junction marker separation that
//!   `separate_out_inner_contour` promises for printable lines);
//! * later widths finite and `>= 0` (the limited strategy's zero-width
//!   sentinels are legitimate interior markers);
//! * the `perimeter_index`/`inset_idx` relation, see [`IndexRule`]: pipeline
//!   output is checked for the source-derived endpoint residue, while the
//!   stronger "every junction carries the inset index" rule is asserted only
//!   on controlled fixtures.
//!
//! For `inner_contour` lines the first width is exactly `0.0`; it is a marker,
//! not an extrudable wall, so no positive-width requirement is made of it.
//!
//! `is_closed` is **not** a universal endpoint-equality claim here: canonical
//! `WallToolPaths::stitchToolPaths` sets the closure flag even when the
//! residual gap missed its append threshold. [`ClosedDomain::ExplicitRepeat`]
//! is used only on controlled fixtures whose closure is an explicit repeated
//! endpoint by construction; the pipeline campaign uses
//! [`ClosedDomain::Pipeline`] and never manufactures closure.
//!
//! # Bounds (authoring values, to be re-measured in Step 7)
//!
//! The five geometry keys are validated exactly as for `H-ARACHNE-GRAPH`
//! (see [`super::graph`]); additionally:
//!
//! | key | minimum | routine | extended |
//! |---|---:|---:|---:|
//! | `max_beads` | 10 (the `ArachneParams::default` cap) | 10 | 16 |
//!
//! The default cap must be inside the profile because the square witness uses
//! `ArachneParams::default()`. Generated caps are even values `2..=max_beads`.

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_core::arachne::pipeline::{run_arachne_pipeline, ArachneParams, ArachnePipelineError};
use slicer_ir::{ExPolygon, ExtrusionLine, Point2, Polygon, UNITS_PER_MM};
use slicer_sdk::test_support::property_testing::domains::TopologyClass;
use slicer_sdk::test_support::property_testing::oracles::validate_geometry;
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

use super::graph::{
    campaign_strategy, realized_domain, validate_graph_bounds, BoundsError, GraphLimits,
    RawGraphBounds, TaggedCase,
};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-ARACHNE-PIPELINE";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_arachne_pipeline_structure";

/// Smallest accepted `max_beads`: the `ArachneParams::default` cap the
/// deterministic square witness runs with.
pub const MIN_MAX_BEADS: u64 = 10;
/// Largest accepted `max_beads` (allocation/safety ceiling).
pub const MAX_BEADS: u64 = 64;

/// Smallest both-axis bounding-box extent, canonical units, of a generated
/// case in the guaranteed-output class (3 mm).
pub const ROBUST_EXTENT_UNITS: i64 = 30_000;
/// Smallest ring-edge length, canonical units, of a generated case in the
/// guaranteed-output class (0.5 mm = 5_000 units).
pub const ROBUST_EDGE_UNITS: i64 = 5_000;

/// Raw `H-ARACHNE-PIPELINE` profile keys, before validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawPipelineBounds {
    /// The five shared geometry keys.
    pub geometry: RawGraphBounds,
    /// `max_beads`.
    pub max_beads: u64,
}

impl RawPipelineBounds {
    /// Read every `H-ARACHNE-PIPELINE` key from the selected registry profile.
    pub fn read(run: &PropertyRun) -> Result<Self, BoundsError> {
        Ok(Self {
            geometry: RawGraphBounds::read(run)?,
            max_beads: run
                .bound("max_beads")
                .map_err(|error| BoundsError::new(format!("bound max_beads: {error}")))?,
        })
    }
}

/// Validated `H-ARACHNE-PIPELINE` limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineLimits {
    /// The shared geometry limits.
    pub graph: GraphLimits,
    /// Largest accepted `max_bead_count` (even).
    pub max_bead_cap: usize,
}

/// Validate a raw profile before any strategy exists.
pub fn validate_pipeline_bounds(raw: RawPipelineBounds) -> Result<PipelineLimits, BoundsError> {
    let graph = validate_graph_bounds(raw.geometry)?;
    if !(MIN_MAX_BEADS..=MAX_BEADS).contains(&raw.max_beads) {
        return Err(BoundsError::new(format!(
            "bound max_beads = {} is outside the accepted range {MIN_MAX_BEADS}..={MAX_BEADS}",
            raw.max_beads
        )));
    }
    let even = raw.max_beads - raw.max_beads % 2;
    let max_bead_cap = usize::try_from(even)
        .map_err(|_| BoundsError::new(format!("max_beads = {even} does not fit usize")))?;
    Ok(PipelineLimits {
        graph,
        max_bead_cap,
    })
}

/// A valid reference profile used by the deterministic controls (authoring
/// tier, not a measured registry profile).
pub fn reference_limits() -> PipelineLimits {
    validate_pipeline_bounds(RawPipelineBounds {
        geometry: RawGraphBounds {
            max_abs_units: 400_000,
            max_vertices: 24,
            max_holes: 2,
            max_components: 3,
            max_shrink_candidates: 64,
        },
        max_beads: 10,
    })
    .expect("reference H-ARACHNE-PIPELINE profile")
}

// ─────────────────────────────────────────────────────────────────────────────
// Declared outcomes and the independent output comparator.
// ─────────────────────────────────────────────────────────────────────────────

/// The declared outcome a case is allowed to produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeMode {
    /// The guaranteed-output class: `Ok` with nonempty printable lines.
    Guaranteed,
    /// Every other declared class: `Ok` (possibly empty) or
    /// `EmptyAfterPreprocess`.
    AllowedEmpty,
}

/// How `perimeter_index` is checked against `inset_idx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexRule {
    /// Pipeline output: the line's inset index must be carried by at least one
    /// endpoint junction. This is the residue of
    /// `generate_junctions`'s `junction.perimeter_index = junction_idx` after
    /// `connect_junctions` (which can pair a fan position with a different
    /// bead index) and after stitching may reverse odd chains.
    PipelineEndpoints,
    /// Controlled fixtures: every junction of a hand-built line carries the
    /// line's inset index, so a single corrupted index is rejected.
    ControlledUniform,
}

/// Whether `is_closed` implies a repeated endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedDomain {
    /// Real pipeline output: the closure flag may legitimately disagree with
    /// an exact repeated endpoint, so equality is not asserted.
    Pipeline,
    /// Controlled fixtures built with an explicit repeated endpoint.
    ExplicitRepeat,
}

/// Statistics used by the measurement pilot and by non-vacuity assertions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutputStats {
    /// Number of printable lines.
    pub printable_lines: usize,
    /// Number of inner-contour marker lines.
    pub inner_contour_lines: usize,
    /// Printable lines whose junctions all carry `inset_idx`.
    pub uniform_lines: usize,
    /// Printable lines with `is_closed`.
    pub closed_lines: usize,
    /// Closed printable lines whose endpoints coincide.
    pub explicit_repeat_lines: usize,
}

fn check_finite_bounded(value: f32, bound: f64, what: &str) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("{what} is not finite ({value})"));
    }
    if f64::from(value).abs() > bound {
        return Err(format!(
            "{what} = {value} exceeds the derived bound {bound}"
        ));
    }
    Ok(())
}

fn check_index_rule(line: &ExtrusionLine, rule: IndexRule) -> Result<(), String> {
    let Some(first) = line.junctions.first() else {
        return Err("line has no junctions".into());
    };
    let last = line.junctions.last().expect("nonempty checked above");
    match rule {
        IndexRule::ControlledUniform => {
            for (index, junction) in line.junctions.iter().enumerate() {
                if junction.perimeter_index != line.inset_idx {
                    return Err(format!(
                        "junction {index} carries perimeter_index {} but inset_idx is {}",
                        junction.perimeter_index, line.inset_idx
                    ));
                }
            }
            Ok(())
        }
        IndexRule::PipelineEndpoints => {
            if first.perimeter_index != line.inset_idx && last.perimeter_index != line.inset_idx {
                return Err(format!(
                    "inset_idx {} is not carried by either endpoint (first {}, last {})",
                    line.inset_idx, first.perimeter_index, last.perimeter_index
                ));
            }
            Ok(())
        }
    }
}

/// Compare a real pipeline result against the declared output contract.
///
/// This is the independent comparator; controls feed it hand-built mutants.
pub fn check_output(
    printable: &[ExtrusionLine],
    inner_contour: &[ExtrusionLine],
    mode: OutcomeMode,
    index_rule: IndexRule,
    closed: ClosedDomain,
    limits: &GraphLimits,
) -> Result<OutputStats, String> {
    let extent_mm = limits.max_abs_units as f64 / UNITS_PER_MM;
    // Coordinates: graph vertices are clamped to the input bbox plus at most
    // 5% of its diagonal (or 1 mm, whichever is larger), and junctions
    // interpolate between such vertices.
    let coord_bound_mm = 1.2 * extent_mm + 1.0;
    // Widths: a bead width is at most the region thickness, which is at most
    // twice the bbox diagonal (`2 * sqrt(2) * (2 * extent) = 5.66 * extent`).
    let width_bound_mm = 6.0 * extent_mm + 1.0;

    if mode == OutcomeMode::Guaranteed && printable.is_empty() {
        return Err(
            "guaranteed-output class returned Ok with no printable lines (an empty class must \
             not satisfy printable coverage)"
                .into(),
        );
    }

    let mut stats = OutputStats {
        printable_lines: printable.len(),
        inner_contour_lines: inner_contour.len(),
        ..OutputStats::default()
    };

    for (line_index, line) in printable.iter().enumerate() {
        if line.junctions.is_empty() {
            return Err(format!("printable line {line_index} has no junctions"));
        }
        let first = &line.junctions[0];
        if !(first.p.width.is_finite() && first.p.width > 0.0) {
            return Err(format!(
                "printable line {line_index} first width {} is not finite and positive \
                 (first-junction marker separation)",
                first.p.width
            ));
        }
        for (junction_index, junction) in line.junctions.iter().enumerate() {
            check_finite_bounded(
                junction.p.x,
                coord_bound_mm,
                &format!("printable line {line_index} junction {junction_index} x"),
            )?;
            check_finite_bounded(
                junction.p.y,
                coord_bound_mm,
                &format!("printable line {line_index} junction {junction_index} y"),
            )?;
            if !junction.p.width.is_finite() || junction.p.width < 0.0 {
                return Err(format!(
                    "printable line {line_index} junction {junction_index} width {} is not \
                     finite and nonnegative",
                    junction.p.width
                ));
            }
            if f64::from(junction.p.width) > width_bound_mm {
                return Err(format!(
                    "printable line {line_index} junction {junction_index} width {} exceeds the \
                     derived bound {width_bound_mm}",
                    junction.p.width
                ));
            }
        }
        check_index_rule(line, index_rule)
            .map_err(|error| format!("printable line {line_index}: {error}"))?;
        if line
            .junctions
            .iter()
            .all(|junction| junction.perimeter_index == line.inset_idx)
        {
            stats.uniform_lines += 1;
        }
        if line.is_closed {
            stats.closed_lines += 1;
            if line.junctions.len() >= 2 {
                let first = &line.junctions[0].p;
                let last = &line.junctions[line.junctions.len() - 1].p;
                if first.x == last.x && first.y == last.y {
                    stats.explicit_repeat_lines += 1;
                }
            }
            if closed == ClosedDomain::ExplicitRepeat {
                if line.junctions.len() < 2 {
                    return Err(format!(
                        "closed printable line {line_index} in the explicit-repeat domain has \
                         fewer than two junctions"
                    ));
                }
                let first = &line.junctions[0].p;
                let last = &line.junctions[line.junctions.len() - 1].p;
                if first.x != last.x || first.y != last.y {
                    return Err(format!(
                        "printable line {line_index} is_closed but its endpoints differ in the \
                         explicit-repeat domain: first ({}, {}), last ({}, {})",
                        first.x, first.y, last.x, last.y
                    ));
                }
            }
        }
    }

    for (line_index, line) in inner_contour.iter().enumerate() {
        let Some(first) = line.junctions.first() else {
            return Err(format!("inner-contour line {line_index} has no junctions"));
        };
        if first.p.width != 0.0 {
            return Err(format!(
                "inner-contour line {line_index} first width {} is not the zero-width marker",
                first.p.width
            ));
        }
        for (junction_index, junction) in line.junctions.iter().enumerate() {
            check_finite_bounded(
                junction.p.x,
                coord_bound_mm,
                &format!("inner-contour line {line_index} junction {junction_index} x"),
            )?;
            check_finite_bounded(
                junction.p.y,
                coord_bound_mm,
                &format!("inner-contour line {line_index} junction {junction_index} y"),
            )?;
            if !junction.p.width.is_finite() || junction.p.width < 0.0 {
                return Err(format!(
                    "inner-contour line {line_index} junction {junction_index} width {} is not \
                     finite and nonnegative",
                    junction.p.width
                ));
            }
        }
    }

    Ok(stats)
}

// ─────────────────────────────────────────────────────────────────────────────
// Guaranteed-output classification.
// ─────────────────────────────────────────────────────────────────────────────

fn ring_extent(ring: &Polygon) -> (i64, i64) {
    let min_x = ring.points.iter().map(|p| p.x).min().unwrap_or(0);
    let max_x = ring.points.iter().map(|p| p.x).max().unwrap_or(0);
    let min_y = ring.points.iter().map(|p| p.y).min().unwrap_or(0);
    let max_y = ring.points.iter().map(|p| p.y).max().unwrap_or(0);
    (max_x - min_x, max_y - min_y)
}

fn edge_length_sq_ok(ring: &Polygon) -> bool {
    ring.points
        .iter()
        .zip(ring.points.iter().cycle().skip(1))
        .take(ring.points.len())
        .all(|(a, b)| {
            let dx = i128::from(b.x) - i128::from(a.x);
            let dy = i128::from(b.y) - i128::from(a.y);
            dx * dx + dy * dy >= i128::from(ROBUST_EDGE_UNITS) * i128::from(ROBUST_EDGE_UNITS)
        })
}

/// Whether a realized case is in the guaranteed-output class derived above.
pub fn is_guaranteed_output(case: &TaggedCase) -> bool {
    if case.class != TopologyClass::Convex {
        return false;
    }
    case.case.polygons.iter().all(|polygon| {
        let (width, height) = ring_extent(&polygon.contour);
        width >= ROBUST_EXTENT_UNITS
            && height >= ROBUST_EXTENT_UNITS
            && edge_length_sq_ok(&polygon.contour)
            && polygon.holes.iter().all(edge_length_sq_ok)
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic witnesses.
// ─────────────────────────────────────────────────────────────────────────────

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

/// Whole millimetres to canonical units (1 unit = 100 nm; 1 mm = 10_000
/// units).
fn mm_units(mm: i64) -> i64 {
    (mm as f64 * UNITS_PER_MM) as i64
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// The 10 mm square from `crates/slicer-core/tests/arachne_pipeline.rs`.
fn square_10mm() -> ExPolygon {
    let side = mm_units(10);
    expoly(vec![p(0, 0), p(side, 0), p(side, side), p(0, side)])
}

/// The tapered wedge from `crates/slicer-core/tests/arachne_pipeline.rs`.
fn tapered_wedge() -> ExPolygon {
    let mm = |v: f64| (v * UNITS_PER_MM) as i64;
    expoly(vec![
        p(0, mm(-0.175)),
        p(mm(12.0), mm(-1.3)),
        p(mm(12.0), mm(1.3)),
        p(0, mm(0.175)),
    ])
}

/// A 8 mm x 0.3 mm strip: one strand strictly below the default 0.4 mm
/// optimal width, so only the thin-wall strategy can emit anything.
fn thin_strip() -> ExPolygon {
    let length = mm_units(8);
    let width = mm_units(3) / 10;
    expoly(vec![p(0, 0), p(length, 0), p(length, width), p(0, width)])
}

/// A 50-unit square (5 µm): below the preprocessing epsilon, so the nine
/// stages erase it entirely.
fn collapsed_square() -> ExPolygon {
    expoly(vec![p(0, 0), p(50, 0), p(50, 50), p(0, 50)])
}

/// Prove the four mandatory deterministic outcomes on named witnesses.
pub fn deterministic_witnesses(limits: &PipelineLimits) {
    {
        let _guard = CaseGuard::new();
        let params = ArachneParams::default();
        let (lines, inner) =
            run_arachne_pipeline(std::slice::from_ref(&square_10mm()), &params, false)
                .expect("H-ARACHNE-PIPELINE: the 10 mm square must return Ok");
        let stats = check_output(
            &lines,
            &inner,
            OutcomeMode::Guaranteed,
            IndexRule::PipelineEndpoints,
            ClosedDomain::ExplicitRepeat,
            &limits.graph,
        )
        .expect("H-ARACHNE-PIPELINE: the 10 mm square satisfies the output contract");
        assert!(
            !lines.is_empty() && stats.printable_lines == lines.len(),
            "H-ARACHNE-PIPELINE: the square witness must yield nonempty printable lines, got \
             {stats:?}"
        );
        assert_eq!(
            stats.uniform_lines, stats.printable_lines,
            "H-ARACHNE-PIPELINE: every measured square junction must carry its line's inset \
             index, got {stats:?}"
        );
        assert_eq!(
            stats.closed_lines, stats.explicit_repeat_lines,
            "H-ARACHNE-PIPELINE: every measured closed square line must repeat its endpoint, \
             got {stats:?}"
        );
    }

    {
        let _guard = CaseGuard::new();
        let params = ArachneParams::default();
        let (lines, inner) =
            run_arachne_pipeline(std::slice::from_ref(&tapered_wedge()), &params, false)
                .expect("H-ARACHNE-PIPELINE: the tapered wedge must return Ok");
        let stats = check_output(
            &lines,
            &inner,
            OutcomeMode::Guaranteed,
            IndexRule::PipelineEndpoints,
            ClosedDomain::ExplicitRepeat,
            &limits.graph,
        )
        .expect("H-ARACHNE-PIPELINE: the tapered wedge satisfies the output contract");
        assert_eq!(
            stats.closed_lines, stats.explicit_repeat_lines,
            "H-ARACHNE-PIPELINE: every measured closed wedge line must repeat its endpoint, \
             got {stats:?}"
        );
        let widths: Vec<f32> = lines
            .iter()
            .flat_map(|line| line.junctions.iter())
            .map(|junction| junction.p.width)
            .collect();
        assert!(
            widths.len() >= 2,
            "H-ARACHNE-PIPELINE: the tapered wedge must emit at least two junction widths, got \
             {stats:?}"
        );
        let first = widths[0];
        assert!(
            widths.iter().any(|width| (*width - first).abs() > 1e-6),
            "H-ARACHNE-PIPELINE: the tapered wedge must emit variable widths, got {widths:?}"
        );
    }

    {
        // Eligible thin wall: `print_thin_walls` on, a strip below one bead.
        // Declared outcome: `Ok` with nonempty printable lines. Measured on
        // this witness: one open, odd, two-junction inset-0 line of width
        // 0.34 mm (the widened minimum output width), because the widening
        // branch emits a single bead for a sub-optimal-width strip. The
        // assertion records that measured nonempty population; a return to
        // emptiness would be a real thin-wall regression, not an accepted
        // alternative.
        let _guard = CaseGuard::new();
        let params = ArachneParams {
            print_thin_walls: true,
            ..ArachneParams::default()
        };
        match run_arachne_pipeline(std::slice::from_ref(&thin_strip()), &params, false) {
            Ok((lines, inner)) => {
                check_output(
                    &lines,
                    &inner,
                    OutcomeMode::AllowedEmpty,
                    IndexRule::PipelineEndpoints,
                    ClosedDomain::Pipeline,
                    &limits.graph,
                )
                .expect("H-ARACHNE-PIPELINE: the thin strip output satisfies the contract");
                assert!(
                    !lines.is_empty(),
                    "H-ARACHNE-PIPELINE: the eligible thin strip measured a nonempty widened \
                     printable line under print_thin_walls = true; an empty result is a \
                     regression of the thin-wall branch, not an accepted outcome"
                );
            }
            Err(error) => panic!(
                "H-ARACHNE-PIPELINE: the eligible thin strip must return Ok (possibly with empty \
                 lines) under print_thin_walls = true, got {error:?}"
            ),
        }
    }

    {
        // Collapsed outline: the preprocessing stages erase it.
        let _guard = CaseGuard::new();
        let params = ArachneParams::default();
        match run_arachne_pipeline(std::slice::from_ref(&collapsed_square()), &params, false) {
            Err(ArachnePipelineError::EmptyAfterPreprocess) => {}
            other => panic!(
                "H-ARACHNE-PIPELINE: the collapsed square must return exactly \
                 EmptyAfterPreprocess, got {other:?}"
            ),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Generated campaign.
// ─────────────────────────────────────────────────────────────────────────────

/// One generated `H-ARACHNE-PIPELINE` input.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineInput {
    /// Realized, independently validated geometry tagged with its class.
    pub case: TaggedCase,
    /// Bounded FRU variation over the audited defaults.
    pub params: ArachneParams,
    /// Layer flag passed to the pipeline (and mirrored into `params`).
    pub is_initial_layer: bool,
}

fn params_strategy(limits: &PipelineLimits) -> BoxedStrategy<(ArachneParams, bool)> {
    let caps: Vec<u32> = (2..=limits.max_bead_cap as u32)
        .filter(|cap| cap % 2 == 0)
        .collect();
    assert!(
        !caps.is_empty(),
        "max_bead_cap {} admits no even cap",
        limits.max_bead_cap
    );
    (
        prop::sample::select(caps),
        1u32..=3u32,
        prop::sample::select(vec![0.2_f64, 0.4, 0.8]),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            move |(cap, distribution_count, wall_transition_length, thin, initial)| {
                // exhaustive: every unlisted field is the audited default
                let params = ArachneParams {
                    max_bead_count: cap,
                    distribution_count,
                    wall_transition_length,
                    print_thin_walls: thin,
                    ..ArachneParams::default()
                };
                (params, initial)
            },
        )
        .boxed()
}

/// Build the `H-ARACHNE-PIPELINE` strategy. Every bound failure surfaces here,
/// before any value is sampled.
pub fn strategy(limits: &PipelineLimits) -> Result<BoxedStrategy<PipelineInput>, BoundsError> {
    let cases = campaign_strategy(&limits.graph)?;
    Ok((cases, params_strategy(limits))
        .prop_map(|(case, (params, is_initial_layer))| PipelineInput {
            case,
            params,
            is_initial_layer,
        })
        .boxed())
}

/// Check one generated case against its declared outcome contract.
pub fn check_case(input: &PipelineInput, limits: &PipelineLimits) -> Result<(), String> {
    let domain = realized_domain(input.case.class, &limits.graph);
    validate_geometry(&input.case.case, &domain).map_err(|error| {
        format!(
            "H-ARACHNE-PIPELINE precondition: {:?} case is not valid for the realized domain \
             (authoring bug, not a shrinkable rejection): {error}",
            input.case.class
        )
    })?;

    let mode = if is_guaranteed_output(&input.case) {
        OutcomeMode::Guaranteed
    } else {
        OutcomeMode::AllowedEmpty
    };

    match run_arachne_pipeline(
        &input.case.case.polygons,
        &input.params,
        input.is_initial_layer,
    ) {
        Ok((lines, inner)) => check_output(
            &lines,
            &inner,
            mode,
            IndexRule::PipelineEndpoints,
            ClosedDomain::Pipeline,
            &limits.graph,
        )
        .map(|_| ()),
        Err(ArachnePipelineError::EmptyAfterPreprocess) if mode == OutcomeMode::AllowedEmpty => {
            Ok(())
        }
        Err(error) => Err(format!(
            "H-ARACHNE-PIPELINE: {:?} case (mode {mode:?}, params {:?}, initial layer {}) \
             returned an unexpected error: {error:?}",
            input.case.class, input.params, input.is_initial_layer
        )),
    }
}

/// Run the registry-selected `H-ARACHNE-PIPELINE` property.
pub fn run_property() {
    let run = PropertyRun::new(PROPERTY_ID, TEST_NAME)
        .expect("use the registry-backed property command for H-ARACHNE-PIPELINE");
    let raw = RawPipelineBounds::read(&run).expect("H-ARACHNE-PIPELINE bounds");
    let limits = validate_pipeline_bounds(raw).expect("H-ARACHNE-PIPELINE profile");
    deterministic_witnesses(&limits);
    let strategy = strategy(&limits).expect("H-ARACHNE-PIPELINE strategy");
    run.check(&strategy, |input, _guard| -> TestCaseResult {
        check_case(&input, &limits).map_err(TestCaseError::fail)
    })
    .expect("H-ARACHNE-PIPELINE charter");
}
