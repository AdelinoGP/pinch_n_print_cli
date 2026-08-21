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
//!
//! # Planned-polygon renderer
//!
//! This module is a narrow polygon scan-fill adapter. It consumes validated
//! structural plan entries for the `traditional` support family via the
//! anchored support events (`PaintRegionLayerView::support_plan_entries_for`)
//! and scan-fills only the planned body/interface polygons into attributed
//! `SupportIR`. It never reads `region.polygons()` or derives support
//! eligibility independently; eligibility is resolved upstream by the
//! `traditional-support-planner`.
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
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

mod interface_regularize;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Default gap between adjacent support-interface extrusions, matching
/// OrcaSlicer's `support_interface_spacing` default of 0.4 mm.
const DEFAULT_INTERFACE_SPACING_MM: f32 = 0.4;

/// Fallback layer height (mm) used only when the region view reports a
/// non-positive `effective_layer_height`. Interface pitch degenerates to the
/// configured gap in that case, which is the pre-flow-term behaviour.
const FALLBACK_LAYER_HEIGHT_MM: f32 = 0.0;

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
    /// Configured top-interface line gap in millimeters (canonical
    /// `support_interface_spacing`). This is the *gap*, not the pitch: the
    /// printed pitch adds the interface flow spacing (see
    /// `interface_pitch_mm`).
    top_interface_spacing_mm: f32,
    /// Configured bottom-interface line gap in millimeters (canonical
    /// `support_bottom_interface_spacing`). Negative mirrors the top value,
    /// matching OrcaSlicer's `-1 == same as top` convention for the paired
    /// interface keys.
    bottom_interface_spacing_mm: f32,
    /// Canonical `support_params.support_style != smsGrid` — whether interface
    /// regularization runs the `closing` + `smooth_outward` branch of
    /// `generate_interface_layers`' `regularize` lambda, or the plain
    /// `union_safety_offset` branch.
    ///
    /// Resolved from `support_style` exactly as canonical `SupportParameters`
    /// resolves it for a **non-tree** `support_type`: any tree style is invalid
    /// here and falls back to `smsDefault`, and `smsDefault` for non-tree is
    /// `smsGrid`. So only an explicit `snug` smooths.
    smooth_supports: bool,
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

        let top_interface_spacing_mm = match config.get("support_interface_spacing") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => DEFAULT_INTERFACE_SPACING_MM,
        };

        let bottom_interface_spacing_mm = match config.get("support_bottom_interface_spacing") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => -1.0,
        };

        // Canonical `SupportParameters` resolves `support_style` against
        // `support_type` first: a tree style on a non-tree object degrades to
        // `smsDefault`, and `smsDefault` for a non-tree object is `smsGrid`.
        // `smooth_supports` is then `support_style != smsGrid`, so within the
        // traditional family only an explicit `snug` regularizes.
        let smooth_supports = match config.get("support_style") {
            Some(ConfigValue::String(s)) => s.eq_ignore_ascii_case("snug"),
            _ => false,
        };

        Ok(Self {
            enabled,
            density,
            base_angle,
            support_speed,
            line_width,
            top_interface_spacing_mm,
            bottom_interface_spacing_mm,
            smooth_supports,
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
            let z = region.z();
            // Canonical `SupportParameters` derives the interface *pitch* as
            // `support_interface_spacing + interface_flow.spacing()`; the
            // config key is the gap between adjacent extrusions, not the
            // centre-to-centre distance. Using the key directly as the pitch
            // over-extruded every interface layer by roughly the ratio of the
            // two (0.4 vs 0.757 mm at a 0.4 mm width / 0.2 mm layer).
            let (interface_flow_spacing_mm, top_interface_pitch_mm, bottom_interface_pitch_mm) =
                self.interface_pitch_mm(region.effective_layer_height());
            let top_interface_line_spacing = slicer_ir::mm_to_units(top_interface_pitch_mm);
            let bottom_interface_line_spacing = slicer_ir::mm_to_units(bottom_interface_pitch_mm);

            // Structural support plans carry semantic regions, not printable
            // paths. A missing entry means this demand was declined; do not
            // resurrect it with a legacy filler.
            let planned_entries =
                paint.support_plan_entries_for(region.object_id().as_str(), *region.region_id());

            if planned_entries.is_empty() {
                continue;
            }

            for entry in planned_entries.iter().filter(|entry| {
                entry.global_layer_index == layer_index as i32 && entry.decline_reason.is_none()
            }) {
                if entry.family_id != "traditional" {
                    return Err(ModuleError::non_fatal(
                        333,
                        format!(
                            "traditional support family-attribution mismatch: {}",
                            entry.family_id
                        ),
                    ));
                }
                if !entry.roles.iter().any(|role| !role.regions.is_empty()) {
                    return Err(ModuleError::non_fatal(
                        334,
                        "traditional support plan-required: no planned polygon",
                    ));
                }
                output.begin_region(region.object_id(), *region.region_id());
                // F-37: canonical `generate_interface_layers` regularizes every
                // interface band (`closing` + `smooth_outward`) and subtracts
                // the result from the base area before anything is filled.
                // `None` means the entry carries no interface role, so the
                // planner's partition is rendered verbatim.
                let regularized = interface_regularize::regularize_entry_roles(
                    &entry.roles,
                    interface_flow_spacing_mm,
                    top_interface_pitch_mm,
                    bottom_interface_pitch_mm,
                    self.smooth_supports,
                );
                let rendered: Vec<(slicer_ir::SupportPlanRole, Vec<ExPolygon>)> =
                    regularized.unwrap_or_else(|| {
                        entry
                            .roles
                            .iter()
                            .map(|r| (r.role, r.regions.clone()))
                            .collect()
                    });
                for (role, regions) in rendered.iter() {
                    let role = *role;
                    let spacing = match role {
                        slicer_ir::SupportPlanRole::SupportBody => line_spacing,
                        slicer_ir::SupportPlanRole::TopInterface => top_interface_line_spacing,
                        slicer_ir::SupportPlanRole::BottomInterface => bottom_interface_line_spacing,
                        slicer_ir::SupportPlanRole::RaftRelated => line_spacing,
                    };
                    for expoly in regions.iter() {
                        let interface = matches!(
                            role,
                            slicer_ir::SupportPlanRole::TopInterface
                                | slicer_ir::SupportPlanRole::BottomInterface
                        );
                        let paths = self.fill_expolygon(
                            expoly,
                            spacing,
                            cos_a,
                            sin_a,
                            z,
                            speed_factor,
                            interface,
                        );
                        for mut path in paths {
                            match role {
                                slicer_ir::SupportPlanRole::SupportBody => {
                                    let _ = output.push_support_path(path);
                                }
                                // The extrusion role must be stamped here, not
                                // left as `SupportMaterial`: `;TYPE:Support
                                // interface` and `support_interface_speed` are
                                // both selected from `ExtrusionRole` in
                                // `crates/slicer-gcode/src/emit.rs`, so an
                                // interface path that keeps the body role is
                                // emitted and fed as plain support.
                                slicer_ir::SupportPlanRole::TopInterface => {
                                    path.role = ExtrusionRole::SupportInterface;
                                    let _ = output.push_interface_path(path, true);
                                }
                                slicer_ir::SupportPlanRole::BottomInterface => {
                                    path.role = ExtrusionRole::SupportInterface;
                                    let _ = output.push_interface_path(path, false);
                                }
                                slicer_ir::SupportPlanRole::RaftRelated => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl TraditionalSupport {
    /// Interface scan-fill pitch, in scaled units, for the top and bottom
    /// interface roles at a given layer height.
    ///
    /// Canonical `SupportParameters` (`SupportParameters.hpp`):
    /// `interface_spacing = support_interface_spacing + interface_flow.spacing()`.
    /// `spacing()` is `Flow::rounded_rectangle_extrusion_spacing`, exposed
    /// in-tree as `slicer_core::flow::line_width_to_spacing`. A width/layer
    /// height pair that yields a non-positive spacing falls back to the bare
    /// configured gap rather than failing the layer.
    /// `(interface_flow_spacing_mm, top_pitch_mm, bottom_pitch_mm)`.
    ///
    /// The bare flow spacing is exposed alongside the pitches because canonical
    /// `generate_interface_layers` derives both its smoothing/closing distance
    /// (`scaled_spacing() * 1.5`) and its minimum island radii
    /// (`scaled_spacing() / interface_density`) from it.
    fn interface_pitch_mm(&self, layer_height_mm: f32) -> (f32, f32, f32) {
        let layer_height = if layer_height_mm > 0.0 {
            layer_height_mm
        } else {
            FALLBACK_LAYER_HEIGHT_MM
        };
        let flow_spacing =
            slicer_core::flow::line_width_to_spacing(self.line_width, layer_height).unwrap_or(0.0);
        let top_gap = self.top_interface_spacing_mm.max(0.0);
        // Negative mirrors the top gap, per the `-1 == same as top` convention
        // OrcaSlicer uses for the paired bottom-interface keys.
        let bottom_gap = if self.bottom_interface_spacing_mm < 0.0 {
            top_gap
        } else {
            self.bottom_interface_spacing_mm
        };
        (
            flow_spacing,
            top_gap + flow_spacing,
            bottom_gap + flow_spacing,
        )
    }

    /// Generate fill lines for a single ExPolygon.
    fn fill_expolygon(
        &self,
        expoly: &ExPolygon,
        line_spacing: i64,
        cos_a: f64,
        sin_a: f64,
        z: f32,
        speed_factor: f32,
        interface: bool,
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
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                };
                let end = Point3WithWidth {
                    x: slicer_ir::units_to_mm(end_x),
                    y: slicer_ir::units_to_mm(end_y),
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
                            dist_to_top_mm: 0.0,
                            overhang_distance_mm: None,
                        },
                        Point3WithWidth {
                            x: slicer_ir::units_to_mm(end_x),
                            y: slicer_ir::units_to_mm(end_y),
                            z,
                            width: self.line_width,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                            dist_to_top_mm: 0.0,
                            overhang_distance_mm: None,
                        },
                    ],
                    role: ExtrusionRole::SupportMaterial,
                    speed_factor,
                });
                i += 2;
            }
        }

        // Contact polygons can have no scan-line span. Keep the
        // contact-inclusive interface layer printable without restoring body
        // geometry that was carved out by the planner.
        if interface && paths.is_empty() && expoly.contour.points.len() >= 2 {
            let mut points = expoly
                .contour
                .points
                .iter()
                .map(|point| Point3WithWidth {
                    x: slicer_ir::units_to_mm(point.x),
                    y: slicer_ir::units_to_mm(point.y),
                    z,
                    width: self.line_width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                })
                .collect::<Vec<_>>();
            points.push(points[0]);
            paths.push(ExtrusionPath3D {
                points,
                role: ExtrusionRole::SupportMaterial,
                speed_factor,
            });
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
        assert!((module.top_interface_spacing_mm - 0.4).abs() < 0.001);
        assert!(module.bottom_interface_spacing_mm < 0.0);
    }

    /// F-7: the interface pitch is the configured gap **plus** the interface
    /// flow spacing, not the configured gap alone. Measured against the
    /// authoritative OrcaSlicer reference: 0.4 mm gap at a 0.4 mm width and
    /// 0.2 mm layer height prints a 0.757 mm X pitch.
    #[test]
    fn interface_pitch_adds_flow_spacing() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = TraditionalSupport::from_config(&config).unwrap();
        let (_, top_mm, bottom_mm) = module.interface_pitch_mm(0.2);
        let (top, bottom) = (slicer_ir::mm_to_units(top_mm), slicer_ir::mm_to_units(bottom_mm));
        let expected = slicer_ir::mm_to_units(0.4 + (0.4 - 0.2 * (1.0 - core::f32::consts::PI / 4.0)));
        assert_eq!(top, expected, "top interface pitch must add flow spacing");
        assert_eq!(bottom, top, "negative bottom spacing mirrors the top gap");
        assert!(
            (slicer_ir::units_to_mm(top) - 0.757).abs() < 0.002,
            "measured Orca pitch is 0.757 mm, got {}",
            slicer_ir::units_to_mm(top)
        );
    }
}
