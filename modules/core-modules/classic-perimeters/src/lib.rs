// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PerimeterGenerator.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Classic perimeter generator module.
//!
//! Implements `LayerModule::run_perimeters` for the `Layer::Perimeters` stage.
//! Generates wall loops from slice contour polygons via iterative Clipper2
//! polygon insets (negative offsets).
//!
//! Per OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp process_classic().

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::HashMap;

use slicer_core::flow::bridging_flow;
use slicer_core::perimeter_utils::{
    apply_seam_paint_bias, build_wall_flags, expolygon_to_path3d,
    generate_sharp_corner_seam_candidates, point_in_any_polygon, wall_sequence_reorder,
    WallSequence, BASE_SPEED,
};
use slicer_core::polygon_ops::{
    difference_ex, offset, offset2_ex, opening_ex, remove_small_and_small_holes, OffsetJoinType,
};
use slicer_core::top_surface_split::split_top_surfaces;
use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{
    units_to_mm, variable_width, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D,
    ExtrusionRole, LoopType, PaintSemantic, PaintValue, WallLoop, WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Classic perimeter generator.
///
/// Produces wall loops via iterative constant-width polygon insets.
/// Outer wall first, then inner walls, with remaining area as infill.
///
/// NOTE (P105 R2): Per-object/per-layer overridable config keys
/// (outer_wall_line_width, inner_wall_line_width, wall_sequence,
/// detect_thin_wall, gap_infill_speed, filter_out_gap_fill, precise_outer_wall)
/// are read per-invocation from `_config` in `run_perimeters`, NOT cached here.
/// Only machine constants that cannot change mid-print are cached.
pub struct ClassicPerimeters {
    /// Number of wall loops to generate.
    wall_count: u32,
    /// Speed factor for outer walls (outer_wall_speed / BASE_SPEED).
    outer_speed_factor: f32,
    /// Speed factor for inner walls (inner_wall_speed / BASE_SPEED).
    inner_speed_factor: f32,
    /// Arc tolerance for polygon offset operations (mm).
    perimeter_arc_tolerance: f32,
}

/// Minimum enclosed area (workspace-unit², 1 unit² = 10⁻⁸ mm²) for a contour to
/// be treated as a real, offsettable island. A contour below this is degenerate:
/// a `<3`-vertex ring or a collinear zero-area sliver (`signed_area` returns 0.0
/// for `<3` vertices). Clipper's polygon offset does NOT make such a contour
/// vanish — it emits a spurious ~0.4 mm wall from empty input — so these are
/// dropped up front. The threshold is astronomically below any printable feature
/// (a 0.4 mm wall spans millions of unit²), so no thin-but-valid island is
/// touched; it rejects only genuinely degenerate contours.
const DEGENERATE_MIN_AREA_SQ_UNITS: f64 = 1.0;

#[slicer_module]
impl LayerModule for ClassicPerimeters {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let wall_count = match config.get("wall_count") {
            Some(ConfigValue::Int(n)) => *n as u32,
            _ => 3, // default
        };

        let outer_wall_speed = match config.get("outer_wall_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => 30.0, // default
        };

        let inner_wall_speed = match config.get("inner_wall_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => 45.0, // default
        };

        let perimeter_arc_tolerance = match config.get("perimeter_arc_tolerance") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.0125,
        };

        Ok(Self {
            wall_count,
            outer_speed_factor: outer_wall_speed / BASE_SPEED,
            inner_speed_factor: inner_wall_speed / BASE_SPEED,
            perimeter_arc_tolerance,
        })
    }

    /// `_paint` is intentionally unread in this module — consumed by Phase 2
    /// follow-up packet 102.
    fn run_perimeters(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // ── R2: Per-invocation config reads (P105) ───────────────────────
        // These 7 keys support per-object/per-layer overrides and MUST be read
        // from _config here, not cached at on_print_start.
        let legacy_line_width = match _config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4,
        };
        let outer_wall_line_width = match _config.get("outer_wall_line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => legacy_line_width,
        };
        let inner_wall_line_width = match _config.get("inner_wall_line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => legacy_line_width,
        };
        let wall_sequence = match _config.get("wall_sequence") {
            Some(ConfigValue::String(s)) => match s.as_str() {
                "InnerOuter" => WallSequence::InnerOuter,
                "OuterInner" => WallSequence::OuterInner,
                "InnerOuterInner" => WallSequence::InnerOuterInner,
                _ => WallSequence::InnerOuter,
            },
            _ => WallSequence::InnerOuter,
        };
        let detect_thin_wall = _config.get_bool("detect_thin_wall").unwrap_or(true);
        // Packet 108: absolute turn-angle threshold (degrees) gating seam-candidate
        // emission to sharp corners only, instead of every outer-wall vertex.
        let seam_candidate_angle_threshold_deg = _config
            .get_float("seam_candidate_angle_threshold_deg")
            .map(|v| v as f32)
            .unwrap_or(30.0);
        let gap_infill_speed = _config
            .get_float("gap_infill_speed")
            .map(|s| s as f32)
            .unwrap_or(30.0);
        let filter_out_gap_fill = _config
            .get_float("filter_out_gap_fill")
            .map(|s| s as f32)
            .unwrap_or(0.5);
        // Medial-axis backend gate (diagnose 2026-06-24). On painted slices the
        // gap-fill / thin-wall medial axis can OOM-abort boostvoronoi on degenerate
        // per-color cell gaps (cube_fuzzyPainted). Until the medial axis is isolated
        // in a worker subprocess, skip it for painted slices (`slice_has_paint`
        // injected by the host) unless the user explicitly opts back in via
        // `gap_fill_medial_axis_on_painted`. Unpainted models keep full parity.
        let gap_fill_medial_axis_on_painted = _config
            .get_bool("gap_fill_medial_axis_on_painted")
            .unwrap_or(false);
        let slice_has_paint = _config.get_bool("slice_has_paint").unwrap_or(false);
        let medial_axis_enabled = gap_fill_medial_axis_on_painted || !slice_has_paint;
        if !medial_axis_enabled && layer_index == 0 {
            slicer_sdk::host::log_warn(
                "medial-axis-skipped reason=backend-unstable scope=painted-slice \
                 (set gap_fill_medial_axis_on_painted=true to re-enable)",
            );
        }
        // R1: precise_outer_wall — gated on wall_sequence==InnerOuter (AC-7, P105).
        // OrcaSlicer PerimeterGenerator.cpp:1501-1506,1644
        let precise_outer_wall_raw = _config.get_bool("precise_outer_wall").unwrap_or(false);
        let precise_outer_wall =
            precise_outer_wall_raw && matches!(wall_sequence, WallSequence::InnerOuter);

        // ── Nozzle diameter for R4 threshold (falls back to inner_wall_line_width
        //    if not provided — preserves behaviour on older profiles). ───────────
        let nozzle_diameter = _config
            .get_float("nozzle_diameter")
            .map(|v| v as f32)
            .unwrap_or(inner_wall_line_width);

        let base_wall_count = _config
            .get_int("wall_count")
            .map(|n| n as u32)
            .unwrap_or(self.wall_count);
        // extra_perimeters (T-070/T-071, P108): per-region bonus wall count.
        // OrcaSlicer PerimeterGenerator.cpp:1569 —
        // `int loop_number = this->config->wall_loops + surface.extra_perimeters - 1;`
        // (0-indexed loops). Translated to an actual wall count, this is simply
        // `wall_count + extra_perimeters`.
        let extra_perimeters = _config.get_int("extra_perimeters").unwrap_or(0).max(0) as u32;
        let base_wall_count = base_wall_count + extra_perimeters;
        // extra_perimeters_on_overhangs (T-077, P108): add ONE extra wall loop
        // inside the region's overhang footprint (region.overhang_areas()),
        // leaving the rest of the region at the base wall count. Additive
        // with the plain extra_perimeters bonus above (independent branch in
        // the planar path); never applies to the non-planar shell branch,
        // which returns before this code runs.
        let extra_perimeters_on_overhangs = _config
            .get_bool("extra_perimeters_on_overhangs")
            .unwrap_or(false);
        // Narrow-island smaller-width override (T-072/T-073, P108). See
        // `classify_narrow_island` for the classification rule; these three
        // keys are per-invocation (not per-object/per-layer overridable yet).
        let smaller_perimeter_line_width = _config
            .get_float("smaller_perimeter_line_width")
            .map(|v| v as f32)
            .unwrap_or(0.25);
        let smaller_perimeter_threshold_mm = _config
            .get_float("smaller_perimeter_threshold_mm")
            .map(|v| v as f32)
            .unwrap_or(0.8);
        let narrow_loop_length_threshold_mm = _config
            .get_float("narrow_loop_length_threshold_mm")
            .map(|v| v as f32)
            .unwrap_or(10.0);
        let only_one_wall_top = _config.get_bool("only_one_wall_top").unwrap_or(false);
        // min_width_top_surface (D-104d, deferred): OrcaSlicer's width
        // threshold (mm) gating the `only_one_wall_top` single-wall cutoff —
        // a top-surface loop narrower than this keeps its full wall count
        // instead of collapsing to one (PerimeterGenerator.cpp:2160-2245;
        // PrintConfig.cpp:1491-1511). `only_one_wall_top` above is
        // unconditional (no per-loop width comparison yet) — behavior
        // deferred, see D-104d-MIN-WIDTH-TOP-SURFACE-NONE. Read and
        // validated here (must be a finite, non-negative mm value) so the
        // config key round-trips correctly ahead of that follow-up; the
        // value itself is intentionally unused until then.
        let min_width_top_surface = _config.get_float("min_width_top_surface").unwrap_or(1.2);
        debug_assert!(
            min_width_top_surface.is_finite() && min_width_top_surface >= 0.0,
            "min_width_top_surface must be a finite, non-negative mm value, got {min_width_top_surface}"
        );
        let _ = min_width_top_surface;
        let only_one_wall_first_layer = _config
            .get_bool("only_one_wall_first_layer")
            .unwrap_or(false);
        let layer_wall_count = if only_one_wall_first_layer && layer_index == 0 {
            1
        } else {
            base_wall_count
        };
        let outer_wall_speed = _config
            .get_float("outer_wall_speed")
            .map(|s| s as f32)
            .or_else(|| _config.get_int("outer_wall_speed").map(|s| s as f32))
            .unwrap_or(self.outer_speed_factor * BASE_SPEED);
        let inner_wall_speed = _config
            .get_float("inner_wall_speed")
            .map(|s| s as f32)
            .or_else(|| _config.get_int("inner_wall_speed").map(|s| s as f32))
            .unwrap_or(self.inner_speed_factor * BASE_SPEED);
        let outer_speed_factor = outer_wall_speed / BASE_SPEED;
        let inner_speed_factor = inner_wall_speed / BASE_SPEED;
        // bridge_flow / thick_bridges (packet 149, D4/D-104g): read once per
        // invocation, applied per-vertex in emit_walls wherever is_bridge is true.
        let bridge_flow_ratio = _config
            .get_float("bridge_flow")
            .map(|v| v as f32)
            .unwrap_or(1.0);
        let thick_bridges = _config.get_bool("thick_bridges").unwrap_or(false);

        for region in regions {
            output.begin_region(region.object_id(), *region.region_id());
            if region.polygons().is_empty() {
                continue;
            }
            // ── Non-planar shell branch (T-074b/c/d, P108) ──────────────
            // Highest precedence: a region backed by a resolved SurfaceGroup
            // (region.nonplanar_surface.is_some() at the IR level) emits
            // exactly `shell_count` NonPlanarShell walls and nothing else —
            // no thin-wall, no gap-fill, no narrow-island override, no
            // extra_perimeters bonus, no infill. This module does not compute
            // per-vertex Z; that is a downstream concern.
            if let Some(surface_group) = region.surface_group() {
                self.emit_nonplanar_shells(
                    region.polygons(),
                    region.z(),
                    surface_group.shell_count,
                    outer_wall_line_width,
                    inner_wall_line_width,
                    output,
                )?;
                continue;
            }
            let top_shell = region.top_shell_index();
            let wall_count = if only_one_wall_top && top_shell == Some(0) {
                1
            } else {
                layer_wall_count
            };
            let polygons = region.polygons();
            let z = region.z();
            if wall_count == 0 {
                output.set_infill_areas(polygons.to_vec())?;
                continue;
            }
            let rid = *region.region_id();
            // Narrow-island override (T-072/T-073, P108): classify against the
            // full region polygon set, additive with extra_perimeters — the
            // narrow-island check only swaps the outer-wall width/spacing,
            // it does not change wall_count.
            let region_outer_wall_line_width = if classify_narrow_island(
                polygons,
                smaller_perimeter_threshold_mm,
                narrow_loop_length_threshold_mm,
                self.perimeter_arc_tolerance,
            ) {
                smaller_perimeter_line_width
            } else {
                outer_wall_line_width
            };
            // D14: painted FuzzySkin travels on variant_chain, not
            // segment_annotations; resolve once and apply per-vertex below.
            let region_fuzzy = region
                .variant_chain()
                .iter()
                .any(|(sem, val)| sem == "fuzzy_skin" && matches!(val, PaintValue::Flag(true)));
            let overhang_areas = region.overhang_areas();
            let overhang_bands = region.overhang_quartile_polygons();
            if extra_perimeters_on_overhangs && !overhang_areas.is_empty() {
                // T-077 (P108): one extra wall loop inside the overhang
                // footprint, base wall count elsewhere. Reuses the
                // intersection/difference + sliver-filter split utility
                // (generic mask split, despite its top-surface-flavoured
                // name/docs) with `region.overhang_areas()` as the mask.
                let split = split_top_surfaces(polygons, overhang_areas);
                if !split.top_portion.is_empty() {
                    self.emit_walls(
                        &split.top_portion,
                        z,
                        region.segment_annotations(),
                        region_fuzzy,
                        true,
                        true,
                        output,
                        wall_count + 1,
                        outer_speed_factor,
                        inner_speed_factor,
                        region.bridge_areas(),
                        bridge_flow_ratio,
                        thick_bridges,
                        region_outer_wall_line_width,
                        inner_wall_line_width,
                        wall_sequence,
                        precise_outer_wall,
                        detect_thin_wall,
                        nozzle_diameter,
                        gap_infill_speed,
                        filter_out_gap_fill,
                        rid,
                        medial_axis_enabled,
                        seam_candidate_angle_threshold_deg,
                        overhang_bands,
                    )?;
                }
                if !split.non_top_portion.is_empty() {
                    self.emit_walls(
                        &split.non_top_portion,
                        z,
                        region.segment_annotations(),
                        region_fuzzy,
                        true,
                        true,
                        output,
                        wall_count,
                        outer_speed_factor,
                        inner_speed_factor,
                        region.bridge_areas(),
                        bridge_flow_ratio,
                        thick_bridges,
                        region_outer_wall_line_width,
                        inner_wall_line_width,
                        wall_sequence,
                        precise_outer_wall,
                        detect_thin_wall,
                        nozzle_diameter,
                        gap_infill_speed,
                        filter_out_gap_fill,
                        rid,
                        medial_axis_enabled,
                        seam_candidate_angle_threshold_deg,
                        overhang_bands,
                    )?;
                }
            } else if only_one_wall_top && matches!(top_shell, Some(n) if n > 0) {
                let split = split_top_surfaces(polygons, region.top_solid_fill());
                if !split.top_portion.is_empty() {
                    self.emit_walls(
                        &split.top_portion,
                        z,
                        region.segment_annotations(),
                        region_fuzzy,
                        true,
                        true,
                        output,
                        1,
                        outer_speed_factor,
                        inner_speed_factor,
                        region.bridge_areas(),
                        bridge_flow_ratio,
                        thick_bridges,
                        region_outer_wall_line_width,
                        inner_wall_line_width,
                        wall_sequence,
                        precise_outer_wall,
                        detect_thin_wall,
                        nozzle_diameter,
                        gap_infill_speed,
                        filter_out_gap_fill,
                        rid,
                        medial_axis_enabled,
                        seam_candidate_angle_threshold_deg,
                        overhang_bands,
                    )?;
                }
                if !split.non_top_portion.is_empty() {
                    self.emit_walls(
                        &split.non_top_portion,
                        z,
                        region.segment_annotations(),
                        region_fuzzy,
                        true,
                        true,
                        output,
                        layer_wall_count,
                        outer_speed_factor,
                        inner_speed_factor,
                        region.bridge_areas(),
                        bridge_flow_ratio,
                        thick_bridges,
                        region_outer_wall_line_width,
                        inner_wall_line_width,
                        wall_sequence,
                        precise_outer_wall,
                        detect_thin_wall,
                        nozzle_diameter,
                        gap_infill_speed,
                        filter_out_gap_fill,
                        rid,
                        medial_axis_enabled,
                        seam_candidate_angle_threshold_deg,
                        overhang_bands,
                    )?;
                }
            } else {
                self.emit_walls(
                    polygons,
                    z,
                    region.segment_annotations(),
                    region_fuzzy,
                    true,
                    true,
                    output,
                    wall_count,
                    outer_speed_factor,
                    inner_speed_factor,
                    region.bridge_areas(),
                    bridge_flow_ratio,
                    thick_bridges,
                    region_outer_wall_line_width,
                    inner_wall_line_width,
                    wall_sequence,
                    precise_outer_wall,
                    detect_thin_wall,
                    nozzle_diameter,
                    gap_infill_speed,
                    filter_out_gap_fill,
                    rid,
                    medial_axis_enabled,
                    seam_candidate_angle_threshold_deg,
                    overhang_bands,
                )?;
            }
        }

        Ok(())
    }
}

