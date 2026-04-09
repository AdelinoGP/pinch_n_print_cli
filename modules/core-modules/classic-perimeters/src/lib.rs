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

use slicer_core::polygon_ops::{offset, OffsetJoinType};
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, LoopType, Point3,
    Point3WithWidth, WallBoundaryType, WallFeatureFlags, WallLoop, WidthProfile,
};
use slicer_sdk::error::ModuleError;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::views::SliceRegionView;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

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
}

impl LayerModule for ClassicPerimeters {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let wall_count = match config.fields.get("wall_count") {
            Some(ConfigValue::Int(n)) => *n as u32,
            _ => 2, // default
        };

        let line_width = match config.fields.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => 0.4, // default
        };

        let outer_wall_speed = match config.fields.get("outer_wall_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let inner_wall_speed = match config.fields.get("inner_wall_speed") {
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
                // No walls — entire input becomes infill area
                let _ = output.set_infill_areas(polygons.to_vec());
                continue;
            }

            // Generate wall loops via iterative insets
            let mut current_polygons = polygons.to_vec();
            let mut all_wall_polygons: Vec<(u32, Vec<ExPolygon>)> = Vec::new();

            for i in 0..self.wall_count {
                let inset_delta = if i == 0 {
                    // Outer wall: inset by half line width
                    -(self.line_width / 2.0)
                } else {
                    // Inner walls: inset by full line width from previous
                    -self.line_width
                };

                let inset_result = offset(&current_polygons, inset_delta, OffsetJoinType::Miter);
                if inset_result.is_empty() {
                    break;
                }

                all_wall_polygons.push((i, inset_result.clone()));
                current_polygons = inset_result;
            }

            // Push wall loops
            for (perimeter_index, wall_polys) in &all_wall_polygons {
                let is_outer = *perimeter_index == 0;
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
                let boundary_type = if is_outer {
                    WallBoundaryType::ExteriorSurface
                } else {
                    WallBoundaryType::Interior
                };
                let speed_factor = if is_outer {
                    self.outer_speed_factor
                } else {
                    self.inner_speed_factor
                };

                for poly in wall_polys {
                    let points = expolygon_to_path3d(&poly.contour, z, self.line_width);
                    if points.is_empty() {
                        continue;
                    }
                    let num_points = points.len();

                    let wall = WallLoop {
                        perimeter_index: *perimeter_index,
                        loop_type: loop_type.clone(),
                        path: ExtrusionPath3D {
                            points,
                            role: role.clone(),
                            speed_factor,
                        },
                        width_profile: WidthProfile {
                            widths: vec![self.line_width; num_points],
                        },
                        feature_flags: vec![default_feature_flags(); num_points],
                        boundary_type: boundary_type.clone(),
                    };
                    let _ = output.push_wall_loop(wall);
                }
            }

            // Generate seam candidates from outer wall concave corners
            if let Some((_, outer_polys)) = all_wall_polygons.first() {
                for poly in outer_polys {
                    generate_seam_candidates(&poly.contour, z, output);
                }
            }

            // Compute infill area: inset innermost wall by half line width
            if !current_polygons.is_empty() {
                let infill = offset(
                    &current_polygons,
                    -(self.line_width / 2.0),
                    OffsetJoinType::Miter,
                );
                if !infill.is_empty() {
                    let _ = output.set_infill_areas(infill);
                }
            }
        }

        Ok(())
    }
}

/// Convert an ExPolygon contour to a Vec<Point3WithWidth> at the given Z and width.
///
/// Converts from scaled i64 coordinates to f32 mm.
fn expolygon_to_path3d(
    contour: &slicer_ir::Polygon,
    z: f32,
    width: f32,
) -> Vec<Point3WithWidth> {
    contour
        .points
        .iter()
        .map(|p| Point3WithWidth {
            x: slicer_ir::units_to_mm(p.x),
            y: slicer_ir::units_to_mm(p.y),
            z,
            width,
            flow_factor: 1.0,
        })
        .collect()
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
            // Collinear — not a real corner
            continue;
        }

        let len1 = ((dx1 * dx1 + dy1 * dy1) as f64).sqrt();
        let len2 = ((dx2 * dx2 + dy2 * dy2) as f64).sqrt();
        let denom = len1 * len2;
        if denom == 0.0 {
            continue;
        }

        let sin_angle = (cross.unsigned_abs() as f64 / denom) as f32;
        // Concave corners score higher (seam hides better)
        let is_concave = if is_ccw { cross < 0 } else { cross > 0 };
        let score = if is_concave {
            sin_angle + 1.0 // concave bias
        } else {
            sin_angle * 0.5 // convex, lower score
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
        let config = ConfigView {
            fields: HashMap::new(),
        };
        let module = ClassicPerimeters::on_print_start(&config).unwrap();
        assert_eq!(module.wall_count, 2);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }
}
