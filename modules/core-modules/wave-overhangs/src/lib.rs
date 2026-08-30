//! Wave-overhang bridge-fill generator module.
//!
//! Holds `claim:bridge-fill` and implements `LayerModule::run_infill` for the
//! `Layer::Infill` stage. External bridge areas are filled with iteratively
//! propagated "wave" contours anchored on the solid material below, following
//! the `dennisklappe/OrcaSlicer-WaveOverhangs` fork's `WaveOverhangs.cpp`
//! (ported in [`generator`]).
//!
//! # Region pipeline
//!
//! 1. `external_bridge_areas = bridge_areas − internal_bridge_areas`
//! 2. `supported_fill = prev_layer_boundary ∩ (top_solid_fill ∪
//!    bottom_solid_fill ∪ sparse_infill_area)`
//! 3. `anchor_band = supported_fill ∩ expand(external, anchor_depth)`
//! 4. `wave_domain = external ∪ anchor_band`, per connected external component
//! 5. Waves are **forced** — holder selection is the enable. Any generator
//!    fallback signal (missing anchors, empty seeds, min-length filtering,
//!    iteration residual, empty output) routes that component to conventional
//!    rectilinear bridge fill instead. No component is ever silently dropped.
//! 6. Waves are emitted as `BridgeInfill`, order-locked with **one tag per
//!    connected wave domain**, anchor-first. Internal-qualified bridge polygons
//!    get **unlocked** rectilinear fallback under today's role mapping.

#![warn(missing_docs)]
#![warn(unused_imports)]

mod generator;

use slicer_core::flow::{
    bridging_flow, canonical_bridging_flow, line_width_to_spacing, resolve_role_width,
    RoleWidthContext,
};
use slicer_core::polygon_ops::{
    clip_polylines, difference, intersection, offset, union, union_ex, OffsetJoinType,
};
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth,
    Polygon,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::config_resolution::resolve_float;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;
use slicer_sdk::OrderLockAllocator;

use generator::{Polyline, WaveOutput, WaveParams};

/// Lowest speed factor the G-code emitter can represent (`resolve_feedrate`).
const MIN_SPEED_FACTOR: f32 = 0.05;
/// Highest speed factor the G-code emitter can represent (`resolve_feedrate`).
const MAX_SPEED_FACTOR: f32 = 5.0;
/// Speed factor applied to conventional rectilinear fallback paths.
const FALLBACK_SPEED_FACTOR: f32 = 1.0;
/// Canonical automatic anchor-depth cap, in millimetres.
const AUTO_ANCHOR_DEPTH_CAP_MM: f32 = 3.0;

/// Module error code for an unrepresentable wave speed factor.
const ERR_SPEED_FACTOR: u32 = 1;
/// Module error code for order-lock tag exhaustion.
const ERR_ORDER_LOCK_EXHAUSTED: u32 = 2;

/// Wave contour ordering strategy.
///
/// Mirrors the fork's `wave_overhang_pattern` enum. Unknown strings resolve to
/// [`WavePattern::Smart`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WavePattern {
    /// Choose per-contour ordering heuristically (fork default).
    #[default]
    Smart,
    /// Emit contours in monotonically increasing order.
    Monotonic,
    /// Alternate contour direction on each iteration.
    Zigzag,
}

impl WavePattern {
    /// Parse a manifest/config string into a [`WavePattern`].
    ///
    /// Unrecognised values fall back to [`WavePattern::Smart`], matching the
    /// manifest default.
    #[must_use]
    pub fn from_str_or_default(raw: &str) -> Self {
        match raw {
            "monotonic" => Self::Monotonic,
            "zigzag" => Self::Zigzag,
            _ => Self::Smart,
        }
    }
}

