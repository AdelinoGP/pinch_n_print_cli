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
//! Multi-layer support planner inspired by OrcaSlicer's TreeSupport::drop_nodes
//!
//! Port of OrcaSlicer's `TreeSupport::detect_overhangs` +
//! `TreeSupport::drop_nodes` (see `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`):
//! the planner walks each object's mesh, classifies overhang/bridge facets
//! via triangle normals, samples overhang polygons for contact points, and
//! propagates the contact-point set top-down through the object's layer
//! range. Per-layer merging uses a Prim minimum spanning tree — the same
//! O(V²) complexity class as OrcaSlicer's `MinimumSpanningTree::prim`.
//!
//! The planner reads real `LayerPlanView` (per-layer Z and effective height)
//! and `RegionSegmentationView` (per-object, per-layer region IDs) to
//! produce per-region `SupportPlanEntry` records.
//!
//! # Algorithmic features (Step 5)
//!
//! - **Avoidance / collision**: per-support-layer `collision_polys` (union of
//!   `SupportGeometryView.outlines`) and `avoidance_polys` (inflated by
//!   `branch_radius + tree_support_branch_distance / 2`). Move-pass clamps
//!   nodes into `avoidance_polys`; nodes whose target lies in
//!   `collision_polys` are dropped.
//! - **Radius tapering**: two-piece per-emit radius. With
//!   `mm_to_top = dist_to_top * effective_layer_height`,
//!   `raw = if mm_to_top <= branch_radius { mm_to_top }
//!          else { branch_radius + (mm_to_top - branch_radius) * tan(diameter_angle) }`,
//!   then `radius = clamp(raw, MIN_BRANCH_RADIUS = 0.4, MAX_BRANCH_RADIUS_MM = 6.0)`.
//!   The top of the column tapers to the minimum branch radius (`mm_to_top = 0 → 0.4`).
//! - **Wall-count scaling**: `max_move_distance = tan(angle) * height *
//!   wall_count.max(1)`.
//! - **dist_to_top tracking**: `u32` counter on each `PlannedSupportNode`
//!   incremented as nodes propagate downward; drives the radius taper formula.
//!
//! This module provides algorithmic shape detection, contact-point emission, top-down MST propagation, and emit logic — it is a faithful port for correctness, not numerical parity with OrcaSlicer.
//!
//! # Raft plan
//!
//! When `support_raft_layers > 0`, the planner emits one configuration-only
//! `RaftPlan`. Raft geometry is owned by a later packet.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_sdk::prelude::*;

const DEFAULT_BRANCH_ANGLE_DEG: f32 = 45.0;
const DEFAULT_MERGE_DISTANCE_MM: f32 = 0.8;
const DEFAULT_MAX_BRANCHES_PER_LAYER: usize = 1024;
const DEFAULT_LINE_WIDTH_MM: f32 = 0.4;
/// Overhang detection threshold: triangles whose normal z-component is below
/// `-sin(OVERHANG_THRESHOLD_DEG)` are flagged as overhang facets. Matches
/// OrcaSlicer's default `support_threshold_angle = 45°`.
const OVERHANG_THRESHOLD_DEG: f32 = 45.0;
/// Hard upper clamp on branch radius in mm. Matches OrcaSlicer's
/// `TreeSupportData::max_radius` hard upper bound (6.0 mm).
const MAX_BRANCH_RADIUS_MM: f32 = 6.0;
const MIN_BRANCH_RADIUS: f32 = 0.4;
/// Default vertical clearance between the top of a support column and the
/// overhang it supports. Matches OrcaSlicer's `support_top_z_distance` default
/// and `traditional-support-planner::DEFAULT_TOP_Z_DISTANCE_MM`, so both
/// families leave the same gap when the key is absent.
const DEFAULT_TOP_Z_DISTANCE_MM: f32 = 0.2;
/// Canonical fallback because this module does not declare `max_bridge_length`.
const DEFAULT_MAX_BRIDGE_LENGTH_MM: f32 = 10.0;

/// Multi-layer organic tree-support planner.
#[allow(dead_code)]
pub struct SupportPlanner {
    enabled: bool,
    /// Canonical support family selected for the matching renderer.
    support_family: String,
    branch_angle_deg: f32,
    merge_distance_mm: f32,
    max_branches_per_layer: usize,
    line_width_mm: f32,
    /// Branch diameter in mm (divide by 2 to get radius).
    tree_support_branch_diameter: f32,
    /// Angle in degrees controlling how fast radius grows with height.
    tree_support_branch_diameter_angle: f32,
    /// Spacing between branches in mm.
    tree_support_branch_distance: f32,
    /// Number of wall rings around each branch. Scales max move distance.
    tree_support_wall_count: u32,
    /// Number of raft layers to describe.
    support_raft_layers: i32,
    /// Density of the first raft layer.
    raft_first_layer_density: f32,
    /// Number of base raft layers.
    base_raft_layers: u32,
    /// Number of interface raft layers.
    interface_raft_layers: u32,
    /// Number of interface layers at top of each branch column.
    support_interface_top_layers: i32,
    /// Number of dense interface layers where branches land on the model.
    /// `-1` mirrors the top interface count (OrcaSlicer convention).
    support_interface_bottom_layers: i32,
    /// Line spacing for interface layer dense fill in mm.
    /// When true, contacts whose XY lies inside the object's projected
    /// footprint at the contact's layer (`to_buildplate = false`) are
    /// rejected at creation time — only build-plate-bound branches are
    /// emitted. Default `false`: to-model contacts are admitted and
    /// propagated like before. Packet 123.
    support_on_build_plate_only: bool,
    /// Vertical clearance in mm between the top of a support column and the
    /// overhang plane that demanded it. Packet 224 RC-11: this key was
    /// declared in the manifest but read nowhere, so tree support printed its
    /// top interface fused to the model.
    support_top_z_distance_mm: f32,
}

#[derive(Clone, Debug)]
struct PlannedSupportNode {
    x: f32,
    y: f32,
    /// Number of layers from this node to the top of its support column.
    /// Drives radius tapering — nodes farther from the top are wider.
    dist_to_top: u32,
    /// Whether this node must reach the build plate (true) or may rest
    /// on the model (false). Set at contact creation from
    /// `!point_in_any_expoly(collision_polys_at_contact_layer, x, y)`
    /// (true iff the contact's XY lies OUTSIDE the object's projected
    /// footprint at the contact's layer — OrcaSlicer's
    /// `generate_contact_points` initial assignment, per packet 123).
    /// Propagated unchanged through move-pass; merged with AND in
    /// multi-neighbour aggregation (any contributor that is `false`
    /// demotes the merged node to `false`).
    to_buildplate: bool,
    /// Stable analysis demands represented by this routed node.
    demand_ids: Vec<String>,
}

/// Assembles a plan entry's roles from structural, roof, and floor segments.
///
/// Canonical keeps roof and floor geometry distinct from body geometry and
/// removes it from `base_areas` (`TreeSupport::generate_toolpaths`' area pass).
/// Reproducing that subtraction is what keeps an interface layer from being
/// printed twice — once as dense interface and again as body underneath.
///
/// A role with no regions is omitted rather than emitted empty, so consumers can
/// treat role presence as meaningful.
fn build_roles(
    branch_segments: &[Vec<Point3WithWidth>],
    interface_segments: &[Vec<Point3WithWidth>],
    floor_segments: &[Vec<Point3WithWidth>],
    branch_radius: f32,
    collision_polys: &[ExPolygon],
) -> Vec<slicer_ir::SupportPlanRoleRegion> {
    let clip_collision = |regions: Vec<ExPolygon>| {
        if collision_polys.is_empty() || regions.is_empty() {
            regions
        } else {
            host::clip_polygons(&regions, collision_polys, ClipOperation::Difference)
        }
    };
    let body = clip_collision(structural_body_regions(branch_segments, branch_radius));
    let roof = clip_collision(structural_body_regions(interface_segments, branch_radius));
    let floor = clip_collision(structural_body_regions(floor_segments, branch_radius));

    // Subtract interface geometry out of the body, per canonical.
    let mut carved = body;
    for cut in [&roof, &floor] {
        if !carved.is_empty() && !cut.is_empty() {
            carved = host::clip_polygons(&carved, cut, ClipOperation::Difference);
        }
    }
    if !roof.is_empty() || !floor.is_empty() {
        carved.clear();
    }
    let mut roles = Vec::new();
    if !carved.is_empty() {
        roles.push(slicer_ir::SupportPlanRoleRegion {
            role: slicer_ir::SupportPlanRole::SupportBody,
            regions: carved,
        });
    }
    if !roof.is_empty() {
        roles.push(slicer_ir::SupportPlanRoleRegion {
            role: slicer_ir::SupportPlanRole::TopInterface,
            regions: roof,
        });
    }
    if !floor.is_empty() {
        roles.push(slicer_ir::SupportPlanRoleRegion {
            role: slicer_ir::SupportPlanRole::BottomInterface,
            regions: floor,
        });
    }
    roles
}

/// Convert planned centerline segments into semantic support-body regions.
///
/// Each segment is swept: the region is the convex hull of the two endpoint
/// circles, so a branch is a continuous capsule rather than a pair of detached
/// discs. Before packet 224 this emitted one independent 16-gon per endpoint
/// and unioned nothing, so a steeply-moving branch printed as a dotted line of
/// discs with gaps between them.
///
/// Zero-width points are the contact tips at the top of a column. They used to
/// be filtered out entirely, which meant the layer that is supposed to meet the
/// overhang produced no printable geometry at all; they are now floored at
/// `MIN_BRANCH_RADIUS` like every other point.
///
/// Overlapping segment hulls are unioned so a merged branch is one region.
pub fn structural_body_regions(
    segments: &[Vec<Point3WithWidth>],
    _branch_radius_mm: f32,
) -> Vec<ExPolygon> {
    let mut regions: Vec<ExPolygon> = Vec::new();
    for segment in segments {
        for pair in segment.windows(2) {
            if let Some(hull) = swept_region(&pair[0], &pair[1]) {
                regions.push(hull);
            }
        }
        if segment.len() == 1 {
            if let Some(disc) = swept_region(&segment[0], &segment[0]) {
                regions.push(disc);
            }
        }
    }
    let mut regions = if regions.len() < 2 {
        regions
    } else {
        // Union so merged branches and consecutive segments form one body
        // rather than a pile of overlapping capsules that would each be
        // walled and filled.
        let (first, rest) = regions.split_at(1);
        host::clip_polygons(first, rest, ClipOperation::Union)
    };
    for region in &mut regions {
        limit_contour_vertices(&mut region.contour.points, BRANCH_CIRCLE_SEGMENTS);
    }
    regions
}

fn limit_contour_vertices(points: &mut Vec<Point2>, limit: usize) {
    while points.len() > limit {
        let mut remove = 0;
        let mut smallest = i128::MAX;
        for index in 0..points.len() {
            let previous = points[(index + points.len() - 1) % points.len()];
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            let area = ((current.x - previous.x) as i128 * (next.y - current.y) as i128
                - (current.y - previous.y) as i128 * (next.x - current.x) as i128)
                .abs();
            if area < smallest {
                smallest = area;
                remove = index;
            }
        }
        points.remove(remove);
    }
}

/// Which semantic role a planned node's own area carries on its layer.
///
/// Canonical produces roof and floor as areas distinct from `base_areas` and
/// subtracts them out of the body, so a node contributes to exactly one of the
/// three collections.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InterfaceRole {
    /// Ordinary branch body.
    Body,
    /// Roof: within the top interface band of its column.
    Roof,
    /// Floor: the branch lands on the model rather than the build plate.
    Floor,
}

impl InterfaceRole {
    /// Select the collection a single node's segment belongs to.
    fn target_for_node<'a>(
        role: InterfaceRole,
        body: &'a mut Vec<Vec<Point3WithWidth>>,
        roof: &'a mut Vec<Vec<Point3WithWidth>>,
        floor: &'a mut Vec<Vec<Point3WithWidth>>,
    ) -> &'a mut Vec<Vec<Point3WithWidth>> {
        match role {
            InterfaceRole::Floor => floor,
            InterfaceRole::Roof => roof,
            InterfaceRole::Body => body,
        }
    }

    /// Select the collection an MST edge belongs to.
    ///
    /// Floor wins if either endpoint lands on the model, matching the existing
    /// floor-over-roof precedence. Roof requires *both* endpoints to be in the
    /// band, so an edge straddling the band boundary stays body rather than
    /// pulling non-interface geometry into the dense roof fill.
    fn target_for_edge<'a>(
        a: InterfaceRole,
        b: InterfaceRole,
        body: &'a mut Vec<Vec<Point3WithWidth>>,
        roof: &'a mut Vec<Vec<Point3WithWidth>>,
        floor: &'a mut Vec<Vec<Point3WithWidth>>,
    ) -> &'a mut Vec<Vec<Point3WithWidth>> {
        if a == InterfaceRole::Floor || b == InterfaceRole::Floor {
            floor
        } else if a == InterfaceRole::Roof && b == InterfaceRole::Roof {
            roof
        } else {
            body
        }
    }
}

/// Number of vertices used to approximate a branch circle.
const BRANCH_CIRCLE_SEGMENTS: usize = 16;

/// Build the swept region between two centerline points as the convex hull of
/// their radius circles. Returns `None` when the result is degenerate.
fn swept_region(a: &Point3WithWidth, b: &Point3WithWidth) -> Option<ExPolygon> {
    let mut points = Vec::with_capacity(BRANCH_CIRCLE_SEGMENTS * 2);
    for point in [a, b] {
        let radius = (point.width * 0.5).max(MIN_BRANCH_RADIUS);
        let radius_units = mm_to_units(radius).max(1);
        let cx = mm_to_units(point.x);
        let cy = mm_to_units(point.y);
        for i in 0..BRANCH_CIRCLE_SEGMENTS {
            let angle = std::f32::consts::TAU * i as f32 / BRANCH_CIRCLE_SEGMENTS as f32;
            points.push(Point2 {
                x: cx + (radius_units as f32 * angle.cos()).round() as i64,
                y: cy + (radius_units as f32 * angle.sin()).round() as i64,
            });
        }
    }
    let hull = convex_hull(points);
    if hull.len() < 3 {
        return None;
    }
    Some(ExPolygon {
        contour: Polygon { points: hull },
        holes: Vec::new(),
    })
}

