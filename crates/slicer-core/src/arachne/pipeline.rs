// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp
// (`WallToolPaths::generate` orchestrator).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! End-to-end Arachne beading-strategy pipeline orchestration (packet 112,
//! Step 9A): chains every stage built across packets 110-112
//! (`preprocess_input_outline` -> `SkeletalTrapezoidationGraph::from_polygons`
//! -> `filter_central` -> `BeadingStrategyFactory::create_stack` ->
//! `assign_bead_counts` -> `propagate_beadings_upward`/`_downward` ->
//! `generate_toolpaths` -> `stitch_extrusions` -> `simplify_toolpaths` ->
//! `remove_small_lines`) into a single `&[ExPolygon] -> Vec<ExtrusionLine>`
//! entry point. This is the native pipeline the host-service bridge
//! (`slicer-wasm-host::host::generate_arachne_walls`, added alongside this
//! module) calls on behalf of a WASM guest that cannot itself link
//! `host-algos` (rayon + boostvoronoi are `cfg(not(target_arch = "wasm32"))`
//! only) — mirroring the existing `medial-axis` host-service bridge.
//!
//! # Honesty note (no OrcaSlicer oracle)
//!
//! Every stage this orchestrator calls documents its own from-first-principles
//! adaptation where this crate's simplified (quad-less, rib-less) skeletal
//! trapezoidation graph diverges from OrcaSlicer's richer topology — see
//! `crate::skeletal_trapezoidation::centrality`,
//! `crate::skeletal_trapezoidation::bead_count`,
//! `crate::skeletal_trapezoidation::propagation`, and
//! `crate::arachne::generate_toolpaths`'s own module-level doc comments. This
//! orchestrator adds no additional numeric claim on top of those — it only
//! asserts that chaining them together produces a self-consistent
//! `Vec<ExtrusionLine>` (non-empty, deterministic, exhibiting real per-junction
//! width variation), never that the result matches OrcaSlicer's
//! `WallToolPaths::generate` output.
//!
//! Host-only: every stage this module calls
//! (`crate::skeletal_trapezoidation::*`) is gated behind the `host-algos`
//! feature (matching `voronoi`, `algos`, `medial_axis`), so this module is
//! gated identically (see `crate::arachne`'s own module doc comment for why
//! `generate_toolpaths` — and now this orchestrator — is the one `arachne`
//! submodule that needs the narrower gate, unlike its preprocess/stitch/
//! simplify/remove_small siblings).

use slicer_ir::{ExPolygon, ExtrusionLine, UNITS_PER_MM};

use crate::arachne::generate_toolpaths::generate_toolpaths;
use crate::arachne::preprocess::{preprocess_input_outline, PreprocessParams};
use crate::arachne::{
    remove_empty_toolpaths, remove_small_lines, separate_out_inner_contour, simplify_toolpaths,
    stitch_extrusions,
};
use crate::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use crate::skeletal_trapezoidation::propagation::propagate_beadings_downward_with_transition_dist;
use crate::skeletal_trapezoidation::{
    apply_transitions, assign_bead_counts, filter_central, filter_noncentral_regions,
    filter_transition_mids, generate_all_transition_ends, generate_extra_ribs,
    generate_transition_mids, populate_beading_propagation, propagate_beadings_upward,
    BeadCountError, CentralityParams, SkeletalTrapezoidationGraph, SktError,
};

