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

use slicer_core::flow::{bridging_flow, flow_to_width, line_width_to_spacing};
use slicer_core::perimeter_utils::{
    build_wall_flags, generate_sharp_corner_seam_candidates, point_in_any_polygon,
    wall_sequence_reorder,
};
use slicer_core::polygon_ops::{difference_ex, offset2_ex, OffsetJoinType};
use slicer_ir::{
    extrusion_line_to_extrusion_path3d, mm_to_units, point_in_polygon_winding, units_to_mm,
    ConfigView, ExPolygon, ExtrusionLine, ExtrusionRole, LoopType, Point2, Polygon,
    WallBoundaryType, WallLoop, WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::host::ArachneParams;
use slicer_sdk::host::WallSequence;
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
///
/// Shoelace signed area of a closed/open path's projected (x, y) points.
/// Positive ⇒ counter-clockwise (CCW), negative ⇒ clockwise (CW). Only the
/// SIGN matters for G1 winding normalization — magnitudes are in whatever
/// coordinate space `Point3WithWidth.x/.y` occupy (mm here), so no
/// scaling is needed.
fn signed_area_of_points(pts: &[slicer_ir::Point3WithWidth]) -> f64 {
    if pts.len() < 2 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..pts.len() {
        let a = &pts[i];
        let b = &pts[(i + 1) % pts.len()];
        area += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
    }
    area / 2.0
}

#[rustfmt::skip]
fn arachne_params_from_config(config: &ConfigView) -> ArachneParams {
    let defaults = ArachneParams::default();

    // `layer_height`/`nozzle_diameter` are Z-axis-convention / physical-spec
    // keys (docs/08_coordinate_system.md): stored and read as plain mm floats,
    // never scaled units — no `units_to_mm` conversion, unlike the bead-width
    // keys below. Defaults mirror layer-planner-default.toml's layer_height
    // default (0.2mm) and the 0.4mm nozzle convention.
    let layer_height_mm = config.get_float("layer_height").unwrap_or(0.2);
    let nozzle_diameter_mm = config.get_float("nozzle_diameter").unwrap_or(0.4);

    // AC-3: feed Flow SPACING (not raw width) to the beading pipeline.
    // OrcaSlicer constructs `WallToolPaths(outline, bead_width_0, bead_width_x,
    // ...)` with bead_width_0 = ext_perimeter_spacing (the OUTER wall's
    // spacing) and bead_width_x = perimeter_spacing = the INNER wall's
    // `perimeter_flow.scaled_spacing()` (`PerimeterGenerator.cpp`). Convert the
    // raw configured line width to spacing via line_width_to_spacing (mm in, mm
    // out) BEFORE handing it to the beading strategy stack.
    //
    // Correction 2026-07-16: this comment used to say `optimal_width` "stands
    // in for OrcaSlicer's ext_perimeter_spacing" — i.e. bead_width_0, the OUTER
    // width. That is wrong, and it contradicted `ArachneParams`' own docs.
    // Canonical `BeadingStrategyFactory::makeStrategy(preferred_bead_width_outer,
    // preferred_bead_width_inner, ...)` — called as `makeStrategy(bead_width_0,
    // bead_width_x, ...)` — sets its internal `optimal_width` local to
    // `preferred_bead_width_inner` whenever `max_bead_count > 2`. So
    // `optimal_width` maps to the INNER width (`bead_width_x`), exactly as this
    // module's manifest entry for the key already recorded. The outer width is
    // `preferred_bead_width_outer` (= bead_width_0), handled separately below.
    //
    // Sourcing (D-160 Bug A): canonical derives both bead-width targets from
    // the USER's wall flows — `bead_width_x = perimeter_spacing =
    // perimeter_flow.scaled_spacing()` (the inner wall width) and
    // `bead_width_0 = ext_perimeter_spacing = ext_perimeter_flow.scaled_spacing()`
    // (the outer wall width), per `PerimeterGenerator.cpp`. This module used to
    // read two Arachne-internal knobs (`optimal_width`,
    // `preferred_bead_width_outer`, in scaled units) that nothing set, so
    // arachne output was INVARIANT to `outer_wall_line_width` /
    // `inner_wall_line_width`. Those keys are retired; the wall-width keys are
    // plain mm (no units_to_mm), same as classic-perimeters reads them.
    let optimal_width = {
        let raw_width_mm = config
            .get_float("inner_wall_line_width")
            .unwrap_or(defaults.optimal_width);
        let spacing = line_width_to_spacing(
            raw_width_mm as f32,
            layer_height_mm as f32,
            nozzle_diameter_mm as f32,
        ) as f64;
        // line_width_to_spacing returns 0 for degenerate width<=layer_height
        // configs (Orca asserts width>=height); fall back to raw width to
        // avoid zero-spacing bead collapse.
        if spacing <= 0.0 {
            raw_width_mm
        } else {
            spacing
        }
    };
    // AC-3 (cont'd): OrcaSlicer sets `bead_width_0 = ext_perimeter_spacing`
    // (PerimeterGenerator.cpp:2129) — the outer bead's target width fed to the
    // beading engine is ALSO the spacing value, not the raw configured width.
    // Keep the raw mm width separately (`preferred_bead_width_outer_raw`) for
    // the precise_outer_wall inset formula below, which mirrors OrcaSlicer's
    // `wall_0_inset = -(ext_perimeter_width/2 - ext_perimeter_spacing/2)` and
    // needs the true (unconverted) `ext_perimeter_width`.
    let preferred_bead_width_outer_raw = config
        .get_float("outer_wall_line_width")
        .unwrap_or(defaults.preferred_bead_width_outer);
    let preferred_bead_width_outer_spacing = line_width_to_spacing(
        preferred_bead_width_outer_raw as f32,
        layer_height_mm as f32,
        nozzle_diameter_mm as f32,
    ) as f64;
    // line_width_to_spacing returns 0 for degenerate width<=layer_height
    // configs (Orca asserts width>=height); fall back to raw width to avoid
    // zero-spacing bead collapse.
    let preferred_bead_width_outer = if preferred_bead_width_outer_spacing <= 0.0 {
        preferred_bead_width_outer_raw
    } else {
        preferred_bead_width_outer_spacing
    };
    let max_bead_count_explicit = config.get_int("max_bead_count");
    let wall_count = config.get_int("wall_count").map(|v| v.max(0) as u32).unwrap_or(3);
    // OrcaSlicer has no user-facing max_bead_count; it is always `2 * inset_count`
    // (`WallToolPaths.cpp:525`) and therefore ALWAYS EVEN. `LimitedBeadingStrategy`
    // warns on an odd cap and its odd-center `compute` branch (`:71-74`) dumps the
    // entire surplus region thickness into a single wide centre bead — up to ~12 mm
    // on a benchy hull's thick medial spine (the D4 taper over-extrusion, surfaced
    // once D5 made those non-central peaks emit). A zero/absent value means
    // "auto-derive `2 * wall_count`" (even, and — unlike a fixed manifest default —
    // actually tracks wall_count). A positive value is an explicit advanced override
    // (e.g. the `max_bead_count_cap` parity fixture) and is honoured verbatim.
    let max_bead_count = match max_bead_count_explicit {
        Some(v) if v > 0 => v as u32,
        _ => (2 * wall_count).max(1),
    };
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
    // `min_feature_size` is now `percent` (packet 150, D-104h): resolve via
    // get_abs_value against nozzle_diameter (mm), which returns an
    // already-absolute mm value directly (no units_to_mm needed) — matches
    // the old get_float+units_to_mm result unit (mm) for the same 25%-of-0.4mm
    // default (25% * 0.4mm = 0.1mm == units_to_mm(1000)).
    let min_feature_size = config
        .get_abs_value("min_feature_size", nozzle_diameter_mm)
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

        // `wall_transition_length` is now `percent` (packet 150, D-104h): resolve
        // via get_abs_value against nozzle_diameter (mm) — already-absolute mm,
        // matching the old get_float+units_to_mm result unit (mm) for the same
        // 100%-of-0.4mm default.
        let wall_transition_length = config.get_abs_value("wall_transition_length", nozzle_diameter_mm).unwrap_or(defaults.wall_transition_length);
        let wall_transition_angle = config.get_float("wall_transition_angle").map(|v| v.to_radians()).unwrap_or(defaults.wall_transition_angle);
        let initial_layer_min_bead_width = config.get_float("initial_layer_min_bead_width").map(|v| units_to_mm(v as i64) as f64).unwrap_or(defaults.initial_layer_min_bead_width);

        // Precise outer wall (packet 148, Step 6; corrected packet 150, Step
        // 4): mirrors OrcaSlicer's `PerimeterGenerator.cpp:2146-2158`
        // `apply_precise_outer_wall = precise_outer_wall && wall_sequence ==
        // InnerOuter` gate, and `wall_0_inset = -(ext_perimeter_width/2 -
        // ext_perimeter_spacing/2)` when the gate is satisfied (else 0).
        // `preferred_bead_width_outer` stands in for upstream's
        // `ext_perimeter_width` (it is this pipeline's own dedicated
        // "outermost bead" width field, per its own doc comment). BOTH terms
        // of the inset must derive from the SAME outer/external flow
        // (`ext_perimeter_width` / `ext_perimeter_spacing`), not the general
        // wall's `optimal_width` — so this pairs `preferred_bead_width_outer_raw`
        // (the true unconverted `ext_perimeter_width`) with
        // `preferred_bead_width_outer` (its own spacing, computed above via
        // `line_width_to_spacing`, mirroring `ext_perimeter_spacing`),
        // matching `wall_0_inset = -(ext_perimeter_width/2 -
        // ext_perimeter_spacing/2)` exactly. (Packet 150 Step 4 first wired
        // this to `optimal_width` — the general/inner wall's spacing — which
        // was wrong: it mixed the outer wall's raw width with a different
        // wall's spacing.)
        // `outer_wall_offset` remains directly overridable via its own
        // registered config key (read above pre-gate as the base/manual
        // value) when the precise-outer-wall gate does not apply, preserving
        // that key's own independent meaning.
        let manual_outer_wall_offset = config.get_float("outer_wall_offset").map(|v| units_to_mm(v as i64) as f64).unwrap_or(defaults.outer_wall_offset);
        let precise_outer_wall = config.get_bool("precise_outer_wall").unwrap_or(false);
        let wall_sequence_is_inner_outer = config
            .get_string("wall_sequence")
            .map(|s| s == "InnerOuter")
            .unwrap_or(true); // default wall_sequence is "InnerOuter"
        let outer_wall_offset = if precise_outer_wall && wall_sequence_is_inner_outer {
            -((preferred_bead_width_outer_raw / 2.0) - (preferred_bead_width_outer / 2.0))
        } else {
            manual_outer_wall_offset
        };

        // G9 (packet 151, Step 5): OrcaSlicer coFloat keys
        // wall_maximum_resolution / wall_maximum_deviation REPLACE
        // meshfix_maximum_resolution / meshfix_maximum_deviation for the Arachne
        // wall simplification tolerances (WallToolPaths.cpp:487-503,702-719).
        // Values are plain mm (config key units); square directly into mm². NO
        // coordinate ÷100 — these are mm-based scalar tolerances, not toolpath
        // coordinates (Orca research: REPLACE, not min()/merge).
        let smallest_line_segment_squared = config
            .get_float("wall_maximum_resolution")
            .map(|v| v * v)
            .unwrap_or(defaults.smallest_line_segment_squared);
        let allowed_error_distance_squared = config
            .get_float("wall_maximum_deviation")
            .map(|v| v * v)
            .unwrap_or(defaults.allowed_error_distance_squared);
        // The third tolerance (meshfix_maximum_extrusion_area_deviation) is a
        // distinct parameter unrelated to the resolution/deviation pair; the
        // packet does not touch it, so it keeps its pipeline default.
        let maximum_extrusion_area_deviation = defaults.maximum_extrusion_area_deviation;

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
            is_bottom_layer: false,
            is_topmost_layer: false,
            smallest_line_segment_squared,
            allowed_error_distance_squared,
            maximum_extrusion_area_deviation,
            wall_sequence: WallSequence::InnerOuter,
        }
    }
}