/// Andrew's monotone-chain convex hull, counter-clockwise, no collinear points.
fn convex_hull(mut points: Vec<Point2>) -> Vec<Point2> {
    points.sort_by(|p, q| p.x.cmp(&q.x).then(p.y.cmp(&q.y)));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    // i128 keeps the cross product exact: coordinates are scaled integers and
    // a branch capsule at plate scale overflows i64 when squared.
    fn cross(o: &Point2, a: &Point2, b: &Point2) -> i128 {
        (a.x as i128 - o.x as i128) * (b.y as i128 - o.y as i128)
            - (a.y as i128 - o.y as i128) * (b.x as i128 - o.x as i128)
    }
    let mut hull: Vec<Point2> = Vec::with_capacity(points.len() * 2);
    for point in points.iter() {
        while hull.len() >= 2 && cross(&hull[hull.len() - 2], &hull[hull.len() - 1], point) <= 0 {
            hull.pop();
        }
        hull.push(*point);
    }
    let lower_len = hull.len() + 1;
    for point in points.iter().rev() {
        while hull.len() >= lower_len
            && cross(&hull[hull.len() - 2], &hull[hull.len() - 1], point) <= 0
        {
            hull.pop();
        }
        hull.push(*point);
    }
    hull.pop();
    hull
}

/// Holds collision and avoidance polygons for a single support layer.
#[derive(Clone, Debug, Default)]
struct LayerCollisionCache {
    /// Direct support-outline ExPolygons — nodes must stay outside these.
    /// Holes are preserved so a point inside a hole is not in collision.
    collision_polys: Vec<ExPolygon>,
    /// Inflated collision ExPolygons — nodes must stay inside these.
    /// Holes are preserved from the offset result.
    avoidance_polys: Vec<ExPolygon>,
}

#[slicer_module]
impl PrepassModule for SupportPlanner {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("enable_support") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => true,
        };
        let support_family = canonical_support_family(config);
        let branch_angle_deg = match config.get("tree_support_branch_angle") {
            Some(ConfigValue::Float(a)) => *a as f32,
            Some(ConfigValue::Int(a)) => *a as f32,
            _ => DEFAULT_BRANCH_ANGLE_DEG,
        };
        let merge_distance_mm = match config.get("support_branch_merge_distance_mm") {
            Some(ConfigValue::Float(a)) => *a as f32,
            Some(ConfigValue::Int(a)) => *a as f32,
            _ => DEFAULT_MERGE_DISTANCE_MM,
        };
        let max_branches_per_layer = match config.get("support_max_branches_per_layer") {
            Some(ConfigValue::Int(n)) => (*n as usize).clamp(1, 10_000),
            Some(ConfigValue::Float(n)) => (*n as usize).clamp(1, 10_000),
            _ => DEFAULT_MAX_BRANCHES_PER_LAYER,
        };
        let line_width_mm = match config.get("line_width") {
            Some(ConfigValue::Float(w)) => *w as f32,
            _ => DEFAULT_LINE_WIDTH_MM,
        };
        // ── New Step-5 config keys (with legacy fallback) ─────────────────
        let tree_support_branch_diameter = match config.get("tree_support_branch_diameter") {
            Some(ConfigValue::Float(d)) => *d as f32,
            Some(ConfigValue::Int(d)) => *d as f32,
            _ => 5.0,
        };
        let tree_support_branch_diameter_angle =
            match config.get("tree_support_branch_diameter_angle") {
                Some(ConfigValue::Float(a)) => *a as f32,
                Some(ConfigValue::Int(a)) => *a as f32,
                _ => 5.0,
            };
        let tree_support_branch_distance = match config.get("tree_support_branch_distance") {
            Some(ConfigValue::Float(d)) => *d as f32,
            Some(ConfigValue::Int(d)) => *d as f32,
            _ => 1.0,
        };
        let tree_support_wall_count = match config.get("tree_support_wall_count") {
            Some(ConfigValue::Int(n)) => *n as u32,
            Some(ConfigValue::Float(n)) => *n as u32,
            _ => 1,
        };
        let support_raft_layers = match config.get("support_raft_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => 0,
        };
        let raft_first_layer_density = match config.get("raft_first_layer_density") {
            Some(ConfigValue::Float(d)) => *d as f32,
            Some(ConfigValue::Int(d)) => *d as f32,
            _ => 0.4,
        };
        let base_raft_layers = match config.get("base_raft_layers") {
            Some(ConfigValue::Int(n)) => *n as u32,
            Some(ConfigValue::Float(n)) => *n as u32,
            _ => 1,
        };
        let interface_raft_layers = match config.get("interface_raft_layers") {
            Some(ConfigValue::Int(n)) => *n as u32,
            Some(ConfigValue::Float(n)) => *n as u32,
            _ => 0,
        };
        let support_interface_bottom_layers = match config.get("support_interface_bottom_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => -1,
        };
        let support_interface_top_layers = match config.get("support_interface_top_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => 2,
        };
        // Packet 123: `support_on_build_plate_only` config — when true,
        // contacts whose `to_buildplate` would be `false` are rejected
        // at creation time (no to-model branches). Default `false` to
        // preserve the current planner behavior.
        let support_on_build_plate_only = match config.get("support_on_build_plate_only") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
        };
        // Packet 224 RC-11: honour the top-Z gap the manifest already declared.
        let support_top_z_distance_mm = match config.get("support_top_z_distance_mm") {
            Some(ConfigValue::Float(d)) => *d as f32,
            Some(ConfigValue::Int(d)) => *d as f32,
            _ => DEFAULT_TOP_Z_DISTANCE_MM,
        }
        .max(0.0);
        Ok(Self {
            enabled,
            support_family,
            branch_angle_deg,
            merge_distance_mm,
            max_branches_per_layer,
            line_width_mm,
            tree_support_branch_diameter,
            tree_support_branch_diameter_angle,
            tree_support_branch_distance,
            tree_support_wall_count,
            support_raft_layers,
            raft_first_layer_density,
            base_raft_layers,
            interface_raft_layers,
            support_interface_top_layers,
            support_interface_bottom_layers,
            support_on_build_plate_only,
            support_top_z_distance_mm,
        })
    }

    fn run_support_geometry(
        &self,
        objects: &[MeshObjectView],
        layer_plan: &LayerPlanView,
        region_segmentation: &RegionSegmentationView,
        support_geometry: &SupportGeometryView,
        output: &mut SupportGeometryOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        self.run_support_geometry_with_analysis(
            objects,
            layer_plan,
            region_segmentation,
            &SupportAnalysisView::default(),
            support_geometry,
            output,
            config,
        )
    }

    fn run_support_geometry_with_analysis(
        &self,
        objects: &[MeshObjectView],
        layer_plan: &LayerPlanView,
        region_segmentation: &RegionSegmentationView,
        support_analysis: &SupportAnalysisView,
        support_geometry: &SupportGeometryView,
        output: &mut SupportGeometryOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        if layer_plan.layers.is_empty() {
            return Err(ModuleError::fatal(1, "empty layer-plan-view"));
        }

        if self.support_raft_layers > 0 {
            output
                .push_raft_plan(RaftPlan {
                    raft_layers: self.support_raft_layers as u32,
                    raft_first_layer_density: self.raft_first_layer_density,
                    base_raft_layers: self.base_raft_layers,
                    interface_raft_layers: self.interface_raft_layers,
                })
                .map_err(|e| ModuleError::fatal(1, format!("push_raft_plan failed: {e}")))?;
        }

        // ── Build per-layer collision / avoidance caches ──────────────────
        // collision_polys[L] = union of all outlines at SupportGeometryView[L]
        // avoidance_polys[L] = collision_polys[L].inflate(branch_radius + branch_distance/2)
        let mut collision_cache: Vec<LayerCollisionCache> =
            vec![LayerCollisionCache::default(); layer_plan.layers.len()];

        let branch_radius = self.tree_support_branch_diameter / 2.0;
        let avoid_inflate = branch_radius + self.tree_support_branch_distance / 2.0;

        // Every outline's inflation is independent of every other's, and this
        // loop dominates the module: a native replay of a captured benchy
        // prepass put 98% of `run_support_geometry`'s runtime here, across ~600
        // calls at ~7 ms each. Guests cannot thread, and `offset_polygons` runs
        // clipper2 *inside* the sandbox, so the only way to parallelize it is to
        // hand the whole set to the host at once (ADR-0049).
        //
        // Collect in exactly the order the serial loop visited, then distribute
        // results back the same way — `batch_offset` keeps each result paired
        // with its own input, so push order into both caches is unchanged.
        let mut outlines: Vec<(usize, &ExPolygon)> = Vec::new();
        for entry in &support_geometry.entries {
            let layer_idx = entry.global_support_layer_index as usize;
            if layer_idx >= collision_cache.len() {
                continue;
            }
            for expoly in &entry.outlines {
                if expoly.contour.points.len() >= 3 {
                    outlines.push((layer_idx, expoly));
                }
            }
        }

        let inflated = slicer_sdk::host_batch::batch_offset(&outlines, |(_, expoly)| {
            slicer_sdk::host_batch::OffsetRequest {
                polygons: vec![(*expoly).clone()],
                delta_mm: avoid_inflate,
                join: OffsetJoinType::Miter,
                arc_tolerance_mm: 0.0,
            }
        });

        for ((layer_idx, expoly), avoidance) in inflated {
            collision_cache[*layer_idx]
                .collision_polys
                .push((*expoly).clone());
            for off in avoidance {
                if off.contour.points.len() >= 3 {
                    collision_cache[*layer_idx].avoidance_polys.push(off);
                }
            }
        }

        // `support_interface_bottom_layers` is implemented as of packet 224: it
        // is read in `from_config` into `self.support_interface_bottom_layers`
        // and drives the `BottomInterface` band (canonical `floor_areas`) where
        // branches land on the model. `-1` mirrors the top interface count,
        // matching canonical's `number_of_support_interface_bottom_layers`.
        //
        // This site previously emitted a code 1003 "not yet implemented"
        // warning. That diagnostic is retired because the feature now exists —
        // leaving it would report a working config key as unsupported.

        // ── Packet 118 B4: cross-object merged cap diagnostic ───────────
        // Accumulate drops across all objects on the same global layer so
        // we emit one code-1001 diagnostic per affected global layer
        // (design.md Locked Assumptions: 'one cap diagnostic per affected
        // global layer, not once per dropped candidate'). The map is
        // populated inside plan_for_object and drained in run_support_geometry
        // after the per-object loop.
        let mut dropped_by_layer: std::collections::BTreeMap<u32, usize> =
            std::collections::BTreeMap::new();

        for obj in objects {
            self.plan_for_object(
                obj,
                layer_plan,
                region_segmentation,
                support_analysis,
                support_geometry,
                &collision_cache,
                output,
                &mut dropped_by_layer,
            )?;
        }

        // Emit one code-1001 diagnostic per affected global layer. The
        // cap is enforced per-layer globally, so a layer hit by multiple
        // objects' drops collapses to a single diagnostic with the merged
        // dropped_count. object_id is None because the cap is layer-level,
        // not object-level.
        for (global_layer_index, dropped) in &dropped_by_layer {
            if *dropped == 0 {
                continue;
            }
            let cap = self.max_branches_per_layer;
            let _ = output.push_diagnostic(Diagnostic {
                severity: DiagnosticSeverity::Warn,
                code: 1001,
                layer: Some(*global_layer_index as i32),
                object_id: None,
                message: format!(
                    "support-planner cap: max_branches_per_layer cap exceeded: \
                     dropped_count={dropped} kept_count={cap}"
                ),
            });
        }

        Ok(())
    }
}

