// -----------------------------------------------------------------------------
// Ported from OrcaSlicer (AGPLv3). This file is an LLM-generated Rust port
// of the rectilinear scan-line discipline in
// OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp
// (fill_surface_by_lines / slice_region_by_vertical_lines) and
// FillBase.cpp (infill_direction, adjust_solid_spacing).
// -----------------------------------------------------------------------------
//
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Fill/FillRectilinear.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
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
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
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
    /// Per-layer scan-line shift step (mm). Alternates sign each layer
    /// to interleave, not stack. Default 0.0 (no shift).
    infill_shift_step: f32,
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

        let infill_shift_step = match config.get("infill_shift_step") {
            Some(ConfigValue::Float(s)) => *s as f32,
            _ => 0.0,
        };

        Ok(Self {
            density,
            base_angle,
            infill_speed,
            line_width,
            infill_shift_step,
        })
    }

    fn run_infill(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if self.density <= 0.0 {
            return Ok(());
        }

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

        // Per-layer pattern shift: alternates sign each layer so scan lines
        // interleave rather than stack. OrcaSlicer's raw `pattern_shift` is
        // always 0 for plain rectilinear; the user-facing per-layer shift is
        // `infill_shift_step` applied here.
        let x_shift_units = slicer_ir::mm_to_units(self.infill_shift_step)
            * if layer_index.is_multiple_of(2) { 1 } else { -1 };

        // Per-role per-polygon emit (Q3 + Q5 partition contract): the host
        // pre-partitions every region's wall-inset into four pairwise-disjoint
        // canonical fill polygons (`sparse_infill_area`, `top_solid_fill`,
        // `bottom_solid_fill`, `bridge_areas`) with precedence
        // bridge > bottom > top > sparse. Each role emits over its own
        // polygon — zero polygon math, zero per-region role-pick. Per-region
        // `infill_density` / `line_width` overrides (packet 131 / TASK-256)
        // are read through `slicer_sdk::config_resolution` and forwarded to
        // each `scan_expolygon` call below.
        // See `crates/slicer-runtime/src/region_partition.rs`.
        for region in regions {
            output.begin_region(region.object_id(), *region.region_id());
            let z = region.z();
            let std_cos_a = cos_a;
            let std_sin_a = sin_a;

            // Per-region config resolution (packet 131 / TASK-256):
            // fall back to module-global defaults when the per-region view
            // is absent or the key is not declared.
            let region_density = slicer_sdk::config_resolution::resolve_float(
                region,
                "infill_density",
                self.density,
            );
            let region_line_width =
                slicer_sdk::config_resolution::resolve_float(region, "line_width", self.line_width);
            if region_density <= 0.0 {
                continue;
            }
            let line_spacing = slicer_ir::mm_to_units(region_line_width / region_density);

            // SparseInfill over the host-partitioned sparse-only polygon.
            let sparse = region.sparse_infill_area();
            if !sparse.is_empty() && region.should_emit(ExtrusionRole::SparseInfill) {
                for expoly in sparse {
                    let paths = scan_expolygon(
                        expoly,
                        line_spacing,
                        std_cos_a,
                        std_sin_a,
                        z,
                        speed_factor,
                        &ExtrusionRole::SparseInfill,
                        region_line_width,
                        false,
                        x_shift_units,
                    );
                    for path in paths {
                        let _ = output.push_sparse_path(path);
                    }
                }
            }

            // Top solid fill. Depth-0 (exposed) is the Top surface; deeper shell
            // layers (index ≥ 1) are Internal solid infill. Gating stays on the
            // top-fill claim. See handoff G4 / OrcaSlicer stTop vs stInternalSolid.
            let top = region.top_solid_fill();
            if !top.is_empty() && region.should_emit(ExtrusionRole::TopSolidInfill) {
                let role = solid_fill_role(region.top_shell_index(), ExtrusionRole::TopSolidInfill);
                for expoly in top {
                    let paths = scan_expolygon(
                        expoly,
                        line_spacing,
                        std_cos_a,
                        std_sin_a,
                        z,
                        speed_factor,
                        &role,
                        region_line_width,
                        true,
                        x_shift_units,
                    );
                    for path in paths {
                        let _ = output.push_solid_path(path);
                    }
                }
            }

            // Bottom solid fill. Depth-0 (exposed) is the Bottom surface; deeper
            // shell layers are Internal solid infill.
            let bottom = region.bottom_solid_fill();
            if !bottom.is_empty() && region.should_emit(ExtrusionRole::BottomSolidInfill) {
                let role = solid_fill_role(
                    region.bottom_shell_index(),
                    ExtrusionRole::BottomSolidInfill,
                );
                for expoly in bottom {
                    let paths = scan_expolygon(
                        expoly,
                        line_spacing,
                        std_cos_a,
                        std_sin_a,
                        z,
                        speed_factor,
                        &role,
                        region_line_width,
                        true,
                        x_shift_units,
                    );
                    for path in paths {
                        let _ = output.push_solid_path(path);
                    }
                }
            }

            // BridgeInfill over bridge_areas at the region's bridge orientation.
            let bridge = region.bridge_areas();
            if !bridge.is_empty() && region.should_emit(ExtrusionRole::BridgeInfill) {
                let deg = region.bridge_orientation_deg() as f64;
                let rad = deg.to_radians();
                let (bridge_cos_a, bridge_sin_a) = (rad.cos(), rad.sin());
                for expoly in bridge {
                    let paths = scan_expolygon(
                        expoly,
                        line_spacing,
                        bridge_cos_a,
                        bridge_sin_a,
                        z,
                        speed_factor,
                        &ExtrusionRole::BridgeInfill,
                        region_line_width,
                        false,
                        x_shift_units,
                    );
                    for path in paths {
                        let _ = output.push_solid_path(path);
                    }
                }
            }
        }

        let _ = angle_rad; // angle_rad retained for fixture readability; no longer used
        Ok(())
    }
}

