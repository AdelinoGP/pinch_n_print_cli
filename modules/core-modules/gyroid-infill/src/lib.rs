//! Gyroid TPMS sparse infill generator module.
//!
//! Implements `LayerModule::run_infill` for the `Layer::Infill` stage.
//! Generates Gyroid triply periodic minimal surface wave patterns that vary
//! with layer Z height, producing interlocking waves between layers.
//!
//! Algorithm adapted from OrcaSlicer FillGyroid.cpp:
//! 1. Compute z phase: z_sin = sin(z / scale), z_cos = cos(z / scale)
//! 2. Choose orientation: vertical if |z_sin| <= |z_cos|, else horizontal
//! 3. Adaptively sample one period of the Gyroid curve f(x)
//! 4. Tile wave periods across bounding box
//! 5. Clip waves to infill polygon boundaries
//! 6. Convert to ExtrusionPath3D with SparseInfill role

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2,
    Point3WithWidth,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

use std::f64::consts::{FRAC_PI_2, PI};

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Density adjustment factor matching OrcaSlicer's DensityAdjust constant.
/// This scales the effective density to produce correct fill weight percentage.
// OrcaSlicer: DensityAdjust = 2.44
const DENSITY_ADJUST: f64 = 2.44;

/// Correction angle applied to the infill rotation (degrees).
/// Matches OrcaSlicer CorrectionAngle = -45.
// OrcaSlicer: CorrectionAngle = -45
const CORRECTION_ANGLE_DEG: f64 = -45.0;

/// Tolerance for adaptive curve sampling (in normalized gyroid units).
/// Matches OrcaSlicer PatternTolerance = 0.2.
// OrcaSlicer: PatternTolerance = 0.2
const PATTERN_TOLERANCE: f64 = 0.2;

/// Gyroid TPMS infill generator.
///
/// Produces sinusoidal wave-based fill patterns using the Gyroid minimal surface
/// equation sin(x)cos(y) + sin(y)cos(z) + sin(z)cos(x) = 0.
/// Wave patterns vary with layer Z, producing strong interlocking 3D structure.
pub struct GyroidInfill {
    /// Infill density (0.0 to 1.0).
    density: f32,
    /// Base infill angle in degrees.
    base_angle: f32,
    /// Infill print speed in mm/s.
    infill_speed: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
}

impl GyroidInfill {
    /// Returns the configured infill density.
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Returns the configured line width.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

impl LayerModule for GyroidInfill {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let density = match config.fields.get("infill_density") {
            Some(ConfigValue::Float(d)) => *d as f32,
            _ => 0.2,
        };