/// Wave-overhang bridge-fill generator.
///
/// Every field is resolved once in `from_config`; per-region overrides of the
/// `wave_overhang_*` keys are re-resolved per region in `run_infill`.
pub struct WaveOverhangs {
    /// Contour ordering strategy.
    pattern: WavePattern,
    /// Spacing between successive wave contours, in millimetres.
    line_spacing: f32,
    /// Overlap of the first wave contour into the adjacent perimeter, in mm.
    perimeter_overlap: f32,
    /// Minimum bridge-area width that still receives wave fill, in mm.
    minimum_width: f32,
    /// Minimum newly-covered area required to keep iterating, in mm^2.
    min_new_area: f32,
    /// Minimum emitted contour length, in mm. Shorter contours are dropped.
    min_length: f32,
    /// Iteration cap; `0` means unbounded.
    max_iterations: u32,
    /// Volumetric flow for wave extrusions, in mm^3/mm.
    flow_mm3_per_mm: f32,
    /// Print speed for wave extrusions, in mm/s.
    print_speed: f32,
    /// Depth the wave anchors into surrounding solid material, in mm.
    anchor_depth_mm: f32,
    /// Host bridge print speed, in mm/s.
    bridge_speed: f32,
    /// Resolved bridge extrusion line width, in mm.
    bridge_line_width: f32,
    /// Bridge flow ratio.
    bridge_flow: f32,
    /// Bridge fill density as a fraction (1.0 == the manifest's `100%`).
    bridge_density: f32,
    /// Nozzle diameter, in mm.
    nozzle_diameter: f32,
    /// Wall (perimeter) count of the enclosing region.
    wall_loops: u32,
    /// Layer height, in mm.
    layer_height: f32,
    /// Whether the printer profile enables thick (round-thread) bridges.
    thick_bridges: bool,
}

impl WaveOverhangs {
    /// Returns the configured contour ordering strategy.
    #[must_use]
    pub fn pattern(&self) -> WavePattern {
        self.pattern
    }

    /// Returns the configured wave line spacing, in millimetres.
    #[must_use]
    pub fn line_spacing(&self) -> f32 {
        self.line_spacing
    }

    /// Returns the configured wave print speed, in mm/s.
    #[must_use]
    pub fn print_speed(&self) -> f32 {
        self.print_speed
    }

    /// Returns the configured iteration cap (`0` = unbounded).
    #[must_use]
    pub fn max_iterations(&self) -> u32 {
        self.max_iterations
    }
}

/// Read a float-ish config key, tolerating `Int` encodings.
fn cfg_float(config: &ConfigView, key: &str, fallback: f32) -> f32 {
    match config.get(key) {
        Some(ConfigValue::Float(v)) => *v as f32,
        Some(ConfigValue::Int(v)) => *v as f32,
        _ => fallback,
    }
}

/// Read a density-style key declared as `float_or_percent` in the manifest,
/// normalising to a fraction (`"100%"` and `1.0` both yield `1.0`).
fn cfg_density(config: &ConfigView, key: &str, fallback: f32) -> f32 {
    match config.get(key) {
        Some(ConfigValue::Percent(v)) => (*v / 100.0) as f32,
        Some(ConfigValue::FloatOrPercent { value, is_percent }) => {
            if *is_percent {
                (*value / 100.0) as f32
            } else {
                *value as f32
            }
        }
        Some(ConfigValue::Float(v)) => *v as f32,
        Some(ConfigValue::Int(v)) => *v as f32,
        _ => fallback,
    }
}

/// Read an int-ish config key, tolerating `Float` encodings.
fn cfg_u32(config: &ConfigView, key: &str, fallback: u32) -> u32 {
    match config.get(key) {
        Some(ConfigValue::Int(v)) => (*v).max(0) as u32,
        Some(ConfigValue::Float(v)) => v.max(0.0) as u32,
        _ => fallback,
    }
}

/// Read a bool config key.
fn cfg_bool(config: &ConfigView, key: &str, fallback: bool) -> bool {
    match config.get(key) {
        Some(ConfigValue::Bool(v)) => *v,
        _ => fallback,
    }
}

/// Read a string config key.
fn cfg_str<'cfg>(config: &'cfg ConfigView, key: &str, fallback: &'cfg str) -> &'cfg str {
    match config.get(key) {
        Some(ConfigValue::String(v)) => v.as_str(),
        _ => fallback,
    }
}

