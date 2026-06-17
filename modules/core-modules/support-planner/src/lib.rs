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
//! Multi-layer organic tree-support planner for `PrePass::SupportGeometry`.
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
//! - **Radius tapering**: per-emit radius = `clamp(branch_diameter/2 +
//!   tan(diameter_angle) * dist_to_top * effective_layer_height,
//!   branch_diameter/2, MAX_BRANCH_RADIUS)` with `MAX_BRANCH_RADIUS = 6.0`.
//! - **Wall-count scaling**: `max_move_distance = tan(angle) * height *
//!   wall_count.max(1)`.
//! - **dist_to_top tracking**: `u32` counter on each `PlannedSupportNode`
//!   incremented as nodes propagate downward; drives the radius taper formula.
//!
//! # Raft prefix layers
//!
//! When `support_raft_layers > 0`, the planner emits raft entries before all
//! model-layer entries. Raft entries carry negative `global_layer_index`
//! (`-1, -2, ..., -raft_layers`) so they always sort before model layers.

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_sdk::host::{log, LogLevel};
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
    /// Number of raft layers to emit (negative indices -1, -2, ...).
    support_raft_layers: i32,
    /// Number of interface layers at top of each branch column.
    support_interface_top_layers: i32,
    /// Number of interface layers at bottom of each branch column (-1 = all layers).
    support_interface_bottom_layers: i32,
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
    /// Direct support-outline polygons — nodes must stay outside these.
    collision_polys: Vec<Vec<[f32; 2]>>,
    /// Inflated collision polygons — nodes must stay inside these.
    avoidance_polys: Vec<Vec<[f32; 2]>>,
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
        let support_interface_top_layers = match config.get("support_interface_top_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => 2,
        };
        let support_interface_bottom_layers = match config.get("support_interface_bottom_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => -1, // -1 means "all layers" per OrcaSlicer convention
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
            support_interface_top_layers,
            support_interface_bottom_layers,
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
                let outer: Vec<[f32; 2]> = expoly
                    .contour
                    .points
                    .iter()
                    .map(|p| [p.x as f32, p.y as f32])
                    .collect();
                if outer.len() >= 3 {
                    collision_cache[layer_idx]
                        .collision_polys
                        .push(outer.clone());
                    let inflated = inflate_polygon(&outer, avoid_inflate);
                    if !inflated.is_empty() {
                        collision_cache[layer_idx].avoidance_polys.push(inflated);
                    }
                }
                for hole in &expoly.holes {
                    let hole_points: Vec<[f32; 2]> = hole
                        .points
                        .iter()
                        .map(|p| [p.x as f32, p.y as f32])
                        .collect();
                    if hole_points.len() >= 3 {
                        collision_cache[layer_idx].collision_polys.push(hole_points);
                    }
                }
            }
        }

        for obj in objects {
            self.plan_for_object(
                obj,
                layer_plan,
                region_segmentation,
                &collision_cache,
                output,
            )?;
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

        // ── Raft prefix layers ─────────────────────────────────────────
        // When support_raft_layers > 0, emit full-cross-section dense-fill
        // raft entries BEFORE all model-layer entries. Each raft entry carries
        // a negative global_layer_index (-1, -2, ...) so raft always sorts
        // before model layers. Z values are z_bed - i * raft_layer_height_mm
        // (raft_layer_height = effective_layer_height of layer 0).
        if self.support_raft_layers > 0 {
            let raft_layer_height_mm = layer_plan.layers[0].effective_layer_height;
            let first_layer_z = layer_plan.layers[0].z;
            // z_bed is the build plate Z; raft sits below first model layer.
            // If layer 0 has z = raft_layer_height_mm, z_bed = first_layer_z - raft_layer_height_mm
            let z_bed = if first_layer_z > raft_layer_height_mm {
                first_layer_z - raft_layer_height_mm
            } else {
                0.0
            };
            // Collect unique region_ids for this object once — raft emits
            // one entry per (raft_layer, region), not per (raft_layer, layer,
            // region). Without dedup, repeated region_ids across model layers
            // would multiply the raft entry count.
            let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            for e in &region_segmentation.entries {
                if e.object_id == obj.object_id {
                    for rid in &e.region_ids {
                        seen.insert(rid.clone());
                    }
                }
            }
            let unique_region_ids: Vec<String> = seen.into_iter().collect();
            for i in 1..=self.support_raft_layers {
                let raft_z = z_bed - (i as f32) * raft_layer_height_mm;
                let raft_index = -i;
                for region_id in &unique_region_ids {
                    entries_in_order.push(SupportPlanEntry {
                        global_layer_index: raft_index,
                        object_id: obj.object_id.clone(),
                        region_id: region_id.clone(),
                        branch_segments: vec![vec![Point3WithWidth {
                            x: 0.0,
                            y: 0.0,
                            z: raft_z,
                            width: self.line_width_mm,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        }]],
                    });
                }
            }
        }

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
                active_nodes.truncate(self.max_branches_per_layer);
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

                branch_segments.push(vec![
                    Point3WithWidth {
                        x: na.x,
                        y: na.y,
                        z: z_current,
                        width: radius_a * 2.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                    Point3WithWidth {
                        x: nb.x,
                        y: nb.y,
                        z: z_current,
                        width: radius_b * 2.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                ]);
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
            // For each surviving node, move toward its MST parent by
            // `step_xy` in the XY plane. Nodes without an MST edge simply
            // propagate unchanged.
            let mut next_nodes: Vec<PlannedSupportNode> = Vec::with_capacity(active_nodes.len());
            // Build a neighbour lookup: for node i, remember its closest MST
            // neighbour (the other endpoint of its lowest-distance edge).
            let mut nearest_neighbour: Vec<Option<usize>> = vec![None; active_nodes.len()];
            let mut nearest_distance: Vec<f32> = vec![f32::INFINITY; active_nodes.len()];
            for (a, b, d) in &mst_edges {
                if *d < nearest_distance[*a] {
                    nearest_distance[*a] = *d;
                    nearest_neighbour[*a] = Some(*b);
                }
                if *d < nearest_distance[*b] {
                    nearest_distance[*b] = *d;
                    nearest_neighbour[*b] = Some(*a);
                }
            }

            for (i, node) in active_nodes.iter().enumerate() {
                if drop[i] {
                    continue;
                }
                let moved = match nearest_neighbour[i] {
                    Some(j) => {
                        let neighbour = &active_nodes[j];
                        let dx = neighbour.x - node.x;
                        let dy = neighbour.y - node.y;
                        let len = (dx * dx + dy * dy).sqrt();

                        let raw_step = if len > max_move_xy && len > 1e-6 {
                            // Scale to max_move_xy (wall-count scaled)
                            let scale = max_move_xy / len;
                            (node.x + dx * scale, node.y + dy * scale)
                        } else if len > 1e-6 {
                            // Short link — move fully toward neighbour
                            (neighbour.x, neighbour.y)
                        } else {
                            (node.x, node.y)
                        };

                        // Clamp into avoidance_polys (Step 5 AC-3)
                        let (cx, cy) = clamp_to_avoidance(raw_step.0, raw_step.1, avoidance_polys);

                        // Drop nodes whose target lies inside collision_polys
                        // (AC-N3: node-clamped-out diagnostic).
                        // Diagnostic is emitted via host-services.log with a
                        // structured `support-planner.node-clamped-out` prefix
                        // until a typed `Diagnostic` channel is plumbed through
                        // the prepass output WIT (follow-up to packet 31b).
                        if point_in_any_polygon(collision_polys, cx, cy) {
                            log(
                                LogLevel::Warn,
                                &format!(
                                    "support-planner.node-clamped-out: layer={} obj={} pos=({:.3},{:.3})",
                                    current_global_layer_index,
                                    obj.object_id,
                                    cx,
                                    cy
                                ),
                            );
                            continue;
                        }

                        PlannedSupportNode {
                            x: cx,
                            y: cy,
                            // dist_to_top increments as we move down
                            dist_to_top: node.dist_to_top.saturating_add(1),
                        }
                    }
                    None => PlannedSupportNode {
                        x: node.x,
                        y: node.y,
                        dist_to_top: node.dist_to_top.saturating_add(1),
                    },
                };
                next_nodes.push(moved);
            }

            active_nodes = next_nodes;
        }

        // Emit entries in top-to-bottom order.
        for entry in entries_in_order {
            output
                .push_support_plan_entry(entry)
                .map_err(|e| ModuleError::fatal(1, format!("push_support_plan failed: {e}")))?;
        }
        Ok(())
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

// ── Step-5 helper functions ───────────────────────────────────────────────────

/// Compute tapered radius at a node given its distance from the top of the column.
///
/// Formula (per OrcaSlicer `TreeSupport.cpp`):
/// `radius = clamp(branch_radius + tan(diameter_angle) * dist_to_top * layer_height,
///                 branch_radius, MAX_BRANCH_RADIUS)`
pub fn tapered_radius(
    branch_radius: f32,
    tan_diameter_angle: f32,
    dist_to_top: u32,
    effective_layer_height: f32,
) -> f32 {
    let expanded =
        branch_radius + tan_diameter_angle * (dist_to_top as f32) * effective_layer_height;
    expanded.clamp(branch_radius, MAX_BRANCH_RADIUS_MM)
}

/// Inflate a polygon by `delta` mm using a simple edge-parallel offset approach.
/// Returns the inflated polygon vertices.
pub(crate) fn inflate_polygon(outer: &[[f32; 2]], delta: f32) -> Vec<[f32; 2]> {
    if outer.len() < 3 || delta <= 0.0 {
        return outer.to_vec();
    }
    let n = outer.len();
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let p0 = outer[i];
        let p1 = outer[(i + 1) % n];
        let p2 = outer[(i + 2) % n];

        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-8 {
            continue;
        }
        // Outward normal (90° CCW) for edge p0→p1
        let nx = -dy / len;
        let ny = dx / len;

        // Average with previous edge normal for smoother miter at vertices
        let dx_prev = p0[0] - p2[0];
        let dy_prev = p0[1] - p2[1];
        let len_prev = (dx_prev * dx_prev + dy_prev * dy_prev).sqrt();
        let nx_prev = if len_prev > 1e-8 {
            -dy_prev / len_prev
        } else {
            nx
        };
        let ny_prev = if len_prev > 1e-8 {
            dx_prev / len_prev
        } else {
            ny
        };

        let avg_nx = nx + nx_prev;
        let avg_ny = ny + ny_prev;
        let avg_len = (avg_nx * avg_nx + avg_ny * avg_ny).sqrt();
        let off_x = if avg_len > 1e-8 { avg_nx / avg_len } else { nx };
        let off_y = if avg_len > 1e-8 { avg_ny / avg_len } else { ny };

        result.push([p1[0] + off_x * delta, p1[1] + off_y * delta]);
    }
    result
}

/// Clamp a point into the union of avoidance polygons.
/// Returns the original point if avoidance_polys is empty; otherwise returns
/// the closest point on any avoidance polygon boundary.
fn clamp_to_avoidance(x: f32, y: f32, avoidance_polys: &[Vec<[f32; 2]>]) -> (f32, f32) {
    if avoidance_polys.is_empty() {
        return (x, y);
    }
    if point_in_any_polygon(avoidance_polys, x, y) {
        return (x, y);
    }
    let mut best_dist = f32::INFINITY;
    let mut best = (x, y);
    for poly in avoidance_polys {
        if poly.len() < 3 {
            continue;
        }
        let (cp, cd) = closest_point_on_polygon(poly, x, y);
        if cd < best_dist {
            best_dist = cd;
            best = (cp[0], cp[1]);
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
    avoidance_polys: &[Vec<[f32; 2]>],
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
            && (!point_in_any_polygon(avoidance_polys, p1x, p1y)
                || !point_in_any_polygon(avoidance_polys, p2x, p2y))
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
            },
            Point3WithWidth {
                x: p2x,
                y: p2y,
                z,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
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
            support_interface_top_layers: 2,
            support_interface_bottom_layers: -1,
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
}