/// Parameters controlling the end-to-end Arachne pipeline.
///
/// Every distance/width field is in millimeters, matching this crate's other
/// pipeline-level parameter structs (e.g. `PreprocessParams`). Conversion to
/// the workspace's scaled-integer unit space (1 unit = 100 nm, see
/// `docs/08_coordinate_system.md`) happens internally when building
/// `BeadingFactoryParams`/`CentralityParams`, both of which are unit-space
/// APIs (`slicer_ir::UNITS_PER_MM` is the single conversion factor used
/// throughout).
#[derive(Debug, Clone, PartialEq)]
pub struct ArachneParams {
    /// Nominal wall width (mm). Feeds `BeadingFactoryParams::optimal_width`
    /// (upstream's `preferred_bead_width_inner`).
    pub optimal_width: f64,
    /// Target width for the outermost/innermost bead (mm). Feeds
    /// `BeadingFactoryParams::preferred_bead_width_outer` and this pipeline's
    /// own `stitch_extrusions` gap threshold (`preferred_bead_width_outer -
    /// 1e-6`, in mm — matching `stitch_extrusions`'s mm-unit `max_gap`
    /// parameter).
    pub preferred_bead_width_outer: f64,
    /// Maximum bead count `LimitedBeadingStrategy` will ever request from its
    /// parent.
    pub max_bead_count: u32,
    /// Gaussian decay radius (bead-count units, dimensionless) for
    /// `DistributedBeadingStrategy`.
    pub distribution_count: u32,
    /// Whisker-dissolve length budget (mm) for `filter_central`'s stage 2,
    /// and (converted to units) `BeadingFactoryParams::transition_filter_dist`
    /// (a reserved parameter there — see that field's own doc comment).
    pub transition_filter_dist: f64,
    /// Depth floor (mm) for `filter_central`'s stage 1: an edge whose deepest
    /// endpoint never reaches this distance from the boundary is never
    /// central.
    pub min_central_distance: f64,
    /// Visvalingam-Whyatt width-weighted area threshold (mm²) for
    /// `simplify_toolpaths`.
    pub visvalingam_area_threshold: f64,
    /// Length-factor multiplier for `remove_small_lines`'s removal threshold
    /// (`min_length_factor * min_width`).
    pub min_length_factor: f64,
    /// Nominal width (mm) used by `remove_small_lines`'s length threshold.
    pub min_width: f64,
    /// Gates whether `WideningBeadingStrategy` is wrapped into the composed
    /// stack at all (packet 112, Step 9C). Feeds
    /// `BeadingFactoryParams::print_thin_walls` verbatim. Maps to the
    /// `detect_thin_wall` config key; `false` (the default) means the
    /// decorator is literally absent from the stack, matching
    /// `BeadingFactoryParams::default()`'s own default.
    pub print_thin_walls: bool,
    /// Threshold (mm) below which `WideningBeadingStrategy` reports no beads
    /// at all. Feeds `BeadingFactoryParams::min_input_width` (converted to
    /// units). Maps to the `min_feature_size` config key.
    pub min_feature_size: f64,
    /// Minimum bead width (mm) `WideningBeadingStrategy` clamps its emitted
    /// bead up to in the `min_feature_size <= thickness < optimal_width`
    /// regime. Feeds `BeadingFactoryParams::min_output_width` (converted to
    /// units). Maps to the `min_bead_width` config key.
    pub min_bead_width: f64,
    /// Transition-ramp length (mm) for `DistributedBeadingStrategy`. Feeds
    /// `BeadingFactoryParams::default_transition_length` (converted to units).
    /// Maps to the `wall_transition_length` config key.
    pub wall_transition_length: f64,
    /// Transition angle (radians) used by beading strategies that reject a
    /// transition when the turn exceeds this angle. Converted from the
    /// `wall_transition_angle` config key (degrees) by
    /// `arachne_params_from_config`.
    pub wall_transition_angle: f64,
    /// Minimum bead width (mm) for the initial layer, overriding the general
    /// thin-wall clamp where the strategy supports layer-specific output.
    /// Maps to the `initial_layer_min_bead_width` config key.
    pub initial_layer_min_bead_width: f64,
    /// Inward offset (mm) applied to the outer wall's toolpath location by
    /// `OuterWallInsetBeadingStrategy`. Feeds
    /// `BeadingFactoryParams::outer_wall_offset` (converted to units). Maps
    /// to the `outer_wall_offset` config key.
    pub outer_wall_offset: f64,
    /// Whether this run corresponds to the initial layer, which lets layer-
    /// aware beading strategies override `min_output_width` with
    /// `initial_layer_min_bead_width`.
    pub is_initial_layer: bool,
    /// Squared distance gate (mm²) for `simplify_toolpaths`: segments shorter
    /// than this AND within `allowed_error_distance_squared` of the chord are
    /// removed. Sourced from `meshfix_maximum_resolution` (mm) squared.
    /// Maps to the `meshfix_maximum_resolution` config key.
    pub smallest_line_segment_squared: f64,
    /// Squared error distance gate (mm²) for `simplify_toolpaths`: the
    /// perpendicular distance threshold for the primary removal gate.
    /// Sourced from `meshfix_maximum_deviation` (mm) squared.
    /// Maps to the `meshfix_maximum_deviation` config key.
    pub allowed_error_distance_squared: f64,
    /// Area deviation threshold (mm²) for `simplify_toolpaths`'s
    /// near-colinear fast-path guard (`calculateExtrusionAreaDeviationError`).
    /// Maps to the `meshfix_maximum_extrusion_area_deviation` config key.
    pub maximum_extrusion_area_deviation: f64,
}

