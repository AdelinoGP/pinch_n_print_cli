// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp
// Original code owner: Ultimaker B.V. (Copyright (c) 2022 Ultimaker B.V.)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Arachne variable-width perimeter generator module.
//!
//! Implements `LayerModule::run_perimeters` for the `Layer::Perimeters` stage.
//! Generates wall loops with variable-width profiles that adapt to local geometry,
//! unlike classic perimeters which use constant-width insets.
//!
//! The algorithm uses iterative polygon insets at fine resolution to approximate
//! the medial-axis approach described in OrcaSlicer's Arachne implementation:
//! - OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.hpp
//! - OrcaSlicerDocumented/generated_documentation/pseudocode_arachne_straight_skeleton.md
//!
//! For regions wider than `wall_count * line_width`, walls are generated at nominal
//! width. For thin regions (narrower than expected), walls are adaptively widened or
//! reduced in count, and width profiles vary per-vertex based on local clearance.

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::HashMap;

use slicer_core::geometry::*;
use slicer_core::perimeter_utils::{
    build_wall_flags, generate_seam_candidates, point_in_any_polygon, wall_sequence_reorder,
    WallSequence, BASE_SPEED,
};
use slicer_core::polygon_ops::{difference_ex, offset, offset2_ex, opening_ex, OffsetJoinType};
use slicer_core::top_surface_split::split_top_surfaces;
use slicer_ir::{
    variable_width, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, LoopType,
    PaintSemantic, PaintValue, Point3WithWidth, WallLoop, WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Minimum extrusion width as a fraction of nominal line_width.
/// Below this, walls are too thin to extrude reliably.
const MIN_WIDTH_FRACTION: f32 = 0.5;

/// Arachne variable-width perimeter generator.
///
/// Produces wall loops with variable-width profiles that adapt to local geometry.
/// Thin regions receive fewer and/or narrower walls rather than being skipped entirely.
/// Width profiles vary per-vertex based on the local distance between adjacent wall
/// boundaries.
///
/// NOTE (P105 R2): Per-object/per-layer overridable config keys
/// (outer_wall_line_width, inner_wall_line_width, wall_sequence,
/// detect_thin_wall, gap_infill_speed, filter_out_gap_fill, precise_outer_wall)
/// are read per-invocation from `_config` in `run_perimeters`, NOT cached here.
/// Only machine constants that cannot change mid-print are cached.
pub struct ArachnePerimeters {
    /// Number of wall loops to generate (target, may be fewer in thin regions).
    wall_count: u32,
    /// Arc tolerance for polygon offset operations (mm).
    perimeter_arc_tolerance: f32,
}

impl ArachnePerimeters {
    /// Returns the configured wall count.
    pub fn wall_count(&self) -> u32 {
        self.wall_count
    }
}

#[slicer_module]
impl LayerModule for ArachnePerimeters {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let wall_count = match config.get("wall_count") {
            Some(ConfigValue::Int(n)) => *n as u32,
            _ => 3,
        };

        let perimeter_arc_tolerance = match config.get("perimeter_arc_tolerance") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.0125,
        };

        Ok(Self {
            wall_count,
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
        let gap_infill_speed = _config
            .get_float("gap_infill_speed")
            .map(|s| s as f32)
            .unwrap_or(30.0);
        let filter_out_gap_fill = _config
            .get_float("filter_out_gap_fill")
            .map(|s| s as f32)
            .unwrap_or(0.5);
        // Medial-axis backend gate (diagnose 2026-06-24) — see classic-perimeters.
        // Skip gap-fill / thin-wall medial axis on painted slices (OOM-prone on
        // degenerate per-color cell gaps) until the medial axis is isolated in a
        // worker subprocess; overridable via `gap_fill_medial_axis_on_painted`.
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
        let only_one_wall_top = _config.get_bool("only_one_wall_top").unwrap_or(false);
        let only_one_wall_first_layer = _config
            .get_bool("only_one_wall_first_layer")
            .unwrap_or(false);
        // Apply first-layer clamp at the layer level (all regions share the same layer).
        let layer_wall_count = if only_one_wall_first_layer && layer_index == 0 {
            1
        } else {
            base_wall_count
        };
        let outer_wall_speed = _config
            .get_float("outer_wall_speed")
            .map(|s| s as f32)
            .or_else(|| _config.get_int("outer_wall_speed").map(|s| s as f32))
            .unwrap_or(30.0);
        let inner_wall_speed = _config
            .get_float("inner_wall_speed")
            .map(|s| s as f32)
            .or_else(|| _config.get_int("inner_wall_speed").map(|s| s as f32))
            .unwrap_or(45.0);
        let outer_speed_factor = outer_wall_speed / BASE_SPEED;
        let inner_speed_factor = inner_wall_speed / BASE_SPEED;

        // Per-color fragmentation (Model A / OrcaSlicer MMU parity): each SlicedRegion
        // traces its own outer+inner walls independently. No union-trace over a shared
        // external contour — that was the old AC-22b approach and is now removed.
        for region in regions {
            output.begin_region(region.object_id(), *region.region_id());
            if region.polygons().is_empty() {
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
            // D14: painted FuzzySkin travels on the region's variant_chain, not
            // segment_annotations. Resolve it once and apply uniformly to every
            // wall vertex of this region via build_wall_flags(variant_fuzzy=…).
            let region_fuzzy = region
                .variant_chain()
                .iter()
                .any(|(sem, val)| sem == "fuzzy_skin" && matches!(val, PaintValue::Flag(true)));
            // Some(N>0) carve: split into top portion (1 wall) and non-top portion
            // (full wall_count). Pass ORIGINAL region polygons as original_polygons
            // to generate_arachne_walls so build_wall_flags paint reprojection
            // stays correct on carved sub-regions.
            // (Ref: OrcaSlicer PerimeterGenerator.cpp split_top_surfaces ~L775)
            if only_one_wall_top && matches!(top_shell, Some(n) if n > 0) {
                let split = split_top_surfaces(polygons, region.top_solid_fill());
                if !split.top_portion.is_empty() {
                    self.generate_arachne_walls(
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
                        outer_wall_line_width,
                        inner_wall_line_width,
                        wall_sequence,
                        precise_outer_wall,
                        detect_thin_wall,
                        nozzle_diameter,
                        gap_infill_speed,
                        filter_out_gap_fill,
                        rid,
                        medial_axis_enabled,
                    )?;
                }
                if !split.non_top_portion.is_empty() {
                    self.generate_arachne_walls(
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
                        outer_wall_line_width,
                        inner_wall_line_width,
                        wall_sequence,
                        precise_outer_wall,
                        detect_thin_wall,
                        nozzle_diameter,
                        gap_infill_speed,
                        filter_out_gap_fill,
                        rid,
                        medial_axis_enabled,
                    )?;
                }
            } else {
                self.generate_arachne_walls(
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
                    outer_wall_line_width,
                    inner_wall_line_width,
                    wall_sequence,
                    precise_outer_wall,
                    detect_thin_wall,
                    nozzle_diameter,
                    gap_infill_speed,
                    filter_out_gap_fill,
                    rid,
                    medial_axis_enabled,
                )?;
            }
        }

        Ok(())
    }
}

impl ArachnePerimeters {
    /// Generate variable-width wall loops using the Arachne approach.
    ///
    /// The algorithm:
    /// 1. Compute iterative polygon insets (outer boundary, then inner boundaries).
    /// 2. For each wall band, determine local width by measuring the distance between
    ///    the outer and inner boundaries of that band at each vertex.
    /// 3. If a region is too thin for the requested wall count, reduce walls and adapt widths.
    ///
    /// `emit_outer` / `emit_inner` gate which wall bands (and the infill) are
    /// produced. All callers now pass `true, true` — each per-color region traces
    /// its own outer+inner walls independently (Model A / OrcaSlicer MMU parity).
    ///
    /// R1 (P105 AC-7): when `precise_outer_wall` is active (precise_outer_wall=true
    /// AND wall_sequence=InnerOuter), the outer inset uses `ext_perimeter_spacing2`
    /// and inner walls are emitted before the outer wall.
    /// OrcaSlicer PerimeterGenerator.cpp:1501-1506,1644
    #[allow(clippy::too_many_arguments)]
    fn generate_arachne_walls(
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
    ) -> Result<(), ModuleError> {
        // Build the boundary rings: boundary[0] = original, boundary[i] = i-th inset
        let mut boundaries: Vec<Vec<ExPolygon>> = Vec::new();
        boundaries.push(polygons.to_vec());

        // Gap-fill (T-063/T-064): collect gaps BETWEEN consecutive perimeter
        // insets (OrcaSlicer PerimeterGenerator.cpp:1665-1670), inner transitions
        // only (i >= 1). The region-boundary → first-wall transition (i == 0) is
        // never a gap source, so the per-color MMU bisector edge (ADR-0013 Model A)
        // does not spawn phantom gap-fill slivers along every color boundary.
        let mut gaps: Vec<ExPolygon> = Vec::new();
        let mut current = polygons.to_vec();
        for i in 0..wall_count {
            let delta = if i == 0 {
                // R1 (P105 AC-7): precise mode uses ext_perimeter_spacing2 for the
                // outer wall inset.
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

            let inset = offset(
                &current,
                delta,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            if inset.is_empty() {
                break;
            }
            // OrcaSlicer gap collection between perimeter (i-1) and perimeter i.
            if i >= 1 {
                let distance = delta.abs();
                let shrunk_prev = offset(
                    &current,
                    -(0.5 * distance),
                    OffsetJoinType::Miter,
                    self.perimeter_arc_tolerance,
                );
                let grown_cur = offset(
                    &inset,
                    0.5 * distance,
                    OffsetJoinType::Miter,
                    self.perimeter_arc_tolerance,
                );
                gaps.extend(difference_ex(&shrunk_prev, &grown_cur));
            }
            boundaries.push(inset.clone());
            current = inset;
        }

        // Final infill-transition gap (OrcaSlicer parity — see classic-perimeters).
        // ~empty for WIDE regions (infill fills the center) but equals the whole
        // leftover core for THIN features where no infill fits, so thin arms/ribs
        // become gap-fill without re-introducing per-color bisector ring slivers.
        if !current.is_empty() {
            let distance = inner_wall_line_width;
            let infill_area = offset(
                &current,
                -distance,
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            let shrunk_inner = offset(
                &current,
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

        // We need at least 2 boundaries to form a wall band (outer + inner boundary)
        if boundaries.len() < 2 {
            // Region too thin for even one wall — make it all infill (only the
            // inner/infill pass owns infill; the shared outer-wall pass does not).
            if emit_inner {
                output.set_infill_areas(polygons.to_vec())?;
            }
            return Ok(());
        }

        let num_walls = boundaries.len() - 1;

        let mut walls: Vec<slicer_ir::WallLoop> = Vec::new();

        // For each wall band, generate wall loops with variable-width profiles
        for wall_idx in 0..num_walls {
            let is_outer = wall_idx == 0;
            // AC-22b: emit only the requested bands (outer-once / inner-per-cell).
            if (is_outer && !emit_outer) || (!is_outer && !emit_inner) {
                continue;
            }
            let _outer_boundary = &boundaries[wall_idx];
            let inner_boundary = &boundaries[wall_idx + 1];

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

            // Generate wall paths from the inner boundary of each band
            // (the centerline of the wall band is approximately the inner inset)
            for (poly_idx, inner_poly) in inner_boundary.iter().enumerate() {
                let points_with_widths = self.compute_variable_width_path(
                    &inner_poly.contour,
                    &boundaries[0], // original polygon boundary for region width
                    num_walls,
                    wall_idx,
                    z,
                    outer_wall_line_width,
                );

                if points_with_widths.is_empty() {
                    continue;
                }

                // Wall loops carry an explicit closing repeat (OrcaSlicer
                // `ExtrusionPath::is_closed()` convention; ExtrusionEntity.hpp:269).
                // Append the first point/width as the last entry so seam-placer,
                // fuzzy-skin, and the G-code emitter all process the closing
                // edge exactly like every other wall segment. Without this,
                // a square wall emits 3 of 4 G1 segments.
                let mut points_with_widths = points_with_widths;
                slicer_sdk::close_loop(&mut points_with_widths);
                let widths: Vec<f32> = points_with_widths.iter().map(|p| p.width).collect();
                let num_points = points_with_widths.len();

                // Propagate segment_annotations into feature flags for outer and inner walls.
                // For inner walls, pass the inset ring's vertex positions and the original
                // polygons so build_wall_flags can use geometric reprojection to correctly
                // sample annotations near concave features.
                let ring_pts: Option<&[slicer_ir::Point2]> = if is_outer {
                    None
                } else {
                    Some(&inner_poly.contour.points)
                };
                let orig_polys: Option<&[ExPolygon]> = if is_outer { None } else { Some(polygons) };
                let (mut feature_flags, wall_boundary_type) = build_wall_flags(
                    num_points,
                    poly_idx,
                    segment_annotations,
                    is_outer,
                    ring_pts,
                    orig_polys,
                    variant_fuzzy,
                );
                // Per-vertex is_bridge: set for each vertex strictly inside any bridge area.
                // inner_poly.contour.points has N entries (integer units); feature_flags has
                // N+1 (closing repeat appended by close_loop). The closing repeat is handled
                // by mirror_first_to_last below.
                for (i, pt) in inner_poly.contour.points.iter().enumerate() {
                    if i < feature_flags.len() {
                        feature_flags[i].is_bridge = point_in_any_polygon(pt, bridge_areas);
                    }
                }
                // Closing-repeat carries the same flag as its identical first vertex.
                slicer_sdk::mirror_first_to_last(&mut feature_flags);

                let wall = WallLoop {
                    perimeter_index: wall_idx as u32,
                    loop_type,
                    path: ExtrusionPath3D {
                        points: points_with_widths,
                        role: role.clone(),
                        speed_factor,
                    },
                    width_profile: WidthProfile { widths },
                    feature_flags,
                    boundary_type: wall_boundary_type,
                };
                walls.push(wall);
            }
        }

        // R1 (P105 AC-7): precise mode reorders inner walls before outer.
        // When precise_outer_wall is active (gated on InnerOuter), emit inner
        // walls first, then outer wall.
        // OrcaSlicer PerimeterGenerator.cpp:1644
        if precise_outer_wall {
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
        // Gaps collected incrementally between consecutive insets above. Apply the
        // OrcaSlicer width-band pre-filter (PerimeterGenerator.cpp:1924-1928) before
        // the medial axis: keep only gaps in [min, max] width. Removes the sub-/
        // super-threshold slivers that drove the RNG medial-axis (non-determinism).
        if emit_inner && medial_axis_enabled {
            // R4 (P105): OrcaSlicer parity gap-fill min_width.
            // OrcaSlicer PerimeterGenerator.cpp:1924.
            let min_gap_fill_width = 0.2 * inner_wall_line_width * (1.0 - 0.2_f32);
            // OrcaSlicer max = 2 * perimeter_spacing (PerimeterGenerator.cpp:1947).
            let perimeter_spacing = (outer_wall_line_width + inner_wall_line_width) / 2.0;
            let max_gap_fill_width = 2.0 * perimeter_spacing;
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
            if !filtered_gaps.is_empty() {
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
                            let flags = vec![
                                slicer_core::perimeter_utils::default_feature_flags();
                                num_pts
                            ];
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
        }

        // Seam candidates belong to the outer wall (the shared-perimeter pass).
        if emit_outer && boundaries.len() >= 2 {
            for poly in &boundaries[1] {
                for candidate in generate_seam_candidates(&poly.contour, z) {
                    output.push_seam_candidate(candidate.position, candidate.score)?;
                }
            }
        }

        // Infill area: inset innermost boundary by half line width. Only the
        // inner/infill pass owns infill (the shared outer-wall pass must not, or it
        // would overwrite each cell's per-colour infill region).
        if emit_inner {
            let innermost = &boundaries[boundaries.len() - 1];
            if !innermost.is_empty() {
                // Inset by a FULL inner-wall width (not half) so the infill region
                // is consistent with the gap-fill infill-transition collection:
                // wide regions keep a non-empty infill center, thin features inset
                // to empty and are owned entirely by gap-fill (no double-count).
                let infill = offset(
                    innermost,
                    -inner_wall_line_width,
                    OffsetJoinType::Miter,
                    self.perimeter_arc_tolerance,
                );
                if !infill.is_empty() {
                    output.set_infill_areas(infill)?;
                }
            }
        }

        Ok(())
    }

    /// Compute a variable-width path for one wall loop.
    ///
    /// For each vertex on the inner boundary contour, compute the local region width
    /// by finding the nearest outer boundary point and then measuring the distance to
    /// the outer boundary on the opposite side (via ray casting). The wall width equals
    /// the full local clearance divided by the number of walls sharing that band,
    /// clamped to `[MIN_WIDTH_FRACTION * line_width, 2 * line_width]`.
    fn compute_variable_width_path(
        &self,
        contour: &slicer_ir::Polygon,
        outer_boundary: &[ExPolygon],
        num_walls: usize,
        _wall_idx: usize,
        z: f32,
        outer_wall_line_width: f32,
    ) -> Vec<Point3WithWidth> {
        let min_width = outer_wall_line_width * MIN_WIDTH_FRACTION;
        let max_width = outer_wall_line_width * 2.0;

        contour
            .points
            .iter()
            .map(|p| {
                let px = slicer_ir::units_to_mm(p.x);
                let py = slicer_ir::units_to_mm(p.y);

                // Compute the full local region width at this vertex
                let region_width = compute_local_region_width(p, outer_boundary);
                let region_width_mm = slicer_ir::units_to_mm(region_width as i64);

                // Distribute the available width among the walls
                // Each wall gets region_width / num_walls
                let local_width = (region_width_mm / num_walls as f32).clamp(min_width, max_width);

                Point3WithWidth {
                    x: px,
                    y: py,
                    z,
                    width: local_width,
                    flow_factor: 1.0,
                    // overhang_quartile: None — placeholder; sibling roadmap item O-T031 in
                    // docs/specs/overhang-pipeline-restructuring.md is the future producer.
                    overhang_quartile: None,
                }
            })
            .collect()
    }
}

/// Compute the local region width at a point inside a polygon boundary.
///
/// Finds the nearest point on the outer boundary, then casts a ray in the
/// opposite direction to find the distance to the far side of the boundary.
/// Returns the total width (near distance + far distance) in scaled units.
fn compute_local_region_width(point: &slicer_ir::Point2, polygons: &[ExPolygon]) -> f64 {
    let cp = closest_point_on_polygons(*point, polygons).unwrap_or(ClosestPoint {
        point: *point,
        distance_sq: 0.0,
    });
    let near_dist = cp.distance_sq.sqrt();
    let near_x = cp.point.x as f64;
    let near_y = cp.point.y as f64;

    if near_dist < 1.0 {
        // Point is essentially on the boundary
        return 0.0;
    }

    // Direction from nearest boundary point to the interior point
    let dx = point.x as f64 - near_x;
    let dy = point.y as f64 - near_y;
    let len = (dx * dx + dy * dy).sqrt();
    let dir_x = dx / len;
    let dir_y = dy / len;

    // Cast a ray from the point in the same direction (continuing away from
    // the nearest boundary) to find the opposite boundary
    // legacy: when far boundary not found, width is just near_dist — documented intent preserved during promotion.
    let far_dist = ray_to_polygons(
        &Ray {
            origin: *point,
            direction: Vec2 { x: dir_x, y: dir_y },
        },
        polygons,
    )
    .map(|hit| hit.distance)
    .unwrap_or(0.0);

    near_dist + far_dist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(HashMap::new());
        let module = ArachnePerimeters::on_print_start(&config).unwrap();
        assert_eq!(module.wall_count, 3);
        // R2: inner/outer wall widths and wall_sequence are now read per-invocation.
        // Verify the module still initialises without error.
        let _ = module.wall_count();
    }

    #[test]
    fn point_to_segment_basic() {
        use slicer_ir::Point2;
        let a = Point2 { x: 0, y: 0 };
        let b = Point2 { x: 10_000, y: 0 }; // 1mm along X
        let p = Point2 { x: 5_000, y: 5_000 }; // 0.5mm above midpoint
        let cp = closest_point_on_segment(p, a, b);
        let d = cp.distance_sq.sqrt();
        let nx = cp.point.x as f64;
        let ny = cp.point.y as f64;
        assert!(
            (d - 5_000.0).abs() < 1.0,
            "Distance should be 5000 units (0.5mm)"
        );
        assert!((nx - 5_000.0).abs() < 1.0, "Nearest X should be 5000");
        assert!(ny.abs() < 1.0, "Nearest Y should be 0");
    }
}