/// Maps a top/bottom shell depth index to the emitted extrusion role.
///
/// Depth 0 is the exposed surface (keeps `exposed` — Top/BottomSolidInfill);
/// any deeper shell layer (index ≥ 1) is `InternalSolidInfill`. A `None` index
/// (fill present without a recorded depth) is treated as the exposed surface to
/// preserve legacy behaviour.
fn solid_fill_role(shell_index: Option<u8>, exposed: ExtrusionRole) -> ExtrusionRole {
    match shell_index {
        Some(0) | None => exposed,
        Some(_) => ExtrusionRole::InternalSolidInfill,
    }
}

/// Adjust solid infill line spacing so that the polygon width is divided
/// evenly, producing uniform scan lines that exactly span the polygon.
///
/// Ported from OrcaSlicer FillBase.cpp::adjust_solid_spacing.
fn adjust_solid_spacing(width: i64, distance: i64) -> i64 {
    let count = width / distance;
    if count < 1 {
        return distance;
    }
    let new_distance = ((width as f64) / (count as f64)).round() as i64;
    if (new_distance as f64) > (distance as f64) * 1.2 {
        return distance;
    }
    new_distance
}

/// Scan a single ExPolygon and produce fill segments.
///
/// Each ExPolygon is scanned independently using its own bounding-box center
/// as the reference point (AC-3 invariant). The half-open vertex test
/// (include at min_y, exclude at max_y) prevents double-counting at
/// polygon vertices (AC-N1).
///
/// When `adjust_for_solid` is true, the line spacing is adjusted via
/// `adjust_solid_spacing` so that the polygon is divided evenly.
#[allow(clippy::too_many_arguments)]
fn scan_expolygon(
    expoly: &ExPolygon,
    line_spacing: i64,
    cos_a: f64,
    sin_a: f64,
    z: f32,
    speed_factor: f32,
    role: &ExtrusionRole,
    line_width: f32,
    adjust_for_solid: bool,
    x_shift: i64,
) -> Vec<ExtrusionPath3D> {
    if line_spacing <= 0 {
        return Vec::new();
    }

    // Collect edges from contour and holes. Inlined per the packet 134 design
    // (replaces the previous `collect_edges` free function).
    let mut edges: Vec<(i64, i64, i64, i64)> = Vec::new();
    let mut contour_edges: Vec<(i64, i64, i64, i64)> = Vec::new();
    let contour_pts = &expoly.contour.points;
    let n = contour_pts.len();
    if n >= 2 {
        for i in 0..n {
            let j = (i + 1) % n;
            let p_i = &contour_pts[i];
            let p_j = &contour_pts[j];
            edges.push((p_i.x, p_i.y, p_j.x, p_j.y));
            contour_edges.push((p_i.x, p_i.y, p_j.x, p_j.y));
        }
    }
    for hole in &expoly.holes {
        let pts = &hole.points;
        let m = pts.len();
        if m >= 2 {
            for i in 0..m {
                let j = (i + 1) % m;
                let p_i = &pts[i];
                let p_j = &pts[j];
                edges.push((p_i.x, p_i.y, p_j.x, p_j.y));
            }
        }
    }
    if edges.is_empty() {
        return Vec::new();
    }

    // Compute bbox center of this expolygon in working (unrotated) space.
    let (mut min_x, mut max_x) = (i64::MAX, i64::MIN);
    let (mut min_y, mut max_y) = (i64::MAX, i64::MIN);
    for &(x1, y1, x2, y2) in &edges {
        min_x = min_x.min(x1).min(x2);
        max_x = max_x.max(x1).max(x2);
        min_y = min_y.min(y1).min(y2);
        max_y = max_y.max(y1).max(y2);
    }
    if min_x >= max_x || min_y >= max_y {
        return Vec::new();
    }
    let refpt_x = min_x + (max_x - min_x) / 2;
    let refpt_y = min_y + (max_y - min_y) / 2;

    // Translate to refpt-centered, then rotate by -angle.
    let cos_neg = cos_a;
    let sin_neg = -sin_a;
    let mut rotated_edges: Vec<(i64, i64, i64, i64)> = Vec::with_capacity(edges.len());
    let mut rotated_contour: Vec<(i64, i64, i64, i64)> = Vec::with_capacity(contour_edges.len());
    for &(x1, y1, x2, y2) in &edges {
        let (rx1, ry1) = rotate_point(x1 - refpt_x, y1 - refpt_y, cos_neg, sin_neg);
        let (rx2, ry2) = rotate_point(x2 - refpt_x, y2 - refpt_y, cos_neg, sin_neg);
        rotated_edges.push((rx1, ry1, rx2, ry2));
    }
    for &(x1, y1, x2, y2) in &contour_edges {
        let (rx1, ry1) = rotate_point(x1 - refpt_x, y1 - refpt_y, cos_neg, sin_neg);
        let (rx2, ry2) = rotate_point(x2 - refpt_x, y2 - refpt_y, cos_neg, sin_neg);
        rotated_contour.push((rx1, ry1, rx2, ry2));
    }

    // Bbox in rotated space.
    let (mut rmin_y, mut rmax_y) = (i64::MAX, i64::MIN);
    for &(_, ry1, _, ry2) in &rotated_edges {
        rmin_y = rmin_y.min(ry1).min(ry2);
        rmax_y = rmax_y.max(ry1).max(ry2);
    }
    if rmin_y >= rmax_y {
        return Vec::new();
    }

    // For solid roles, adjust spacing so the polygon is divided evenly.
    let effective_spacing = if adjust_for_solid {
        adjust_solid_spacing(rmax_y - rmin_y, line_spacing)
    } else {
        line_spacing
    };

    // Skip both the main scan-line loop and the post-pass when the polygon
    // is too small for the line spacing.
    if rmax_y - rmin_y < effective_spacing {
        return Vec::new();
    }

    let mut paths = Vec::new();
    let mut scan_y = rmin_y;

    while scan_y <= rmax_y {
        let mut x_intersections: Vec<i64> = Vec::new();

        for &(rx1, ry1, rx2, ry2) in &rotated_edges {
            // Skip horizontal edges.
            if ry1 == ry2 {
                continue;
            }
            let (lo, hi) = if ry1 < ry2 { (ry1, ry2) } else { (ry2, ry1) };
            // Half-open: include at min_y, exclude at max_y.
            if scan_y < lo || scan_y >= hi {
                continue;
            }
            let t = (scan_y - ry1) as f64 / (ry2 - ry1) as f64;
            let x = rx1 as f64 + t * (rx2 - rx1) as f64;
            x_intersections.push(x.round() as i64);
        }

        x_intersections.sort();

        let mut i = 0;
        while i + 1 < x_intersections.len() {
            let x_start = x_intersections[i];
            let x_end = x_intersections[i + 1];

            // Skip degenerate zero-length segments.
            if x_start == x_end {
                i += 2;
                continue;
            }

            // Rotate back by +angle about refpt. The x_shift is applied
            // here (in world space) so that the output endpoints shift
            // by `x_shift` units, matching OrcaSlicer's `pattern_shift`
            // semantics (FillRectilinear.cpp:3023-3024).
            let (sx, sy) = rotate_point(x_start, scan_y, cos_a, sin_a);
            let (ex, ey) = rotate_point(x_end, scan_y, cos_a, sin_a);

            let start = Point3WithWidth {
                x: slicer_ir::units_to_mm(sx + refpt_x + x_shift),
                y: slicer_ir::units_to_mm(sy + refpt_y),
                z,
                width: line_width,
                flow_factor: 1.0,
                overhang_quartile: None,
            };
            let end = Point3WithWidth {
                x: slicer_ir::units_to_mm(ex + refpt_x + x_shift),
                y: slicer_ir::units_to_mm(ey + refpt_y),
                z,
                width: line_width,
                flow_factor: 1.0,
                overhang_quartile: None,
            };

            paths.push(ExtrusionPath3D {
                points: vec![start, end],
                role: role.clone(),
                speed_factor,
            });

            i += 2;
        }

        scan_y += effective_spacing;
    }

    // Post-pass: emit horizontal contour edges at the top boundary (rmax_y).
    // The half-open vertex test excludes the top boundary from scan lines, so
    // we add it here to ensure the top edge of the polygon is filled.
    for &(rx1, ry1, rx2, ry2) in &rotated_contour {
        if ry1 == ry2 && ry1 == rmax_y {
            let (sx, sy) = rotate_point(rx1, ry1, cos_a, sin_a);
            let (ex, ey) = rotate_point(rx2, ry2, cos_a, sin_a);
            let start = Point3WithWidth {
                x: slicer_ir::units_to_mm(sx + refpt_x + x_shift),
                y: slicer_ir::units_to_mm(sy + refpt_y),
                z,
                width: line_width,
                flow_factor: 1.0,
                overhang_quartile: None,
            };
            let end = Point3WithWidth {
                x: slicer_ir::units_to_mm(ex + refpt_x + x_shift),
                y: slicer_ir::units_to_mm(ey + refpt_y),
                z,
                width: line_width,
                flow_factor: 1.0,
                overhang_quartile: None,
            };
            paths.push(ExtrusionPath3D {
                points: vec![start, end],
                role: role.clone(),
                speed_factor,
            });
        }
    }

    paths
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
        let module = RectilinearInfill::on_print_start(&config).unwrap();
        assert!((module.density - 0.2).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }
}
