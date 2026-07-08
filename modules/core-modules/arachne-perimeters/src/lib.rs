//! Arachne perimeter generator module (M2 foundations + P112 wire-up).
//!
//! Implements `LayerModule::run_perimeters` for the `Layer::Perimeters` stage
//! by delegating to the host-service bridge
//! `slicer_sdk::host::generate_arachne_walls`, which (on native targets)
//! drives the real Arachne beading-strategy pipeline
//! (`slicer_core::arachne::pipeline::run_arachne_pipeline`) directly, and (on
//! `wasm32` guest builds) forwards the call across the WIT boundary to the
//! host, which runs the same native pipeline on the guest's behalf.
//!
//! # Honesty note (no OrcaSlicer oracle)
//!
//! This module emits real variable-width walls from the from-first-principles
//! Arachne pipeline built across packets 110-112. It does **not** claim
//! numeric parity with OrcaSlicer's `WallToolPaths`/`PerimeterGenerator` — see
//! `slicer_core::arachne::pipeline`'s own module doc comment for the honesty
//! caveats this module inherits.
//!
//! # Per-color (MMU) wall generation (P112 Step 10B)
//!
//! Painted multi-color (MMU) models do **not** need any per-color polygon
//! splitting inside this module. Splitting already happens upstream, in
//! `PrePass::PaintSegmentation` (`slicer_core::algos::paint_segmentation`):
//! each painted color becomes its own `SlicedRegion` in `SliceIR.regions`,
//! with a distinct synthesized `region_id` and a `variant_chain` entry
//! `("material", PaintValue::ToolIndex(n))`. By the time `run_perimeters` is
//! called, `regions: &[SliceRegionView]` already contains one entry per
//! paint-color cell (plus a residual/base-color region) — the same list
//! `classic-perimeters` consumes with no special-casing either (see that
//! module's own `for region in regions { output.begin_region(...); ... }`
//! loop, which never reads paint data for splitting).
//!
//! This module's loop below already mirrors that exactly: it iterates every
//! (already-split) region, calls `output.begin_region(region.object_id(),
//! *region.region_id())` before pushing that region's walls, and runs the
//! Arachne pipeline once per region's own polygon set (which may itself be
//! several disjoint islands of the *same* color — that's the "combined
//! polygon set" this emits from, not a merge across colors). Downstream, the
//! host resolves each emitted `WallLoop`'s `PrintEntity.tool_index` from
//! `SliceIR.regions[*].variant_chain` keyed by `(object_id, region_id)`
//! (`crates/slicer-runtime/src/layer_executor.rs`'s `assemble_ordered_entities`
//! / `variant_tool_by_region`, with a point-in-polygon spatial fallback) —
//! entirely independent of which perimeter generator produced the wall. So
//! per-color `T<N>` tool-change fragmentation in gcode falls out of the
//! existing per-region loop for free, with no additional plumbing required.
//!
//! The `_paint_regions: &PaintRegionLayerView` parameter itself carries no
//! usable per-color data at this stage regardless: the `#[slicer_module]`
//! macro's generated component glue
//! (`crates/slicer-macros/src/lib.rs`'s `perimeters_arm`) builds it via the
//! bare `__slicer_adapt_paint_layer` adapter with no `SliceIR`/`SupportPlanIR`
//! attached — only the `Layer::Support` arm additionally calls
//! `.with_slice_ir(...)`/`.with_support_plan(...)` before invoking the trait
//! method. So `semantics_on_layer()`/`paint_policy_for(...)` are structurally
//! empty/`DefaultEligible` here no matter what a `Layer::Perimeters` module
//! does with them. `classic-perimeters` already reflects this by leaving its
//! own `_paint` parameter unread; this module does the same, for the same
//! verified reason, rather than fabricate a read that would always be a
//! no-op in production.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    extrusion_line_to_extrusion_path3d, mm_to_units, units_to_mm, ConfigView, ExPolygon,
    ExtrusionRole, LoopType, Point2, Polygon, WallBoundaryType, WallFeatureFlags, WallLoop,
    WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::host::ArachneParams;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Arachne perimeter generator.
