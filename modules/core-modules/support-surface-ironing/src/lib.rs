// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/Ironing.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Surface ironing module.
//!
//! Implements `LayerModule::run_support_postprocess` for the `Layer::SupportPostProcess` stage.
//! Generates low-flow rectilinear passes over top surfaces to smooth them.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point3WithWidth,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Surface ironing module.
///
/// Produces low-flow rectilinear scan lines over top surfaces (infill areas
/// from perimeter regions) to smooth them, using `ExtrusionRole::Ironing`.
pub struct SupportSurfaceIroning {
    enabled: bool,
    ironing_speed: f32,
    support_ironing_flow: f32,
    support_ironing_spacing: f32,
    line_width: f32,
}

impl SupportSurfaceIroning {
    /// Whether ironing is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Ironing speed in mm/s.
    pub fn ironing_speed(&self) -> f32 {
        self.ironing_speed
    }

    /// Ironing flow multiplier (canonical `support_ironing_flow`, 10% default).
    pub fn support_ironing_flow(&self) -> f32 {
        self.support_ironing_flow
    }

    /// Ironing line spacing in mm.
    pub fn support_ironing_spacing(&self) -> f32 {
        self.support_ironing_spacing
    }

    /// Extrusion line width in mm.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }

    /// Generate ironing scan lines for a single ExPolygon.
    fn fill_expolygon(
        &self,
        expoly: &ExPolygon,
        z: f32,
        speed_factor: f32,
    ) -> Vec<ExtrusionPath3D> {
        let line_spacing = slicer_ir::mm_to_units(self.support_ironing_spacing);

        // Collect all edges (contour + holes)
        let mut edges: Vec<(i64, i64, i64, i64)> = Vec::new();
        collect_edges(&expoly.contour.points, &mut edges);
        for hole in &expoly.holes {
            collect_edges(&hole.points, &mut edges);
        }

        // Compute bounding box
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

        // Generate horizontal scan lines
        let mut paths = Vec::new();
        let mut scan_y = min_y;

        while scan_y < max_y {
            // Find intersections with all edges
            let mut x_intersections: Vec<i64> = Vec::new();

            for &(x1, y1, x2, y2) in &edges {
                if y1 == y2 {
                    continue;
                }
                let (edge_min_y, edge_max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };

                // Include the lower endpoint and exclude the upper endpoint.
                if scan_y < edge_min_y || scan_y >= edge_max_y {
                    continue;
                }
                let x = x1 as f64 + (scan_y - y1) as f64 * (x2 - x1) as f64 / (y2 - y1) as f64;
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

                let start = Point3WithWidth {
                    x: slicer_ir::units_to_mm(x_start),
                    y: slicer_ir::units_to_mm(scan_y),
                    z,
                    width: self.line_width,
                    flow_factor: self.support_ironing_flow,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                };
                let end = Point3WithWidth {
                    x: slicer_ir::units_to_mm(x_end),
                    y: slicer_ir::units_to_mm(scan_y),
                    z,
                    width: self.line_width,
                    flow_factor: self.support_ironing_flow,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                };

                paths.push(ExtrusionPath3D {
                    points: vec![start, end],
                    role: ExtrusionRole::Ironing,
                    speed_factor,
                    tool_index: None,
                    order_lock: None,
                });

                i += 2;
            }

            scan_y += line_spacing;
        }

        paths
    }
}

#[slicer_module]
impl LayerModule for SupportSurfaceIroning {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("ironing_enabled") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
        };

        let ironing_speed = match config.get("ironing_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => 15.0,
        };

        let support_ironing_flow = match config.get("support_ironing_flow") {
            Some(ConfigValue::Float(f)) => *f as f32,
            _ => 0.1,
        };

        let support_ironing_spacing = match config.get("support_ironing_spacing") {
            Some(ConfigValue::Float(s)) => *s as f32,
            _ => 0.1,
        };

        let line_width = match config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4,
        };

        Ok(Self {
            enabled,
            ironing_speed,
            support_ironing_flow,
            support_ironing_spacing,
            line_width,
        })
    }

    fn run_support_postprocess(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut SupportOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        let speed_factor = self.ironing_speed / BASE_SPEED;

        for region in regions {
            // Tag every path pushed below with this region's identity. Without
            // it the builder leaves `current_origin` unset, the host commits
            // through the untagged branch of `convert_support_output_with_plan`
            // (empty object_id / family_id), and the native leg diverges from
            // the wasm leg — which recovers an origin anyway via the host's
            // `touch_slice_region` fallback on the first view accessor call.
            output.begin_region(region.object_id(), *region.region_id());
            let z = region.z();

            let polygons = region.polygons();
            if polygons.is_empty() {
                continue;
            }
            output.begin_region(region.object_id().as_str(), *region.region_id());

            for expoly in polygons {
                let paths = self.fill_expolygon(expoly, z, speed_factor);
                for path in paths {
                    let _ = output.push_support_path(path);
                }
            }
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_config_defaults() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = SupportSurfaceIroning::from_config(&config).unwrap();
        assert!(!module.enabled);
        assert!((module.ironing_speed - 15.0).abs() < 0.001);
        assert!((module.support_ironing_flow - 0.1).abs() < 0.001);
        assert!((module.support_ironing_spacing - 0.1).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }
}
