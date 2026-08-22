// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Support/TreeSupport.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Tree-support **renderer**: turns `SupportPlanIR` regions into extrusions
//!
//! Implements `LayerModule::run_support` for the `Layer::Support` stage as the
//! renderer half of the tree family's planner/renderer pair. All branch
//! geometry — contact sampling, avoidance, the MST propagation and the radius
//! taper — is decided upstream by `tree-support-planner`
//! (`modules/core-modules/tree-support-planner/src/lib.rs`, the port of
//! canonical `TreeSupport::drop_nodes`). This module owns no tree topology of
//! its own; the standalone grid-MST filler it used to carry was deleted in
//! packet 224 because it double-extruded every body polygon that
//! `render_polygon` already covered.
//!
//! Algorithm (per region, per layer):
//! 1. Read the planner's `SupportPlanEntry` records via
//!    `PaintRegionLayerView::support_plan_entries_for`, honouring paint
//!    overrides through `SupportPaintPolicy`.
//! 2. For each planned role region (`SupportBody` / `TopInterface` /
//!    `BottomInterface`), render the polygon with `render_polygon`: `wall_count`
//!    inset perimeter passes plus a scan fill inset clear of them.
//! 3. Pitch the fill from `support_density` for bodies, and from the canonical
//!    `support_interface_spacing` / `support_bottom_interface_spacing` pair for
//!    interfaces (see `interface_pitch_mm`).
//! 4. Stamp `ExtrusionRole::SupportInterface` on interface paths so
//!    `crates/slicer-gcode/src/emit.rs` selects `;TYPE:Support interface` and
//!    `support_interface_speed`.
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
use slicer_sdk::host::{self, OffsetJoinType};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView, SupportPaintPolicy};
use slicer_sdk::views::SliceRegionView;

mod interface_regularize;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Default gap between adjacent support-interface extrusions, matching
/// OrcaSlicer's `support_interface_spacing` default of 0.4 mm.
const DEFAULT_INTERFACE_SPACING_MM: f32 = 0.4;

/// Tree-support renderer.
///
/// Renders the `SupportPlanIR` regions produced by `tree-support-planner` as
/// inset perimeter passes plus a scan fill. The branching topology is the
/// planner's; this type only decides walls, pitch, role and speed.
pub struct TreeSupport {
    /// Whether support generation is enabled.
    enabled: bool,
    /// Support density (0.0 to 1.0).
    density: f32,
    /// Support print speed in mm/s.
    support_speed: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
    /// Number of perimeter passes used to represent a support body.
    wall_count: usize,
    /// Configured top-interface line gap in millimeters (canonical
    /// `support_interface_spacing`). This is the *gap*, not the pitch.
    top_interface_spacing_mm: f32,
    /// Configured bottom-interface line gap in millimeters (canonical
    /// `support_bottom_interface_spacing`). Negative mirrors the top value.
    bottom_interface_spacing_mm: f32,
}