///
/// Holds no state. Config keys (the `BeadingStrategy` stack registered in
/// `arachne-perimeters.toml`, packet 111 T-218) are read per-invocation in
/// `run_perimeters` rather than cached here, mirroring `classic-perimeters`'
/// own R2 convention for per-object/per-layer overridable keys.
pub struct ArachnePerimeters;

/// Builds the SDK's [`ArachneParams`] mirror from `config`, falling back to
/// [`ArachneParams::default`] for any key that is absent or whose type
/// doesn't match the schema (mirrors `ConfigView::get_float`/`get_int`'s own
/// strict-type-match, no-fallback convention used throughout
/// `classic-perimeters`).
///
/// 9 of the 13 `arachne-perimeters.toml` config keys now map onto
/// `ArachneParams` fields (packet 112, Step 9C added `min_feature_size`,
/// `min_bead_width`, and `detect_thin_wall` to the 6 wired by a prior step)
/// — the remainder (`wall_transition_length`, `wall_transition_angle`,
/// `initial_layer_min_bead_width`, `outer_wall_offset`) are registered for
/// `BeadingStrategyFactory` but not yet threaded through
/// `slicer_core::arachne::pipeline::ArachneParams`'s narrower field set (see
/// that type's own doc comment). `min_central_distance`, `dp_epsilon`, and
/// `min_width` have no corresponding manifest key at all and always take
/// their pipeline defaults.
///
/// Numeric keys tagged `unit = "units"` in the manifest (integer slicer-unit
/// space, 1 unit = 100 nm) are converted to the millimeter space
/// `ArachneParams` expects via [`units_to_mm`]; `min_length_factor` is a
/// dimensionless ratio and needs no conversion.
#[rustfmt::skip]
fn arachne_params_from_config(config: &ConfigView) -> ArachneParams {
    let defaults = ArachneParams::default();

    let optimal_width = config
        .get_float("optimal_width")
        .map(|v| units_to_mm(v as i64) as f64)
        .unwrap_or(defaults.optimal_width);
    let preferred_bead_width_outer = config
        .get_float("preferred_bead_width_outer")
        .map(|v| units_to_mm(v as i64) as f64)
        .unwrap_or(defaults.preferred_bead_width_outer);
    let max_bead_count = config
        .get_int("max_bead_count")
        .map(|v| v as u32)
        .unwrap_or(defaults.max_bead_count);
    let distribution_count = config
        .get_int("wall_distribution_count")
        .map(|v| v as u32)
        .unwrap_or(defaults.distribution_count);
    let transition_filter_dist = config
        .get_float("wall_transition_filter_deviation")
        .map(|v| units_to_mm(v as i64) as f64)
        .unwrap_or(defaults.transition_filter_dist);
    let min_length_factor = config
        .get_float("min_length_factor")
        .unwrap_or(defaults.min_length_factor);
    let min_feature_size = config
        .get_float("min_feature_size")
        .map(|v| units_to_mm(v as i64) as f64)
        .unwrap_or(defaults.min_feature_size);
    let min_bead_width = config
        .get_float("min_bead_width")
        .map(|v| units_to_mm(v as i64) as f64)
        .unwrap_or(defaults.min_bead_width);
    let print_thin_walls = config
        .get_bool("detect_thin_wall")
        .unwrap_or(defaults.print_thin_walls);

    // The three closure-parity keys below now have SDK mirror fields. The
    // four legacy keys (wall_transition_length, wall_transition_angle,
    // initial_layer_min_bead_width, outer_wall_offset) are still registered in
    // the manifest and validated by reading them, but the SDK mirror has not
    // yet grown fields for them, so their values are read and discarded here.
    {
        let min_central_distance = config.get_float("min_central_distance").map(|v| units_to_mm(v as i64) as f64).unwrap_or(defaults.min_central_distance);
        let visvalingam_area_threshold = config.get_float("visvalingam_area_threshold").map(|v| units_to_mm(v as i64) as f64).unwrap_or(defaults.visvalingam_area_threshold);
        let min_width = config.get_float("min_width").map(|v| units_to_mm(v as i64) as f64).unwrap_or(defaults.min_width);

        let wall_transition_length = config.get_float("wall_transition_length").map(|v| units_to_mm(v as i64) as f64).unwrap_or(defaults.wall_transition_length);
        let wall_transition_angle = config.get_float("wall_transition_angle").map(|v| v.to_radians()).unwrap_or(defaults.wall_transition_angle);
        let initial_layer_min_bead_width = config.get_float("initial_layer_min_bead_width").map(|v| units_to_mm(v as i64) as f64).unwrap_or(defaults.initial_layer_min_bead_width);
        let outer_wall_offset = config.get_float("outer_wall_offset").map(|v| units_to_mm(v as i64) as f64).unwrap_or(defaults.outer_wall_offset);

        // Distance-gate config keys for simplify_toolpaths (N13).
        // Stored as squared mm² values; config keys are in mm (resolution/deviation).
        let smallest_line_segment_squared = config
            .get_float("meshfix_maximum_resolution")
            .map(|v| { let mm = units_to_mm(v as i64) as f64; mm * mm })
            .unwrap_or(defaults.smallest_line_segment_squared);
        let allowed_error_distance_squared = config
            .get_float("meshfix_maximum_deviation")
            .map(|v| { let mm = units_to_mm(v as i64) as f64; mm * mm })
            .unwrap_or(defaults.allowed_error_distance_squared);
        let maximum_extrusion_area_deviation = config
            .get_float("meshfix_maximum_extrusion_area_deviation")
            .map(|v| units_to_mm(v as i64) as f64)
            .unwrap_or(defaults.maximum_extrusion_area_deviation);

        ArachneParams {
            optimal_width,
            preferred_bead_width_outer,
            max_bead_count,
            distribution_count,
            transition_filter_dist,
            min_length_factor,
            min_feature_size,
            min_bead_width,
            print_thin_walls,
            min_central_distance,
            visvalingam_area_threshold,
            min_width,
            wall_transition_length,
            wall_transition_angle,
            initial_layer_min_bead_width,
            outer_wall_offset,
            is_initial_layer: false,
            smallest_line_segment_squared,
            allowed_error_distance_squared,
            maximum_extrusion_area_deviation,
        }
    }
}

