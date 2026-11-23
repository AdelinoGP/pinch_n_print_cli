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
    units_to_mm, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, Polygon,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
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
/// non-empty. Generates connected zigzag polylines over the top-solid
/// ExPolygon, clipped to the polygon's axis-aligned bounding box. A path
/// continues across rows and between a row's intervals only while the joining
/// move stays inside the region, so no emitted segment is extruded across a
/// hole, a notch, or a slanted boundary that leaves the material; a move that
/// would leave the region starts a new path instead.
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

/// Tolerance for deciding that a probe point lies on a polygon edge, in mm.
///
/// One canonical unit (100 nm). Joins between scan rows are constructed from
/// grid-aligned endpoints, so a point that should be exactly on an edge is
/// within one unit after `f32` interpolation for top-surface coordinates up to
/// a few hundred millimetres; beyond that the `f32` ulp exceeds the tolerance
/// and the test fails conservatively (a spurious split, never an out-of-region
/// join). Real features are orders of magnitude larger than the tolerance.
const ON_EDGE_EPS_MM: f64 = 1.0e-4;

/// True when `(x, y)` lies on any edge of `poly` (contour or hole), within
/// [`ON_EDGE_EPS_MM`].
fn on_boundary(poly: &ExPolygon, x: f32, y: f32) -> bool {
    let (x, y) = (f64::from(x), f64::from(y));
    for ring in std::iter::once(&poly.contour).chain(poly.holes.iter()) {
        let points = &ring.points;
        if points.len() < 2 {
            continue;
        }
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            let (px, py) = (
                f64::from(units_to_mm(points[i].x)),
                f64::from(units_to_mm(points[i].y)),
            );
            let (qx, qy) = (
                f64::from(units_to_mm(points[j].x)),
                f64::from(units_to_mm(points[j].y)),
            );
            let (dx, dy) = (qx - px, qy - py);
            let length = (dx * dx + dy * dy).sqrt();
            if length < ON_EDGE_EPS_MM {
                continue;
            }
            let distance = ((x - px) * dy - (y - py) * dx).abs() / length;
            if distance > ON_EDGE_EPS_MM {
                continue;
            }
            if x >= px.min(qx) - ON_EDGE_EPS_MM
                && x <= px.max(qx) + ON_EDGE_EPS_MM
                && y >= py.min(qy) - ON_EDGE_EPS_MM
                && y <= py.max(qy) + ON_EDGE_EPS_MM
            {
                return true;
            }
        }
    }
    false
}

/// Even-odd point-in-region test with the boundary counted as inside.
fn point_in_expolygon(poly: &ExPolygon, x: f32, y: f32) -> bool {
    if on_boundary(poly, x, y) {
        return true;
    }
    let mut xs: Vec<f32> = Vec::new();
    collect_x_crossings(&poly.contour, y, &mut xs);
    for hole in &poly.holes {
        collect_x_crossings(hole, y, &mut xs);
    }
    xs.iter().filter(|crossing| **crossing < x).count() % 2 == 1
}

/// Parameter `t` in `[0, 1]` where segment `a -> b` meets segment `p -> q`,
/// or `None` when they are parallel or the meeting lies outside either span.
fn segment_meeting_parameter(
    a: (f32, f32),
    b: (f32, f32),
    p: (f32, f32),
    q: (f32, f32),
) -> Option<f64> {
    let (ax, ay) = (f64::from(a.0), f64::from(a.1));
    let (bx, by) = (f64::from(b.0), f64::from(b.1));
    let (px, py) = (f64::from(p.0), f64::from(p.1));
    let (qx, qy) = (f64::from(q.0), f64::from(q.1));
    let (dx, dy) = (bx - ax, by - ay);
    let (ex, ey) = (qx - px, qy - py);
    let denominator = dx * ey - dy * ex;
    if denominator.abs() < 1e-12 {
        return None;
    }
    let t = ((px - ax) * ey - (py - ay) * ex) / denominator;
    let u = ((px - ax) * dy - (py - ay) * dx) / denominator;
    if (-1e-9..=1.0 + 1e-9).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u) {
        Some(t.clamp(0.0, 1.0))
    } else {
        None
    }
}

/// True when the whole open segment `a -> b` stays inside `poly`, boundary
/// included.
///
/// The segment is split at every edge meeting parameter and each open
/// sub-interval's midpoint must be inside the region. A join that leaves the
/// region — across a hole, a notch, or a slanted boundary — therefore returns
/// `false`, and the caller starts a new path instead of extruding it.
fn segment_stays_inside(poly: &ExPolygon, a: (f32, f32), b: (f32, f32)) -> bool {
    let mut parameters = vec![0.0_f64, 1.0_f64];
    for ring in std::iter::once(&poly.contour).chain(poly.holes.iter()) {
        let points = &ring.points;
        if points.len() < 2 {
            continue;
        }
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            let p = (units_to_mm(points[i].x), units_to_mm(points[i].y));
            let q = (units_to_mm(points[j].x), units_to_mm(points[j].y));
            if let Some(t) = segment_meeting_parameter(a, b, p, q) {
                parameters.push(t);
            }
        }
    }
    parameters.sort_by(f64::total_cmp);
    for pair in parameters.windows(2) {
        if pair[1] - pair[0] <= 1e-9 {
            continue;
        }
        let middle = 0.5 * (pair[0] + pair[1]);
        let x = a.0 + (b.0 - a.0) * middle as f32;
        let y = a.1 + (b.1 - a.1) * middle as f32;
        if !point_in_expolygon(poly, x, y) {
            return false;
        }
    }
    true
}