impl SupportPlanner {
    fn plan_for_object(
        &self,
        obj: &MeshObjectView,
        layer_plan: &LayerPlanView,
        region_segmentation: &RegionSegmentationView,
        support_analysis: &SupportAnalysisView,
        _support_geometry: &SupportGeometryView,
        collision_cache: &[LayerCollisionCache],
        output: &mut SupportGeometryOutput,
        dropped_by_layer: &mut std::collections::BTreeMap<u32, usize>,
    ) -> Result<(), ModuleError> {
        // ── Layer range from committed layer plan ────────────────────────
        let num_layers = layer_plan.layers.len() as u32;
        if num_layers == 0 {
            return Ok(());
        }

        // Skip objects with no region segmentation entries.
        let has_regions = region_segmentation
            .entries
            .iter()
            .any(|e| e.object_id == obj.object_id);
        if !has_regions {
            return Ok(());
        }

        // Host analysis owns candidate discovery and policy. Geometry remains
        // an independent compatibility input for collision avoidance below.
        let mut contacts_by_layer: Vec<Vec<PlannedSupportNode>> =
            vec![Vec::new(); num_layers as usize];
        let mut fallback_family_emitted = false;

        // Per-affected-layer drop count for the code 1001 cap diagnostic.
        // Keyed by global_layer_index so the message carries the right value
        // even when layer_rev doesn't line up with the layer-plan index.
        // Owned by run_support_geometry; this function increments into the
        // shared map so per-layer totals are merged across all objects
        // before emission.

        // Keep projected mesh contacts as the primary legacy path. Analysis
        // augments these contacts but never decides whether they are admitted.
        if let Some((bmin, _)) = compute_bounds(&obj.vertices) {
            let blockers = collect_paint_blocker_polygons(obj);
            let (mesh_min, mesh_max) = compute_bounds(&obj.vertices).expect("bounds checked");
            let origin = (
                (mesh_min[0] + mesh_max[0]) * 0.5,
                (mesh_min[1] + mesh_max[1]) * 0.5,
            );
            let mut polygons_by_layer: std::collections::BTreeMap<usize, Vec<ExPolygon>> =
                std::collections::BTreeMap::new();
            for (v0, v1, v2) in detect_overhang_facets(obj, OVERHANG_THRESHOLD_DEG) {
                let z = (v0[2] + v1[2] + v2[2]) / 3.0;
                if z <= bmin[2] + layer_plan.layers[0].effective_layer_height * 0.5 {
                    continue;
                }
                let layer_idx = layer_plan
                    .layers
                    .iter()
                    .position(|layer| layer.z >= z)
                    .unwrap_or(layer_plan.layers.len() - 1);
                // Legacy-path compatibility shim: canonical input is the
                // host-computed per-layer overhang polygon. These fixtures are
                // coplanar plates, so project downward triangles instead of
                // slicing an otherwise empty closed-solid cross-section.
                polygons_by_layer
                    .entry(layer_idx)
                    .or_default()
                    .push(ExPolygon {
                        contour: Polygon {
                            points: vec![
                                Point2::from_mm(v0[0], v0[1]),
                                Point2::from_mm(v1[0], v1[1]),
                                Point2::from_mm(v2[0], v2[1]),
                            ],
                        },
                        holes: Vec::new(),
                    });
            }
            for (layer_idx, polygons) in polygons_by_layer {
                let polygons = if polygons.len() > 1 {
                    let (first, rest) = polygons.split_at(1);
                    host::clip_polygons(first, rest, ClipOperation::Union)
                } else {
                    polygons
                };
                let sampled: Vec<(f32, f32)> = sample_contact_points(
                    &polygons,
                    origin,
                    Some((mesh_min[0], mesh_max[0], mesh_min[1], mesh_max[1])),
                    self.tree_support_branch_distance,
                    MIN_BRANCH_RADIUS.max(self.tree_support_branch_diameter / 2.0),
                )
                .into_iter()
                .collect();
                for (x, y) in sampled {
                    if !point_in_any_polygon(&blockers, x, y) {
                        push_contact(
                            &mut contacts_by_layer,
                            layer_plan,
                            collision_cache,
                            layer_idx,
                            x,
                            y,
                            self,
                            dropped_by_layer,
                        );
                    }
                }
            }
            for (layer_idx, x, y) in collect_paint_enforcer_contacts(obj) {
                if !point_in_any_polygon(&blockers, x, y) {
                    push_contact(
                        &mut contacts_by_layer,
                        layer_plan,
                        collision_cache,
                        (layer_idx as usize).min(num_layers as usize - 1),
                        x,
                        y,
                        self,
                        dropped_by_layer,
                    );
                }
            }
        }
        // Analysis augments the legacy contacts only for scopes it actually
        // populated. This preserves the prior behavior for an absent or
        // partial SupportAnalysisView while consuming host candidates when
        // available.
        for candidate in support_analysis.candidates.iter().filter(|candidate| {
            candidate.object_id == obj.object_id
                && candidate.blocked
                && candidate_family(candidate, support_analysis, &self.support_family).as_deref()
                    == Some("tree")
        }) {
            let _ = output.push_support_plan_entry(slicer_sdk::prepass_types::SupportPlanEntry {
                global_layer_index: candidate.global_layer_index as i32,
                object_id: obj.object_id.clone(),
                region_id: candidate.region_id.clone(),
                family_id: "tree".to_string(),
                demand_ids: vec![format!("demand-{}", candidate.id)],
                body_ids: Vec::new(),
                anchor_layer_index: candidate.global_layer_index,
                anchor_z: candidate.z_units,
                roles: Vec::new(),
                skeleton: None,
                capabilities: Vec::new(),
                provenance: vec!["support-planner".to_string()],
                decline_reason: Some(slicer_ir::SupportPlanDeclineReason::Blocked),
            });
        }
        for candidate in support_analysis.candidates.iter().filter(|candidate| {
            candidate.object_id == obj.object_id
                && !candidate.blocked
                && candidate_family(candidate, support_analysis, &self.support_family).as_deref()
                    == Some("tree")
                && candidate
                    .geometry
                    .iter()
                    .any(|polygon| polygon.contour.points.len() >= 3)
                && region_segmentation.entries.iter().any(|entry| {
                    entry.object_id == obj.object_id
                        && entry.layer_index == candidate.global_layer_index
                        && entry
                            .region_ids
                            .iter()
                            .any(|region_id| region_id == &candidate.region_id)
                })
        }) {
            let layer_idx = layer_plan
                .layers
                .iter()
                .position(|layer| layer.global_layer_index == candidate.global_layer_index)
                .unwrap_or_else(|| candidate.global_layer_index.min(num_layers - 1) as usize);
            let (origin, grid_bounds) = compute_bounds(&obj.vertices)
                .map(|(min, max)| {
                    (
                        ((min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5),
                        Some((min[0], max[0], min[1], max[1])),
                    )
                })
                .unwrap_or(((0.0, 0.0), None));
            for (sample_idx, (x, y)) in sample_contact_points(
                &candidate.geometry,
                origin,
                grid_bounds,
                self.tree_support_branch_distance,
                MIN_BRANCH_RADIUS.max(self.tree_support_branch_diameter / 2.0),
            )
            .into_iter()
            .enumerate()
            {
                push_analysis_contact(
                    &mut contacts_by_layer,
                    layer_plan,
                    collision_cache,
                    layer_idx,
                    x,
                    y,
                    self,
                    dropped_by_layer,
                    format!("demand-{}-{}", candidate.id, sample_idx),
                );
            }
        }

        // Some host projections provide support geometry but not mesh facets.
        // Use outlines only as a fallback, so they neither replace nor suppress
        // contacts from the legacy mesh path.
        // An empty triangle list is not enough to activate this compatibility
        // path: projected model outlines are also present for ordinary meshes
        // whose facets produced no contacts. Only a genuinely mesh-less object
        // may use outlines as legacy contact geometry.
        if obj.vertices.is_empty()
            && obj.triangles.is_empty()
            && contacts_by_layer.iter().all(|contacts| contacts.is_empty())
        {
            for entry in _support_geometry
                .entries
                .iter()
                .filter(|entry| entry.object_id == obj.object_id)
            {
                let layer_idx = layer_plan
                    .layers
                    .iter()
                    .position(|layer| layer.global_layer_index == entry.global_support_layer_index)
                    .unwrap_or_else(|| {
                        entry.global_support_layer_index.min(num_layers - 1) as usize
                    });
                let Some((x, y)) = candidate_contact_point(&entry.outlines) else {
                    continue;
                };
                push_contact(
                    &mut contacts_by_layer,
                    layer_plan,
                    collision_cache,
                    layer_idx,
                    x,
                    y,
                    self,
                    dropped_by_layer,
                );
            }
        }

        // Bail out when nothing needs support.
        if contacts_by_layer.iter().all(|v| v.is_empty()) {
            return Ok(());
        }

        // ── Step 10: top-down propagation + per-layer MST merging ────────
        // Walk from top layer down to layer 0. Each iteration:
        //   a) pull in propagated nodes from layer (l+1) plus fresh contacts at l
        //   b) group nodes and run Prim MST
        //   c) merge nodes within merge_distance; record MST edges as segments
        //   d) move each surviving node toward its MST neighbor by step_xy
        //   e) pass surviving nodes down to layer (l-1)
        let tan_angle = self.branch_angle_deg.to_radians().tan();
        let tan_diameter_angle = self.tree_support_branch_diameter_angle.to_radians().tan();
        let branch_radius = self.tree_support_branch_diameter / 2.0;
        // wall_count multiplier — fall back to 1 per OrcaSlicer line 2632
        let wall_count_factor = self.tree_support_wall_count.max(1) as f32;

        let mut active_nodes: Vec<PlannedSupportNode> = Vec::new();
        let mut roof_band_layers_emitted = 0u32;

        // Accumulate entries bottom-up so the plan keeps a deterministic,
        // top-to-bottom layer order in output.
        let mut entries_in_order: Vec<SupportPlanEntry> = Vec::new();

        // Iterate top → bottom.
        let top = num_layers as usize;
        for layer_rev in (0..top).rev() {
            let current_global_layer_index = layer_plan.layers[layer_rev].global_layer_index;
            // Merge freshly-detected contacts at this layer.
            active_nodes.extend(std::mem::take(&mut contacts_by_layer[layer_rev]));
            if active_nodes.is_empty() {
                continue;
            }
            if active_nodes.len() > self.max_branches_per_layer {
                let dropped = active_nodes.len() - self.max_branches_per_layer;
                active_nodes.truncate(self.max_branches_per_layer);
                *dropped_by_layer
                    .entry(current_global_layer_index)
                    .or_insert(0) += dropped;
            }

            // Sort for deterministic MST/merge ordering.
            active_nodes.sort_by(|a, b| match a.x.partial_cmp(&b.x) {
                Some(std::cmp::Ordering::Equal) | None => {
                    a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal)
                }
                Some(ord) => ord,
            });

            // Run Prim MST on the active node set.
            let mst_edges = prim_mst(&active_nodes);

            // Merge nodes within merge_distance: mark the higher-index endpoint
            // of every short edge for removal.
            let mut drop = vec![false; active_nodes.len()];
            for (a, b, d) in &mst_edges {
                if *d < self.merge_distance_mm {
                    drop[*a.max(b)] = true;
                    let (keep, removed) = if a < b { (*a, *b) } else { (*b, *a) };
                    let ids = active_nodes[removed].demand_ids.clone();
                    for id in ids {
                        if !active_nodes[keep].demand_ids.contains(&id) {
                            active_nodes[keep].demand_ids.push(id);
                        }
                    }
                }
            }

            // Record the committed edges as branch segments (mm-space) on
            // this layer. Points sit at this layer's Z.
            let effective_height = layer_plan.layers[layer_rev].effective_layer_height;
            // Wall-count scaled max move distance (Step 5 AC-5)
            let max_move_xy = (tan_angle * effective_height * wall_count_factor).max(0.0);
            let z_current = layer_plan.layers[layer_rev].z;

            // Collision/avoidance polygons for this layer (Step 5 AC-3)
            let cache_idx = current_global_layer_index as usize;
            let (collision_polys, avoidance_polys) = if cache_idx < collision_cache.len() {
                (
                    collision_cache[cache_idx].collision_polys.as_slice(),
                    collision_cache[cache_idx].avoidance_polys.as_slice(),
                )
            } else {
                (&[][..], &[][..])
            };
            // Host analysis carries the exact per-layer occupancy used by the
            // closure gate. Prefer it for emission checks when present; the
            // support-outline cache remains the compatibility fallback.
            let model_collision: Vec<ExPolygon> = support_analysis
                .model_occupancy
                .iter()
                .filter(|entry| {
                    entry.object_id == obj.object_id
                        && entry.global_support_layer_index == current_global_layer_index
                })
                .flat_map(|entry| entry.polygons.iter().cloned())
                .collect();
            let collision_polys = if model_collision.is_empty() {
                collision_polys
            } else {
                model_collision.as_slice()
            };

            // Emit branch segments with radius tapering (Step 5 AC-2)
            let mut branch_segments: Vec<Vec<Point3WithWidth>> = Vec::new();
            let mut interface_segments: Vec<Vec<Point3WithWidth>> = Vec::new();
            let mut floor_segments: Vec<Vec<Point3WithWidth>> = Vec::new();

            // Canonical keeps roof and floor as distinct *areas* carved out of
            // `base_areas` (`TreeSupport::generate_toolpaths`' area pass), and
            // fills them at the interface spacing. This classifies each node's
            // own branch area, which is what the renderer then fills.
            //
            // Until packet 224 the interface was instead a set of axis-aligned
            // scan lines over the node's bounding box, carrying the full branch
            // diameter as their width — so every scan-line endpoint expanded
            // into a branch-sized disc and the roof bore no relation to the
            // branch footprint.
            let top_n = self.support_interface_top_layers.max(0) as u32;
            // `-1` mirrors the top interface count, matching canonical's
            // `number_of_support_interface_bottom_layers` fallback.
            let bottom_n = if self.support_interface_bottom_layers < 0 {
                top_n
            } else {
                self.support_interface_bottom_layers.max(0) as u32
            };
            let node_roles: Vec<InterfaceRole> = active_nodes
                .iter()
                .map(|node| {
                    // Floor: the branch lands on the model rather than the plate.
                    // The lookup is by *global* layer index; it previously mixed
                    // in `layer_rev`, the reverse loop counter, and indexed the
                    // collision cache with it.
                    let is_floor = bottom_n > 0
                        && !self.support_on_build_plate_only
                        && (1..=bottom_n).any(|k| {
                            cache_idx
                                .checked_sub(k as usize)
                                .and_then(|below| collision_cache.get(below))
                                .is_some_and(|cache| {
                                    point_in_any_expoly(&cache.collision_polys, node.x, node.y)
                                })
                        });
                    // Roof: the top `top_n` layers of the column, counting the
                    // contact layer itself. `dist_to_top` is canonical's
                    // per-node `support_roof_layers_below` counter. The contact
                    // layer must remain interface geometry under the model.
                    let is_roof = top_n > roof_band_layers_emitted && node.dist_to_top < top_n;
                    if is_floor {
                        InterfaceRole::Floor
                    } else if is_roof {
                        InterfaceRole::Roof
                    } else {
                        InterfaceRole::Body
                    }
                })
                .collect();
            let mut origin_contacts_emitted = vec![false; active_nodes.len()];
            let mut mst_emitted = vec![false; active_nodes.len()];
            for (a_idx, b_idx, _) in &mst_edges {
                if drop[*a_idx] || drop[*b_idx] {
                    continue;
                }
                mst_emitted[*a_idx] = true;
                mst_emitted[*b_idx] = true;
                let na = &active_nodes[*a_idx];
                let nb = &active_nodes[*b_idx];

                // Tapered radii at the two endpoints
                let radius_a = tapered_radius(
                    branch_radius,
                    tan_diameter_angle,
                    na.dist_to_top,
                    effective_height,
                );
                let radius_b = tapered_radius(
                    branch_radius,
                    tan_diameter_angle,
                    nb.dist_to_top,
                    effective_height,
                );

                // Interface nodes are allowed to meet the model, but that
                // exemption is per endpoint. A mixed body/interface edge must
                // still reject its body endpoint; exempting the whole edge
                // lets body geometry leak into exact-Z model occupancy.
                let body_endpoint_collides = (node_roles[*a_idx] == InterfaceRole::Body
                    && body_intersects(collision_polys, na.x, na.y, radius_a))
                    || (node_roles[*b_idx] == InterfaceRole::Body
                        && body_intersects(collision_polys, nb.x, nb.y, radius_b));
                let segment_collides =
                    body_segment_intersects(collision_polys, na, nb, radius_a, radius_b);
                if body_endpoint_collides || segment_collides {
                    let _ = output.push_diagnostic(Diagnostic {
                        severity: DiagnosticSeverity::Warn,
                        code: 1203,
                        layer: Some(current_global_layer_index as i32),
                        object_id: Some(obj.object_id.clone()),
                        message: "tree body rejected: complete radius intersects model occupancy"
                            .into(),
                    });
                    continue;
                }

                let dist_a_mm = na.dist_to_top as f32 * effective_height;
                let dist_b_mm = nb.dist_to_top as f32 * effective_height;
                InterfaceRole::target_for_edge(
                    node_roles[*a_idx],
                    node_roles[*b_idx],
                    &mut branch_segments,
                    &mut interface_segments,
                    &mut floor_segments,
                )
                .push(vec![
                    Point3WithWidth {
                        x: na.x,
                        y: na.y,
                        z: z_current,
                        width: radius_a * 2.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: dist_a_mm,
                        overhang_distance_mm: None,
                    },
                    Point3WithWidth {
                        x: nb.x,
                        y: nb.y,
                        z: z_current,
                        width: radius_b * 2.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: dist_b_mm,
                        overhang_distance_mm: None,
                    },
                ]);
                if na.dist_to_top == 0 {
                    origin_contacts_emitted[*a_idx] = true;
                }
                if nb.dist_to_top == 0 {
                    origin_contacts_emitted[*b_idx] = true;
                }
            }

            // A fresh contact is the tip of a support column and must be
            // represented on its origin layer even when it has no surviving
            // MST edge. This is intentionally limited to dist_to_top == 0;
            // propagated nodes remain subject to collision exclusion below.
            for (i, node) in active_nodes.iter().enumerate() {
                if node.dist_to_top != 0 || origin_contacts_emitted[i] {
                    continue;
                }
                // A contact tip used to be emitted with `width = 0.0` and was
                // then dropped by `structural_body_regions`, so the layer that
                // meets the overhang produced no printable geometry. It now
                // carries the tapered radius like any other node.
                let radius = tapered_radius(
                    branch_radius,
                    tan_diameter_angle,
                    node.dist_to_top,
                    effective_height,
                );
                if body_intersects(collision_polys, node.x, node.y, radius) {
                    let _ = output.push_diagnostic(Diagnostic {
                        severity: DiagnosticSeverity::Warn,
                        code: 1203,
                        layer: Some(current_global_layer_index as i32),
                        object_id: Some(obj.object_id.clone()),
                        message: "tree contact tip rejected: radius intersects model occupancy"
                            .to_string(),
                    });
                    continue;
                }
                let width = radius * 2.0;
                // Origin contacts are the support tips required to reach the
                // overhang centroid. They may intentionally lie in model
                // collision geometry; propagated nodes remain guarded below.
                let (contact_x, contact_y) = (node.x, node.y);
                let point = Point3WithWidth {
                    x: contact_x,
                    y: contact_y,
                    z: z_current,
                    width,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                };
                InterfaceRole::target_for_node(
                    node_roles[i],
                    &mut branch_segments,
                    &mut interface_segments,
                    &mut floor_segments,
                )
                .push(vec![point, point]);
            }

            if top_n > roof_band_layers_emitted && !interface_segments.is_empty() {
                roof_band_layers_emitted += 1;
            }

            // A surviving lone propagated node (dist_to_top > 0) with no surviving
            // MST edge still reaches the buildplate and must be emitted as a
            // degenerate current-layer segment (OrcaSlicer draw_circles parity).
            for (i, node) in active_nodes.iter().enumerate() {
                if drop[i] || mst_emitted[i] || node.dist_to_top == 0 {
                    continue;
                }
                if node.dist_to_top > 0 {
                    let radius = tapered_radius(
                        branch_radius,
                        tan_diameter_angle,
                        node.dist_to_top,
                        effective_height,
                    );
                    if body_intersects(collision_polys, node.x, node.y, radius) {
                        continue;
                    }
                    let width = radius * 2.0;
                    let dist_mm = node.dist_to_top as f32 * effective_height;
                    let point = Point3WithWidth {
                        x: node.x,
                        y: node.y,
                        z: z_current,
                        width,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: dist_mm,
                        overhang_distance_mm: None,
                    };
                    InterfaceRole::target_for_node(
                        node_roles[i],
                        &mut branch_segments,
                        &mut interface_segments,
                        &mut floor_segments,
                    )
                    .push(vec![point, point]);
                }
            }

            // Emit when *either* structural or interface geometry exists. Gating
            // on `branch_segments` alone was what collapsed the branch shaft in
            // an earlier attempt at this split: layers carrying only interface
            // geometry produced no entry at all.
            if !branch_segments.is_empty()
                || !interface_segments.is_empty()
                || !floor_segments.is_empty()
            {
                // Find all regions for this (layer, object) pair.
                let regions_for_this: Vec<_> = region_segmentation
                    .entries
                    .iter()
                    .filter(|e| {
                        e.object_id == obj.object_id && e.layer_index == current_global_layer_index
                    })
                    .flat_map(|e| e.region_ids.iter())
                    .collect();
                for region_id in regions_for_this {
                    // No self-default: a region the host did not assign to this
                    // family is not this planner's to plan. See `candidate_family`.
                    let assignments_empty = support_analysis.family_assignments.is_empty();
                    let Some(support_family) = (if assignments_empty {
                        Some(canonical_support_family_alias(Some(&self.support_family)))
                    } else {
                        support_analysis
                            .family_assignments
                            .iter()
                            .find(|assignment| {
                                assignment.object_id == obj.object_id
                                    && assignment.region_id == *region_id
                            })
                            .map(|assignment| {
                                canonical_support_family_alias(Some(&assignment.family_id))
                            })
                    }) else {
                        continue;
                    };
                    if support_family != "tree" {
                        continue;
                    }
                    fallback_family_emitted |= assignments_empty;
                    let model_occupancy: Vec<ExPolygon> = support_analysis
                        .model_occupancy
                        .iter()
                        .filter(|entry| {
                            entry.object_id == obj.object_id
                                && entry.global_support_layer_index == current_global_layer_index
                                && entry.region_id == *region_id
                        })
                        .flat_map(|entry| entry.polygons.iter().cloned())
                        .collect();
                    let role_collision = if model_occupancy.is_empty() {
                        collision_polys
                    } else {
                        model_occupancy.as_slice()
                    };
                    let mut roles = build_roles(
                        &branch_segments,
                        &interface_segments,
                        &floor_segments,
                        branch_radius,
                        role_collision,
                    );
                    // Keep the emitted IR subject to the same exact-Z
                    // occupancy contract as the runtime closure gate. This
                    // final guard is needed for concave occupancy where a
                    // preserved clamp position can evade the centerline test.
                    for role in &mut roles {
                        role.regions.retain(|region| {
                            host::clip_polygons(
                                std::slice::from_ref(region),
                                role_collision,
                                ClipOperation::Intersection,
                            )
                            .is_empty()
                        });
                    }
                    roles.retain(|role| !role.regions.is_empty());
                    if roles.is_empty() {
                        continue;
                    }
                    entries_in_order.push(SupportPlanEntry {
                        global_layer_index: current_global_layer_index as i32,
                        object_id: obj.object_id.clone(),
                        region_id: region_id.clone(),
                        family_id: support_family,
                        demand_ids: active_nodes
                            .iter()
                            .flat_map(|node| node.demand_ids.iter().cloned())
                            .collect(),
                        body_ids: vec![format!(
                            "tree-body-{}-{}",
                            obj.object_id, current_global_layer_index
                        )],
                        anchor_layer_index: current_global_layer_index,
                        // SupportPlanIR stores physical Z in canonical slicer
                        // units (1 unit = 100 nm), not a WIT-specific scale.
                        anchor_z: mm_to_units(z_current),
                        roles,
                        skeleton: Some(slicer_ir::SupportPlanSkeleton {
                            points: branch_segments
                                .iter()
                                .chain(interface_segments.iter())
                                .chain(floor_segments.iter())
                                .flat_map(|segment| segment.iter())
                                .map(|point| slicer_ir::Point3 {
                                    x: point.x,
                                    y: point.y,
                                    z: point.z,
                                })
                                .collect(),
                        }),
                        capabilities: vec!["tree-branch-skeleton".to_string()],
                        provenance: vec!["support-planner".to_string()],
                        decline_reason: None,
                    });
                }
            }

            // Build the "moved" node set for the next (lower) layer.
            //
            // For each surviving node, move toward the reciprocal-distance-
            // squared weighted aggregate of ALL its MST neighbours (Orca
            // `TreeSupport::drop_nodes` non-`is_strong` behaviour, packet 122).
            // Nodes without an MST edge simply propagate unchanged. The
            // existing `max_move_xy` cap and `clamp_to_avoidance` post-cap
            // are preserved: only the move *direction* changes.
            let mut next_nodes: Vec<PlannedSupportNode> = Vec::with_capacity(active_nodes.len());
            // Per-node list of (neighbour_index, edge_distance) for every
            // MST edge incident on the node. Replaces the old
            // `nearest_neighbour` / `nearest_distance` single-entry lookup.
            let mut neighbours_of: Vec<Vec<(usize, f32)>> = vec![Vec::new(); active_nodes.len()];
            for (a, b, d) in &mst_edges {
                neighbours_of[*a].push((*b, *d));
                neighbours_of[*b].push((*a, *d));
            }

            for (i, node) in active_nodes.iter().enumerate() {
                if drop[i] {
                    continue;
                }
                let neighbours = &neighbours_of[i];
                let moved = if neighbours.is_empty() {
                    // No MST edge: propagate the node unchanged.
                    PlannedSupportNode {
                        x: node.x,
                        y: node.y,
                        dist_to_top: node.dist_to_top.saturating_add(1),
                        to_buildplate: node.to_buildplate,
                        demand_ids: node.demand_ids.clone(),
                    }
                } else {
                    // Build the parallel slices for the aggregate helper.
                    let positions: Vec<(f32, f32)> = neighbours
                        .iter()
                        .map(|&(j, _)| (active_nodes[j].x, active_nodes[j].y))
                        .collect();
                    let distances: Vec<f32> = neighbours.iter().map(|&(_, d)| d).collect();
                    let (tx, ty) = aggregate_neighbour_targets(&positions, &distances)
                        .unwrap_or((node.x, node.y));

                    // Apply the existing `max_move_xy` cap to the displacement
                    // from the current node toward the aggregate target. This
                    // preserves the wall-count-scaled step cap (line 715 in
                    // the old code; packet 122 explicitly preserves it).
                    let dx = tx - node.x;
                    let dy = ty - node.y;
                    let len = (dx * dx + dy * dy).sqrt();
                    let raw_step = if len > max_move_xy && len > 1e-6 {
                        let scale = max_move_xy / len;
                        (node.x + dx * scale, node.y + dy * scale)
                    } else if len > 1e-6 {
                        (tx, ty)
                    } else {
                        (node.x, node.y)
                    };

                    // Push the node out of `avoidance_polys` if it landed inside
                    // (Step 5 AC-3).
                    let (cx, cy) = clamp_to_avoidance(raw_step.0, raw_step.1, avoidance_polys);

                    // A branch may only travel `max_move_xy` per layer — that is
                    // the branch-angle budget. Escaping avoidance is not exempt
                    // from it: the nearest point outside avoidance can be
                    // arbitrarily far away, and taking it unconditionally would
                    // teleport the branch off the overhang it supports. When the
                    // escape exceeds the budget there is no legal destination,
                    // so the node is dropped with the typed code 1002
                    // `node-clamped-out` diagnostic (AC-N3).
                    let escape_dx = cx - node.x;
                    let escape_dy = cy - node.y;
                    let escape_len = (escape_dx * escape_dx + escape_dy * escape_dy).sqrt();
                    if escape_len > max_move_xy + 1e-6 {
                        let _ = output.push_diagnostic(Diagnostic {
                            severity: DiagnosticSeverity::Warn,
                            code: 1002,
                            layer: Some(current_global_layer_index as i32),
                            object_id: Some(obj.object_id.clone()),
                            message: format!(
                                "node-clamped-out: layer={} obj={} pos=({:.3},{:.3}) escape={:.3}mm budget={:.3}mm to_buildplate={}",
                                current_global_layer_index,
                                obj.object_id,
                                cx,
                                cy,
                                escape_len,
                                max_move_xy,
                                node.to_buildplate
                            ),
                        });
                        // Preserve the last legal position when the avoidance
                        // escape is over budget, unless that position is in
                        // the next layer's occupancy. This prevents orphaned
                        // lower layers without leaking into exact-Z occupancy.
                        let next_cache_idx = current_global_layer_index.saturating_sub(1) as usize;
                        let next_collision = collision_cache
                            .get(next_cache_idx)
                            .map_or(&[][..], |cache| cache.collision_polys.as_slice());
                        let next_model_collision: Vec<ExPolygon> = support_analysis
                            .model_occupancy
                            .iter()
                            .filter(|entry| {
                                entry.object_id == obj.object_id
                                    && entry.global_support_layer_index
                                        == current_global_layer_index.saturating_sub(1)
                            })
                            .flat_map(|entry| entry.polygons.iter().cloned())
                            .collect();
                        let next_collision = if next_model_collision.is_empty() {
                            next_collision
                        } else {
                            next_model_collision.as_slice()
                        };
                        let next_radius = tapered_radius(
                            branch_radius,
                            tan_diameter_angle,
                            node.dist_to_top.saturating_add(1),
                            effective_height,
                        );
                        if body_intersects(next_collision, node.x, node.y, next_radius) {
                            continue;
                        }
                        PlannedSupportNode {
                            x: node.x,
                            y: node.y,
                            dist_to_top: node.dist_to_top.saturating_add(1),
                            to_buildplate: node.to_buildplate,
                            demand_ids: node.demand_ids.clone(),
                        }
                    } else {
                        PlannedSupportNode {
                            x: cx,
                            y: cy,
                            // dist_to_top increments as we move down
                            dist_to_top: node.dist_to_top.saturating_add(1),
                            to_buildplate: node.to_buildplate,
                            demand_ids: node.demand_ids.clone(),
                        }
                    }
                };
                next_nodes.push(moved);
            }

            active_nodes = next_nodes;
        }

        // Do not smooth after exact-Z collision validation. `smooth_branches`
        // translates emitted role polygons, which can move a previously legal
        // body back into concave model occupancy without another validation
        // pass.

        // ── Packet 118 B4: cap drops are merged into the shared map ──────
        // (Emission happens in run_support_geometry after all objects are
        // processed, so the diagnostic is one per affected global layer
        // across all objects, not one per (object, layer) pair.)

        // Emit entries in top-to-bottom order.
        for entry in entries_in_order {
            output
                .push_support_plan_entry(entry)
                .map_err(|e| ModuleError::fatal(1, format!("push_support_plan failed: {e}")))?;
        }
        if fallback_family_emitted {
            let _ = output.push_diagnostic(Diagnostic {
                severity: DiagnosticSeverity::Warn,
                code: 1004,
                layer: None,
                object_id: Some(obj.object_id.clone()),
                message: "support-planner: no family assignments; using configured support family"
                    .to_string(),
            });
        }
        Ok(())
    }
}

