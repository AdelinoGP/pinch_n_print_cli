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
//! via triangle normals, emits contact points at their centroids, and
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
//!   then `radius = clamp(raw, 0.0, MAX_BRANCH_RADIUS_MM = 6.0)`. The top of the column
//!   tapers to a point (`mm_to_top = 0 → 0.0`).
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

/// Multi-layer organic tree-support planner.
#[allow(dead_code)]
pub struct SupportPlanner {
    enabled: bool,
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
    /// Line spacing for interface layer dense fill in mm.
    tree_support_interface_spacing_mm: f32,
}

#[derive(Clone, Debug)]
struct PlannedSupportNode {
    x: f32,
    y: f32,
    /// Number of layers from this node to the top of its support column.
    /// Drives radius tapering — nodes farther from the top are wider.
    dist_to_top: u32,
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
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("support_enabled") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => true,
        };
        let branch_angle_deg = match config.get("support_branch_angle_deg") {
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
        let support_interface_top_layers = match config.get("support_interface_top_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => 2,
        };
        let tree_support_interface_spacing_mm =
            match config.get("tree_support_interface_spacing_mm") {
                Some(ConfigValue::Float(w)) => *w as f32,
                Some(ConfigValue::Int(w)) => *w as f32,
                _ => 0.4,
            };
        Ok(Self {
            enabled,
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
            tree_support_interface_spacing_mm,
        })
    }

    fn run_support_geometry(
        &self,
        objects: &[MeshObjectView],
        layer_plan: &LayerPlanView,
        region_segmentation: &RegionSegmentationView,
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

        for entry in &support_geometry.entries {
            let layer_idx = entry.global_support_layer_index as usize;
            if layer_idx >= collision_cache.len() {
                continue;
            }
            for expoly in &entry.outlines {
                if expoly.contour.points.len() >= 3 {
                    collision_cache[layer_idx]
                        .collision_polys
                        .push(expoly.clone());
                    let inflated = host::offset_polygons(
                        &[expoly.clone()],
                        avoid_inflate,
                        OffsetJoinType::Miter,
                    );
                    for off in inflated {
                        if off.contour.points.len() >= 3 {
                            collision_cache[layer_idx].avoidance_polys.push(off);
                        }
                    }
                }
            }
        }

        // ── Packet 118 D11: planner-owned code 1003 warning ─────────────
        // Read the preserved `support_interface_bottom_layers` config key
        // and emit exactly one typed diagnostic before the layer loop when
        // the value is not -1. Packet 116 owns dead-state cleanup and
        // emits no warning; this packet owns the typed record.
        let interface_bottom_layers = match _config.get("support_interface_bottom_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => -1,
        };
        if interface_bottom_layers != -1 {
            let _ = output.push_diagnostic(Diagnostic {
                severity: DiagnosticSeverity::Warn,
                code: 1003,
                layer: None,
                object_id: None,
                message: format!(
                    "support-planner interface-bottom-layers: support_interface_bottom_layers \
                     is not yet implemented (config value={interface_bottom_layers})"
                ),
            });
        }

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
        collision_cache: &[LayerCollisionCache],
        output: &mut SupportGeometryOutput,
        dropped_by_layer: &mut std::collections::BTreeMap<u32, usize>,
    ) -> Result<(), ModuleError> {
        if obj.triangles.is_empty() {
            return Ok(());
        }

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

        // Bounds are still needed for build-plate proximity checks.
        let (bmin, _bmax) = match compute_bounds(&obj.vertices) {
            Some(b) => b,
            None => return Ok(()),
        };

        // ── Step 9: simplified detect_overhangs ──────────────────────────
        let overhang_facets = detect_overhang_facets(obj, OVERHANG_THRESHOLD_DEG);
        let enforcer_contacts = collect_paint_enforcer_contacts(obj);
        let blocker_polys = collect_paint_blocker_polygons(obj);

        // Contact points per origin-layer. A facet whose centroid z falls
        // into layer L contributes exactly one contact point at L.
        let mut contacts_by_layer: Vec<Vec<PlannedSupportNode>> =
            vec![Vec::new(); num_layers as usize];

        // Per-affected-layer drop count for the code 1001 cap diagnostic.
        // Keyed by global_layer_index so the message carries the right value
        // even when layer_rev doesn't line up with the layer-plan index.
        // Owned by run_support_geometry; this function increments into the
        // shared map so per-layer totals are merged across all objects
        // before emission.

        for (v0, v1, v2) in &overhang_facets {
            let centroid = [
                (v0[0] + v1[0] + v2[0]) / 3.0,
                (v0[1] + v1[1] + v2[1]) / 3.0,
                (v0[2] + v1[2] + v2[2]) / 3.0,
            ];
            if point_in_any_polygon(&blocker_polys, centroid[0], centroid[1]) {
                continue;
            }
            // Overhangs whose centroid sits on the build plate (layer 0 ±
            // half a layer) do not need tree supports — OrcaSlicer's
            // detect_overhangs short-circuits the same case.
            let first_layer_height = layer_plan.layers[0].effective_layer_height;
            let rel_z = (centroid[2] - bmin[2]).max(0.0);
            if rel_z < first_layer_height * 0.5 {
                continue;
            }
            // Find the layer whose Z range contains the centroid Z.
            // Layer Z values represent the top of each layer, so the first
            // layer with z >= centroid_z is the one containing the overhang.
            let layer_idx = layer_plan
                .layers
                .iter()
                .position(|l| l.z >= centroid[2])
                .unwrap_or(layer_plan.layers.len() - 1);
            if contacts_by_layer[layer_idx].len() >= self.max_branches_per_layer {
                let global_li = layer_plan.layers[layer_idx].global_layer_index;
                *dropped_by_layer.entry(global_li).or_insert(0) += 1;
                continue;
            }
            contacts_by_layer[layer_idx].push(PlannedSupportNode {
                x: centroid[0],
                y: centroid[1],
                dist_to_top: 0,
            });
        }

        for (layer_idx, x, y) in &enforcer_contacts {
            let li = (*layer_idx as usize).min(num_layers as usize - 1);
            if point_in_any_polygon(&blocker_polys, *x, *y) {
                continue;
            }
            if contacts_by_layer[li].len() >= self.max_branches_per_layer {
                let global_li = layer_plan.layers[li].global_layer_index;
                *dropped_by_layer.entry(global_li).or_insert(0) += 1;
                continue;
            }
            contacts_by_layer[li].push(PlannedSupportNode {
                x: *x,
                y: *y,
                dist_to_top: 0,
            });
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

            // Emit branch segments with radius tapering (Step 5 AC-2)
            let mut branch_segments: Vec<Vec<Point3WithWidth>> = Vec::new();
            let mut origin_contacts_emitted = vec![false; active_nodes.len()];
            for (a_idx, b_idx, _) in &mst_edges {
                if drop[*a_idx] || drop[*b_idx] {
                    continue;
                }
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

                if point_in_any_expoly(collision_polys, na.x, na.y)
                    || point_in_any_expoly(collision_polys, nb.x, nb.y)
                {
                    continue;
                }

                let dist_a_mm = na.dist_to_top as f32 * effective_height;
                let dist_b_mm = nb.dist_to_top as f32 * effective_height;
                branch_segments.push(vec![
                    Point3WithWidth {
                        x: na.x,
                        y: na.y,
                        z: z_current,
                        width: radius_a * 2.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: dist_a_mm,
                    },
                    Point3WithWidth {
                        x: nb.x,
                        y: nb.y,
                        z: z_current,
                        width: radius_b * 2.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: dist_b_mm,
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
                let width = tapered_radius(
                    branch_radius,
                    tan_diameter_angle,
                    node.dist_to_top,
                    effective_height,
                ) * 2.0;
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
                };
                branch_segments.push(vec![point, point]);
            }

            // Top-interface densification (Step 6 AC-4):
            // Per OrcaSlicer TreeSupport.cpp 1460-1700, the top
            // `support_interface_top_layers` layers below each branch
            // contact carry dense rectilinear scan-line fill in addition
            // to structural branch segments. dist_to_top tracks layers
            // below the column's contact (0 = contact layer itself); the
            // interface band is [1 .. top_n] (excludes the contact layer
            // since the contact is the model boundary).
            let top_n = self.support_interface_top_layers.max(0) as u32;
            if top_n > 0 && self.tree_support_interface_spacing_mm > 0.0 {
                // Alternate scan-line direction by layer parity (X-axis on
                // even layers, Y-axis on odd) per OrcaSlicer convention.
                let layer_parity = (current_global_layer_index as i32).rem_euclid(2);
                for (i, node) in active_nodes.iter().enumerate() {
                    if drop[i] {
                        continue;
                    }
                    if node.dist_to_top == 0 || node.dist_to_top > top_n {
                        continue;
                    }
                    let radius = tapered_radius(
                        branch_radius,
                        tan_diameter_angle,
                        node.dist_to_top,
                        effective_height,
                    );
                    let bbox_half = radius + self.tree_support_branch_distance * 0.5;
                    push_interface_scan_lines(
                        &mut branch_segments,
                        node.x,
                        node.y,
                        z_current,
                        bbox_half,
                        radius * 2.0,
                        self.tree_support_interface_spacing_mm,
                        layer_parity,
                        avoidance_polys,
                        collision_polys,
                    );
                }
            }

            if !branch_segments.is_empty() {
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
                    entries_in_order.push(SupportPlanEntry {
                        global_layer_index: current_global_layer_index as i32,
                        object_id: obj.object_id.clone(),
                        region_id: region_id.clone(),
                        branch_segments: branch_segments.clone(),
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

                    // Clamp into avoidance_polys (Step 5 AC-3)
                    let (cx, cy) = clamp_to_avoidance(raw_step.0, raw_step.1, avoidance_polys);

                    // Drop nodes whose target lies inside collision_polys
                    // (AC-N3: code 1002 node-clamped-out typed diagnostic).
                    if point_in_any_expoly(collision_polys, cx, cy) {
                        let _ = output.push_diagnostic(Diagnostic {
                            severity: DiagnosticSeverity::Warn,
                            code: 1002,
                            layer: Some(current_global_layer_index as i32),
                            object_id: Some(obj.object_id.clone()),
                            message: format!(
                                "node-clamped-out: layer={} obj={} pos=({:.3},{:.3})",
                                current_global_layer_index, obj.object_id, cx, cy
                            ),
                        });
                        continue;
                    }

                    PlannedSupportNode {
                        x: cx,
                        y: cy,
                        // dist_to_top increments as we move down
                        dist_to_top: node.dist_to_top.saturating_add(1),
                    }
                };
                next_nodes.push(moved);
            }

            active_nodes = next_nodes;
        }

        // Apply per-column Laplacian smoothing (Orca TreeSupport::smooth_nodes port; packet 121).
        smooth_branches(&mut entries_in_order, 100);

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
        Ok(())
    }
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

/// Returns `(x, y, width)` of the first point of the first branch segment, if
/// present. Used by `smooth_branches` so malformed entries never panic.
fn first_point_xyw(entry: &slicer_sdk::prepass_types::SupportPlanEntry) -> Option<(f32, f32, f32)> {
    entry
        .branch_segments
        .first()
        .and_then(|p| p.first())
        .map(|pt| (pt.x, pt.y, pt.width))
}

/// Rust port of Orca's `TreeSupport::smooth_nodes`. Applies an in-place
/// three-point Laplacian smoother to each `(object_id, region_id)` column of
/// `SupportPlanEntry` rows, chaining the single point of each entry's first
/// branch segment. Endpoints (first and last in the descending-layer chain) are
/// held fixed. Only the `(x, y, width)` of interior points are mutated; `z`,
/// `role`, `speed_factor`, layer index, ids, and all counts are preserved.
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
                    let mut new_w = (prev.2 + cur.2 + next.2) / 3.0;
                    new_w = new_w.clamp(0.0, MAX_BRANCH_RADIUS_MM);
                    if let Some(path) = entries[column[i]].branch_segments.first_mut() {
                        if let Some(pt) = path.first_mut() {
                            pt.x = new_x;
                            pt.y = new_y;
                            pt.width = new_w;
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
///   Clamped to `[0, MAX_BRANCH_RADIUS_MM]`.
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
    raw.clamp(0.0, MAX_BRANCH_RADIUS_MM)
}

/// Clamp a point into the union of avoidance polygons.
/// Returns the original point if avoidance_polys is empty; otherwise returns
/// the closest point on any avoidance polygon boundary.
fn clamp_to_avoidance(x: f32, y: f32, avoidance_polys: &[ExPolygon]) -> (f32, f32) {
    if avoidance_polys.is_empty() {
        return (x, y);
    }
    if point_in_any_expoly(avoidance_polys, x, y) {
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

/// Append rectilinear scan-line dense interface fill segments around a node.
///
/// Generates parallel scan-lines at `spacing` apart spanning the bounding box
/// `[cx ± half, cy ± half]`. Layer parity selects scan direction (X-aligned
/// on even layers, Y-aligned on odd) — matches OrcaSlicer's alternating fill
/// convention from `TreeSupport.cpp` 1460–1700.
///
/// When `avoidance_polys` is non-empty, each scan-line is emitted only if both
/// endpoints lie inside the avoidance union; otherwise the line is skipped.
/// This is a coarse clip (no per-edge intersection) — sufficient for v1 since
/// avoidance polys already cover the support footprint with safety margin.
#[allow(clippy::too_many_arguments)]
fn push_interface_scan_lines(
    out: &mut Vec<Vec<Point3WithWidth>>,
    cx: f32,
    cy: f32,
    z: f32,
    half: f32,
    width: f32,
    spacing: f32,
    parity: i32,
    avoidance_polys: &[ExPolygon],
    collision_polys: &[ExPolygon],
) {
    if spacing <= 0.0 || half <= 0.0 {
        return;
    }
    let xmin = cx - half;
    let xmax = cx + half;
    let ymin = cy - half;
    let ymax = cy + half;
    let n = ((half * 2.0) / spacing).max(1.0).ceil() as i32;
    for k in 0..=n {
        let t = (k as f32) * spacing;
        if t > half * 2.0 + 1e-6 {
            break;
        }
        let (p1x, p1y, p2x, p2y) = if parity == 0 {
            // X-aligned line at varying y
            let y = ymin + t;
            (xmin, y, xmax, y)
        } else {
            let x = xmin + t;
            (x, ymin, x, ymax)
        };
        if !avoidance_polys.is_empty()
            && (!point_in_any_expoly(avoidance_polys, p1x, p1y)
                || !point_in_any_expoly(avoidance_polys, p2x, p2y))
        {
            continue;
        }
        if !collision_polys.is_empty()
            && (point_in_any_expoly(collision_polys, p1x, p1y)
                || point_in_any_expoly(collision_polys, p2x, p2y))
        {
            continue;
        }
        out.push(vec![
            Point3WithWidth {
                x: p1x,
                y: p1y,
                z,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
            Point3WithWidth {
                x: p2x,
                y: p2y,
                z,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
        ]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_planner() -> SupportPlanner {
        SupportPlanner {
            enabled: true,
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
            tree_support_interface_spacing_mm: 0.4,
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
            .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::default())
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
        let rs = default_region_segmentation("plate", 10);
        let sg = SupportGeometryView { entries: vec![] };
        let mut output = SupportGeometryOutput::new();
        planner
            .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::default())
            .unwrap();
        assert!(
            !output.entries().is_empty(),
            "overhanging plate must yield non-empty plan; got {} entries",
            output.entries().len()
        );
    }

    #[test]
    fn lone_fresh_contact_emits_tip_on_origin_layer() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.8],
            [4.0, 0.0, 1.8],
            [4.0, 4.0, 1.8],
        ];
        let triangles = vec![[1, 3, 2]];
        let obj = MeshObjectView {
            object_id: "lone-contact".to_string(),
            vertices,
            triangles,
            paint_layers: vec![],
        };
        let planner = default_planner();
        let lp = default_layer_plan(10, 0.0, 0.2);
        let rs = default_region_segmentation("lone-contact", 10);
        let collision_box = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(-10.0, -10.0),
                    Point2::from_mm(14.0, -10.0),
                    Point2::from_mm(14.0, 14.0),
                    Point2::from_mm(-10.0, 14.0),
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
            .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::default())
            .unwrap();

        let origin_entry = output
            .entries()
            .iter()
            .find(|entry| entry.global_layer_index == 8)
            .expect("lone fresh contact must emit on its origin layer");
        assert_eq!(origin_entry.branch_segments.len(), 1);
        let segment = &origin_entry.branch_segments[0];
        assert_eq!(segment.len(), 2);
        assert_eq!(segment[0].x, segment[1].x);
        assert_eq!(segment[0].y, segment[1].y);
        assert!((segment[0].z - 1.8).abs() < 1e-5);
        assert!((segment[1].z - 1.8).abs() < 1e-5);
        assert_eq!(segment[0].width, 0.0);
        assert_eq!(segment[1].width, 0.0);
        assert_eq!(segment[0].dist_to_top_mm, 0.0);
        assert_eq!(segment[1].dist_to_top_mm, 0.0);
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
            .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::default())
            .unwrap();

        let mut distances_by_layer = std::collections::BTreeMap::<u32, Vec<u32>>::new();
        for entry in output.entries() {
            assert!(entry.global_layer_index >= 0);
            for segment in &entry.branch_segments {
                for point in segment {
                    let distance_in_layers = point.dist_to_top_mm / layer_height;
                    let rounded_distance = distance_in_layers.round();
                    assert!(
                        (distance_in_layers - rounded_distance).abs() <= 1e-4,
                        "dist_to_top_mm={} is not an integral layer distance",
                        point.dist_to_top_mm
                    );
                    distances_by_layer
                        .entry(entry.global_layer_index as u32)
                        .or_default()
                        .push(rounded_distance as u32);
                }
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
    fn prim_mst_on_two_nodes_returns_one_edge() {
        let nodes = vec![
            PlannedSupportNode {
                x: 0.0,
                y: 0.0,
                dist_to_top: 0,
            },
            PlannedSupportNode {
                x: 3.0,
                y: 4.0,
                dist_to_top: 0,
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
        let rs = RegionSegmentationView { entries: vec![] };
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
    fn tapered_radius_at_tip_is_zero() {
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
            (result - 0.0).abs() < 1e-6,
            "tapered_radius at tip (dist_to_top=0) must be 0.0; got {result}"
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
        let result = host::offset_polygons(&[ex], 0.5, OffsetJoinType::Miter);
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
        let result = host::offset_polygons(&[ex], 0.5, OffsetJoinType::Miter);
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
        let result = host::offset_polygons(&[ex], 0.5, OffsetJoinType::Miter);
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
