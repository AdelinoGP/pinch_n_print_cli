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
//! Generates parallel scan-line fill patterns at the configured angle.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_core::flow::{
    bridging_flow, canonical_bridging_flow, line_width_to_spacing, resolve_role_width,
    RoleWidthContext,
};
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point3WithWidth,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Solid shell polygons are emitted at full density, independently of sparse
/// infill density.
const SOLID_DENSITY: f32 = 1.0;

/// Rectilinear infill generator.
///
/// Produces parallel fill lines via scan-line polygon intersection,
/// alternating direction by 90 degrees on each layer.
pub struct RectilinearInfill {
    /// Infill density (0.0 to 1.0).
    density: f32,
    /// Base infill angle in degrees.
    base_angle: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
    /// Global role-specific width inputs used for solid paths.
    width_context: RoleWidthContext,
    /// Bridge line density, expressed as a fraction of full density.
    bridge_density: f32,
    /// Bridge extrusion speed in millimeters per second.
    bridge_speed: f32,
    /// Bridge flow ratio.
    bridge_flow_ratio: f32,
    /// Whether bridge paths use the round-thread flow model.
    thick_bridges: bool,
    /// Internal bridge density, expressed as a fraction of full density.
    internal_bridge_density: f32,
    /// Internal bridge extrusion speed in millimeters per second.
    internal_bridge_speed: f32,
    /// Internal bridge flow ratio.
    internal_bridge_flow_ratio: f32,
    /// Whether internal bridge paths use the round-thread flow model.
    thick_internal_bridges: bool,
    /// Role-specific speeds retained for module configuration compatibility.
    top_surface_speed: f32,
    internal_solid_infill_speed: f32,
    sparse_infill_speed: f32,
    /// Owned internal-bridge controls. The angle and extra-layer controls are
    /// parsed here; the post-process seam currently consumes neither channel.
    dont_filter_internal_bridges: bool,
    enable_extra_bridge_layer: bool,
    internal_bridge_angle: f32,
    /// Per-layer scan-line shift step (mm). Alternates sign each layer
    /// to interleave, not stack. Default 0.0 (no shift).
    infill_shift_step: f32,
}

#[slicer_module]
impl LayerModule for RectilinearInfill {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let density = match config.get("infill_density") {
            Some(ConfigValue::Float(d)) => *d as f32,
            _ => 0.2,
        };

        let base_angle = match config.get("infill_direction") {
            Some(ConfigValue::Float(a)) => *a as f32,
            _ => 0.0,
        };

        let infill_speed = match config.get("infill_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => 60.0,
        };

        let speed_value = |key: &str, default: f32| match config.get(key) {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => default,
        };

        let width_value = |key: &str| match config.get(key) {
            Some(ConfigValue::Float(w)) => *w as f32,
            Some(ConfigValue::Int(w)) => *w as f32,
            _ => 0.0,
        };
        let width_context = RoleWidthContext {
            // Packet 185 (AC-5): absent `line_width` is the canonical auto-0
            // sentinel (resolved to 1.125 × nozzle by `resolve_role_width`),
            // not the legacy 0.4 mm default.
            line_width: match config.get("line_width") {
                Some(ConfigValue::Float(w)) => *w as f32,
                Some(ConfigValue::Int(w)) => *w as f32,
                _ => 0.0,
            },
            nozzle_diameter: 0.4,
            bridge_line_width: width_value("bridge_line_width"),
            initial_layer_line_width: width_value("initial_layer_line_width"),
            top_surface_line_width: width_value("top_surface_line_width"),
            internal_solid_infill_line_width: width_value("internal_solid_infill_line_width"),
            sparse_infill_line_width: width_value("sparse_infill_line_width"),
            ..RoleWidthContext::default()
        };
        let line_width =
            resolve_role_width(ExtrusionRole::SparseInfill, false, false, &width_context);

