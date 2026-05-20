//! Tree-style branching support generator module.
//!
//! Implements `LayerModule::run_support` for the `Layer::Support` stage.
//! Generates branching polyline structures instead of traditional grid fills.
//! Branches converge toward fewer build-plate contact points, using less material.
//!
//! Algorithm (single-layer simplified tree support):
//! 1. Sample support polygon interior points on a grid (spacing from density)
//! 2. Build a nearest-neighbor tree connecting sample points from centroid
//! 3. Generate branch paths from tree edges
//! 4. Convert to ExtrusionPath3D with SupportMaterial role

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_core::paint_region::{point_in_paint_region, BoundaryInclusion};
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, PaintRegionIR,
    PaintSemantic, Point2, Point3WithWidth,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Default base speed used for normalizing speed factors (mm/s).
const BASE_SPEED: f32 = 50.0;

/// Cap on interior sample points per ExPolygon per layer. The tree MST
/// builder is O(n²) Prim (same algorithmic class as OrcaSlicer's
/// `MinimumSpanningTree::prim`), so grid refinements that push `n` into
/// the millions produce effectively unbounded work. If the density-derived
/// grid would exceed this cap, the spacing is widened so that the sample
/// count stays ≤ `MAX_SAMPLES_PER_EXPOLY`. Deterministic and coverage-
/// preserving — no random downsampling.
const MAX_SAMPLES_PER_EXPOLY: f64 = 2000.0;

/// Tree support branching generator.
///
/// Produces tree-like branching fill patterns for support material areas.
/// Branches converge toward fewer contact points, reducing material usage
/// compared to traditional rectilinear support.
pub struct TreeSupport {
    /// Whether support generation is enabled.
    enabled: bool,
    /// Support density (0.0 to 1.0).
    density: f32,
    /// Base support angle in degrees (reserved for future use).
    #[allow(dead_code)]
    base_angle: f32,
    /// Support print speed in mm/s.
    support_speed: f32,
    /// Extrusion line width in millimeters.
    line_width: f32,
}

