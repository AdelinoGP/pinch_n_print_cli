//! Multi-layer organic tree-support planner for `PrePass::SupportGeneration`.
//!
//! Simplified port of OrcaSlicer's `TreeSupport::detect_overhangs` +
//! `TreeSupport::drop_nodes` (see `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`):
//! the planner walks each object's mesh, classifies overhang/bridge facets
//! via triangle normals, emits contact points at their centroids, and
//! propagates the contact-point set top-down through the object's layer
//! range. Per-layer merging uses a Prim minimum spanning tree — the same
//! O(V²) complexity class as OrcaSlicer's `MinimumSpanningTree::prim`.
//!
//! # v1 limitations (deferred to follow-up packets)
//!
//! - **Layer-height-agnostic.** The planner walks the object's bounding-box
//!   Z range at a fixed `DEFAULT_LAYER_HEIGHT_MM` (0.2 mm) rather than
//!   reading `LayerPlanIR.layers`. `LayerPlanIR` is a host-side scheduling
//!   prerequisite (see `ensure_stage_prerequisites` for
//!   `PrePass::SupportGeneration`) but is not surfaced to the prepass guest
//!   today, so it is not listed in this module's manifest
//!   `[ir-access].reads`. Variable-height layer plans will silently
//!   misalign until a follow-up packet plumbs `LayerPlanIR.layers` through
//!   the prepass WIT.
//! - **Single-region per object.** `MeshObjectView` does not currently
//!   carry per-region segmentation, so every emitted `SupportPlanEntry`
//!   uses the canonical `region_id = "0"` bucket. Single-region objects
//!   (the Benchy fixture and the live-dispatch test geometries) work
//!   correctly because `tree-support`'s `support_plan_segments_for` is
//!   invoked with `region_id = 0` for those regions; multi-region objects
//!   will collapse all branches under the first region.
//! - **No avoidance / collision caches** (`TreeSupportData`).
//! - **No radius tapering** along `tan(angle) * height`.
//! - **No raft / interface-layer stacking.**
//! - **No wall-count-aware `max_move_distance` scaling.**
//! - **No `tree_support_branch_angle` / `_diameter` / `_distance` tuning.**

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_sdk::prelude::*;

const DEFAULT_BRANCH_ANGLE_DEG: f32 = 45.0;
const DEFAULT_MERGE_DISTANCE_MM: f32 = 0.8;
const DEFAULT_MAX_BRANCHES_PER_LAYER: usize = 1024;
const DEFAULT_LINE_WIDTH_MM: f32 = 0.4;
const DEFAULT_LAYER_HEIGHT_MM: f32 = 0.2;
/// Overhang detection threshold: triangles whose normal z-component is below
/// `-sin(OVERHANG_THRESHOLD_DEG)` are flagged as overhang facets. Matches
/// OrcaSlicer's default `support_threshold_angle = 45°`.
const OVERHANG_THRESHOLD_DEG: f32 = 45.0;

/// Multi-layer organic tree-support planner.
pub struct SupportPlanner {
    enabled: bool,
    branch_angle_deg: f32,
    merge_distance_mm: f32,
    max_branches_per_layer: usize,
    line_width_mm: f32,
}

#[derive(Clone, Debug)]
struct PlannedSupportNode {
    x: f32,
    y: f32,
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
        Ok(Self {
            enabled,
            branch_angle_deg,
            merge_distance_mm,
            max_branches_per_layer,
            line_width_mm,
        })
    }

    fn run_support_generation(
        &self,
        objects: &[MeshObjectView],
        output: &mut SupportGenerationOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled {
            return Ok(());
        }

        for obj in objects {
            self.plan_for_object(obj, output)?;
        }
        Ok(())
    }
}