        let bridge_density = config
            .get_abs_value("bridge_density", 1.0)
            .map(|d| d as f32)
            .unwrap_or(1.0);
        let bridge_speed = match config.get("bridge_speed") {
            Some(ConfigValue::Float(s)) => *s as f32,
            Some(ConfigValue::Int(s)) => *s as f32,
            _ => 25.0,
        };
        let bridge_flow_ratio = config
            .get_float("bridge_flow")
            .map(|flow| flow as f32)
            .unwrap_or(1.0);
        let thick_bridges = config.get_bool("thick_bridges").unwrap_or(false);

        let internal_bridge_density = config
            .get_abs_value("internal_bridge_density", 1.0)
            .map(|density| density as f32)
            .unwrap_or(1.0);
        let internal_bridge_speed = config
            .get_abs_value("internal_bridge_speed", f64::from(bridge_speed))
            .map(|speed| speed as f32)
            .unwrap_or(bridge_speed * 1.5);
        let internal_bridge_flow_ratio = config
            .get_float("internal_bridge_flow")
            .map(|flow| flow as f32)
            .unwrap_or(1.0);
        let thick_internal_bridges = config.get_bool("thick_internal_bridges").unwrap_or(true);

        let top_surface_speed = speed_value("top_surface_speed", 60.0);
        let internal_solid_infill_speed = speed_value("internal_solid_infill_speed", 60.0);
        let sparse_infill_speed = speed_value("sparse_infill_speed", infill_speed);
        let dont_filter_internal_bridges = config
            .get_bool("dont_filter_internal_bridges")
            .unwrap_or(false);
        let enable_extra_bridge_layer = config
            .get_bool("enable_extra_bridge_layer")
            .unwrap_or(false);
        let internal_bridge_angle = speed_value("internal_bridge_angle", 0.0);

        let infill_shift_step = match config.get("infill_shift_step") {
            Some(ConfigValue::Float(s)) => *s as f32,
            _ => 0.0,
        };