impl TreeSupport {
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
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("support_enabled") {
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

        Ok(Self {
            enabled,
            density,
            base_angle,
            support_speed,
            line_width,
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

        let paint_ir = paint.paint_regions();

        for region in regions {
            let polygons = region.polygons();
            if polygons.is_empty() {
                continue;
            }

            let z = region.z();

            // Planner-consuming tier (TASK-161): when a `SupportPlanIR`
            // is committed on the blackboard and carries an entry for
            // this `(layer, object, region)` triple, emit its pre-planned
            // branch geometry directly and skip the per-layer grid-MST
            // fill. The grid-MST filler remains the fallback path when
            // no planner module is installed.
            let planned_segments =
                paint.support_plan_segments_for(region.object_id().as_str(), *region.region_id());
            if !planned_segments.is_empty() {
                for segment in planned_segments {
                    let mut path = segment.clone();
                    path.role = ExtrusionRole::SupportMaterial;
                    path.speed_factor = speed_factor;
                    let _ = output.push_support_path(path);
                }
                continue;
            }

            for expoly in polygons {
                // Eligibility precedence (docs/01 Layer::Support, docs/02
                // support precedence rules):
                //   blocker → skip; enforcer → generate;
                //   default → consult SurfaceClassificationIR.needs_support.
                match support_paint_policy(paint_ir, layer_index, expoly) {
                    SupportPaintPolicy::Blocked => continue,
                    SupportPaintPolicy::Enforced => {}
                    SupportPaintPolicy::DefaultEligible => {
                        if !region.needs_support() {
                            continue;
                        }
                    }
                }

                let paths = self.fill_expolygon_tree(expoly, z, speed_factor);
                for path in paths {
                    let _ = output.push_support_path(path);
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupportPaintPolicy {
    Blocked,
    Enforced,
    DefaultEligible,
}

fn support_paint_policy(
    paint_ir: &PaintRegionIR,
    layer_index: u32,
    expoly: &ExPolygon,
) -> SupportPaintPolicy {
    let centroid = expolygon_centroid(expoly);

    let is_blocked = point_in_paint_region(
        paint_ir,
        layer_index,
        &PaintSemantic::SupportBlocker,
        centroid,
        BoundaryInclusion::Include,
        None,
    )
    .ok()
    .flatten()
    .is_some();

    if is_blocked {
        return SupportPaintPolicy::Blocked;
    }

    let is_enforced = point_in_paint_region(
        paint_ir,
        layer_index,
        &PaintSemantic::SupportEnforcer,
        centroid,
        BoundaryInclusion::Include,
        None,
    )
    .ok()
    .flatten()
    .is_some();

    if is_enforced {
        SupportPaintPolicy::Enforced
    } else {
        SupportPaintPolicy::DefaultEligible
    }
}

impl TreeSupport {
    /// Generate tree-style branching fill for a single ExPolygon.
    ///
    /// Algorithm:
    /// 1. Compute bounding box, derive grid spacing from density
    /// 2. Sample interior points on grid (point-in-polygon test)
    /// 3. Build nearest-neighbor tree from centroid
    /// 4. Walk tree edges to generate branch polylines
    /// 5. Each branch becomes an ExtrusionPath3D with SupportMaterial role
    fn fill_expolygon_tree(
        &self,
        expoly: &ExPolygon,
        z: f32,
        speed_factor: f32,
    ) -> Vec<ExtrusionPath3D> {
        // `support_density` is declared in tree-support.toml as a 0-100
        // percentage (min=0, max=100, default=20), matching OrcaSlicer's
        // UI convention. Convert to a 0-1 ratio before using it as the
        // spacing divisor. A density of 0 has already been filtered
        // upstream (run_support early-returns when density <= 0).
        let density_ratio = (self.density as f64 / 100.0).max(f64::EPSILON);
        let mut spacing_mm = self.line_width as f64 / density_ratio;

        // Compute bounding box in mm
        let (bb_min_x, bb_min_y, bb_max_x, bb_max_y) = polygon_bbox_mm(expoly);
        let bb_width = bb_max_x - bb_min_x;
        let bb_height = bb_max_y - bb_min_y;

        if bb_width <= 0.0 || bb_height <= 0.0 {
            return Vec::new();
        }

        // Sample-count cap: widen spacing so (bb_w/spacing)*(bb_h/spacing)
        // ≤ MAX_SAMPLES_PER_EXPOLY. Bounds the O(n²) Prim work per
        // ExPolygon; see MAX_SAMPLES_PER_EXPOLY doc.
        let min_spacing_from_cap = (bb_width * bb_height / MAX_SAMPLES_PER_EXPOLY).sqrt();
        if min_spacing_from_cap > spacing_mm {
            spacing_mm = min_spacing_from_cap;
        }

        // Sample interior points on a grid with spacing
        let mut samples: Vec<(f64, f64)> = Vec::new();

        let mut gy = bb_min_y + spacing_mm * 0.5;
        while gy < bb_max_y {
            let mut gx = bb_min_x + spacing_mm * 0.5;
            while gx < bb_max_x {
                if point_in_expolygon(gx, gy, expoly) {
                    samples.push((gx, gy));
                }
                gx += spacing_mm;
            }
            gy += spacing_mm;
        }

        // Centroid fallback: when the grid yields no samples (e.g. polygon
        // smaller than `spacing_mm` so no cell midpoint lands inside the
        // bbox at all), drop a single sample at the polygon centroid so any
        // non-empty support polygon still emits at least one branch path.
        if samples.is_empty() {
            let cx = (bb_min_x + bb_max_x) * 0.5;
            let cy = (bb_min_y + bb_max_y) * 0.5;
            if point_in_expolygon(cx, cy, expoly) {
                samples.push((cx, cy));
            }
        }

        if samples.is_empty() {
            return Vec::new();
        }

        if samples.len() == 1 {
            // Single point: emit a short path from point toward nearest boundary
            let (sx, sy) = samples[0];
            let (bx, by) = nearest_boundary_point(sx, sy, expoly);
            return vec![ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: sx as f32,
                        y: sy as f32,
                        z,
                        width: self.line_width,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                    Point3WithWidth {
                        x: bx as f32,
                        y: by as f32,
                        z,
                        width: self.line_width,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                ],
                role: ExtrusionRole::SupportMaterial,
                speed_factor,
            }];
        }

        // Compute centroid of samples
        let cx: f64 = samples.iter().map(|s| s.0).sum::<f64>() / samples.len() as f64;
        let cy: f64 = samples.iter().map(|s| s.1).sum::<f64>() / samples.len() as f64;

        // Find the sample nearest to the centroid as root
        let root_idx = samples
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (a.0 - cx).powi(2) + (a.1 - cy).powi(2);
                let db = (b.0 - cx).powi(2) + (b.1 - cy).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Build nearest-neighbor tree starting from root
        // parent[i] = index of parent node in tree (-1 for root)
        let tree = build_nearest_neighbor_tree(&samples, root_idx);

        // Convert tree edges to extrusion paths
        let mut paths = Vec::new();
        for (child_idx, parent_idx) in tree.iter().enumerate() {
            if let Some(pidx) = parent_idx {
                let (cx_pt, cy_pt) = samples[child_idx];
                let (px, py) = samples[*pidx];

                paths.push(ExtrusionPath3D {
                    points: vec![
                        Point3WithWidth {
                            x: px as f32,
                            y: py as f32,
                            z,
                            width: self.line_width,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        Point3WithWidth {
                            x: cx_pt as f32,
                            y: cy_pt as f32,
                            z,
                            width: self.line_width,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                    ],
                    role: ExtrusionRole::SupportMaterial,
                    speed_factor,
                });
            }
        }

        paths
    }
}

/// Compute the centroid of an ExPolygon's contour as the average of its vertices.
fn expolygon_centroid(expoly: &ExPolygon) -> Point2 {
    let pts = &expoly.contour.points;
    if pts.is_empty() {
        return Point2 { x: 0, y: 0 };
    }
    let n = pts.len() as i64;
    let sum_x: i64 = pts.iter().map(|p| p.x).sum();
    let sum_y: i64 = pts.iter().map(|p| p.y).sum();
    Point2 {
        x: sum_x / n,
        y: sum_y / n,
    }
}

/// Build a nearest-neighbor tree from sample points.
///
/// Starting from `root_idx`, repeatedly find the unvisited point nearest to
/// any visited point and connect it. Returns a Vec where `result[i] = Some(parent)`
/// for each node, or `None` for the root.
fn build_nearest_neighbor_tree(samples: &[(f64, f64)], root_idx: usize) -> Vec<Option<usize>> {
    let n = samples.len();
    let mut parent: Vec<Option<usize>> = vec![None; n];
    let mut visited = vec![false; n];

    visited[root_idx] = true;
    let mut visited_count = 1;

    while visited_count < n {
        let mut best_unvisited = 0;
        let mut best_visited = 0;
        let mut best_dist_sq = f64::MAX;

        for (ui, &(ux, uy)) in samples.iter().enumerate() {
            if visited[ui] {
                continue;
            }
            for (vi, &(vx, vy)) in samples.iter().enumerate() {
                if !visited[vi] {
                    continue;
                }
                let d_sq = (ux - vx).powi(2) + (uy - vy).powi(2);
                if d_sq < best_dist_sq {
                    best_dist_sq = d_sq;
                    best_unvisited = ui;
                    best_visited = vi;
                }
            }
        }

        parent[best_unvisited] = Some(best_visited);
        visited[best_unvisited] = true;
        visited_count += 1;
    }

    parent
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
    if !point_in_polygon(x, y, &expoly.contour.points) {
        return false;
    }
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

/// Find the nearest point on the boundary of an ExPolygon to a given point.
fn nearest_boundary_point(x: f64, y: f64, expoly: &ExPolygon) -> (f64, f64) {
    let mut best = (x, y);
    let mut best_dist = f64::MAX;

    nearest_point_on_polygon(x, y, &expoly.contour.points, &mut best, &mut best_dist);

    for hole in &expoly.holes {
        nearest_point_on_polygon(x, y, &hole.points, &mut best, &mut best_dist);
    }

    best
}

/// Find the nearest point on polygon edges, updating best/best_dist.
fn nearest_point_on_polygon(
    x: f64,
    y: f64,
    points: &[Point2],
    best: &mut (f64, f64),
    best_dist: &mut f64,
) {
    let n = points.len();
    if n < 2 {
        return;
    }

    for i in 0..n {
        let j = (i + 1) % n;
        let ax = slicer_ir::units_to_mm(points[i].x) as f64;
        let ay = slicer_ir::units_to_mm(points[i].y) as f64;
        let bx = slicer_ir::units_to_mm(points[j].x) as f64;
        let by = slicer_ir::units_to_mm(points[j].y) as f64;

        let (cx, cy) = closest_point_on_segment(x, y, ax, ay, bx, by);
        let dx = x - cx;
        let dy = y - cy;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < *best_dist {
            *best_dist = dist;
            *best = (cx, cy);
        }
    }
}

/// Find the closest point on a line segment to a given point.
fn closest_point_on_segment(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> (f64, f64) {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-15 {
        return (ax, ay);
    }

    let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    (ax + t * dx, ay + t * dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_print_start_defaults() {
        let config = ConfigView::from_map(std::collections::HashMap::new());
        let module = TreeSupport::on_print_start(&config).unwrap();
        assert!(!module.enabled);
        assert!((module.density - 0.2).abs() < 0.001);
        assert!((module.line_width - 0.4).abs() < 0.001);
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

    #[test]
    fn nearest_boundary_basic() {
        let expoly = ExPolygon {
            contour: slicer_ir::Polygon {
                points: vec![
                    Point2::from_mm(-5.0, -5.0),
                    Point2::from_mm(5.0, -5.0),
                    Point2::from_mm(5.0, 5.0),
                    Point2::from_mm(-5.0, 5.0),
                ],
            },
            holes: vec![],
        };
        let (bx, by) = nearest_boundary_point(0.0, 0.0, &expoly);
        let dist = ((bx * bx) + (by * by)).sqrt();
        assert!((dist - 5.0).abs() < 0.01);
    }
}