fn candidate_contact_point(polygons: &[ExPolygon]) -> Option<(f32, f32)> {
    let mut count = 0_i64;
    let mut x = 0_i64;
    let mut y = 0_i64;
    for polygon in polygons {
        for point in &polygon.contour.points {
            x += point.x;
            y += point.y;
            count += 1;
        }
    }
    (count > 0).then(|| (units_to_mm(x / count), units_to_mm(y / count)))
}

/// Sample overhangs using canonical corner, arc, and rotated-interior streams.
///
/// `grid_bounds` is the object bounding box in mm `(min_x, max_x, min_y,
/// max_y)`. Canonical `TreeSupport::generate_contact_points` builds the
/// interior grid once per object over the whole-object bounding box, rotated
/// 22 degrees about the object bbox center, and filters per overhang. Deriving
/// the index span from the overhang bbox instead would drop lattice points
/// whose rotated images fall inside the overhang near corners. `None` falls
/// back to the overhang bbox (mesh-less objects).
fn sample_contact_points(
    polygons: &[ExPolygon],
    origin: (f32, f32),
    grid_bounds: Option<(f32, f32, f32, f32)>,
    point_spread: f32,
    base_radius: f32,
) -> Vec<(f32, f32)> {
    let mut result = Vec::new();
    let mut buckets = std::collections::HashSet::new();
    let cell = mm_to_units(base_radius).max(1) + 1;
    let mut add = |x: f32, y: f32| {
        let key = (
            mm_to_units(x).div_euclid(cell),
            mm_to_units(y).div_euclid(cell),
        );
        if buckets.insert(key) {
            result.push((x, y));
        }
    };

    for polygon in polygons {
        let points = &polygon.contour.points;
        if points.len() < 3 {
            continue;
        }
        for i in 0..points.len() {
            let previous = points[(i + points.len() - 1) % points.len()];
            let current = points[i];
            let next = points[(i + 1) % points.len()];
            let a = (
                (previous.x - current.x) as f32,
                (previous.y - current.y) as f32,
            );
            let b = ((next.x - current.x) as f32, (next.y - current.y) as f32);
            let lengths = (a.0 * a.0 + a.1 * a.1).sqrt() * (b.0 * b.0 + b.1 * b.1).sqrt();
            if lengths > 0.0 && (a.0 * b.0 + a.1 * b.1) / lengths > -0.7 {
                add(units_to_mm(current.x), units_to_mm(current.y));
            }
        }

        for ring in std::iter::once(&polygon.contour).chain(polygon.holes.iter()) {
            let mut cumulative = 0.0;
            let mut edges = Vec::with_capacity(ring.points.len());
            for i in 0..ring.points.len() {
                let a = ring.points[i];
                let b = ring.points[(i + 1) % ring.points.len()];
                let length = (((b.x - a.x) as f32).powi(2) + ((b.y - a.y) as f32).powi(2)).sqrt();
                edges.push((a, b, cumulative, length));
                cumulative += length;
            }
            if cumulative > 0.0 && point_spread > 0.0 {
                let mut distance = 0.0;
                while distance < cumulative {
                    if let Some((a, b, start, length)) = edges
                        .iter()
                        .find(|(_, _, start, length)| distance < *start + *length)
                    {
                        let t = (distance - *start) / *length;
                        add(
                            units_to_mm((a.x as f32 + (b.x - a.x) as f32 * t) as i64),
                            units_to_mm((a.y as f32 + (b.y - a.y) as f32 * t) as i64),
                        );
                    }
                    distance += mm_to_units(point_spread) as f32;
                }
            }
        }
    }

    let eroded = host::offset_polygons(polygons, -base_radius, OffsetJoinType::Miter, 0.0);
    let (min_x, max_x, min_y, max_y) = match grid_bounds {
        Some(bounds) => bounds,
        None => {
            let (min_x, max_x, min_y, max_y) =
                polygons.iter().flat_map(|p| p.contour.points.iter()).fold(
                    (i64::MAX, i64::MIN, i64::MAX, i64::MIN),
                    |(min_x, max_x, min_y, max_y), point| {
                        (
                            min_x.min(point.x),
                            max_x.max(point.x),
                            min_y.min(point.y),
                            max_y.max(point.y),
                        )
                    },
                );
            (
                units_to_mm(min_x),
                units_to_mm(max_x),
                units_to_mm(min_y),
                units_to_mm(max_y),
            )
        }
    };
    let step = point_spread.max(DEFAULT_MAX_BRIDGE_LENGTH_MM / 2.0);
    let cos = 22.0_f32.to_radians().cos();
    let sin = 22.0_f32.to_radians().sin();
    let ix_min = (((min_x - origin.0) / step).floor() as i32) - 1;
    let ix_max = (((max_x - origin.0) / step).ceil() as i32) + 1;
    let iy_min = (((min_y - origin.1) / step).floor() as i32) - 1;
    let iy_max = (((max_y - origin.1) / step).ceil() as i32) + 1;
    for iy in iy_min..=iy_max {
        for ix in ix_min..=ix_max {
            let x = origin.0 + (ix as f32 * step) * cos - (iy as f32 * step) * sin;
            let y = origin.1 + (ix as f32 * step) * sin + (iy as f32 * step) * cos;
            if point_in_any_expoly(&eroded, x, y) {
                add(x, y);
            }
        }
    }
    result
}