/// Generate rectilinear (horizontal-zigzag, snake) ironing paths over the
/// `poly` ExPolygon at z. For each scan row at spacing `spacing_mm`, the
/// algorithm:
///
/// 1. Collects edge x-crossings on the row (contour + every hole).
/// 2. Sorts ascending and pairs into `[x_{2k}, x_{2k+1}]` interior intervals.
/// 3. Emits one stroke per interval, alternating row direction so the path
///    stays a connected snake within each fill block.
/// 4. Starts a new path whenever the implicit move from the last emitted point
///    to the next interval's start would leave the region (`segment_stays_inside`).
///    For axis-aligned fixtures that move is a vertical run on the outer
///    boundary and the snake is preserved; a hole or notch gap, and a slanted
///    boundary that cuts the move, both start a new path, so no segment is
///    extruded outside the material.
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
        dist_to_top_mm: 0.0,
        overhang_distance_mm: None,
    };

    let flush = |paths: &mut Vec<ExtrusionPath3D>, points: &mut Vec<Point3WithWidth>| {
        if points.len() >= 2 {
            paths.push(ExtrusionPath3D {
                points: std::mem::take(points),
                role: ExtrusionRole::Ironing,
                speed_factor,
                tool_index: None,
                order_lock: None,
            });
        } else {
            points.clear();
        }
    };

    let mut paths: Vec<ExtrusionPath3D> = Vec::new();
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
            // Continue the snake only when the move from the previous emitted
            // point to this interval's start stays inside the region. For the
            // axis-aligned fixtures that move is a vertical run on the outer
            // boundary between rows and a zero-length move between adjacent
            // intervals; for a slanted or concave boundary it can leave the
            // material, in which case the interval starts its own path instead
            // of extruding an out-of-region join.
            let start = mk(xa, y);
            let continues = points.last().is_none_or(|last| {
                segment_stays_inside(poly, (last.x, last.y), (start.x, start.y))
            });
            if !continues {
                flush(&mut paths, &mut points);
            }
            points.push(start);
            points.push(mk(xb, y));
        }
    }
    flush(&mut paths, &mut points);

    paths
}

#[slicer_module]
impl LayerModule for TopSurfaceIroning {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        // Required reads (config-scope-resolution plan): every key below is
        // declared in `top-surface-ironing.toml`, so the bound view always
        // holds its registry default. A missing key is a contract violation,
        // not a fallback — `require_*` failures convert to fatal `ModuleError`s.
        //
        // `ironing_enabled` defaults OFF to match OrcaSlicer
        // (`ironing_type = no ironing`); users opt in explicitly.
        let enabled = config.require_bool("ironing_enabled")?;

        let ironing_speed = config.require_float("ironing_speed")?;

        // Canonical default: coPercent 10 (10 = 10%) —
        // `PrintConfigDef::init_fff_params`
        // (`OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`).
        let ironing_flow = config.require_float("ironing_flow")?;

        if ironing_flow <= 0.0 {
            return Err(ModuleError::fatal(
                1,
                "ironing_flow must be greater than 0.0 (key: ironing_flow)",
            ));
        }

        let ironing_spacing_mm = config.require_float("ironing_spacing_mm")?;

        let ironing_pattern = config.require_string("ironing_pattern")?;
        if ironing_pattern != "rectilinear" {
            return Err(ModuleError::fatal(
                2,
                format!(
                    "unsupported ironing_pattern '{}'; only 'rectilinear' is supported \
                     (key: ironing_pattern)",
                    ironing_pattern
                ),
            ));
        }
        let ironing_pattern = ironing_pattern.to_string();

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
    /// **Note on IR access:** the manifest declares `reads = ["SliceIR"]` (it
    /// writes `InfillIR`) and orders this module after gyroid/rectilinear/
    /// lightning at `Layer::Infill` via `[compatibility].requires`
    /// (ExplicitRequires DAG edges), resolving what was previously an advisory
    /// `WriteConflict { orderable: false }`. This impl never reads from
    /// `InfillIR` at runtime — the ironing polygon comes from
    /// `SliceRegionView::top_solid_fill()`.
    fn run_infill(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        let speed_factor = (self.ironing_speed / BASE_SPEED) as f32;
        // `ironing_flow` is percent magnitude (coPercent 0..100); the ÷100
        // happens at consumption, exactly canonical
        // `ConfigOptionPercent::get_abs_value`
        // (`OrcaSlicerDocumented/src/libslic3r/Config.hpp`).
        let flow_factor = self.ironing_flow as f32 / 100.0;

        for region in regions {
            output.begin_region(
                region.object_id(),
                *region.region_id(),
                region.variant_chain(),
            );
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

// Out-of-line property module. The two separate gates are load-bearing: the
// default build never compiles it, and the feature alone (without a test
// build) cannot pull Proptest or the property engine into a normal graph.
// `cargo xtask check-literals` does not follow this declaration — see the
// file's own literal-review note.
#[cfg(test)]
#[cfg(feature = "property_testing")]
mod property_tests;