impl Default for ArachneParams {
    /// Mirrors `BeadingFactoryParams::default()`'s registered-config defaults
    /// converted to millimeters (`optimal_width` = 0.4mm,
    /// `preferred_bead_width_outer` = 0.4mm, `max_bead_count` = 9,
    /// `distribution_count` = 1, `transition_filter_dist` = 0.1mm — the
    /// factory's own `1000.0`-unit default), plus this pipeline's own
    /// post-process defaults: `min_central_distance` = 0.0mm (no floor,
    /// matching `CentralityParams::default()`), `visvalingam_area_threshold` =
    /// 0.01 mm² (matching a typical bead-width-weighted area default derived
    /// from OrcaSlicer's `maximum_deviation` × typical 0.4 mm width),
    /// `min_length_factor` = 0.5 (matching OrcaSlicer's `removeSmallLines`
    /// default multiplier, also reused by this crate's own
    /// `tests/remove_small.rs`), `min_width` = 0.4mm (matching
    /// `optimal_width`). `print_thin_walls` = `detect_thin_wall` (`false`,
    /// parity-correct — see that field's own doc comment),
    /// `min_feature_size` = `min_feature_size` (0.1mm, the registered
    /// config default of `1000` units converted to mm), `min_bead_width` =
    /// `min_bead_width` (0.4mm, the registered config default of `4000`
    /// units converted to mm) — packet 112, Step 9C.
    fn default() -> Self {
        Self {
            optimal_width: 0.4,
            preferred_bead_width_outer: 0.4,
            max_bead_count: 9,
            distribution_count: 1,
            transition_filter_dist: 0.1,
            min_central_distance: 0.0,
            visvalingam_area_threshold: 0.01,
            min_length_factor: 0.5,
            min_width: 0.4,
            print_thin_walls: false,
            min_feature_size: 0.1,
            min_bead_width: 0.4,
            wall_transition_length: 0.4,
            wall_transition_angle: 10.0_f64.to_radians(),
            initial_layer_min_bead_width: 0.34,
            outer_wall_offset: 0.0,
            is_initial_layer: false,
            // Distance-gate defaults for simplify_toolpaths (N13).
            // meshfix_maximum_resolution = 0.05mm, squared = 0.0025 mm².
            smallest_line_segment_squared: 0.0025,
            // meshfix_maximum_deviation = 0.005mm, squared = 0.000025 mm².
            allowed_error_distance_squared: 0.000025,
            // meshfix_maximum_extrusion_area_deviation = 0.005 mm².
            maximum_extrusion_area_deviation: 0.005,
        }
    }
}

/// Errors from [`run_arachne_pipeline`].
#[derive(Debug, Clone, PartialEq)]
pub enum ArachnePipelineError {
    /// `preprocess_input_outline` reduced the input to nothing (e.g. every
    /// supplied polygon was smaller than
    /// `PreprocessParams::epsilon_offset_mm`).
    EmptyAfterPreprocess,
    /// `SkeletalTrapezoidationGraph::from_polygons` failed.
    Skt(SktError),
    /// `assign_bead_counts` failed.
    BeadCount(BeadCountError),
}

impl std::fmt::Display for ArachnePipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArachnePipelineError::EmptyAfterPreprocess => write!(
                f,
                "run_arachne_pipeline: input outline preprocessing produced no polygons"
            ),
            ArachnePipelineError::Skt(e) => write!(f, "run_arachne_pipeline: {e}"),
            ArachnePipelineError::BeadCount(e) => write!(f, "run_arachne_pipeline: {e}"),
        }
    }
}

impl std::error::Error for ArachnePipelineError {}

impl From<SktError> for ArachnePipelineError {
    fn from(e: SktError) -> Self {
        ArachnePipelineError::Skt(e)
    }
}

impl From<BeadCountError> for ArachnePipelineError {
    fn from(e: BeadCountError) -> Self {
        ArachnePipelineError::BeadCount(e)
    }
}