/// Classifies a single [`slicer_ir::ExtrusionLine`] into its `(role,
/// loop_type)` pair.
///
/// `is_odd` lines (odd-width transition regions, per `ExtrusionLine::is_odd`'s
/// own doc comment) are treated as gap-fill regardless of `inset_idx`, with
/// one exception (packet 148, AC-2): an `is_odd` line at `inset_idx == 0`
/// with `print_thin_walls` enabled is the single widened center-line bead
/// `WideningBeadingStrategy` produces for a region thinner than one full
/// bead. OrcaSlicer's own Arachne path assigns this bead no special
/// thin-wall/gap-fill role at all — the Arachne skeletal-graph algorithm has
/// no such role; `is_odd` is purely structural, and every emitted
/// `ExtrusionLine` becomes `erExternalPerimeter`/`erPerimeter` via
/// `inset_idx == 0` (`PerimeterGenerator.cpp:383-384`); `print_thin_walls`
/// only gates whether `WideningBeadingStrategy` runs at all. `LoopType::ThinWall`
/// here is PnP's own IR-level semantic refinement of that same bead — a
/// deliberate deviation from upstream, not a ported behavior — allowing
/// downstream consumers (feature-flag/report tooling) to distinguish a
/// widened thin region from an ordinary outer wall. Deeper odd lines
/// (`inset_idx > 0`, transition regions between full beads) stay `GapFill`
/// regardless of `print_thin_walls`; classic-perimeters' `medial_axis`-based
/// thin-wall gate does not apply here since Arachne's beading strategy
/// already produces the widened bead directly.
fn classify_line(
    line: &slicer_ir::ExtrusionLine,
    print_thin_walls: bool,
) -> (ExtrusionRole, LoopType) {
    if line.is_odd && line.inset_idx == 0 && print_thin_walls {
        (ExtrusionRole::ThinWall, LoopType::ThinWall)
    } else if line.is_odd {
        (ExtrusionRole::GapFill, LoopType::GapFill)
    } else if line.inset_idx == 0 {
        (ExtrusionRole::OuterWall, LoopType::Outer)
    } else {
        (ExtrusionRole::InnerWall, LoopType::Inner)
    }
}