/// Per-region resolution of a `wave_overhang_*` integer key.
fn resolve_u32(region: &SliceRegionView, key: &str, fallback: u32) -> u32 {
    match region.config().and_then(|c| c.get(key)) {
        Some(ConfigValue::Int(v)) => (*v).max(0) as u32,
        Some(ConfigValue::Float(v)) => v.max(0.0) as u32,
        _ => fallback,
    }
}

/// Per-region resolution of a `wave_overhang_*` string key.
fn resolve_str(region: &SliceRegionView, key: &str, fallback: &str) -> String {
    match region.config().and_then(|c| c.get(key)) {
        Some(ConfigValue::String(v)) => v.clone(),
        _ => fallback.to_string(),
    }
}

/// Rotate a point about the origin by `(cos, sin)`.
#[inline]
fn rotate(p: Point2, cos: f64, sin: f64) -> Point2 {
    Point2 {
        x: (p.x as f64 * cos - p.y as f64 * sin).round() as i64,
        y: (p.x as f64 * sin + p.y as f64 * cos).round() as i64,
    }
}

/// Rotate a polygon set about the origin by `(cos, sin)`.
fn rotate_polys(polys: &[ExPolygon], cos: f64, sin: f64) -> Vec<ExPolygon> {
    polys
        .iter()
        .map(|exp| ExPolygon {
            contour: Polygon {
                points: exp
                    .contour
                    .points
                    .iter()
                    .map(|p| rotate(*p, cos, sin))
                    .collect(),
            },
            holes: exp
                .holes
                .iter()
                .map(|h| Polygon {
                    points: h.points.iter().map(|p| rotate(*p, cos, sin)).collect(),
                })
                .collect(),
        })
        .collect()
}

/// Conventional rectilinear bridge scanlines over `polys`.
///
/// Own copy of the rectilinear-infill precedent (ADR-0026 forbids sharing fill
/// implementations between modules). The scan direction comes from the host's
/// `bridge_orientation_deg`; this module never computes a bridge angle itself.
fn rectilinear_scanlines(polys: &[ExPolygon], angle_deg: f32, spacing_units: f64) -> Vec<Polyline> {
    if polys.is_empty() || spacing_units <= 0.0 {
        return Vec::new();
    }
    let angle = f64::from(angle_deg).to_radians();
    let (sin, cos) = angle.sin_cos();
    // Rotate into a frame where the scan direction is +X.
    let rotated = rotate_polys(polys, cos, -sin);

    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for exp in &rotated {
        for p in &exp.contour.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    if min_x > max_x || min_y > max_y {
        return Vec::new();
    }

    let mut lines: Vec<Polyline> = Vec::new();
    let mut y = min_y as f64 + spacing_units / 2.0;
    while y < max_y as f64 {
        let yi = y.round() as i64;
        lines.push(vec![
            Point2 {
                x: min_x - 1,
                y: yi,
            },
            Point2 {
                x: max_x + 1,
                y: yi,
            },
        ]);
        y += spacing_units;
    }
    if lines.is_empty() {
        return Vec::new();
    }

    let clipped = clip_polylines(&lines, &rotated);
    // Rotate back into the layer frame.
    clipped
        .into_iter()
        .map(|pl| pl.into_iter().map(|p| rotate(p, cos, sin)).collect())
        .filter(|pl: &Polyline| pl.len() >= 2)
        .collect()
}

/// Build an `ExtrusionPath3D` from a polyline in scaled units.
fn to_path(
    pl: &Polyline,
    z: f32,
    width: f32,
    flow_factor: f32,
    role: ExtrusionRole,
    speed_factor: f32,
    order_lock: Option<u64>,
) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: pl
            .iter()
            .map(|p| {
                let (x, y) = p.to_mm();
                Point3WithWidth {
                    x,
                    y,
                    z,
                    width,
                    flow_factor,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                }
            })
            .collect(),
        role,
        speed_factor,
        tool_index: None,
        order_lock,
    }
}

