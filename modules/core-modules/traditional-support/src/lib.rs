// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/SupportMaterial.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Per-layer rectilinear scan-line filler for Layer::Support
//!
//! Implements `LayerModule::run_support` for the `Layer::Support` stage.
//! Generates parallel scan-line fill patterns for support material areas
//! with per-layer 90-degree angle alternation.
//! Depends entirely on upstream SliceRegionView::needs_support().
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
//!
//! # Speed normalization
//!
//! All extrusion speeds are normalized relative to a base speed:
//! `speed_factor = configured_speed / BASE_SPEED` where `BASE_SPEED = 50.0`.
//! The configured speed is read from the `support_speed` config key at
//! `from_config` and stored as `self.support_speed`.

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
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("enable_support") {
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
            output.begin_region(region.object_id().as_str(), *region.region_id());

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

        // Compute the unrotated bounding-box centre and rotate around it.
        let (mut min_x, mut max_x) = (i64::MAX, i64::MIN);
        let (mut min_y, mut max_y) = (i64::MAX, i64::MIN);
        for &(x1, y1, x2, y2) in &edges {
            min_x = min_x.min(x1).min(x2);
            max_x = max_x.max(x1).max(x2);
            min_y = min_y.min(y1).min(y2);
            max_y = max_y.max(y1).max(y2);
        }
        if min_x >= max_x || min_y >= max_y || line_spacing <= 0 {
            return Vec::new();
        }
        let refpt_x = min_x + (max_x - min_x) / 2;
        let refpt_y = min_y + (max_y - min_y) / 2;

        let rotated_edges: Vec<(i64, i64, i64, i64)> = edges
            .iter()
            .map(|&(x1, y1, x2, y2)| {
                let (rx1, ry1) = rotate_point(x1 - refpt_x, y1 - refpt_y, cos_a, -sin_a);
                let (rx2, ry2) = rotate_point(x2 - refpt_x, y2 - refpt_y, cos_a, -sin_a);
                (rx1, ry1, rx2, ry2)
            })
            .collect();

        // Compute bounding box in rotated space
        let (mut min_y, mut max_y) = (i64::MAX, i64::MIN);
        for &(_, ry1, _, ry2) in &rotated_edges {
            min_y = min_y.min(ry1).min(ry2);
            max_y = max_y.max(ry1).max(ry2);
        }

        if min_y >= max_y {
            return Vec::new();
        }

        // Generate scan lines
        let mut paths = Vec::new();
        let mut scan_y = min_y;

        while scan_y < max_y {
            // Find intersections with all edges
            let mut x_intersections: Vec<i64> = Vec::new();

            for &(rx1, ry1, rx2, ry2) in &rotated_edges {
                if ry1 == ry2 {
                    continue;
                }
                let (lo, hi) = if ry1 < ry2 { (ry1, ry2) } else { (ry2, ry1) };
                if scan_y < lo || scan_y >= hi {
                    continue;
                }
                let x =
                    rx1 as f64 + (scan_y - ry1) as f64 * (rx2 - rx1) as f64 / (ry2 - ry1) as f64;
                x_intersections.push(x.round() as i64);
            }

            x_intersections.sort();

            // Pair intersections as enter/exit segments
            let mut i = 0;
            while i + 1 < x_intersections.len() {
                let x_start = x_intersections[i];
                let x_end = x_intersections[i + 1];

                if x_start == x_end {
                    i += 2;
                    continue;
                }

                // Rotate back by +angle
                let (start_x, start_y) = rotate_point(x_start, scan_y, cos_a, sin_a);
                let (end_x, end_y) = rotate_point(x_end, scan_y, cos_a, sin_a);

                let start = Point3WithWidth {
                    x: slicer_ir::units_to_mm(start_x + refpt_x),
                    y: slicer_ir::units_to_mm(start_y + refpt_y),
                    z,
                    width: self.line_width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                };
                let end = Point3WithWidth {
                    x: slicer_ir::units_to_mm(end_x + refpt_x),
                    y: slicer_ir::units_to_mm(end_y + refpt_y),
                    z,
                    width: self.line_width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                };

                paths.push(ExtrusionPath3D {
                    points: vec![start, end],
                    role: ExtrusionRole::SupportMaterial,
                    speed_factor,
                    tool_index: None,
                });

                i += 2;
            }

            scan_y += line_spacing;
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
    fn from_config_defaults() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = TraditionalSupport::from_config(&config).unwrap();
        assert!(!module.enabled);
        assert!((module.density - 0.2).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }
}