fn commit_wall_sequence(walls: &mut Vec<WallLoop>, sequence: WallSequence) {
    if walls.len() <= 1 {
        return;
    }

    // The finalized Arachne lines carry path order, but the native and WIT
    // paths may expose the same region in opposite orientation. Normalize that
    // orientation from wall identity, never from perimeter_index. This keeps
    // each region's committed batch intact, so a later region can never be
    // pulled ahead merely because its outer index is smaller.
    let outer_at_front = walls
        .first()
        .is_some_and(|wall| wall.loop_type == LoopType::Outer);
    let outer_at_back = walls
        .last()
        .is_some_and(|wall| wall.loop_type == LoopType::Outer);
    match sequence {
        WallSequence::InnerOuter if outer_at_back && !outer_at_front => walls.reverse(),
        WallSequence::OuterInner if outer_at_front && !outer_at_back => walls.reverse(),
        WallSequence::InnerOuterInner => {
            if outer_at_back && !outer_at_front {
                walls.reverse();
            }
            wall_sequence_reorder(walls, sequence, &[]);
        }
        _ => {}
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
        // is_bottom_layer keys the classic "first/last layer" threshold (layer 0
        // in object coordinates). PnP historically folded this into
        // is_initial_layer; both flags are kept distinct so downstream flag
        // derivation (is_top_or_bottom_layer = is_bottom_layer ||
        // is_topmost_layer, G10) can be wired later — and both fire on layer 0.
        let is_bottom_layer = layer_index == 0;
        params.is_initial_layer = layer_index == 0;
        params.is_bottom_layer = is_bottom_layer;
        let wall_sequence = config.get_string("wall_sequence").unwrap_or("InnerOuter");
        params.wall_sequence = match wall_sequence {
            "OuterInner" => WallSequence::OuterInner,
            "InnerOuterInner" => WallSequence::InnerOuterInner,
            _ => WallSequence::InnerOuter,
        };
        // Orca enables sandwich ordering only after the first layer. The
        // pipeline has already produced the finalized region order; this is
        // the sole module-side commit-time sequence adjustment.
        let sequence = if params.wall_sequence == WallSequence::InnerOuterInner && layer_index == 0
        {
            WallSequence::InnerOuter
        } else {
            params.wall_sequence
        };

        // only_one_wall_top (G3 part 1): the config key round-trips correctly
        // and is now CONSUMED (D-104d deferred behavior begins here). When set
        // and the region is the topmost shell (top_shell_index == Some(0) in the
        // PnP IR — Orca's upper_slices == nullptr), the wall stack is collapsed
        // to a single wall (OrcaSlicer PerimeterGenerator.cpp:2140-2144 forces
        // loop_number = 0 on the topmost layer). The actual clamp to one emitted
        // wall happens per-region inside the region loop below, because
        // topmost-ness is region metadata, not a layer-wide property.
        let only_one_wall_top = config.get_bool("only_one_wall_top").unwrap_or(false);

        // wall_direction (packet 151, Step 2 / G1): OrcaSlicer coEnum
        // wall_direction (PrintConfig.cpp:2188-2198, default CounterClockwise)
        // forces contour (ExteriorSurface) winding CCW or CW via
        // make_counter_clockwise/make_clockwise (PerimeterGenerator.cpp:527-545),
        // holes always opposite. Default "counter_clockwise" must reproduce the
        // prior default winding (AC-N2); absence from raw config falls back to
        // CCW so no regression is introduced.
        let wall_direction = config
            .get_string("wall_direction")
            .unwrap_or("counter_clockwise");
        let contour_should_be_ccw = wall_direction != "clockwise";

        // alternate_extra_wall (T-149 AC-3, D-104e closed): OrcaSlicer adds
        // one extra wall loop on every second (0-indexed-odd) layer by
        // incrementing `loop_number` before constructing `WallToolPaths`
        // (`PrintConfig.cpp:5059-5066`; `WallToolPaths(..., loop_number + 1,
        // ...)` with `max_bead_count = 2 * inset_count` inside the
        // beading-strategy factory). Skipped when `spiral_vase` is on (a
        // spiral print has no discrete wall stack to alternate) or
        // `sparse_infill_density <= 0.0` (a solid/no-infill part has no
        // interior room to grow an extra wall into). Applied here as a
        // post-construction bump to `params.max_bead_count` — the
        // beading-stack's own input cap, not a post-hoc wall-count mutation
        // downstream of it, mirroring how `params.is_initial_layer` is set
        // just above (this function has no access to `layer_index`).
        let alternate_extra_wall = config.get_bool("alternate_extra_wall").unwrap_or(false);
        let spiral_vase = config.get_bool("spiral_vase").unwrap_or(false);
        let sparse_infill_density = config.get_float("sparse_infill_density").unwrap_or(20.0);
        if alternate_extra_wall
            && layer_index % 2 == 1
            && !spiral_vase
            && sparse_infill_density > 0.0
        {
            // Empirically measured (see alternate_extra_wall_tdd.rs's module
            // doc comment): `LimitedBeadingStrategy` inserts a symmetric
            // sentinel pair that `remove_small_lines` filters as zero-width
            // (beading/limited.rs's own doc comment), so the emitted wall
            // count for an even `max_bead_count` is `max_bead_count / 2`; a
            // `+1` bump does not reliably cross that /2 floor into an extra
            // emitted wall, but `+2` does (same parity class, +1 emitted
            // wall) — matching OrcaSlicer's own `max_bead_count = 2 *
            // inset_count` relation (one extra `inset_count` step == two
            // `max_bead_count` units).
            params.max_bead_count += 2;
        }

        // only_one_wall_first_layer (packet 151, Step 3 / G2): OrcaSlicer
        // coBool key only_one_wall_first_layer forces a single perimeter wall on
        // the first layer by setting `loop_number = 0` before constructing
        // `WallToolPaths` (PerimeterGenerator.cpp:2137-2139). We achieve the
        // same effect by clamping `max_bead_count` to 2 (one wall == two beads
        // in the OverhangRestricted / LimitedBeadingStrategy logic). Placed
        // after the `alternate_extra_wall` block above so the clamp wins —
        // though the two conditions never co-fire (`is_initial_layer` is only
        // true for layer_index == 0, while `alternate_extra_wall` only fires on
        // odd layers).
        let only_one_wall_first_layer = config
            .get_bool("only_one_wall_first_layer")
            .unwrap_or(false);
        if only_one_wall_first_layer && (params.is_initial_layer || is_bottom_layer) {
            params.max_bead_count = 2;
        }

        // G7 (packet 151, Step 4): overhang-reverse winding. OrcaSlicer
        // coBool key overhang_reverse flips the print direction of wall loops
        // on odd layers containing overhang wall segments (anti-warping;
        // PerimeterGenerator.cpp:68-77). This packet only wires the parity
        // flip (detect_overhang_wall disabled + overhang_reverse enabled);
        // the full steep-overhang threshold detection is out of scope.
        // Read all three keys (the third is advisory for now).
        let detect_overhang_wall = config.get_bool("detect_overhang_wall").unwrap_or(true);
        let overhang_reverse = config.get_bool("overhang_reverse").unwrap_or(false);
        let _overhang_reverse_internal_only = config
            .get_bool("overhang_reverse_internal_only")
            .unwrap_or(false);
        let _overhang_reverse_threshold = config
            .get_float("overhang_reverse_threshold")
            .unwrap_or(0.0);
        // Compose: when detect_overhang_wall==false && overhang_reverse==true,
        // reverse contour winding on odd layers.
        let g7_reverse = !detect_overhang_wall && overhang_reverse && (layer_index % 2 == 1);

        // bridge_flow / thick_bridges (packet 149, D4/D-104g): read once per
        // invocation, applied per-vertex below wherever is_bridge is true.
        let bridge_flow_ratio = config
            .get_float("bridge_flow")
            .map(|v| v as f32)
            .unwrap_or(1.0);
        let thick_bridges = config.get_bool("thick_bridges").unwrap_or(false);
        // nozzle_diameter/layer_height (packet 150 step 5): re-read here (mm,
        // same config keys + defaults as arachne_params_from_config's own
        // local reads above) so the thick_bridges round-cross-section
        // formula below has the physical-spec inputs it needs; ArachneParams
        // does not carry them back out of that function.
        let nozzle_diameter_mm = config
            .get_float("nozzle_diameter")
            .map(|v| v as f32)
            .unwrap_or(0.4);
        let layer_height_mm = config
            .get_float("layer_height")
            .map(|v| v as f32)
            .unwrap_or(0.2);

        // Per-color (MMU) wiring (P112 Step 10B): `regions` already contains
        // one entry per paint-color cell (see PrePass::PaintSegmentation) plus
        // any residual base-color region — no per-tool grouping/splitting is
        // needed here. `begin_region` below tags each pushed WallLoop with its
        // region's (object_id, region_id) origin, which the host resolves to
        // a tool_index via `SliceIR.regions[*].variant_chain`
        // (`layer_executor.rs::assemble_ordered_entities`), independent of
        // this loop — so single-region (unpainted) and multi-region (painted)
        // inputs are handled by the exact same code path below.
        // Base bead cap after the first-layer / alternate-extra-wall clamps
        // above; captured once so the per-region topmost clamp below can reset
        // to it for non-topmost regions (a topmost region in a mixed layer must
        // not clamp its siblings).
        let base_max_bead_count = params.max_bead_count;
        for region in regions {
            // G3 part 1: a single wall on the topmost shell when only_one_wall_top
            // is enabled. top_shell_index == Some(0) marks the exposed top
            // (Orca's upper_slices == nullptr); PerimeterGenerator.cpp:2140-2144
            // forces loop_number = 0 there, which we render as a single emitted
            // wall (max_bead_count clamped to 2 == one wall, mirroring
            // only_one_wall_first_layer). Per-region because topmost-ness is
            // region metadata.
            let is_topmost_layer = region.top_shell_index() == Some(0);
            params.is_topmost_layer = is_topmost_layer;
            if only_one_wall_top && is_topmost_layer {
                params.max_bead_count = 2;
            } else {
                params.max_bead_count = base_max_bead_count;
            }
            output.begin_region(region.object_id(), *region.region_id());
            let polygons = region.polygons();
            if polygons.is_empty() {
                continue;
            }
            let z = region.z();

            // G3 part 2: non-topmost region with a top sub-area. When
            // `only_one_wall_top` is enabled, the region is NOT the exposed top
            // shell (`top_shell_index != Some(0)`, i.e. Orca's
            // `upper_slices != nullptr`), AND the host supplied a top-region
            // polygon, run the second `WallToolPaths` pass over the non-top
            // remainder and merge it with the single top-area wall. The top-area
            // source is `top_solid_fill` (PnP's precomputed top-region polygon);
            // see `D-152-TOP-AREA-SOURCE` for the divergence from Orca's
            // `diff_ex(infill_contour, upper_slices_clipped)`.
            if only_one_wall_top && !is_topmost_layer && !region.top_solid_fill().is_empty() {
                self.emit_only_one_wall_top_second_pass(
                    region,
                    polygons,
                    z,
                    &params,
                    base_max_bead_count,
                    config,
                    contour_should_be_ccw,
                    g7_reverse,
                    layer_height_mm,
                    nozzle_diameter_mm,
                    bridge_flow_ratio,
                    thick_bridges,
                    output,
                )?;
                continue;
            }

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

            // Walls are built from the Arachne `ExtrusionLine`s via `build_walls`
            // (the same path the G3 part-2 second pass uses); `bridge_areas` /
            // `overhang_bands` are recomputed inside `build_walls` per call.
            let mut walls = self.build_walls(
                &lines,
                region,
                z,
                polygons,
                &params,
                contour_should_be_ccw,
                g7_reverse,
                layer_height_mm,
                nozzle_diameter_mm,
                bridge_flow_ratio,
                thick_bridges,
            )?;
            commit_wall_sequence(&mut walls, sequence);
            for wall in walls {
                output.push_wall_loop(wall)?;
            }
            // AC-6 (packet 148): sharp-corner seam candidates, once per
            // region polygon (island), against each input polygon's outer
            // contour (units-space `slicer_ir::Polygon`) — NOT the mm-space
            // wall path. Mirrors classic-perimeters' own per-island loop
            // (`for (poly_idx, poly) in outer_polys.iter().enumerate()`,
            // lib.rs ~888-902): a region may contain several disjoint
            // islands of the same color, and each island's own sharp
            // corners must contribute candidates, not just the first
            // island's. Holes are not iterated — contours only, same as
            // classic.
            let seam_candidate_angle_threshold_deg = config
                .get_float("seam_candidate_angle_threshold_deg")
                .unwrap_or(30.0) as f32;
            for polygon in polygons {
                let candidates = generate_sharp_corner_seam_candidates(
                    &polygon.contour,
                    z,
                    seam_candidate_angle_threshold_deg,
                );
                for candidate in candidates {
                    output.push_seam_candidate(candidate.position, candidate.score)?;
                }
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

impl ArachnePerimeters {
    /// Builds `WallLoop`s from Arachne `ExtrusionLine`s, applying the same
    /// bridge/overhang/thin-wall/winding normalization as the single-pass
    /// emission path (`run_perimeters`'s per-region loop).
    fn build_walls(
        &self,
        lines: &[ExtrusionLine],
        region: &SliceRegionView,
        z: f32,
        polygons: &[ExPolygon],
        params: &ArachneParams,
        contour_should_be_ccw: bool,
        g7_reverse: bool,
        layer_height_mm: f32,
        nozzle_diameter_mm: f32,
        bridge_flow_ratio: f32,
        thick_bridges: bool,
    ) -> Result<Vec<WallLoop>, ModuleError> {
        let bridge_areas = region.bridge_areas();
        let overhang_bands = region.overhang_quartile_polygons();
        let mut walls: Vec<WallLoop> = Vec::with_capacity(lines.len());
        for line in lines {
            let (role, loop_type) = classify_line(line, params.print_thin_walls);
            let mut path = extrusion_line_to_extrusion_path3d(line, role);
            if path.points.is_empty() {
                continue;
            }
            for pt in &mut path.points {
                pt.z = z;
                // Beading junction widths are Flow SPACING values, not extrusion
                // widths: `arachne_params_from_config` feeds the strategy stack
                // `line_width_to_spacing(...)` because canonical does
                // (`bead_width_0 = ext_perimeter_spacing`). Canonical converts
                // back at emission — `VariableWidth.cpp::thick_polyline_to_multi_path`
                // does `flow.with_width(unscale(w) + height * (1 - PI/4))`, reached
                // from `PerimeterGenerator.cpp::traverse_extrusions` via
                // `extrusion_paths_append(dst, ExtrusionLine, role, flow)`. This
                // is that conversion, at the matching seam (the ExtrusionLine ->
                // path boundary), and it must run BEFORE `widths` is snapshotted
                // below so the width_profile carries the true width too.
                //
                // Omitting it emitted spacing as width: 0.3571mm for a 0.4mm
                // wall, ~10.7% narrow on every arachne wall at default config
                // (D-160). Do NOT "simplify" this away — `classic-perimeters`
                // needs no such step only because it never converts its widths
                // to spacing in the first place (see its `bead_flow_width_mm`
                // comment); arachne does, so it must convert back.
                pt.width = flow_to_width(pt.width, layer_height_mm);
            }
            let num_points = path.points.len();
            let widths: Vec<f32> = path.points.iter().map(|p| p.width).collect();

            // D-154: Arachne's beading-derived vertices have no index
            // correspondence to `polygons`' original vertex ordering — not
            // even for the outer wall, unlike classic-perimeters' simple
            // polygon-offset walls — so paint attribution always uses
            // `build_wall_flags`'s geometric-reprojection path (nearest
            // original vertex/edge) rather than its index-based fallback.
            // `ring_pts_units` (mm→units, matching `polygons`' coordinate
            // space) is also reused below for the bridge-area lookup.
            let ring_pts_units: Vec<Point2> = path
                .points
                .iter()
                .map(|p| Point2 {
                    x: mm_to_units(p.x),
                    y: mm_to_units(p.y),
                })
                .collect();
            let is_outer = line.inset_idx == 0;
            let (mut feature_flags, boundary_type) = build_wall_flags(
                num_points,
                usize::MAX, // unused by the reprojection path; no single original poly_idx applies to a beading-derived wall
                region.segment_annotations(),
                is_outer,
                Some(&ring_pts_units),
                Some(polygons),
                false, // D14 painted-variant fuzzy skin (variant_chain) is not wired into arachne-perimeters yet — out of scope here
            );

            // AC-3 (packet 148): is_thin_wall is set on every vertex of a
            // ThinWall wall only — never on Outer/Inner walls, even if
            // geometrically narrow (mirrors classic-perimeters' own
            // is_thin_wall flag shape).
            if loop_type == LoopType::ThinWall {
                for flag in &mut feature_flags {
                    flag.is_thin_wall = true;
                }
            }

            // AC-4 (packet 148): is_bridge is set per-vertex, ONLY on
            // Outer/Inner walls (never ThinWall/GapFill/NonPlanarShell),
            // for every vertex whose path point lies inside one of the
            // region's bridge areas.
            if !bridge_areas.is_empty() && matches!(loop_type, LoopType::Outer | LoopType::Inner) {
                for i in 0..path.points.len() {
                    let units_pt = ring_pts_units[i];
                    if point_in_any_polygon(&units_pt, bridge_areas) {
                        feature_flags[i].is_bridge = true;
                        // D4/packet 150 step 5: bridge vertices get the
                        // bridging flow factor, whose round-cross-section
                        // formula needs the true mm flow width.
                        //
                        // This used to call `flow_to_width(path.points[i].width,
                        // ...)` to recover that width, because the vertex still
                        // held a raw beading SPACING. It no longer does — the
                        // spacing -> width conversion now happens once, at the
                        // emission seam above, per canonical
                        // `thick_polyline_to_multi_path`. Converting again here
                        // would add `height * (1 - PI/4)` twice and over-state
                        // bridge flow.
                        path.points[i].flow_factor = bridging_flow(
                            bridge_flow_ratio,
                            thick_bridges,
                            nozzle_diameter_mm,
                            path.points[i].width,
                            layer_height_mm,
                        );
                    }
                }
            }

            // AC-5 (packet 148): overhang_quartile is set per-vertex on
            // every wall type, for every vertex whose path point lies
            // inside a `overhang_quartile_polygons` band's polygon(s).
            if !overhang_bands.is_empty() {
                for pt in &mut path.points {
                    pt.overhang_quartile = overhang_bands
                        .iter()
                        .filter(|band| {
                            band.polygons.iter().any(|poly| {
                                point_in_polygon_winding(poly, pt.x as f64, pt.y as f64, 0.0)
                            })
                        })
                        .map(|band| band.quartile)
                        .max();
                }
            }

            // G1 (packet 151, Step 2): normalize loop winding to
            // wall_direction. Holes (non-ExteriorSurface) wind opposite the
            // contour.
            let is_hole = boundary_type != WallBoundaryType::ExteriorSurface;
            let base_want_ccw = if is_hole {
                !contour_should_be_ccw
            } else {
                contour_should_be_ccw
            };
            let final_want_ccw = if g7_reverse {
                !base_want_ccw
            } else {
                base_want_ccw
            };
            if path.points.len() >= 3 {
                let signed = signed_area_of_points(&path.points);
                let is_ccw = signed > 0.0;
                if is_ccw != final_want_ccw {
                    path.points.reverse();
                }
            }

            walls.push(WallLoop {
                perimeter_index: line.inset_idx,
                loop_type,
                path,
                width_profile: WidthProfile { widths },
                feature_flags,
                boundary_type,
            });
        }

        Ok(walls)
    }

    /// G3 part 2 (packet 152 Step 4): second `WallToolPaths` pass for a
    /// non-topmost region whose top surface is a SUB-AREA.
    ///
    /// Mirrors `PerimeterGenerator.cpp:2160-2246`: derive the top sub-area, run
    /// a single wall over it, run a second pass over the non-top remainder with
    /// `inner_loop_number + 1` walls, renumber each inner wall's `inset_idx`
    /// (`perimeter_index`) by +1 BEFORE the merge, then concatenate.
    ///
    /// The top-area source here is PnP's `top_solid_fill` (the host's
    /// precomputed top-region polygon) rather than Orca's
    /// `diff_ex(infill_contour, upper_slices_clipped)` — PnP has no
    /// `upper_slices`/lower-slices access on the region view. This divergence is
    /// recorded in `D-152-TOP-AREA-SOURCE`.
    fn emit_only_one_wall_top_second_pass(
        &self,
        region: &SliceRegionView,
        polygons: &[ExPolygon],
        z: f32,
        params: &ArachneParams,
        base_max_bead_count: u32,
        config: &ConfigView,
        contour_should_be_ccw: bool,
        g7_reverse: bool,
        layer_height_mm: f32,
        nozzle_diameter_mm: f32,
        bridge_flow_ratio: f32,
        thick_bridges: bool,
        output: &mut PerimeterOutputBuilder,
    ) -> Result<(), ModuleError> {
        let sequence =
            if params.wall_sequence == WallSequence::InnerOuterInner && params.is_initial_layer {
                WallSequence::InnerOuter
            } else {
                params.wall_sequence
            };
        // Step 1: top-area derivation. PnP uses `top_solid_fill` (see
        // D-152-TOP-AREA-SOURCE); Orca derives
        // `diff_ex(infill_contour, upper_slices_clipped)`.
        let mut top_area: Vec<ExPolygon> = region.top_solid_fill().to_vec();

        // Step 3: `min_width_top_surface` filter via `get_abs_value` (packet
        // 150 resolution mechanism). Absolute value, NOT raw float.
        let perimeter_width_mm = params.preferred_bead_width_outer;
        let min_width_top = config
            .get_abs_value("min_width_top_surface", perimeter_width_mm)
            .unwrap_or(0.0);
        if min_width_top > 0.0 {
            top_area.retain(|ep| (ex_polygon_min_width_mm(ep) as f64) >= min_width_top);
        }

        // Step 4: `offset2_ex` shrink by `-top_surface_min_width` then expand by
        // `+top_surface_min_width + 0.85*perimeter_width` (thin-lettering
        // preservation; the `0.85` magic constant is kept verbatim from Orca).
        let expanded = offset2_ex(
            &top_area,
            -min_width_top,
            min_width_top + 0.85 * perimeter_width_mm,
            OffsetJoinType::Miter,
            3.0,
        );
        let top_expolygons = if expanded.is_empty() {
            top_area
        } else {
            expanded
        };

        // Step 2 (bridge exclusion): Orca carves out `lower_slices_clipped`
        // from the top area; PnP has no `lower_slices` access on the region
        // view, so this step is SKIPPED — recorded in D-152-TOP-AREA-SOURCE.

        // First contribution: the top sub-area's single wall (inset_idx == 0).
        let mut top_params = *params;
        top_params.max_bead_count = 2;
        let (top_lines, _) =
            match slicer_sdk::host::generate_arachne_walls(region.top_solid_fill(), &top_params) {
                Ok(r) => r,
                Err(e) => {
                    slicer_sdk::host::log_warn(&format!(
                        "arachne-perimeters: G3p2 top-area wall generation failed for region \
                         object_id={} region_id={}: {e}",
                        region.object_id(),
                        region.region_id()
                    ));
                    return Ok(());
                }
            };
        let top_walls = self.build_walls(
            &top_lines,
            region,
            z,
            region.top_solid_fill(),
            params,
            contour_should_be_ccw,
            g7_reverse,
            layer_height_mm,
            nozzle_diameter_mm,
            bridge_flow_ratio,
            thick_bridges,
        )?;

        // Not-top remainder.
        let not_top = difference_ex(polygons, &top_expolygons);
        if not_top.is_empty() {
            // Step 8: empty-top fallback — rerun over the full region with
            // `inner_loop_number + 2` walls.
            let mut fb_params = *params;
            fb_params.max_bead_count = base_max_bead_count + 2;
            let (fb_lines, _) = match slicer_sdk::host::generate_arachne_walls(polygons, &fb_params)
            {
                Ok(r) => r,
                Err(e) => {
                    slicer_sdk::host::log_warn(&format!(
                        "arachne-perimeters: G3p2 fallback generation failed for region \
                             object_id={} region_id={}: {e}",
                        region.object_id(),
                        region.region_id()
                    ));
                    return Ok(());
                }
            };
            let mut fb_walls = self.build_walls(
                &fb_lines,
                region,
                z,
                polygons,
                params,
                contour_should_be_ccw,
                g7_reverse,
                layer_height_mm,
                nozzle_diameter_mm,
                bridge_flow_ratio,
                thick_bridges,
            )?;
            commit_wall_sequence(&mut fb_walls, sequence);
            for w in fb_walls {
                output.push_wall_loop(w)?;
            }
            return Ok(());
        }

        // Step 5: second pass over the non-top remainder with
        // `inner_loop_number + 1` walls (here, the region's base bead count),
        // inset 0.
        let mut second_params = *params;
        second_params.max_bead_count = base_max_bead_count;
        let mut second_lines =
            match slicer_sdk::host::generate_arachne_walls(&not_top, &second_params) {
                Ok((lines, _)) => lines,
                Err(e) => {
                    slicer_sdk::host::log_warn(&format!(
                        "arachne-perimeters: G3p2 second-pass generation failed for region \
                         object_id={} region_id={}: {e}",
                        region.object_id(),
                        region.region_id()
                    ));
                    return Ok(());
                }
            };

        // Step 6: renumber inner walls (+1) BEFORE merge. Mirrors Orca's
        // `++el.inset_idx` on each inner perimeter (`PerimeterGenerator.cpp:
        // 2160-2246`): every inner `ExtrusionLine` (inset_idx > 0) from the
        // second pass gets its `inset_idx` incremented by 1 so the merged
        // non-top remainder walls sit one inset deeper than the single top
        // wall.
        for line in &mut second_lines {
            if line.inset_idx > 0 {
                line.inset_idx += 1;
            }
        }
        let mut second_walls = self.build_walls(
            &second_lines,
            region,
            z,
            &not_top,
            params,
            contour_should_be_ccw,
            g7_reverse,
            layer_height_mm,
            nozzle_diameter_mm,
            bridge_flow_ratio,
            thick_bridges,
        )?;

        // Step 7: merge — top single wall + renumbered second-pass inner walls.
        for w in top_walls {
            output.push_wall_loop(w)?;
        }
        commit_wall_sequence(&mut second_walls, sequence);
        for w in second_walls {
            output.push_wall_loop(w)?;
        }
        Ok(())
    }
}

/// Minimum bounding-box extent (mm) of an `ExPolygon`'s outer contour. Used by
/// the `min_width_top_surface` filter (G3 part 2) to drop top sub-areas too
/// narrow to carry a bead.
fn ex_polygon_min_width_mm(ep: &ExPolygon) -> f32 {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in &ep.contour.points {
        let x = units_to_mm(p.x);
        let y = units_to_mm(p.y);
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    (max_x - min_x).min(max_y - min_y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ConfigKey, ConfigValue};
    use std::collections::HashMap;

    /// G9 wiring (AC-6b, packet 151 Step 5): `ArachneParams.smallest_line_segment_squared`
    /// must equal `wall_maximum_resolution²` (mm²) and `allowed_error_distance_squared`
    /// must equal `wall_maximum_deviation²` (mm²) — NOT the `meshfix_*`-sourced defaults.
    /// Values are plain mm and squared directly (no coordinate ÷100).
    #[test]
    fn wall_maximum_resolution_wired() {
        let mut fields: HashMap<ConfigKey, ConfigValue> = HashMap::new();
        fields.insert(
            "wall_maximum_resolution".to_string(),
            ConfigValue::Float(0.5),
        );
        fields.insert(
            "wall_maximum_deviation".to_string(),
            ConfigValue::Float(0.025),
        );
        let config = ConfigView::from_map(fields);
        let params = arachne_params_from_config(&config);
        assert!(
            (params.smallest_line_segment_squared - 0.25).abs() < 1e-9,
            "expected smallest_line_segment_squared = 0.5² = 0.25 mm², got {}",
            params.smallest_line_segment_squared
        );
        assert!(
            (params.allowed_error_distance_squared - 0.000625).abs() < 1e-9,
            "expected allowed_error_distance_squared = 0.025² = 0.000625 mm², got {}",
            params.allowed_error_distance_squared
        );
    }
}