impl TreeSupport {
    /// Interface scan-fill pitch in millimetres for the top and bottom
    /// interface roles at a given layer height.
    ///
    /// Canonical `SupportParameters` (`SupportParameters.hpp`):
    /// `interface_spacing = support_interface_spacing + interface_flow.spacing()`,
    /// where `spacing()` is `Flow::rounded_rectangle_extrusion_spacing`
    /// (in-tree: `slicer_core::flow::line_width_to_spacing`).
    /// Returns `(interface_flow_spacing_mm, top_pitch_mm, bottom_pitch_mm)`.
    /// The bare flow spacing is exposed because canonical
    /// `generate_interface_layers` derives its smoothing/closing distance
    /// (`scaled_spacing() * 1.5`) and its minimum island radii
    /// (`scaled_spacing() / interface_density`) from it.
    fn interface_pitch_mm(&self, layer_height_mm: f32) -> (f32, f32, f32) {
        let layer_height = layer_height_mm.max(0.0);
        let flow_spacing =
            slicer_core::flow::line_width_to_spacing(self.line_width, layer_height).unwrap_or(0.0);
        let top_gap = self.top_interface_spacing_mm.max(0.0);
        // Negative mirrors the top gap, per OrcaSlicer's `-1 == same as top`
        // convention for the paired bottom-interface keys.
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

    /// Returns whether support is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the configured support density.
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Returns the configured line width.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

#[slicer_module]
impl LayerModule for TreeSupport {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("enable_support") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
        };

        let density = match config.get("support_density") {
            // The manifest and user-facing config express density as a percent.
            Some(ConfigValue::Float(d)) => (*d as f32) / 100.0,
            _ => 0.2,
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
        let wall_count = match config.get("tree_support_wall_count") {
            Some(ConfigValue::Int(value)) => (*value).max(1) as usize,
            Some(ConfigValue::Float(value)) => (*value).max(1.0) as usize,
            _ => 2,
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

        Ok(Self {
            enabled,
            density,
            support_speed,
            line_width,
            wall_count,
            top_interface_spacing_mm,
            bottom_interface_spacing_mm,
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

        let speed_factor = self.support_speed / BASE_SPEED;
        for region in regions {
            let z = region.z();
            // F-7: interface roles are pitched by canonical
            // `SupportParameters::interface_spacing`
            // (`support_interface_spacing + interface_flow.spacing()`), not by
            // `support_density`. The tree renderer previously had no interface
            // spacing at all and scan-filled roofs and floors at the body
            // pitch.
            let (interface_flow_spacing_mm, top_interface_spacing, bottom_interface_spacing) =
                self.interface_pitch_mm(region.effective_layer_height());

            // Structural support plans carry semantic regions, not printable
            // paths. A missing entry means this demand was declined; do not
            // resurrect it with the legacy grid-MST filler.
            let planned_entries =
                paint.support_plan_entries_for(region.object_id().as_str(), *region.region_id());

            if planned_entries.is_empty() {
                continue;
            }

            for entry in planned_entries.iter().filter(|entry| {
                entry.global_layer_index == layer_index as i32 && entry.decline_reason.is_none()
            }) {
                if entry.family_id != "tree" {
                    return Err(ModuleError::non_fatal(
                        332,
                        format!(
                            "tree support family-attribution mismatch: {}",
                            entry.family_id
                        ),
                    ));
                }
                output.begin_region(region.object_id(), *region.region_id());
                // F-37: canonical `generate_interface_layers` regularizes every
                // interface band (`closing` + `smooth_outward`) and subtracts
                // the result from the base area before anything is filled.
                // Tree/organic styles always take the smoothing branch:
                // canonical `SupportParameters` resolves every style valid for
                // a tree `support_type` to a tree style (never `smsGrid`), and
                // `smooth_supports` is `support_style != smsGrid`.
                // `None` means the entry carries no interface role, so the
                // planner's partition is rendered verbatim.
                let regularized = interface_regularize::regularize_entry_roles(
                    &entry.roles,
                    interface_flow_spacing_mm,
                    top_interface_spacing,
                    bottom_interface_spacing,
                    true,
                );
                let rendered: Vec<(slicer_ir::SupportPlanRole, Vec<ExPolygon>)> = regularized
                    .unwrap_or_else(|| {
                        entry
                            .roles
                            .iter()
                            .map(|r| (r.role, r.regions.clone()))
                            .collect()
                    });
                for (role, role_regions) in rendered.iter() {
                    let role = *role;
                    for expoly in role_regions.iter() {
                        match paint.paint_policy_for(expoly) {
                            // Painted "no support here" still overrides the plan.
                            SupportPaintPolicy::Blocked => continue,
                            // Painted "support here", and the default case, both
                            // render what the planner planned.
                            //
                            // `DefaultEligible` previously additionally required
                            // `region.needs_support()`. That re-litigated the
                            // plan at render time: a `SupportPlanIR` entry *is*
                            // the determination that support is needed, made by
                            // the planner from `PrePass::SupportAnalysis`
                            // contacts. When the flag disagreed, every planned
                            // polygon was skipped silently — no paths, no
                            // diagnostic — so the tree family emitted a full
                            // 126-entry plan and no `;TYPE:Support` at all.
                            // `traditional-support` never had this gate, so the
                            // two families also disagreed on what a plan means.
                            SupportPaintPolicy::Enforced | SupportPaintPolicy::DefaultEligible => {}
                        }

                        // `render_polygon` already covers the whole region
                        // with inset walls plus a density-pitched fill. The
                        // grid-MST `fill_expolygon_tree` used to be appended
                        // here for `SupportBody` on top of that, so every body
                        // polygon was extruded twice over the same area.
                        let fill_spacing = match role {
                            slicer_ir::SupportPlanRole::TopInterface => Some(top_interface_spacing),
                            slicer_ir::SupportPlanRole::BottomInterface => {
                                Some(bottom_interface_spacing)
                            }
                            _ => None,
                        };
                        let paths = self.render_polygon(expoly, z, speed_factor, fill_spacing);
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

// SupportPaintPolicy was moved to `slicer_sdk::traits::SupportPaintPolicy`
// (packet 95 closure) so that tree-support and traditional-support both consume
// the same query implementation through `PaintRegionLayerView::paint_policy_for`.

impl TreeSupport {
    /// Render a semantic support polygon as inset perimeter passes plus scan-fill.
    ///
    /// Each wall is inset half a line width past the previous one and the fill
    /// region is inset clear of all of them, so the passes do not overlap.
    /// Before packet 224 this emitted `wall_count` copies of the *same* contour
    /// (coincident, no inset) and then scan-filled the full polygon at a
    /// `line_width` pitch — 100% density regardless of `support_density` — so a
    /// support body was extruded several times over.
    ///
    /// `fill_spacing_override_mm` supplies the canonical interface pitch for
    /// interface roles; `None` keeps the density-derived body pitch.
    fn render_polygon(
        &self,
        expoly: &ExPolygon,
        z: f32,
        speed_factor: f32,
        fill_spacing_override_mm: Option<f32>,
    ) -> Vec<ExtrusionPath3D> {
        let mut paths = Vec::new();
        if expoly.contour.points.len() < 3 {
            return paths;
        }
        let line_width = self.line_width.max(f32::EPSILON);
        let source = [expoly.clone()];

        for wall_index in 0..self.wall_count {
            let inset = -line_width * (wall_index as f32 + 0.5);
            let ring_set = host::offset_polygons(&source, inset, OffsetJoinType::Miter, 0.0);
            for ring_poly in &ring_set {
                for ring in std::iter::once(&ring_poly.contour).chain(ring_poly.holes.iter()) {
                    if ring.points.len() < 3 {
                        continue;
                    }
                    let mut wall = ring
                        .points
                        .iter()
                        .map(|point| self.support_point(point.x, point.y, z))
                        .collect::<Vec<_>>();
                    wall.push(wall[0]);
                    paths.push(ExtrusionPath3D {
                        points: wall,
                        role: ExtrusionRole::SupportMaterial,
                        speed_factor,
                        tool_index: None,
                    });
                }
            }
        }

        // Fill only the area the walls do not already cover.
        let fill_regions = if self.wall_count == 0 {
            source.to_vec()
        } else {
            host::offset_polygons(
                &source,
                -line_width * self.wall_count as f32,
                OffsetJoinType::Miter,
                0.0,
            )
        };

        // `support_density` is a fraction in (0, 1]; 1.0 gives a solid
        // `line_width` pitch. A non-positive density means walls only.
        let spacing = match fill_spacing_override_mm {
            Some(override_mm) if override_mm > 0.0 => override_mm as f64,
            Some(_) => return paths,
            None => {
                if self.density <= 0.0 {
                    return paths;
                }
                (line_width / self.density.min(1.0)) as f64
            }
        };
        for region in &fill_regions {
            paths.extend(self.scan_fill_region(region, spacing, z, speed_factor));
        }
        paths
    }

    /// Build one support vertex from scaled-integer coordinates.
    fn support_point(&self, x: i64, y: i64, z: f32) -> Point3WithWidth {
        Point3WithWidth {
            x: slicer_ir::units_to_mm(x),
            y: slicer_ir::units_to_mm(y),
            z,
            width: self.line_width,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
            overhang_distance_mm: None,
        }
    }

    /// Axis-aligned scan fill of one `ExPolygon`, honouring its holes.
    ///
    /// Crossings are gathered from the contour *and* every hole ring, so an
    /// interior void is not filled over.
    fn scan_fill_region(
        &self,
        expoly: &ExPolygon,
        spacing: f64,
        z: f32,
        speed_factor: f32,
    ) -> Vec<ExtrusionPath3D> {
        let mut paths = Vec::new();
        if expoly.contour.points.len() < 3 || spacing <= 0.0 {
            return paths;
        }
        let (min_x, min_y, max_x, max_y) = polygon_bbox_mm(expoly);
        let rings: Vec<&slicer_ir::Polygon> = std::iter::once(&expoly.contour)
            .chain(expoly.holes.iter())
            .collect();
        let mut y = min_y + spacing * 0.5;
        while y < max_y {
            let mut crossings = Vec::new();
            for ring in &rings {
                let points = &ring.points;
                for i in 0..points.len() {
                    let a = &points[i];
                    let b = &points[(i + 1) % points.len()];
                    let ay = slicer_ir::units_to_mm(a.y) as f64;
                    let by = slicer_ir::units_to_mm(b.y) as f64;
                    if (ay > y) != (by > y) {
                        let ax = slicer_ir::units_to_mm(a.x) as f64;
                        let bx = slicer_ir::units_to_mm(b.x) as f64;
                        crossings.push(ax + (y - ay) * (bx - ax) / (by - ay));
                    }
                }
            }
            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for pair in crossings.chunks_exact(2) {
                if pair[1] > pair[0] && pair[0] >= min_x && pair[1] <= max_x {
                    paths.push(ExtrusionPath3D {
                        points: vec![
                            self.support_point(
                                slicer_ir::mm_to_units(pair[0] as f32),
                                slicer_ir::mm_to_units(y as f32),
                                z,
                            ),
                            self.support_point(
                                slicer_ir::mm_to_units(pair[1] as f32),
                                slicer_ir::mm_to_units(y as f32),
                                z,
                            ),
                        ],
                        role: ExtrusionRole::SupportMaterial,
                        speed_factor,
                        tool_index: None,
                    });
                }
            }
            y += spacing;
        }
        paths
    }
}

// expolygon_centroid was an artifact of the deleted local support_paint_policy
// stub.  The v2 query lives in `PaintRegionLayerView::paint_policy_for` (slicer-sdk).

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

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::Point2;

    #[test]
    fn from_config_defaults() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = TreeSupport::from_config(&config).unwrap();
        assert!(!module.enabled);
        assert!((module.density - 0.2).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
    }

    /// F-7: the tree renderer had no interface pitch at all — roofs and floors
    /// were scan-filled at the density-derived body pitch. Canonical is
    /// `support_interface_spacing + interface_flow.spacing()`, measured at
    /// 0.757 mm for a 0.4 mm gap / 0.4 mm width / 0.2 mm layer height.
    #[test]
    fn interface_pitch_adds_flow_spacing() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = TreeSupport::from_config(&config).unwrap();
        let (_, top, bottom) = module.interface_pitch_mm(0.2);
        assert!(
            (top - 0.757).abs() < 0.002,
            "measured Orca interface pitch is 0.757 mm, got {top}"
        );
        assert_eq!(bottom, top, "negative bottom spacing mirrors the top gap");
        // The interface pitch must not be the body pitch (line_width/density).
        let body_pitch = module.line_width / module.density.min(1.0);
        assert!(
            (top - body_pitch).abs() > 0.01,
            "interface pitch must be independent of support_density"
        );
    }

    #[test]
    fn walls_are_inset_and_fill_does_not_overlap_them() {
        // Guards the packet-224 fix: `render_polygon` used to emit `wall_count`
        // coincident copies of the same contour and then scan-fill the whole
        // polygon at a `line_width` pitch, so a body was extruded several times
        // over the same area.
        let mut map = std::collections::HashMap::new();
        map.insert("enable_support".to_string(), ConfigValue::Bool(true));
        map.insert("tree_support_wall_count".to_string(), ConfigValue::Int(2));
        let module = TreeSupport::from_config(&ConfigView::from_map(map)).unwrap();

        let square = ExPolygon {
            contour: slicer_ir::Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(10.0, 0.0),
                    Point2::from_mm(10.0, 10.0),
                    Point2::from_mm(0.0, 10.0),
                ],
            },
            holes: vec![],
        };
        let paths = module.render_polygon(&square, 1.0, 1.0, None);
        let closed: Vec<&ExtrusionPath3D> =
            paths.iter().filter(|path| path.points.len() > 2).collect();
        assert_eq!(closed.len(), 2, "expected exactly `wall_count` wall loops");

        // Each wall must sit at its own inset, not on top of the previous one.
        let extent = |path: &ExtrusionPath3D| {
            let xs: Vec<f32> = path.points.iter().map(|p| p.x).collect();
            xs.iter().cloned().fold(f32::MIN, f32::max)
                - xs.iter().cloned().fold(f32::MAX, f32::min)
        };
        let outer = extent(closed[0]);
        let inner = extent(closed[1]);
        assert!(
            outer - inner > 0.3,
            "walls are coincident: outer extent {outer}, inner extent {inner}"
        );

        // Fill lines must start clear of the walls, not span the full polygon.
        let fill_max_x = paths
            .iter()
            .filter(|path| path.points.len() == 2)
            .flat_map(|path| path.points.iter())
            .map(|p| p.x)
            .fold(f32::MIN, f32::max);
        assert!(
            fill_max_x < 10.0 - module.line_width,
            "fill reaches {fill_max_x}, overlapping the wall band"
        );
    }

    #[test]
    fn fill_pitch_honours_support_density() {
        let build = |density: f64| {
            let mut map = std::collections::HashMap::new();
            map.insert("enable_support".to_string(), ConfigValue::Bool(true));
            map.insert("tree_support_wall_count".to_string(), ConfigValue::Int(1));
            map.insert("support_density".to_string(), ConfigValue::Float(density));
            TreeSupport::from_config(&ConfigView::from_map(map)).unwrap()
        };
        let square = ExPolygon {
            contour: slicer_ir::Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(20.0, 0.0),
                    Point2::from_mm(20.0, 20.0),
                    Point2::from_mm(0.0, 20.0),
                ],
            },
            holes: vec![],
        };
        let count = |module: &TreeSupport| {
            module
                .render_polygon(&square, 1.0, 1.0, None)
                .iter()
                .filter(|path| path.points.len() == 2)
                .count()
        };
        let sparse = count(&build(20.0));
        let solid = count(&build(100.0));
        assert!(
            solid > sparse * 3,
            "support_density is ignored: {sparse} lines at 20% vs {solid} at 100%"
        );
    }
}
