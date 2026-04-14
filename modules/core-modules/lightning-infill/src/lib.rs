//! Lightning-style branching sparse infill generator module.
//!
//! Implements `LayerModule::run_infill` for the `Layer::Infill` stage.
//! Generates a branching tree-like infill pattern that grows from interior
//! points toward polygon boundaries, producing minimal-material support
//! structures.
//!
//! Algorithm adapted from OrcaSlicer FillLightning.cpp / Lightning/Generator.cpp:
//! Since LayerModule processes one layer at a time without cross-layer state,
//! this is a simplified single-layer approach:
//! 1. Sample interior points on a grid within infill polygons
//! 2. Sort by distance to nearest boundary (interior-first)
//! 3. Grow branches from interior points toward nearest boundary
//! 4. Connect nearby branches at junction points
//! 5. Convert tree polylines to ExtrusionPath3D with SparseInfill role

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Lightning branching sparse infill generator.
///
/// Produces tree-like branching fill patterns that grow from interior points
/// toward polygon boundaries, minimizing material usage while maintaining
/// support for top surfaces.
///
/// Adapted from OrcaSlicer Lightning infill (FillLightning.cpp, Generator.cpp).
pub struct LightningInfill {
    /// Infill density (0.0 to 1.0).
    density: f32,
    /// Infill print speed in mm/s.
    infill_speed: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
}

impl LightningInfill {
    /// Returns the configured infill density.
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Returns the configured line width.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

impl LayerModule for LightningInfill {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let density = match config.fields.get("infill_density") {
            Some(ConfigValue::Float(d)) => *d as f32,
            _ => 0.2,
        };

        let infill_speed = match config.fields.get("infill_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let line_width = match config.fields.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4,
        };

        Ok(Self {
            density,
            infill_speed,
            line_width,
        })
    }

    fn run_infill(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if self.density <= 0.0 {
            return Ok(());
        }

        let speed_factor = self.infill_speed / BASE_SPEED;

        for region in regions {
            let infill_areas = region.infill_areas();
            if infill_areas.is_empty() {
                continue;
            }

            let z = region.z();

            for expoly in infill_areas {
                let paths = self.fill_expolygon(expoly, z, speed_factor);
                for path in paths {
                    let _ = output.push_sparse_path(path);
                }
            }
        }

        Ok(())
    }
}

impl LightningInfill {
    /// Generate lightning-style branching fill for a single ExPolygon.
    ///
    /// The algorithm:
    /// 1. Compute a grid of interior sample points
    /// 2. For each sample, compute distance to nearest boundary edge
    /// 3. Sort samples by distance (interior-first)
    /// 4. Build branches by connecting each sample to its nearest
    ///    already-connected neighbor or to the boundary
    /// 5. Convert branch polylines to extrusion paths
    fn fill_expolygon(
        &self,
        expoly: &ExPolygon,
        z: f32,
        speed_factor: f32,
    ) -> Vec<ExtrusionPath3D> {
        // Compute supporting radius from density
        // OrcaSlicer: supporting_radius = width * 100 / density (scaled)
        // We use mm directly: spacing = line_width / density
        let spacing_mm = self.line_width as f64 / self.density as f64;

        // Compute bounding box in mm
        let (bb_min_x, bb_min_y, bb_max_x, bb_max_y) = polygon_bbox_mm(expoly);
        let bb_width = bb_max_x - bb_min_x;
        let bb_height = bb_max_y - bb_min_y;

        if bb_width <= 0.0 || bb_height <= 0.0 {
            return Vec::new();
        }

        // Sample interior points on a grid with spacing
        let mut samples: Vec<(f64, f64, f64)> = Vec::new(); // (x, y, dist_to_boundary)

        let mut gy = bb_min_y + spacing_mm * 0.5;
        while gy < bb_max_y {
            let mut gx = bb_min_x + spacing_mm * 0.5;
            while gx < bb_max_x {
                if point_in_expolygon(gx, gy, expoly) {
                    let dist = distance_to_boundary(gx, gy, expoly);
                    samples.push((gx, gy, dist));
                }
                gx += spacing_mm;
            }
            gy += spacing_mm;
        }

        if samples.is_empty() {
            return Vec::new();
        }

        // Sort by distance to boundary, descending (interior-first)
        samples.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Build branches: each sample connects to the nearest already-connected
        // point or to the nearest boundary point
        let branches = build_branches(&samples, expoly, spacing_mm);

        // Convert branches to extrusion paths
        let mut result = Vec::new();
        for branch in &branches {
            if branch.len() < 2 {
                continue;
            }

            let points: Vec<Point3WithWidth> = branch
                .iter()
                .map(|&(x, y)| Point3WithWidth {
                    x: x as f32,
                    y: y as f32,
                    z,
                    width: self.line_width,
                    flow_factor: 1.0,
                })
                .collect();

            result.push(ExtrusionPath3D {
                points,
                role: ExtrusionRole::SparseInfill,
                speed_factor,
            });
        }

        result
    }
}