        Ok(Self {
            density,
            base_angle,
            line_width,
            width_context,
            bridge_density,
            bridge_speed,
            bridge_flow_ratio,
            thick_bridges,
            internal_bridge_density,
            internal_bridge_speed,
            internal_bridge_flow_ratio,
            thick_internal_bridges,
            top_surface_speed,
            internal_solid_infill_speed,
            sparse_infill_speed,
            dont_filter_internal_bridges,
            enable_extra_bridge_layer,
            internal_bridge_angle,
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
        // Rectilinear direction is constant across layers; only the optional
        // scan-line shift alternates.
        let angle_deg = self.base_angle as f64;
        let angle_rad = angle_deg.to_radians();

        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        // Feedrate is resolved by the host from each emitted role. Do not
        // couple sparse, solid, and bridge paths through one scalar.
        let speed_factor = 1.0;
        // These settings are intentionally owned by the runtime seam: the WIT
        // infill output has no bridge-postprocess control interface. The host
        // receives the same resolved ConfigView and applies angle/filter/flow;
        // extra-layer remains parse-only until a neighboring-layer API exists.
        let _host_bridge_settings = (
            self.top_surface_speed,
            self.internal_solid_infill_speed,
            self.sparse_infill_speed,
            self.dont_filter_internal_bridges,
            self.enable_extra_bridge_layer,
            self.internal_bridge_angle,
        );

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

            // A per-region base width override remains the fallback for role
            // widths whose dedicated setting is unset.
            let mut region_width_context = self.width_context;
            region_width_context.line_width = slicer_sdk::config_resolution::resolve_float(
                region,
                "line_width",
                self.width_context.line_width,
            );
            region_width_context.bridge_line_width = slicer_sdk::config_resolution::resolve_float(
                region,
                "bridge_line_width",
                self.width_context.bridge_line_width,
            );

            let sparse_spacing = if region_density > 0.0 {
                slicer_ir::mm_to_units(region_line_width / region_density)
            } else {
                0
            };

            // SparseInfill over the host-partitioned sparse-only polygon.
            let sparse = region.sparse_infill_area();
            if sparse_spacing > 0
                && !sparse.is_empty()
                && region.should_emit(ExtrusionRole::SparseInfill)
            {
                for expoly in sparse {
                    let paths = scan_expolygon(
                        expoly,
                        sparse_spacing,
                        std_cos_a,
                        std_sin_a,
                        z,
                        speed_factor,
                        1.0,
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
                let solid_line_width = resolve_role_width(
                    role.clone(),
                    layer_index == 0,
                    false,
                    &region_width_context,
                );
                let solid_spacing = slicer_ir::mm_to_units(solid_line_width / SOLID_DENSITY);
                for expoly in top {
                    let paths = scan_expolygon(
                        expoly,
                        solid_spacing,
                        std_cos_a,
                        std_sin_a,
                        z,
                        speed_factor,
                        1.0,
                        &role,
                        solid_line_width,
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
                let solid_line_width = resolve_role_width(
                    role.clone(),
                    layer_index == 0,
                    false,
                    &region_width_context,
                );
                let solid_spacing = slicer_ir::mm_to_units(solid_line_width / SOLID_DENSITY);
                for expoly in bottom {
                    let paths = scan_expolygon(
                        expoly,
                        solid_spacing,
                        std_cos_a,
                        std_sin_a,
                        z,
                        speed_factor,
                        1.0,
                        &role,
                        solid_line_width,
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
                let is_internal_bridge = region.is_internal_bridge();
                let bridge_role = if is_internal_bridge {
                    ExtrusionRole::InternalBridgeInfill
                } else {
                    ExtrusionRole::BridgeInfill
                };
                let deg = region.bridge_orientation_deg() as f64;
                let rad = deg.to_radians();
                let (bridge_cos_a, bridge_sin_a) = (rad.cos(), rad.sin());
                let bridge_width = resolve_role_width(
                    ExtrusionRole::BridgeInfill,
                    layer_index == 0,
                    true,
                    &region_width_context,
                );
                let layer_height = region.effective_layer_height();
                let bridge_flow_ratio = slicer_sdk::config_resolution::resolve_float(
                    region,
                    if is_internal_bridge {
                        "internal_bridge_flow"
                    } else {
                        "bridge_flow"
                    },
                    if is_internal_bridge {
                        self.internal_bridge_flow_ratio
                    } else {
                        self.bridge_flow_ratio
                    },
                );
                let thick_bridges = region
                    .config()
                    .and_then(|config| {
                        config.get_bool(if is_internal_bridge {
                            "thick_internal_bridges"
                        } else {
                            "thick_bridges"
                        })
                    })
                    .unwrap_or(if is_internal_bridge {
                        self.thick_internal_bridges
                    } else {
                        self.thick_bridges
                    });
                let bridge_spacing_mm = if thick_bridges {
                    canonical_bridging_flow(
                        region_width_context.bridge_line_width,
                        bridge_flow_ratio,
                        region_width_context.nozzle_diameter,
                    )
                    .spacing_mm
                } else {
                    line_width_to_spacing(bridge_width, layer_height).unwrap_or(bridge_width)
                };
                let bridge_density = slicer_sdk::config_resolution::resolve_float(
                    region,
                    if is_internal_bridge {
                        "internal_bridge_density"
                    } else {
                        "bridge_density"
                    },
                    if is_internal_bridge {
                        self.internal_bridge_density
                    } else {
                        self.bridge_density
                    },
                );
                let bridge_density = region
                    .config()
                    .and_then(|config| {
                        config.get_abs_value(
                            if is_internal_bridge {
                                "internal_bridge_density"
                            } else {
                                "bridge_density"
                            },
                            1.0,
                        )
                    })
                    .map(|density| density as f32)
                    .unwrap_or(bridge_density);
                let bridge_spacing = if bridge_density > 0.0 {
                    slicer_ir::mm_to_units(bridge_spacing_mm / bridge_density)
                } else {
                    0
                };
                let bridge_speed = if is_internal_bridge {
                    region
                        .config()
                        .and_then(|config| {
                            config.get_abs_value("internal_bridge_speed", self.bridge_speed as f64)
                        })
                        .map(|speed| speed as f32)
                        .unwrap_or(self.internal_bridge_speed)
                } else {
                    slicer_sdk::config_resolution::resolve_float(
                        region,
                        "bridge_speed",
                        self.bridge_speed,
                    )
                };
                // The G-code emitter resolves the role's base speed; this
                // scalar carries per-region speed overrides into that role.
                let configured_base_speed = if is_internal_bridge {
                    self.internal_bridge_speed
                } else {
                    self.bridge_speed
                };
                let bridge_speed_factor = if configured_base_speed > 0.0 {
                    bridge_speed / configured_base_speed
                } else {
                    1.0
                };
                let thread_base_width = if region_width_context.bridge_line_width > 0.0 {
                    bridge_width
                } else {
                    region_width_context.nozzle_diameter
                };
                let bridge_flow_factor = bridging_flow(
                    bridge_flow_ratio,
                    thick_bridges,
                    thread_base_width,
                    bridge_width,
                    layer_height,
                );
                for expoly in bridge {
                    let paths = scan_expolygon(
                        expoly,
                        bridge_spacing,
                        bridge_cos_a,
                        bridge_sin_a,
                        z,
                        bridge_speed_factor,
                        bridge_flow_factor,
                        &bridge_role,
                        bridge_width,
                        false,
                        x_shift_units,
                    );
                    for path in paths {
                        let _ = output.push_solid_path(path);
                    }
                }
            }
        }

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
/// D-209-ADJUST-SOLID-SPACING-DIVERGENCE: differs from OrcaSlicer's
/// `Fill::_adjust_solid_spacing` (`FillBase.cpp`) in three ways: PnP uses bare
/// `width` instead of canonical `(width - EPSILON)` as the numerator of both
/// divisions; PnP rounds where canonical truncates; and PnP returns the
/// unmodified `distance` on the over-cap branch where canonical returns
/// `floor(distance * 1.2 + 0.5)`.
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
/// as the reference point (AC-3 invariant). Scan rows use a half-open grid:
/// `scan_y` starts at `rmin_y`, advances by the effective spacing, and emits
/// while `scan_y < rmax_y`, yielding `ceil(height / spacing)` rows. The
/// half-open vertex test (include at min_y, exclude at max_y) prevents
/// double-counting at polygon vertices (AC-N1).
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
    flow_factor: f32,
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
    let contour_pts = &expoly.contour.points;
    let n = contour_pts.len();
    if n >= 2 {
        for i in 0..n {
            let j = (i + 1) % n;
            let p_i = &contour_pts[i];
            let p_j = &contour_pts[j];
            edges.push((p_i.x, p_i.y, p_j.x, p_j.y));
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
    for &(x1, y1, x2, y2) in &edges {
        let (rx1, ry1) = rotate_point(x1 - refpt_x, y1 - refpt_y, cos_neg, sin_neg);
        let (rx2, ry2) = rotate_point(x2 - refpt_x, y2 - refpt_y, cos_neg, sin_neg);
        rotated_edges.push((rx1, ry1, rx2, ry2));
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

    let mut paths = Vec::new();
    let mut scan_y = rmin_y;

    while scan_y < rmax_y {
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
                flow_factor,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            };
            let end = Point3WithWidth {
                x: slicer_ir::units_to_mm(ex + refpt_x + x_shift),
                y: slicer_ir::units_to_mm(ey + refpt_y),
                z,
                width: line_width,
                flow_factor,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            };

            paths.push(ExtrusionPath3D {
                points: vec![start, end],
                role: role.clone(),
                speed_factor,
                tool_index: None,
                order_lock: None,
            });

            i += 2;
        }

        scan_y += effective_spacing;
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
    fn from_config_defaults() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = RectilinearInfill::from_config(&config).unwrap();
        assert!((module.density - 0.2).abs() < 0.001);
        // Packet 185 (AC-5): absent line_width resolves to the canonical
        // auto width 1.125 × nozzle_diameter (0.45 at the module's fixed
        // 0.4 mm nozzle), not the legacy 0.4 mm default.
        assert!((module.line_width - 0.45).abs() < 0.001);
    }
}
