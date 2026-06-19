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

use std::collections::{BTreeMap, HashMap};

use slicer_core::perimeter_utils::{
    build_wall_flags, expolygon_to_path3d, generate_seam_candidates, point_in_any_polygon,
    BASE_SPEED,
};
use slicer_core::polygon_ops::{offset, OffsetJoinType};
use slicer_core::top_surface_split::split_top_surfaces;
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, LoopType, PaintSemantic,
    PaintValue, WallLoop, WidthProfile,
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
pub struct ClassicPerimeters {
    /// Number of wall loops to generate.
    wall_count: u32,
    /// Extrusion line width in millimeters.
    line_width: f32,
    /// Speed factor for outer walls (outer_wall_speed / BASE_SPEED).
    outer_speed_factor: f32,
    /// Speed factor for inner walls (inner_wall_speed / BASE_SPEED).
    inner_speed_factor: f32,
    /// Arc tolerance for polygon offset operations (mm).
    perimeter_arc_tolerance: f32,
}

#[slicer_module]
impl LayerModule for ClassicPerimeters {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let wall_count = match config.get("wall_count") {
            Some(ConfigValue::Int(n)) => *n as u32,
            _ => 3, // default
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4, // default
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
            line_width,
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
            .unwrap_or(self.outer_speed_factor * BASE_SPEED);
        let inner_wall_speed = _config
            .get_float("inner_wall_speed")
            .map(|s| s as f32)
            .or_else(|| _config.get_int("inner_wall_speed").map(|s| s as f32))
            .unwrap_or(self.inner_speed_factor * BASE_SPEED);
        let outer_speed_factor = outer_wall_speed / BASE_SPEED;
        let inner_speed_factor = inner_wall_speed / BASE_SPEED;

        // Group regions by object so each painted object's model perimeter is
        // traced exactly once (AC-22b bisector-edge dedup).
        let mut by_object: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, region) in regions.iter().enumerate() {
            if region.polygons().is_empty() {
                continue;
            }
            by_object
                .entry(region.object_id().clone())
                .or_default()
                .push(i);
        }

        let empty_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> =
            HashMap::new();

