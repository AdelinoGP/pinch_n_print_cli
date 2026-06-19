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

use std::collections::{BTreeMap, HashMap};

use slicer_core::perimeter_utils::{
    build_outer_wall_flags, default_feature_flags, generate_seam_candidates, BASE_SPEED,
};
use slicer_core::polygon_ops::{offset, OffsetJoinType};
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, LoopType, PaintSemantic,
    PaintValue, Point3WithWidth, WallBoundaryType, WallLoop, WidthProfile,
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
pub struct ArachnePerimeters {
    /// Number of wall loops to generate (target, may be fewer in thin regions).
    wall_count: u32,
    /// Nominal extrusion line width in millimeters.
    line_width: f32,
    /// Arc tolerance for polygon offset operations (mm).
    perimeter_arc_tolerance: f32,
}

impl ArachnePerimeters {
    /// Returns the configured wall count.
    pub fn wall_count(&self) -> u32 {
        self.wall_count
    }

    /// Returns the configured nominal line width in millimeters.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

#[slicer_module]
impl LayerModule for ArachnePerimeters {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let wall_count = match config.get("wall_count") {
            Some(ConfigValue::Int(n)) => *n as u32,
            _ => 3,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4,
        };

        let perimeter_arc_tolerance = match config.get("perimeter_arc_tolerance") {
            Some(ConfigValue::Float(v)) => *v as f32,
            _ => 0.0125,
        };

        Ok(Self {
            wall_count,
            line_width,
            perimeter_arc_tolerance,
        })
    }

