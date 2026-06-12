//! Traditional rectilinear support fill generator module.
//!
//! Implements `LayerModule::run_support` for the `Layer::Support` stage.
//! Generates parallel scan-line fill patterns for support material areas
//! with per-layer 90-degree angle alternation.
//!
//! # Per-layer scan-line nature
//!
//! This module is intentionally a per-layer scan-line filler. Its fill is a set of independent
//! horizontal passes with no cross-layer dependency — each layer is a fresh
//! scan at a rotated angle, deterministic from the layer index alone. It
//! therefore does **not** declare `SupportPlanIR` as a read in its manifest
//! and does **not** consume `PrePass::SupportGeometry` output. The
//! planner-consuming tier is limited to `tree-support`, whose organic
//! branches require multi-layer top-down propagation; see packet
//! `28_tree-support-multi-layer-propagation` and docs/01 §Layer::Support.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point3WithWidth,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView, SupportPaintPolicy};
use slicer_sdk::views::SliceRegionView;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Traditional support fill generator.
///
/// Produces parallel fill lines via scan-line polygon intersection
/// for support material areas, alternating direction by 90 degrees
/// on each layer.
pub struct TraditionalSupport {
    /// Whether support generation is enabled.
    enabled: bool,
    /// Support density (0.0 to 1.0).
    density: f32,
    /// Base support angle in degrees.
    base_angle: f32,
    /// Support print speed in mm/s.
    support_speed: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
}

#[slicer_module]
impl LayerModule for TraditionalSupport {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("support_enabled") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
        };

        let density = match config.get("support_density") {
            Some(ConfigValue::Float(d)) => *d as f32,
            _ => 0.2,
        };

        let base_angle = match config.get("support_angle") {
            Some(ConfigValue::Float(a)) => *a as f32,
            _ => 0.0,
        };

        let support_speed = match config.get("support_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4,
        };

        Ok(Self {
            enabled,
            density,
            base_angle,
            support_speed,
            line_width,
        })
    }

    fn run_support(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut SupportOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled || self.density <= 0.0 {
            return Ok(());
        }

        // `support_density` is declared in traditional-support.toml as a
        // 0-100 percentage (matching OrcaSlicer's UI convention). Convert
        // to a 0-1 ratio before using it as the spacing divisor.
        let density_ratio = (self.density / 100.0).max(f32::EPSILON);
        let line_spacing_mm = self.line_width / density_ratio;
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

        let speed_factor = self.support_speed / BASE_SPEED;

        for region in regions {
            let polygons = region.polygons();
            if polygons.is_empty() {
                continue;
            }

            let z = region.z();

            for expoly in polygons {
                // Eligibility precedence (docs/01 Layer::Support, docs/02
                // support precedence rules):
                //   blocker → skip (always wins)
                //   enforcer → generate (overrides needs_support)
                //   default → consult SurfaceClassificationIR.needs_support
                let _ = layer_index;
                match paint.paint_policy_for(expoly) {
                    SupportPaintPolicy::Blocked => continue,
                    SupportPaintPolicy::Enforced => {}
                    SupportPaintPolicy::DefaultEligible => {
                        if !region.needs_support() {
                            continue;
                        }
                    }
                }

                let paths =
                    self.fill_expolygon(expoly, line_spacing, cos_a, sin_a, z, speed_factor);
                for path in paths {
                    let _ = output.push_support_path(path);
                }
            }
        }

        Ok(())
    }
}

// SupportPaintPolicy was moved to `slicer_sdk::traits::SupportPaintPolicy`
// (packet 95 closure) so that tree-support and traditional-support both consume
// the same query implementation through `PaintRegionLayerView::paint_policy_for`.

impl TraditionalSupport {
    /// Generate fill lines for a single ExPolygon.
    fn fill_expolygon(
        &self,
        expoly: &ExPolygon,
        line_spacing: i64,
        cos_a: f64,
        sin_a: f64,
        z: f32,
        speed_factor: f32,
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
                    overhang_quartile: None,
                };
                let end = Point3WithWidth {
                    x: slicer_ir::units_to_mm(end_x),
                    y: slicer_ir::units_to_mm(end_y),
                    z,
                    width: self.line_width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                };

                paths.push(ExtrusionPath3D {
                    points: vec![start, end],
                    role: ExtrusionRole::SupportMaterial,
                    speed_factor,
                });

                i += 2;
            }

            scan_y += line_spacing;
        }

        // Centroid fallback: when the polygon is smaller than `line_spacing`
        // along the scan axis, the scan-line loop emits nothing. Drop a
        // single horizontal segment across the polygon's centroid so any
        // non-empty support polygon yields at least one fill path.
        if paths.is_empty() {
            let centroid_y = (min_y + max_y) / 2;
            let mut centroid_xs: Vec<i64> = Vec::new();
            for &(rx1, ry1, rx2, ry2) in &rotated_edges {
                let (edge_min_y, edge_max_y) = if ry1 < ry2 { (ry1, ry2) } else { (ry2, ry1) };
                if centroid_y > edge_min_y && centroid_y < edge_max_y {
                    let x = rx1 as f64
                        + (centroid_y - ry1) as f64 * (rx2 - rx1) as f64 / (ry2 - ry1) as f64;
                    centroid_xs.push(x.round() as i64);
                }
            }
            centroid_xs.sort();
            let mut i = 0;
            while i + 1 < centroid_xs.len() {
                let (start_x, start_y) = rotate_point(centroid_xs[i], centroid_y, cos_a, sin_a);
                let (end_x, end_y) = rotate_point(centroid_xs[i + 1], centroid_y, cos_a, sin_a);
                paths.push(ExtrusionPath3D {
                    points: vec![
                        Point3WithWidth {
                            x: slicer_ir::units_to_mm(start_x),
                            y: slicer_ir::units_to_mm(start_y),
                            z,
                            width: self.line_width,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        Point3WithWidth {
                            x: slicer_ir::units_to_mm(end_x),
                            y: slicer_ir::units_to_mm(end_y),
                            z,
                            width: self.line_width,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                    ],
                    role: ExtrusionRole::SupportMaterial,
                    speed_factor,
                });
                i += 2;
            }
        }

        paths
    }
}

// expolygon_centroid was an artifact of the deleted local support_paint_policy
// stub.  The v2 query lives in `PaintRegionLayerView::paint_policy_for` (slicer-sdk).

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
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = TraditionalSupport::on_print_start(&config).unwrap();
        assert!(!module.enabled);
        assert!((module.density - 0.2).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }
}
