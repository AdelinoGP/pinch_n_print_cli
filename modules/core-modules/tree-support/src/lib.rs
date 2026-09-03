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
//!    `BottomInterface`), render the polygon with `render_polygon`: `tree_support_wall_count`
//!    inset perimeter passes plus a scan fill inset clear of them.
//! 3. Derive body and interface pitches from canonical flow spacing helpers.
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
use slicer_sdk::LayerCollectionBuilder;

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
    /// Support print speed in mm/s.
    support_speed: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
    /// Resolved interface flow ratio as a percentage.
    interface_flow_percent: f32,
    /// Number of perimeter passes used to represent a support body.
    wall_loops: usize,
    /// Configured top-interface line gap in millimeters (canonical
    /// `support_interface_spacing`). This is the *gap*, not the pitch.
    top_interface_spacing_mm: f32,
    /// Configured bottom-interface line gap in millimeters (canonical
    /// `support_bottom_interface_spacing`). Negative mirrors the top value.
    bottom_interface_spacing_mm: f32,
    /// Configured gap between adjacent body lines in millimeters.
    base_pattern_spacing_mm: f32,
}

impl TreeSupport {
    /// Interface scan-fill pitch in millimetres for the top and bottom
    /// interface roles at a given layer height.
    ///
    /// Canonical `SupportParameters` (`SupportParameters.hpp`):
    /// `interface_spacing = support_interface_spacing + interface_flow.spacing()`,
    /// where `spacing()` is `Flow::rounded_rectangle_extrusion_spacing`
    /// (in-tree: `slicer_core::flow::line_width_to_spacing`).
    /// Returns `(interface_width_mm, interface_flow_spacing_mm, body_pitch_mm,
    /// top_pitch_mm, bottom_pitch_mm)`.
    /// The bare flow spacing is exposed because canonical
    /// `generate_interface_layers` derives its smoothing/closing distance
    /// (`scaled_spacing() * 1.5`) and its minimum island radii
    /// (`scaled_spacing() / interface_density`) from it.
    fn pitches_mm(&self, layer_height_mm: f32) -> Result<(f32, f32, f32, f32, f32), ModuleError> {
        let layer_height = layer_height_mm.max(0.0);
        let interface_width = self.line_width
            * (slicer_core::support_regularize::resolved_interface_flow_ratio(
                self.interface_flow_percent,
            ) / 100.0);
        let body_flow_spacing =
            slicer_core::flow::line_width_to_spacing(self.line_width, layer_height)
                .map_err(|error| ModuleError::non_fatal(333, error.to_string()))?;
        let interface_flow_spacing =
            slicer_core::flow::line_width_to_spacing(interface_width, layer_height)
                .map_err(|error| ModuleError::non_fatal(333, error.to_string()))?;
        let top_gap = self.top_interface_spacing_mm.max(0.0);
        // Negative mirrors the top gap, per OrcaSlicer's `-1 == same as top`
        // convention for the paired bottom-interface keys.
        let bottom_gap = if self.bottom_interface_spacing_mm < 0.0 {
            top_gap
        } else {
            self.bottom_interface_spacing_mm
        };
        let body_density = slicer_core::support_regularize::body_density(
            self.line_width,
            layer_height,
            self.base_pattern_spacing_mm,
        )
        .map_err(|error| ModuleError::non_fatal(333, error.to_string()))?;
        let top_density = slicer_core::support_regularize::interface_density(
            interface_width,
            layer_height,
            top_gap,
        )
        .map_err(|error| ModuleError::non_fatal(333, error.to_string()))?;
        let bottom_density = slicer_core::support_regularize::bottom_interface_density(
            interface_width,
            layer_height,
            bottom_gap,
        )
        .map_err(|error| ModuleError::non_fatal(333, error.to_string()))?;
        let pitch = |density: f32| {
            (interface_flow_spacing / density.max(f32::EPSILON)).max(interface_width)
        };
        let body_pitch = (body_flow_spacing / body_density.max(f32::EPSILON)).max(self.line_width);
        Ok((
            interface_width,
            interface_flow_spacing,
            body_pitch,
            pitch(top_density),
            pitch(bottom_density),
        ))
    }