        let base_angle = match config.fields.get("infill_angle") {
            Some(ConfigValue::Float(a)) => *a as f32,
            _ => 0.0,
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
            base_angle,
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

impl GyroidInfill {
    /// Generate gyroid wave fill for a single ExPolygon.
    fn fill_expolygon(
        &self,
        expoly: &ExPolygon,
        z: f32,
        speed_factor: f32,
    ) -> Vec<ExtrusionPath3D> {
        // Compute density-adjusted spacing in mm
        let density_adjusted = (self.density as f64) * DENSITY_ADJUST;
        if density_adjusted <= 0.0 {
            return Vec::new();
        }
        let spacing_mm = self.line_width as f64 / density_adjusted;

        // Compute bounding box of the polygon in mm
        let (bb_min_x, bb_min_y, bb_max_x, bb_max_y) = polygon_bbox_mm(expoly);
        let bb_width = bb_max_x - bb_min_x;
        let bb_height = bb_max_y - bb_min_y;

        if bb_width <= 0.0 || bb_height <= 0.0 {
            return Vec::new();
        }

        // Compute rotation angle (base + correction)
        let infill_angle_rad = ((self.base_angle as f64) + CORRECTION_ANGLE_DEG).to_radians();

        // Scale factor: converts from normalized gyroid units to mm
        let scale_factor = spacing_mm;
        if scale_factor <= 0.0 {
            return Vec::new();
        }

        // Normalized Z in gyroid units
        let z_norm = (z as f64) / scale_factor;
        let z_sin = z_norm.sin();
        let z_cos = z_norm.cos();

        // Choose orientation based on z phase
        let vertical = z_sin.abs() <= z_cos.abs();

        // Expand bounding box by a few spacings to prevent edge artifacts
        let expand = 4.0 * spacing_mm;
        let bb_min_x = bb_min_x - expand;
        let bb_min_y = bb_min_y - expand;
        let bb_width = bb_width + 2.0 * expand;
        let bb_height = bb_height + 2.0 * expand;

        // Convert bbox dimensions to gyroid units
        let mut width = bb_width / scale_factor;
        let mut height = bb_height / scale_factor;

        // Set up orientation-dependent bounds
        let (lower_bound, upper_bound, flip_init);
        if vertical {
            flip_init = false;
            lower_bound = -PI;
            upper_bound = width - FRAC_PI_2;
            std::mem::swap(&mut width, &mut height);
        } else {
            flip_init = true;
            lower_bound = 0.0;
            upper_bound = height;
        }

        // Adaptive sample tolerance
        let tolerance = (spacing_mm / 2.0).min(PATTERN_TOLERANCE) / scale_factor;
        let tolerance = tolerance.max(0.01); // prevent infinite refinement

        // Build two period templates: odd and even (phase-shifted)
        let one_period_odd = make_one_period(width, z_cos, z_sin, vertical, flip_init, tolerance);
        let one_period_even = make_one_period(width, z_cos, z_sin, vertical, !flip_init, tolerance);

        // Generate wave polylines
        let mut wave_points_set: Vec<Vec<(f64, f64)>> = Vec::new();

        let mut y0 = lower_bound;
        while y0 < upper_bound + 1e-9 {
            // Odd wave
            let pts = make_wave(
                &one_period_odd,
                width,
                height,
                y0,
                z_cos,
                z_sin,
                vertical,
                flip_init,
            );
            if !pts.is_empty() {
                wave_points_set.push(pts);
            }

            // Even wave (offset by PI)
            y0 += PI;
            if y0 < upper_bound + 1e-9 {
                let pts = make_wave(
                    &one_period_even,
                    width,
                    height,
                    y0,
                    z_cos,
                    z_sin,
                    vertical,
                    !flip_init,
                );
                if !pts.is_empty() {
                    wave_points_set.push(pts);
                }
            }

            y0 += PI;
        }

        // Convert wave polylines from gyroid units to mm, then clip to polygon
        let cos_a = infill_angle_rad.cos();
        let sin_a = infill_angle_rad.sin();

        let mut result = Vec::new();

        for wave_pts in &wave_points_set {
            // Convert to mm coordinates relative to bbox origin, then to absolute mm
            let mm_points: Vec<(f64, f64)> = wave_pts
                .iter()
                .map(|&(gx, gy)| {
                    let mx = gx * scale_factor + bb_min_x;
                    let my = gy * scale_factor + bb_min_y;
                    // Apply rotation around bbox center
                    let cx = bb_min_x + bb_width / 2.0;
                    let cy = bb_min_y + bb_height / 2.0;
                    let dx = mx - cx;
                    let dy = my - cy;
                    let rx = dx * cos_a - dy * sin_a + cx;
                    let ry = dx * sin_a + dy * cos_a + cy;
                    (rx, ry)
                })
                .collect();

            // Clip the polyline to the polygon and collect segments
            let clipped = clip_polyline_to_expolygon(&mm_points, expoly);

            for segment in clipped {
                if segment.len() < 2 {
                    continue;
                }

                let points: Vec<Point3WithWidth> = segment
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
        }

        result
    }
}

/// Evaluate the Gyroid curve y = f(x) at fixed z.
///
/// Inverts the implicit Gyroid surface equation
/// sin(x)cos(y) + sin(y)cos(z) + sin(z)cos(x) = 0
/// to solve for y given x and z.
///
/// Adapted from OrcaSlicer FillGyroid.cpp `f()`.
fn gyroid_f(x: f64, z_sin: f64, z_cos: f64, vertical: bool, flip: bool) -> f64 {
    if vertical {
        let phase_offset = if z_cos < 0.0 { PI } else { 0.0 } + PI;
        let a = (x + phase_offset).sin();
        let b = -z_cos;
        let res = z_sin * (x + phase_offset + if flip { PI } else { 0.0 }).cos();
        let r = (a * a + b * b).sqrt();
        if r < 1e-15 {
            return PI;
        }
        // Clamp to prevent NaN from asin
        let v1 = (a / r).clamp(-1.0, 1.0);
        let v2 = (res / r).clamp(-1.0, 1.0);
        v1.asin() + v2.asin() + PI
    } else {
        let phase_offset = if z_sin < 0.0 { PI } else { 0.0 };
        let a = (x + phase_offset).cos();
        let b = -z_sin;
        let res = z_cos * (x + phase_offset + if flip { 0.0 } else { PI }).sin();
        let r = (a * a + b * b).sqrt();
        if r < 1e-15 {
            return FRAC_PI_2;
        }
        // Clamp to prevent NaN from asin
        let v1 = (a / r).clamp(-1.0, 1.0);
        let v2 = (res / r).clamp(-1.0, 1.0);
        v1.asin() + v2.asin() + FRAC_PI_2
    }
}

/// Adaptively sample one period of the Gyroid curve.
///
/// Seeds with coarse samples at PI/2 intervals, then refines by inserting
/// midpoints wherever the cross-product area exceeds tolerance^2.
///
/// Adapted from OrcaSlicer FillGyroid.cpp `make_one_period()`.
fn make_one_period(
    width: f64,
    z_cos: f64,
    z_sin: f64,
    vertical: bool,
    flip: bool,
    tolerance: f64,
) -> Vec<(f64, f64)> {
    let limit = (2.0 * PI).min(width);
    let dx = FRAC_PI_2;

    let mut points: Vec<(f64, f64)> = Vec::new();

    // Seed with coarse samples
    let mut x = 0.0;
    while x < limit - 1e-9 {
        points.push((x, gyroid_f(x, z_sin, z_cos, vertical, flip)));
        x += dx;
    }
    points.push((limit, gyroid_f(limit, z_sin, z_cos, vertical, flip)));

    // Adaptive refinement
    let tol_sq = tolerance * tolerance;
    loop {
        let size = points.len();
        let mut new_pts: Vec<(f64, f64)> = Vec::new();

        for i in 1..size {
            let (lx, ly) = points[i - 1];
            let (rx, ry) = points[i];
            let mx = lx + (rx - lx) / 2.0;
            let my = gyroid_f(mx, z_sin, z_cos, vertical, flip);

            // Cross-product area test
            let ipx = mx - lx;
            let ipy = my - ly;
            let irx = mx - rx;
            let iry = my - ry;
            let cross = (ipx * iry - ipy * irx).abs();

            if cross > tol_sq {
                new_pts.push((mx, my));
            }
        }

        if new_pts.is_empty() {
            break;
        }

        points.extend(new_pts);
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    }

    points
}

/// Replicate one period template across the full width, apply offset, clamp,
/// and handle vertical axis swap.
///
/// Adapted from OrcaSlicer FillGyroid.cpp `make_wave()`.
#[allow(clippy::too_many_arguments)]
fn make_wave(
    one_period: &[(f64, f64)],
    width: f64,
    height: f64,
    offset: f64,
    z_cos: f64,
    z_sin: f64,
    vertical: bool,
    flip: bool,
) -> Vec<(f64, f64)> {
    if one_period.is_empty() {
        return Vec::new();
    }

    let mut points = one_period.to_vec();
    let period = points.last().unwrap().0;

    if period <= 0.0 {
        return Vec::new();
    }

    if width > period + 1e-9 {
        // Remove last point before tiling (it will be replaced by shifted copies)
        points.pop();
        let n = points.len();
        if n == 0 {
            return Vec::new();
        }

        let mut idx = 0;
        loop {
            let src = points[idx % n];
            let new_x = src.0 + ((idx / n) as f64 + 1.0) * period;
            if new_x >= width - 1e-9 {
                break;
            }
            points.push((new_x, src.1));
            idx += 1;
        }

        // Add final point at exactly width
        points.push((width, gyroid_f(width, z_sin, z_cos, vertical, flip)));
    }

    // Apply offset, clamp, and axis swap
    let mut result: Vec<(f64, f64)> = Vec::with_capacity(points.len());
    for (px, py) in points {
        let mut y = py + offset;
        y = y.clamp(0.0, height);
        if vertical {
            result.push((y, px));
        } else {
            result.push((px, y));
        }
    }

    result
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
    // Must be inside contour
    if !point_in_polygon(x, y, &expoly.contour.points) {
        return false;
    }
    // Must be outside all holes
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

/// Clip a polyline (series of mm points) to an ExPolygon.
///
/// Returns a list of segments (sub-polylines) that are inside the polygon.
fn clip_polyline_to_expolygon(
    points: &[(f64, f64)],
    expoly: &ExPolygon,
) -> Vec<Vec<(f64, f64)>> {
    if points.len() < 2 {
        return Vec::new();
    }

    let mut result: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current_segment: Vec<(f64, f64)> = Vec::new();

    for &(x, y) in points {
        if point_in_expolygon(x, y, expoly) {
            current_segment.push((x, y));
        } else {
            if current_segment.len() >= 2 {
                result.push(std::mem::take(&mut current_segment));
            } else {
                current_segment.clear();
            }
        }
    }

    if current_segment.len() >= 2 {
        result.push(current_segment);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView {
            fields: std::collections::HashMap::new(),
        };
        let module = GyroidInfill::on_print_start(&config).unwrap();
        assert!((module.density - 0.2).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }

    #[test]
    fn gyroid_f_no_nan() {
        // Test the gyroid function at various z values to ensure no NaN
        for z in [0.0_f64, 0.5, 1.0, 1.5707, 3.14159, 6.28, 100.0] {
            let z_sin = z.sin();
            let z_cos = z.cos();
            for x in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0] {
                let y_h = gyroid_f(x, z_sin, z_cos, false, false);
                let y_v = gyroid_f(x, z_sin, z_cos, true, false);
                assert!(!y_h.is_nan(), "NaN at x={}, z={} (horizontal)", x, z);
                assert!(!y_v.is_nan(), "NaN at x={}, z={} (vertical)", x, z);
            }
        }
    }

    #[test]
    fn make_one_period_produces_points() {
        let z = 1.0_f64;
        let pts = make_one_period(10.0, z.cos(), z.sin(), false, true, 0.2);
        assert!(pts.len() >= 5, "should produce at least seed points");
        // Should be sorted by x
        for i in 1..pts.len() {
            assert!(pts[i].0 >= pts[i - 1].0, "points should be x-sorted");
        }
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
}