/// Lower a flush contact layer by `support_top_z_distance_mm`, measured along
/// actual layer Z.
///
/// `contact_layer` is the layer whose top plane the support would touch with a
/// zero gap — i.e. the overhang plane. The column must instead stop at the
/// highest layer whose top plane is at or below `overhang_plane_z - gap`.
///
/// The walk is over `LayerPlanViewEntry.z` on purpose. Do **not** reimplement
/// this as `gap / effective_layer_height`: the host's two producers of that
/// field disagree (`project_layer_plan_view` takes a max over participating
/// objects, `build_native_prepass_request` takes the first match), so it is not
/// a dependable per-layer thickness in the guest view. Dividing by it opened a
/// 35-layer gap here. `traditional-support-planner::plan_candidate` walks Z for
/// the same reason.
fn contact_layer_after_top_gap(
    layer_plan: &LayerPlanView,
    contact_layer: usize,
    support_top_z_distance_mm: f32,
) -> usize {
    if support_top_z_distance_mm <= 0.0 || layer_plan.layers.is_empty() {
        return contact_layer;
    }
    let contact_layer = contact_layer.min(layer_plan.layers.len() - 1);
    let overhang_plane_z = layer_plan.layers[contact_layer].z;
    let target_top_z = overhang_plane_z - support_top_z_distance_mm;
    let mut emit_top_layer = contact_layer;
    while emit_top_layer > 0 && layer_plan.layers[emit_top_layer].z > target_top_z {
        emit_top_layer -= 1;
    }
    emit_top_layer
}

fn push_contact(
    contacts: &mut [Vec<PlannedSupportNode>],
    layer_plan: &LayerPlanView,
    collision_cache: &[LayerCollisionCache],
    layer_idx: usize,
    x: f32,
    y: f32,
    planner: &SupportPlanner,
    dropped: &mut std::collections::BTreeMap<u32, usize>,
) {
    // Name the demand after the layer the contact will actually land on, so
    // the id stays stable against the gap shift applied below.
    let target_idx =
        contact_layer_after_top_gap(layer_plan, layer_idx, planner.support_top_z_distance_mm);
    push_contact_with_demand(
        contacts,
        layer_plan,
        collision_cache,
        layer_idx,
        x,
        y,
        planner,
        dropped,
        format!(
            "mesh-demand-{}-{}",
            layer_plan.layers[target_idx].global_layer_index,
            contacts[target_idx].len()
        ),
    );
}

fn push_contact_with_demand(
    contacts: &mut [Vec<PlannedSupportNode>],
    layer_plan: &LayerPlanView,
    collision_cache: &[LayerCollisionCache],
    layer_idx: usize,
    x: f32,
    y: f32,
    planner: &SupportPlanner,
    dropped: &mut std::collections::BTreeMap<u32, usize>,
    demand_id: String,
) {
    // Hold the top Z gap: a contact belongs `support_top_z_distance_mm` BELOW
    // the overhang that demanded it, not flush against it.
    let layer_idx =
        contact_layer_after_top_gap(layer_plan, layer_idx, planner.support_top_z_distance_mm);
    let global_layer = layer_plan.layers[layer_idx].global_layer_index;
    let collision = collision_cache
        .get(global_layer as usize)
        .map_or(&[][..], |cache| cache.collision_polys.as_slice());
    let to_buildplate = !point_in_any_expoly(collision, x, y);
    if planner.support_on_build_plate_only && !to_buildplate {
        return;
    }
    if contacts[layer_idx].len() >= planner.max_branches_per_layer {
        *dropped.entry(global_layer).or_insert(0) += 1;
        return;
    }
    contacts[layer_idx].push(PlannedSupportNode {
        x,
        y,
        dist_to_top: 0,
        to_buildplate,
        demand_ids: vec![demand_id],
    });
}

fn push_analysis_contact(
    contacts: &mut [Vec<PlannedSupportNode>],
    layer_plan: &LayerPlanView,
    collision_cache: &[LayerCollisionCache],
    layer_idx: usize,
    x: f32,
    y: f32,
    planner: &SupportPlanner,
    dropped: &mut std::collections::BTreeMap<u32, usize>,
    demand_id: String,
) {
    // Analysis candidates already carry the host-selected contact layer.
    // Applying the gap again moves sampled geometry off its demand layer.
    let layer_idx = layer_idx.min(layer_plan.layers.len().saturating_sub(1));
    let global_layer = layer_plan.layers[layer_idx].global_layer_index;
    let collision = collision_cache
        .get(global_layer as usize)
        .map_or(&[][..], |cache| cache.collision_polys.as_slice());
    let to_buildplate = !point_in_any_expoly(collision, x, y);
    if planner.support_on_build_plate_only && !to_buildplate {
        return;
    }
    if contacts[layer_idx].len() >= planner.max_branches_per_layer {
        *dropped.entry(global_layer).or_insert(0) += 1;
        return;
    }
    contacts[layer_idx].push(PlannedSupportNode {
        x,
        y,
        dist_to_top: 0,
        to_buildplate,
        demand_ids: vec![demand_id],
    });
}

/// Resolve the global support selection to the family vocabulary shared by
/// the planner and both renderers. Orca-style `support_type` aliases remain
/// accepted, with the legacy key taking precedence when both are present.
fn canonical_support_family(config: &ConfigView) -> String {
    let value = config
        .get("support_type")
        .or_else(|| config.get("support_family"))
        .and_then(|value| match value {
            ConfigValue::String(value) => Some(value.as_str()),
            _ => None,
        });
    value
        .map(|value| canonical_support_family_alias(Some(value)))
        .unwrap_or_else(|| "tree-support".to_string())
}

fn canonical_support_family_alias(value: Option<&str>) -> String {
    slicer_ir::canonical_support_family(value).to_string()
}

/// Resolve the canonical support family for a candidate from the host's
/// per-region family assignments.
///
/// Returns the planner's configured family only when the host supplied no
/// assignments at all. When assignments exist, an unmatched region remains
/// unassigned so another family can own it.
fn candidate_family(
    candidate: &SupportAnalysisCandidate,
    analysis: &SupportAnalysisView,
    fallback: &str,
) -> Option<String> {
    if analysis.family_assignments.is_empty() {
        return Some(canonical_support_family_alias(Some(fallback)));
    }
    analysis
        .family_assignments
        .iter()
        .find(|assignment| {
            assignment.object_id == candidate.object_id
                && assignment.region_id == candidate.region_id
        })
        .map(|assignment| canonical_support_family_alias(Some(&assignment.family_id)))
}

/// Group `SupportPlanEntry` indices by `(object_id, region_id)`, each group
/// sorted by `global_layer_index` descending (tip → root). Returns the list of
/// index groups referencing positions in the original `entries` slice.
pub fn group_branches_into_columns(
    entries: &[slicer_sdk::prepass_types::SupportPlanEntry],
) -> Vec<Vec<usize>> {
    let mut groups: std::collections::BTreeMap<
        (
            slicer_sdk::prepass_types::ObjectId,
            slicer_sdk::prepass_types::RegionId,
        ),
        Vec<usize>,
    > = std::collections::BTreeMap::new();
    for (idx, entry) in entries.iter().enumerate() {
        groups
            .entry((entry.object_id.clone(), entry.region_id.clone()))
            .or_default()
            .push(idx);
    }
    let mut columns: Vec<Vec<usize>> = groups.into_values().collect();
    for col in columns.iter_mut() {
        col.sort_by(|&a, &b| {
            entries[b]
                .global_layer_index
                .cmp(&entries[a].global_layer_index)
        });
    }
    columns
}

/// Returns `(x, y, width)` of the first structural skeleton point, if present.
fn first_point_xyw(entry: &slicer_sdk::prepass_types::SupportPlanEntry) -> Option<(f32, f32, f32)> {
    entry
        .skeleton
        .as_ref()
        .and_then(|skeleton| skeleton.points.first())
        .map(|point| (point.x, point.y, 0.0))
}