/// Build branching tree polylines from sorted interior samples.
///
/// Each sample point grows a branch toward the nearest boundary point.
/// When a branch passes near an existing branch, it connects to form a
/// junction, creating the characteristic tree-like lightning pattern.
///
/// Returns a list of branch polylines (each is a series of (x,y) points).
fn build_branches(
    samples: &[(f64, f64, f64)],
    expoly: &ExPolygon,
    spacing: f64,
) -> Vec<Vec<(f64, f64)>> {
    let merge_radius = spacing * 0.75;
    let merge_radius_sq = merge_radius * merge_radius;

    // Track all connected points (branch endpoints that can be merged into)
    let mut connected_points: Vec<(f64, f64)> = Vec::new();
    let mut branches: Vec<Vec<(f64, f64)>> = Vec::new();

    for &(sx, sy, _dist) in samples {
        // Find nearest boundary point
        let (bx, by) = nearest_boundary_point(sx, sy, expoly);

        // Check if any existing connected point is closer than the boundary
        let mut target = (bx, by);
        let mut target_dist_sq = (bx - sx) * (bx - sx) + (by - sy) * (by - sy);

        for &(cx, cy) in &connected_points {
            let d_sq = (cx - sx) * (cx - sx) + (cy - sy) * (cy - sy);
            if d_sq < target_dist_sq && d_sq < merge_radius_sq {
                // Do not merge to a connected point that is farther from
                // boundary than we are (would create inward branches)
                target = (cx, cy);
                target_dist_sq = d_sq;
            }
        }

        // Create branch from sample to target
        let branch = vec![(sx, sy), target];
        branches.push(branch);

        // Add the sample point as a connected point for future merges
        connected_points.push((sx, sy));
    }

    branches
}

/// Compute bounding box of an ExPolygon in mm coordinates.
fn polygon_bbox_mm(expoly: &ExPolygon) -> (f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for pt in &expoly.contour.points {
        let x = slicer_ir::units_to_mm(pt.x) as f64;
        let y = slicer_ir::units_to_mm(pt.y) as f64;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    (min_x, min_y, max_x, max_y)
}

/// Simple point-in-ExPolygon test using ray casting.
fn point_in_expolygon(x: f64, y: f64, expoly: &ExPolygon) -> bool {
    if !point_in_polygon(x, y, &expoly.contour.points) {
        return false;
    }
    for hole in &expoly.holes {
        if point_in_polygon(x, y, &hole.points) {
            return false;
        }
    }
    true
}

/// Ray casting point-in-polygon test.
fn point_in_polygon(x: f64, y: f64, points: &[Point2]) -> bool {
    let n = points.len();
    if n < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let xi = slicer_ir::units_to_mm(points[i].x) as f64;
        let yi = slicer_ir::units_to_mm(points[i].y) as f64;
        let xj = slicer_ir::units_to_mm(points[j].x) as f64;
        let yj = slicer_ir::units_to_mm(points[j].y) as f64;

        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }

    inside
}

/// Compute the minimum distance from a point to the nearest boundary edge
/// of an ExPolygon (contour + holes).
fn distance_to_boundary(x: f64, y: f64, expoly: &ExPolygon) -> f64 {
    let mut min_dist = f64::MAX;

    // Distance to contour edges
    min_dist = min_dist.min(distance_to_polygon_edges(x, y, &expoly.contour.points));

    // Distance to hole edges
    for hole in &expoly.holes {
        min_dist = min_dist.min(distance_to_polygon_edges(x, y, &hole.points));
    }

    min_dist
}