        for indices in by_object.values() {
            // A painted object exposes a shared external contour on its cells.
            let shared_boundary = indices.iter().find_map(|&i| regions[i].external_contour());

            if let Some(boundary) = shared_boundary {
                // Effective wall count for the shared boundary uses the first region's top-shell.
                // Some(0) → blanket 1-wall clamp; Some(N>0) → carve handled per-cell below;
                // None → full wall_count.
                let wall_count = {
                    let top_shell = regions[indices[0]].top_shell_index();
                    if only_one_wall_top && top_shell == Some(0) {
                        1
                    } else {
                        layer_wall_count
                    }
                };
                // Trace the model perimeter ONCE as the outer wall (single loop).
                if wall_count > 0 {
                    let z = regions[indices[0]].z();
                    let bridge = regions[indices[0]].bridge_areas();
                    self.emit_walls(
                        boundary,
                        z,
                        &empty_annotations,
                        true,
                        false,
                        output,
                        wall_count,
                        outer_speed_factor,
                        inner_speed_factor,
                        bridge,
                    )?;
                }
                // Each cell adds only inner walls + infill from its own polygon.
                for &i in indices {
                    let region = &regions[i];
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
                    // Some(N>0) carve: split into top portion (1 wall) and non-top portion
                    // (full wall_count). Pass ORIGINAL region polygons as original_polygons
                    // so that build_wall_flags paint reprojection samples annotations
                    // correctly even on the carved sub-regions.
                    if only_one_wall_top && matches!(top_shell, Some(n) if n > 0) {
                        let split = split_top_surfaces(polygons, region.top_solid_fill());
                        if !split.top_portion.is_empty() {
                            self.emit_walls(
                                &split.top_portion,
                                z,
                                region.segment_annotations(),
                                false,
                                true,
                                output,
                                1,
                                outer_speed_factor,
                                inner_speed_factor,
                                region.bridge_areas(),
                            )?;
                        }
                        if !split.non_top_portion.is_empty() {
                            self.emit_walls(
                                &split.non_top_portion,
                                z,
                                region.segment_annotations(),
                                false,
                                true,
                                output,
                                layer_wall_count,
                                outer_speed_factor,
                                inner_speed_factor,
                                region.bridge_areas(),
                            )?;
                        }
                    } else {
                        self.emit_walls(
                            polygons,
                            z,
                            region.segment_annotations(),
                            false,
                            true,
                            output,
                            wall_count,
                            outer_speed_factor,
                            inner_speed_factor,
                            region.bridge_areas(),
                        )?;
                    }
                }
            } else {
                // Unpainted object: full per-region emission.
                for &i in indices {
                    let region = &regions[i];
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
                    // Some(N>0) carve: split into top portion (1 wall) and non-top portion
                    // (full wall_count). Pass ORIGINAL region polygons as original_polygons
                    // to build_wall_flags so paint reprojection stays correct on carved
                    // sub-regions. (Ref: OrcaSlicer PerimeterGenerator.cpp split_top_surfaces ~L775)
                    if only_one_wall_top && matches!(top_shell, Some(n) if n > 0) {
                        let split = split_top_surfaces(polygons, region.top_solid_fill());
                        if !split.top_portion.is_empty() {
                            self.emit_walls(
                                &split.top_portion,
                                z,
                                region.segment_annotations(),
                                true,
                                true,
                                output,
                                1,
                                outer_speed_factor,
                                inner_speed_factor,
                                region.bridge_areas(),
                            )?;
                        }
                        if !split.non_top_portion.is_empty() {
                            self.emit_walls(
                                &split.non_top_portion,
                                z,
                                region.segment_annotations(),
                                true,
                                true,
                                output,
                                layer_wall_count,
                                outer_speed_factor,
                                inner_speed_factor,
                                region.bridge_areas(),
                            )?;
                        }
                    } else {
                        self.emit_walls(
                            polygons,
                            z,
                            region.segment_annotations(),
                            true,
                            true,
                            output,
                            wall_count,
                            outer_speed_factor,
                            inner_speed_factor,
                            region.bridge_areas(),
                        )?;
                    }
                }
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
    /// `emit_outer` / `emit_inner` gate which bands and the infill are produced
    /// (AC-22b): a painted object's perimeter is traced ONCE from the shared
    /// external contour (`true, false`) so the outer-wall count matches the
    /// unpainted baseline, and each colour cell adds only its inner walls + infill
    /// (`false, true`). Unpainted regions pass `true, true`.
    #[allow(clippy::too_many_arguments)]
    fn emit_walls(
        &self,
        polygons: &[ExPolygon],
        z: f32,
        segment_annotations: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
        emit_outer: bool,
        emit_inner: bool,
        output: &mut PerimeterOutputBuilder,
        wall_count: u32,
        outer_speed_factor: f32,
        inner_speed_factor: f32,
        bridge_areas: &[ExPolygon],
    ) -> Result<(), ModuleError> {
        // Generate wall loops via iterative insets.
        let mut current_polygons = polygons.to_vec();
        let mut all_wall_polygons: Vec<(u32, Vec<ExPolygon>)> = Vec::new();

        for i in 0..wall_count {
            let inset_delta = if i == 0 {
                -(self.line_width / 2.0)
            } else {
                -self.line_width
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
            all_wall_polygons.push((i, inset_result.clone()));
            current_polygons = inset_result;
        }

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
                let points = expolygon_to_path3d(&poly.contour, z, self.line_width);
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
                );
                // Per-vertex is_bridge: set for each vertex strictly inside any bridge area.
                // poly.contour.points has N entries (integer units); feature_flags has N+1
                // (closing repeat appended by expolygon_to_path3d). The closing repeat is
                // handled by mirror_first_to_last below.
                for (i, pt) in poly.contour.points.iter().enumerate() {
                    if i < feature_flags.len() {
                        feature_flags[i].is_bridge = point_in_any_polygon(pt, bridge_areas);
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
                        widths: vec![self.line_width; num_points],
                    },
                    feature_flags,
                    boundary_type,
                };
                output.push_wall_loop(wall)?;
            }
        }

        // Seam candidates belong to the outer wall (the shared-perimeter pass).
        if emit_outer {
            if let Some((_, outer_polys)) = all_wall_polygons.first() {
                for poly in outer_polys {
                    for candidate in generate_seam_candidates(&poly.contour, z) {
                        output.push_seam_candidate(candidate.position, candidate.score)?;
                    }
                }
            }
        }

        // Only the inner/infill pass owns the infill region.
        if emit_inner && !current_polygons.is_empty() {
            let infill = offset(
                &current_polygons,
                -(self.line_width / 2.0),
                OffsetJoinType::Miter,
                self.perimeter_arc_tolerance,
            );
            if !infill.is_empty() {
                output.set_infill_areas(infill)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(HashMap::new());
        let module = ClassicPerimeters::on_print_start(&config).unwrap();
        assert_eq!(module.wall_count, 3);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }
}