/// Classifies a single [`slicer_ir::ExtrusionLine`] into its `(role,
/// loop_type)` pair.
///
/// `is_odd` lines (odd-width transition regions, per `ExtrusionLine::is_odd`'s
/// own doc comment) are treated as gap-fill regardless of `inset_idx`;
/// otherwise `inset_idx == 0` is the outermost wall and anything deeper is an
/// inner wall.
fn classify_line(line: &slicer_ir::ExtrusionLine) -> (ExtrusionRole, LoopType) {
    if line.is_odd {
        (ExtrusionRole::GapFill, LoopType::GapFill)
    } else if line.inset_idx == 0 {
        (ExtrusionRole::OuterWall, LoopType::Outer)
    } else {
        (ExtrusionRole::InnerWall, LoopType::Inner)
    }
}

#[slicer_module]
impl LayerModule for ArachnePerimeters {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }

    /// `_paint_regions` is intentionally unread — see this module's top-level
    /// doc comment ("Per-color (MMU) wall generation", P112 Step 10B) for why:
    /// it carries no `SliceIR`/`SupportPlanIR` enrichment at the
    /// `Layer::Perimeters` stage (only `Layer::Support` gets that), and
    /// per-color fragmentation is already achieved by iterating the
    /// pre-split `regions` list below — exactly mirroring
    /// `classic-perimeters`' own (also paint-data-unread) loop. Runs the
    /// pipeline once per region's own combined polygon set (that region's
    /// possibly-multiple same-color islands, NOT a merge across colors —
    /// each paint color already arrived as its own region).
    fn run_perimeters(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        _paint_regions: &PaintRegionLayerView,
        output: &mut PerimeterOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let mut params = arachne_params_from_config(config);
        params.is_initial_layer = layer_index == 0;

        // Per-color (MMU) wiring (P112 Step 10B): `regions` already contains
        // one entry per paint-color cell (see PrePass::PaintSegmentation) plus
        // any residual base-color region — no per-tool grouping/splitting is
        // needed here. `begin_region` below tags each pushed WallLoop with its
        // region's (object_id, region_id) origin, which the host resolves to
        // a tool_index via `SliceIR.regions[*].variant_chain`
        // (`layer_executor.rs::assemble_ordered_entities`), independent of
        // this loop — so single-region (unpainted) and multi-region (painted)
        // inputs are handled by the exact same code path below.
        for region in regions {
            output.begin_region(region.object_id(), *region.region_id());
            let polygons = region.polygons();
            if polygons.is_empty() {
                continue;
            }
            let z = region.z();

            let (lines, inner_contour) =
                match slicer_sdk::host::generate_arachne_walls(polygons, &params) {
                    Ok(result) => result,
                    Err(e) => {
                        // A single region's geometry failing the pipeline (e.g.
                        // preprocessing reduces it to nothing) must not abort
                        // perimeter generation for every other region on this
                        // layer — log and move on, mirroring classic-perimeters'
                        // own permissive handling of medial-axis failures.
                        slicer_sdk::host::log_warn(&format!(
                            "arachne-perimeters: generate_arachne_walls failed for region \
                             object_id={} region_id={}: {e}",
                            region.object_id(),
                            region.region_id()
                        ));
                        continue;
                    }
                };

            let mut walls: Vec<WallLoop> = Vec::with_capacity(lines.len());
            for line in &lines {
                let (role, loop_type) = classify_line(line);
                let mut path = extrusion_line_to_extrusion_path3d(line, role);
                if path.points.is_empty() {
                    continue;
                }
                for pt in &mut path.points {
                    pt.z = z;
                }
                let num_points = path.points.len();
                let widths: Vec<f32> = path.points.iter().map(|p| p.width).collect();

                walls.push(WallLoop {
                    perimeter_index: line.inset_idx,
                    loop_type,
                    path,
                    width_profile: WidthProfile { widths },
                    feature_flags: vec![WallFeatureFlags::default(); num_points],
                    boundary_type: WallBoundaryType::Interior,
                });
            }

            // AC-9: walls sorted by perimeter_index ascending. The pipeline's
            // own `generate_toolpaths` bucketizes by ascending inset_idx
            // already (`BTreeMap`-backed), but a stable sort here makes the
            // ordering an explicit, guaranteed contract of this module's
            // output rather than an incidental property of upstream stage
            // internals.
            walls.sort_by_key(|w| w.perimeter_index);

            for wall in walls {
                output.push_wall_loop(wall)?;
            }

            // Convert inner-contour marker lines to infill area polygons.
            // Matches canonical WallToolPaths::separateOutInnerContour
            // (line 905): skip odd lines (centerline single beads), convert
            // closed even lines to polygons, then union to normalize winding.
            let infill_candidates: Vec<ExPolygon> = inner_contour
                .iter()
                .filter(|line| !line.is_odd && line.is_closed)
                .map(|line| ExPolygon {
                    contour: Polygon {
                        points: line
                            .junctions
                            .iter()
                            .map(|j| Point2 {
                                x: mm_to_units(j.p.x),
                                y: mm_to_units(j.p.y),
                            })
                            .collect(),
                    },
                    holes: Vec::new(),
                })
                .collect();
            if !infill_candidates.is_empty() {
                let infill_areas = slicer_sdk::host::clip_polygons(
                    &infill_candidates,
                    &[],
                    slicer_sdk::host::ClipOperation::Union,
                );
                if !infill_areas.is_empty() {
                    output.set_infill_areas(infill_areas)?;
                }
            }
        }

        Ok(())
    }
}
