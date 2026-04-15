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

use slicer_core::polygon_ops::{offset, OffsetJoinType};
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, LoopType, PaintSemantic,
    PaintValue, Point3, Point3WithWidth, WallBoundaryType, WallFeatureFlags, WallLoop,
    WidthProfile,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::slicer_module;
use slicer_sdk::error::ModuleError;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

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
    /// Speed factor for outer walls (outer_wall_speed / BASE_SPEED).
    outer_speed_factor: f32,
    /// Speed factor for inner walls (inner_wall_speed / BASE_SPEED).
    inner_speed_factor: f32,
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
            _ => 2,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4,
        };

        let outer_wall_speed = match config.get("outer_wall_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let inner_wall_speed = match config.get("inner_wall_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        Ok(Self {
            wall_count,
            line_width,
            outer_speed_factor: outer_wall_speed / BASE_SPEED,
            inner_speed_factor: inner_wall_speed / BASE_SPEED,
        })
    }

    fn run_perimeters(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        for region in regions {
            let polygons = region.polygons();
            if polygons.is_empty() {
                continue;
            }

            let z = region.z();

            if self.wall_count == 0 {
                let _ = output.set_infill_areas(polygons.to_vec());
                continue;
            }

            self.generate_arachne_walls(polygons, z, region.boundary_paint(), output);
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
    fn generate_arachne_walls(
        &self,
        polygons: &[ExPolygon],
        z: f32,
        boundary_paint: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
        output: &mut PerimeterOutputBuilder,
    ) {
        // Build the boundary rings: boundary[0] = original, boundary[i] = i-th inset
        let mut boundaries: Vec<Vec<ExPolygon>> = Vec::new();
        boundaries.push(polygons.to_vec());

        let mut current = polygons.to_vec();
        for i in 0..self.wall_count {
            let delta = if i == 0 {
                -(self.line_width / 2.0)
            } else {
                -self.line_width
            };

            let inset = offset(&current, delta, OffsetJoinType::Miter);
            if inset.is_empty() {
                break;
            }
            boundaries.push(inset.clone());
            current = inset;
        }

        // We need at least 2 boundaries to form a wall band (outer + inner boundary)
        if boundaries.len() < 2 {
            // Region too thin for even one wall — make it all infill
            let _ = output.set_infill_areas(polygons.to_vec());
            return;
        }

        let num_walls = boundaries.len() - 1;

        // For each wall band, generate wall loops with variable-width profiles
        for wall_idx in 0..num_walls {
            let is_outer = wall_idx == 0;
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
                self.outer_speed_factor
            } else {
                self.inner_speed_factor
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

                let widths: Vec<f32> = points_with_widths.iter().map(|p| p.width).collect();
                let num_points = points_with_widths.len();

                // Propagate boundary_paint into feature flags for outer walls only
                let (feature_flags, wall_boundary_type) = if is_outer {
                    build_outer_wall_flags(num_points, poly_idx, boundary_paint)
                } else {
                    (
                        vec![default_feature_flags(); num_points],
                        WallBoundaryType::Interior,
                    )
                };

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
                let _ = output.push_wall_loop(wall);
            }
        }

        // Seam candidates from outer wall contours
        if boundaries.len() >= 2 {
            for poly in &boundaries[1] {
                generate_seam_candidates(&poly.contour, z, output);
            }
        }

        // Infill area: inset innermost boundary by half line width
        let innermost = &boundaries[boundaries.len() - 1];
        if !innermost.is_empty() {
            let infill = offset(innermost, -(self.line_width / 2.0), OffsetJoinType::Miter);
            if !infill.is_empty() {
                let _ = output.set_infill_areas(infill);
            }
        }
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

/// Build feature flags for outer wall points by propagating boundary_paint.
///
/// Reads Material and FuzzySkin semantics from `boundary_paint` for the given
/// polygon index. Sets `tool_index` from Material ToolIndex values, `fuzzy_skin`
/// from FuzzySkin Flag values. Detects adjacent material changes and returns
/// `WallBoundaryType::MaterialBoundary` when different tool indices are adjacent.
fn build_outer_wall_flags(
    num_points: usize,
    poly_idx: usize,
    boundary_paint: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
) -> (Vec<WallFeatureFlags>, WallBoundaryType) {
    let mut flags = vec![default_feature_flags(); num_points];

    // Extract per-point Material paint values for this polygon
    let material_values: Option<&Vec<Option<PaintValue>>> = boundary_paint
        .get(&PaintSemantic::Material)
        .and_then(|per_poly| per_poly.get(poly_idx));

    // Extract per-point FuzzySkin paint values for this polygon
    let fuzzy_values: Option<&Vec<Option<PaintValue>>> = boundary_paint
        .get(&PaintSemantic::FuzzySkin)
        .and_then(|per_poly| per_poly.get(poly_idx));

    // Propagate Material -> tool_index
    if let Some(mat_vals) = material_values {
        for (i, flag) in flags.iter_mut().enumerate() {
            if let Some(Some(PaintValue::ToolIndex(tool))) = mat_vals.get(i) {
                flag.tool_index = Some(*tool);
            }
        }
    }

    // Propagate FuzzySkin -> fuzzy_skin
    if let Some(fuzzy_vals) = fuzzy_values {
        for (i, flag) in flags.iter_mut().enumerate() {
            if let Some(Some(PaintValue::Flag(true))) = fuzzy_vals.get(i) {
                flag.fuzzy_skin = true;
            }
        }
    }

    // Detect material boundary: adjacent points with different tool_index
    let has_material_boundary = if let Some(mat_vals) = material_values {
        has_adjacent_material_change(mat_vals)
    } else {
        false
    };

    let boundary_type = if has_material_boundary {
        let adjacent_tool = find_adjacent_tool(material_values.unwrap());
        WallBoundaryType::MaterialBoundary { adjacent_tool }
    } else {
        WallBoundaryType::ExteriorSurface
    };

    (flags, boundary_type)
}

/// Check if adjacent points in a material paint list have different tool indices.
fn has_adjacent_material_change(mat_vals: &[Option<PaintValue>]) -> bool {
    let n = mat_vals.len();
    if n < 2 {
        return false;
    }
    for i in 0..n {
        let next = (i + 1) % n;
        let tool_a = extract_tool_index(&mat_vals[i]);
        let tool_b = extract_tool_index(&mat_vals[next]);
        if tool_a != tool_b {
            return true;
        }
    }
    false
}

/// Find the adjacent tool index from the first material boundary transition.
fn find_adjacent_tool(mat_vals: &[Option<PaintValue>]) -> u32 {
    let n = mat_vals.len();
    for i in 0..n {
        let next = (i + 1) % n;
        let tool_a = extract_tool_index(&mat_vals[i]);
        let tool_b = extract_tool_index(&mat_vals[next]);
        if tool_a != tool_b {
            return tool_b.or(tool_a).unwrap_or(0);
        }
    }
    0
}

/// Extract tool index from a PaintValue, if it is a ToolIndex variant.
fn extract_tool_index(val: &Option<PaintValue>) -> Option<u32> {
    match val {
        Some(PaintValue::ToolIndex(t)) => Some(*t),
        _ => None,
    }
}

/// Create default WallFeatureFlags (no paint, no bridge, no thin wall).
fn default_feature_flags() -> WallFeatureFlags {
    WallFeatureFlags {
        tool_index: None,
        fuzzy_skin: false,
        is_bridge: false,
        is_thin_wall: false,
        skip_ironing: false,
        custom: HashMap::new(),
    }
}

/// Generate seam candidates at sharp corners of the outer wall path.
///
/// All corners with a non-trivial turn angle are candidates. Concave corners
/// receive a higher score (seam is less visible there), convex corners get a
/// lower but positive score.
fn generate_seam_candidates(
    contour: &slicer_ir::Polygon,
    z: f32,
    output: &mut PerimeterOutputBuilder,
) {
    let pts = &contour.points;
    let n = pts.len();
    if n < 3 {
        return;
    }

    // Determine winding via signed area
    let mut signed_area: i128 = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        signed_area += (pts[i].x as i128) * (pts[j].y as i128);
        signed_area -= (pts[j].x as i128) * (pts[i].y as i128);
    }
    let is_ccw = signed_area > 0;

    for i in 0..n {
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let next = (i + 1) % n;

        let dx1 = pts[i].x - pts[prev].x;
        let dy1 = pts[i].y - pts[prev].y;
        let dx2 = pts[next].x - pts[i].x;
        let dy2 = pts[next].y - pts[i].y;

        let cross = dx1 * dy2 - dy1 * dx2;
        if cross == 0 {
            continue;
        }

        let len1 = ((dx1 * dx1 + dy1 * dy1) as f64).sqrt();
        let len2 = ((dx2 * dx2 + dy2 * dy2) as f64).sqrt();
        let denom = len1 * len2;
        if denom == 0.0 {
            continue;
        }

        let sin_angle = (cross.unsigned_abs() as f64 / denom) as f32;
        let is_concave = if is_ccw { cross < 0 } else { cross > 0 };
        let score = if is_concave {
            sin_angle + 1.0
        } else {
            sin_angle * 0.5
        };

        let pos = Point3 {
            x: slicer_ir::units_to_mm(pts[i].x),
            y: slicer_ir::units_to_mm(pts[i].y),
            z,
        };
        let _ = output.push_seam_candidate(pos, score);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(HashMap::new(),);
        let module = ArachnePerimeters::on_print_start(&config).unwrap();
        assert_eq!(module.wall_count, 2);
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