impl ClassicPerimeters {
    /// Returns the configured wall count.
    pub fn wall_count(&self) -> u32 {
        self.wall_count
    }

    /// Emit wall loops (plus seam candidates and infill) for `polygons`.
    ///
    /// T-051/T-052: outer (i==0) uses `outer_wall_line_width`; inner (i>=1) uses
    /// `inner_wall_line_width`. The first inset is by `outer_wall_line_width / 2`;
    /// subsequent insets are by `inner_wall_line_width` (canonical OrcaSlicer
    /// ext_perimeter_spacing2 perimeter_spacing arithmetic).
    ///
    /// R1 (P105 AC-7): when `precise_outer_wall` is active (precise_outer_wall=true
    /// AND wall_sequence=InnerOuter), the first inset uses `ext_perimeter_spacing2`
    /// = (outer_wall_line_width + inner_wall_line_width)/2 and inner walls are
    /// emitted before the outer wall.
    /// OrcaSlicer PerimeterGenerator.cpp:1501-1506,1644
    #[allow(clippy::too_many_arguments)]
    fn line_width_for(
        &self,
        perimeter_index: u32,
        outer_wall_line_width: f32,
        inner_wall_line_width: f32,
    ) -> f32 {
        if perimeter_index == 0 {
            outer_wall_line_width
        } else {
            inner_wall_line_width
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_walls(
        &self,
        polygons: &[ExPolygon],
        z: f32,
        segment_annotations: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
        variant_fuzzy: bool,
        emit_outer: bool,
        emit_inner: bool,
        output: &mut PerimeterOutputBuilder,
        wall_count: u32,
        outer_speed_factor: f32,
        inner_speed_factor: f32,
        bridge_areas: &[ExPolygon],
        bridge_flow_ratio: f32,
        thick_bridges: bool,
        outer_wall_line_width: f32,
        inner_wall_line_width: f32,
        wall_sequence: WallSequence,
        precise_outer_wall: bool,
        detect_thin_wall: bool,
        nozzle_diameter: f32,
        gap_infill_speed: f32,
        filter_out_gap_fill: f32,
        region_id: u64,
        medial_axis_enabled: bool,
        seam_candidate_angle_threshold_deg: f32,
        overhang_bands: &[QuartileBand],
    ) -> Result<(), ModuleError> {
        // P109 degeneracy guard: a contour needs >=3 non-collinear vertices
        // (strictly positive enclosed area) to be offsettable. Clipper's polygon
        // offset of a degenerate contour (0-/2-vertex, or collinear zero-area)
        // does NOT vanish — it emits a spurious ~0.4 mm sliver, leaking phantom
        // Outer walls from empty input (mirrors the medial-axis
        // `axis.points.len() < 2` guard's spirit). Drop those contours up front
        // so BOTH the wall-offset loop and the thin-wall path below see only real
        // geometry. `remove_small_and_small_holes` retains a contour iff
        // |signed_area| >= min_area (and signed_area == 0 for <3 vertices); the
        // sub-unit² threshold removes exactly the degenerate contours while
        // leaving every thin-but-valid feature (millions of unit²) intact.
        let mut valid_polygons = polygons.to_vec();
        remove_small_and_small_holes(&mut valid_polygons, DEGENERATE_MIN_AREA_SQ_UNITS, 0.0);
        let polygons: &[ExPolygon] = &valid_polygons;

        // Generate wall loops via iterative insets.
        let mut current_polygons = polygons.to_vec();
        let mut all_wall_polygons: Vec<(u32, Vec<ExPolygon>)> = Vec::new();
        // Gap-fill (T-063/T-064): collect gaps BETWEEN consecutive perimeter
        // insets, matching OrcaSlicer PerimeterGenerator.cpp:1665-1670. Gaps are
        // only collected for INNER transitions (i >= 1): the region-boundary →
        // first-wall transition (i == 0) is NEVER a gap source, so the per-color
        // MMU bisector edge (ADR-0013 Model A — adjacent colors offset half a
        // line-width inward from the shared bisector) does NOT spawn phantom
        // gap-fill slivers along every color boundary. The previous single-shot
        // `difference_ex(current_polygons, infill_inset)` rang the entire
        // innermost contour (bisector included), flooding 300+ slivers per cube.
        let mut gaps: Vec<ExPolygon> = Vec::new();

        for i in 0..wall_count {
            let inset_delta = if i == 0 {
                // R1 (P105 AC-7): precise mode uses ext_perimeter_spacing2 for the
                // outer wall inset (same as the gap between outer and first inner).
                // OrcaSlicer PerimeterGenerator.cpp:1501-1506
                if precise_outer_wall {
                    -((outer_wall_line_width + inner_wall_line_width) / 2.0)
                } else {
                    -(outer_wall_line_width / 2.0)
                }
            } else if i == 1 {
                -((outer_wall_line_width + inner_wall_line_width) / 2.0)
            } else {
                -inner_wall_line_width
            };
            let inset_result = offset(
                &current_polygons,
                inset_delta,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            if inset_result.is_empty() {
                break;
            }
            // OrcaSlicer gap collection between perimeter (i-1) and perimeter i:
            // diff(offset(prev, -0.5d), offset(cur, +0.5d)) captures the region
            // where the actual spacing exceeds `d` (a true gap). Skipped at i==0.
            if i >= 1 {
                let distance = inset_delta.abs();
                let shrunk_prev = offset(
                    &current_polygons,
                    -(0.5 * distance),
                    OffsetJoinType::Miter,
                    self.perimeter_arc_tolerance,
                );
                let grown_cur = offset(
                    &inset_result,
                    0.5 * distance,
                    OffsetJoinType::Miter,
                    self.perimeter_arc_tolerance,
                );
                gaps.extend(difference_ex(&shrunk_prev, &grown_cur));
            }
            all_wall_polygons.push((i, inset_result.clone()));
            current_polygons = inset_result;
        }

        // Final infill-transition gap (OrcaSlicer parity). The gap between the
        // innermost wall and where infill begins is ~empty for WIDE regions (the
        // infill fills the center, so shrunk-innermost and grown-infill meet) but
        // equals the whole leftover core for THIN features where no infill fits.
        // This captures thin arms/ribs as gap-fill without re-introducing the
        // per-color MMU bisector ring slivers — wide cells produce ~zero here.
        if !current_polygons.is_empty() {
            let distance = inner_wall_line_width;
            let infill_area = offset(
                &current_polygons,
                -distance,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            let shrunk_inner = offset(
                &current_polygons,
                -(0.5 * distance),
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            let grown_infill = offset(
                &infill_area,
                0.5 * distance,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            gaps.extend(difference_ex(&shrunk_inner, &grown_infill));
        }

        let mut walls: Vec<slicer_ir::WallLoop> = Vec::new();

        for (perimeter_index, wall_polys) in &all_wall_polygons {
            let is_outer = *perimeter_index == 0;
            // AC-22b: emit only the requested bands (outer-once / inner-per-cell).
            if (is_outer && !emit_outer) || (!is_outer && !emit_inner) {
                continue;
            }
            let loop_type = if is_outer {
                LoopType::Outer
            } else {
                LoopType::Inner
            };
            let role = if is_outer {
                ExtrusionRole::OuterWall
            } else {
                ExtrusionRole::InnerWall
            };
            let speed_factor = if is_outer {
                outer_speed_factor
            } else {
                inner_speed_factor
            };

            for (poly_idx, poly) in wall_polys.iter().enumerate() {
                let mut points = expolygon_to_path3d(
                    &poly.contour,
                    z,
                    self.line_width_for(
                        *perimeter_index,
                        outer_wall_line_width,
                        inner_wall_line_width,
                    ),
                    overhang_bands,
                );
                if points.is_empty() {
                    continue;
                }
                let num_points = points.len();

                let ring_pts: Option<&[slicer_ir::Point2]> = if is_outer {
                    None
                } else {
                    Some(&poly.contour.points)
                };
                let orig_polys: Option<&[ExPolygon]> = if is_outer { None } else { Some(polygons) };
                let (mut feature_flags, boundary_type) = build_wall_flags(
                    num_points,
                    poly_idx,
                    segment_annotations,
                    is_outer,
                    ring_pts,
                    orig_polys,
                    variant_fuzzy,
                );
                // Per-vertex is_bridge: set for each vertex strictly inside any bridge area.
                // poly.contour.points has N entries (integer units); feature_flags has N+1
                // (closing repeat appended by expolygon_to_path3d). The closing repeat is
                // handled by mirror_first_to_last below.
                for (i, pt) in poly.contour.points.iter().enumerate() {
                    if i < feature_flags.len() {
                        let is_bridge = point_in_any_polygon(pt, bridge_areas);
                        feature_flags[i].is_bridge = is_bridge;
                        if is_bridge {
                            points[i].flow_factor = bridging_flow(bridge_flow_ratio, thick_bridges);
                        }
                    }
                }
                slicer_sdk::mirror_first_to_last(&mut feature_flags);

                let wall = WallLoop {
                    perimeter_index: *perimeter_index,
                    loop_type,
                    path: ExtrusionPath3D {
                        points,
                        role: role.clone(),
                        speed_factor,
                    },
                    width_profile: WidthProfile {
                        widths: vec![
                            self.line_width_for(
                                *perimeter_index,
                                outer_wall_line_width,
                                inner_wall_line_width
                            );
                            num_points
                        ],
                    },
                    feature_flags,
                    boundary_type,
                };
                walls.push(wall);
            }
        }

        // R1 (P105 AC-7): precise mode reorders inner walls before outer.
        // When precise_outer_wall is active (gated on InnerOuter), emit inner
        // walls first, then outer wall — overrides the standard InnerOuter
        // canonical order (which is outer-first).
        // OrcaSlicer PerimeterGenerator.cpp:1644
        if precise_outer_wall {
            // Split into outer and inner, emit inner first then outer.
            let mut outer_walls: Vec<slicer_ir::WallLoop> = walls
                .iter()
                .filter(|w| w.loop_type == LoopType::Outer)
                .cloned()
                .collect();
            let mut inner_walls: Vec<slicer_ir::WallLoop> = walls
                .iter()
                .filter(|w| w.loop_type != LoopType::Outer)
                .cloned()
                .collect();
            // Inner first, then outer (precise mode inner-first ordering).
            inner_walls.append(&mut outer_walls);
            for wall in inner_walls {
                output.push_wall_loop(wall)?;
            }
        } else {
            wall_sequence_reorder(&mut walls, wall_sequence, &[]);
            for wall in walls {
                output.push_wall_loop(wall)?;
            }
        }

        // ── Thin-wall detection (T-061/T-062) ──────────────────────────
        if detect_thin_wall && emit_outer && medial_axis_enabled {
            // R4 (P105): OrcaSlicer parity thin-wall min_width.
            // OrcaSlicer PerimeterGenerator.cpp:1603: min_width = nozzle_diameter()/3
            let min_width = nozzle_diameter / 3.0;
            let thick_core = opening_ex(
                polygons,
                min_width as f64,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance as f64,
            );
            let thin_protrusions = difference_ex(polygons, &thick_core);
            for protrusion in &thin_protrusions {
                let axes = slicer_sdk::host::medial_axis(
                    protrusion,
                    min_width,
                    inner_wall_line_width * 2.0,
                );
                if let Err(e) = &axes {
                    slicer_sdk::host::log_warn(&format!(
                        "medial-axis-failed region={region_id} fixture=thin_wall error={e}"
                    ));
                }
                if let Ok(axes) = axes {
                    for axis in &axes {
                        if axis.points.len() < 2 {
                            continue;
                        }
                        let num_pts = axis.points.len();
                        let mut path = variable_width(axis, ExtrusionRole::ThinWall);
                        for pt in &mut path.points {
                            pt.z = z;
                        }
                        let mut flags =
                            vec![slicer_core::perimeter_utils::default_feature_flags(); num_pts];
                        for flag in &mut flags {
                            flag.is_thin_wall = true;
                        }
                        // ThinWall paths are closed loops (ExtrusionRole::is_closed_loop
                        // returns true for ThinWall).  Both the path points and the
                        // parallel feature_flags must carry the N+1 closing repeat so
                        // that feature_flags.len() == path.points.len() (docs/03 invariant).
                        slicer_sdk::close_loop(&mut path.points);
                        slicer_sdk::close_loop(&mut flags);
                        // Build widths from the (now closed) path.points to keep
                        // width_profile.widths parallel with path.points.
                        let widths = path.points.iter().map(|p| p.width).collect();
                        output.push_wall_loop(WallLoop {
                            perimeter_index: 0,
                            loop_type: LoopType::ThinWall,
                            path,
                            width_profile: WidthProfile { widths },
                            feature_flags: flags,
                            boundary_type: slicer_ir::WallBoundaryType::Interior,
                        })?;
                    }
                }
            }
        }

        // ── Gap-fill emission (T-063/T-064) ────────────────────────────
        // Gaps were collected incrementally between consecutive insets above
        // (OrcaSlicer PerimeterGenerator.cpp:1665-1670). Apply the morphological
        // width-band pre-filter (PerimeterGenerator.cpp:1924-1928) before feeding
        // the medial axis: keep only gaps whose width is in [min, max]. This both
        // matches Orca parity AND removes the sub-/super-threshold slivers that
        // were driving the RNG medial-axis (and thus non-deterministic gcode).
        if emit_inner && !gaps.is_empty() && medial_axis_enabled {
            // R4 (P105): OrcaSlicer parity gap-fill min_width.
            // OrcaSlicer PerimeterGenerator.cpp:1924:
            // min_gap_fill_width = 0.2 * line_width * (1.0 - INSET_OVERLAP_TOLERANCE)
            // INSET_OVERLAP_TOLERANCE = 0.2 (OrcaSlicer default; no matching const in repo).
            let min_gap_fill_width = 0.2 * inner_wall_line_width * (1.0 - 0.2_f32);
            // OrcaSlicer max = 2 * perimeter_spacing (PerimeterGenerator.cpp:1947).
            let perimeter_spacing = (outer_wall_line_width + inner_wall_line_width) / 2.0;
            let max_gap_fill_width = 2.0 * perimeter_spacing;
            // diff(open(gaps, min/2), open(gaps, max/2)) = gaps in width band [min, max].
            let opened_min = opening_ex(
                &gaps,
                (min_gap_fill_width / 2.0) as f64,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance as f64,
            );
            let opened_max = offset2_ex(
                &gaps,
                -((max_gap_fill_width / 2.0) as f64),
                (max_gap_fill_width / 2.0) as f64,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance as f64,
            );
            let filtered_gaps = difference_ex(&opened_min, &opened_max);
            for gap in &filtered_gaps {
                let axes =
                    slicer_sdk::host::medial_axis(gap, min_gap_fill_width, max_gap_fill_width);
                if let Err(e) = &axes {
                    slicer_sdk::host::log_warn(&format!(
                        "medial-axis-failed region={region_id} fixture=gap_fill error={e}"
                    ));
                }
                if let Ok(axes) = axes {
                    for axis in &axes {
                        if axis.points.len() < 2 {
                            continue;
                        }
                        // AC-4 segment-length filter: drop gap-fill polylines whose
                        // total length is below filter_out_gap_fill (e.g. 0.5 mm).
                        // This is a LENGTH filter, not a width threshold.
                        let total_len: f32 = axis
                            .points
                            .windows(2)
                            .map(|w| {
                                let dx = w[1].x - w[0].x;
                                let dy = w[1].y - w[0].y;
                                (dx * dx + dy * dy).sqrt()
                            })
                            .sum();
                        if total_len < filter_out_gap_fill {
                            continue;
                        }
                        let num_pts = axis.points.len();
                        let mut path = variable_width(axis, ExtrusionRole::GapFill);
                        for pt in &mut path.points {
                            pt.z = z;
                        }
                        path.speed_factor = gap_infill_speed / BASE_SPEED;
                        let flags =
                            vec![slicer_core::perimeter_utils::default_feature_flags(); num_pts];
                        output.push_wall_loop(WallLoop {
                            perimeter_index: 0,
                            loop_type: LoopType::GapFill,
                            path,
                            width_profile: WidthProfile {
                                widths: axis.points.iter().map(|p| p.width).collect(),
                            },
                            feature_flags: flags,
                            boundary_type: slicer_ir::WallBoundaryType::Interior,
                        })?;
                    }
                }
            }
        }

        // Seam candidates belong to the outer wall (the shared-perimeter pass).
        //
        // Packet 108 (T-P98-SEAM, D-108-SEAM-CONSUMED): consume painted
        // `seam_enforcer`/`seam_blocker` semantics at candidate-generation
        // time. Outer-wall vertex ordering/count is preserved from the
        // original region contour (see `build_wall_flags` doc comment), so
        // per-vertex `segment_annotations` lookups by `poly_idx`/vertex-index
        // are valid against `poly.contour.points` here.
        if emit_outer {
            if let Some((_, outer_polys)) = all_wall_polygons.first() {
                for (poly_idx, poly) in outer_polys.iter().enumerate() {
                    let mut candidates = generate_sharp_corner_seam_candidates(
                        &poly.contour,
                        z,
                        seam_candidate_angle_threshold_deg,
                    );
                    let enforcer_polys =
                        seam_paint_boxes(poly_idx, poly, segment_annotations, "seam_enforcer");
                    let blocker_polys =
                        seam_paint_boxes(poly_idx, poly, segment_annotations, "seam_blocker");
                    apply_seam_paint_bias(&mut candidates, &enforcer_polys, &blocker_polys);
                    for candidate in candidates {
                        output.push_seam_candidate(candidate.position, candidate.score)?;
                    }
                }
            }
        }

        // Only the inner/infill pass owns the infill region. Inset the innermost
        // wall by a FULL `inner_wall_line_width` (not half) so the infill region
        // is consistent with the gap-fill infill-transition collection above:
        // wide regions keep a non-empty infill center, thin features inset to
        // empty and are owned entirely by gap-fill. Using half-width here left a
        // thin residual strip that was double-counted as BOTH infill and gap.
        if emit_inner && !current_polygons.is_empty() {
            let infill = offset(
                &current_polygons,
                -inner_wall_line_width,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            if !infill.is_empty() {
                output.set_infill_areas(infill)?;
            }
        }

        Ok(())
    }

    /// Emit `shell_count` concentric `LoopType::NonPlanarShell` wall loops for
    /// a region backed by a resolved `SurfaceGroup` (T-074b/c/d, P108).
    ///
    /// This is our own extension — absent in OrcaSlicer's classic perimeter
    /// generator — for regions whose `nonplanar_surface` resolved to a
    /// `SurfaceGroup`. Unlike [`Self::emit_walls`], this path emits no
    /// thin-wall, no gap-fill, and no infill: `shell_count` overrides the
    /// normal wall-count logic entirely, and the leftover core (if any) is
    /// left unfilled by design (a future non-planar infill module owns it).
    /// Z is passed through unchanged per vertex — this module does not
    /// compute per-vertex Z for non-planar surfaces.
    fn emit_nonplanar_shells(
        &self,
        polygons: &[ExPolygon],
        z: f32,
        shell_count: u32,
        outer_wall_line_width: f32,
        inner_wall_line_width: f32,
        output: &mut PerimeterOutputBuilder,
    ) -> Result<(), ModuleError> {
        let mut current_polygons = polygons.to_vec();
        for i in 0..shell_count {
            let inset_delta = if i == 0 {
                -(outer_wall_line_width / 2.0)
            } else {
                -inner_wall_line_width
            };
            let inset_result = offset(
                &current_polygons,
                inset_delta,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            if inset_result.is_empty() {
                break;
            }
            let width = self.line_width_for(i, outer_wall_line_width, inner_wall_line_width);
            let role = if i == 0 {
                ExtrusionRole::OuterWall
            } else {
                ExtrusionRole::InnerWall
            };
            for poly in &inset_result {
                // Non-planar shells are an explicitly separate concern (D-3); never
                // stamp overhang_quartile here.
                let points = expolygon_to_path3d(&poly.contour, z, width, &[]);
                if points.is_empty() {
                    continue;
                }
                let num_points = points.len();
                let feature_flags =
                    vec![slicer_core::perimeter_utils::default_feature_flags(); num_points];
                output.push_wall_loop(WallLoop {
                    perimeter_index: i,
                    loop_type: LoopType::NonPlanarShell,
                    path: ExtrusionPath3D {
                        points,
                        role: role.clone(),
                        speed_factor: 1.0,
                    },
                    width_profile: WidthProfile {
                        widths: vec![width; num_points],
                    },
                    feature_flags,
                    boundary_type: slicer_ir::WallBoundaryType::Interior,
                })?;
            }
            current_polygons = inset_result;
        }
        Ok(())
    }
}

/// Extract small "coverage" boxes around outer-wall vertices painted with
/// `semantic_name` (packet 108: `seam_enforcer` / `seam_blocker` consumption,
/// D-108-SEAM-CONSUMED).
///
/// `segment_annotations` values live at `PaintSemantic::Custom(semantic_name)`,
/// indexed `[poly_idx][vertex_idx]` against the ORIGINAL region contour;
/// outer-wall vertex ordering/count is preserved from that contour (see
/// `slicer_core::perimeter_utils::build_wall_flags` doc comment), so index
/// alignment against `poly.contour.points` is valid. Each painted vertex
/// becomes a small square `ExPolygon` (1 mm half-size) centered on it, since
/// `apply_seam_paint_bias` tests candidate positions via point-in-polygon,
/// not point-to-point proximity.
fn seam_paint_boxes(
    poly_idx: usize,
    poly: &ExPolygon,
    segment_annotations: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    semantic_name: &str,
) -> Vec<ExPolygon> {
    const HALF_SIZE_MM: f32 = 1.0;
    let half = slicer_ir::mm_to_units(HALF_SIZE_MM);
    let semantic = PaintSemantic::Custom(semantic_name.to_string());
    let Some(vals) = segment_annotations
        .get(&semantic)
        .and_then(|per_poly| per_poly.get(poly_idx))
    else {
        return Vec::new();
    };
    poly.contour
        .points
        .iter()
        .enumerate()
        .filter(|(i, _)| matches!(vals.get(*i), Some(Some(PaintValue::Flag(true)))))
        .map(|(_, pt)| seam_paint_box(*pt, half))
        .collect()
}

/// Build a small axis-aligned square `ExPolygon` centered on `center` with
/// `half`-unit half-size (see [`seam_paint_boxes`]).
fn seam_paint_box(center: slicer_ir::Point2, half: i64) -> ExPolygon {
    ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                slicer_ir::Point2 {
                    x: center.x - half,
                    y: center.y - half,
                },
                slicer_ir::Point2 {
                    x: center.x + half,
                    y: center.y - half,
                },
                slicer_ir::Point2 {
                    x: center.x + half,
                    y: center.y + half,
                },
                slicer_ir::Point2 {
                    x: center.x - half,
                    y: center.y + half,
                },
            ],
        },
        holes: Vec::new(),
    }
}

/// Narrow-island classification (T-072/T-073, P108).
///
/// An island is classified "narrow" when it is narrower than
/// `threshold_mm` everywhere (an inward-then-outward offset — a
/// morphological opening — by `threshold_mm / 2` erodes it to nothing) AND
/// its longest bounding-box dimension is at least `min_length_mm` — this
/// second condition filters out tiny slivers/noise from being misclassified
/// as genuine narrow islands, it is not an upper bound.
///
/// Loosely follows OrcaSlicer PerimeterGenerator.cpp:1611-1628's "narrow but
/// not too long" island classification, adapted to this port's own
/// `smaller_perimeter_threshold_mm` / `narrow_loop_length_threshold_mm`
/// config keys (see classic-perimeters.toml).
fn classify_narrow_island(
    polygons: &[ExPolygon],
    threshold_mm: f32,
    min_length_mm: f32,
    arc_tolerance: f32,
) -> bool {
    if polygons.is_empty() {
        return false;
    }
    let opened = opening_ex(
        polygons,
        (threshold_mm / 2.0) as f64,
        OffsetJoinType::Miter,
        arc_tolerance as f64,
    );
    if !opened.is_empty() {
        return false;
    }
    let mut min_x = i64::MAX;
    let mut max_x = i64::MIN;
    let mut min_y = i64::MAX;
    let mut max_y = i64::MIN;
    for poly in polygons {
        for pt in &poly.contour.points {
            min_x = min_x.min(pt.x);
            max_x = max_x.max(pt.x);
            min_y = min_y.min(pt.y);
            max_y = max_y.max(pt.y);
        }
    }
    if min_x > max_x {
        return false;
    }
    let longest_dim_mm = units_to_mm(max_x - min_x).max(units_to_mm(max_y - min_y));
    longest_dim_mm >= min_length_mm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(HashMap::new());
        let module = ClassicPerimeters::on_print_start(&config).unwrap();
        assert_eq!(module.wall_count, 3);
        // R2: inner_wall_line_width is now read per-invocation, not cached.
        // Verify the module still initialises without error (struct fields reduced).
        let _ = module.wall_count();
    }
}