impl SupportPlanner {
    fn plan_for_object(
        &self,
        obj: &MeshObjectView,
        output: &mut SupportGenerationOutput,
    ) -> Result<(), ModuleError> {
        if obj.triangles.is_empty() {
            return Ok(());
        }

        // ── Bounds + layer range ────────────────────────────────────────
        // v1 is layer-height-agnostic: we walk the object's bounding-box
        // Z range at a fixed `DEFAULT_LAYER_HEIGHT_MM` rather than
        // consulting `LayerPlanIR.layers`. `LayerPlanIR` is a host-side
        // scheduling prerequisite but is not surfaced to the prepass
        // guest today (see module-level docs). A follow-up packet will
        // plumb the real layer plan through the prepass WIT.
        let (bmin, bmax) = match compute_bounds(&obj.vertices) {
            Some(b) => b,
            None => return Ok(()),
        };

        let object_height = (bmax[2] - bmin[2]).max(0.0);
        let layer_height = DEFAULT_LAYER_HEIGHT_MM;
        if object_height <= 0.0 || layer_height <= 0.0 {
            return Ok(());
        }
        let num_layers = (object_height / layer_height).ceil() as u32;
        if num_layers == 0 {
            return Ok(());
        }

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
            let rel_z = (centroid[2] - bmin[2]).max(0.0);
            if rel_z < layer_height * 0.5 {
                continue;
            }
            let layer_idx = (rel_z / layer_height).floor() as usize;
            let layer_idx = layer_idx.min(num_layers as usize - 1);
            if contacts_by_layer[layer_idx].len() >= self.max_branches_per_layer {
                continue;
            }
            contacts_by_layer[layer_idx].push(PlannedSupportNode {
                x: centroid[0],
                y: centroid[1],
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
            contacts_by_layer[li].push(PlannedSupportNode { x: *x, y: *y });
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
        let step_xy = (tan_angle * layer_height).max(0.0);

        let mut active_nodes: Vec<PlannedSupportNode> = Vec::new();

        // Accumulate entries bottom-up so the plan keeps a deterministic,
        // top-to-bottom layer order in output.
        let mut entries_in_order: Vec<SupportPlanEntry> = Vec::new();

        // Iterate top → bottom.
        let top = num_layers as usize;
        for layer_rev in (0..top).rev() {
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
                Some(std::cmp::Ordering::Equal) | None => a
                    .y
                    .partial_cmp(&b.y)
                    .unwrap_or(std::cmp::Ordering::Equal),
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
            let z_current = bmin[2] + (layer_rev as f32) * layer_height;
            let mut branch_segments: Vec<Vec<Point3WithWidth>> = Vec::new();
            for (a, b, _) in &mst_edges {
                let na = &active_nodes[*a];
                let nb = &active_nodes[*b];
                branch_segments.push(vec![
                    Point3WithWidth {
                        x: na.x,
                        y: na.y,
                        z: z_current,
                        width: self.line_width_mm,
                        flow_factor: 1.0,
                    },
                    Point3WithWidth {
                        x: nb.x,
                        y: nb.y,
                        z: z_current,
                        width: self.line_width_mm,
                        flow_factor: 1.0,
                    },
                ]);
            }

            if !branch_segments.is_empty() {
                // v1 single-region: `MeshObjectView` does not surface
                // per-region segmentation, so every entry is keyed under
                // the canonical `region_id = "0"` bucket. Single-region
                // objects match correctly; multi-region objects collapse.
                // See module-level docs.
                entries_in_order.push(SupportPlanEntry {
                    global_layer_index: layer_rev as u32,
                    object_id: obj.object_id.clone(),
                    region_id: "0".to_string(),
                    branch_segments,
                });
            }

            // Build the "moved" node set for the next (lower) layer.
            //
            // For each surviving node, move toward its MST parent by
            // `step_xy` in the XY plane. Nodes without an MST edge simply
            // propagate unchanged.
            let mut next_nodes: Vec<PlannedSupportNode> =
                Vec::with_capacity(active_nodes.len());
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
                        if len > step_xy && len > 1e-6 {
                            let scale = step_xy / len;
                            PlannedSupportNode {
                                x: node.x + dx * scale,
                                y: node.y + dy * scale,
                            }
                        } else {
                            // Short link: merge toward neighbour (next layer
                            // will deduplicate via MST).
                            PlannedSupportNode {
                                x: neighbour.x,
                                y: neighbour.y,
                            }
                        }
                    }
                    None => node.clone(),
                };
                next_nodes.push(moved);
            }

            active_nodes = next_nodes;
        }

        // Emit entries in top-to-bottom order.
        for entry in entries_in_order {
            output
                .push_support_plan(entry)
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
            result.push(vec![
                [v0[0], v0[1]],
                [v1[0], v1[1]],
                [v2[0], v2[1]],
            ]);
        }
    }
    result
}

fn point_in_any_polygon(polygons: &[Vec<[f32; 2]>], x: f32, y: f32) -> bool {
    polygons.iter().any(|poly| point_in_polygon(poly, x, y))
}

fn point_in_polygon(poly: &[[f32; 2]], x: f32, y: f32) -> bool {
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
        }
    }

    #[test]
    fn empty_objects_emits_nothing() {
        let planner = default_planner();
        let mut output = SupportGenerationOutput::new();
        planner
            .run_support_generation(&[], &mut output, &ConfigView::default())
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
        let mut output = SupportGenerationOutput::new();
        planner
            .run_support_generation(&[obj], &mut output, &ConfigView::default())
            .unwrap();
        assert!(output.entries().is_empty(), "cube without overhangs → empty plan");
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
        let mut output = SupportGenerationOutput::new();
        planner
            .run_support_generation(&[obj], &mut output, &ConfigView::default())
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
            PlannedSupportNode { x: 0.0, y: 0.0 },
            PlannedSupportNode { x: 3.0, y: 4.0 },
        ];
        let edges = prim_mst(&nodes);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0, 0);
        assert_eq!(edges[0].1, 1);
        assert!((edges[0].2 - 5.0).abs() < 1e-4);
    }
}