    /// Returns whether support is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the configured line width.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}

/// Return the largest skeleton-requested wall count whose point lies in a role
/// region. Counts are positional and default to zero for malformed carriers.
fn skeleton_wall_count(skeleton: &slicer_ir::SupportPlanSkeleton, expoly: &ExPolygon) -> usize {
    let contained = skeleton
        .points
        .iter()
        .zip(skeleton.wall_counts.iter().copied())
        .filter(|(point, _)| point_in_expolygon(expoly, point.x, point.y))
        .map(|(_, count)| count as usize)
        .max()
        .unwrap_or(0);
    contained
}

/// Ray-cast a millimetre-coordinate point against an IR polygon.
fn point_in_expolygon(expoly: &ExPolygon, x: f32, y: f32) -> bool {
    if expoly.contour.points.is_empty() {
        return false;
    }
    let min_x = expoly
        .contour
        .points
        .iter()
        .map(|point| point.x)
        .min()
        .unwrap();
    let max_x = expoly
        .contour
        .points
        .iter()
        .map(|point| point.x)
        .max()
        .unwrap();
    let min_y = expoly
        .contour
        .points
        .iter()
        .map(|point| point.y)
        .min()
        .unwrap();
    let max_y = expoly
        .contour
        .points
        .iter()
        .map(|point| point.y)
        .max()
        .unwrap();
    let point = slicer_ir::Point2::from_mm(x, y);
    if point.x < min_x || point.x > max_x || point.y < min_y || point.y > max_y {
        return false;
    }
    fn inside(ring: &[slicer_ir::Point2], x: i64, y: i64) -> bool {
        let mut result = false;
        let mut previous = ring.len().saturating_sub(1);
        for current in 0..ring.len() {
            let a = ring[current];
            let b = ring[previous];
            if (a.y > y) != (b.y > y) {
                let crossing =
                    (b.x - a.x) as f64 * (y - a.y) as f64 / (b.y - a.y) as f64 + a.x as f64;
                if (x as f64) < crossing {
                    result = !result;
                }
            }
            previous = current;
        }
        result
    }
    inside(&expoly.contour.points, point.x, point.y)
        && !expoly
            .holes
            .iter()
            .any(|hole| inside(&hole.points, point.x, point.y))
}

#[slicer_module]
impl LayerModule for TreeSupport {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("enable_support") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
        };

        let support_speed = match config.get("support_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => BASE_SPEED,
        };

        let nozzle_diameter = config.get_float("nozzle_diameter").unwrap_or(0.4);
        let line_width = config
            .get_abs_value("support_line_width", nozzle_diameter)
            .or_else(|| config.get_int("support_line_width").map(|v| v as f64))
            .map(|w| {
                if w > 0.0 {
                    w as f32
                } else {
                    (1.125 * nozzle_diameter) as f32
                }
            })
            .filter(|w| *w > 0.0)
            .unwrap_or(1.125 * nozzle_diameter as f32);
        let interface_flow_percent = match config.get("support_interface_flow") {
            Some(ConfigValue::Float(value)) => *value as f32,
            Some(ConfigValue::Int(value)) => *value as f32,
            _ => 100.0,
        };
        let wall_loops = match config.get("tree_support_wall_count") {
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
        let base_pattern_spacing_mm = config
            .get_float("support_base_pattern_spacing")
            .unwrap_or(2.5) as f32;

        Ok(Self {
            enabled,
            support_speed,
            line_width,
            interface_flow_percent,
            wall_loops,
            top_interface_spacing_mm,
            bottom_interface_spacing_mm,
            base_pattern_spacing_mm,
        })
    }

