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
    Point3WithWidth, Polygon,
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

/// Push the x-coordinates (mm) where the horizontal scan line `y_mm` crosses
/// the edges of `poly`. Uses the half-open `[y_lo, y_hi)` rule so vertices
/// shared between two non-horizontal edges contribute exactly one crossing
/// (the lower endpoint of the upper edge), yielding consistent even parity.
/// Horizontal edges are skipped — adjacent non-horizontal edges already
/// account for them.
///
/// Mirrors OrcaSlicer's even-odd scan-line classification used by
/// `FillRectilinear::fill_surface_by_lines` (libslic3r/Fill/FillRectilinear.cpp).
fn collect_x_crossings(poly: &Polygon, y_mm: f32, xs: &mut Vec<f32>) {
    let pts = &poly.points;
    let n = pts.len();
    if n < 3 {
        return;
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let yi = units_to_mm(pts[i].y);
        let yj = units_to_mm(pts[j].y);
        let (y_lo, y_hi) = if yi <= yj { (yi, yj) } else { (yj, yi) };
        if (yj - yi).abs() < 1e-9 {
            // Horizontal edge — skip.
            continue;
        }
        if y_mm < y_lo || y_mm >= y_hi {
            continue;
        }
        let xi = units_to_mm(pts[i].x);
        let xj = units_to_mm(pts[j].x);
        let x = xi + (xj - xi) * (y_mm - yi) / (yj - yi);
        xs.push(x);
    }
}

/// Generate a rectilinear (horizontal-zigzag, snake) ironing path over the
/// `poly` ExPolygon at z. For each scan row at spacing `spacing_mm`, the
/// algorithm:
///
/// 1. Collects edge x-crossings on the row (contour + every hole).
/// 2. Sorts ascending and pairs into `[x_{2k}, x_{2k+1}]` interior intervals.
/// 3. Emits one stroke per interval, alternating row direction so the path
///    stays a connected snake within each fill block.
///
/// Cost is O(P) per row (P = vertex count) instead of the old per-row
/// O(span / 0.05 · P) walk-inward clipping — which trapped on benchy
/// layer 59's top-fill polygon. For non-convex polygons (notches, holes)
/// each row correctly emits multiple disjoint strokes that never traverse
/// the gaps.
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
    let n_rows = ((span / spacing).floor() as usize).saturating_add(1);

    let mk = |x: f32, y: f32| Point3WithWidth {
        x,
        y,
        z,
        width: IRONING_LINE_WIDTH,
        flow_factor,
        overhang_quartile: None,
    };

    let mut points: Vec<Point3WithWidth> = Vec::new();
    let mut xs: Vec<f32> = Vec::new();
    for i in 0..n_rows {
        let y = (bb.y_min + (i as f32) * spacing).min(bb.y_max);

        xs.clear();
        collect_x_crossings(&poly.contour, y, &mut xs);
        for hole in &poly.holes {
            collect_x_crossings(hole, y, &mut xs);
        }
        if xs.len() < 2 {
            continue;
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Odd crossings indicate a vertex-grazing degeneracy; drop the last
        // one rather than emit an unmatched start/end. Even-odd pairing then
        // yields valid [x_{2k}, x_{2k+1}] interior intervals.
        let pair_count = xs.len() / 2;
        let reverse_row = i % 2 == 1;
        for k in 0..pair_count {
            let idx = if reverse_row { pair_count - 1 - k } else { k };
            let a = xs[2 * idx];
            let b = xs[2 * idx + 1];
            if (b - a) <= 1e-4 {
                continue;
            }
            let (xa, xb) = if reverse_row { (b, a) } else { (a, b) };
            points.push(mk(xa, y));
            points.push(mk(xb, y));
        }
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
            // Default OFF to match OrcaSlicer (`ironing_type = no ironing`).
            // Ironing at 0.1 mm spacing over every top surface roughly doubles
            // top-surface emission; users opt in explicitly.
            _ => false,
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

    /// Emits ironing strokes over polygon-precise top solid fill areas.
    ///
    /// **Note on IR access:** the manifest declares `reads = ["SliceIR", "InfillIR"]`
    /// for **DAG-ordering purposes only** (so the validator orders this module
    /// after gyroid/rectilinear/lightning at `Layer::Infill`, resolving what was
    /// previously an advisory `WriteConflict { orderable: false }`). This impl
    /// never reads from `InfillIR` at runtime — the ironing polygon comes from
    /// `SliceRegionView::top_solid_fill()`. See DEV-065 (2026-06-09).
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
            output.begin_region(region.object_id(), *region.region_id());
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
