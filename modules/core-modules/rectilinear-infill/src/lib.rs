//! Rectilinear sparse infill generator module.
//!
//! Implements `LayerModule::run_infill` for the `Layer::Infill` stage.
//! Generates parallel scan-line fill patterns with per-layer angle alternation.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point3WithWidth,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::slicer_module;
use slicer_sdk::error::ModuleError;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Rectilinear infill generator.
///
/// Produces parallel fill lines via scan-line polygon intersection,
/// alternating direction by 90 degrees on each layer.
pub struct RectilinearInfill {
    /// Infill density (0.0 to 1.0).
    density: f32,
    /// Base infill angle in degrees.
    base_angle: f32,
    /// Infill print speed in mm/s.
    infill_speed: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
}

#[slicer_module]
impl LayerModule for RectilinearInfill {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let density = match config.get("infill_density") {
            Some(ConfigValue::Float(d)) => *d as f32,
            _ => 0.2,
        };

        let base_angle = match config.get("infill_angle") {
            Some(ConfigValue::Float(a)) => *a as f32,
            _ => 0.0,
        };

        let infill_speed = match config.get("infill_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4,
        };

        Ok(Self {
            density,
            base_angle,
            infill_speed,
            line_width,
        })
    }

    fn run_infill(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if self.density <= 0.0 {
            return Ok(());
        }

        let line_spacing_mm = self.line_width / self.density;
        let line_spacing = slicer_ir::mm_to_units(line_spacing_mm);

        // Compute angle: base + 90 degree alternation per layer
        let layer_rotation = if layer_index.is_multiple_of(2) {
            0.0_f64
        } else {
            90.0_f64
        };
        let angle_deg = self.base_angle as f64 + layer_rotation;
        let angle_rad = angle_deg.to_radians();

        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let speed_factor = self.infill_speed / BASE_SPEED;

        for region in regions {
            let infill_areas = region.infill_areas();
            if infill_areas.is_empty() {
                continue;
            }

            let z = region.z();

            // Determine fill role based on surface classification.
            // Priority: bridge > top > bottom > sparse.
            // Top/bottom surfaces get solid fill; bridge gets bridge fill;
            // everything else gets sparse infill.
            let role = if region.is_bridge() {
                ExtrusionRole::BridgeInfill
            } else if region.is_top_surface() {
                ExtrusionRole::TopSolidInfill
            } else if region.is_bottom_surface() {
                ExtrusionRole::BottomSolidInfill
            } else {
                ExtrusionRole::SparseInfill
            };

            for expoly in infill_areas {
                let paths =
                    self.fill_expolygon(expoly, line_spacing, cos_a, sin_a, z, speed_factor, role.clone());
                for path in paths {
                    let _ = output.push_sparse_path(path);
                }
            }
        }

        Ok(())
    }
}

impl RectilinearInfill {
    /// Generate fill lines for a single ExPolygon.
    #[allow(clippy::too_many_arguments)]
    fn fill_expolygon(
        &self,
        expoly: &ExPolygon,
        line_spacing: i64,
        cos_a: f64,
        sin_a: f64,
        z: f32,
        speed_factor: f32,
        role: ExtrusionRole,
    ) -> Vec<ExtrusionPath3D> {
        // Collect all edges (contour + holes)
        let mut edges: Vec<(i64, i64, i64, i64)> = Vec::new();
        collect_edges(&expoly.contour.points, &mut edges);
        for hole in &expoly.holes {
            collect_edges(&hole.points, &mut edges);
        }

        // Rotate all edge endpoints by -angle into working space
        let rotated_edges: Vec<(i64, i64, i64, i64)> = edges
            .iter()
            .map(|&(x1, y1, x2, y2)| {
                let (rx1, ry1) = rotate_point(x1, y1, cos_a, -sin_a);
                let (rx2, ry2) = rotate_point(x2, y2, cos_a, -sin_a);
                (rx1, ry1, rx2, ry2)
            })
            .collect();

        // Compute bounding box in rotated space
        let (mut min_y, mut max_y) = (i64::MAX, i64::MIN);
        for &(_, ry1, _, ry2) in &rotated_edges {
            min_y = min_y.min(ry1).min(ry2);
            max_y = max_y.max(ry1).max(ry2);
        }

        if min_y >= max_y || line_spacing <= 0 {
            return Vec::new();
        }

        // Generate scan lines
        let mut paths = Vec::new();
        let mut scan_y = min_y + line_spacing;

        while scan_y < max_y {
            // Find intersections with all edges
            let mut x_intersections: Vec<i64> = Vec::new();

            for &(rx1, ry1, rx2, ry2) in &rotated_edges {
                let (edge_min_y, edge_max_y) = if ry1 < ry2 { (ry1, ry2) } else { (ry2, ry1) };

                // Strictly between
                if scan_y > edge_min_y && scan_y < edge_max_y {
                    let x = rx1 as f64
                        + (scan_y - ry1) as f64 * (rx2 - rx1) as f64 / (ry2 - ry1) as f64;
                    x_intersections.push(x.round() as i64);
                }
            }

            x_intersections.sort();

            // Pair intersections as enter/exit segments
            let mut i = 0;
            while i + 1 < x_intersections.len() {
                let x_start = x_intersections[i];
                let x_end = x_intersections[i + 1];

                // Rotate back by +angle
                let (start_x, start_y) = rotate_point(x_start, scan_y, cos_a, sin_a);
                let (end_x, end_y) = rotate_point(x_end, scan_y, cos_a, sin_a);

                let start = Point3WithWidth {
                    x: slicer_ir::units_to_mm(start_x),
                    y: slicer_ir::units_to_mm(start_y),
                    z,
                    width: self.line_width,
                    flow_factor: 1.0,
                };
                let end = Point3WithWidth {
                    x: slicer_ir::units_to_mm(end_x),
                    y: slicer_ir::units_to_mm(end_y),
                    z,
                    width: self.line_width,
                    flow_factor: 1.0,
                };

                paths.push(ExtrusionPath3D {
                    points: vec![start, end],
                    role: role.clone(),
                    speed_factor,
                });

                i += 2;
            }

            scan_y += line_spacing;
        }

        paths
    }
}

/// Collect edges from a polygon's point list as (x1, y1, x2, y2) tuples.
fn collect_edges(points: &[slicer_ir::Point2], edges: &mut Vec<(i64, i64, i64, i64)>) {
    let n = points.len();
    if n < 2 {
        return;
    }
    for i in 0..n {
        let j = (i + 1) % n;
        edges.push((points[i].x, points[i].y, points[j].x, points[j].y));
    }
}

/// Rotate a point by angle. cos_a, sin_a are cos/sin of the rotation angle.
/// x' = x*cos - y*sin, y' = x*sin + y*cos
fn rotate_point(x: i64, y: i64, cos_a: f64, sin_a: f64) -> (i64, i64) {
    let xf = x as f64;
    let yf = y as f64;
    let rx = (xf * cos_a - yf * sin_a).round() as i64;
    let ry = (xf * sin_a + yf * cos_a).round() as i64;
    (rx, ry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(std::collections::HashMap::new(),);
        let module = RectilinearInfill::on_print_start(&config).unwrap();
        assert!((module.density - 0.2).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }
}