    /// `_paint` is intentionally unread in this module — consumed by Phase 2
    /// follow-up packet 102.
    fn run_perimeters(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let wall_count = _config
            .get_int("wall_count")
            .map(|n| n as u32)
            .unwrap_or(self.wall_count);
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
                // Trace the model perimeter ONCE as the outer wall — a single loop,
                // so the painted object's outer-wall count matches the unpainted
                // baseline instead of fragmenting across colour cells.
                if wall_count > 0 {
                    let z = regions[indices[0]].z();
                    self.generate_arachne_walls(
                        boundary,
                        z,
                        &empty_annotations,
                        true,
                        false,
                        output,
                        wall_count,
                        outer_speed_factor,
                        inner_speed_factor,
                    )?;
                }
                // Each cell contributes only inner walls + infill from its own
                // polygon (no per-cell outer wall).
                for &i in indices {
                    let region = &regions[i];
                    let polygons = region.polygons();
                    let z = region.z();
                    if wall_count == 0 {
                        output.set_infill_areas(polygons.to_vec())?;
                        continue;
                    }
                    self.generate_arachne_walls(
                        polygons,
                        z,
                        region.segment_annotations(),
                        false,
                        true,
                        output,
                        wall_count,
                        outer_speed_factor,
                        inner_speed_factor,
                    )?;
                }
            } else {
                // Unpainted object: full per-region emission (unchanged).
                for &i in indices {
                    let region = &regions[i];
                    let polygons = region.polygons();
                    let z = region.z();
                    if wall_count == 0 {
                        output.set_infill_areas(polygons.to_vec())?;
                        continue;
                    }
                    self.generate_arachne_walls(
                        polygons,
                        z,
                        region.segment_annotations(),
                        true,
                        true,
                        output,
                        wall_count,
                        outer_speed_factor,
                        inner_speed_factor,
                    )?;
                }
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
    /// produced (AC-22b). For a painted cell group the model perimeter is traced
    /// ONCE from the shared external contour (`emit_outer=true, emit_inner=false`),
    /// and each cell contributes only its inner walls + infill
    /// (`emit_outer=false, emit_inner=true`) — so the perimeter is a single loop
    /// (matching the unpainted baseline count) while infill stays per-colour.
    /// Unpainted regions pass `true, true` (unchanged full emission).
    #[allow(clippy::too_many_arguments)]
    fn generate_arachne_walls(
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
    ) -> Result<(), ModuleError> {
        // Build the boundary rings: boundary[0] = original, boundary[i] = i-th inset
        let mut boundaries: Vec<Vec<ExPolygon>> = Vec::new();
        boundaries.push(polygons.to_vec());

        let mut current = polygons.to_vec();
        for i in 0..wall_count {
            let delta = if i == 0 {
                -(self.line_width / 2.0)
            } else {
                -self.line_width
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
            boundaries.push(inset.clone());
            current = inset;
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

                // Propagate segment_annotations into feature flags for outer walls only
                let (mut feature_flags, wall_boundary_type) = if is_outer {
                    build_outer_wall_flags(num_points, poly_idx, segment_annotations)
                } else {
                    (
                        vec![default_feature_flags(); num_points],
                        WallBoundaryType::Interior,
                    )
                };
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
                output.push_wall_loop(wall)?;
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
                let infill = offset(
                    innermost,
                    -(self.line_width / 2.0),
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
    ) -> Vec<Point3WithWidth> {
        let min_width = self.line_width * MIN_WIDTH_FRACTION;
        let max_width = self.line_width * 2.0;

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
    let (near_dist, near_x, near_y) = nearest_point_on_polygons(point, polygons);

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
    let far_dist = ray_to_polygons(point.x as f64, point.y as f64, dir_x, dir_y, polygons);

    near_dist + far_dist
}

/// Find the nearest point on any polygon contour edge to the given point.
///
/// Returns (distance, nearest_x, nearest_y) in scaled integer units.
fn nearest_point_on_polygons(point: &slicer_ir::Point2, polygons: &[ExPolygon]) -> (f64, f64, f64) {
    let mut min_dist = f64::MAX;
    let mut best_x = 0.0;
    let mut best_y = 0.0;

    for poly in polygons {
        let pts = &poly.contour.points;
        let n = pts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let (d, px, py) = point_to_segment_nearest(point, &pts[i], &pts[j]);
            if d < min_dist {
                min_dist = d;
                best_x = px;
                best_y = py;
            }
        }
    }

    (min_dist, best_x, best_y)
}

/// Cast a ray from (ox, oy) in direction (dx, dy) and find the nearest intersection
/// with any polygon contour edge. Returns the distance in scaled units, or a large
/// value if no intersection is found.
fn ray_to_polygons(ox: f64, oy: f64, dx: f64, dy: f64, polygons: &[ExPolygon]) -> f64 {
    let mut min_t = f64::MAX;
    let ray = Ray { ox, oy, dx, dy };

    for poly in polygons {
        let pts = &poly.contour.points;
        let n = pts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            if let Some(t) = ray_segment_intersect(
                &ray,
                pts[i].x as f64,
                pts[i].y as f64,
                pts[j].x as f64,
                pts[j].y as f64,
            ) {
                if t > 1.0 && t < min_t {
                    // t > 1.0 to skip the near boundary we came from
                    min_t = t;
                }
            }
        }
    }

    if min_t == f64::MAX {
        0.0 // No far boundary found; width is just near_dist
    } else {
        min_t
    }
}

/// Ray origin and direction for intersection tests.
struct Ray {
    ox: f64,
    oy: f64,
    dx: f64,
    dy: f64,
}

/// Compute ray-segment intersection.
///
/// Ray: P = ray.o + t * ray.d, t >= 0
/// Segment: from (ax, ay) to (bx, by)
///
/// Returns Some(t) if the ray intersects the segment, None otherwise.
fn ray_segment_intersect(ray: &Ray, ax: f64, ay: f64, bx: f64, by: f64) -> Option<f64> {
    let sx = bx - ax;
    let sy = by - ay;

    let denom = ray.dx * sy - ray.dy * sx;
    if denom.abs() < 1e-10 {
        return None; // Parallel
    }

    let t = ((ax - ray.ox) * sy - (ay - ray.oy) * sx) / denom;
    let u = ((ax - ray.ox) * ray.dy - (ay - ray.oy) * ray.dx) / denom;

    if t >= 0.0 && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

/// Compute the nearest point on a line segment to a given point.
///
/// Returns (distance, nearest_x, nearest_y) in scaled units.
fn point_to_segment_nearest(
    p: &slicer_ir::Point2,
    a: &slicer_ir::Point2,
    b: &slicer_ir::Point2,
) -> (f64, f64, f64) {
    let dx = (b.x - a.x) as f64;
    let dy = (b.y - a.y) as f64;
    let len_sq = dx * dx + dy * dy;

    if len_sq == 0.0 {
        let dpx = (p.x - a.x) as f64;
        let dpy = (p.y - a.y) as f64;
        return ((dpx * dpx + dpy * dpy).sqrt(), a.x as f64, a.y as f64);
    }

    let t = (((p.x - a.x) as f64 * dx + (p.y - a.y) as f64 * dy) / len_sq).clamp(0.0, 1.0);

    let proj_x = a.x as f64 + t * dx;
    let proj_y = a.y as f64 + t * dy;

    let dpx = p.x as f64 - proj_x;
    let dpy = p.y as f64 - proj_y;
    ((dpx * dpx + dpy * dpy).sqrt(), proj_x, proj_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(HashMap::new());
        let module = ArachnePerimeters::on_print_start(&config).unwrap();
        assert_eq!(module.wall_count, 3);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }

    #[test]
    fn point_to_segment_basic() {
        use slicer_ir::Point2;
        let a = Point2 { x: 0, y: 0 };
        let b = Point2 { x: 10_000, y: 0 }; // 1mm along X
        let p = Point2 { x: 5_000, y: 5_000 }; // 0.5mm above midpoint
        let (d, nx, ny) = point_to_segment_nearest(&p, &a, &b);
        assert!(
            (d - 5_000.0).abs() < 1.0,
            "Distance should be 5000 units (0.5mm)"
        );
        assert!((nx - 5_000.0).abs() < 1.0, "Nearest X should be 5000");
        assert!(ny.abs() < 1.0, "Nearest Y should be 0");
    }
}
