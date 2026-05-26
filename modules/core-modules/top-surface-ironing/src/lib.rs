//! Top-surface ironing module (rev 0.2 — Layer::Infill).
//!
//! After the slicing-promotion refactor, top/bottom surface classification
//! lives in `PrePass::ShellClassification`. Every `SliceRegionView` carries
//! `top_shell_index: Option<u8>` plus polygon-precise `top_solid_fill`
//! polygons. This module now runs in `Layer::Infill` and emits low-flow
//! ironing strokes clipped to `top_solid_fill` for each region with
//! `top_shell_index() == Some(0)` (exposed top surface). Holds `claim:ironing`
//! so the dedup pass does not consider it a duplicate of the surviving
//! infill module (`claim:top-fill` / `claim:bottom-fill` / etc.).
//!
//! Coordinate system reminder: `ExtrusionPath3D.points` carry mm `f32`
//! coordinates while `ExPolygon` contour points are integer 100 nm units.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_ir::{
    units_to_mm, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole,
    Point3WithWidth,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

/// Base speed (mm/s) used to normalise the ironing speed into a
/// `speed_factor` multiplier on the emitted `ExtrusionPath3D`.
const BASE_SPEED: f64 = 50.0;

/// Default ironing line width (mm).
const IRONING_LINE_WIDTH: f32 = 0.4;

/// Top-surface ironing module.
///
/// Implements `LayerModule::run_infill`. Per-region self-gates on
/// `top_shell_index() == Some(0)` (exposed top) and `top_solid_fill` being
/// non-empty. Generates a single connected zigzag polyline per top-solid
/// ExPolygon contour, clipped to the polygon's axis-aligned bounding box.
#[derive(Debug)]
pub struct TopSurfaceIroning {
    enabled: bool,
    ironing_speed: f64,
    ironing_flow: f64,
    ironing_spacing_mm: f64,
    ironing_pattern: String,
}

impl TopSurfaceIroning {
    /// Whether ironing is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Ironing speed in mm/s.
    pub fn ironing_speed(&self) -> f64 {
        self.ironing_speed
    }

    /// Ironing flow multiplier.
    pub fn ironing_flow(&self) -> f64 {
        self.ironing_flow
    }

    /// Ironing line spacing in mm.
    pub fn ironing_spacing_mm(&self) -> f64 {
        self.ironing_spacing_mm
    }

    /// Ironing pattern name.
    pub fn ironing_pattern(&self) -> &str {
        &self.ironing_pattern
    }
}

/// Axis-aligned bounding box in mm.
#[derive(Debug, Clone, Copy)]
struct BBox2D {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}

/// Compute the axis-aligned bounding box of an `ExPolygon`'s contour, in mm.
fn bbox_of_expoly(poly: &ExPolygon) -> Option<BBox2D> {
    let pts = &poly.contour.points;
    if pts.is_empty() {
        return None;
    }
    let mut bb = BBox2D {
        x_min: units_to_mm(pts[0].x),
        y_min: units_to_mm(pts[0].y),
        x_max: units_to_mm(pts[0].x),
        y_max: units_to_mm(pts[0].y),
    };
    for p in pts.iter().skip(1) {
        let x = units_to_mm(p.x);
        let y = units_to_mm(p.y);
        bb.x_min = bb.x_min.min(x);
        bb.y_min = bb.y_min.min(y);
        bb.x_max = bb.x_max.max(x);
        bb.y_max = bb.y_max.max(y);
    }
    Some(bb)
}

/// Tolerance (mm) used by stroke-endpoint containment checks. Equal to one
/// slicer integer-coordinate unit — points within ±0.0001 mm of a polygon
/// edge count as inside, preventing the f32 even-odd test's flip on
/// stroke endpoints that coincide with the contour.
const STROKE_CONTAIN_EPS_MM: f64 = 0.001;

/// Containment predicate used during stroke clipping. Delegates to the
/// shared `slicer_ir::point_in_polygon_winding` (f64 winding-number with
/// edge tolerance) so behaviour stays consistent across modules.
fn point_in_polygon_mm(poly: &ExPolygon, px: f32, py: f32) -> bool {
    slicer_ir::point_in_polygon_winding(poly, px as f64, py as f64, STROKE_CONTAIN_EPS_MM)
}