/// Builds a `BeadingFactoryParams` from `params`, starting from
/// `BeadingFactoryParams::default()` for every field this packet's brief
/// calls "remaining" (`outer_wall_offset`, `minimum_variable_line_ratio`,
/// `default_transition_length`) and overriding the fields `ArachneParams`
/// itself exposes, converting mm -> slicer units via `UNITS_PER_MM`.
/// `print_thin_walls` passes through unconverted (a plain `bool`);
/// `min_feature_size`/`min_bead_width` feed `min_input_width`/
/// `min_output_width` respectively (packet 112, Step 9C).
fn to_beading_factory_params(params: &ArachneParams) -> BeadingFactoryParams {
    let base_min_output_width = params.min_bead_width * UNITS_PER_MM;
    let initial_layer_min_output_width = params.initial_layer_min_bead_width * UNITS_PER_MM;

    BeadingFactoryParams {
        optimal_width: params.optimal_width * UNITS_PER_MM,
        preferred_bead_width_outer: params.preferred_bead_width_outer * UNITS_PER_MM,
        max_bead_count: params.max_bead_count as usize,
        distribution_count: params.distribution_count as usize,
        transition_filter_dist: params.transition_filter_dist * UNITS_PER_MM,
        default_transition_length: params.wall_transition_length * UNITS_PER_MM,
        min_input_width: params.min_feature_size * UNITS_PER_MM,
        min_output_width: if params.is_initial_layer {
            initial_layer_min_output_width
        } else {
            base_min_output_width
        },
        outer_wall_offset: params.outer_wall_offset * UNITS_PER_MM,
        print_thin_walls: params.print_thin_walls,
        wall_transition_angle: params.wall_transition_angle,
        initial_layer_min_bead_width: initial_layer_min_output_width,
        ..BeadingFactoryParams::default()
    }
}

/// Builds a `CentralityParams` from `params`, converting mm -> slicer units.
///
/// With the quad/rib topology from Step 1, the outer-edge filter no longer
/// needs to be artificially weakened to let radial spine edges through; ribs
/// are filtered by `EdgeType::EXTRA_VD` instead. The user-facing
/// `transition_filter_dist` therefore maps directly to
/// `CentralityParams::transition_filter_dist`.
///
fn to_centrality_params(params: &ArachneParams) -> CentralityParams {
    CentralityParams::new(
        params.transition_filter_dist * UNITS_PER_MM,
        params.min_central_distance * UNITS_PER_MM,
    )
}