    fn run_support(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        paint: &PaintRegionLayerView,
        output: &mut SupportOutputBuilder,
        anchored_collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        let speed_factor = self.support_speed / BASE_SPEED;
        for region in regions {
            let z = region.z();
            // F-7: interface roles are pitched by canonical
            // `SupportParameters::interface_spacing`
            // (`support_interface_spacing + interface_flow.spacing()`), not by
            // the body pitch. The tree renderer previously had no interface
            // spacing at all and scan-filled roofs and floors at the body
            // pitch.
            let layer_height = if region.effective_layer_height() > 0.0 {
                region.effective_layer_height()
            } else {
                _config.get_float("layer_height").unwrap_or(0.2) as f32
            };
            let (
                interface_width_mm,
                interface_flow_spacing_mm,
                body_spacing,
                top_interface_spacing,
                bottom_interface_spacing,
            ) = self.pitches_mm(layer_height)?;

            // Structural support plans carry semantic regions, not printable
            // paths. A missing entry means this demand was declined; do not
            // resurrect it with the legacy grid-MST filler.
            let planned_entries: Vec<&slicer_ir::SupportPlanEntry> = paint
                .support_plan()
                .map(|plan| {
                    plan.entries
                        .iter()
                        .filter(|entry| {
                            entry.object_id == region.object_id().as_str()
                                && entry.region_id == *region.region_id()
                                && (entry.global_layer_index == layer_index as i32
                                    || (entry.anchor_layer_index == layer_index
                                        && entry.anchor_z.abs_diff(slicer_ir::mm_to_units(z))
                                            > slicer_ir::AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS
                                                as u64))
                        })
                        .collect()
                })
                .unwrap_or_default();

            if planned_entries.is_empty() {
                continue;
            }

            let mut off_grid_entities: Vec<slicer_ir::AnchoredEntity> = Vec::new();
            for entry in planned_entries.iter().filter(|entry| {
                entry.decline_reason.is_none()
                    && (entry.global_layer_index == layer_index as i32
                        || (entry.anchor_layer_index == layer_index
                            && entry.anchor_z.abs_diff(slicer_ir::mm_to_units(z))
                                > slicer_ir::AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS
                                    as u64))
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
                // Packet 239c Step 4: the plan entry DECLARES its own print
                // plane (`entry.anchor_z`, canonical units). On-grid entries
                // render at `region.z()` through the unchanged on-grid push
                // route; off-grid entries render at the declared plane and
                // leave as an anchored event collection through 239b's drain
                // (the `collection` parameter this `run_support` receives),
                // so `region.z()` can never misplace an off-grid row. The
                // single on-grid/off-grid discriminator is
                // `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`, the
                // same constant the planner used to derive the plane.
                let plan_z_units = slicer_ir::mm_to_units(z);
                let off_grid = entry.anchor_z.abs_diff(plan_z_units)
                    > slicer_ir::AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS as u64;
                let print_z_mm = if off_grid {
                    slicer_ir::units_to_mm(entry.anchor_z)
                } else {
                    z
                };

                // Off-grid paths accumulate here in role order so they can be
                // proposed as ONE anchored collection per dispatch (the
                // builder rejects a second proposal).
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
                let regularized = slicer_core::support_regularize::regularize_entry_roles(
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
                    })
                    .into_iter()
                    .map(|(role, regions)| (role, slicer_core::polygon_ops::union_ex(&regions)))
                    .collect();
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
                            slicer_ir::SupportPlanRole::SupportBody => body_spacing,
                            slicer_ir::SupportPlanRole::TopInterface => top_interface_spacing,
                            slicer_ir::SupportPlanRole::BaseInterface => top_interface_spacing,
                            slicer_ir::SupportPlanRole::BottomInterface => bottom_interface_spacing,
                            slicer_ir::SupportPlanRole::RaftRelated => continue,
                        };
                        let vertical = entry.anchor_layer_index % 2 != 0;
                        let extra_walls = entry
                            .skeleton
                            .as_ref()
                            .map(|skeleton| skeleton_wall_count(skeleton, expoly))
                            .unwrap_or(0);
                        let mut paths = self.render_polygon_with_wall_count(
                            expoly,
                            print_z_mm,
                            speed_factor,
                            fill_spacing,
                            vertical,
                            self.wall_loops + extra_walls,
                        );
                        if matches!(
                            role,
                            slicer_ir::SupportPlanRole::TopInterface
                                | slicer_ir::SupportPlanRole::BaseInterface
                                | slicer_ir::SupportPlanRole::BottomInterface
                        ) {
                            for path in &mut paths {
                                for point in &mut path.points {
                                    point.width = interface_width_mm;
                                }
                            }
                        }
                        let extrusion_role = match role {
                            // The extrusion role must be stamped here, not
                            // left as `SupportMaterial`: `;TYPE:Support
                            // interface` and `support_interface_speed` are
                            // both selected from `ExtrusionRole` in
                            // `crates/slicer-gcode/src/emit.rs`, so an
                            // interface path that keeps the body role is
                            // emitted and fed as plain support.
                            slicer_ir::SupportPlanRole::SupportBody => {
                                ExtrusionRole::SupportMaterial
                            }
                            slicer_ir::SupportPlanRole::TopInterface => {
                                ExtrusionRole::SupportInterface
                            }
                            slicer_ir::SupportPlanRole::BaseInterface => {
                                ExtrusionRole::SupportBaseInterface
                            }
                            slicer_ir::SupportPlanRole::BottomInterface => {
                                ExtrusionRole::SupportInterface
                            }
                            slicer_ir::SupportPlanRole::RaftRelated => {
                                ExtrusionRole::SupportMaterial
                            }
                        };
                        // Packet 239c Step 4: the off-grid branch carries its
                        // paths as ordered anchored events (declared plane =
                        // `entry.anchor_z`, mm points) with the plan entry's
                        // identity retained in the provenance; the on-grid
                        // route below is byte-identical to pre-239c.
                        if off_grid {
                            for path in paths {
                                off_grid_entities.push(slicer_ir::AnchoredEntity {
                                    local_id: off_grid_entities.len() as u64,
                                    anchor_global_layer_index: entry.anchor_layer_index,
                                    geometry: slicer_ir::AnchoredGeometryContract::Planar {
                                        z: entry.anchor_z,
                                    },
                                    input_capabilities: vec!["support.plan".to_string()],
                                    output_capabilities: vec!["extrusion.paths".to_string()],
                                    provenance: slicer_ir::AnchoredEntityProvenance {
                                        requesting_feature: "support-stage".to_string(),
                                        source_plan_entry: format!(
                                            "{}:{}:{}",
                                            entry.object_id,
                                            entry.region_id,
                                            entry.body_ids.first().cloned().unwrap_or_default()
                                        ),
                                    },
                                    path_points: path
                                        .points
                                        .iter()
                                        .map(|point| slicer_ir::Point3WithWidth {
                                            z: slicer_ir::units_to_mm(entry.anchor_z),
                                            ..*point
                                        })
                                        .collect(),
                                    role: extrusion_role.clone(),
                                });
                            }
                            continue;
                        }
                        for mut path in paths {
                            match role {
                                slicer_ir::SupportPlanRole::SupportBody => {
                                    let _ = output.push_support_path(path);
                                }
                                slicer_ir::SupportPlanRole::TopInterface => {
                                    path.role = ExtrusionRole::SupportInterface;
                                    let _ = output.push_interface_path(path, true);
                                }
                                slicer_ir::SupportPlanRole::BaseInterface => {
                                    path.role = ExtrusionRole::SupportBaseInterface;
                                    let _ = output.push_base_interface_path(path, true);
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
            // Packet 239c Step 4: propose all off-grid entries anchored to this
            // model layer as one atomic collection through 239b's drain.
            if !off_grid_entities.is_empty() {
                anchored_collection
                    .set_anchored_event_collection(slicer_ir::OrderedEventCollection {
                        anchor_global_layer_index: off_grid_entities[0].anchor_global_layer_index,
                        events: off_grid_entities,
                        runtime_hooks: slicer_ir::AnchoredEventRuntimeHooks::default(),
                    })
                    .map_err(|e| ModuleError::non_fatal(335, e))?;
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
    /// Before packet 224 this emitted `tree_support_wall_count` copies of the *same* contour
    /// (coincident, no inset) and then scan-filled the full polygon at a
    /// `line_width` pitch — solid regardless of configured spacing — so a
    /// support body was extruded several times over.
    ///
    /// `fill_spacing_override_mm` supplies the canonical interface pitch for
    /// interface roles; `None` keeps the density-derived body pitch.
    #[allow(dead_code)]
    fn render_polygon(
        &self,
        expoly: &ExPolygon,
        z: f32,
        speed_factor: f32,
        fill_spacing_mm: f32,
        vertical: bool,
    ) -> Vec<ExtrusionPath3D> {
        self.render_polygon_with_wall_count(
            expoly,
            z,
            speed_factor,
            fill_spacing_mm,
            vertical,
            self.wall_loops,
        )
    }

    /// Render a polygon with an explicit perimeter count from a structural
    /// skeleton override.
    fn render_polygon_with_wall_count(
        &self,
        expoly: &ExPolygon,
        z: f32,
        speed_factor: f32,
        fill_spacing_mm: f32,
        vertical: bool,
        wall_loops: usize,
    ) -> Vec<ExtrusionPath3D> {
        let mut paths = Vec::new();
        if expoly.contour.points.len() < 3 {
            return paths;
        }
        let line_width = self.line_width.max(f32::EPSILON);
        let source = [expoly.clone()];

        for wall_index in 0..wall_loops {
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
                        order_lock: None,
                    });
                }
            }
        }

        // Fill only the area the walls do not already cover.
        let fill_regions = if wall_loops == 0 {
            source.to_vec()
        } else {
            host::offset_polygons(
                &source,
                -line_width * wall_loops as f32,
                OffsetJoinType::Miter,
                0.0,
            )
        };

        let spacing = fill_spacing_mm.max(line_width) as f64;
        for region in &fill_regions {
            paths.extend(self.scan_fill_region(region, spacing, z, speed_factor, vertical));
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
        vertical: bool,
    ) -> Vec<ExtrusionPath3D> {
        let mut paths = Vec::new();
        if expoly.contour.points.len() < 3 || spacing <= 0.0 {
            return paths;
        }
        let (min_x, min_y, max_x, max_y) = polygon_bbox_mm(expoly);
        let rings: Vec<&slicer_ir::Polygon> = std::iter::once(&expoly.contour)
            .chain(expoly.holes.iter())
            .collect();
        let (scan_min, scan_max) = if vertical {
            (min_x, max_x)
        } else {
            (min_y, max_y)
        };
        let mut scan = scan_min + spacing * 0.5;
        let crossings_at = |scan: f64| {
            let mut crossings = Vec::new();
            for ring in &rings {
                let points = &ring.points;
                for i in 0..points.len() {
                    let a = &points[i];
                    let b = &points[(i + 1) % points.len()];
                    let ay = if vertical {
                        slicer_ir::units_to_mm(a.x)
                    } else {
                        slicer_ir::units_to_mm(a.y)
                    } as f64;
                    let by = if vertical {
                        slicer_ir::units_to_mm(b.x)
                    } else {
                        slicer_ir::units_to_mm(b.y)
                    } as f64;
                    if (ay > scan) != (by > scan) {
                        let ax = if vertical {
                            slicer_ir::units_to_mm(a.y)
                        } else {
                            slicer_ir::units_to_mm(a.x)
                        } as f64;
                        let bx = if vertical {
                            slicer_ir::units_to_mm(b.y)
                        } else {
                            slicer_ir::units_to_mm(b.x)
                        } as f64;
                        crossings.push(ax + (scan - ay) * (bx - ax) / (by - ay));
                    }
                }
            }
            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            crossings
        };
        let emit_scanline = |scan: f64, paths: &mut Vec<ExtrusionPath3D>| {
            let crossings = crossings_at(scan);
            for pair in crossings.chunks_exact(2) {
                if pair[1] > pair[0] && pair[0] >= min_x && pair[1] <= max_x {
                    paths.push(ExtrusionPath3D {
                        points: vec![
                            self.support_point(
                                slicer_ir::mm_to_units(if vertical {
                                    scan as f32
                                } else {
                                    pair[0] as f32
                                }),
                                slicer_ir::mm_to_units(if vertical {
                                    pair[0] as f32
                                } else {
                                    scan as f32
                                }),
                                z,
                            ),
                            self.support_point(
                                slicer_ir::mm_to_units(if vertical {
                                    scan as f32
                                } else {
                                    pair[1] as f32
                                }),
                                slicer_ir::mm_to_units(if vertical {
                                    pair[1] as f32
                                } else {
                                    scan as f32
                                }),
                                z,
                            ),
                        ],
                        role: ExtrusionRole::SupportMaterial,
                        speed_factor,
                        tool_index: None,
                        order_lock: None,
                    });
                }
            }
        };
        while scan < scan_max {
            emit_scanline(scan, &mut paths);
            scan += spacing;
        }
        if paths.is_empty() {
            emit_scanline((scan_min + scan_max) * 0.5, &mut paths);
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
        assert!((module.line_width - 0.45).abs() < 0.001);
    }

    /// F-7: the tree renderer had no interface pitch at all — roofs and floors
    /// were scan-filled at the density-derived body pitch. Canonical is
    /// `support_interface_spacing + interface_flow.spacing()`. At defaults the
    /// resolved support line width is 1.125 × 0.4 nozzle = 0.45 mm (238a auto
    /// resolution), so `line_width_to_spacing(0.45, 0.2) = 0.4070796` and the
    /// top pitch is 0.4 + 0.4070796 = 0.807 mm. With the key absent from the
    /// raw config map (as here) the in-code fallback stays the legacy −1.0
    /// mirror-top sentinel, so bottom == top; in production the manifest
    /// default 0.5 (DEV-145) is host-injected and yields a 0.907 mm bottom
    /// pitch instead.
    #[test]
    fn interface_pitch_adds_flow_spacing() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = TreeSupport::from_config(&config).unwrap();
        let (_, _, _, top, bottom) = module.pitches_mm(0.2).unwrap();
        assert!(
            (top - 0.807).abs() < 0.002,
            "canonical interface pitch is 0.807 mm at defaults, got {top}"
        );
        assert_eq!(
            bottom, top,
            "absent bottom-spacing key falls back to the mirror-top sentinel"
        );
        // The interface pitch must not be the body pitch (line_width/density).
        let body_pitch = module.pitches_mm(0.2).unwrap().2;
        assert!(
            (top - body_pitch).abs() > 0.01,
            "interface pitch must be independent of body spacing"
        );
    }

    #[test]
    fn walls_are_inset_and_fill_does_not_overlap_them() {
        // Guards the packet-224 fix: `render_polygon` used to emit `tree_support_wall_count`
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
        let paths = module.render_polygon(&square, 1.0, 1.0, 2.5, false);
        let closed: Vec<&ExtrusionPath3D> =
            paths.iter().filter(|path| path.points.len() > 2).collect();
        assert_eq!(
            closed.len(),
            2,
            "expected exactly `tree_support_wall_count` wall loops"
        );

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
    fn fill_pitch_derives_from_base_spacing() {
        let build = |density: f64| {
            let mut map = std::collections::HashMap::new();
            map.insert("enable_support".to_string(), ConfigValue::Bool(true));
            map.insert("tree_support_wall_count".to_string(), ConfigValue::Int(1));
            map.insert(
                "support_base_pattern_spacing".to_string(),
                ConfigValue::Float(density),
            );
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
        // Pitch derivation moved out of render_polygon into execute (via
        // pitches_mm), so the test derives each config's body pitch the same
        // way production does and feeds it through.
        let count = |module: &TreeSupport| {
            let body_pitch = module.pitches_mm(0.2).unwrap().2;
            module
                .render_polygon(&square, 1.0, 1.0, body_pitch, false)
                .iter()
                .filter(|path| path.points.len() == 2)
                .count()
        };
        let sparse = count(&build(2.5));
        let solid = count(&build(0.1));
        assert!(
            solid > sparse * 3,
            "base pattern spacing is ignored: {sparse} lines at wide pitch vs {solid} at narrow pitch"
        );
    }

    #[test]
    fn narrow_region_gets_one_fill_line_when_pitch_has_no_scan_rows() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = TreeSupport::from_config(&config).unwrap();
        let region = ExPolygon {
            contour: slicer_ir::Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(0.5, 0.0),
                    Point2::from_mm(0.5, 0.5),
                    Point2::from_mm(0.0, 0.5),
                ],
            },
            holes: vec![],
        };
        let fills = module.scan_fill_region(&region, 2.0, 1.0, 1.0, false);
        assert!(!fills.is_empty(), "a small tip region must not be hollow");
    }
}