/// Generate a rectilinear (horizontal-zigzag, snake) ironing polyline over
/// `bb` at z, returning a single connected `ExtrusionPath3D` whose vertices
/// fall inside `poly`. Endpoints that lie outside the polygon contour are
/// clipped — for a convex top-fill polygon this trims neatly to the
/// polygon outline.
fn generate_zigzag_strokes_for_polygon(
    poly: &ExPolygon,
    bb: &BBox2D,
    z: f32,
    spacing_mm: f64,
    flow_factor: f32,
    speed_factor: f32,
) -> Vec<ExtrusionPath3D> {
    let spacing = spacing_mm as f32;
    if spacing <= 0.0 || bb.x_max <= bb.x_min || bb.y_max <= bb.y_min {
        return Vec::new();
    }

    let span = bb.y_max - bb.y_min;
    let n = ((span / spacing).floor() as usize).saturating_add(1);
    if n == 0 {
        return Vec::new();
    }

    let mk = |x: f32, y: f32| Point3WithWidth {
        x,
        y,
        z,
        width: IRONING_LINE_WIDTH,
        flow_factor,
        overhang_quartile: None,
    };

    let mut points: Vec<Point3WithWidth> = Vec::with_capacity(n * 2);
    for i in 0..n {
        let y = (bb.y_min + (i as f32) * spacing).min(bb.y_max);
        let (mut x_start, mut x_end) = if i % 2 == 0 {
            (bb.x_min, bb.x_max)
        } else {
            (bb.x_max, bb.x_min)
        };
        // Clip stroke endpoints to the polygon. Walk inward in 0.05mm steps
        // until both endpoints lie inside; bail out if the row is fully
        // outside the polygon.
        let step = 0.05_f32;
        let dir = (x_end - x_start).signum();
        if dir == 0.0 {
            continue;
        }
        let mut clipped_start = x_start;
        while !point_in_polygon_mm(poly, clipped_start, y) {
            clipped_start += dir * step;
            if (clipped_start - x_end).abs() <= step {
                break;
            }
        }
        let mut clipped_end = x_end;
        while !point_in_polygon_mm(poly, clipped_end, y) {
            clipped_end -= dir * step;
            if (clipped_start - clipped_end).abs() <= step {
                break;
            }
        }
        if (clipped_start - clipped_end).abs() <= step {
            continue;
        }
        x_start = clipped_start;
        x_end = clipped_end;
        points.push(mk(x_start, y));
        points.push(mk(x_end, y));
    }

    if points.len() < 2 {
        return Vec::new();
    }

    vec![ExtrusionPath3D {
        points,
        role: ExtrusionRole::Ironing,
        speed_factor,
    }]
}

#[slicer_module]
impl LayerModule for TopSurfaceIroning {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("ironing_enabled") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => true,
        };

        let ironing_speed = match config.get("ironing_speed") {
            Some(ConfigValue::Float(s)) => *s,
            Some(ConfigValue::Int(s)) => *s as f64,
            _ => 20.0,
        };

        let ironing_flow = match config.get("ironing_flow") {
            Some(ConfigValue::Float(f)) => *f,
            Some(ConfigValue::Int(f)) => *f as f64,
            _ => 0.10,
        };

        if ironing_flow <= 0.0 {
            return Err(ModuleError::fatal(
                1,
                "ironing_flow must be greater than 0.0 (key: ironing_flow)",
            ));
        }

        let ironing_spacing_mm = match config.get("ironing_spacing_mm") {
            Some(ConfigValue::Float(s)) => *s,
            Some(ConfigValue::Int(s)) => *s as f64,
            _ => 0.1,
        };

        let ironing_pattern = match config.get("ironing_pattern") {
            Some(ConfigValue::String(p)) => {
                if p != "rectilinear" {
                    return Err(ModuleError::fatal(
                        2,
                        format!(
                            "unsupported ironing_pattern '{}'; only 'rectilinear' is supported \
                             (key: ironing_pattern)",
                            p
                        ),
                    ));
                }
                p.clone()
            }
            _ => "rectilinear".to_string(),
        };

        Ok(Self {
            enabled,
            ironing_speed,
            ironing_flow,
            ironing_spacing_mm,
            ironing_pattern,
        })
    }

    fn run_infill(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        let speed_factor = (self.ironing_speed / BASE_SPEED) as f32;
        let flow_factor = self.ironing_flow as f32;

        for region in regions {
            // Self-gate per region: only the topmost exposed surface gets
            // ironed (depth 0). Deeper top-shell layers and the bottom-shell
            // zone are skipped.
            if region.top_shell_index() != Some(0) {
                continue;
            }
            let top_fill = region.top_solid_fill();
            if top_fill.is_empty() {
                continue;
            }
            let z = region.z();
            for poly in top_fill {
                let bb = match bbox_of_expoly(poly) {
                    Some(b) => b,
                    None => continue,
                };
                let paths = generate_zigzag_strokes_for_polygon(
                    poly,
                    &bb,
                    z,
                    self.ironing_spacing_mm,
                    flow_factor,
                    speed_factor,
                );
                for path in paths {
                    output
                        .push_ironing_path(path)
                        .map_err(|e| ModuleError::fatal(3, e))?;
                }
            }
        }

        Ok(())
    }
}