/// Rust port of Orca's `TreeSupport::smooth_nodes`. Applies an in-place
/// three-point Laplacian smoother to each `(object_id, region_id)` column of
/// structural skeleton rows. Endpoints are held fixed.
pub fn smooth_branches(
    entries: &mut Vec<slicer_sdk::prepass_types::SupportPlanEntry>,
    iterations: usize,
) {
    if entries.is_empty() {
        return;
    }
    // Heuristic: branches in different support trees are typically separated by
    // 25mm+; per-layer stairsteps are 1-2mm. 5mm comfortably separates "tree"
    // from "stairstep" without affecting legitimate smoothing within a single
    // tree.
    const CHAIN_BREAK_THRESHOLD_MM: f32 = 5.0;
    let columns = group_branches_into_columns(entries);
    for column in columns {
        if column.len() < 3 {
            continue;
        }
        // Split each column into sub-chains at gaps > CHAIN_BREAK_THRESHOLD_MM
        // between consecutive (x, y) points. Distinct support trees merged into
        // one region column must not be smoothed across their topological
        // discontinuity. Sub-chain boundaries act as additional pinning points.
        let mut sub_starts: Vec<usize> = vec![0usize];
        for k in 1..column.len() {
            let a = match first_point_xyw(&entries[column[k - 1]]) {
                Some(p) => p,
                None => break,
            };
            let b = match first_point_xyw(&entries[column[k]]) {
                Some(p) => p,
                None => break,
            };
            let dx = b.0 - a.0;
            let dy = b.1 - a.1;
            if (dx * dx + dy * dy).sqrt() > CHAIN_BREAK_THRESHOLD_MM {
                sub_starts.push(k);
            }
        }
        sub_starts.push(column.len());
        for w in sub_starts.windows(2) {
            let (s, e) = (w[0], w[1]);
            if e - s < 3 {
                continue;
            }
            for _ in 0..iterations {
                for i in (s + 1)..(e - 1) {
                    let prev = match first_point_xyw(&entries[column[i - 1]]) {
                        Some(p) => p,
                        None => continue,
                    };
                    let cur = match first_point_xyw(&entries[column[i]]) {
                        Some(p) => p,
                        None => continue,
                    };
                    let next = match first_point_xyw(&entries[column[i + 1]]) {
                        Some(p) => p,
                        None => continue,
                    };
                    let new_x = (prev.0 + cur.0 + next.0) / 3.0;
                    let new_y = (prev.1 + cur.1 + next.1) / 3.0;
                    // Move the printed geometry with the skeleton. Smoothing
                    // used to mutate `skeleton.points[0]` only — a field no
                    // renderer reads — so it changed nothing that gets printed.
                    let dx = new_x - cur.0;
                    let dy = new_y - cur.1;
                    let entry = &mut entries[column[i]];
                    if let Some(skeleton) = entry.skeleton.as_mut() {
                        for point in skeleton.points.iter_mut() {
                            point.x += dx;
                            point.y += dy;
                        }
                    }
                    let dx_units = mm_to_units(dx);
                    let dy_units = mm_to_units(dy);
                    if dx_units != 0 || dy_units != 0 {
                        for role in entry.roles.iter_mut() {
                            for expoly in role.regions.iter_mut() {
                                for ring in std::iter::once(&mut expoly.contour)
                                    .chain(expoly.holes.iter_mut())
                                {
                                    for point in ring.points.iter_mut() {
                                        point.x += dx_units;
                                        point.y += dy_units;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn compute_bounds(vertices: &[[f32; 3]]) -> Option<([f32; 3], [f32; 3])> {
    if vertices.is_empty() {
        return None;
    }
    let mut mn = vertices[0];
    let mut mx = vertices[0];
    for v in vertices.iter().skip(1) {
        mn[0] = mn[0].min(v[0]);
        mn[1] = mn[1].min(v[1]);
        mn[2] = mn[2].min(v[2]);
        mx[0] = mx[0].max(v[0]);
        mx[1] = mx[1].max(v[1]);
        mx[2] = mx[2].max(v[2]);
    }
    Some((mn, mx))
}

fn detect_overhang_facets(
    obj: &MeshObjectView,
    threshold_deg: f32,
) -> Vec<([f32; 3], [f32; 3], [f32; 3])> {
    // Triangles whose downward-facing normal z-component is below
    // `-sin(threshold_deg)` are overhang facets. OrcaSlicer uses the
    // same z-normal threshold in `detect_overhangs`.
    let threshold_nz = -(threshold_deg.to_radians().sin());
    let mut result = Vec::new();
    for triangle in &obj.triangles {
        let v0 = obj.vertices[triangle[0] as usize];
        let v1 = obj.vertices[triangle[1] as usize];
        let v2 = obj.vertices[triangle[2] as usize];
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
        let nz = e1[0] * e2[1] - e1[1] * e2[0];
        let nx = e1[1] * e2[2] - e1[2] * e2[1];
        let ny = e1[2] * e2[0] - e1[0] * e2[2];
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        if len < 1e-8 {
            continue;
        }
        let nz_unit = nz / len;
        if nz_unit <= threshold_nz {
            result.push((v0, v1, v2));
        }
    }
    result
}

/// Collect support-enforcer contact centroids from the object's paint layers.
///
/// A `PaintLayerView` with `semantic == "support_enforcer"` has per-facet
/// flag values; every facet whose flag is `Some(true)` contributes a
/// contact point at its centroid. The `layer_idx` field on the paint layer
/// (derived from the host-side `PaintRegionIR.per_layer`) pins each contact
/// to its origin global-layer index.
fn collect_paint_enforcer_contacts(obj: &MeshObjectView) -> Vec<(u32, f32, f32)> {
    let mut result = Vec::new();
    for (paint_layer_idx, layer) in obj.paint_layers.iter().enumerate() {
        if layer.semantic != "support_enforcer" && layer.semantic != "SupportEnforcer" {
            continue;
        }
        for (facet_idx, value) in layer.facet_values.iter().enumerate() {
            let active = matches!(value.as_ref().and_then(|v| v.flag), Some(true));
            if !active {
                continue;
            }
            if facet_idx >= obj.triangles.len() {
                continue;
            }
            let triangle = &obj.triangles[facet_idx];
            let v0 = obj.vertices[triangle[0] as usize];
            let v1 = obj.vertices[triangle[1] as usize];
            let v2 = obj.vertices[triangle[2] as usize];
            let cx = (v0[0] + v1[0] + v2[0]) / 3.0;
            let cy = (v0[1] + v1[1] + v2[1]) / 3.0;
            result.push((paint_layer_idx as u32, cx, cy));
        }
    }
    result
}

fn collect_paint_blocker_polygons(obj: &MeshObjectView) -> Vec<Vec<[f32; 2]>> {
    // The support-planner sees paint values per facet on per-layer `PaintLayerView`s.
    // Support-blocker semantics mask out facets whose flag is true; we collect their
    // triangle centroids as a 1-point "polygon" so `point_in_any_polygon` can reject
    // any contact that falls close to a blocker facet.
    let mut result = Vec::new();
    for layer in obj.paint_layers.iter() {
        if layer.semantic != "support_blocker" && layer.semantic != "SupportBlocker" {
            continue;
        }
        for (facet_idx, value) in layer.facet_values.iter().enumerate() {
            let active = matches!(value.as_ref().and_then(|v| v.flag), Some(true));
            if !active {
                continue;
            }
            if facet_idx >= obj.triangles.len() {
                continue;
            }
            let triangle = &obj.triangles[facet_idx];
            let v0 = obj.vertices[triangle[0] as usize];
            let v1 = obj.vertices[triangle[1] as usize];
            let v2 = obj.vertices[triangle[2] as usize];
            // Treat the triangle as a 2D polygon projected onto XY.
            result.push(vec![[v0[0], v0[1]], [v1[0], v1[1]], [v2[0], v2[1]]]);
        }
    }
    result
}

fn point_in_any_polygon(polygons: &[Vec<[f32; 2]>], x: f32, y: f32) -> bool {
    polygons.iter().any(|poly| point_in_polygon(poly, x, y))
}

fn point_in_any_expoly(polygons: &[ExPolygon], x: f32, y: f32) -> bool {
    let sx = x * SCALING_FACTOR as f32;
    let sy = y * SCALING_FACTOR as f32;
    polygons.iter().any(|ex| {
        let outer: Vec<[f32; 2]> = ex
            .contour
            .points
            .iter()
            .map(|p| [p.x as f32, p.y as f32])
            .collect();
        point_in_polygon(&outer, sx, sy)
            && !ex.holes.iter().any(|h| {
                point_in_polygon(
                    &h.points
                        .iter()
                        .map(|p| [p.x as f32, p.y as f32])
                        .collect::<Vec<_>>(),
                    sx,
                    sy,
                )
            })
    })
}

/// Conservative full-footprint collision test. A center-only test is unsafe
/// for tapered branches because the radius can cross a model boundary while
/// the center remains outside it.
/// Return whether the complete circular body at `(x, y)` overlaps occupancy.
/// This is the same predicate used before branch-body emission.
pub fn body_overlaps_occupancy(polygons: &[ExPolygon], x: f32, y: f32, radius_mm: f32) -> bool {
    if point_in_any_expoly(polygons, x, y) {
        return true;
    }
    let qx = x * SCALING_FACTOR as f32;
    let qy = y * SCALING_FACTOR as f32;
    let radius = mm_to_units(radius_mm.max(MIN_BRANCH_RADIUS)) as f32;
    polygons.iter().any(|ex| {
        let poly: Vec<[f32; 2]> = ex
            .contour
            .points
            .iter()
            .map(|p| [p.x as f32, p.y as f32])
            .collect();
        if poly.len() < 3 {
            return false;
        }
        let (_closest, distance) = closest_point_on_polygon(&poly, qx, qy);
        // The disc overlaps when its centre is inside the outline, or when the
        // outline comes within one radius of the centre. A previous fourth
        // clause asked `point_in_polygon(closest)` — whether the closest point
        // ON the boundary is inside — which is decided by floating-point
        // accident at the boundary and answered "overlapping" for a body
        // arbitrarily far away, rejecting every branch near any occupancy.
        distance <= radius
            || poly.iter().any(|p| {
                let dx = p[0] - qx;
                let dy = p[1] - qy;
                dx * dx + dy * dy <= radius * radius
            })
            || point_in_polygon(&poly, qx, qy)
    })
}

fn body_intersects(polygons: &[ExPolygon], x: f32, y: f32, radius_mm: f32) -> bool {
    body_overlaps_occupancy(polygons, x, y, radius_mm)
}

/// Check the complete emitted capsule, not just its endpoint discs. A branch
/// can cross an obstacle between two individually clear endpoints.
fn body_segment_intersects(
    polygons: &[ExPolygon],
    a: &PlannedSupportNode,
    b: &PlannedSupportNode,
    radius_a: f32,
    radius_b: f32,
) -> bool {
    let Some(segment) = swept_region(
        &Point3WithWidth {
            x: a.x,
            y: a.y,
            z: 0.0,
            width: radius_a * 2.0,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
            overhang_distance_mm: None,
        },
        &Point3WithWidth {
            x: b.x,
            y: b.y,
            z: 0.0,
            width: radius_b * 2.0,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
            overhang_distance_mm: None,
        },
    ) else {
        return false;
    };
    !host::clip_polygons(
        std::slice::from_ref(&segment),
        polygons,
        ClipOperation::Intersection,
    )
    .is_empty()
}

/// Ray-casting point-in-polygon test: returns true if (x, y) is inside `poly`.
pub fn point_in_polygon(poly: &[[f32; 2]], x: f32, y: f32) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        if (pi[1] > y) != (pj[1] > y) {
            let x_intersect = (pj[0] - pi[0]) * (y - pi[1]) / (pj[1] - pi[1]) + pi[0];
            if x < x_intersect {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Prim's minimum spanning tree. Returns `(a_idx, b_idx, distance)` tuples.
///
/// Matches OrcaSlicer's `MinimumSpanningTree::prim` complexity class (O(V²)).
/// The `V` input here is the propagated node count, bounded by
/// `support_max_branches_per_layer`.
fn prim_mst(nodes: &[PlannedSupportNode]) -> Vec<(usize, usize, f32)> {
    let n = nodes.len();
    if n < 2 {
        return Vec::new();
    }
    let mut in_tree = vec![false; n];
    let mut min_dist = vec![f32::INFINITY; n];
    let mut parent: Vec<Option<usize>> = vec![None; n];

    in_tree[0] = true;
    for i in 1..n {
        let d = euclidean_distance(&nodes[0], &nodes[i]);
        min_dist[i] = d;
        parent[i] = Some(0);
    }

    let mut edges = Vec::with_capacity(n - 1);
    for _ in 1..n {
        let mut best = None;
        let mut best_dist = f32::INFINITY;
        for i in 0..n {
            if !in_tree[i] && min_dist[i] < best_dist {
                best_dist = min_dist[i];
                best = Some(i);
            }
        }
        let Some(next) = best else { break };
        in_tree[next] = true;
        if let Some(p) = parent[next] {
            let a = next.min(p);
            let b = next.max(p);
            edges.push((a, b, best_dist));
        }
        for i in 0..n {
            if !in_tree[i] {
                let d = euclidean_distance(&nodes[next], &nodes[i]);
                if d < min_dist[i] {
                    min_dist[i] = d;
                    parent[i] = Some(next);
                }
            }
        }
    }
    edges.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
    });
    edges
}

fn euclidean_distance(a: &PlannedSupportNode, b: &PlannedSupportNode) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// Reciprocal-distance-squared weighted aggregate of MST-neighbour positions.
///
/// Pure math helper used by the propagation block in `plan_for_object` to
/// synthesise the move target for a node from ALL its MST neighbours at once
/// (replacing the old single-neighbour lookup). Matches OrcaSlicer's
/// `TreeSupport::drop_nodes` non-`is_strong` aggregation: each neighbour's
/// position is weighted by `1.0 / D_j²` where `D_j` is the MST edge distance
/// from the central node to neighbour `j`. Weights are normalised so they
/// sum to 1.0. With equal `D_j`s (symmetric fan) the aggregate equals the
/// geometric centroid; with one close neighbour the close neighbour
/// dominates (1/d² is a strong bias).
///
/// Degenerate `D_j < 1e-6 mm` (coincident point): weight saturates to
/// infinity; implementation short-circuits and returns that neighbour's
/// position directly. This avoids the divide-by-zero path AND the unstable
/// "huge weight / huge denominator" path that would otherwise depend on
/// floating-point ordering of the sum.
///
/// Empty input → `None`. Single-element input → that element's position.
///
/// Reference: OrcaSlicer `TreeSupport::drop_nodes` (the second-pass move
/// step), `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`. The
/// packet 122 design reconciles Orca's 1/d² weighting with the implementation.
pub fn aggregate_neighbour_targets(
    neighbour_positions: &[(f32, f32)],
    distances: &[f32],
) -> Option<(f32, f32)> {
    debug_assert_eq!(
        neighbour_positions.len(),
        distances.len(),
        "neighbour_positions and distances must be parallel slices"
    );
    if neighbour_positions.is_empty() {
        return None;
    }
    if neighbour_positions.len() == 1 {
        return Some(neighbour_positions[0]);
    }
    // Degenerate-collision short-circuit: any D_j below the epsilon collapses
    // the aggregate to that neighbour's position.
    const EPS_MM: f32 = 1e-6;
    for &d in distances {
        if d < EPS_MM {
            // Find the matching position. Multiple zeros are possible; pick
            // the first — the test asserts it does not panic and the result
            // equals ONE of the zero-distance neighbours' positions.
            for (idx, &dd) in distances.iter().enumerate() {
                if dd < EPS_MM {
                    return Some(neighbour_positions[idx]);
                }
            }
        }
    }
    // 1/d² weighted mean.
    let mut sum_wx = 0.0_f64;
    let mut sum_wy = 0.0_f64;
    let mut sum_w = 0.0_f64;
    for (idx, &(nx, ny)) in neighbour_positions.iter().enumerate() {
        let d = distances[idx] as f64;
        let w = 1.0 / (d * d);
        sum_wx += w * (nx as f64);
        sum_wy += w * (ny as f64);
        sum_w += w;
    }
    if sum_w <= 0.0 {
        // Defensive: should not happen given the short-circuit above, but
        // if all distances are non-finite or NaN we fall back to the
        // unweighted centroid of the neighbour positions.
        let n = neighbour_positions.len() as f64;
        let mx = neighbour_positions.iter().map(|p| p.0 as f64).sum::<f64>() / n;
        let my = neighbour_positions.iter().map(|p| p.1 as f64).sum::<f64>() / n;
        return Some((mx as f32, my as f32));
    }
    Some(((sum_wx / sum_w) as f32, (sum_wy / sum_w) as f32))
}

// ── Step-5 helper functions ───────────────────────────────────────────────────

/// Compute tapered radius at a node given its distance from the top of the column.
///
/// Two-piece tip-cone formula:
/// - If `mm_to_top <= branch_radius`: radius = mm_to_top (linearly widen from 0 at the tip
///   to `branch_radius` at the cone base).
/// - Otherwise: radius = branch_radius + (mm_to_top - branch_radius) * tan_diameter_angle
///   (continue the same slope above the cone).
///   Clamped to `[MIN_BRANCH_RADIUS, MAX_BRANCH_RADIUS_MM]`.
pub fn tapered_radius(
    branch_radius: f32,
    tan_diameter_angle: f32,
    dist_to_top: u32,
    effective_layer_height: f32,
) -> f32 {
    let mm_to_top = (dist_to_top as f32) * effective_layer_height;
    let raw = if mm_to_top <= branch_radius {
        mm_to_top
    } else {
        branch_radius + (mm_to_top - branch_radius) * tan_diameter_angle
    };
    raw.clamp(MIN_BRANCH_RADIUS, MAX_BRANCH_RADIUS_MM)
}

/// Clamp a point into the union of avoidance polygons.
/// Returns the original point if avoidance_polys is empty; otherwise returns
/// the closest point on any avoidance polygon boundary.
fn clamp_to_avoidance(x: f32, y: f32, avoidance_polys: &[ExPolygon]) -> (f32, f32) {
    if avoidance_polys.is_empty() {
        return (x, y);
    }
    // `avoidance_polys` is the model outline inflated by `branch_radius +
    // branch_distance/2` — the region a branch must stay *out* of, matching
    // canonical tree support's avoidance semantics. A node already outside it
    // is safe and must be left where it is; only a node inside is pushed out to
    // the nearest boundary point.
    //
    // This guard was inverted before packet 224: it returned early for nodes
    // *inside* avoidance and snapped every node *outside* it onto the boundary,
    // so branches descending through open space were dragged into the model
    // each layer instead of descending freely, and died a few layers below
    // their contact.
    if !point_in_any_expoly(avoidance_polys, x, y) {
        return (x, y);
    }
    let mut best_dist = f32::INFINITY;
    let mut best = (x, y);
    let query_x_internal = x * SCALING_FACTOR as f32;
    let query_y_internal = y * SCALING_FACTOR as f32;
    for ex in avoidance_polys {
        let poly: Vec<[f32; 2]> = ex
            .contour
            .points
            .iter()
            .map(|p| [p.x as f32, p.y as f32])
            .collect();
        if poly.len() < 3 {
            continue;
        }
        let (cp, cd) = closest_point_on_polygon(&poly, query_x_internal, query_y_internal);
        if cd < best_dist {
            best_dist = cd;
            best = (cp[0] / SCALING_FACTOR as f32, cp[1] / SCALING_FACTOR as f32);
        }
    }
    best
}

/// Returns the closest point on polygon boundary to (x, y) and its squared distance.
fn closest_point_on_polygon(poly: &[[f32; 2]], x: f32, y: f32) -> ([f32; 2], f32) {
    let n = poly.len();
    let mut min_dist = f32::INFINITY;
    let mut closest = [x, y];

    for i in 0..n {
        let p0 = poly[i];
        let p1 = poly[(i + 1) % n];
        let cp = closest_point_on_segment(p0, p1, [x, y]);
        let dx = cp[0] - x;
        let dy = cp[1] - y;
        let d = (dx * dx + dy * dy).sqrt();
        if d < min_dist {
            min_dist = d;
            closest = cp;
        }
    }
    (closest, min_dist)
}

/// Closest point on line segment p0→p1 to target point t.
fn closest_point_on_segment(p0: [f32; 2], p1: [f32; 2], t: [f32; 2]) -> [f32; 2] {
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-10 {
        return p0;
    }
    let tdx = t[0] - p0[0];
    let tdy = t[1] - p0[1];
    let mut tt = (tdx * dx + tdy * dy) / len_sq;
    tt = tt.clamp(0.0, 1.0);
    [p0[0] + tt * dx, p0[1] + tt * dy]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assign every region of `object_id` to the tree family.
    ///
    /// `PrePass::SupportAnalysis` is the single authority for a region's
    /// family. The planner no longer falls back to its own identity when an
    /// assignment is missing, because that fallback let it publish a full plan
    /// for regions region routing had assigned to the traditional family.
    /// `RegionSegmentationView::region_support_configs` is marshalled by the
    /// host but read by neither planner, so it cannot stand in for this.
    fn tree_analysis(object_id: &str, region_ids: &[&str]) -> SupportAnalysisView {
        SupportAnalysisView {
            family_assignments: region_ids
                .iter()
                .map(
                    |region_id| slicer_sdk::prepass_types::SupportFamilyAssignment {
                        object_id: object_id.to_string(),
                        region_id: region_id.to_string(),
                        family_id: "tree".to_string(),
                    },
                )
                .collect(),
            ..Default::default()
        }
    }

    fn default_planner() -> SupportPlanner {
        SupportPlanner {
            enabled: true,
            support_family: "tree".to_string(),
            branch_angle_deg: DEFAULT_BRANCH_ANGLE_DEG,
            merge_distance_mm: DEFAULT_MERGE_DISTANCE_MM,
            max_branches_per_layer: DEFAULT_MAX_BRANCHES_PER_LAYER,
            line_width_mm: DEFAULT_LINE_WIDTH_MM,
            tree_support_branch_diameter: 5.0,
            tree_support_branch_diameter_angle: 5.0,
            tree_support_branch_distance: 1.0,
            tree_support_wall_count: 1,
            support_raft_layers: 0,
            raft_first_layer_density: 0.4,
            base_raft_layers: 1,
            interface_raft_layers: 0,
            support_interface_top_layers: 2,
            support_interface_bottom_layers: -1,
            support_on_build_plate_only: false,
            support_top_z_distance_mm: DEFAULT_TOP_Z_DISTANCE_MM,
        }
    }

    fn default_layer_plan(num_layers: u32, base_z: f32, layer_height: f32) -> LayerPlanView {
        LayerPlanView {
            layers: (0..num_layers)
                .map(|i| LayerPlanViewEntry {
                    global_layer_index: i,
                    z: base_z + (i as f32 + 1.0) * layer_height,
                    effective_layer_height: layer_height,
                })
                .collect(),
        }
    }

    fn default_region_segmentation(object_id: &str, num_layers: u32) -> RegionSegmentationView {
        RegionSegmentationView {
            entries: (0..num_layers)
                .map(|i| RegionSegmentationViewEntry {
                    object_id: object_id.to_string(),
                    layer_index: i,
                    region_ids: vec!["0".to_string()],
                })
                .collect(),
            region_support_configs: Vec::new(),
        }
    }

    #[test]
    fn empty_objects_emits_nothing() {
        let planner = default_planner();
        let lp = default_layer_plan(10, 0.0, 0.2);
        let rs = default_region_segmentation("plate", 10);
        let sg = SupportGeometryView { entries: vec![] };
        let mut output = SupportGeometryOutput::new();
        planner
            .run_support_geometry(&[], &lp, &rs, &sg, &mut output, &ConfigView::default())
            .unwrap();
        assert!(output.entries().is_empty());
    }

    #[test]
    fn cube_with_no_overhangs_emits_empty_plan() {
        // A simple cube with all faces either vertical or top/bottom —
        // no overhangs ⇒ no plan entries.
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let triangles = vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let obj = MeshObjectView {
            object_id: "cube".to_string(),
            vertices,
            triangles,
            paint_layers: vec![],
        };
        let planner = default_planner();
        let lp = default_layer_plan(10, 0.0, 0.2);
        let rs = default_region_segmentation("cube", 10);
        let sg = SupportGeometryView { entries: vec![] };
        let mut output = SupportGeometryOutput::new();
        planner
            .run_support_geometry_with_analysis(
                &[obj],
                &lp,
                &rs,
                &tree_analysis("plate", &["0", "1"]),
                &sg,
                &mut output,
                &ConfigView::default(),
            )
            .unwrap();
        assert!(
            output.entries().is_empty(),
            "cube without overhangs → empty plan"
        );
    }

    #[test]
    fn overhanging_plate_emits_branches() {
        // A downward-facing quad plate (two triangles) floating at z=2.0
        // with a reference vertex at z=0.0 so the object spans ≥10 layers
        // (layer_height = 0.2 mm). Two downward-facing triangles give two
        // distinct contact centroids that can form an MST edge on the
        // overhang layer and propagate down.
        let vertices = vec![
            // Anchor vertex at the origin so the object bounds span from
            // z=0 to z=2.0 and num_layers is ≥10.
            [0.0, 0.0, 0.0],
            // Lower plate (downward-facing — the overhang).
            [0.0, 0.0, 1.8],
            [4.0, 0.0, 1.8],
            [4.0, 4.0, 1.8],
            [0.0, 4.0, 1.8],
        ];
        let triangles = vec![
            // Two downward-facing overhang triangles (CW when viewed
            // from above → normal points down with z-component < 0).
            [1, 3, 2],
            [1, 4, 3],
        ];
        let obj = MeshObjectView {
            object_id: "plate".to_string(),
            vertices,
            triangles,
            paint_layers: vec![],
        };
        let planner = default_planner();
        let lp = default_layer_plan(10, 0.0, 0.2);
        let mut rs = default_region_segmentation("plate", 10);
        for layer_index in 0..10 {
            rs.entries[layer_index as usize].region_ids = vec!["0".to_string(), "1".to_string()];
            rs.region_support_configs
                .push(slicer_sdk::prepass_types::RegionSupportConfig {
                    object_id: "plate".to_string(),
                    layer_index,
                    region_id: "1".to_string(),
                    support_family: None,
                    support_type: Some("tree(auto)".to_string()),
                });
            rs.region_support_configs
                .push(slicer_sdk::prepass_types::RegionSupportConfig {
                    object_id: "plate".to_string(),
                    layer_index,
                    region_id: "0".to_string(),
                    support_family: Some("traditional".to_string()),
                    support_type: None,
                });
        }
        let sg = SupportGeometryView { entries: vec![] };
        let mut output = SupportGeometryOutput::new();
        planner
            .run_support_geometry_with_analysis(
                &[obj],
                &lp,
                &rs,
                &tree_analysis("plate", &["0", "1"]),
                &sg,
                &mut output,
                &ConfigView::default(),
            )
            .unwrap();
        assert!(
            !output.entries().is_empty(),
            "overhanging plate must yield non-empty plan; got {} entries",
            output.entries().len()
        );
        let families: std::collections::BTreeSet<_> = output
            .entries()
            .iter()
            .map(|entry| entry.family_id.as_str())
            .collect();
        // The tree planner resolves family from the analysis layer's
        // `family_assignments`, falling back to its own canonical "tree"
        // identity. Per-region `region_support_configs` are not consulted, so
        // a region configured "traditional" still plans as "tree" here (the
        // analysis layer owns the family split).
        assert_eq!(families, ["tree"].into_iter().collect());
    }

    #[test]
    fn lone_fresh_contact_emits_tip_on_origin_layer() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.8],
            [0.2, 0.0, 1.8],
            [0.2, 0.2, 1.8],
        ];
        let triangles = vec![[1, 3, 2]];
        let obj = MeshObjectView {
            object_id: "lone-contact".to_string(),
            vertices,
            triangles,
            paint_layers: vec![],
        };
        // This test is about tip *emission* on the contact's own layer, not
        // about the top-Z gap, so pin the gap to zero and keep the authored
        // fixture coordinates (layer 8, z = 1.8mm) exactly as-is. Packet 224
        // RC-11 gave `SupportPlanner` a default gap of
        // `DEFAULT_TOP_Z_DISTANCE_MM`; RC-11's own coverage lives in
        // `tests/orca_parity_tdd.rs::top_z_distance_lowers_the_tree_contact_layer`.
        let planner = SupportPlanner {
            support_top_z_distance_mm: 0.0,
            ..default_planner()
        };
        let lp = default_layer_plan(10, 0.0, 0.2);
        let rs = default_region_segmentation("lone-contact", 10);
        // Model occupancy placed clear of the contact centroid (~0.13, 0.07).
        // It used to span the whole plane, which put the contact *inside* the
        // model; a zero-width tip contributed no body geometry so nothing
        // noticed, but now that tips carry real area they are collision-checked
        // like any other node and such a contact is correctly rejected.
        let collision_box = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(8.0, 8.0),
                    Point2::from_mm(14.0, 8.0),
                    Point2::from_mm(14.0, 14.0),
                    Point2::from_mm(8.0, 14.0),
                ],
            },
            holes: vec![],
        };
        let sg = SupportGeometryView {
            entries: (0..10)
                .map(|layer| SupportGeometryViewEntry {
                    global_support_layer_index: layer,
                    object_id: "lone-contact".to_string(),
                    region_id: "0".to_string(),
                    outlines: vec![collision_box.clone()],
                })
                .collect(),
        };
        let mut output = SupportGeometryOutput::new();
        planner
            .run_support_geometry_with_analysis(
                &[obj],
                &lp,
                &rs,
                &tree_analysis("lone-contact", &["0", "1"]),
                &sg,
                &mut output,
                &ConfigView::default(),
            )
            .unwrap();

        let origin_entry = output
            .entries()
            .iter()
            .find(|entry| entry.global_layer_index == 8)
            .unwrap_or_else(|| {
                panic!(
                    "lone fresh contact must emit on its origin layer; got layers {:?} diags {:?}",
                    output
                        .entries()
                        .iter()
                        .map(|e| (
                            e.global_layer_index,
                            e.roles.iter().map(|r| r.role).collect::<Vec<_>>()
                        ))
                        .collect::<Vec<_>>(),
                    output
                        .diagnostics()
                        .iter()
                        .map(|d| (d.code, d.layer, d.message.clone()))
                        .collect::<Vec<_>>()
                )
            });
        let segment = &origin_entry.skeleton.as_ref().unwrap().points;
        assert_eq!(segment.len(), 2);
        assert_eq!(segment[0].x, segment[1].x);
        assert_eq!(segment[0].y, segment[1].y);
        assert!((segment[0].z - 1.8).abs() < 1e-5);
        assert!((segment[1].z - 1.8).abs() < 1e-5);
    }

    #[test]
    fn dist_to_top_increments_for_parent_child_propagation() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.8],
            [4.0, 0.0, 1.8],
            [4.0, 4.0, 1.8],
            [0.0, 4.0, 1.8],
        ];
        let triangles = vec![[1, 3, 2], [1, 4, 3]];
        let obj = MeshObjectView {
            object_id: "plate".to_string(),
            vertices,
            triangles,
            paint_layers: vec![],
        };
        let mut planner = default_planner();
        planner.support_interface_top_layers = 0;
        let layer_height = 0.2_f32;
        let lp = default_layer_plan(10, 0.0, layer_height);
        let rs = default_region_segmentation("plate", 10);
        let sg = SupportGeometryView { entries: vec![] };
        let mut output = SupportGeometryOutput::new();
        planner
            .run_support_geometry_with_analysis(
                &[obj],
                &lp,
                &rs,
                &tree_analysis("plate", &["0", "1"]),
                &sg,
                &mut output,
                &ConfigView::default(),
            )
            .unwrap();

        let mut distances_by_layer = std::collections::BTreeMap::<u32, Vec<u32>>::new();
        for entry in output.entries() {
            assert!(entry.global_layer_index >= 0);
            for point in &entry.skeleton.as_ref().unwrap().points {
                let distance_in_layers = (1.8 - point.z).abs() / layer_height;
                let rounded_distance = distance_in_layers.round();
                assert!((distance_in_layers - rounded_distance).abs() <= 1e-4);
                distances_by_layer
                    .entry(entry.global_layer_index as u32)
                    .or_default()
                    .push(rounded_distance as u32);
            }
        }

        let emitted_layers: Vec<u32> = distances_by_layer.keys().copied().collect();
        assert!(
            emitted_layers.len() >= 2,
            "fixture must emit at least one parent-child layer pair, got {:?}",
            emitted_layers
        );
        for layer_pair in emitted_layers.windows(2) {
            let child_layer = layer_pair[0];
            let parent_layer = layer_pair[1];
            assert_eq!(
                parent_layer,
                child_layer + 1,
                "fixture must expose adjacent parent-child propagation layers: layers={:?} distances={:?}",
                emitted_layers,
                distances_by_layer
            );
            let parent_distances = &distances_by_layer[&parent_layer];
            let child_distances = &distances_by_layer[&child_layer];
            let parent_dist = parent_distances[0];
            assert!(
                parent_distances
                    .iter()
                    .all(|&distance| distance == parent_dist),
                "parent layer {} has inconsistent dist_to_top values: {:?}",
                parent_layer,
                parent_distances
            );
            assert!(
                child_distances
                    .iter()
                    .all(|&distance| distance == parent_dist + 1),
                "child layer {} must have dist_to_top = parent layer {} + 1; parent={:?} child={:?}",
                child_layer,
                parent_layer,
                parent_distances,
                child_distances
            );
        }
    }

    #[test]
    fn body_clear_of_occupancy_does_not_overlap() {
        // `body_overlaps_occupancy` ended with
        // `point_in_polygon(&poly, closest[0], closest[1])` — asking whether the
        // closest point ON the boundary is inside the polygon, which is true or
        // false by floating-point accident and carries no information. It made
        // the predicate answer "overlaps" for a node arbitrarily far away, so
        // every branch body near any model occupancy was rejected.
        let box_far = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(8.0, 8.0),
                    Point2::from_mm(14.0, 8.0),
                    Point2::from_mm(14.0, 14.0),
                    Point2::from_mm(8.0, 14.0),
                ],
            },
            holes: vec![],
        };
        assert!(
            !body_overlaps_occupancy(&[box_far.clone()], 2.67, 1.33, 2.5),
            "a body 8 mm clear of occupancy must not be reported as overlapping"
        );
        assert!(
            body_overlaps_occupancy(&[box_far.clone()], 11.0, 11.0, 2.5),
            "a body inside occupancy must overlap"
        );
        assert!(
            body_overlaps_occupancy(&[box_far], 6.0, 11.0, 2.5),
            "a body whose radius reaches occupancy must overlap"
        );
    }

    #[test]
    fn prim_mst_on_two_nodes_returns_one_edge() {
        let nodes = vec![
            PlannedSupportNode {
                x: 0.0,
                y: 0.0,
                dist_to_top: 0,
                to_buildplate: true,
                demand_ids: Vec::new(),
            },
            PlannedSupportNode {
                x: 3.0,
                y: 4.0,
                dist_to_top: 0,
                to_buildplate: true,
                demand_ids: Vec::new(),
            },
        ];
        let edges = prim_mst(&nodes);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0, 0);
        assert_eq!(edges[0].1, 1);
        assert!((edges[0].2 - 5.0).abs() < 1e-4);
    }

    #[test]
    fn empty_layer_plan_view_returns_fatal_module_error() {
        let planner = default_planner();
        let obj = MeshObjectView {
            object_id: "test".to_string(),
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[0, 1, 2]],
            paint_layers: vec![],
        };
        let lp = LayerPlanView { layers: vec![] };
        let rs = RegionSegmentationView {
            entries: vec![],
            region_support_configs: vec![],
        };
        let sg = SupportGeometryView { entries: vec![] };
        let mut output = SupportGeometryOutput::new();
        let result = planner.run_support_geometry(
            &[obj],
            &lp,
            &rs,
            &sg,
            &mut output,
            &ConfigView::default(),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("empty layer-plan-view"),
            "error was: {err}"
        );
    }

    #[test]
    fn tapered_radius_at_tip_is_floored_at_min_branch_radius() {
        let branch_radius = 2.5_f32;
        let tan_diameter_angle = (5.0_f32).to_radians().tan();
        let dist_to_top = 0_u32;
        let effective_layer_height = 0.2_f32;
        let result = tapered_radius(
            branch_radius,
            tan_diameter_angle,
            dist_to_top,
            effective_layer_height,
        );
        assert!(
            (result - MIN_BRANCH_RADIUS).abs() < 1e-6,
            "tapered_radius at tip (dist_to_top=0) must use the 0.4 floor; got {result}"
        );
    }

    #[test]
    fn tapered_radius_inside_cone_is_mm_to_top() {
        let branch_radius = 2.5_f32;
        let tan_diameter_angle = (5.0_f32).to_radians().tan();
        let dist_to_top = 12_u32;
        let effective_layer_height = 0.2_f32;
        let result = tapered_radius(
            branch_radius,
            tan_diameter_angle,
            dist_to_top,
            effective_layer_height,
        );
        let expected = 2.4_f32;
        assert!(
            (result - expected).abs() < 1e-6,
            "tapered_radius inside cone must be {expected}; got {result}"
        );
    }

    #[test]
    fn tapered_radius_above_cone_is_linear() {
        let branch_radius = 2.5_f32;
        let tan_diameter_angle = (5.0_f32).to_radians().tan();
        let dist_to_top = 50_u32;
        let effective_layer_height = 0.2_f32;
        let result = tapered_radius(
            branch_radius,
            tan_diameter_angle,
            dist_to_top,
            effective_layer_height,
        );
        let mm_to_top = 50.0 * 0.2;
        let expected = branch_radius + (mm_to_top - branch_radius) * tan_diameter_angle;
        assert!(
            (result - expected).abs() < 1e-6,
            "tapered_radius above cone must be {expected}; got {result}"
        );
    }

    #[test]
    fn tapered_radius_clamps_at_max() {
        let branch_radius = 2.5_f32;
        let tan_diameter_angle = (80.0_f32).to_radians().tan();
        let dist_to_top = 10_000_u32;
        let effective_layer_height = 0.5_f32;
        let result = tapered_radius(
            branch_radius,
            tan_diameter_angle,
            dist_to_top,
            effective_layer_height,
        );
        assert!(
            (result - MAX_BRANCH_RADIUS_MM).abs() < 1e-12,
            "tapered_radius must clamp at MAX_BRANCH_RADIUS_MM={MAX_BRANCH_RADIUS_MM}; got {result}"
        );
    }

    #[test]
    fn tapered_radius_no_longer_floors_at_branch_radius() {
        let branch_radius = 2.5_f32;
        let tan_diameter_angle = (5.0_f32).to_radians().tan();
        let dist_to_top = 10_u32;
        let effective_layer_height = 0.2_f32;
        let result = tapered_radius(
            branch_radius,
            tan_diameter_angle,
            dist_to_top,
            effective_layer_height,
        );
        let expected = 2.0_f32;
        assert!(
            (result - expected).abs() < 1e-6,
            "tapered_radius must be {expected} (not floor at branch_radius={branch_radius}); got {result}"
        );
    }

    #[test]
    fn offset_concave_l_shape_no_self_intersection() {
        let ex = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(3.0, 0.0),
                    Point2::from_mm(3.0, 1.0),
                    Point2::from_mm(1.0, 1.0),
                    Point2::from_mm(1.0, 3.0),
                    Point2::from_mm(0.0, 3.0),
                ],
            },
            holes: vec![],
        };
        let result = host::offset_polygons(&[ex], 0.5, OffsetJoinType::Miter, 0.0);
        assert!(
            !result.is_empty(),
            "offset must return at least one polygon"
        );
        for poly in &result {
            let pts = &poly.contour.points;
            let n = pts.len();
            for i in 0..n {
                let a1 = pts[i];
                let a2 = pts[(i + 1) % n];
                for j in 0..n {
                    if j == i || j == (i + 1) % n || j == (i + n - 1) % n {
                        continue;
                    }
                    let b1 = pts[j];
                    let b2 = pts[(j + 1) % n];
                    let (x1, y1) = (a1.x as f32, a1.y as f32);
                    let (x2, y2) = (a2.x as f32, a2.y as f32);
                    let (x3, y3) = (b1.x as f32, b1.y as f32);
                    let (x4, y4) = (b2.x as f32, b2.y as f32);
                    let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
                    if denom.abs() < 1e-12 {
                        continue;
                    }
                    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
                    let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / denom;
                    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                        panic!(
                            "self-intersection at edges {}->{} and {}->{}",
                            i,
                            (i + 1) % n,
                            j,
                            (j + 1) % n
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn offset_polygon_with_hole_preserves_hole() {
        let outer = Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        };
        let hole = Polygon {
            points: vec![
                Point2::from_mm(3.0, 3.0),
                Point2::from_mm(3.0, 7.0),
                Point2::from_mm(7.0, 7.0),
                Point2::from_mm(7.0, 3.0),
            ],
        };
        let ex = ExPolygon {
            contour: outer,
            holes: vec![hole],
        };
        let result = host::offset_polygons(&[ex], 0.5, OffsetJoinType::Miter, 0.0);
        assert!(
            !result.is_empty(),
            "offset must return at least one polygon"
        );
        for poly in &result {
            assert!(
                !poly.holes.is_empty(),
                "offset polygon must preserve at least one hole"
            );
            for h in &poly.holes {
                let area_units = {
                    let pts = &h.points;
                    let n = pts.len();
                    let mut a = 0.0_f64;
                    for i in 0..n {
                        let (x1, y1) = (pts[i].x as f64, pts[i].y as f64);
                        let (x2, y2) = (pts[(i + 1) % n].x as f64, pts[(i + 1) % n].y as f64);
                        a += x1 * y2 - x2 * y1;
                    }
                    a.abs() / 2.0
                };
                let area_mm2 = area_units / 100_000_000.0;
                assert!(
                    area_mm2 < 16.0,
                    "hole area {area_mm2} mm² must be less than original 16 mm²"
                );
            }
        }
    }

    #[test]
    fn offset_preserves_mm_coordinate_boundary() {
        let ex = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(1.0, 0.0),
                    Point2::from_mm(1.0, 1.0),
                    Point2::from_mm(0.0, 1.0),
                ],
            },
            holes: vec![],
        };
        let result = host::offset_polygons(&[ex], 0.5, OffsetJoinType::Miter, 0.0);
        assert!(
            !result.is_empty(),
            "offset must return at least one polygon"
        );
        let pts = &result[0].contour.points;
        let xs: Vec<f32> = pts.iter().map(|p| units_to_mm(p.x)).collect();
        let ys: Vec<f32> = pts.iter().map(|p| units_to_mm(p.y)).collect();
        let min_x = xs.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_x = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let min_y = ys.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_y = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let span_x = max_x - min_x;
        let span_y = max_y - min_y;
        assert!(
            (span_x - 2.0).abs() < 1e-4,
            "span_x must be ~2.0 mm; got {span_x}"
        );
        assert!(
            (span_y - 2.0).abs() < 1e-4,
            "span_y must be ~2.0 mm; got {span_y}"
        );
    }
}