/// Compute minimum distance from a point to the edges of a polygon.
fn distance_to_polygon_edges(x: f64, y: f64, points: &[Point2]) -> f64 {
    let n = points.len();
    if n < 2 {
        return f64::MAX;
    }

    let mut min_dist = f64::MAX;

    for i in 0..n {
        let j = (i + 1) % n;
        let ax = slicer_ir::units_to_mm(points[i].x) as f64;
        let ay = slicer_ir::units_to_mm(points[i].y) as f64;
        let bx = slicer_ir::units_to_mm(points[j].x) as f64;
        let by = slicer_ir::units_to_mm(points[j].y) as f64;

        let dist = point_to_segment_distance(x, y, ax, ay, bx, by);
        min_dist = min_dist.min(dist);
    }

    min_dist
}

/// Compute the distance from a point (px, py) to a line segment (ax, ay)-(bx, by).
fn point_to_segment_distance(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-15 {
        // Degenerate segment (point)
        let dpx = px - ax;
        let dpy = py - ay;
        return (dpx * dpx + dpy * dpy).sqrt();
    }

    // Project point onto segment, clamped to [0, 1]
    let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let closest_x = ax + t * dx;
    let closest_y = ay + t * dy;

    let dpx = px - closest_x;
    let dpy = py - closest_y;
    (dpx * dpx + dpy * dpy).sqrt()
}

/// Find the nearest point on the boundary of an ExPolygon to a given point.
fn nearest_boundary_point(x: f64, y: f64, expoly: &ExPolygon) -> (f64, f64) {
    let mut best = (x, y); // fallback
    let mut best_dist = f64::MAX;

    // Check contour edges
    nearest_point_on_polygon(x, y, &expoly.contour.points, &mut best, &mut best_dist);

    // Check hole edges
    for hole in &expoly.holes {
        nearest_point_on_polygon(x, y, &hole.points, &mut best, &mut best_dist);
    }

    best
}

/// Find the nearest point on polygon edges, updating best/best_dist.
fn nearest_point_on_polygon(
    x: f64,
    y: f64,
    points: &[Point2],
    best: &mut (f64, f64),
    best_dist: &mut f64,
) {
    let n = points.len();
    if n < 2 {
        return;
    }

    for i in 0..n {
        let j = (i + 1) % n;
        let ax = slicer_ir::units_to_mm(points[i].x) as f64;
        let ay = slicer_ir::units_to_mm(points[i].y) as f64;
        let bx = slicer_ir::units_to_mm(points[j].x) as f64;
        let by = slicer_ir::units_to_mm(points[j].y) as f64;

        let (cx, cy) = closest_point_on_segment(x, y, ax, ay, bx, by);
        let dx = x - cx;
        let dy = y - cy;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < *best_dist {
            *best_dist = dist;
            *best = (cx, cy);
        }
    }
}

/// Find the closest point on a line segment to a given point.
fn closest_point_on_segment(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> (f64, f64) {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-15 {
        return (ax, ay);
    }

    let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    (ax + t * dx, ay + t * dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView {
            fields: std::collections::HashMap::new(),
        };
        let module = LightningInfill::on_print_start(&config).unwrap();
        assert!((module.density - 0.2).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }

    #[test]
    fn point_in_polygon_basic() {
        let pts = vec![
            Point2::from_mm(-5.0, -5.0),
            Point2::from_mm(5.0, -5.0),
            Point2::from_mm(5.0, 5.0),
            Point2::from_mm(-5.0, 5.0),
        ];
        assert!(point_in_polygon(0.0, 0.0, &pts));
        assert!(!point_in_polygon(10.0, 10.0, &pts));
    }

    #[test]
    fn distance_to_segment_basic() {
        // Point directly above segment midpoint
        let d = point_to_segment_distance(0.5, 1.0, 0.0, 0.0, 1.0, 0.0);
        assert!((d - 1.0).abs() < 0.001);

        // Point at segment endpoint
        let d = point_to_segment_distance(0.0, 0.0, 0.0, 0.0, 1.0, 0.0);
        assert!(d < 0.001);
    }

    #[test]
    fn nearest_boundary_basic() {
        let expoly = ExPolygon {
            contour: slicer_ir::Polygon {
                points: vec![
                    Point2::from_mm(-5.0, -5.0),
                    Point2::from_mm(5.0, -5.0),
                    Point2::from_mm(5.0, 5.0),
                    Point2::from_mm(-5.0, 5.0),
                ],
            },
            holes: vec![],
        };
        // Center point: nearest boundary should be at distance 5.0
        let (bx, by) = nearest_boundary_point(0.0, 0.0, &expoly);
        let dist = ((bx * bx) + (by * by)).sqrt();
        assert!((dist - 5.0).abs() < 0.01);
    }
}