/// Reorder wave fronts so that the path touching supported material prints
/// first (AC-4 "anchor-first").
///
/// The generator already emits levels outward from the seed, which sits on the
/// anchor; this is a deterministic safety net for assembly strategies that may
/// reorder. Relative order of the remaining paths is preserved.
fn anchor_first(paths: &mut Vec<Polyline>, supported_fill: &[ExPolygon]) {
    if paths.len() < 2 || supported_fill.is_empty() {
        return;
    }
    let touches =
        |pl: &Polyline| !clip_polylines(std::slice::from_ref(pl), supported_fill).is_empty();
    if touches(&paths[0]) {
        return;
    }
    if let Some(idx) = paths.iter().position(touches) {
        let anchored = paths.remove(idx);
        paths.insert(0, anchored);
    }
}

#[slicer_module]
impl LayerModule for WaveOverhangs {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let pattern =
            WavePattern::from_str_or_default(cfg_str(config, "wave_overhang_pattern", "smart"));

        Ok(Self {
            pattern,
            line_spacing: cfg_float(config, "wave_overhang_line_spacing", 0.35),
            perimeter_overlap: cfg_float(config, "wave_overhang_perimeter_overlap", 0.1),
            minimum_width: cfg_float(config, "wave_overhang_minimum_width", 0.7),
            min_new_area: cfg_float(config, "wave_overhang_min_new_area", 0.01),
            min_length: cfg_float(config, "wave_overhang_min_length", 0.0),
            max_iterations: cfg_u32(config, "wave_overhang_max_iterations", 0),
            flow_mm3_per_mm: cfg_float(config, "wave_overhang_flow_mm3_per_mm", 0.15),
            print_speed: cfg_float(config, "wave_overhang_print_speed", 2.0),
            anchor_depth_mm: cfg_float(config, "wave_overhang_anchor_depth_mm", 0.0),
            bridge_speed: cfg_float(config, "bridge_speed", 25.0),
            bridge_line_width: cfg_float(config, "bridge_line_width", 0.0),
            bridge_flow: cfg_float(config, "bridge_flow", 1.0),
            bridge_density: cfg_density(config, "bridge_density", 1.0),
            nozzle_diameter: cfg_float(config, "nozzle_diameter", 0.4),
            wall_loops: cfg_u32(config, "wall_loops", 2),
            layer_height: cfg_float(config, "layer_height", 0.2),
            // Not a manifest key: printer profiles supply it, and its absence
            // means the flat-thread bridge model, matching the host default.
            thick_bridges: cfg_bool(config, "thick_bridges", false),
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
        let mut locks = OrderLockAllocator::new();

        for region in regions {
            output.begin_region(region.object_id(), *region.region_id());
            if !region.should_emit(ExtrusionRole::BridgeInfill) || region.bridge_areas().is_empty()
            {
                continue;
            }

            // ---- Speed factor (AC-7). Fatal, never a silent clamp. ----------
            let print_speed = resolve_float(region, "wave_overhang_print_speed", self.print_speed);
            let bridge_speed = resolve_float(region, "bridge_speed", self.bridge_speed);
            let speed_factor = if bridge_speed > 0.0 {
                print_speed / bridge_speed
            } else {
                f32::INFINITY
            };
            if !speed_factor.is_finite()
                || !(MIN_SPEED_FACTOR..=MAX_SPEED_FACTOR).contains(&speed_factor)
            {
                return Err(ModuleError::fatal(
                    ERR_SPEED_FACTOR,
                    format!(
                        "wave-overhangs: speed factor {speed_factor} \
                         (wave_overhang_print_speed {print_speed} mm/s / bridge_speed \
                         {bridge_speed} mm/s) is outside the representable range \
                         [{MIN_SPEED_FACTOR}, {MAX_SPEED_FACTOR}]"
                    ),
                ));
            }

            // ---- Flow / width (AC-7). Both are PER-POINT. -------------------
            let wave_width = self.nozzle_diameter;
            let layer_h = if region.effective_layer_height() > 0.0 {
                region.effective_layer_height()
            } else {
                self.layer_height
            };
            let wave_flow_key = resolve_float(
                region,
                "wave_overhang_flow_mm3_per_mm",
                self.flow_mm3_per_mm,
            );
            let wave_flow_factor = if wave_width > 0.0 && layer_h > 0.0 {
                wave_flow_key / (wave_width * layer_h)
            } else {
                1.0
            };

            // ---- Region partition (steps 1-4). ------------------------------
            let external = difference(region.bridge_areas(), region.internal_bridge_areas());
            let internal_qualified =
                intersection(region.bridge_areas(), region.internal_bridge_areas());

            // The packet docs call this `prev_object_boundary`; the accessor
            // that actually exists is `prev_layer_boundary()` (object-scoped
            // since packet 243). Naming drift in the docs, not here.
            let solid_and_sparse = union(
                &union(region.top_solid_fill(), region.bottom_solid_fill()),
                region.sparse_infill_area(),
            );
            let supported_fill = intersection(region.prev_layer_boundary(), &solid_and_sparse);

            let bridge_spacing_mm = canonical_bridging_flow(
                self.bridge_line_width,
                self.bridge_flow,
                self.nozzle_diameter,
            )
            .spacing_mm;
            let anchor_depth_cfg = resolve_float(
                region,
                "wave_overhang_anchor_depth_mm",
                self.anchor_depth_mm,
            );
            // Reproduces the generator's own canonical `anchors_size`
            // (`EXTERNAL_INFILL_MARGIN_MM.min(base_spacing * (wall_loops + 1))`).
            let anchors_size_mm =
                AUTO_ANCHOR_DEPTH_CAP_MM.min(bridge_spacing_mm * (self.wall_loops as f32 + 1.0));
            let anchor_depth = if anchor_depth_cfg > 0.0 {
                anchor_depth_cfg
            } else {
                // WHY the floor: the auto depth sets how far the anchor band
                // reaches into the supported fill, and the generator then
                // erodes the anchors by canonical `anchors_size` to build
                // `inset_anchors`. An auto depth equal to `anchors_size` --
                // which is exactly what the uncapped formula yields, since it
                // is the same expression -- leaves `inset_anchors` empty, so
                // seed generation finds no anchored start and every component
                // falls back to conventional rectilinear bridge fill (zero
                // waves out of the box). Flooring at
                // `anchors_size + base_spacing` guarantees at least one
                // base-spacing-wide ring of anchor survives the erosion.
                // `bridge_spacing_mm` is strictly positive, so the floor is
                // unconditionally the deeper of the two and is used directly.
                anchors_size_mm + bridge_spacing_mm
            };

            // ---- Generator parameters (per-region overrides applied). -------
            let params = WaveParams {
                line_spacing: resolve_float(
                    region,
                    "wave_overhang_line_spacing",
                    self.line_spacing,
                ),
                perimeter_overlap: resolve_float(
                    region,
                    "wave_overhang_perimeter_overlap",
                    self.perimeter_overlap,
                ),
                minimum_width: resolve_float(
                    region,
                    "wave_overhang_minimum_width",
                    self.minimum_width,
                ),
                min_new_area: resolve_float(
                    region,
                    "wave_overhang_min_new_area",
                    self.min_new_area,
                ),
                min_length: resolve_float(region, "wave_overhang_min_length", self.min_length),
                max_iterations: resolve_u32(
                    region,
                    "wave_overhang_max_iterations",
                    self.max_iterations,
                ),
                pattern: WavePattern::from_str_or_default(&resolve_str(
                    region,
                    "wave_overhang_pattern",
                    match self.pattern {
                        WavePattern::Smart => "smart",
                        WavePattern::Monotonic => "monotonic",
                        WavePattern::Zigzag => "zigzag",
                    },
                )),
                flow_width: wave_width,
                // The same `flow_factor` stamped on every emitted point below,
                // so the generator's trim inset and the emitter's deposited
                // bead width cannot drift apart.
                flow_ratio: wave_flow_factor,
                // FLOW-derived spacing, deliberately distinct from
                // `line_spacing` (canonical `overhang_flow.scaled_spacing()`).
                base_spacing: bridge_spacing_mm,
                wall_loops: self.wall_loops,
            };

            // ---- Conventional bridge fill parameters (fallback). ------------
            let width_ctx = RoleWidthContext {
                line_width: 0.0,
                nozzle_diameter: self.nozzle_diameter,
                bridge_line_width: self.bridge_line_width,
                initial_layer_line_width: 0.0,
                outer_wall_line_width: 0.0,
                inner_wall_line_width: 0.0,
                top_surface_line_width: 0.0,
                internal_solid_infill_line_width: 0.0,
                sparse_infill_line_width: 0.0,
            };
            let bridge_width = resolve_role_width(
                ExtrusionRole::BridgeInfill,
                layer_index == 0,
                true,
                &width_ctx,
            );
            let thread_base_width = if self.bridge_line_width > 0.0 {
                bridge_width
            } else {
                self.nozzle_diameter
            };
            let base_spacing_mm = if self.thick_bridges {
                canonical_bridging_flow(
                    self.bridge_line_width,
                    self.bridge_flow,
                    self.nozzle_diameter,
                )
                .spacing_mm
            } else {
                line_width_to_spacing(bridge_width, layer_h).unwrap_or(bridge_width)
            };
            let fallback_spacing_units = if self.bridge_density > 0.0 {
                generator::units(base_spacing_mm / self.bridge_density)
            } else {
                0.0
            };
            let fallback_flow = bridging_flow(
                self.bridge_flow,
                self.thick_bridges,
                thread_base_width,
                bridge_width,
                layer_h,
            );
            let fallback_role = if region.is_internal_bridge() {
                ExtrusionRole::InternalBridgeInfill
            } else {
                ExtrusionRole::BridgeInfill
            };
            let angle = region.bridge_orientation_deg();
            let z = region.z();

            // ---- Per connected external component. --------------------------
            for component in union_ex(&external) {
                let component_slice = std::slice::from_ref(&component);
                let band = intersection(
                    &supported_fill,
                    &offset(component_slice, anchor_depth, OffsetJoinType::Round, 0.0),
                );
                let wave_domain = union(component_slice, &band);

                let WaveOutput {
                    mut paths,
                    filled: _,
                    fallbacks,
                } = generator::generate(&wave_domain, &supported_fill, &params);
                // Any fallback signal forces conventional bridge fill for this
                // component; the reasons are kept on `WaveOutput` for tests and
                // future diagnostics.

                let waves_usable = fallbacks.is_empty() && !paths.is_empty();
                if waves_usable {
                    anchor_first(&mut paths, &supported_fill);
                    // One order-lock tag per connected wave domain: the whole
                    // wave prints as a single unbreakable ordered run.
                    let tag = locks.allocate().ok_or_else(|| {
                        ModuleError::fatal(
                            ERR_ORDER_LOCK_EXHAUSTED,
                            "wave-overhangs: order-lock tag space exhausted",
                        )
                    })?;
                    for pl in &paths {
                        output
                            .push_solid_path(to_path(
                                pl,
                                z,
                                wave_width,
                                wave_flow_factor,
                                ExtrusionRole::BridgeInfill,
                                speed_factor,
                                Some(tag),
                            ))
                            .map_err(ModuleError::from)?;
                    }
                    continue;
                }

                // Fallback: conventional rectilinear bridge fill, unlocked.
                // Every nonempty external component reaches this branch when
                // waves are impossible, so no component is silently dropped.
                for pl in rectilinear_scanlines(component_slice, angle, fallback_spacing_units) {
                    output
                        .push_solid_path(to_path(
                            &pl,
                            z,
                            bridge_width,
                            fallback_flow,
                            fallback_role.clone(),
                            FALLBACK_SPEED_FACTOR,
                            None,
                        ))
                        .map_err(ModuleError::from)?;
                }
            }

            // ---- Internal-qualified polygons: unlocked rectilinear. ---------
            for pl in rectilinear_scanlines(&internal_qualified, angle, fallback_spacing_units) {
                output
                    .push_solid_path(to_path(
                        &pl,
                        z,
                        bridge_width,
                        fallback_flow,
                        fallback_role.clone(),
                        FALLBACK_SPEED_FACTOR,
                        None,
                    ))
                    .map_err(ModuleError::from)?;
            }
        }
        Ok(())
    }
}