/// Runs the full Arachne beading-strategy pipeline end to end over `polygons`,
/// producing the final, post-processed [`ExtrusionLine`] set.
///
/// Stage order: `preprocess_input_outline` -> `from_polygons` ->
/// `filter_central` -> `BeadingStrategyFactory::create_stack` ->
/// `assign_bead_counts` -> `propagate_beadings_upward` ->
/// `propagate_beadings_downward` -> `generate_toolpaths` (flattened across
/// insets) -> `stitch_extrusions` -> `simplify_toolpaths` ->
/// `remove_small_lines`.
///
/// # Honesty note
///
/// See this module's doc comment: no OrcaSlicer oracle backs this pipeline.
/// It only asserts that chaining the packet's own from-first-principles
/// stages together produces a self-consistent, deterministic result.
///
/// Deterministic (every composed stage is independently deterministic) and
/// panic-free — every fallible stage's error is mapped into
/// [`ArachnePipelineError`] via `?` rather than unwrapped.
pub fn run_arachne_pipeline(
    polygons: &[ExPolygon],
    params: &ArachneParams,
    is_initial_layer: bool,
) -> Result<Vec<ExtrusionLine>, ArachnePipelineError> {
    let mut params = params.clone();
    params.is_initial_layer = is_initial_layer;
    let preprocess_params = PreprocessParams::default();
    let cleaned = preprocess_input_outline(polygons, &preprocess_params);
    if cleaned.is_empty() {
        return Err(ArachnePipelineError::EmptyAfterPreprocess);
    }

    // Packet 113c Step 3: `from_polygons` now builds the real interleaved
    // rib/spine topology directly (faithful `transferEdge`/`makeRib` port),
    // so the separate `build_quad_rib_topology` pass (packet 113b's
    // reflex-corner-only approximation) is no longer needed here.
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(&cleaned)?;

    let centrality_params = to_centrality_params(&params);
    let beading_params = to_beading_factory_params(&params);
    let strategy = BeadingStrategyFactory::create_stack(&beading_params);
    filter_central(
        &mut graph,
        &centrality_params,
        beading_params.wall_transition_angle,
    );
    assign_bead_counts(&mut graph, strategy.as_ref())?;
    filter_noncentral_regions(&mut graph);

    generate_transition_mids(&mut graph, strategy.as_ref());
    filter_transition_mids(&mut graph, strategy.as_ref());
    generate_all_transition_ends(&mut graph, strategy.as_ref());
    apply_transitions(&mut graph);
    generate_extra_ribs(&mut graph, strategy.as_ref());
    propagate_beadings_upward(&mut graph);
    // Packet 113c Step 8b: thread the pipeline's *actual* configured
    // beading-propagation transition distance (`wall_transition_length`,
    // already converted to slicer units in `beading_params`) rather than
    // `propagate_beadings_downward`'s no-argument default — see
    // `propagation.rs`'s doc comments for why the frozen no-arg entry point
    // exists at all (every existing test call site invokes it directly) and
    // why this pipeline uses the richer variant instead.
    propagate_beadings_downward_with_transition_dist(
        &mut graph,
        beading_params.default_transition_length,
    );

    // Packet 141 (N7): populate the `BeadingPropagation` side table so
    // `generate_junctions`'s `get_beding`/`get_nearest_beding` lookups
    // resolve a real, propagated beading at each peak vertex instead of
    // falling through to a fresh `strategy.compute()` call every time.
    populate_beading_propagation(&mut graph, strategy.as_ref());

    let buckets = generate_toolpaths(&graph, strategy.as_ref());
    let lines: Vec<ExtrusionLine> = buckets.into_iter().flatten().collect();

    // Matches this packet's brief verbatim: max_gap = preferred_bead_width_outer
    // - 1e-6 (mm, matching `stitch_extrusions`'s mm-unit `max_gap` parameter),
    // floored at 0.0 so a pathological near-zero `preferred_bead_width_outer`
    // never produces a negative gap threshold.
    let max_gap = (params.preferred_bead_width_outer - 1e-6).max(0.0);
    let stitched = stitch_extrusions(lines, max_gap);
    // Canonical post-process order (WallToolPaths.cpp:679-699):
    // stitch → removeSmallLines → separateOutInnerContour → simplify → removeEmpty
    let without_small = remove_small_lines(
        stitched,
        params.min_length_factor,
        params.min_width,
        params.is_initial_layer,
    );
    let (toolpaths, _inner_contour) = separate_out_inner_contour(without_small);
    let simplified = simplify_toolpaths(
        toolpaths,
        params.visvalingam_area_threshold,
        params.smallest_line_segment_squared,
        params.allowed_error_distance_squared,
        params.maximum_extrusion_area_deviation,
    );
    let final_lines = remove_empty_toolpaths(simplified);

    Ok(final_lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{Point2, Polygon, UNITS_PER_MM};

    fn p(x: i64, y: i64) -> Point2 {
        Point2 { x, y }
    }

    fn expoly(points: Vec<Point2>) -> ExPolygon {
        ExPolygon {
            contour: Polygon { points },
            holes: Vec::new(),
        }
    }

    /// A 10mm square. See `crates/slicer-core/tests/arachne_pipeline.rs` for
    /// why this needs to be considerably larger than the millimeter-scale
    /// "unit square" fixture other `skeletal_trapezoidation` golden tests use
    /// (that smaller square's medial-axis depth never clears
    /// `ArachneParams::default()`'s `optimal_width`, so every edge's bead
    /// count would come out `0` and `generate_toolpaths` would emit nothing).
    fn square_10mm() -> ExPolygon {
        let side_units = (10.0 * UNITS_PER_MM) as i64;
        expoly(vec![
            p(0, 0),
            p(side_units, 0),
            p(side_units, side_units),
            p(0, side_units),
        ])
    }

    #[test]
    fn run_arachne_pipeline_square_produces_lines() {
        let square = square_10mm();
        let result = run_arachne_pipeline(
            std::slice::from_ref(&square),
            &ArachneParams::default(),
            false,
        );
        let lines = result.expect("10mm square should produce Ok(lines)");
        assert!(!lines.is_empty(), "expected at least one ExtrusionLine");
    }

    #[test]
    fn run_arachne_pipeline_is_deterministic() {
        let square = square_10mm();
        let params = ArachneParams::default();
        let first = run_arachne_pipeline(std::slice::from_ref(&square), &params, false)
            .expect("first run should succeed");
        let second = run_arachne_pipeline(std::slice::from_ref(&square), &params, false)
            .expect("second run should succeed");
        assert_eq!(first, second, "pipeline must be deterministic");
    }
}
