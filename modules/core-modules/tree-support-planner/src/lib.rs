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
//! - **Avoidance / collision**: `TreeVolumes`, the port of canonical
//!   `TreeSupportData`. `get_collision(r, l)` is the layer outline inflated by
//!   `r + support_object_xy_distance` and simplified at the
//!   `m_radius_sample_resolution` grid; `get_avoidance(r, l)` is the bottom-up
//!   recurrence `union(erode(avoidance(r, l-1), max_move[l-1]), collision(r, l))`.
//!   The move pass does **not** clamp a node out of avoidance. Per canonical
//!   `drop_nodes`, every surviving node steps exactly `get_max_move_dist(node)`
//!   along a *direction*: the outward projection out of the **next** layer's
//!   `get_avoidance(calc_radius(...))`, or — when that projection is cut by a
//!   contour, overshoots `max_move^2 * layer^2`, or the node is already outside
//!   avoidance — the STUDIO-4252 retry against the next layer's *collision*,
//!   falling back to the 1/d^2-weighted neighbour-convergence direction when
//!   there is nothing to escape from. A node is never dropped for landing in
//!   collision: a node found inside `get_collision(0, layer)` whose clearance
//!   to the boundary is at least its own radius (or whose link to its parent is
//!   cut by a contour) has `valid` cleared, which stops propagation but still
//!   draws the node on its own layer — that is how a branch terminates on the
//!   model. Only `support_on_buildplate_only` escalates that to pruning the
//!   column, via the `to_buildplate` pass, which recomputes the flag from each
//!   moved position against the raw outlines of the layer below.
//! - **Radius tapering**: two-piece per-emit radius. With
//!   `mm_to_top = dist_to_top * effective_layer_height`,
//!   `raw = if mm_to_top <= branch_radius { mm_to_top }
//!          else { branch_radius + (mm_to_top - branch_radius) * tan(diameter_angle) }`,
//!   then `radius = clamp(raw, MIN_BRANCH_RADIUS = 0.4, MAX_BRANCH_RADIUS_MM = 10.0)`.
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
/// Canonical `support_line_width` (`PrintConfig.cpp`, `coFloatOrPercent`).
/// Canonical derives the support extrusion width from
/// `Flow::auto_extrusion_width(frSupportMaterial, nozzle_diameter)` when the
/// setting is 0; this module has no nozzle diameter in scope, so it takes the
/// same 0.35 mm default the G-code serializer already uses.
const DEFAULT_SUPPORT_LINE_WIDTH_MM: f32 = 0.35;
/// Canonical libslic3r `EPSILON`, in mm.
const CANONICAL_EPSILON_MM: f32 = 1e-4;
const DEFAULT_MAX_BRANCHES_PER_LAYER: usize = 1024;
const DEFAULT_LINE_WIDTH_MM: f32 = 0.4;
/// Overhang detection threshold: triangles whose normal z-component is below
/// `-sin(OVERHANG_THRESHOLD_DEG)` are flagged as overhang facets. Matches
/// OrcaSlicer's default `support_threshold_angle = 45°`.
const OVERHANG_THRESHOLD_DEG: f32 = 45.0;
/// Hard upper clamp on branch radius in mm. Matches OrcaSlicer's
/// `TreeSupportData::max_radius` hard upper bound (10.0 mm).
const MAX_BRANCH_RADIUS_MM: f32 = 10.0;
/// Canonical `DO_NOT_MOVER_UNDER_MM` for the non-slim tree styles. Below this
/// `print_z` the F-13 move pass forbids neighbour convergence entirely; the
/// slim style uses `0`.
const DO_NOT_MOVER_UNDER_MM: f32 = 5.0;
/// Canonical `drop_nodes` sends a sharp tail along its own `skin_direction`
/// rather than toward its neighbours while it is still within this distance
/// of the tip.
const SHARP_TAIL_SKIN_FOLLOW_MM: f32 = 3.0;
const MIN_BRANCH_RADIUS: f32 = 0.4;
/// Default vertical clearance between the top of a support column and the
/// overhang it supports. Matches OrcaSlicer's `support_top_z_distance` default
/// and `traditional-support-planner::DEFAULT_TOP_Z_DISTANCE_MM`, so both
/// families leave the same gap when the key is absent.
const DEFAULT_TOP_Z_DISTANCE_MM: f32 = 0.2;
/// Defensive fallback when `max_bridge_length` is absent or non-positive.
const DEFAULT_MAX_BRIDGE_LENGTH_MM: f32 = 10.0;
/// Canonical `smooth_nodes`' `const int iterations = 100` — the number of
/// three-point relaxation passes run over each branch chain before the
/// smoothed positions are committed.
const SMOOTH_NODES_ITERATIONS: usize = 100;
/// Canonical `smooth_nodes`' `thresh_tall_branch`, in mm: a chain whose summed
/// node heights exceed this is "very tall", and its tip needs an extra wall.
const SMOOTH_NODES_THRESH_TALL_BRANCH_MM: f32 = 100.0;
/// Canonical `smooth_nodes`' `thresh_dist_to_top`, in mm.
const SMOOTH_NODES_THRESH_DIST_TO_TOP_MM: f32 = 30.0;
/// Canonical `draw_circles`' `CIRCLE_RESOLUTION` when the model carries so many
/// branches that a full circle per node per layer would be ruinous: the branch
/// cross-section degenerates to a quad aligned with `ContactStats::nodes_angle`.
const CIRCLE_RESOLUTION_COARSE: usize = 4;
/// Canonical `draw_circles`' `CIRCLE_RESOLUTION` in the ordinary case.
const CIRCLE_RESOLUTION_FINE: usize = 100;
/// Canonical `draw_circles` picks [`CIRCLE_RESOLUTION_COARSE`] when
/// `avg_node_per_layer` exceeds this.
const COARSE_CIRCLE_NODE_THRESHOLD: usize = 200;
/// Multi-layer organic tree-support planner.
#[allow(dead_code)]
pub struct SupportPlanner {
    enabled: bool,
    /// Canonical support family selected for the matching renderer.
    support_family: String,
    branch_angle_deg: f32,
    /// Canonical `support_line_width` — the support-material extrusion width,
    /// in mm. This is the cap term in canonical `get_max_move_dist`
    /// (`min(tan_angle * node->height, support_extrusion_width)`), which is
    /// the merge radius the F-11 pass tests against. It replaces the invented
    /// flat `support_branch_merge_distance_mm` constant.
    support_line_width_mm: f32,
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
    /// Canonical `is_slim` (`support_style == smsTreeSlim`). Selects the
    /// `DO_NOT_MOVER_UNDER_MM` threshold in the F-13 move pass: `0` when slim,
    /// `5` mm otherwise.
    tree_support_is_slim: bool,
    tree_support_style: TreeSupportStyle,
    /// Config explicitly asked for the unimplemented organic engine; emit a
    /// once-per-slice code-1005 Warn about the Strong substitution.
    organic_substitution_requested: bool,
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
    /// Explicit band below a roof contact rendered as base interface.
    num_top_base_interface_layers: i32,
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
    /// Support layer height in mm (0.0 = use the object's effective layer
    /// height, the documented sentinel).
    support_layer_height_mm: f32,
    /// Packet 239c: support rows may leave the object layer grid. When true,
    /// canonical `bottom_contact_layer` (enabled branch) plus `generate_support_layers`
    /// let intermediate support rows print between object planes; when false,
    /// `anchor_z` stays a grid-exact copy of the object plane (canonical
    /// `sync_gap_with_object_layer`).
    independent_support_layer_height: bool,
    /// Canonical `TreeSupportData::m_xy_distance` — the horizontal clearance
    /// every collision volume is inflated by. Defect F-16: the planner used to
    /// inflate avoidance by `tree_support_branch_distance / 2`, which is
    /// canonical's contact-point `point_spread`, not a clearance at all.
    support_object_xy_distance: f32,
    max_bridge_length_mm: f32,
}

/// Canonical `SupportNode::type`. `ePolygon` nodes draw their stored overhang
/// rather than a circle in `draw_circles`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TreeNodeType {
    /// Canonical `eCircle`.
    Circle,
    /// Canonical `ePolygon`. Consumed by the step 7 `draw_circles` rewrite.
    #[allow(dead_code)]
    Polygon,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TreeSupportStyle {
    Default,
    Slim,
    Strong,
    Hybrid,
}

impl TreeSupportStyle {
    /// Canonical `SupportParameters.hpp` substitution chain, with this
    /// port's organic-engine alias:
    ///
    /// - non-tree support type: every style degrades to `smsDefault`
    ///   (canonical then routes it to the grid engine, which is not this
    ///   planner) — mapped `Default` here.
    /// - tree support type: explicit `tree_slim`/`tree_strong`/`tree_hybrid`
    ///   keep themselves; `grid`/`snug` degrade to `smsDefault`; and
    ///   `smsDefault` selects `smsTreeOrganic` in canonical. The organic
    ///   engine (`TreeSupport3D.cpp`) is not implemented in this port, so
    ///   every canonically-organic input runs the **Strong** style of the
    ///   old engine instead (deviation row in docs/DEVIATION_LOG.md; the
    ///   organic port is queued in
    ///   docs/specs/support-generation-remediation-plan.md).
    fn from_config(config: &ConfigView) -> Self {
        if !config_family_is_tree(config) {
            return Self::Default;
        }
        match config.get("support_style") {
            Some(ConfigValue::String(style)) if style == "tree_slim" => Self::Slim,
            Some(ConfigValue::String(style)) if style == "tree_strong" => Self::Strong,
            Some(ConfigValue::String(style)) if style == "tree_hybrid" => Self::Hybrid,
            // default / organic / grid / snug on a tree family all land on
            // smsTreeOrganic in canonical — aliased to Strong here.
            _ => Self::Strong,
        }
    }
}

/// Whether the config's support family resolves to tree. Absent
/// `support_type` falls back to this planner's own family default (tree),
/// matching `canonical_support_family`.
fn config_family_is_tree(config: &ConfigView) -> bool {
    canonical_support_family(config).starts_with("tree")
}

/// True when the config *explicitly* requests the organic engine on a tree
/// family. The engine is not implemented; `TreeSupportStyle::from_config`
/// substitutes Strong, and `run_support_geometry` emits a once-per-slice
/// code-1005 Warn for this case (the plain `default` alias is silent — a
/// product decision, documented in docs/DEVIATION_LOG.md).
pub fn organic_substitution_requested(config: &ConfigView) -> bool {
    config_family_is_tree(config)
        && matches!(
            config.get("support_style"),
            Some(ConfigValue::String(style)) if style == "organic"
        )
}

/// Handle into [`NodeArena`]. Canonical holds raw `SupportNode*`; the arena
/// index is the borrow-checker-safe equivalent, and is what makes cross-layer
/// `parent` / `child` / `parents` mutation expressible at all.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct NodeId(usize);

/// Port of canonical `SupportNode`.
///
/// Field names track `TreeSupport.hpp` deliberately: the drop/merge/move
/// passes ported in later steps read them by those names, and renaming them
/// here would make every subsequent diff unverifiable against canonical.
#[derive(Clone, Debug)]
struct PlannedSupportNode {
    /// Canonical `position`, in scaled units (1 unit = 100 nm).
    position: Point2,
    /// Canonical `movement` — the last applied move delta.
    /// Consumed by the step 5 move pass (F-13) and the step 6 `smooth_nodes`
    /// pass (F-33).
    movement: Point2,
    /// Canonical `distance_to_top`. **Signed**: negative marks the virtual
    /// top-Z-gap node created by F-34, which is propagated but never extruded.
    distance_to_top: i32,
    /// Canonical `dist_mm_to_top`. Zero on the virtual gap node, which
    /// "directly contacts the bottom". Consumed by the step 7 `draw_circles`
    /// rewrite (the ellipse/first-layer-brim radius terms).
    #[allow(dead_code)]
    dist_mm_to_top: f32,
    /// Canonical `radius`, in mm.
    radius: f32,
    /// Canonical `max_move_dist`. Consumed by the step 5 move pass (F-13).
    max_move_dist: f32,
    /// Canonical `support_roof_layers_below` — the **per-node** roof counter
    /// (F-1). Seeded from `add_interface ? support_roof_layers : 0` and
    /// decremented once per descendant whose parent had
    /// `distance_to_top >= 0`. Merges take the max.
    support_roof_layers_below: i32,
    /// Canonical `obj_layer_nr` — the object layer this node lives on.
    /// Consumed by the step 3 merge pass (F-11), which indexes layers by it.
    #[allow(dead_code)]
    obj_layer_nr: usize,
    /// Canonical `print_z`, in mm. Consumed by the step 7 `draw_circles`
    /// rewrite.
    #[allow(dead_code)]
    print_z: f32,
    /// Canonical `height`, in mm. On the virtual gap node this is exactly
    /// `z_distance_top`. Consumed by the step 7 `draw_circles` rewrite.
    #[allow(dead_code)]
    height: f32,
    /// Whether this node must reach the build plate (true) or may rest
    /// on the model (false). Set at contact creation from
    /// `!point_in_any_expoly(collision_polys_at_contact_layer, x, y)`
    /// (true iff the contact's XY lies OUTSIDE the object's projected
    /// footprint at the contact's layer, per packet 123). Canonical seeds
    /// this unconditionally `true` and recomputes it in `drop_nodes`; that
    /// recompute is F-14, step 5.
    to_buildplate: bool,
    /// Canonical `type`. Consumed by the step 7 `draw_circles` rewrite.
    #[allow(dead_code)]
    type_: TreeNodeType,
    /// Canonical `overhang`. The virtual gap node draws *this* into
    /// `roof_gap_areas` instead of a circle (F-34); step 7 owns that draw.
    overhang: ExPolygon,
    /// Canonical `skin_direction`, set from vertical enforcer normals.
    /// Consumed by the step 6 `smooth_nodes` pass (F-33).
    skin_direction: Point2,
    /// Canonical `is_sharp_tail`. Suppresses interface seeding and the
    /// inner-lattice stream.
    is_sharp_tail: bool,
    /// Canonical `is_corner`. Consumed by the step 7 `draw_circles` rewrite.
    #[allow(dead_code)]
    is_corner: bool,
    /// Canonical `need_extra_wall`. Consumed by the step 6 `smooth_nodes`
    /// pass (F-33).
    need_extra_wall: bool,
    /// Canonical `valid`. Cleared instead of erasing, so ids stay stable.
    valid: bool,
    /// Canonical `is_processed`. Consumed by the step 3 merge pass (F-11).
    is_processed: bool,
    /// Canonical `parent` — the node one layer **above** this one.
    /// Consumed by the step 3 merge pass (F-11) and step 6 `smooth_nodes`.
    parent: Option<NodeId>,
    /// Canonical `child` — the node one layer **below** this one.
    /// Consumed by the step 3 merge pass (F-11) and step 6 `smooth_nodes`.
    child: Option<NodeId>,
    /// Canonical `parents` — every upper-layer node that feeds this one.
    /// Consumed by the step 3 merge pass (F-11) and step 6 `smooth_nodes`.
    parents: Vec<NodeId>,
    /// Canonical `merged_neighbours`. Consumed by the step 3 merge pass
    /// (F-11).
    #[allow(dead_code)]
    merged_neighbours: Vec<NodeId>,
    /// Stable analysis demands represented by this routed node. PnP-specific;
    /// canonical has no equivalent.
    demand_ids: Vec<String>,
}

impl PlannedSupportNode {
    /// X position in mm.
    fn x(&self) -> f32 {
        units_to_mm(self.position.x)
    }

    /// Y position in mm.
    fn y(&self) -> f32 {
        units_to_mm(self.position.y)
    }

    /// Position in mm, in the `(x, y)` shape the emit helpers take.
    fn xy(&self) -> (f32, f32) {
        (self.x(), self.y())
    }

    /// Canonical `support_roof_layers_below > 0` roof test.
    fn is_roof(&self) -> bool {
        self.support_roof_layers_below > 0
    }

    /// True for the F-34 virtual top-Z-gap node — the one canonical
    /// `draw_circles` diverts into `roof_gap_areas`, which is never extruded.
    /// Sharp tails are exempt, which is how canonical gives them a zero
    /// contact distance.
    fn is_virtual_gap(&self) -> bool {
        self.distance_to_top < 0 && !self.is_sharp_tail
    }
}

/// An empty `ExPolygon`, for nodes with no stored overhang.
fn empty_expolygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon { points: Vec::new() },
        holes: Vec::new(),
    }
}

/// Owns every [`PlannedSupportNode`] for one object.
///
/// Canonical `TreeSupportData::create_node` allocates into a pool and hands
/// out pointers that the drop/merge/move passes mutate **across layers**
/// (`node->parent`, `node->child`, `node->parents`). The previous per-layer
/// `Vec<PlannedSupportNode>`-by-value could not express that at all: a node
/// handed to the next layer was a copy, so any back-edge written into it was
/// discarded. The arena is the enabling change for steps 3 through 7.
#[derive(Default)]
struct NodeArena {
    nodes: Vec<PlannedSupportNode>,
}

impl NodeArena {
    /// Port of canonical `TreeSupportData::create_node`.
    #[allow(clippy::too_many_arguments)]
    fn create_node(
        &mut self,
        position: Point2,
        distance_to_top: i32,
        obj_layer_nr: usize,
        support_roof_layers_below: i32,
        to_buildplate: bool,
        parent: Option<NodeId>,
        print_z: f32,
        height: f32,
        dist_mm_to_top: f32,
        radius: f32,
    ) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(PlannedSupportNode {
            position,
            movement: Point2 { x: 0, y: 0 },
            distance_to_top,
            dist_mm_to_top,
            radius,
            max_move_dist: 0.0,
            support_roof_layers_below,
            obj_layer_nr,
            print_z,
            height,
            to_buildplate,
            type_: TreeNodeType::Circle,
            overhang: empty_expolygon(),
            skin_direction: Point2 { x: 0, y: 0 },
            is_sharp_tail: false,
            is_corner: false,
            need_extra_wall: false,
            valid: true,
            is_processed: false,
            parent,
            child: None,
            parents: parent.into_iter().collect(),
            merged_neighbours: Vec::new(),
            demand_ids: Vec::new(),
        });
        if let Some(parent) = parent {
            self.nodes[parent.0].child = Some(id);
            // Canonical `SupportNode` ctor (`TreeSupport.hpp`): every merged
            // neighbour of the parent also adopts the new node as its child
            // and joins `parents`. This is what makes a merge-absorbed node a
            // chain *interior* node in `smooth_nodes` (its fixed head is the
            // surviving column's descendant); without it the absorbed node is
            // pinned at its raw drop-pass position and the branch silhouette
            // pops outward on every merge layer.
            let merged = self.nodes[parent.0].merged_neighbours.clone();
            for neighbour in merged {
                self.nodes[neighbour.0].child = Some(id);
                self.nodes[id.0].parents.push(neighbour);
            }
        }
        id
    }

    /// Number of nodes allocated so far. Consumed by the step 3 merge pass
    /// (F-11) and by this module's own tests.
    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.nodes.len()
    }
}

impl std::ops::Index<NodeId> for NodeArena {
    type Output = PlannedSupportNode;
    fn index(&self, id: NodeId) -> &PlannedSupportNode {
        &self.nodes[id.0]
    }
}

impl std::ops::IndexMut<NodeId> for NodeArena {
    fn index_mut(&mut self, id: NodeId) -> &mut PlannedSupportNode {
        &mut self.nodes[id.0]
    }
}

/// One layer's committed `drop_nodes` result, replayed by the emit pass.
///
/// Canonical runs `drop_nodes` over every layer and only then calls
/// `draw_circles`. F-14's `unsupported_branch_leaves` pruning walks *up* the
/// parent chain, so an interleaved plan/emit loop could not withdraw geometry
/// it had already written for a higher layer.
struct LayerRecord {
    /// Index into `layer_plan.layers` (the forward index, not `layer_rev`'s
    /// reverse counter — they are the same number here, kept explicit).
    layer_rev: usize,
    /// Surviving nodes on this layer, after merge drops.
    active: Vec<NodeId>,
    /// Surviving MST edges as node-id pairs.
    edges: Vec<(NodeId, NodeId)>,
}

/// Canonical `avg_node_per_layer` / `nodes_angle`, computed once over every
/// contact position at the end of `generate_contact_points`.
///
/// Consumed by the step 6 `smooth_nodes` pass (F-33), which uses the node
/// orientation to decide the smoothing direction.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ContactStats {
    /// Canonical `avg_node_per_layer = nNodes / nonempty_layers`.
    avg_node_per_layer: usize,
    /// Canonical
    /// `nodes_angle = atan2(n*mxy - mx*my, n*mx2 - SQ(mx))`, radians.
    nodes_angle: f32,
}

/// Port of the line-fit block that closes canonical
/// `TreeSupport::generate_contact_points`. `positions` are contact positions
/// in **mm**; `nonempty_layers` is the number of layers that received at
/// least one contact.
fn contact_stats(positions: &[(f32, f32)], nonempty_layers: usize) -> ContactStats {
    let n = positions.len();
    if n == 0 || nonempty_layers == 0 {
        return ContactStats::default();
    }
    let (mut mx, mut my, mut mxy, mut mx2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for (x, y) in positions {
        mx += x;
        my += y;
        mxy += x * y;
        mx2 += x * x;
    }
    let nf = n as f32;
    ContactStats {
        avg_node_per_layer: n / nonempty_layers,
        nodes_angle: (nf * mxy - mx * my).atan2(nf * mx2 - mx * mx),
    }
}

/// Port of canonical `TreeSupport::smooth_nodes`, finding F-33.
///
/// Canonical calls this unconditionally between `drop_nodes` and
/// `draw_circles`; before packet 224 step 6 this module never called any
/// smoothing stage in production, so every node's `movement` stayed at the
/// last raw move delta (or zero) and `draw_circles` had nothing to orient a
/// branch cross-section with.
///
/// The pass walks each branch chain once. Chains are collected by following
/// `parent` (the node one layer **above**), seeded with the starting node's
/// `child` as a fixed head — canonical's "add a fixed head if it's not a
/// polygon node, see STUDIO-4403", which is why a `Polygon`-typed child is
/// skipped. Chains shorter than three nodes are left alone. Interior nodes
/// are relaxed by canonical's unweighted three-point kernel
///
/// ```text
/// pts1[i]   = (pts[i - 1] + pts[i] + pts[i + 1]) / 3
/// radii1[i] = (radii[i - 1] + radii[i] + radii[i + 1]) / 3
/// ```
///
/// applied [`SMOOTH_NODES_ITERATIONS`] times (Jacobi: each pass reads the
/// previous pass' array), and then committed on the final pass together with
/// the canonical movement rule
///
/// ```text
/// movement = (pts[i + 1] - pts[i - 1]) / 2
/// ```
///
/// Note the asymmetry canonical deliberately has: the committed *position* is
/// the final pass' **output** (`pts1[i]`) while the committed *movement* is
/// read off that pass' **input** (`pts`). This port reproduces it.
///
/// This is the **only** producer of the final per-node `movement`;
/// `draw_circles`' ellipse matrix is its only consumer.
///
/// Endpoints (`branch[0]` and `branch[last]`) are held fixed, which is what
/// keeps a chain's contact tip on its overhang and its root on the plate.
/// Because the kernel is a convex combination of immediate chain neighbours,
/// the relaxed chain converges toward the straight segment between those two
/// pinned endpoints, so every per-layer delta stays a convex combination of
/// the drop pass' original per-layer deltas and no smoothed node can outrun
/// the lateral budget that
/// `tree_family_tdd::branch_angle_scales_the_per_layer_lateral_move` asserts.
///
/// **Deviation (arithmetic only).** Canonical relaxes in `Point` integer
/// arithmetic, so each of the 100 passes truncates. This port relaxes in
/// `f64` scaled units and rounds once at commit; over 100 passes the
/// truncation would otherwise bias every chain toward its head. The kernel,
/// the iteration count, and the commit rules are canonical.
fn smooth_nodes(arena: &mut NodeArena, layer_records: &[LayerRecord], support_line_width_mm: f32) {
    use std::collections::HashSet;
    // Canonical `float max_move = scale_(m_object_config->support_line_width / 2)`.
    // 1 unit = 100 nm here, so `mm_to_units` is the local `scale_`.
    let max_move_units = mm_to_units(support_line_width_mm / 2.0) as f64;
    // Only nodes that survived the F-14 prune are chain members: canonical
    // has already erased the rest from `contact_nodes` by this point.
    let alive: HashSet<NodeId> = layer_records
        .iter()
        .flat_map(|record| record.active.iter().copied())
        .collect();
    for id in alive.iter() {
        arena[*id].is_processed = false;
    }
    // Canonical walks layers **bottom-up** here while the chain walk goes
    // parent-ward (upward), so the first chain started from a column's root
    // covers the whole column. `layer_records` is pushed top-first by
    // `drop_nodes`, so it is consumed in reverse. Iterating top-first instead
    // would shatter every column into overlapping three-node windows, leaving
    // two nodes out of every three with no movement at all.
    for record in layer_records.iter().rev() {
        for start in &record.active {
            if arena[*start].is_processed {
                continue;
            }
            let mut branch: Vec<NodeId> = Vec::new();
            let mut total_height_mm = 0.0f32;
            // Canonical: "add a fixed head if it's not a polygon node, see
            // STUDIO-4403. Polygon node can't be added because the move
            // distance might be huge, making the nodes in between jump and
            // dangling."
            if let Some(child) = arena[*start].child {
                if alive.contains(&child) && arena[child].type_ != TreeNodeType::Polygon {
                    branch.push(child);
                    total_height_mm += arena[child].height;
                }
            }
            let mut cursor = Some(*start);
            while let Some(current) = cursor {
                if !alive.contains(&current) || arena[current].is_processed {
                    break;
                }
                branch.push(current);
                total_height_mm += arena[current].height;
                cursor = arena[current].parent;
            }
            if branch.len() < 3 {
                continue;
            }
            // f64 in scaled units: see the arithmetic deviation above.
            let mut pts: Vec<(f64, f64)> = branch
                .iter()
                .map(|id| (arena[*id].position.x as f64, arena[*id].position.y as f64))
                .collect();
            let mut radii: Vec<f64> = branch.iter().map(|id| arena[*id].radius as f64).collect();
            let mut out = pts.clone();
            let mut radii_out = radii.clone();
            let last = pts.len() - 1;
            for iteration in 0..SMOOTH_NODES_ITERATIONS {
                for i in 1..last {
                    let lo = i - 1;
                    let hi = i + 1;
                    out[i] = (
                        (pts[lo].0 + pts[i].0 + pts[hi].0) / 3.0,
                        (pts[lo].1 + pts[i].1 + pts[hi].1) / 3.0,
                    );
                    radii_out[i] = (radii[lo] + radii[i] + radii[hi]) / 3.0;
                }
                if iteration + 1 < SMOOTH_NODES_ITERATIONS {
                    pts.clone_from(&out);
                    radii.clone_from(&radii_out);
                }
            }
            for i in 1..last {
                let id = branch[i];
                arena[id].position = Point2 {
                    x: out[i].0.round() as i64,
                    y: out[i].1.round() as i64,
                };
                arena[id].radius = radii_out[i] as f32;
                // Canonical reads the movement off the final pass' *input*
                // array, not its output.
                let movement = Point2 {
                    x: ((pts[i + 1].0 - pts[i - 1].0) / 2.0).round() as i64,
                    y: ((pts[i + 1].1 - pts[i - 1].1) / 2.0).round() as i64,
                };
                arena[id].movement = movement;
                arena[id].is_processed = true;
                // Canonical:
                //   branch[i]->parents.size() > 1
                //   || movement.x() > max_move || movement.y() > max_move
                //   || (total_height > thresh_tall_branch
                //       && branch[i]->dist_mm_to_top < thresh_dist_to_top)
                // The move test is componentwise and **signed** in canonical
                // (not a magnitude), and the cap is half the support line
                // width — not the node's own `max_move_dist`.
                if arena[id].parents.len() > 1
                    || movement.x as f64 > max_move_units
                    || movement.y as f64 > max_move_units
                    || (total_height_mm > SMOOTH_NODES_THRESH_TALL_BRANCH_MM
                        && arena[id].dist_mm_to_top < SMOOTH_NODES_THRESH_DIST_TO_TOP_MM)
                {
                    arena[id].need_extra_wall = true;
                }
            }
            // Canonical "interpolate need_extra_wall in the end": a node
            // bracketed by two extra-wall nodes gets one too, so the wall
            // count does not flicker along a branch.
            for i in 1..branch.len().saturating_sub(1) {
                if arena[branch[i - 1]].need_extra_wall && arena[branch[i + 1]].need_extra_wall {
                    arena[branch[i]].need_extra_wall = true;
                }
            }
        }
    }
}

/// Canonical `draw_circles`' `branch_circle`: a regular polygon of
/// `resolution` vertices and radius `radius_units`, centred on the origin.
///
/// Canonical rotates the coarse (4-vertex) variant onto the dominant node
/// direction so the degenerate quad still runs along the branch field rather
/// than staying axis-aligned; the fine variant is rotation-invariant enough
/// that canonical leaves it alone.
fn branch_circle(resolution: usize, radius_units: f64, rotate_rad: f32) -> Vec<(f64, f64)> {
    let rotate = rotate_rad as f64;
    (0..resolution)
        .map(|i| {
            let angle = std::f64::consts::TAU * i as f64 / resolution as f64 + rotate;
            (radius_units * angle.cos(), radius_units * angle.sin())
        })
        .collect()
}

/// Canonical `draw_circles`' per-node cross-section: the base branch circle
/// pushed through the movement-derived ellipse matrix and translated onto the
/// node.
///
/// ```text
/// move_x    = movement.x / (scale * branch_radius)
/// move_y    = movement.y / (scale * branch_radius)
/// vsize_inv = 0.5 / (0.01 + hypot(move_x, move_y))
/// matrix    = scale * [ 1 + move_x^2 * vsize_inv,     move_x * move_y * vsize_inv
///                       move_x * move_y * vsize_inv, 1 + move_y^2 * vsize_inv ]
/// ```
///
/// A moving node is stretched along its direction of travel, which is what
/// makes a leaning branch print as a continuous solid instead of a stack of
/// offset discs.
///
/// Canonical gates the matrix on `!SQUARE_SUPPORT && std::abs(moveX) > 0.001
/// && std::abs(moveY) > 0.001` and otherwise falls back to `circle.points[i] *
/// scale + node.position` — the plain scaled circle. Both conditions matter:
/// the degenerate 4-gon of the square-support path is never elongated, and a
/// node whose movement is axis-aligned (one component ~0) keeps a round
/// cross-section rather than being stretched along that single axis.
fn node_ellipse(
    base_circle: &[(f64, f64)],
    center: Point2,
    scale: f64,
    movement: Point2,
    branch_radius_units: f64,
    square_support: bool,
) -> Option<ExPolygon> {
    if base_circle.len() < 3 || scale <= 0.0 || branch_radius_units <= 0.0 {
        return None;
    }
    let denom = scale * branch_radius_units;
    let move_x = movement.x as f64 / denom;
    let move_y = movement.y as f64 / denom;
    let (m00, m01, m11) = if !square_support && move_x.abs() > 0.001 && move_y.abs() > 0.001 {
        let vsize_inv = 0.5 / (0.01 + (move_x * move_x + move_y * move_y).sqrt());
        (
            scale * (1.0 + move_x * move_x * vsize_inv),
            scale * (move_x * move_y * vsize_inv),
            scale * (1.0 + move_y * move_y * vsize_inv),
        )
    } else {
        (scale, 0.0, scale)
    };
    let points: Vec<Point2> = base_circle
        .iter()
        .map(|(x, y)| Point2 {
            x: center.x + (m00 * x + m01 * y).round() as i64,
            y: center.y + (m01 * x + m11 * y).round() as i64,
        })
        .collect();
    Some(ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    })
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
/// Inflate raw model cross-sections into a `draw_circles` carve set.
///
/// Canonical `draw_circles` never carves the drawn circles out of the bare
/// object outline: its local `get_collision` lambda returns
/// `offset_ex(m_layer_outlines[obj_layer_nr], scale_(m_xy_distance))`, so the
/// printed footprint keeps a full `support_object_xy_distance` gap to the wall.
///
/// `SupportAnalysisView::model_occupancy` carries the RAW `SliceIR` region
/// polygons (`support_analysis_producer` inserts `region.polygons` verbatim,
/// with zero inflation). It is preferred over the `TreeVolumes` ladder because
/// it is the exact per-layer occupancy, but it must be inflated here or the
/// carve degenerates to a difference against the wall itself.
///
/// Measured on `resources/regression_wedge.stl` before this inflation: 940 of
/// 19856 emitted role-region vertices sat within 0.05 mm of the model outline
/// (many exactly on it) — e.g. layer 107 vertices at x = 21.0000 and
/// x = 29.0000, flush against the model edges at those coordinates.
fn inflate_model_occupancy(polys: &[ExPolygon], xy_distance_mm: f32) -> Vec<ExPolygon> {
    if polys.is_empty() || xy_distance_mm <= 0.0 {
        return polys.to_vec();
    }
    host::offset_polygons(polys, xy_distance_mm, OffsetJoinType::Miter, 0.0)
}

/// Apply the final emit-time collision carve to already-drawn support regions.
///
/// This is public so contract tests can verify that post-smoothing geometry is
/// passed through the same final gate as an unsmoothed baseline.
#[doc(hidden)]
pub fn carve_emitted_regions(
    regions: &[ExPolygon],
    collision_polys: &[ExPolygon],
) -> Vec<ExPolygon> {
    if collision_polys.is_empty() || regions.is_empty() {
        regions.to_vec()
    } else {
        regions
            .iter()
            .filter_map(|region| {
                host::clip_polygons(
                    std::slice::from_ref(region),
                    collision_polys,
                    ClipOperation::Difference,
                )
                .into_iter()
                .max_by(|a, b| {
                    expolygon_area(a)
                        .total_cmp(&expolygon_area(b))
                        .then_with(|| a.contour.points.cmp(&b.contour.points))
                })
            })
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
#[doc(hidden)]
pub fn build_roles(
    branch_segments: &[Vec<Point3WithWidth>],
    interface_segments: &[Vec<Point3WithWidth>],
    base_segments: &[Vec<Point3WithWidth>],
    floor_segments: &[Vec<Point3WithWidth>],
    branch_areas: &[ExPolygon],
    interface_areas: &[ExPolygon],
    base_areas: &[ExPolygon],
    floor_areas: &[ExPolygon],
    branch_radius: f32,
    collision_polys: &[ExPolygon],
    avg_node_per_layer: usize,
    line_width_mm: f32,
) -> Vec<slicer_ir::SupportPlanRoleRegion> {
    // Both canonical tree engines emit one per-node cross-section per layer;
    // neither adds same-layer connectors between skeleton edges. The segment
    // geometry retained here is therefore limited to degenerate per-node disc
    // fallbacks. Distinct-point segments remain available to the skeleton only.
    //
    // Canonical simplifies only `base_areas`, only for square support, at half
    // the support line width. Roof and floor areas, and every normal-density
    // circle, retain their drawn resolution.
    let with_areas =
        |segments: &[Vec<Point3WithWidth>], areas: &[ExPolygon], is_base_area: bool| {
            let mut regions = structural_body_regions(segments, branch_radius);
            regions.extend_from_slice(areas);
            // Canonical carves each drawn node circle before appending it to
            // the role area. Carving after this union can join separate node
            // circles through collision and then retain only one fragment.
            let regions = carve_emitted_regions(&regions, collision_polys);
            // Canonical accumulates the carved circles into `base_areas` /
            // `roof_areas` and then runs them through Clipper boolean ops
            // (`diff_ex(base_areas, roofs)`, `intersection_ex(base_areas,
            // m_machine_border)`, `closing_ex`/`diff_clipped` for the
            // interfaces). Every one of those returns non-overlapping
            // `ExPolygons`, so adjacent node cross-sections come out fused
            // into a single outline. Emitting them unmerged puts a duplicate
            // perimeter through the inside of each fused branch pair and makes
            // the branch silhouette pop between layers as neighbouring circles
            // drift in and out of contact.
            let regions = union_expolys(regions);
            let regions =
                match role_simplify_tolerance(is_base_area, avg_node_per_layer, line_width_mm) {
                    Some(tolerance) => expolygons_simplify(&regions, tolerance),
                    None => regions,
                };
            if collision_polys.is_empty() {
                regions
            } else {
                // Simplification may move a boundary back into collision. The
                // per-circle largest-part selection already happened above, so
                // this is canonical's plain set-wide difference.
                host::clip_polygons(&regions, collision_polys, ClipOperation::Difference)
            }
        };
    let body = with_areas(branch_segments, branch_areas, true);
    let roof = with_areas(interface_segments, interface_areas, false);
    let base = with_areas(base_segments, base_areas, false);
    let floor = with_areas(floor_segments, floor_areas, false);

    // Subtract interface geometry out of the body, per canonical.
    let mut carved = body;
    for cut in [&roof, &base, &floor] {
        if !carved.is_empty() && !cut.is_empty() {
            carved = host::clip_polygons(&carved, cut, ClipOperation::Difference);
        }
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
    if !base.is_empty() {
        roles.push(slicer_ir::SupportPlanRoleRegion {
            role: slicer_ir::SupportPlanRole::BaseInterface,
            regions: base,
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

fn role_simplify_tolerance(
    is_base_area: bool,
    avg_node_per_layer: usize,
    line_width_mm: f32,
) -> Option<f64> {
    (is_base_area && avg_node_per_layer > COARSE_CIRCLE_NODE_THRESHOLD)
        .then(|| mm_to_units(line_width_mm * 0.5).max(1) as f64)
}

/// Convert planned centerline nodes into semantic support-body regions.
///
/// Only degenerate segments represent per-node disc fallbacks. Distinct-point
/// segments are MST skeleton edges and must not contribute role geometry.
///
/// Zero-width points are the contact tips at the top of a column. They used to
/// be filtered out entirely, which meant the layer that is supposed to meet the
/// overhang produced no printable geometry at all; they are now floored at
/// `MIN_BRANCH_RADIUS` like every other point.
///
pub fn structural_body_regions(
    segments: &[Vec<Point3WithWidth>],
    _branch_radius_mm: f32,
) -> Vec<ExPolygon> {
    let mut regions: Vec<ExPolygon> = Vec::new();
    for segment in segments {
        let Some((first, rest)) = segment.split_first() else {
            continue;
        };
        if rest
            .iter()
            .any(|point| point.x != first.x || point.y != first.y)
        {
            continue;
        }
        if let Some(disc) = swept_region(first, first) {
            regions.push(disc);
        }
    }
    regions
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
    /// Explicit dense band immediately below a roof contact.
    Base,
    /// Floor: the branch lands on the model rather than the build plate.
    Floor,
}

impl InterfaceRole {
    /// Select the collection a single node's segment belongs to.
    fn target_for_node<'a>(
        role: InterfaceRole,
        body: &'a mut Vec<Vec<Point3WithWidth>>,
        roof: &'a mut Vec<Vec<Point3WithWidth>>,
        base: &'a mut Vec<Vec<Point3WithWidth>>,
        floor: &'a mut Vec<Vec<Point3WithWidth>>,
    ) -> &'a mut Vec<Vec<Point3WithWidth>> {
        match role {
            InterfaceRole::Floor => floor,
            InterfaceRole::Roof => roof,
            InterfaceRole::Base => base,
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
        base: &'a mut Vec<Vec<Point3WithWidth>>,
        floor: &'a mut Vec<Vec<Point3WithWidth>>,
    ) -> &'a mut Vec<Vec<Point3WithWidth>> {
        if a == InterfaceRole::Floor || b == InterfaceRole::Floor {
            floor
        } else if a == InterfaceRole::Roof && b == InterfaceRole::Roof {
            roof
        } else if a == InterfaceRole::Base && b == InterfaceRole::Base {
            base
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

// ── Canonical tree-support volumes layer (TreeSupportData) ────────────────
//
// Port of `TreeSupportData::calculate_collision` / `::calculate_avoidance`
// (`OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`).
//
// Canonical:
//   collision(r, l) = simplify(offset_ex(m_layer_outlines[l], scale_(r + m_xy_distance)),
//                              scale_(m_radius_sample_resolution))
//   avoidance(r, l) = union_ex(offset_ex(avoidance(r, l - 1),
//                                        scale_(-m_max_move_distances[l - 1])),
//                              collision(r, l))
//   avoidance(r, 0) = collision(r, 0)
//
// Canonical evaluates the avoidance recurrence lazily with a
// `max_recursion_depth = 100` trampoline; that trampoline exists only to bound
// C++ stack depth. This port fills the whole ladder iteratively bottom-up: a
// wasm guest has a far smaller stack than the host C++ build, and a recursive
// port risks blowing it on tall objects.

/// Canonical `m_radius_sample_resolution`
/// (`g_config_tree_support_collision_resolution` in `libslic3r.h`), in mm.
/// Radii are bucketed to this grid so the collision/avoidance caches stay small.
const RADIUS_SAMPLE_RESOLUTION_MM: f32 = 0.2;

/// Canonical branch-A push-out keeps collision dilation independent from its
/// layer-specific movement budget.
#[doc(hidden)]
pub fn branch_a_move_out_args(max_move_budget: f32) -> (f32, f32) {
    let dilation = RADIUS_SAMPLE_RESOLUTION_MM + CANONICAL_EPSILON_MM;
    (dilation, max_move_budget + dilation)
}

/// STUDIO-4252 retries against collision with `max_move_between_samples` as
/// both the dilation and movement limit.
#[doc(hidden)]
pub fn studio_4252_move_out_args(max_move_distance: f32) -> (f32, f32) {
    let max_move_between_samples =
        max_move_distance + RADIUS_SAMPLE_RESOLUTION_MM + CANONICAL_EPSILON_MM;
    (max_move_between_samples, max_move_between_samples)
}

/// Canonical `m_xy_distance` default — the `support_object_xy_distance` print
/// setting. Used when the config key is absent.
const DEFAULT_SUPPORT_OBJECT_XY_DISTANCE_MM: f32 = 0.35;

/// Canonical `TreeSupportData::ceil_radius`: snap a radius up onto the
/// `m_radius_sample_resolution` grid so nearby radii share a cache slot.
fn ceil_radius(radius_mm: f32) -> f32 {
    if !radius_mm.is_finite() || radius_mm <= 0.0 {
        return 0.0;
    }
    (radius_mm / RADIUS_SAMPLE_RESOLUTION_MM).ceil() * RADIUS_SAMPLE_RESOLUTION_MM
}

/// Cache key for a bucketed radius. Scaled units keep the key exact — an `f32`
/// is not `Hash`/`Eq`, and rounding to units is the same quantisation the
/// geometry itself is subject to.
fn radius_key(radius_mm: f32) -> i64 {
    mm_to_units(ceil_radius(radius_mm))
}

/// Iterative Douglas-Peucker on an open polyline, in scaled units.
///
/// `host::simplify_polygon` ignores its `tolerance_mm` argument on both the
/// wasm and the native path, so canonical's `expolygons_simplify` has to be
/// implemented guest-side. The recursion is an explicit stack: the guest has a
/// small wasm stack and a deeply-sampled contour would otherwise risk it.
fn douglas_peucker_open(points: &[Point2], tolerance_units: f64) -> Vec<Point2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    let last = points.len() - 1;
    keep[last] = true;
    let tol_sq = tolerance_units * tolerance_units;

    let mut stack: Vec<(usize, usize)> = vec![(0, last)];
    while let Some((lo, hi)) = stack.pop() {
        if hi <= lo + 1 {
            continue;
        }
        let a = points[lo];
        let b = points[hi];
        let dx = (b.x - a.x) as f64;
        let dy = (b.y - a.y) as f64;
        let seg_len_sq = dx * dx + dy * dy;
        let mut best_idx = lo;
        let mut best_dist_sq = -1.0_f64;
        for (offset, p) in points[lo + 1..hi].iter().enumerate() {
            let px = (p.x - a.x) as f64;
            let py = (p.y - a.y) as f64;
            let dist_sq = if seg_len_sq <= f64::EPSILON {
                px * px + py * py
            } else {
                // Perpendicular distance to the *segment*, matching Slic3r's
                // `MultiPoint::douglas_peucker`, which clamps the projection.
                let t = ((px * dx + py * dy) / seg_len_sq).clamp(0.0, 1.0);
                let ex = px - t * dx;
                let ey = py - t * dy;
                ex * ex + ey * ey
            };
            if dist_sq > best_dist_sq {
                best_dist_sq = dist_sq;
                best_idx = lo + 1 + offset;
            }
        }
        if best_dist_sq > tol_sq {
            keep[best_idx] = true;
            stack.push((lo, best_idx));
            stack.push((best_idx, hi));
        }
    }

    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| if k { Some(*p) } else { None })
        .collect()
}

/// Canonical `Polygon::simplify`: close the ring, Douglas-Peucker it as an open
/// polyline, then drop the duplicated closing point. Rings that collapse below
/// three vertices are discarded by the caller.
fn simplify_ring(ring: &Polygon, tolerance_units: f64) -> Polygon {
    if ring.points.len() < 3 {
        return ring.clone();
    }
    let mut closed = ring.points.clone();
    closed.push(ring.points[0]);
    let mut simplified = douglas_peucker_open(&closed, tolerance_units);
    simplified.pop();
    Polygon { points: simplified }
}

/// Canonical `expolygons_simplify`.
fn expolygons_simplify(polys: &[ExPolygon], tolerance_units: f64) -> Vec<ExPolygon> {
    polys
        .iter()
        .filter_map(|ex| {
            let contour = simplify_ring(&ex.contour, tolerance_units);
            if contour.points.len() < 3 {
                return None;
            }
            let holes = ex
                .holes
                .iter()
                .map(|h| simplify_ring(h, tolerance_units))
                .filter(|h| h.points.len() >= 3)
                .collect();
            Some(ExPolygon { contour, holes })
        })
        .collect()
}

/// Canonical `ExPolygon::simplify`: simplify each ring, then normalize the
/// resulting paths through a union so touching holes and parts may merge.
fn expolygons_simplify_union(polys: &[ExPolygon], tolerance_units: f64) -> Vec<ExPolygon> {
    let simplified = expolygons_simplify(polys, tolerance_units);
    if simplified.is_empty() {
        return simplified;
    }
    let (head, tail) = simplified.split_at(1);
    host::clip_polygons(head, tail, ClipOperation::Union)
}

/// Union a polygon set with itself, collapsing overlaps.
fn union_expolys(polys: Vec<ExPolygon>) -> Vec<ExPolygon> {
    if polys.len() <= 1 {
        return polys;
    }
    let (head, tail) = polys.split_at(1);
    host::clip_polygons(head, tail, ClipOperation::Union)
}

/// Canonical `TreeSupportData` — the radius-keyed collision / avoidance volumes
/// a tree-support run is planned against.
///
/// `collision` and `avoidance` are keyed by `(radius_key, layer_index)`. Call
/// [`TreeVolumes::ensure_radius`] once per radius bucket before reading; the
/// getters return an empty slice for an unfilled bucket rather than computing
/// on demand, so the expensive host offsets stay out of the inner node loops.
struct TreeVolumes {
    /// Canonical `m_layer_outlines`: each global support layer simplified at
    /// `m_radius_sample_resolution` during construction.
    layer_outlines: Vec<Vec<ExPolygon>>,
    /// Canonical `m_layer_outlines_below`: the running union of every outline
    /// at or below each layer, built in the `TreeSupportData` constructor.
    /// Consumed by the per-part MST and `to_buildplate` passes.
    layer_outlines_below: Vec<Vec<ExPolygon>>,
    /// Canonical `m_max_move_distances[l] = layer->height * branch_scale_factor`
    /// with `branch_scale_factor = tan(tree_support_branch_angle)`, in mm.
    max_move_distances: Vec<f32>,
    /// Canonical `m_xy_distance` — the `support_object_xy_distance` setting.
    xy_distance: f32,
    collision: std::cell::RefCell<std::collections::HashMap<(i64, usize), PolySet>>,
    avoidance: std::cell::RefCell<std::collections::HashMap<(i64, usize), PolySet>>,
}

/// Shared, cheaply-cloned handle to one cached `(radius, layer)` volume.
///
/// The move pass (F-13) queries collision and avoidance at each node's *own*
/// tapered radius, so the bucket set is no longer known before the layer loop
/// and cannot be filled by a pair of `ensure_*` calls up front. The caches
/// therefore materialise lazily behind a `RefCell` and hand out `Rc`s rather
/// than borrows, so a caller can hold a volume across a further `get_*` call
/// that fills a different bucket. The guest is single-threaded, so neither
/// `Rc` nor `RefCell` costs anything here.
type PolySet = std::rc::Rc<Vec<ExPolygon>>;

fn empty_poly_set() -> PolySet {
    std::rc::Rc::new(Vec::new())
}

impl TreeVolumes {
    /// Build the outline stack from `SupportGeometryView` and precompute the
    /// running below-union and the per-layer move budget.
    fn new(
        layer_plan: &LayerPlanView,
        support_geometry: &SupportGeometryView,
        branch_angle_deg: f32,
        xy_distance: f32,
    ) -> Self {
        let layer_count = layer_plan.layers.len();
        let mut layer_outlines: Vec<Vec<ExPolygon>> = vec![Vec::new(); layer_count];
        for entry in &support_geometry.entries {
            let layer_idx = entry.global_support_layer_index as usize;
            if layer_idx >= layer_count {
                continue;
            }
            for expoly in &entry.outlines {
                if expoly.contour.points.len() >= 3 {
                    layer_outlines[layer_idx].push(expoly.clone());
                }
            }
        }

        let outline_tolerance = mm_to_units(RADIUS_SAMPLE_RESOLUTION_MM) as f64;
        for outlines in &mut layer_outlines {
            *outlines = expolygons_simplify_union(outlines, outline_tolerance);
        }

        // Canonical builds `m_layer_outlines_below` as a running union in the
        // `TreeSupportData` constructor. Layers that contribute nothing reuse
        // the accumulator verbatim, which keeps the host call count down to the
        // number of layers that actually carry outlines.
        let mut layer_outlines_below: Vec<Vec<ExPolygon>> = Vec::with_capacity(layer_count);
        let mut below: Vec<ExPolygon> = Vec::new();
        for outlines in &layer_outlines {
            if !outlines.is_empty() {
                below = if below.is_empty() {
                    union_expolys(outlines.clone())
                } else {
                    host::clip_polygons(&below, outlines, ClipOperation::Union)
                };
            }
            layer_outlines_below.push(below.clone());
        }

        let branch_scale_factor = branch_angle_deg.to_radians().tan().max(0.0);
        let max_move_distances = layer_plan
            .layers
            .iter()
            .map(|l| (l.effective_layer_height * branch_scale_factor).max(0.0))
            .collect();

        Self {
            layer_outlines,
            layer_outlines_below,
            max_move_distances,
            xy_distance: xy_distance.max(0.0),
            collision: std::cell::RefCell::new(std::collections::HashMap::new()),
            avoidance: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }

    fn layer_count(&self) -> usize {
        self.layer_outlines.len()
    }

    /// Raw (uninflated) object outlines at a layer — canonical `m_layer_outlines`.
    fn outlines_at(&self, layer: usize) -> &[ExPolygon] {
        self.layer_outlines
            .get(layer)
            .map_or(&[][..], |v| v.as_slice())
    }

    /// Canonical `m_layer_outlines_below` at a layer. The F-12 per-part
    /// spanning trees group nodes by the part of this set they fall in.
    fn outlines_below(&self, layer: usize) -> &[ExPolygon] {
        self.layer_outlines_below
            .get(layer)
            .map_or(&[][..], |v| v.as_slice())
    }

    /// Canonical `TreeSupportData::get_collision`. Materialises the radius
    /// bucket on first use, exactly as canonical's cache does.
    fn get_collision(&self, radius_mm: f32, layer: usize) -> PolySet {
        self.ensure_collision(radius_mm);
        self.collision
            .borrow()
            .get(&(radius_key(radius_mm), layer))
            .cloned()
            .unwrap_or_else(empty_poly_set)
    }

    /// Canonical `TreeSupportData::get_avoidance`. Materialises the radius
    /// bucket on first use.
    fn get_avoidance(&self, radius_mm: f32, layer: usize) -> PolySet {
        self.ensure_avoidance(radius_mm);
        self.avoidance
            .borrow()
            .get(&(radius_key(radius_mm), layer))
            .cloned()
            .unwrap_or_else(empty_poly_set)
    }

    /// Fill the collision ladder for one radius bucket.
    ///
    /// `collision(r, l) = simplify(offset_ex(outlines[l], r + xy_distance))`.
    /// Every layer's inflation is independent, so the whole stack goes to the
    /// host in one batch (ADR-0049) rather than one call per layer.
    fn ensure_collision(&self, radius_mm: f32) {
        let key = radius_key(radius_mm);
        if self.layer_outlines.is_empty() || self.collision.borrow().contains_key(&(key, 0)) {
            return;
        }
        let inflate = ceil_radius(radius_mm) + self.xy_distance;
        let tolerance_units = mm_to_units(RADIUS_SAMPLE_RESOLUTION_MM) as f64;
        let indexed: Vec<usize> = (0..self.layer_count())
            .filter(|l| !self.layer_outlines[*l].is_empty())
            .collect();
        let inflated = slicer_sdk::host_batch::batch_offset(&indexed, |layer| {
            slicer_sdk::host_batch::OffsetRequest {
                polygons: self.layer_outlines[*layer].clone(),
                delta_mm: inflate,
                join: OffsetJoinType::Miter,
                arc_tolerance_mm: 0.0,
                miter_limit: Some(3.0),
            }
        });
        let mut by_layer: Vec<Vec<ExPolygon>> = vec![Vec::new(); self.layer_count()];
        for (layer, polys) in inflated {
            by_layer[*layer] = expolygons_simplify(&polys, tolerance_units);
        }
        let mut cache = self.collision.borrow_mut();
        for (layer, polys) in by_layer.into_iter().enumerate() {
            cache.insert((key, layer), std::rc::Rc::new(polys));
        }
    }

    /// Fill the avoidance ladder for one radius bucket, bottom-up.
    ///
    /// Avoidance is a strict recurrence — each layer erodes the layer below it
    /// — so it is walked serially, but iteratively rather than recursively
    /// (see the module note on canonical's recursion trampoline).
    fn ensure_avoidance(&self, radius_mm: f32) {
        let key = radius_key(radius_mm);
        if self.layer_outlines.is_empty() || self.avoidance.borrow().contains_key(&(key, 0)) {
            return;
        }
        self.ensure_collision(radius_mm);
        let mut previous: Vec<ExPolygon> = Vec::new();
        for layer in 0..self.layer_count() {
            let collision = self.get_collision(radius_mm, layer).as_ref().clone();
            let avoidance = if layer == 0 || previous.is_empty() {
                collision
            } else {
                // `offset_polygons` takes a SIGNED delta in mm and erodes on a
                // negative one, so it serves canonical's negative `offset_ex`.
                let step = self
                    .max_move_distances
                    .get(layer - 1)
                    .copied()
                    .unwrap_or(0.0);
                let eroded = if step <= 0.0 {
                    previous.clone()
                } else {
                    host::offset_polygons_with_miter_limit(
                        &previous,
                        -step,
                        OffsetJoinType::Miter,
                        0.0,
                        3.0,
                    )
                };
                if eroded.is_empty() {
                    collision
                } else if collision.is_empty() {
                    eroded
                } else {
                    host::clip_polygons(&eroded, &collision, ClipOperation::Union)
                }
            };
            previous = avoidance.clone();
            self.avoidance
                .borrow_mut()
                .insert((key, layer), std::rc::Rc::new(avoidance));
        }
    }
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
        let nozzle_diameter = config.get_float("nozzle_diameter").unwrap_or(0.0);
        let support_line_width_mm = config
            .get_abs_value("support_line_width", nozzle_diameter)
            // Preserve hand-written legacy configs that encode an absolute
            // width as an integer rather than a Float/FloatOrPercent.
            .or_else(|| config.get_int("support_line_width").map(|v| v as f64))
            .map(|v| {
                if v > 0.0 {
                    v as f32
                } else {
                    nozzle_diameter as f32
                }
            })
            .filter(|v| *v > 0.0)
            .unwrap_or(DEFAULT_SUPPORT_LINE_WIDTH_MM);
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
        let tree_support_style = TreeSupportStyle::from_config(config);
        let organic_substitution = organic_substitution_requested(config);
        // Canonical `is_slim = support_style == smsTreeSlim`.
        let tree_support_is_slim = tree_support_style == TreeSupportStyle::Slim;
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
        let num_top_base_interface_layers = match config.get("num_top_base_interface_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => 0,
        }
        .max(0);
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
        // Canonical `m_xy_distance`. Same key and default (0.35 mm) as
        // `traditional-support-planner`.
        let support_object_xy_distance = match config.get("support_object_xy_distance") {
            Some(ConfigValue::Float(d)) => *d as f32,
            Some(ConfigValue::Int(d)) => *d as f32,
            _ => DEFAULT_SUPPORT_OBJECT_XY_DISTANCE_MM,
        }
        .max(0.0);
        // Existing key: 0.0 is the documented "same as the model layer height"
        // sentinel.
        let support_layer_height_mm = match config.get("support_layer_height_mm") {
            Some(ConfigValue::Float(d)) => *d as f32,
            Some(ConfigValue::Int(d)) => *d as f32,
            _ => 0.0,
        };
        // Packet 239c: default true, matching the manifest declaration and
        // canonical `PrintConfig.cpp` `init_fff_params` (coBool, default true).
        // When true, `plan_for_object` derives free-floating intermediate
        // support planes from `support_layer_height_mm`; when false the plan
        // is byte-identical to the pre-239c grid-exact behavior.
        let independent_support_layer_height = config
            .get_bool("independent_support_layer_height")
            .unwrap_or(true);
        let max_bridge_length_mm = match config.get("max_bridge_length") {
            Some(ConfigValue::Float(length)) if *length > 0.0 => *length as f32,
            Some(ConfigValue::Int(length)) if *length > 0 => *length as f32,
            _ => DEFAULT_MAX_BRIDGE_LENGTH_MM,
        };
        Ok(Self {
            enabled,
            support_family,
            branch_angle_deg,
            support_line_width_mm,
            max_branches_per_layer,
            line_width_mm,
            tree_support_branch_diameter,
            tree_support_branch_diameter_angle,
            tree_support_branch_distance,
            tree_support_wall_count,
            tree_support_is_slim,
            tree_support_style,
            organic_substitution_requested: organic_substitution,
            support_raft_layers,
            raft_first_layer_density,
            base_raft_layers,
            interface_raft_layers,
            support_interface_top_layers,
            num_top_base_interface_layers,
            support_interface_bottom_layers,
            support_on_build_plate_only,
            support_top_z_distance_mm,
            support_layer_height_mm,
            independent_support_layer_height,
            support_object_xy_distance,
            max_bridge_length_mm,
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

        if self.organic_substitution_requested {
            // The typed diagnostic lands in host audits; the log line is what
            // reaches the console (see machine-gcode-emit's code-12 pattern).
            slicer_sdk::host::log_warn(concat!(
                "support_style=organic requested but the organic tree engine ",
                "(canonical TreeSupport3D) is not implemented; running the ",
                "classic tree engine, strong style (code 1005)"
            ));
            let _ = output.push_diagnostic(Diagnostic {
                severity: DiagnosticSeverity::Warn,
                code: 1005,
                layer: None,
                object_id: None,
                message: concat!(
                    "support_style=organic requested but the organic tree engine ",
                    "(canonical TreeSupport3D) is not implemented; running the ",
                    "classic tree engine, strong style"
                )
                .into(),
            });
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

        // ── Build the canonical tree-support volumes (TreeSupportData) ────
        //
        // Defect F-16: this used to be a per-layer `LayerCollisionCache` whose
        // collision set was the raw, ZERO-inflated outlines and whose avoidance
        // set was those outlines inflated by
        // `branch_radius + tree_support_branch_distance / 2` — a per-layer
        // quantity with no recursion. Canonical inflates collision by
        // `radius + m_xy_distance` and derives avoidance as a recurrence down
        // the layer stack, which is what stops a branch being trapped.
        let volumes = TreeVolumes::new(
            layer_plan,
            support_geometry,
            self.branch_angle_deg,
            self.support_object_xy_distance,
        );
        // Both ladders are now keyed on the canonical *avoidance query radius*
        // `calc_radius(dist_mm_to_top + height)` (global base-radius taper) and
        // materialise on demand (see `PolySet`). Until packet 224 step 5 the
        // module pre-filled exactly two buckets here — `collision(0.0)` and
        // `avoidance(branch_radius)` — because the getters returned borrows
        // and could not be filled inside the layer loop. The canonical move
        // pass (F-13) queries `get_avoidance(calc_radius(...), l)` and
        // `get_collision(calc_radius(...), l)`, so the bucket set is not
        // knowable before the loop runs.

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
                &volumes,
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
        volumes: &TreeVolumes,
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

        // ── Canonical `generate_contact_points` head (F-34) ───────────────
        // `top_z_distance = max(top_z_distance, min_layer_height)` when the
        // configured gap is non-zero; a configured zero gap stays zero (that
        // is the soluble-interface case).
        let min_layer_height = layer_plan
            .layers
            .iter()
            .map(|layer| layer.effective_layer_height)
            .fold(f32::INFINITY, f32::min);
        let nominal_layer_height = layer_plan.layers[0].effective_layer_height;
        let z_distance_top = if self.support_top_z_distance_mm > f32::EPSILON {
            self.support_top_z_distance_mm.max(min_layer_height)
        } else {
            0.0
        };
        // Canonical `round_up_divide(scale_(z_distance_top), scale_(layer_height)) + 1`
        // — "support must always be 1 layer below overhang".
        let z_distance_top_layers = if nominal_layer_height > 0.0 {
            let num = mm_to_units(z_distance_top);
            let den = mm_to_units(nominal_layer_height).max(1);
            (num.div_euclid(den) + i64::from(num.rem_euclid(den) != 0)) as usize + 1
        } else {
            1
        };
        // Canonical "fix bug of generating support for very thin objects".
        if layer_plan.layers.len() <= z_distance_top_layers + 1 {
            return Ok(());
        }
        let contact_ctx = ContactContext {
            z_distance_top,
            gap_layers: i32::from(z_distance_top != 0.0),
            support_roof_layers: self.support_interface_top_layers.max(0),
        };

        // Host analysis owns candidate discovery and policy. Geometry remains
        // an independent compatibility input for collision avoidance below.
        let mut arena = NodeArena::default();
        let mut contacts_by_layer: Vec<Vec<NodeId>> = vec![Vec::new(); num_layers as usize];
        let mut fallback_family_emitted = false;
        let base_radius = MIN_BRANCH_RADIUS.max(self.tree_support_branch_diameter / 2.0);
        // Canonical builds `grid_points` once per object over the whole-object
        // bounding box, rotated 22 degrees (F-35).
        let sample_step = self
            .tree_support_branch_distance
            .max(self.max_bridge_length_mm / 2.0);
        let object_grid: Option<Vec<(f32, f32)>> = compute_bounds(&obj.vertices)
            .map(|(min, max)| build_grid_points((min[0], max[0], min[1], max[1]), sample_step));

        // Per-affected-layer drop count for the code 1001 cap diagnostic.
        // Keyed by global_layer_index so the message carries the right value
        // even when layer_rev doesn't line up with the layer-plan index.
        // Owned by run_support_geometry; this function increments into the
        // shared map so per-layer totals are merged across all objects
        // before emission.

        // Canonical `generate_contact_points` reads ONE contact source: the
        // host-computed per-layer overhang polygons (`layer->loverhangs`).
        // In-tree that source is `SupportAnalysisView::candidates`, whose
        // geometry `detect_support_overhangs` derives with the same 2D
        // slice-difference canonical uses. The mesh-facet projection below is
        // a compatibility shim for fixtures that carry no analysis at all
        // (coplanar plates whose closed-solid cross-section is empty).
        //
        // Running BOTH seeds two independent contact chains for the same
        // overhang, and the union of their two roof bands is longer than the
        // configured `support_interface_top_layers`. Measured on
        // SupportTest.stl before this gate: top=1 emitted 2 interface layers,
        // top=2 emitted 4, top=3 emitted 5, while traditional (single-source)
        // emitted the correct 1/2/3.
        let has_analysis_contacts = support_analysis.candidates.iter().any(|candidate| {
            candidate.object_id == obj.object_id
                && candidate
                    .geometry
                    .iter()
                    .any(|polygon| polygon.contour.points.len() >= 3)
        });
        if let Some((bmin, _)) = compute_bounds(&obj.vertices).filter(|_| !has_analysis_contacts) {
            let blockers = collect_paint_blocker_polygons(obj);
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
                let samples = sample_contact_points(
                    &polygons,
                    object_grid.as_deref(),
                    self.tree_support_branch_distance,
                    base_radius,
                    self.max_bridge_length_mm,
                    false,
                );
                for sample in samples {
                    if point_in_any_polygon(&blockers, sample.x, sample.y) {
                        continue;
                    }
                    let overhang = &polygons[sample.overhang];
                    // Canonical `add_interface = area(overhang) > minimum_roof_area
                    // && !is_sharp_tail` — the F-1 per-node roof seed.
                    let oc = OverhangContext {
                        ctx: &contact_ctx,
                        overhang,
                        add_interface: expolygon_area(overhang) > minimum_roof_area(),
                        is_sharp_tail: false,
                    };
                    // Name the demand after the layer the contact lands on so
                    // the id stays stable against the one-layer shift.
                    let target_idx = layer_idx.saturating_sub(1);
                    let demand_id = format!(
                        "mesh-demand-{}-{}",
                        layer_plan.layers[target_idx].global_layer_index,
                        contacts_by_layer[target_idx].len()
                    );
                    insert_contact_point(
                        &mut arena,
                        &mut contacts_by_layer,
                        layer_plan,
                        self,
                        volumes,
                        dropped_by_layer,
                        layer_idx,
                        (sample.x, sample.y),
                        sample.radius,
                        sample.is_corner,
                        &oc,
                        demand_id,
                    );
                }
            }
            let enforcer_overhang = empty_expolygon();
            for (layer_idx, x, y) in collect_paint_enforcer_contacts(obj) {
                if point_in_any_polygon(&blockers, x, y) {
                    continue;
                }
                let layer_idx = (layer_idx as usize).min(num_layers as usize - 1);
                // Canonical fakes vertical enforcer points as sharp tails so
                // the contact distance is zero.
                let oc = OverhangContext {
                    ctx: &contact_ctx,
                    overhang: &enforcer_overhang,
                    add_interface: false,
                    is_sharp_tail: true,
                };
                let target_idx = layer_idx.saturating_sub(1);
                let demand_id = format!(
                    "mesh-demand-{}-{}",
                    layer_plan.layers[target_idx].global_layer_index,
                    contacts_by_layer[target_idx].len()
                );
                insert_contact_point(
                    &mut arena,
                    &mut contacts_by_layer,
                    layer_plan,
                    self,
                    volumes,
                    dropped_by_layer,
                    layer_idx,
                    (x, y),
                    base_radius,
                    false,
                    &oc,
                    demand_id,
                );
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
            let samples = sample_contact_points(
                &candidate.geometry,
                object_grid.as_deref(),
                self.tree_support_branch_distance,
                base_radius,
                self.max_bridge_length_mm,
                false,
            );
            for (sample_idx, sample) in samples.into_iter().enumerate() {
                let overhang = &candidate.geometry[sample.overhang];
                let oc = OverhangContext {
                    ctx: &contact_ctx,
                    overhang,
                    add_interface: expolygon_area(overhang) > minimum_roof_area(),
                    is_sharp_tail: false,
                };
                // Canonical `generate_contact_points` has exactly ONE contact
                // seeding rule: the node goes on `layer_nr - 1` and starts at
                // `distance_to_top = -gap_layers` (the virtual top-Z-gap node
                // that `draw_circles` diverts into `roof_gap_areas`). Step 2
                // gave analysis candidates their own unshifted rule on the
                // reasoning that the host hands over a *contact* layer rather
                // than an overhang layer; it does not — `detect_support_over-
                // hangs` reports the layer that CONTAINS the overhang, the
                // same input canonical shifts. Running two rules seeded two
                // roof bands one layer apart for the same overhang, and their
                // union gave N+1..N+2 interface layers instead of N
                // (measured: top=1 -> 2, top=2 -> 4, top=3 -> 5 on
                // SupportTest.stl).
                insert_contact_point(
                    &mut arena,
                    &mut contacts_by_layer,
                    layer_plan,
                    self,
                    volumes,
                    dropped_by_layer,
                    layer_idx,
                    (sample.x, sample.y),
                    sample.radius,
                    sample.is_corner,
                    &oc,
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
                let overhang = entry
                    .outlines
                    .first()
                    .cloned()
                    .unwrap_or_else(empty_expolygon);
                let oc = OverhangContext {
                    ctx: &contact_ctx,
                    overhang: &overhang,
                    add_interface: expolygon_area(&overhang) > minimum_roof_area(),
                    is_sharp_tail: false,
                };
                let target_idx = layer_idx.saturating_sub(1);
                let demand_id = format!(
                    "mesh-demand-{}-{}",
                    layer_plan.layers[target_idx].global_layer_index,
                    contacts_by_layer[target_idx].len()
                );
                insert_contact_point(
                    &mut arena,
                    &mut contacts_by_layer,
                    layer_plan,
                    self,
                    volumes,
                    dropped_by_layer,
                    layer_idx,
                    (x, y),
                    base_radius,
                    false,
                    &oc,
                    demand_id,
                );
            }
        }

        // Bail out when nothing needs support.
        if contacts_by_layer.iter().all(|v| v.is_empty()) {
            return Ok(());
        }

        // Canonical closes `generate_contact_points` with a line fit over
        // every contact position, feeding `smooth_nodes` in step 6 (F-33).
        let contact_positions: Vec<(f32, f32)> = contacts_by_layer
            .iter()
            .flat_map(|layer| layer.iter())
            .map(|id| arena[*id].xy())
            .collect();
        let nonempty_layers = contacts_by_layer
            .iter()
            .filter(|layer| !layer.is_empty())
            .count();
        let contact_stats = contact_stats(&contact_positions, nonempty_layers);

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
        // wall_count multiplier — fall back to 1 per canonical
        // `generate_toolpaths` (`TreeSupport.cpp`)
        let wall_count_factor = self.tree_support_wall_count.max(1) as f32;

        // Node ids only. The nodes themselves live in `arena`, so a
        // back-edge written into an upper-layer node survives the handoff.
        let mut active_nodes: Vec<NodeId> = Vec::new();

        // Accumulate entries bottom-up so the plan keeps a deterministic,
        // top-to-bottom layer order in output.
        let mut entries_in_order: Vec<SupportPlanEntry> = Vec::new();

        // Canonical `drop_nodes`' `unsupported_branch_leaves` (F-14): branch
        // leaves with no legal footing, drained after every layer has run.
        let mut unsupported_branch_leaves: std::collections::VecDeque<NodeId> =
            std::collections::VecDeque::new();
        // Per-layer committed state for the emit pass.
        let mut layer_records: Vec<LayerRecord> = Vec::new();

        // Iterate top → bottom.
        let top = num_layers as usize;
        for layer_rev in (0..top).rev() {
            // Canonical caches `is_line_cut_by_contour` per `drop_nodes` layer
            // pass; the contours it tests against are this layer's.
            let mut line_cut = LineCutCache::default();
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
            active_nodes.sort_by(|a, b| {
                let (ax, ay) = arena[*a].xy();
                let (bx, by) = arena[*b].xy();
                match ax.partial_cmp(&bx) {
                    Some(std::cmp::Ordering::Equal) | None => {
                        ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    Some(ord) => ord,
                }
            });

            // Geometry constants for this layer. These are hoisted above the
            // MST/merge block because the F-11 two-leaf collapse creates its
            // merged node on the *next* layer down and needs the budget and
            // the volumes here.
            let effective_height = layer_plan.layers[layer_rev].effective_layer_height;
            // Wall-count scaled max move distance (Step 5 AC-5)
            let max_move_xy = (tan_angle * effective_height * wall_count_factor).max(0.0);
            let z_current = layer_plan.layers[layer_rev].z;
            // Collision/avoidance polygons for this layer (Step 5 AC-3)
            let cache_idx = current_global_layer_index as usize;
            let next_cache_idx = cache_idx.saturating_sub(1);
            let (next_print_z, next_layer_height) = if layer_rev > 0 {
                (
                    layer_plan.layers[layer_rev - 1].z,
                    layer_plan.layers[layer_rev - 1].effective_layer_height,
                )
            } else {
                (z_current, effective_height)
            };

            // -- F-12: per-part spanning trees ---------------------------
            // Canonical `drop_nodes` sizes `nodes_per_part` at
            // `1 + parts.size()`, with
            // `parts = m_ts_data->m_layer_outlines_below[obj_layer_nr]`, and
            // runs `MinimumSpanningTree` once **per group**. The module used
            // to run one global Prim MST over every active node, so nodes on
            // opposite sides of the object could become MST neighbours, merge,
            // and drag each other across it.
            let parts: Vec<ExPolygon> = volumes.outlines_below(cache_idx).to_vec();
            let group_of: Vec<usize> = active_nodes
                .iter()
                .map(|id| {
                    let node = &arena[*id];
                    assign_node_group(&parts, node.to_buildplate, node.x(), node.y())
                })
                .collect();
            // F-14: the canonical grouping loop drops a node that must reach
            // the plate but no longer can — `continue` before `nodes_per_part`,
            // so it joins no spanning tree, is never moved and is never
            // propagated — and files it as an unsupported branch leaf so the
            // whole column above it is pruned.
            let mut unsupported: Vec<bool> = vec![false; active_nodes.len()];
            for (i, id) in active_nodes.iter().enumerate() {
                if self.support_on_build_plate_only && !arena[*id].to_buildplate {
                    unsupported_branch_leaves.push_back(*id);
                    unsupported[i] = true;
                }
            }
            let mut mst_edges: Vec<(usize, usize, f32)> = Vec::new();
            for group in 0..=parts.len() {
                let members: Vec<usize> = (0..active_nodes.len())
                    .filter(|i| group_of[*i] == group)
                    .collect();
                if members.len() < 2 {
                    continue;
                }
                let positions: Vec<(f32, f32)> = members
                    .iter()
                    .map(|i| arena[active_nodes[*i]].xy())
                    .collect();
                for (a, b, d) in prim_mst(&positions) {
                    let (ga, gb) = (members[a], members[b]);
                    mst_edges.push((ga.min(gb), ga.max(gb), d));
                }
            }
            mst_edges.sort_by(|a, b| {
                a.0.cmp(&b.0)
                    .then(a.1.cmp(&b.1))
                    .then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            });

            // -- F-11: canonical merge -----------------------------------
            // The previous rule was `if edge_length < merge_distance_mm { drop
            // the higher-INDEX endpoint }` - a flat invented constant with no
            // leaf-degree test, no midpoint node and no `dist_mm_to_top`
            // ordering. Canonical's first `drop_nodes` pass has two branches,
            // both keyed on the MST adjacency of the node's own group.
            let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); active_nodes.len()];
            for (a, b, _) in &mst_edges {
                adjacency[*a].push(*b);
                adjacency[*b].push(*a);
            }
            let mut drop = unsupported;
            // Branch A's merged node is created directly on the layer below -
            // canonical emits it into `contact_nodes[layer_nr - 1]`, so it
            // never sees this layer's move pass.
            let mut collapsed_into_next: Vec<NodeId> = Vec::new();
            for i in 0..active_nodes.len() {
                if drop[i] {
                    continue;
                }
                let id = active_nodes[i];
                if !arena[id].valid {
                    continue;
                }
                let max_move_dist_sq =
                    get_max_move_dist(&arena[id], tan_angle, self.support_line_width_mm, 2);
                let (node_x, node_y) = arena[id].xy();
                let neighbours: Vec<usize> = adjacency[i].clone();

                if neighbours.len() == 1 {
                    // Branch A - two-leaf collapse. Every condition is
                    // required: the neighbour is within `get_max_move_dist`
                    // (squared, mm), the neighbour is itself a leaf, and the
                    // neighbour is not an `ePolygon` node.
                    let j = neighbours[0];
                    let nid = active_nodes[j];
                    if drop[j] || !arena[nid].valid {
                        continue;
                    }
                    let (nb_x, nb_y) = arena[nid].xy();
                    let dist_sq =
                        (nb_x - node_x) * (nb_x - node_x) + (nb_y - node_y) * (nb_y - node_y);
                    if dist_sq >= max_move_dist_sq
                        || adjacency[j].len() != 1
                        || arena[nid].type_ == TreeNodeType::Polygon
                    {
                        continue;
                    }
                    // The merged node sits at the midpoint of the two.
                    let mut next_position = ((node_x + nb_x) * 0.5, (node_y + nb_y) * 0.5);
                    // Parent selection: whichever of the two is further from
                    // the top wins; when only one has a parent, that one.
                    let (self_parent, nb_parent) = (arena[id].parent, arena[nid].parent);
                    let parent_id = match (self_parent, nb_parent) {
                        (Some(_), Some(_)) | (None, None) => {
                            if arena[id].dist_mm_to_top >= arena[nid].dist_mm_to_top {
                                id
                            } else {
                                nid
                            }
                        }
                        (Some(_), None) => id,
                        (None, Some(_)) => nid,
                    };
                    let other_id = if parent_id == id { nid } else { id };
                    let next_distance_to_top = arena[parent_id].distance_to_top.saturating_add(1);
                    let next_dist_mm_to_top =
                        arena[parent_id].dist_mm_to_top + arena[parent_id].height;
                    let next_radius = arena[parent_id].radius
                        + (next_dist_mm_to_top - arena[parent_id].dist_mm_to_top)
                            * tan_diameter_angle;
                    if group_of[i] == 0 {
                        // Canonical `drop_nodes` branch-A push-out queries the
                        // avoidance ladder at
                        // `calc_radius(node.dist_mm_to_top + height_next)`;
                        // the created node's stored radius stays the ctor
                        // inheritance (`next_radius` below).
                        let avoid_radius = calc_radius(
                            branch_radius,
                            tan_diameter_angle,
                            next_dist_mm_to_top,
                            self.support_interface_top_layers,
                        );
                        let avoidance = volumes.get_avoidance(avoid_radius, next_cache_idx);
                        let (dilation, max_move) = branch_a_move_out_args(max_move_xy);
                        let _ =
                            move_out_expolys(&avoidance, &mut next_position, dilation, max_move);
                    }
                    let collision = volumes.get_collision(0.0, next_cache_idx);
                    let to_buildplate = branch_a_to_buildplate(&collision, next_position);
                    let roof_below = branch_a_roof_counter(
                        arena[parent_id].support_roof_layers_below,
                        arena[parent_id].distance_to_top,
                    );
                    let is_sharp_tail = arena[id].is_sharp_tail || arena[nid].is_sharp_tail;
                    let mut demand_ids = arena[id].demand_ids.clone();
                    for d in arena[nid].demand_ids.clone() {
                        if !demand_ids.contains(&d) {
                            demand_ids.push(d);
                        }
                    }
                    let (parent_x, parent_y) = arena[parent_id].xy();
                    // Canonical: `node_parent->merged_neighbours.push_front(
                    // node_parent == p_node ? neighbour : p_node)` BEFORE
                    // `create_node`, so the ctor loop wires the faded twin's
                    // `child` and `parents` entry.
                    arena[parent_id].merged_neighbours.push(other_id);
                    let new_id = arena.create_node(
                        Point2::from_mm(next_position.0, next_position.1),
                        next_distance_to_top,
                        layer_rev.saturating_sub(1),
                        roof_below,
                        to_buildplate,
                        Some(parent_id),
                        next_print_z,
                        next_layer_height,
                        next_dist_mm_to_top,
                        next_radius,
                    );
                    arena[new_id].movement =
                        Point2::from_mm(next_position.0 - parent_x, next_position.1 - parent_y);
                    arena[new_id].max_move_dist = max_move_xy;
                    arena[new_id].is_sharp_tail = is_sharp_tail;
                    arena[new_id].demand_ids = demand_ids;
                    // Both originals feed the merged node: `create_node`'s
                    // ctor loop wired `other_id.child` / `parents` from the
                    // `merged_neighbours` push above.
                    arena[id].valid = false;
                    arena[nid].valid = false;
                    drop[i] = true;
                    drop[j] = true;
                    collapsed_into_next.push(new_id);
                } else if neighbours.len() > 1 {
                    // Branch B - absorb every close neighbour into this node.
                    let node_dist_mm_to_top = arena[id].dist_mm_to_top;
                    for j in neighbours {
                        if drop[j] || j == i {
                            continue;
                        }
                        let nid = active_nodes[j];
                        if !arena[nid].valid || arena[nid].type_ == TreeNodeType::Polygon {
                            continue;
                        }
                        let (nb_x, nb_y) = arena[nid].xy();
                        let dist_sq =
                            (nb_x - node_x) * (nb_x - node_x) + (nb_y - node_y) * (nb_y - node_y);
                        if dist_sq >= max_move_dist_sq {
                            continue;
                        }
                        // STUDIO-6326: only the bigger node absorbs. Without
                        // this, two nodes at different heights each claim the
                        // other and the column forks.
                        if node_dist_mm_to_top < arena[nid].dist_mm_to_top {
                            continue;
                        }
                        let ids = arena[nid].demand_ids.clone();
                        for d in ids {
                            if !arena[id].demand_ids.contains(&d) {
                                arena[id].demand_ids.push(d);
                            }
                        }
                        let (dist, roof) = {
                            let removed = &arena[nid];
                            (removed.distance_to_top, removed.support_roof_layers_below)
                        };
                        // Canonical also splices the absorbed node's own
                        // merged list, so `child` reassignment reaches every
                        // transitively-absorbed node of the column.
                        let grand_merged = arena[nid].merged_neighbours.clone();
                        let keep = &mut arena[id];
                        keep.distance_to_top = keep.distance_to_top.max(dist);
                        keep.support_roof_layers_below =
                            insert_dropped_node_roof_counter(keep.support_roof_layers_below, roof);
                        keep.merged_neighbours.push(nid);
                        keep.merged_neighbours.extend(grand_merged);
                        arena[nid].valid = false;
                        drop[j] = true;
                    }
                }
            }
            // ── F-13: canonical `drop_nodes` move pass ───────────────────
            //
            // Until packet 224 step 5 this stepped toward a 1/d^2 weighted
            // *mean of neighbour positions*, capped the displacement at
            // `tan_angle * layer_height * wall_count`, then post-hoc clamped
            // the result out of avoidance and dropped the node with a typed
            // code-1002 `node-clamped-out` diagnostic when the escape exceeded
            // that budget. None of the capping, the clamping or the escape
            // budget is canonical: canonical always takes a step of exactly
            // `get_max_move_dist(&node)` along a direction that is either the
            // outward projection out of the *next* layer's avoidance or the
            // neighbour-convergence direction. The 1/d^2 weighting survives as
            // the direction (`neighbour_direction_sum`); the caps do not.
            let mut next_nodes: Vec<NodeId> = Vec::with_capacity(active_nodes.len());
            // Steps 3-4 built the neighbour lists from **every** MST edge,
            // including edges whose endpoint the merge pass had since dropped,
            // so a survivor could aim at a merged-away neighbour's stale
            // position. Both endpoints must still be live.
            let live_edges: Vec<(usize, usize)> = mst_edges
                .iter()
                .filter(|(a, b, _)| {
                    // `valid` here is the *merge* pass flag, read before the
                    // move pass runs, so a node the move pass later terminates
                    // on the model keeps its edges for this layer emit.
                    !drop[*a]
                        && !drop[*b]
                        && arena[active_nodes[*a]].valid
                        && arena[active_nodes[*b]].valid
                })
                .map(|(a, b, _)| (*a, *b))
                .collect();
            let mut neighbours_of: Vec<Vec<usize>> = vec![Vec::new(); active_nodes.len()];
            for (a, b) in &live_edges {
                neighbours_of[*a].push(*b);
                neighbours_of[*b].push(*a);
            }

            // Canonical `DO_NOT_MOVER_UNDER_MM = is_slim ? 0 : 5`: below this
            // print_z a branch is not allowed to converge onto its neighbours
            // at all, so the column stays plumb near the plate.
            let do_not_move_under = if self.tree_support_is_slim {
                0.0
            } else {
                DO_NOT_MOVER_UNDER_MM
            };
            let layer_outlines = volumes.outlines_at(cache_idx).to_vec();

            for i in 0..active_nodes.len() {
                if drop[i] {
                    continue;
                }
                let id = active_nodes[i];
                if !arena[id].valid {
                    continue;
                }
                let (node_x, node_y) = arena[id].xy();
                let distance_to_top = arena[id].distance_to_top;
                let support_roof_layers_below = arena[id].support_roof_layers_below;
                let radius = arena[id].radius;
                let is_sharp_tail = arena[id].is_sharp_tail;
                let dist_mm_to_top = arena[id].dist_mm_to_top;
                let print_z = arena[id].print_z;
                let skin_direction = arena[id].skin_direction;
                let demand_ids = arena[id].demand_ids.clone();

                // Canonical: "If the branch falls completely inside a collision
                // area (the entire branch would be removed by the X/Y offset),
                // delete it." Only `support_on_buildplate_only` escalates that
                // to pruning the whole column; otherwise canonical just clears
                // `valid`, which stops propagation but still DRAWS the node on
                // its own layer — that is how a branch terminates on the model.
                if group_of[i] > 0 {
                    let collision = volumes.get_collision(0.0, cache_idx);
                    if is_inside_ex(&collision, node_x, node_y) {
                        let to_outside = projection_onto(&collision, (node_x, node_y));
                        let dist2_to_outside = (to_outside.0 - node_x) * (to_outside.0 - node_x)
                            + (to_outside.1 - node_y) * (to_outside.1 - node_y);
                        if dist2_to_outside >= radius * radius {
                            if self.support_on_build_plate_only {
                                unsupported_branch_leaves.push_back(id);
                            } else {
                                arena[id].valid = false;
                            }
                            continue;
                        }
                        if let Some(parent) = arena[id].parent {
                            // "if the link between parent and current is cut by
                            // contours, mark current as bottom contact node".
                            let parent_xy = arena[parent].xy();
                            if line_cut.is_line_cut_by_contour(
                                &layer_outlines,
                                (node_x, node_y),
                                parent_xy,
                            ) {
                                arena[id].valid = false;
                                continue;
                            }
                        }
                    }
                }

                // Canonical `get_max_move_dist(&node)` — the FULL step length,
                // not a cap on a shorter one.
                let max_move =
                    get_max_move_dist(&arena[id], tan_angle, self.support_line_width_mm, 1);
                let max_move2 =
                    get_max_move_dist(&arena[id], tan_angle, self.support_line_width_mm, 2);

                // Canonical `drop_nodes`: the descendant is one layer closer
                // to the plate, and the per-node roof counter (F-1) ticks down
                // only once the column is real — the virtual top-Z-gap node
                // (`distance_to_top < 0`) does not consume a roof layer.
                let next_distance_to_top = distance_to_top.saturating_add(1);
                let next_roof_layers_below =
                    support_roof_layers_below - i32::from(distance_to_top >= 0);

                // -- `move_to_neighbor_center` ---------------------------
                let neighbours = &neighbours_of[i];
                let mut move_to_neighbor_center = (0.0_f32, 0.0_f32);
                let first_d2 = neighbours
                    .first()
                    .map(|j| {
                        let (nx, ny) = arena[active_nodes[*j]].xy();
                        (nx - node_x) * (nx - node_x) + (ny - node_y) * (ny - node_y)
                    })
                    .unwrap_or(0.0);
                if print_z > do_not_move_under
                    && (neighbours.len() > 1 || (neighbours.len() == 1 && first_d2 >= max_move2))
                {
                    let branch_bottom_radius = calc_radius(
                        branch_radius,
                        tan_diameter_angle,
                        dist_mm_to_top + print_z,
                        self.support_interface_top_layers,
                    );
                    let mut converging: Vec<(f32, f32)> = Vec::with_capacity(neighbours.len());
                    for &j in neighbours {
                        let nid = active_nodes[j];
                        if !arena[nid].valid {
                            continue;
                        }
                        let (nx, ny) = arena[nid].xy();
                        let d2 = (nx - node_x) * (nx - node_x) + (ny - node_y) * (ny - node_y);
                        if d2 <= 0.0 {
                            continue;
                        }
                        let neighbour_bottom_radius = calc_radius(
                            branch_radius,
                            tan_diameter_angle,
                            arena[nid].dist_mm_to_top + arena[nid].print_z,
                            self.support_interface_top_layers,
                        );
                        let max_converge_distance = tan_angle * (print_z - do_not_move_under)
                            + branch_bottom_radius.max(neighbour_bottom_radius);
                        if d2 > max_converge_distance * max_converge_distance {
                            continue;
                        }
                        if line_cut.is_line_cut_by_contour(
                            &layer_outlines,
                            (node_x, node_y),
                            (nx, ny),
                        ) {
                            continue;
                        }
                        converging.push((nx, ny));
                    }
                    if arena[id].type_ != TreeNodeType::Polygon {
                        move_to_neighbor_center = style_neighbour_direction_for(
                            self.tree_support_style,
                            (node_x, node_y),
                            &converging,
                        );
                    }
                }

                // -- `direction_to_outer` --------------------------------
                let next_dist_mm_to_top = dist_mm_to_top + arena[id].height;
                let inherited_radius =
                    radius + (next_dist_mm_to_top - dist_mm_to_top) * tan_diameter_angle;
                // Canonical `drop_nodes` move pass queries the avoidance and
                // collision ladders at `calc_radius(node.dist_mm_to_top +
                // height_next)` — the global base-radius taper — while the
                // child's *stored* radius stays the ctor inheritance below.
                // Two distinct quantities; with support_interface_top_layers>0
                // the ladder is additionally raised to base_radius.
                let next_avoid_radius = calc_radius(
                    branch_radius,
                    tan_diameter_angle,
                    next_dist_mm_to_top,
                    self.support_interface_top_layers,
                );
                let avoidance_next = volumes.get_avoidance(next_avoid_radius, next_cache_idx);
                let to_outside = projection_onto(&avoidance_next, (node_x, node_y));
                let mut direction_to_outer = (to_outside.0 - node_x, to_outside.1 - node_y);
                let mut dist2_to_outer = direction_to_outer.0 * direction_to_outer.0
                    + direction_to_outer.1 * direction_to_outer.1;
                // `max_move_distance2 * SQ(obj_layer_nr)`: the further from the
                // plate, the further a branch is allowed to jump outward.
                let layer_scale = (cache_idx as f32) * (cache_idx as f32);
                if line_cut.is_line_cut_by_contour(&layer_outlines, (node_x, node_y), to_outside)
                    || dist2_to_outer > max_move2 * layer_scale
                    || !is_inside_ex(&avoidance_next, node_x, node_y)
                {
                    // STUDIO-4252 retries the escape against **collision**,
                    // not avoidance: avoidance is the accumulated no-go cone,
                    // and projecting onto it can be arbitrarily far away.
                    let collision_next = volumes.get_collision(next_avoid_radius, next_cache_idx);
                    let mut candidate = (node_x, node_y);
                    let (dilation, max_move_between_samples) = studio_4252_move_out_args(max_move);
                    let _ = move_out_expolys(
                        &collision_next,
                        &mut candidate,
                        dilation,
                        max_move_between_samples,
                    );
                    direction_to_outer = (candidate.0 - node_x, candidate.1 - node_y);
                    dist2_to_outer = direction_to_outer.0 * direction_to_outer.0
                        + direction_to_outer.1 * direction_to_outer.1;
                    if dist2_to_outer <= f32::EPSILON {
                        direction_to_outer = (0.0, 0.0);
                    }
                }

                // The step is ALWAYS full length.
                let mut movement = style_movement_for(
                    self.tree_support_style,
                    direction_to_outer,
                    move_to_neighbor_center,
                    max_move,
                );
                // A sharp tail near its tip follows the painted skin normal
                // instead — that is how canonical keeps a thin spike plumb.
                if is_sharp_tail && dist_mm_to_top < SHARP_TAIL_SKIN_FOLLOW_MM {
                    movement = normal_to_length(
                        (units_to_mm(skin_direction.x), units_to_mm(skin_direction.y)),
                        max_move,
                    );
                }
                let next_x = node_x + movement.0;
                let next_y = node_y + movement.1;

                // LOCKED: move-pass recompute tests RAW outlines forever.
                // Canonical deliberately differs from contact seeding and the
                // branch-A merge here, which test collision(0) instead.
                let to_buildplate =
                    move_pass_to_buildplate(volumes.outlines_at(next_cache_idx), (next_x, next_y));

                // The node one layer down is a *new* arena node whose `parent`
                // points back up, so later steps can walk the column in either
                // direction. Canonical `create_node(..., parent = p_node)` plus
                // `p_node->child = next_node`.
                let mut next_radius = inherited_radius;
                // STUDIO-7883: a branch may not grow wider than its clearance
                // to the model, and may never shrink below its parent.
                let collision_here = volumes.get_collision(0.0, next_cache_idx);
                if !collision_here.is_empty() {
                    let projected = projection_onto(&collision_here, (next_x, next_y));
                    let dist_to_outer = ((projected.0 - next_x) * (projected.0 - next_x)
                        + (projected.1 - next_y) * (projected.1 - next_y))
                        .sqrt();
                    next_radius = radius.max(inherited_radius.min(dist_to_outer));
                }

                let next_id = arena.create_node(
                    Point2::from_mm(next_x, next_y),
                    next_distance_to_top,
                    layer_rev.saturating_sub(1),
                    next_roof_layers_below,
                    to_buildplate,
                    Some(id),
                    next_print_z,
                    next_layer_height,
                    next_dist_mm_to_top,
                    next_radius,
                );
                arena[next_id].movement = Point2::from_mm(movement.0, movement.1);
                arena[next_id].max_move_dist = max_move;
                arena[next_id].is_sharp_tail = is_sharp_tail;
                arena[next_id].demand_ids = demand_ids;
                next_nodes.push(next_id);
            }

            // Record what this layer committed so the emit pass can replay it
            // after F-14 pruning has run. Canonical runs `drop_nodes` to
            // completion over every layer and only then calls `draw_circles`;
            // pruning walks *up* the parent chain, so a single interleaved
            // pass could not un-emit an upper layer it had already written.
            layer_records.push(LayerRecord {
                layer_rev,
                // Canonical `draw_circles` iterates every node left in
                // `contact_nodes`, including merge-invalid nodes. Only the
                // later unsupported-branch prune marks nodes `is_processed`
                // and removes them before drawing.
                active: active_nodes.clone(),
                edges: live_edges
                    .iter()
                    .map(|(a, b)| (active_nodes[*a], active_nodes[*b]))
                    .collect(),
            });

            // Canonical branch-A merged nodes were already created on the
            // layer below and must not be moved again this layer.
            next_nodes.extend(collapsed_into_next);

            active_nodes = next_nodes;
        }

        // ── F-14: drain `unsupported_branch_leaves` ──────────────────────
        //
        // Canonical `drop_nodes` collects every branch leaf it decided cannot
        // reach a legal footing, then walks each one *up* its parent chain
        // marking `is_processed`, re-linking the neighbours it passes, and
        // enqueuing any node that had merged into it. The whole column is then
        // erased from every layer. Nothing of this existed before packet 224
        // step 5: `to_buildplate` was decided once at contact creation and a
        // node that could not descend was simply dropped where it stood,
        // leaving an orphaned stub in the layers above it.
        while let Some(leaf) = unsupported_branch_leaves.pop_front() {
            let mut cursor = Some(leaf);
            while let Some(i_node) = cursor {
                if arena[i_node].is_processed {
                    break;
                }
                arena[i_node].is_processed = true;
                arena[i_node].valid = false;
                let parent = arena[i_node].parent;
                let child = arena[i_node].child;
                if let Some(c) = child {
                    if arena[c].parent == Some(i_node) {
                        arena[c].parent = parent;
                    }
                    arena[c].parents.retain(|p| *p != i_node);
                    // Canonical `append(i_node->child->parents, i_node->parents)`.
                    for p in arena[i_node].parents.clone() {
                        if !arena[c].parents.contains(&p) {
                            arena[c].parents.push(p);
                        }
                    }
                }
                for p in arena[i_node].parents.clone() {
                    if arena[p].child == Some(i_node) {
                        arena[p].child = child;
                    }
                }
                if let Some(p) = parent {
                    if arena[p].child == Some(i_node) {
                        arena[p].child = child;
                    }
                }
                for merged in arena[i_node].merged_neighbours.clone() {
                    if !arena[merged].is_processed {
                        unsupported_branch_leaves.push_back(merged);
                    }
                }
                cursor = parent;
            }
        }

        // Canonical `erase_if(is_processed)` across ALL layers, applied here as
        // a filter on the recorded per-layer node sets.
        for record in &mut layer_records {
            record.active.retain(|id| !arena[*id].is_processed);
            record
                .edges
                .retain(|(a, b)| !arena[*a].is_processed && !arena[*b].is_processed);
        }

        // ── Canonical `smooth_nodes` (F-33) ──────────────────────────────
        //
        // Canonical calls this unconditionally, here: after `drop_nodes` has
        // finished every layer and its `unsupported_branch_leaves` erase has
        // run, and before `draw_circles`. It is the only producer of the final
        // per-node `movement` that the ellipse matrix below consumes. Every
        // collision gate in the emit pass runs *after* this, so a smoothed
        // position is still validated against model occupancy before it is
        // allowed to print.
        smooth_nodes(&mut arena, &layer_records, self.support_line_width_mm);

        // Canonical `draw_circles`' `CIRCLE_RESOLUTION`: a full 100-gon per
        // node per layer is unaffordable once the model carries hundreds of
        // branches per layer, so canonical degenerates the cross-section to a
        // quad aligned with the contact-set line fit.
        let circle_resolution = if contact_stats.avg_node_per_layer > COARSE_CIRCLE_NODE_THRESHOLD {
            CIRCLE_RESOLUTION_COARSE
        } else {
            CIRCLE_RESOLUTION_FINE
        };
        let branch_radius_units = mm_to_units(branch_radius).max(1) as f64;
        // Canonical: `angle = i / CIRCLE_RESOLUTION * TAU + M_PI_4 + nodes_angle`
        // for the square path, plain `i / CIRCLE_RESOLUTION * TAU` otherwise.
        // The `M_PI_4` puts the quad's *edges*, not its corners, across the
        // branch direction.
        let base_circle = branch_circle(
            circle_resolution,
            branch_radius_units,
            if circle_resolution == CIRCLE_RESOLUTION_COARSE {
                std::f32::consts::FRAC_PI_4 + contact_stats.nodes_angle
            } else {
                0.0
            },
        );
        // ── Emit pass (canonical `draw_circles`) ─────────────────────────
        for record in &layer_records {
            let layer_rev = record.layer_rev;
            let active_nodes: Vec<NodeId> = record.active.clone();
            if active_nodes.is_empty() {
                continue;
            }
            let current_global_layer_index = layer_plan.layers[layer_rev].global_layer_index;
            let cache_idx = current_global_layer_index as usize;
            let z_current = layer_plan.layers[layer_rev].z;
            let effective_height = layer_plan.layers[layer_rev].effective_layer_height;
            let index_of: std::collections::HashMap<NodeId, usize> = active_nodes
                .iter()
                .enumerate()
                .map(|(i, id)| (*id, i))
                .collect();
            let mst_edges: Vec<(usize, usize)> = record
                .edges
                .iter()
                .filter_map(|(a, b)| Some((*index_of.get(a)?, *index_of.get(b)?)))
                .collect();

            // Record the committed edges as branch segments (mm-space) on
            // this layer. Points sit at this layer's Z.
            // Canonical `get_collision` / `get_avoidance`. Collision carries
            // `m_xy_distance` (F-16: it used to carry no inflation at all); the
            // node's own tapered radius is folded into the drawn footprint,
            // so the pair sums to canonical's `radius + m_xy_distance` keyed
            // on the per-node radius rather than a constant one.
            // The carve uses the radius-free bucket because the drawn region
            // already contains the branch radius. Emit rejection below instead
            // queries each node's radius-baked bucket with a point-in test.
            let collision_polys = volumes.get_collision(0.0, cache_idx);
            // Host analysis carries the exact per-layer occupancy used by the
            // closure gate. Prefer it for emission checks when present; the
            // support-outline cache remains the compatibility fallback.
            let model_collision: Vec<ExPolygon> = inflate_model_occupancy(
                &support_analysis
                    .model_occupancy
                    .iter()
                    .filter(|entry| {
                        entry.object_id == obj.object_id
                            && entry.global_support_layer_index == current_global_layer_index
                    })
                    .flat_map(|entry| entry.polygons.iter().cloned())
                    .collect::<Vec<_>>(),
                self.support_object_xy_distance,
            );
            let collision_polys: &[ExPolygon] = if model_collision.is_empty() {
                collision_polys.as_slice()
            } else {
                model_collision.as_slice()
            };
            // ── Drop only what the model swallows whole ───────────────────
            //
            // Canonical has two distinct rules and this module had collapsed
            // them into one that is stronger than either.
            //
            // 1. `drop_nodes` deletes a node only when "the branch falls
            //    completely inside a collision area (the entire branch would
            //    be removed by the X/Y offset)". `get_collision(r, l)` is
            //    `outlines ⊕ (r + m_xy_distance)`, so "the whole footprint of
            //    radius r lies inside that volume" is exactly that test.
            // 2. `draw_circles` never drops a node whose cross-section merely
            //    *touches* the model: it computes
            //    `avoid_object_remove_extra_small_parts(circle, collision)` —
            //    a difference that keeps the largest surviving part — so a
            //    branch running alongside a wall still prints the sliver that
            //    clears it. `build_roles` performs that same difference here.
            //
            // Until this fix the emit pass rejected a node (and an MST edge)
            // on any *overlap* between the node's full tapered radius and the
            // model, which is strictly stronger than both. On real geometry it
            // is catastrophic, because canonical expects nodes to sit inside
            // collision: `move_out_expolys` clamps an over-budget push-out to
            // `pt_max`, which may leave a branch beside a wall intersecting
            // collision and relying on the carve. Measured on
            // `resources/regression_wedge.stl` before the fix: 68 of 72 nodes
            // and 69 of 70 MST edges rejected on layer 99, every node lost on
            // layers 145-151, and ten layers emitted role areas with an EMPTY
            // skeleton. The synthetic fixtures never caught it because they
            // run with an empty `model_occupancy`.
            //
            // The drop gate reads the radius-bucketed ladder (rule 1); the
            // carve in `build_roles` reads `collision_polys` (rule 2).
            let swallowed_by_collision = |x: f32, y: f32, radius: f32| -> bool {
                let gate = volumes.get_collision(radius, cache_idx);
                !gate.is_empty() && point_inside_collision_volume(&gate, x, y)
            };
            let node_swallowed = |x: f32, y: f32, radius: f32| -> bool {
                let gate = volumes.get_collision(radius, cache_idx);
                !gate.is_empty() && point_inside_collision_volume(&gate, x, y)
            };

            // Emit branch segments with radius tapering (Step 5 AC-2)
            let mut branch_segments: Vec<Vec<Point3WithWidth>> = Vec::new();
            let mut interface_segments: Vec<Vec<Point3WithWidth>> = Vec::new();
            let mut base_segments: Vec<Vec<Point3WithWidth>> = Vec::new();
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
            let base_n = self.num_top_base_interface_layers as usize;
            // `-1` mirrors the top interface count, matching canonical's
            // `number_of_support_interface_bottom_layers` fallback.
            let bottom_n = if self.support_interface_bottom_layers < 0 {
                top_n
            } else {
                self.support_interface_bottom_layers.max(0) as u32
            };
            let node_roles: Vec<InterfaceRole> = active_nodes
                .iter()
                .map(|id| {
                    let node = &arena[*id];
                    // Floor: the branch lands on the model rather than the plate.
                    // The lookup is by *global* layer index; it previously mixed
                    // in `layer_rev`, the reverse loop counter, and indexed the
                    // collision cache with it.
                    let is_floor = bottom_n > 0
                        && !self.support_on_build_plate_only
                        && (1..=bottom_n).any(|k| {
                            cache_idx.checked_sub(k as usize).is_some_and(|below| {
                                point_in_any_expoly(volumes.outlines_at(below), node.x(), node.y())
                            })
                        });
                    // Roof: canonical `node->support_roof_layers_below > 0`,
                    // the per-node counter seeded at contact creation and
                    // decremented once per descendant (F-1). The old
                    // `roof_band_layers_emitted` object-wide counter is gone:
                    // it starved every overhang after the first of interface.
                    let is_roof = top_n > 0 && node.is_roof();
                    let mut below_roof = 0usize;
                    let mut reached_roof = false;
                    let mut ancestor = node.parent;
                    while let Some(parent_id) = ancestor {
                        below_roof += 1;
                        if arena[parent_id].is_roof() {
                            reached_roof = true;
                            break;
                        }
                        ancestor = arena[parent_id].parent;
                    }
                    if is_floor {
                        InterfaceRole::Floor
                    } else if is_roof {
                        InterfaceRole::Roof
                    } else if reached_roof && below_roof <= base_n {
                        InterfaceRole::Base
                    } else {
                        InterfaceRole::Body
                    }
                })
                .collect();
            // ── Canonical `draw_circles`: one ellipse per node per layer ──
            //
            // Canonical draws every surviving node's own cross-section into
            // `base_areas` / `roof_areas`, oriented by the `movement` the
            // `smooth_nodes` pass above just produced (falling back to the
            // node's `skin_direction`, which is the direction canonical gives
            // a sharp tail that never moved). Until packet 224 step 6 this
            // module drew nothing per node at all — only the swept capsules
            // between MST endpoints — so an isolated or terminal node
            // contributed no area and a leaning branch had no elongation.
            let mut branch_areas: Vec<ExPolygon> = Vec::new();
            let mut interface_areas: Vec<ExPolygon> = Vec::new();
            let mut base_areas: Vec<ExPolygon> = Vec::new();
            let mut floor_areas: Vec<ExPolygon> = Vec::new();
            let mut layer_needs_extra_wall = false;
            for (i, id) in active_nodes.iter().enumerate() {
                let node = &arena[*id];
                // The F-34 virtual top-Z-gap node draws into `roof_gap_areas`,
                // which is never extruded.
                if node.is_virtual_gap() {
                    continue;
                }
                let radius = node.radius;
                // Same gate the contact-tip path applies: a node's own drawn
                // cross-section may never sit inside model occupancy,
                // whatever role it carries. (The MST-edge gate exempts
                // *interface endpoints* because the edge's other endpoint is
                // what reaches the model; a node's own ellipse has no such
                // partner.)
                let direction = if node.movement.x != 0 || node.movement.y != 0 {
                    node.movement
                } else {
                    node.skin_direction
                };
                let Some(ellipse) = node_ellipse(
                    &base_circle,
                    node.position,
                    (radius / branch_radius) as f64,
                    direction,
                    branch_radius_units,
                    circle_resolution == CIRCLE_RESOLUTION_COARSE,
                ) else {
                    continue;
                };
                // Canonical `drop_nodes`: only a cross-section the collision
                // volume swallows whole is lost.
                if swallowed_by_collision(node.x(), node.y(), radius) {
                    continue;
                }
                layer_needs_extra_wall |= node.need_extra_wall;
                match node_roles[i] {
                    InterfaceRole::Body => branch_areas.push(ellipse),
                    InterfaceRole::Roof => interface_areas.push(ellipse),
                    InterfaceRole::Base => base_areas.push(ellipse),
                    InterfaceRole::Floor => floor_areas.push(ellipse),
                }
            }

            let mut origin_contacts_emitted = vec![false; active_nodes.len()];
            let mut mst_emitted = vec![false; active_nodes.len()];
            for (a_idx, b_idx) in &mst_edges {
                let na = &arena[active_nodes[*a_idx]];
                let nb = &arena[active_nodes[*b_idx]];
                // The F-34 virtual top-Z-gap node is propagated but never
                // extruded: canonical `draw_circles` sends
                // `distance_to_top < 0 && !is_sharp_tail` into
                // `roof_gap_areas`, which `generate_toolpaths` never fills.
                // Sharp tails are exempt — that is how canonical gives them a
                // zero contact distance.
                if na.is_virtual_gap() || nb.is_virtual_gap() {
                    continue;
                }

                let radius_a = na.radius;
                let radius_b = nb.radius;

                // The swept capsule follows the same canonical carve rule as
                // a node's own cross-section: `build_roles` differences it
                // against this collision set, so only a capsule the model
                // swallows entirely has nothing left to print. Rejecting on
                // mere *intersection* — which is what this gate used to do,
                // plus a per-endpoint disc test — discarded almost every edge
                // on real geometry (69 of 70 on wedge layer 99).
                let segment_swallowed = swallowed_by_collision(na.x(), na.y(), radius_a)
                    && swallowed_by_collision(nb.x(), nb.y(), radius_b);
                if segment_swallowed {
                    if self.support_interface_top_layers > 0 {
                        let _ = output.push_diagnostic(Diagnostic {
                            severity: DiagnosticSeverity::Warn,
                            code: 1002,
                            layer: Some(current_global_layer_index as i32),
                            object_id: Some(obj.object_id.clone()),
                            message: "node-clamped-out".into(),
                        });
                        continue;
                    }
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
                // Only an edge that actually contributes geometry counts as
                // emitted. Setting this before the rejection above left a node
                // whose sole edge was refused with no representation at all —
                // the degenerate-node fallback below skipped it — so the layer
                // produced role areas with an EMPTY skeleton.
                mst_emitted[*a_idx] = true;
                mst_emitted[*b_idx] = true;

                let dist_a_mm = na.distance_to_top.max(0) as f32 * effective_height;
                let dist_b_mm = nb.distance_to_top.max(0) as f32 * effective_height;
                InterfaceRole::target_for_edge(
                    node_roles[*a_idx],
                    node_roles[*b_idx],
                    &mut branch_segments,
                    &mut interface_segments,
                    &mut base_segments,
                    &mut floor_segments,
                )
                .push(vec![
                    Point3WithWidth {
                        x: na.x(),
                        y: na.y(),
                        z: z_current,
                        width: radius_a * 2.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: dist_a_mm,
                        overhang_distance_mm: None,
                    },
                    Point3WithWidth {
                        x: nb.x(),
                        y: nb.y(),
                        z: z_current,
                        width: radius_b * 2.0,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: dist_b_mm,
                        overhang_distance_mm: None,
                    },
                ]);
                if na.distance_to_top <= 0 {
                    origin_contacts_emitted[*a_idx] = true;
                }
                if nb.distance_to_top <= 0 {
                    origin_contacts_emitted[*b_idx] = true;
                }
            }

            // A fresh contact is the tip of a support column and must be
            // represented on its origin layer even when it has no surviving
            // MST edge. This is intentionally limited to dist_to_top == 0;
            // propagated nodes remain subject to collision exclusion below.
            for (i, id) in active_nodes.iter().enumerate() {
                let node = &arena[*id];
                // A sharp-tail contact is a tip on its own layer even though
                // its `distance_to_top` is negative (see `is_virtual_gap`).
                if node.distance_to_top > 0 || node.is_virtual_gap() || origin_contacts_emitted[i] {
                    continue;
                }
                // A contact tip used to be emitted with `width = 0.0` and was
                // then dropped by `structural_body_regions`, so the layer that
                // meets the overhang produced no printable geometry. It now
                // carries the tapered radius like any other node.
                let radius = node.radius;
                if node_swallowed(node.x(), node.y(), radius) {
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
                let (contact_x, contact_y) = node.xy();
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
                    &mut base_segments,
                    &mut floor_segments,
                )
                .push(vec![point, point]);
            }

            // A surviving lone propagated node (dist_to_top > 0) with no surviving
            // MST edge still reaches the buildplate and must be emitted as a
            // degenerate current-layer segment (OrcaSlicer draw_circles parity).
            for (i, id) in active_nodes.iter().enumerate() {
                let node = &arena[*id];
                if mst_emitted[i] || node.distance_to_top <= 0 {
                    continue;
                }
                {
                    let radius = node.radius;
                    if node_swallowed(node.x(), node.y(), radius) {
                        continue;
                    }
                    let width = radius * 2.0;
                    let dist_mm = node.distance_to_top as f32 * effective_height;
                    let point = Point3WithWidth {
                        x: node.x(),
                        y: node.y(),
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
                        &mut base_segments,
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
                || !base_segments.is_empty()
                || !floor_segments.is_empty()
                || !branch_areas.is_empty()
                || !interface_areas.is_empty()
                || !floor_areas.is_empty()
            {
                // Find all regions for this (layer, object) pair.
                let mut regions_for_this: Vec<String> = region_segmentation
                    .entries
                    .iter()
                    .filter(|e| {
                        e.object_id == obj.object_id && e.layer_index == current_global_layer_index
                    })
                    .flat_map(|e| e.region_ids.iter().cloned())
                    .collect();
                regions_for_this.extend(
                    support_analysis
                        .family_assignments
                        .iter()
                        .filter(|assignment| {
                            assignment.object_id == obj.object_id
                                && canonical_support_family_alias(Some(&assignment.family_id))
                                    == "tree"
                        })
                        .map(|assignment| assignment.region_id.clone()),
                );
                regions_for_this.sort();
                regions_for_this.dedup();
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
                    let model_occupancy: Vec<ExPolygon> = inflate_model_occupancy(
                        &support_analysis
                            .model_occupancy
                            .iter()
                            .filter(|entry| {
                                entry.object_id == obj.object_id
                                    && entry.global_support_layer_index
                                        == current_global_layer_index
                                    && entry.region_id == *region_id
                            })
                            .flat_map(|entry| entry.polygons.iter().cloned())
                            .collect::<Vec<_>>(),
                        self.support_object_xy_distance,
                    );
                    let role_collision = if model_occupancy.is_empty() {
                        collision_polys
                    } else {
                        model_occupancy.as_slice()
                    };
                    let mut roles = build_roles(
                        &branch_segments,
                        &interface_segments,
                        &base_segments,
                        &floor_segments,
                        &branch_areas,
                        &interface_areas,
                        &base_areas,
                        &floor_areas,
                        branch_radius,
                        role_collision,
                        contact_stats.avg_node_per_layer,
                        self.support_line_width_mm,
                    );
                    // The exact-Z occupancy contract is discharged entirely by
                    // `build_roles`' carve against `role_collision`, which now
                    // runs after simplification and is therefore final.
                    //
                    // This site used to re-check it with
                    // `role.regions.retain(|r| intersection(r,
                    // role_collision).is_empty())` — an ALL-OR-NOTHING
                    // rejection that discarded a whole connected region
                    // because one of its lobes touched the model. Canonical
                    // `draw_circles` never does that: it uses
                    // `avoid_object_remove_extra_small_parts(circle,
                    // get_collision(...))`, a difference that keeps what
                    // survives. Measured on `SupportTest.stl` (AC-1
                    // `fixture_invariants`): the tree column below the
                    // cantilever is ONE connected body — the wall-hugging
                    // nodes at x = 0, the MST capsules and the free branch at
                    // x ≈ 12 union into a single ExPolygon — so a
                    // simplification nudge of the wall-side contour threw the
                    // free branch away on every layer from 38 down to 0 and
                    // the plan terminated at z = 8.0 instead of the plate.
                    // Canonical Orca prints tree support down to Z0.2 on this
                    // fixture (`SupportTest_Tree_Orca.gcode`).
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
                            .flat_map(|id| arena[*id].demand_ids.iter().cloned())
                            .collect(),
                        body_ids: vec![format!(
                            "tree-body-{}-{}",
                            obj.object_id, current_global_layer_index
                        )],
                        anchor_layer_index: layer_rev as u32,
                        // SupportPlanIR stores physical Z in canonical slicer
                        // units (1 unit = 100 nm), not a WIT-specific scale.
                        anchor_z: mm_to_units(z_current),
                        roles,
                        skeleton: Some({
                            let skeleton_points: Vec<_> = branch_segments
                                .iter()
                                .chain(interface_segments.iter())
                                .chain(floor_segments.iter())
                                .flat_map(|segment| segment.iter())
                                .collect();
                            let points = skeleton_points
                                .iter()
                                .map(|point| slicer_ir::Point3 {
                                    x: point.x,
                                    y: point.y,
                                    z: point.z,
                                })
                                .collect::<Vec<_>>();
                            let wall_counts = skeleton_points
                                .iter()
                                .map(|point| {
                                    u32::from(active_nodes.iter().any(|id| {
                                        let node = &arena[*id];
                                        node.need_extra_wall
                                            && node.x() == point.x
                                            && node.y() == point.y
                                    }))
                                })
                                .collect::<Vec<_>>();
                            assert_eq!(wall_counts.len(), points.len());
                            slicer_ir::SupportPlanSkeleton {
                                points,
                                wall_counts,
                            }
                        }),
                        capabilities: vec!["tree-branch-skeleton".to_string()],
                        provenance: if layer_needs_extra_wall {
                            vec![
                                "support-planner".to_string(),
                                "tree-branch-extra-wall".to_string(),
                            ]
                        } else {
                            vec!["support-planner".to_string()]
                        },
                        decline_reason: None,
                    });
                }
            }
        }

        // Family assignments can cover regions absent from either the
        // segmentation projection or the candidate geometry path. Stamp each
        // object/layer from one deterministic successful entry, without
        // replacing a blocked candidate record.
        let mut templates: std::collections::BTreeMap<(String, i32), SupportPlanEntry> =
            std::collections::BTreeMap::new();
        let mut covered_regions: std::collections::BTreeSet<(String, i32, String)> =
            std::collections::BTreeSet::new();
        for entry in &entries_in_order {
            if entry.decline_reason.is_none() && entry.skeleton.is_some() {
                covered_regions.insert((
                    entry.object_id.clone(),
                    entry.global_layer_index,
                    entry.region_id.clone(),
                ));
                templates
                    .entry((entry.object_id.clone(), entry.global_layer_index))
                    .or_insert_with(|| entry.clone());
            }
        }
        for ((object_id, layer_index), template) in templates {
            let mut assigned_regions: Vec<String> = support_analysis
                .family_assignments
                .iter()
                .filter(|assignment| {
                    assignment.object_id == object_id
                        && canonical_support_family_alias(Some(&assignment.family_id)) == "tree"
                })
                .map(|assignment| assignment.region_id.clone())
                .collect();
            assigned_regions.sort();
            assigned_regions.dedup();
            for region_id in assigned_regions {
                if covered_regions.insert((object_id.clone(), layer_index, region_id.clone())) {
                    let mut stamped = template.clone();
                    stamped.region_id = region_id;
                    entries_in_order.push(stamped);
                }
            }
        }

        // ── Packet 239c Step 2: off-grid intermediate support planes ─────
        //
        // `independent_support_layer_height` (default true, read in
        // `from_config`): when a configured support pitch is finer than the
        // gap between two adjacent support-bearing object planes, canonical
        // `generate_support_layers` interleaves intermediate rows between
        // them (`n_layers_extra = ceil((dist - EPSILON) /
        // max_support_layer_height)`, `step = dist / n_layers_extra`,
        // `print_z = bottom_z + k * step`). Each interpolated entry clones
        // the lower bracketing row's geometry — the column cross-section
        // changes by at most one step across the bracketed 0.2 mm gap — and
        // carries its own strictly-between plane as the declared
        // `anchor_z`. The bracketing grid rows themselves keep their exact
        // grid `anchor_z`, so nothing is deleted, duplicated, or inverted
        // and the disabled branch (and every default profile, via the 0.0
        // sentinel) stays byte-identical to pre-239c:
        // `sync_gap_with_object_layer`.
        let support_pitch_mm = if self.support_layer_height_mm > 0.0 {
            self.support_layer_height_mm as f64
        } else {
            // [FWD] 0.0 sentinel decision, option (b) of design.md §Open
            // Questions: the pitch defaults to the object's own
            // effective-layer pitch. No pair of adjacent object planes is
            // ever finer than that pitch, so the enabled branch degrades to
            // grid-exact on every default profile. Option (a) — deriving the
            // pitch from the interface line width, closer to canonical
            // `bottom_contact_layer`'s interface-flow height — was rejected
            // here because this tree does not model the interface flow
            // height, and option (b) is the safer default-profile behavior.
            // AC-1's fixture config sets an explicit pitch, so the feature
            // stays provable.
            layer_plan
                .layers
                .first()
                .map(|layer| layer.effective_layer_height as f64)
                .unwrap_or(0.0)
        };
        if self.independent_support_layer_height && self.support_layer_height_mm > 0.0 {
            // Support-bearing rows, ascending by global layer index. A row
            // prints iff it survived with real geometry (the same predicate
            // the template pass above uses); declined/empty rows print
            // nothing and therefore bracket nothing.
            let mut support_rows_by_object: std::collections::BTreeMap<
                (String, String),
                std::collections::BTreeMap<u32, Vec<&SupportPlanEntry>>,
            > = std::collections::BTreeMap::new();
            let mut support_rows_239c_by_object: std::collections::BTreeMap<
                String,
                std::collections::BTreeMap<u32, Vec<&SupportPlanEntry>>,
            > = std::collections::BTreeMap::new();
            for entry in entries_in_order
                .iter()
                .filter(|entry| entry.decline_reason.is_none() && entry.skeleton.is_some())
            {
                support_rows_by_object
                    .entry((entry.object_id.clone(), entry.region_id.clone()))
                    .or_default()
                    .entry(entry.anchor_layer_index)
                    .or_default()
                    .push(entry);
                support_rows_239c_by_object
                    .entry(entry.object_id.clone())
                    .or_default()
                    .entry(entry.anchor_layer_index)
                    .or_default()
                    .push(entry);
            }
            let z_of_layer = |index: u32| -> Option<f32> {
                layer_plan.layers.get(index as usize).map(|layer| layer.z)
            };
            let mut interpolated: Vec<SupportPlanEntry> = Vec::new();
            let mut coarse_candidates = Vec::<(f64, i32, SupportPlanEntry)>::new();
            let mut intermediate_plane_indices = std::collections::BTreeMap::<i64, i32>::new();
            let mut coarse_ranges = Vec::<(String, String, u32, u32)>::new();
            let mut coarse_used = false;
            let explicit_pitch = self.support_layer_height_mm > 0.0;
            let pitch_units = slicer_ir::mm_to_units(support_pitch_mm as f32);
            for ((object_id, region_id), support_rows_by_layer) in &support_rows_by_object {
                let demanded_layers: Vec<u32> = support_rows_by_layer.keys().copied().collect();
                for run in demanded_layers.chunk_by(|a, b| *b == *a + 1) {
                    if run.len() < 2 {
                        continue;
                    }
                    let mut interface_layers: Vec<u32> = run
                        .iter()
                        .copied()
                        .filter(|layer| {
                            support_rows_by_layer[layer].iter().any(|entry| {
                                entry.roles.iter().any(|role| {
                                    matches!(
                                        role.role,
                                        slicer_ir::SupportPlanRole::TopInterface
                                            | slicer_ir::SupportPlanRole::BaseInterface
                                            | slicer_ir::SupportPlanRole::BottomInterface
                                    )
                                })
                            })
                        })
                        .collect();
                    // Q1 counts distinct physical interface planes, not the
                    // number of object layers carrying an interface role.
                    interface_layers.sort_by_key(|layer| support_rows_by_layer[layer][0].anchor_z);
                    interface_layers.dedup_by_key(|layer| support_rows_by_layer[layer][0].anchor_z);
                    let mut brackets = if interface_layers.len() >= 2 {
                        interface_layers
                    } else {
                        let mut supplemented = interface_layers;
                        supplemented.push(run[0]);
                        supplemented.push(*run.last().unwrap());
                        supplemented
                    };
                    brackets.sort_by_key(|layer| support_rows_by_layer[layer][0].anchor_z);
                    brackets.dedup_by_key(|layer| support_rows_by_layer[layer][0].anchor_z);
                    for pair in brackets.windows(2) {
                        let below_layer = pair[0];
                        let above_layer = pair[1];
                        let covered: Vec<u32> = run
                            .iter()
                            .copied()
                            .filter(|layer| *layer >= below_layer && *layer <= above_layer)
                            .collect();
                        let local_support_gap = covered
                            .windows(2)
                            .filter_map(|layers| {
                                let below = support_rows_by_layer[&layers[0]][0].anchor_z;
                                let above = support_rows_by_layer[&layers[1]][0].anchor_z;
                                (above > below).then_some(above - below)
                            })
                            .max()
                            .unwrap_or(0);
                        let coarse = explicit_pitch
                            && local_support_gap > 0
                            && pitch_units >= local_support_gap;
                        if coarse {
                            coarse_used = true;
                            coarse_ranges.push((
                                object_id.clone(),
                                region_id.clone(),
                                below_layer,
                                above_layer,
                            ));
                            let (Some(below_z), Some(above_z)) =
                                (z_of_layer(below_layer), z_of_layer(above_layer))
                            else {
                                continue;
                            };
                            for candidate_z in
                                packet239d_coarse_planes(below_z, above_z, support_pitch_mm)
                            {
                                if candidate_z == above_z as f64 {
                                    continue;
                                }
                                for lower_entry in &support_rows_by_layer[&below_layer] {
                                    let source_global_layer_index = lower_entry.global_layer_index;
                                    let mut clone = (*lower_entry).clone();
                                    clone.roles = clone
                                        .roles
                                        .into_iter()
                                        .map(|role| slicer_ir::SupportPlanRoleRegion {
                                            role: slicer_ir::SupportPlanRole::SupportBody,
                                            regions: role.regions,
                                        })
                                        .collect();
                                    coarse_candidates.push((
                                        candidate_z,
                                        source_global_layer_index,
                                        clone,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            if coarse_used {
                // Coarse brackets replace only their covered row pairs. Every other
                // pair keeps the 239c candidate multiplicity and insertion order.
                for ((object_id, region_id), support_rows_by_layer) in &support_rows_by_object {
                    let demanded_layers: Vec<u32> = support_rows_by_layer.keys().copied().collect();
                    for run in demanded_layers.chunk_by(|a, b| *b == *a + 1) {
                        for pair in run.windows(2) {
                            let prev = pair[0];
                            let layer = pair[1];
                            if let (Some(below_z), Some(above_z)) =
                                (z_of_layer(prev), z_of_layer(layer))
                            {
                                for plane in packet239c_intermediate_planes(
                                    below_z,
                                    above_z,
                                    support_pitch_mm,
                                ) {
                                    for lower_entry in &support_rows_by_layer[&prev] {
                                        let belongs_to_coarse_range = coarse_ranges.iter().any(
                                            |(coarse_object, coarse_region, below, above)| {
                                                object_id == coarse_object
                                                    && region_id == coarse_region
                                                    && prev >= *below
                                                    && layer <= *above
                                            },
                                        );
                                        if belongs_to_coarse_range {
                                            continue;
                                        }
                                        let mut clone = (*lower_entry).clone();
                                        let plane_ordinal =
                                            i32::try_from(intermediate_plane_indices.len())
                                                .map_err(|_| {
                                                    ModuleError::fatal(
                                                        1,
                                                        "too many intermediate tree-support planes",
                                                    )
                                                })?;
                                        let next_index = i32::MIN
                                            .checked_add(plane_ordinal)
                                            .ok_or_else(|| {
                                                ModuleError::fatal(
                                                    1,
                                                    "too many intermediate tree-support planes",
                                                )
                                            })?;
                                        clone.global_layer_index = *intermediate_plane_indices
                                            .entry(plane)
                                            .or_insert(next_index);
                                        clone.anchor_layer_index = prev;
                                        clone.anchor_z = plane;
                                        interpolated.push(clone);
                                    }
                                }
                            }
                        }
                    }
                }

                let mut synthesized_seen =
                    std::collections::BTreeSet::<(i32, String, String, Vec<String>, i64)>::new();
                coarse_candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut first = 0;
                while first < coarse_candidates.len() {
                    let mut last = first;
                    while last + 1 < coarse_candidates.len()
                        && coarse_candidates[last + 1].0 - coarse_candidates[first].0
                            <= CANONICAL_EPSILON_MM as f64
                    {
                        last += 1;
                    }
                    let plane = slicer_ir::mm_to_units(
                        (0.5 * (coarse_candidates[first].0 + coarse_candidates[last].0)) as f32,
                    );
                    let plane_ordinal =
                        i32::try_from(intermediate_plane_indices.len()).map_err(|_| {
                            ModuleError::fatal(1, "too many intermediate tree-support planes")
                        })?;
                    let next_index = i32::MIN.checked_add(plane_ordinal).ok_or_else(|| {
                        ModuleError::fatal(1, "too many intermediate tree-support planes")
                    })?;
                    let global_layer_index = *intermediate_plane_indices
                        .entry(plane)
                        .or_insert(next_index);
                    let anchor_layer_index = layer_plan
                        .layers
                        .iter()
                        .enumerate()
                        .min_by_key(|(index, layer)| {
                            (plane.abs_diff(slicer_ir::mm_to_units(layer.z)), *index)
                        })
                        .map(|(index, _)| index as u32)
                        .unwrap_or(0);
                    for (_, source_global_layer_index, mut clone) in
                        coarse_candidates[first..=last].iter().cloned()
                    {
                        clone.global_layer_index = global_layer_index;
                        clone.anchor_layer_index = anchor_layer_index;
                        clone.anchor_z = plane;
                        if synthesized_seen.insert((
                            source_global_layer_index,
                            clone.object_id.clone(),
                            clone.region_id.clone(),
                            clone.body_ids.clone(),
                            clone.anchor_z,
                        )) {
                            interpolated.push(clone);
                        }
                    }
                    first = last + 1;
                }
                entries_in_order.retain(|entry| {
                    !coarse_ranges
                        .iter()
                        .any(|(object_id, region_id, below, above)| {
                            entry.object_id == *object_id
                                && entry.region_id == *region_id
                                && entry.anchor_layer_index > *below
                                && entry.anchor_layer_index < *above
                                && !entry.roles.iter().any(|role| {
                                    matches!(
                                        role.role,
                                        slicer_ir::SupportPlanRole::TopInterface
                                            | slicer_ir::SupportPlanRole::BaseInterface
                                            | slicer_ir::SupportPlanRole::BottomInterface
                                    )
                                })
                        })
                });
                entries_in_order.extend(interpolated);
                // A coarse-active object is emitted Z-first as one stack. A
                // stable sort keeps the 239c identity and multiplicity of
                // finer candidates, including their relative order at one Z.
                entries_in_order.sort_by_key(|entry| entry.anchor_z);
            } else {
                // Exact packet-239c path: object grouping, candidate identity,
                // multiplicity, and append order are intentionally unchanged.
                for (_, support_rows_by_layer) in support_rows_239c_by_object {
                    let mut previous_layer: Option<&u32> = None;
                    for layer in support_rows_by_layer.keys() {
                        if let Some(prev) = previous_layer {
                            if let (Some(below_z), Some(above_z)) =
                                (z_of_layer(*prev), z_of_layer(*layer))
                            {
                                for plane in packet239c_intermediate_planes(
                                    below_z,
                                    above_z,
                                    support_pitch_mm,
                                ) {
                                    if let Some(lower_entries) = support_rows_by_layer.get(prev) {
                                        for lower_entry in lower_entries {
                                            let mut clone = (*lower_entry).clone();
                                            let plane_ordinal =
                                                i32::try_from(intermediate_plane_indices.len())
                                                    .map_err(|_| {
                                                        ModuleError::fatal(
                                                    1,
                                                    "too many intermediate tree-support planes",
                                                )
                                                    })?;
                                            let next_index = i32::MIN
                                                .checked_add(plane_ordinal)
                                                .ok_or_else(|| {
                                                    ModuleError::fatal(
                                                        1,
                                                        "too many intermediate tree-support planes",
                                                    )
                                                })?;
                                            clone.global_layer_index = *intermediate_plane_indices
                                                .entry(plane)
                                                .or_insert(next_index);
                                            clone.anchor_layer_index = *prev;
                                            clone.anchor_z = plane;
                                            interpolated.push(clone);
                                        }
                                    }
                                }
                            }
                        }
                        previous_layer = Some(layer);
                    }
                }
                entries_in_order.extend(interpolated);
                // Preserve the pre-239c final identity gate on the finer-only
                // path. Coarse synthesis has its own source-aware key above;
                // applying that key here would change legacy identity and
                // multiplicity semantics.
                let mut seen = std::collections::BTreeSet::<(String, i32, String, i64)>::new();
                entries_in_order.retain(|entry| {
                    seen.insert((
                        entry.object_id.clone(),
                        entry.global_layer_index,
                        entry.region_id.clone(),
                        entry.anchor_z,
                    ))
                });
            }
        }

        // Smoothing happens *before* the emit pass (canonical `smooth_nodes`,
        // above), never here: the entry-level `smooth_branches` translates
        // already-validated role polygons, which can move a previously legal
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

/// Packet 239c (Step 2): canonical `generate_support_layers`
/// (`Support/SupportCommon.cpp`) intermediate-row stepping for one vertical
/// gap between two bracketing object planes.
///
/// Canonical rule (flag-independent there; gated here by
/// `independent_support_layer_height`):
/// `n_layers_extra = ceil((dist - EPSILON) / max_support_layer_height)`,
/// `step = dist / n_layers_extra`, `print_z = bottom_z + k * step`.
///
/// `below_z_mm` / `above_z_mm` are the two bracketing object planes
/// (ascending, mm); the plate at Z 0 brackets the bottom-most object row.
/// Returns the k = 1..n rows strictly between the brackets, in canonical
/// units, ascending. EPSILON is canonical's 1e-4 mm — exactly one canonical
/// unit. Insertion happens only when the configured pitch is finer than the
/// gap (`n >= 2`), so the bracketing grid planes themselves never move and
/// no plane is duplicated, deleted, or inverted. Deterministic: pure
/// function of the pair plus the pitch.
fn packet239c_intermediate_planes(below_z_mm: f32, above_z_mm: f32, pitch_mm: f64) -> Vec<i64> {
    const EPSILON_MM: f64 = 1e-4;
    let below_units = slicer_ir::mm_to_units(below_z_mm);
    let above_units = slicer_ir::mm_to_units(above_z_mm);
    if pitch_mm <= 0.0 || above_units <= below_units {
        return Vec::new();
    }
    let dist = (above_z_mm - below_z_mm) as f64;
    let n = ((dist - EPSILON_MM) / pitch_mm).ceil();
    if n < 2.0 {
        return Vec::new();
    }
    let step = dist / n;
    let n = n as i64;
    (1..n)
        .map(|k| slicer_ir::mm_to_units((below_z_mm as f64 + k as f64 * step) as f32))
        .filter(|plane| *plane > below_units && *plane < above_units)
        .collect()
}

/// Packet 239d coarse stack: include the aligned upper bracket so callers can
/// deduplicate it against the demanded interface row.
fn packet239d_coarse_planes(below_z_mm: f32, above_z_mm: f32, pitch_mm: f64) -> Vec<f64> {
    let below_units = slicer_ir::mm_to_units(below_z_mm);
    let above_units = slicer_ir::mm_to_units(above_z_mm);
    if pitch_mm <= 0.0 || above_units <= below_units {
        return Vec::new();
    }
    let dist = (above_z_mm - below_z_mm) as f64;
    let n = ((dist / pitch_mm).ceil() as i64).max(1);
    let step = dist / n as f64;
    (1..=n)
        .map(|k| {
            if k == n {
                above_z_mm as f64
            } else {
                below_z_mm as f64 + k as f64 * step
            }
        })
        .filter(|plane| {
            let units = slicer_ir::mm_to_units(*plane as f32);
            units > below_units && units <= above_units
        })
        .collect()
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

/// Bounding box of a polygon set, in scaled units. `None` when empty.
fn expolygons_bbox(polygons: &[ExPolygon]) -> Option<(i64, i64, i64, i64)> {
    let mut bbox: Option<(i64, i64, i64, i64)> = None;
    for polygon in polygons {
        for point in &polygon.contour.points {
            bbox = Some(match bbox {
                None => (point.x, point.x, point.y, point.y),
                Some((min_x, max_x, min_y, max_y)) => (
                    min_x.min(point.x),
                    max_x.max(point.x),
                    min_y.min(point.y),
                    max_y.max(point.y),
                ),
            });
        }
    }
    bbox
}

/// Signed shoelace area of a ring, in scaled units².
fn ring_area(ring: &Polygon) -> f64 {
    let points = &ring.points;
    if points.len() < 3 {
        return 0.0;
    }
    let mut twice = 0.0f64;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        twice += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
    }
    twice / 2.0
}

/// Canonical `area(const ExPolygon&)`: contour area minus hole areas, in
/// scaled units². Used for the F-1 `minimum_roof_area` test.
fn expolygon_area(polygon: &ExPolygon) -> f64 {
    let mut area = ring_area(&polygon.contour).abs();
    for hole in &polygon.holes {
        area -= ring_area(hole).abs();
    }
    area
}

/// Canonical `minimum_roof_area = SQ(scaled(1.0))` — one square millimetre,
/// expressed in this codebase's scaled units (1 unit = 100 nm).
fn minimum_roof_area() -> f64 {
    let one_mm = mm_to_units(1.0) as f64;
    one_mm * one_mm
}

/// Canonical `BoundingBox::radius()` — half the box diagonal, in mm.
fn bbox_radius_mm(bbox: (i64, i64, i64, i64)) -> f32 {
    let (min_x, max_x, min_y, max_y) = bbox;
    let dx = units_to_mm(max_x - min_x) as f64;
    let dy = units_to_mm(max_y - min_y) as f64;
    (0.5 * (dx * dx + dy * dy).sqrt()) as f32
}

/// Canonical rotated-bbox interior lattice (defect F-35).
///
/// `TreeSupport::generate_contact_points` builds `grid_points` **once per
/// object**, before any overhang is looked at:
///
/// ```text
/// rotated_dims = (size.x*cos + size.y*sin, size.x*sin + size.y*cos) / 2
/// for x in -rotated_dims.x .. rotated_dims.x step sample_step
///   for y in -rotated_dims.y .. rotated_dims.y step sample_step
///     pt = rotate(x, y, 22deg) + bounding_box_middle(object bbox)
///     if bounding_box.contains(pt) { keep }
/// ```
///
/// The span comes from the **rotated** dimensions, which is what lets a
/// rotated lattice point still land near a bbox corner. Until packet 224 this
/// module derived the index span from the unrotated bbox and rotated
/// afterwards, so the sampled set was the bbox *shrunk* by the rotation —
/// interior contacts near the corners were never generated, the exact failure
/// the function's own doc comment claimed to avoid.
///
/// `bbox_mm` is `(min_x, max_x, min_y, max_y)` in mm.
fn build_grid_points(bbox_mm: (f32, f32, f32, f32), sample_step: f32) -> Vec<(f32, f32)> {
    let (min_x, max_x, min_y, max_y) = bbox_mm;
    if sample_step <= 0.0 || max_x < min_x || max_y < min_y {
        return Vec::new();
    }
    let rotate_angle = 22.0f32 / 180.0 * std::f32::consts::PI;
    let (sin_angle, cos_angle) = (rotate_angle.sin(), rotate_angle.cos());
    let size_x = max_x - min_x;
    let size_y = max_y - min_y;
    let rotated_dim_x = (size_x * cos_angle + size_y * sin_angle) / 2.0;
    let rotated_dim_y = (size_x * sin_angle + size_y * cos_angle) / 2.0;
    let center = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);

    let mut grid = Vec::new();
    let mut x = -rotated_dim_x;
    while x < rotated_dim_x {
        let mut y = -rotated_dim_y;
        while y < rotated_dim_y {
            // `Point::rotate(cos, sin)` is the standard CCW rotation.
            let rx = x * cos_angle - y * sin_angle + center.0;
            let ry = x * sin_angle + y * cos_angle + center.1;
            if rx >= min_x && rx <= max_x && ry >= min_y && ry <= max_y {
                grid.push((rx, ry));
            }
            y += sample_step;
        }
        x += sample_step;
    }
    grid
}

/// One sampled contact position, carrying the per-overhang facts canonical
/// `insert_point` needs.
#[derive(Clone, Debug)]
struct ContactSample {
    /// Position in mm.
    x: f32,
    /// Position in mm.
    y: f32,
    /// Canonical per-overhang
    /// `clamp(unscale_(overhang_bounds.radius()), MIN_BRANCH_RADIUS, base_radius)`.
    radius: f32,
    /// Index into the overhang polygon slice this sample came from.
    overhang: usize,
    /// Canonical `contact_node->is_corner = true` on the corner stream.
    is_corner: bool,
}

/// Sample overhangs using canonical corner, arc, and rotated-interior streams.
///
/// `grid_points` is the per-object lattice from [`build_grid_points`]; pass
/// `None` for a mesh-less object, in which case the lattice is derived from
/// the overhang polygons' own bbox.
fn sample_contact_points(
    polygons: &[ExPolygon],
    grid_points: Option<&[(f32, f32)]>,
    point_spread: f32,
    base_radius: f32,
    max_bridge_length_mm: f32,
    is_sharp_tail: bool,
) -> Vec<ContactSample> {
    let mut result: Vec<ContactSample> = Vec::new();
    let mut buckets = std::collections::HashSet::new();
    let cell = mm_to_units(base_radius).max(1) + 1;
    let sample_step = point_spread.max(max_bridge_length_mm / 2.0);
    let owned_grid = match grid_points {
        Some(_) => None,
        None => expolygons_bbox(polygons).map(|(min_x, max_x, min_y, max_y)| {
            build_grid_points(
                (
                    units_to_mm(min_x),
                    units_to_mm(max_x),
                    units_to_mm(min_y),
                    units_to_mm(max_y),
                ),
                sample_step,
            )
        }),
    };
    let grid_points: &[(f32, f32)] = match (grid_points, owned_grid.as_ref()) {
        (Some(grid), _) => grid,
        (None, Some(grid)) => grid,
        (None, None) => &[],
    };

    let mut add = |sample: ContactSample, result: &mut Vec<ContactSample>| {
        let key = (
            mm_to_units(sample.x).div_euclid(cell),
            mm_to_units(sample.y).div_euclid(cell),
        );
        if buckets.insert(key) {
            result.push(sample);
        }
    };

    for (poly_idx, polygon) in polygons.iter().enumerate() {
        let Some(bbox) = expolygons_bbox(std::slice::from_ref(polygon)) else {
            continue;
        };
        // Canonical per-overhang radius: half the overhang bbox diagonal,
        // clamped into [MIN_BRANCH_RADIUS, base_radius].
        let radius =
            bbox_radius_mm(bbox).clamp(MIN_BRANCH_RADIUS, base_radius.max(MIN_BRANCH_RADIUS));
        let mk = |x: f32, y: f32, is_corner: bool| ContactSample {
            x,
            y,
            radius,
            overhang: poly_idx,
            is_corner,
        };
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
                add(
                    mk(units_to_mm(current.x), units_to_mm(current.y), true),
                    &mut result,
                );
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
                            mk(
                                units_to_mm((a.x as f32 + (b.x - a.x) as f32 * t) as i64),
                                units_to_mm((a.y as f32 + (b.y - a.y) as f32 * t) as i64),
                                false,
                            ),
                            &mut result,
                        );
                    }
                    distance += mm_to_units(point_spread) as f32;
                }
            }
        }

        // Canonical: "don't add inner supports for sharp tails".
        if is_sharp_tail || grid_points.is_empty() {
            continue;
        }
        // Canonical filters the shared per-object lattice per overhang:
        // the point must be inside the overhang bbox AND inside the overhang
        // eroded by the per-overhang radius.
        let (min_x, max_x, min_y, max_y) = bbox;
        let eroded = host::offset_polygons_with_miter_limit(
            std::slice::from_ref(polygon),
            -radius,
            OffsetJoinType::Miter,
            0.0,
            3.0,
        );
        if eroded.is_empty() {
            continue;
        }
        for &(x, y) in grid_points {
            let (ux, uy) = (mm_to_units(x), mm_to_units(y));
            if ux < min_x || ux > max_x || uy < min_y || uy > max_y {
                continue;
            }
            if point_in_any_expoly(&eroded, x, y) {
                add(mk(x, y, false), &mut result);
            }
        }
    }

    result
}

/// Per-object contact-generation constants, ported from the head of canonical
/// `TreeSupport::generate_contact_points`.
#[derive(Clone, Copy, Debug)]
struct ContactContext {
    /// Canonical `top_z_distance`, in mm, after
    /// `if (top_z_distance > EPSILON) top_z_distance = max(top_z_distance, min_layer_height)`.
    z_distance_top: f32,
    /// Canonical `gap_layers = z_distance_top == 0 ? 0 : 1`.
    gap_layers: i32,
    /// Canonical `support_roof_layers = config.support_interface_top_layers`.
    support_roof_layers: i32,
}

/// The per-overhang facts canonical `insert_point` closes over.
struct OverhangContext<'a> {
    /// Per-object constants.
    ctx: &'a ContactContext,
    /// The overhang `ExPolygon` this contact came from. Stored on the node so
    /// the step 7 `draw_circles` rewrite can draw it into `roof_gap_areas`.
    overhang: &'a ExPolygon,
    /// Canonical `add_interface = area(overhang) > minimum_roof_area && !is_sharp_tail`.
    add_interface: bool,
    /// Canonical `is_sharp_tail`.
    is_sharp_tail: bool,
}

impl OverhangContext<'_> {
    /// Canonical `size_t roof_layers = add_interface ? support_roof_layers : 0`.
    fn roof_layers(&self) -> i32 {
        if self.add_interface {
            self.ctx.support_roof_layers.max(0)
        } else {
            0
        }
    }
}

/// Port of canonical `insert_point` inside `TreeSupport::generate_contact_points`
/// (defects F-1 and F-34).
///
/// `overhang_layer_idx` is canonical's `layer_nr` — the layer whose overhang
/// demanded support. The node is created at `layer_nr - 1`, **always exactly
/// one layer below**, with `distance_to_top = -gap_layers`. When there is a
/// top-Z gap that makes the contact a *virtual* node: it is propagated like
/// any other node but is never extruded (canonical draws it into
/// `roof_gap_areas`, which `generate_toolpaths` never fills), so the printed
/// column starts one further layer down and the gap is exactly one layer.
///
/// The roof counter is **per node** (F-1): `support_roof_layers_below` is
/// seeded here from `add_interface ? support_roof_layers : 0`. The old
/// per-object `roof_band_layers_emitted` counter is gone — it incremented on
/// every interface-emitting layer of the whole object, so after the first
/// `support_interface_top_layers` such layers a second, lower overhang on the
/// same object received no top interface at all.
#[allow(clippy::too_many_arguments)]
fn insert_contact_point(
    arena: &mut NodeArena,
    contacts: &mut [Vec<NodeId>],
    layer_plan: &LayerPlanView,
    planner: &SupportPlanner,
    volumes: &TreeVolumes,
    dropped: &mut std::collections::BTreeMap<u32, usize>,
    overhang_layer_idx: usize,
    sample: (f32, f32),
    radius: f32,
    is_corner: bool,
    oc: &OverhangContext<'_>,
    demand_id: String,
) -> Option<NodeId> {
    // Canonical iterates `layer_nr` from 1, so `layer_nr - 1` is always valid.
    if overhang_layer_idx == 0 || overhang_layer_idx >= layer_plan.layers.len() {
        return None;
    }
    let target_idx = overhang_layer_idx - 1;
    let overhang_layer = &layer_plan.layers[overhang_layer_idx];
    // Canonical `m_object->get_layer(layer_nr)->bottom_z()`.
    let bottom_z = overhang_layer.z - overhang_layer.effective_layer_height;
    let (x, y) = sample;
    let global_layer = layer_plan.layers[target_idx].global_layer_index;
    let collision = volumes.get_collision(0.0, target_idx);
    let to_buildplate = contact_seed_to_buildplate(&collision, (x, y));
    if contacts[target_idx].len() >= planner.max_branches_per_layer {
        *dropped.entry(global_layer).or_insert(0) += 1;
        return None;
    }
    let id = arena.create_node(
        Point2::from_mm(x, y),
        -oc.ctx.gap_layers,
        target_idx,
        oc.roof_layers(),
        to_buildplate,
        None,
        bottom_z,
        oc.ctx.z_distance_top,
        0.0,
        radius,
    );
    arena[id].overhang = oc.overhang.clone();
    if planner.tree_support_style == TreeSupportStyle::Hybrid
        && expolygon_area(oc.overhang) > minimum_roof_area()
    {
        arena[id].type_ = TreeNodeType::Polygon;
    }
    arena[id].is_sharp_tail = oc.is_sharp_tail;
    arena[id].is_corner = is_corner;
    arena[id].demand_ids = vec![demand_id];
    contacts[target_idx].push(id);
    Some(id)
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

/// Entry-level three-point Laplacian smoother over each
/// `(object_id, region_id)` column of structural skeleton rows. Endpoints are
/// held fixed.
///
/// **Superseded in production by [`smooth_nodes`]** (packet 224 step 6,
/// finding F-33). Canonical smooths the *node graph* between `drop_nodes` and
/// `draw_circles`, where the branch topology (`parent` / `child` / `parents`)
/// is still available and every downstream collision gate still runs; this
/// function smooths already-emitted `SupportPlanEntry` rows, which cannot see
/// the topology and would translate validated geometry back into model
/// occupancy. It is retained as a standalone helper with its own contract
/// tests (`tests/smooth_nodes_tdd.rs`) and is deliberately not called from the
/// planner pipeline.
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

/// Contact seeding classification against canonical `get_collision(0, layer)`.
#[doc(hidden)]
pub fn contact_seed_to_buildplate(collision: &[ExPolygon], position: (f32, f32)) -> bool {
    !is_inside_ex(collision, position.0, position.1)
}

/// Branch-A merged-node classification against canonical collision(0).
#[doc(hidden)]
pub fn branch_a_to_buildplate(collision: &[ExPolygon], position: (f32, f32)) -> bool {
    !is_inside_ex(collision, position.0, position.1)
}

/// F-14's locked canonical exception: classify against raw layer outlines.
#[doc(hidden)]
pub fn move_pass_to_buildplate(raw_outlines: &[ExPolygon], position: (f32, f32)) -> bool {
    !is_inside_ex(raw_outlines, position.0, position.1)
}

/// Canonical branch-A inheritance from the selected parent node.
#[doc(hidden)]
pub fn branch_a_roof_counter(parent_counter: i32, parent_distance_to_top: i32) -> i32 {
    parent_counter - i32::from(parent_distance_to_top >= 0)
}

/// Canonical same-position `insert_dropped_node` roof-counter merge.
#[doc(hidden)]
pub fn insert_dropped_node_roof_counter(existing: i32, incoming: i32) -> i32 {
    existing.max(incoming)
}

/// Point-in predicate used by emit gates after `get_collision(radius, layer)`
/// has baked the querying node's radius into the supplied volume.
#[doc(hidden)]
pub fn point_inside_collision_volume(polygons: &[ExPolygon], x: f32, y: f32) -> bool {
    is_inside_ex(polygons, x, y)
}

/// Test-only predicate retained for integration fixtures that exercise the
/// retired F-13 disc-inflation behavior. Production emit gates query
/// `get_collision(radius, layer)` and use point-in tests instead.
#[doc(hidden)]
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
fn prim_mst(nodes: &[(f32, f32)]) -> Vec<(usize, usize, f32)> {
    let n = nodes.len();
    if n < 2 {
        return Vec::new();
    }
    let mut in_tree = vec![false; n];
    let mut min_dist = vec![f32::INFINITY; n];
    let mut parent: Vec<Option<usize>> = vec![None; n];

    in_tree[0] = true;
    for i in 1..n {
        let d = euclidean_distance(nodes[0], nodes[i]);
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
                let d = euclidean_distance(nodes[next], nodes[i]);
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

fn euclidean_distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

/// Canonical `drop_nodes`' `sum_direction` accumulator: the sum over MST
/// neighbours of `(neighbour - node) * (1 / dist2)`, in mm.
///
/// Only the *direction* of this vector is canonical — `drop_nodes` feeds it to
/// `normal(move_to_neighbor_center, scale_(get_max_move_dist(&node)))`, which
/// rescales it to the full per-layer move budget. Until packet 224 step 5 this
/// module instead computed a 1/d^2-weighted *mean of neighbour positions* and
/// stepped toward it with a fractional cap. The two agree on direction (the
/// mean minus the node position is this sum divided by the positive weight
/// total), which is why the weighting survived the re-port and the capping did
/// not.
///
/// Reference: canonical `TreeSupport::drop_nodes` (`TreeSupport.cpp`), the
/// `move_to_neighbor_center` accumulation.
pub fn neighbour_direction_sum(node: (f32, f32), neighbour_positions: &[(f32, f32)]) -> (f32, f32) {
    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    for &(nx, ny) in neighbour_positions {
        let dx = (nx - node.0) as f64;
        let dy = (ny - node.1) as f64;
        let dist2 = dx * dx + dy * dy;
        if dist2 <= 1e-12 {
            continue;
        }
        sum_x += dx / dist2;
        sum_y += dy / dist2;
    }
    (sum_x as f32, sum_y as f32)
}

fn style_neighbour_direction_for(
    style: TreeSupportStyle,
    node: (f32, f32),
    neighbour_positions: &[(f32, f32)],
) -> (f32, f32) {
    if style == TreeSupportStyle::Strong {
        neighbour_positions
            .iter()
            .fold((0.0, 0.0), |sum, neighbour| {
                (sum.0 + neighbour.0 - node.0, sum.1 + neighbour.1 - node.1)
            })
    } else {
        neighbour_direction_sum(node, neighbour_positions)
    }
}

/// Canonical `drop_nodes`' final movement chain: `normal(direction_to_outer,
/// max_move)` when `dist2_to_outer > 0`, else `normal(move_to_neighbor_center,
/// max_move)` — for EVERY style. Canonical's preceding `is_strong`
/// composition block is dead code: its result is unconditionally overwritten
/// by this chain (and its dot gate reads an uninitialized `movement`). This
/// port used to implement that composition as live, which routed Strong
/// branches along `outer + center` where canonical takes `outer` alone.
fn style_movement_for(
    _style: TreeSupportStyle,
    direction_to_outer: (f32, f32),
    move_to_neighbor_center: (f32, f32),
    max_move: f32,
) -> (f32, f32) {
    let outer_len2 =
        direction_to_outer.0 * direction_to_outer.0 + direction_to_outer.1 * direction_to_outer.1;
    if outer_len2 > 0.0 {
        normal_to_length(direction_to_outer, max_move)
    } else {
        normal_to_length(move_to_neighbor_center, max_move)
    }
}

/// Returns the tree behavior selected by `support_style` for contract tests.
pub fn resolve_tree_style(config: &ConfigView) -> &'static str {
    match TreeSupportStyle::from_config(config) {
        TreeSupportStyle::Default => "default",
        TreeSupportStyle::Slim => "tree_slim",
        TreeSupportStyle::Strong => "tree_strong",
        TreeSupportStyle::Hybrid => "tree_hybrid",
    }
}

/// Computes the style-specific neighbour accumulator used by `drop_nodes`.
pub fn style_neighbour_direction(
    style: &str,
    node: (f32, f32),
    neighbour_positions: &[(f32, f32)],
) -> (f32, f32) {
    style_neighbour_direction_for(tree_style_from_str(style), node, neighbour_positions)
}

/// Composes outward and neighbour movement according to the selected style.
pub fn style_movement(
    style: &str,
    direction_to_outer: (f32, f32),
    move_to_neighbor_center: (f32, f32),
    max_move: f32,
) -> (f32, f32) {
    style_movement_for(
        tree_style_from_str(style),
        direction_to_outer,
        move_to_neighbor_center,
        max_move,
    )
}

/// Reports whether a contact with the given area is a hybrid polygon contact.
pub fn hybrid_contact_is_polygon(style: &str, overhang_area_mm2: f64) -> bool {
    tree_style_from_str(style) == TreeSupportStyle::Hybrid && overhang_area_mm2 > 1.0
}

fn tree_style_from_str(style: &str) -> TreeSupportStyle {
    match style {
        "tree_slim" => TreeSupportStyle::Slim,
        "tree_strong" => TreeSupportStyle::Strong,
        "tree_hybrid" => TreeSupportStyle::Hybrid,
        _ => TreeSupportStyle::Default,
    }
}

/// Canonical `normal(Point, len)`: rescale a vector to exactly `len`.
/// A zero-length input stays zero (canonical divides by the norm, so the
/// caller must never hand it one; this port is defensive instead).
fn normal_to_length(v: (f32, f32), len: f32) -> (f32, f32) {
    let n = (v.0 * v.0 + v.1 * v.1).sqrt();
    if n <= 1e-12 || !n.is_finite() {
        return (0.0, 0.0);
    }
    (v.0 / n * len, v.1 / n * len)
}

/// Nearest point on any ring of `expolys` — contours **and** holes — to `pt`.
///
/// Canonical spells this `projection_onto(const ExPolygons&, const Point&)`.
/// Guest-side because `slicer_core::polygon_ops` is `host-algos`-gated and the
/// WIT surface exposes no nearest-point query. Coordinates in and out are mm.
/// Returns `pt` unchanged when there is no ring to project onto.
fn projection_onto(expolys: &[ExPolygon], pt: (f32, f32)) -> (f32, f32) {
    let qx = pt.0 * SCALING_FACTOR as f32;
    let qy = pt.1 * SCALING_FACTOR as f32;
    let mut best_dist = f32::INFINITY;
    let mut best: Option<[f32; 2]> = None;
    for ex in expolys {
        for ring in std::iter::once(&ex.contour).chain(ex.holes.iter()) {
            if ring.points.len() < 3 {
                continue;
            }
            let poly: Vec<[f32; 2]> = ring
                .points
                .iter()
                .map(|p| [p.x as f32, p.y as f32])
                .collect();
            let (cp, cd) = closest_point_on_polygon(&poly, qx, qy);
            if cd < best_dist {
                best_dist = cd;
                best = Some(cp);
            }
        }
    }
    match best {
        Some(cp) => (cp[0] / SCALING_FACTOR as f32, cp[1] / SCALING_FACTOR as f32),
        None => pt,
    }
}

/// Canonical `TreeSupport::is_line_cut_by_contour(Point a, Point b)`: true when
/// the segment `a`-`b` crosses any edge of the current layer's model contours.
///
/// Canonical memoises the answer in a `std::map<std::pair<Point, Point>, bool>`
/// keyed under **both** orderings of the endpoint pair, because `drop_nodes`
/// asks the same question from both ends of an MST edge. This port keeps that
/// cache (`LineCutCache`), keyed on the scaled integer coordinates so the key
/// is hashable and exactly reproduces canonical's `Point` equality.
///
/// Both contours and holes are tested: a branch that would pass through a hole
/// wall is cut just as surely as one crossing the outer wall.
#[derive(Default)]
struct LineCutCache {
    cache: std::collections::HashMap<((i64, i64), (i64, i64)), bool>,
}

impl LineCutCache {
    fn is_line_cut_by_contour(
        &mut self,
        outlines: &[ExPolygon],
        a: (f32, f32),
        b: (f32, f32),
    ) -> bool {
        let ka = (mm_to_units(a.0), mm_to_units(a.1));
        let kb = (mm_to_units(b.0), mm_to_units(b.1));
        if let Some(hit) = self.cache.get(&(ka, kb)) {
            return *hit;
        }
        let mut cut = false;
        'outer: for ex in outlines {
            for ring in std::iter::once(&ex.contour).chain(ex.holes.iter()) {
                let n = ring.points.len();
                if n < 2 {
                    continue;
                }
                for i in 0..n {
                    let p0 = &ring.points[i];
                    let p1 = &ring.points[(i + 1) % n];
                    let c = (units_to_mm(p0.x) as f64, units_to_mm(p0.y) as f64);
                    let d = (units_to_mm(p1.x) as f64, units_to_mm(p1.y) as f64);
                    if segments_intersect((a.0 as f64, a.1 as f64), (b.0 as f64, b.1 as f64), c, d)
                    {
                        cut = true;
                        break 'outer;
                    }
                }
            }
        }
        // Canonical inserts under both orderings.
        self.cache.insert((ka, kb), cut);
        self.cache.insert((kb, ka), cut);
        cut
    }
}

/// Proper/improper segment intersection test. Iterative, allocation-free.
fn segments_intersect(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), p4: (f64, f64)) -> bool {
    fn orient(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    }
    fn on_segment(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> bool {
        c.0 >= a.0.min(b.0) && c.0 <= a.0.max(b.0) && c.1 >= a.1.min(b.1) && c.1 <= a.1.max(b.1)
    }
    let d1 = orient(p3, p4, p1);
    let d2 = orient(p3, p4, p2);
    let d3 = orient(p1, p2, p3);
    let d4 = orient(p1, p2, p4);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    (d1 == 0.0 && on_segment(p3, p4, p1))
        || (d2 == 0.0 && on_segment(p3, p4, p2))
        || (d3 == 0.0 && on_segment(p1, p2, p3))
        || (d4 == 0.0 && on_segment(p1, p2, p4))
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
    calc_radius(
        branch_radius,
        tan_diameter_angle,
        (dist_to_top as f32) * effective_layer_height,
        0,
    )
}

/// Raise a tapered radius back to the branch base while it is in a roof band.
/// Canonical applies this after the ordinary taper and only when interface
/// layers are configured; body nodes and zero-layer configurations are unchanged.
pub fn interface_adjusted_radius(
    radius: f32,
    base_radius: f32,
    support_interface_top_layers: i32,
    is_roof: bool,
) -> f32 {
    if support_interface_top_layers > 0 && is_roof {
        radius.max(base_radius)
    } else {
        radius
    }
}

/// Canonical `calc_branch_radius(branch_radius, mm_to_top, diameter_angle)` —
/// the same taper as [`tapered_radius`] but taking the distance from the tip
/// directly in **mm** rather than in layers.
///
/// The F-13 move pass needs the mm form: canonical evaluates the taper at
/// `node->dist_mm_to_top + node->print_z` (branch *bottom* radius) and at
/// `node->dist_mm_to_top + height_next`, neither of which is a whole number of
/// layers.
pub fn calc_radius(
    branch_radius: f32,
    tan_diameter_angle: f32,
    mm_to_top: f32,
    support_interface_top_layers: i32,
) -> f32 {
    let raw = if mm_to_top <= branch_radius {
        mm_to_top
    } else {
        branch_radius + (mm_to_top - branch_radius) * tan_diameter_angle
    };
    let radius = raw.clamp(MIN_BRANCH_RADIUS, MAX_BRANCH_RADIUS_MM);
    if support_interface_top_layers > 0 {
        radius.max(branch_radius)
    } else {
        radius
    }
}

/// Clamp a point into the union of avoidance polygons.
/// Canonical `is_inside_ex(const ExPolygons&, const Point&)`.
///
/// Guest-side because `slicer_core::polygon_ops` is `host-algos`-gated and is
/// not compiled for `wasm32`, and the WIT surface exposes no point-in-polygon
/// query. Delegates to [`point_in_any_expoly`], which is inside-contour AND
/// outside-every-hole — a point in a hole reads as OUTSIDE, matching
/// canonical `ExPolygon::contains`.
fn is_inside_ex(expolys: &[ExPolygon], x: f32, y: f32) -> bool {
    point_in_any_expoly(expolys, x, y)
}

/// Guest-side port of the file-scope static `move_out_expolys` in canonical
/// `TreeSupport.cpp`.
///
/// Canonical pushes `from` toward the boundary of the union of `polygons`
/// offset by `min_dist`. When that target exceeds `max_dist`, it clamps to
/// `pt_max = from0 + normal(outward_dir, max_dist)`; the saved `from0` is never
/// restored. Returns whether the point moved.
///
/// Distances are in mm. Iterative (no recursion — wasm stack).
fn move_out_expolys(
    polygons: &[ExPolygon],
    from: &mut (f32, f32),
    min_dist: f32,
    max_dist: f32,
) -> bool {
    let from0 = *from;
    let (x, y) = from0;
    if polygons.is_empty() || !is_inside_ex(polygons, x, y) {
        return false;
    }
    let polygons_dilated = union_expolys(host::offset_polygons(
        polygons,
        min_dist,
        OffsetJoinType::Miter,
        0.0,
    ));
    let target = projection_onto(&polygons_dilated, from0);
    let outward_dir = (target.0 - from0.0, target.1 - from0.1);
    let dist2 = outward_dir.0 * outward_dir.0 + outward_dir.1 * outward_dir.1;
    *from = if dist2 > max_dist * max_dist {
        let clamped = normal_to_length(outward_dir, max_dist);
        (from0.0 + clamped.0, from0.1 + clamped.1)
    } else {
        target
    };
    *from != from0
}

/// Canonical `drop_nodes`' `nodes_per_part` bucketing (F-12).
///
/// `parts` is `m_ts_data->m_layer_outlines_below[obj_layer_nr]`. Group 0 takes
/// the nodes that must reach the build plate (and every node when there are no
/// parts at all); a node inside `parts[i]` goes to `i + 1`; otherwise the node
/// joins the part whose contour it is closest to. Canonical minimises
/// `vsize2_with_unscale(position - *parts[i].contour.closest_point(position))`
/// — a squared mm distance; this compares the un-squared scaled distance
/// [`closest_point_on_polygon`] already returns, which is order-equivalent.
fn assign_node_group(parts: &[ExPolygon], to_buildplate: bool, x: f32, y: f32) -> usize {
    if to_buildplate || parts.is_empty() {
        return 0;
    }
    let qx = x * SCALING_FACTOR as f32;
    let qy = y * SCALING_FACTOR as f32;
    let mut closest_part = 0usize;
    let mut closest_dist = f32::INFINITY;
    for (i, part) in parts.iter().enumerate() {
        if is_inside_ex(std::slice::from_ref(part), x, y) {
            return i + 1;
        }
        let poly: Vec<[f32; 2]> = part
            .contour
            .points
            .iter()
            .map(|p| [p.x as f32, p.y as f32])
            .collect();
        if poly.len() < 3 {
            continue;
        }
        let (_cp, cd) = closest_point_on_polygon(&poly, qx, qy);
        if cd < closest_dist {
            closest_dist = cd;
            closest_part = i;
        }
    }
    closest_part + 1
}

/// Canonical `TreeSupport::get_max_move_dist(node, power)`:
/// `min(tan_angle * node->height, support_extrusion_width)`, in mm.
/// `power == 2` returns the SQUARE, which is what the F-11 merge tests
/// against squared mm node distances.
fn get_max_move_dist(
    node: &PlannedSupportNode,
    tan_angle: f32,
    support_extrusion_width: f32,
    power: u32,
) -> f32 {
    let d = (tan_angle * node.height)
        .min(support_extrusion_width)
        .max(0.0);
    if power == 2 {
        d * d
    } else {
        d
    }
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

    /// Canonical `SupportNode` ctor: `for (auto& neighbor : parent->merged_neighbours)
    /// { neighbor->child = this; parents.push_back(neighbor); }` (`TreeSupport.hpp`).
    /// Without it a merge-absorbed node never gains a child, so `smooth_nodes`
    /// pins it at its raw drop-pass position and the branch silhouette pops
    /// outward on every merge layer (the Z 9.2-13 terracing on SupportTest).
    #[test]
    fn create_node_wires_merged_neighbours_child_and_parents() {
        let mut arena = NodeArena::default();
        let survivor = arena.create_node(
            Point2::from_mm(0.0, 0.0),
            3,
            10,
            0,
            true,
            None,
            2.0,
            0.2,
            0.6,
            1.0,
        );
        let absorbed = arena.create_node(
            Point2::from_mm(1.0, 0.0),
            3,
            10,
            0,
            true,
            None,
            2.0,
            0.2,
            0.6,
            1.0,
        );
        arena[survivor].merged_neighbours.push(absorbed);
        arena[absorbed].valid = false;
        let child = arena.create_node(
            Point2::from_mm(0.1, 0.0),
            4,
            9,
            0,
            true,
            Some(survivor),
            1.8,
            0.2,
            0.8,
            1.0,
        );
        assert_eq!(
            arena[absorbed].child,
            Some(child),
            "every merged neighbour of the parent must adopt the new node as its child"
        );
        assert!(
            arena[child].parents.contains(&absorbed),
            "merged neighbours must join the new node's parents"
        );
    }

    /// The user-visible half of the same bug: after `smooth_nodes`, a
    /// merge-absorbed node must be an *interior* chain node (its child is the
    /// surviving column's descendant) and get pulled onto the smoothed line,
    /// not stay pinned at its raw drop-pass position.
    #[test]
    fn smooth_nodes_pulls_merge_absorbed_node_toward_the_column() {
        let mut arena = NodeArena::default();
        let mk = |arena: &mut NodeArena, x: f32, layer: usize, parent, z: f32| {
            arena.create_node(
                Point2::from_mm(x, 0.0),
                3,
                layer,
                0,
                true,
                parent,
                z,
                0.2,
                0.6,
                1.0,
            )
        };
        // Absorbed branch: tip Mpp (z2.4) -> Mp (z2.2) -> M (z2.0), all at x=2.
        let mpp = mk(&mut arena, 2.0, 7, None, 2.4);
        let mp = mk(&mut arena, 2.0, 6, Some(mpp), 2.2);
        let m = mk(&mut arena, 2.0, 5, Some(mp), 2.0);
        // Surviving trunk node S at x=0 absorbs M on its layer (STUDIO-6326).
        let s = mk(&mut arena, 0.0, 5, None, 2.0);
        arena[s].merged_neighbours.push(m);
        arena[m].valid = false;
        // The move pass then creates S's descendant one layer down.
        let c = mk(&mut arena, 0.0, 4, Some(s), 1.8);
        let records = vec![
            LayerRecord {
                layer_rev: 7,
                active: vec![mpp],
                edges: vec![],
            },
            LayerRecord {
                layer_rev: 6,
                active: vec![mp],
                edges: vec![],
            },
            LayerRecord {
                layer_rev: 5,
                active: vec![s, m],
                edges: vec![],
            },
            LayerRecord {
                layer_rev: 4,
                active: vec![c],
                edges: vec![],
            },
        ];
        smooth_nodes(&mut arena, &records, 0.4);
        assert!(
            arena[m].is_processed,
            "the absorbed node must be smoothed as a chain interior node"
        );
        assert!(
            arena[m].position.x < mm_to_units(1.5),
            "the absorbed node must be pulled toward the surviving column, got x={}",
            units_to_mm(arena[m].position.x)
        );
    }

    use super::*;

    // ── Volumes layer (defect F-16) ───────────────────────────────────────

    /// F-35: the lattice span comes from the *rotated* bbox dimensions, so
    /// rotated lattice points still reach the corners of the object bbox.
    /// The previous unrotated-span derivation could not produce a point in
    /// the corner quadrants at all.
    #[test]
    fn rotated_lattice_reaches_bbox_corners() {
        // 20x20 mm box at the origin, sampled at 1 mm.
        let grid = build_grid_points((0.0, 20.0, 0.0, 20.0), 1.0);
        assert!(!grid.is_empty());
        // Every kept point is inside the bbox.
        for (x, y) in &grid {
            assert!(
                (0.0..=20.0).contains(x) && (0.0..=20.0).contains(y),
                "grid point ({x},{y}) escaped the bbox"
            );
        }
        // Corner coverage: at least one point in each 3 mm corner square.
        let corner = |cx: f32, cy: f32| {
            grid.iter()
                .any(|(x, y)| (x - cx).abs() <= 3.0 && (y - cy).abs() <= 3.0)
        };
        assert!(
            corner(0.0, 0.0),
            "no lattice point near the (min,min) corner"
        );
        assert!(
            corner(20.0, 0.0),
            "no lattice point near the (max,min) corner"
        );
        assert!(
            corner(0.0, 20.0),
            "no lattice point near the (min,max) corner"
        );
        assert!(
            corner(20.0, 20.0),
            "no lattice point near the (max,max) corner"
        );
    }

    /// The lattice really is rotated 22 degrees: successive points along one
    /// column differ by `(-step*sin, step*cos)`.
    #[test]
    fn rotated_lattice_uses_the_canonical_22_degree_angle() {
        let grid = build_grid_points((0.0, 10.0, 0.0, 10.0), 2.0);
        let angle = 22.0f32.to_radians();
        let expected = (-2.0 * angle.sin(), 2.0 * angle.cos());
        // Points are emitted column-major (x outer, y inner), so consecutive
        // entries within a column are one `sample_step` apart in local y.
        let mut found = false;
        for pair in grid.windows(2) {
            let dx = pair[1].0 - pair[0].0;
            let dy = pair[1].1 - pair[0].1;
            if (dx - expected.0).abs() < 1e-3 && (dy - expected.1).abs() < 1e-3 {
                found = true;
                break;
            }
        }
        assert!(found, "no lattice step matched the 22-degree rotation");
    }

    /// F-1: `minimum_roof_area` is one square millimetre in scaled units.
    #[test]
    fn minimum_roof_area_is_one_square_millimetre() {
        let square_1mm = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(1.0, 0.0),
                    Point2::from_mm(1.0, 1.0),
                    Point2::from_mm(0.0, 1.0),
                ],
            },
            holes: Vec::new(),
        };
        assert!((expolygon_area(&square_1mm) - minimum_roof_area()).abs() < 1.0);
        // A 1.5 mm square is above the threshold; a 0.5 mm square is below.
        let scaled = |mm: f32| ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(mm, 0.0),
                    Point2::from_mm(mm, mm),
                    Point2::from_mm(0.0, mm),
                ],
            },
            holes: Vec::new(),
        };
        assert!(expolygon_area(&scaled(1.5)) > minimum_roof_area());
        assert!(expolygon_area(&scaled(0.5)) < minimum_roof_area());
    }

    /// Holes subtract, matching canonical `ExPolygon::area()`.
    #[test]
    fn expolygon_area_subtracts_holes() {
        let with_hole = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(4.0, 0.0),
                    Point2::from_mm(4.0, 4.0),
                    Point2::from_mm(0.0, 4.0),
                ],
            },
            holes: vec![Polygon {
                points: vec![
                    Point2::from_mm(1.0, 1.0),
                    Point2::from_mm(3.0, 1.0),
                    Point2::from_mm(3.0, 3.0),
                    Point2::from_mm(1.0, 3.0),
                ],
            }],
        };
        // 16 mm² minus 4 mm² = 12 mm².
        let one_mm2 = minimum_roof_area();
        assert!((expolygon_area(&with_hole) / one_mm2 - 12.0).abs() < 1e-3);
    }

    /// Canonical closing line fit of `generate_contact_points`, feeding the
    /// step 6 `smooth_nodes` pass.
    /// Builds a straight vertical chain of `n` nodes, one per layer, linked
    /// parent (above) / child (below), and the matching per-layer records.
    fn chain_arena(offsets: &[(f32, f32)]) -> (NodeArena, Vec<LayerRecord>) {
        let mut arena = NodeArena::default();
        let mut ids = Vec::new();
        for (i, (x, y)) in offsets.iter().enumerate() {
            let parent = if i == 0 { None } else { Some(ids[i - 1]) };
            let id = arena.create_node(
                Point2::from_mm(*x, *y),
                i as i32,
                i,
                0,
                true,
                parent,
                i as f32,
                1.0,
                i as f32,
                1.0,
            );
            ids.push(id);
        }
        let records = ids
            .iter()
            .enumerate()
            .map(|(i, id)| LayerRecord {
                layer_rev: i,
                active: vec![*id],
                edges: Vec::new(),
            })
            .collect();
        (arena, records)
    }

    /// F-33: `smooth_nodes` is the only producer of the final per-node
    /// `movement`, and canonical defines it as the half-difference of the
    /// smoothed neighbours.
    #[test]
    fn smooth_nodes_sets_movement_to_the_neighbour_half_difference() {
        let (mut arena, records) =
            chain_arena(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (2.0, 1.0), (2.0, 0.0)]);
        smooth_nodes(&mut arena, &records, DEFAULT_SUPPORT_LINE_WIDTH_MM);
        let ids: Vec<NodeId> = records.iter().map(|r| r.active[0]).collect();
        // `chain_arena` creates the column top-first, and the smoother walks
        // parent-ward from the root, so the chain order is the reverse of the
        // creation order.
        let chain: Vec<NodeId> = ids.iter().rev().copied().collect();
        for i in 1..chain.len() - 1 {
            let expected_x =
                (arena[chain[i + 1]].position.x - arena[chain[i - 1]].position.x) as f64 / 2.0;
            let expected_y =
                (arena[chain[i + 1]].position.y - arena[chain[i - 1]].position.y) as f64 / 2.0;
            // +/- 1 unit (100 nm): `movement` is derived from the smoothed
            // f64 chain, while this recomputes it from the committed integer
            // positions, so the two roundings can disagree in the last unit.
            assert!(
                (arena[chain[i]].movement.x - expected_x.round() as i64).abs() <= 1,
                "movement.x {} != (next - prev)/2 = {expected_x}",
                arena[chain[i]].movement.x
            );
            assert!(
                (arena[chain[i]].movement.y - expected_y.round() as i64).abs() <= 1,
                "movement.y {} != (next - prev)/2 = {expected_y}",
                arena[chain[i]].movement.y
            );
            assert!(
                arena[chain[i]].movement != Point2 { x: 0, y: 0 },
                "an interior node of a zig-zag chain must carry a movement"
            );
        }
        // Endpoints are held fixed and never receive a movement.
        assert_eq!(arena[chain[0]].position, Point2::from_mm(2.0, 0.0));
        assert_eq!(
            arena[chain[chain.len() - 1]].position,
            Point2::from_mm(0.0, 0.0)
        );
        assert_eq!(arena[chain[0]].movement, Point2 { x: 0, y: 0 });
    }

    /// A zig-zag chain must come out straighter than it went in.
    #[test]
    fn smooth_nodes_reduces_chain_deviation() {
        let offsets = [(0.0, 0.0), (1.0, 0.0), (0.0, 0.0), (1.0, 0.0), (0.0, 0.0)];
        let (mut arena, records) = chain_arena(&offsets);
        let ids: Vec<NodeId> = records.iter().map(|r| r.active[0]).collect();
        let before: i64 = ids.iter().map(|id| arena[*id].position.x.abs()).sum();
        smooth_nodes(&mut arena, &records, DEFAULT_SUPPORT_LINE_WIDTH_MM);
        let after: i64 = ids.iter().map(|id| arena[*id].position.x.abs()).sum();
        assert!(
            after < before,
            "smoothing must flatten the zig-zag: {after} !< {before}"
        );
    }

    /// A chain shorter than three nodes has no interior node to relax.
    #[test]
    fn smooth_nodes_leaves_short_chains_alone() {
        let (mut arena, records) = chain_arena(&[(0.0, 0.0), (3.0, 4.0)]);
        let ids: Vec<NodeId> = records.iter().map(|r| r.active[0]).collect();
        smooth_nodes(&mut arena, &records, DEFAULT_SUPPORT_LINE_WIDTH_MM);
        assert_eq!(arena[ids[0]].position, Point2::from_mm(0.0, 0.0));
        assert_eq!(arena[ids[1]].position, Point2::from_mm(3.0, 4.0));
        assert_eq!(arena[ids[1]].movement, Point2 { x: 0, y: 0 });
    }

    /// Canonical `draw_circles` resolution selection.
    #[test]
    fn circle_resolution_switches_at_two_hundred_nodes_per_layer() {
        let pick = |avg: usize| {
            if avg > COARSE_CIRCLE_NODE_THRESHOLD {
                CIRCLE_RESOLUTION_COARSE
            } else {
                CIRCLE_RESOLUTION_FINE
            }
        };
        assert_eq!(pick(200), 100);
        assert_eq!(pick(201), 4);
    }

    /// The ellipse matrix degenerates to a plain scaled circle when the node
    /// has not moved, and elongates along the movement direction when it has.
    ///
    /// Canonical's guard is `!SQUARE_SUPPORT && std::abs(moveX) > 0.001 &&
    /// std::abs(moveY) > 0.001`, so an axis-aligned movement (one component
    /// zero) is *not* elongated — asserted below alongside the diagonal case.
    #[test]
    fn node_ellipse_elongates_along_movement() {
        let radius_units = mm_to_units(2.0) as f64;
        let base = branch_circle(CIRCLE_RESOLUTION_FINE, radius_units, 0.0);
        let center = Point2 { x: 0, y: 0 };
        let still = node_ellipse(
            &base,
            center,
            1.0,
            Point2 { x: 0, y: 0 },
            radius_units,
            false,
        )
        .unwrap();
        let extent = |poly: &ExPolygon, axis: fn(&Point2) -> i64| {
            poly.contour.points.iter().map(axis).max().unwrap()
                - poly.contour.points.iter().map(axis).min().unwrap()
        };
        let still_x = extent(&still, |p| p.x);
        let still_y = extent(&still, |p| p.y);
        assert!(
            (still_x - still_y).abs() <= 2,
            "a stationary node draws a circle"
        );

        let diagonal = Point2 {
            x: mm_to_units(1.0),
            y: mm_to_units(1.0),
        };
        let moving = node_ellipse(&base, center, 1.0, diagonal, radius_units, false).unwrap();
        let span = |poly: &ExPolygon, ux: f64, uy: f64| {
            let proj: Vec<f64> = poly
                .contour
                .points
                .iter()
                .map(|p| p.x as f64 * ux + p.y as f64 * uy)
                .collect();
            proj.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                - proj.iter().cloned().fold(f64::INFINITY, f64::min)
        };
        let axis = std::f64::consts::FRAC_1_SQRT_2;
        assert!(
            span(&moving, axis, axis) > span(&still, axis, axis),
            "the ellipse must stretch along the movement direction"
        );
        assert!(
            (span(&moving, axis, -axis) - span(&still, axis, -axis)).abs() <= 2.0,
            "and must not stretch across it"
        );

        // Canonical leaves an axis-aligned mover, and every square-support
        // node, as a plain scaled circle.
        for (movement, square) in [
            (
                Point2 {
                    x: mm_to_units(1.0),
                    y: 0,
                },
                false,
            ),
            (diagonal, true),
        ] {
            let plain = node_ellipse(&base, center, 1.0, movement, radius_units, square).unwrap();
            assert_eq!(extent(&plain, |p| p.x), still_x);
            assert_eq!(extent(&plain, |p| p.y), still_y);
        }
    }

    #[test]
    fn contact_stats_fit_a_45_degree_line() {
        let positions = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
        let stats = contact_stats(&positions, 2);
        assert_eq!(stats.avg_node_per_layer, 2);
        assert!(
            (stats.nodes_angle - std::f32::consts::FRAC_PI_4).abs() < 1e-4,
            "expected 45 degrees, got {}",
            stats.nodes_angle
        );
        assert_eq!(contact_stats(&[], 0), ContactStats::default());
    }

    /// The arena is what makes cross-layer parent/child edges expressible:
    /// creating a child writes the back-edge into the already-created parent.
    #[test]
    fn arena_create_node_links_parent_and_child_both_ways() {
        let mut arena = NodeArena::default();
        let parent = arena.create_node(
            Point2::from_mm(1.0, 2.0),
            -1,
            5,
            3,
            true,
            None,
            1.0,
            0.2,
            0.0,
            0.4,
        );
        let child = arena.create_node(
            Point2::from_mm(1.0, 2.0),
            0,
            4,
            3,
            true,
            Some(parent),
            0.8,
            0.2,
            0.0,
            0.4,
        );
        assert_eq!(arena.len(), 2);
        assert_eq!(arena[child].parent, Some(parent));
        assert_eq!(arena[child].parents, vec![parent]);
        assert_eq!(arena[parent].child, Some(child));
        assert!((arena[parent].x() - 1.0).abs() < 1e-4);
        assert!((arena[parent].y() - 2.0).abs() < 1e-4);
        // F-34: the seeded contact is virtual, so it is a roof node but must
        // never be extruded.
        assert!(arena[parent].distance_to_top < 0);
        assert!(arena[parent].is_roof());
    }
    #[test]
    fn ceil_radius_snaps_up_to_the_canonical_sample_resolution() {
        // Canonical `m_radius_sample_resolution` is 0.2 mm.
        assert!((ceil_radius(0.0) - 0.0).abs() < 1e-6);
        assert!((ceil_radius(0.01) - 0.2).abs() < 1e-6);
        assert!((ceil_radius(0.2) - 0.2).abs() < 1e-6);
        assert!((ceil_radius(0.21) - 0.4).abs() < 1e-6);
        assert!((ceil_radius(2.5) - 2.6).abs() < 1e-6);
        // Negative / non-finite radii bucket to zero rather than panicking.
        assert!((ceil_radius(-1.0) - 0.0).abs() < 1e-6);
        assert!((ceil_radius(f32::NAN) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn radius_key_buckets_nearby_radii_into_one_cache_slot() {
        assert_eq!(radius_key(0.41), radius_key(0.59));
        assert_ne!(radius_key(0.41), radius_key(0.61));
        assert_eq!(radius_key(0.6), mm_to_units(0.6));
    }

    #[test]
    fn douglas_peucker_drops_collinear_points_and_keeps_corners() {
        // A straight run of samples collapses to its endpoints; the corner at
        // the far end survives.
        let pts: Vec<Point2> = vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 1_000, y: 0 },
            Point2 { x: 2_000, y: 0 },
            Point2 { x: 3_000, y: 0 },
            Point2 { x: 3_000, y: 3_000 },
        ];
        let out = douglas_peucker_open(&pts, mm_to_units(0.2) as f64);
        assert_eq!(
            out,
            vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 3_000, y: 0 },
                Point2 { x: 3_000, y: 3_000 },
            ]
        );
    }

    #[test]
    fn douglas_peucker_keeps_deviation_above_tolerance() {
        // 0.2 mm tolerance = 2000 units. A 3000-unit bump must survive.
        let pts: Vec<Point2> = vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 5_000, y: 3_000 },
            Point2 { x: 10_000, y: 0 },
        ];
        let out = douglas_peucker_open(&pts, mm_to_units(0.2) as f64);
        assert_eq!(out.len(), 3, "deviation above tolerance must be kept");
    }

    #[test]
    fn expolygons_simplify_preserves_a_square_and_its_hole() {
        let square = |half: i64| Polygon {
            points: vec![
                Point2 { x: -half, y: -half },
                Point2 { x: half, y: -half },
                Point2 { x: half, y: half },
                Point2 { x: -half, y: half },
            ],
        };
        // Densify one edge with points that are exactly collinear.
        let mut contour = square(mm_to_units(10.0));
        contour.points.insert(
            1,
            Point2 {
                x: 0,
                y: -mm_to_units(10.0),
            },
        );
        let input = vec![ExPolygon {
            contour,
            holes: vec![square(mm_to_units(2.0))],
        }];
        let out = expolygons_simplify(&input, mm_to_units(0.2) as f64);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].contour.points.len(), 4, "collinear insert removed");
        assert_eq!(out[0].holes.len(), 1, "hole survives simplification");
        assert_eq!(out[0].holes[0].points.len(), 4);
    }

    #[test]
    fn expolygons_simplify_drops_rings_that_collapse() {
        // A sliver far below the tolerance has no vertex worth keeping.
        let sliver = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 { x: 10, y: 0 },
                    Point2 { x: 10, y: 10 },
                ],
            },
            holes: vec![],
        };
        let out = expolygons_simplify(&[sliver], mm_to_units(0.2) as f64);
        assert!(
            out.is_empty(),
            "collapsed ring must be dropped, not emitted degenerate"
        );
    }

    fn expolygons_simplify_notched_outline() -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(10.0, 0.0),
                    Point2::from_mm(10.0, 4.0),
                    Point2::from_mm(9.9, 4.0),
                    Point2::from_mm(9.9, 6.0),
                    Point2::from_mm(10.0, 6.0),
                    Point2::from_mm(10.0, 10.0),
                    Point2::from_mm(0.0, 10.0),
                ],
            },
            holes: Vec::new(),
        }
    }

    fn expolygons_simplify_notch_island() -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(9.92, 4.5),
                    Point2::from_mm(9.98, 4.5),
                    Point2::from_mm(9.98, 5.5),
                    Point2::from_mm(9.92, 5.5),
                ],
            },
            holes: Vec::new(),
        }
    }

    #[test]
    fn expolygons_simplify_union_merges_parts_after_ring_simplification() {
        let input = vec![
            expolygons_simplify_notched_outline(),
            expolygons_simplify_notch_island(),
        ];
        let raw_union = union_expolys(input.clone());
        assert_eq!(
            raw_union.len(),
            2,
            "the island starts outside the notched part"
        );

        let simplified =
            expolygons_simplify_union(&input, mm_to_units(RADIUS_SAMPLE_RESOLUTION_MM) as f64);
        assert_eq!(
            simplified.len(),
            1,
            "the final union must merge parts after the shallow notch is removed"
        );
    }

    #[test]
    fn expolygons_simplify_union_merges_a_touching_hole_into_the_contour() {
        let mut outline = square_mm(0.0, 0.0, 10.0);
        outline.holes.push(Polygon {
            points: vec![
                Point2::from_mm(0.0, 4.0),
                Point2::from_mm(0.1, 4.5),
                Point2::from_mm(0.0, 5.0),
                Point2::from_mm(2.0, 5.0),
                Point2::from_mm(2.0, 4.0),
            ],
        });

        let simplified =
            expolygons_simplify_union(&[outline], mm_to_units(RADIUS_SAMPLE_RESOLUTION_MM) as f64);
        assert_eq!(simplified.len(), 1);
        assert!(
            simplified[0].holes.is_empty(),
            "the final union must absorb a hole that simplification joins to the contour"
        );
    }

    #[test]
    fn expolygons_simplify_union_runs_before_tree_volumes_outlines_below() {
        let outlines = vec![
            expolygons_simplify_notched_outline(),
            expolygons_simplify_notch_island(),
        ];
        assert_eq!(union_expolys(outlines.clone()).len(), 2);
        let layer_plan = default_layer_plan(1, 0.0, 0.2);
        let support_geometry = SupportGeometryView {
            entries: vec![SupportGeometryViewEntry {
                global_support_layer_index: 0,
                object_id: "outline-order".to_string(),
                region_id: "0".to_string(),
                outlines,
            }],
        };

        let volumes = TreeVolumes::new(&layer_plan, &support_geometry, 40.0, 0.35);
        assert_eq!(
            volumes.outlines_at(0).len(),
            1,
            "constructor must store the simplified layer outlines"
        );
        assert_eq!(
            volumes.outlines_below(0).len(),
            1,
            "outlines_below must be built from the already-simplified layer"
        );
    }

    fn build_roles_square(x0: f32, y0: f32, side: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(x0, y0),
                    Point2::from_mm(x0 + side, y0),
                    Point2::from_mm(x0 + side, y0 + side),
                    Point2::from_mm(x0, y0 + side),
                ],
            },
            holes: Vec::new(),
        }
    }

    #[test]
    fn build_roles_mixed_layer_keeps_exact_disjoint_body_difference() {
        let base = build_roles_square(0.0, 0.0, 4.0);
        let roof = build_roles_square(1.0, 0.0, 2.0);
        let expected = host::clip_polygons(
            std::slice::from_ref(&base),
            std::slice::from_ref(&roof),
            ClipOperation::Difference,
        );
        let roles = build_roles(
            &[],
            &[],
            &[],
            &[],
            &[base],
            &[roof],
            &[],
            &[],
            1.0,
            &[],
            0,
            0.4,
        );
        let body = &roles
            .iter()
            .find(|role| role.role == slicer_ir::SupportPlanRole::SupportBody)
            .expect("mixed layer must retain body")
            .regions;
        let roof = &roles
            .iter()
            .find(|role| role.role == slicer_ir::SupportPlanRole::TopInterface)
            .expect("mixed layer must retain roof")
            .regions;
        assert_eq!(body, &expected, "body must equal base minus roofs union");
        assert!(
            host::clip_polygons(body, roof, ClipOperation::Intersection).is_empty(),
            "body and roof must be disjoint"
        );
    }

    /// Two overlapping node cross-sections on one layer must print as a
    /// single merged outline, not as two full loops crossing each other.
    ///
    /// Canonical `draw_circles` appends every carved node circle into
    /// `base_areas` and then runs `diff_ex(base_areas, roofs)` /
    /// `intersection_ex(base_areas, m_machine_border)`; both are Clipper
    /// boolean ops over the whole subject set, so the returned `ExPolygons`
    /// are non-overlapping — adjacent circles come out fused. Emitting them
    /// unmerged put a duplicate perimeter through the inside of every fused
    /// branch pair and made the branch silhouette pop between layers as
    /// neighbouring circles drifted in and out of contact.
    #[test]
    fn build_roles_merges_overlapping_node_cross_sections_into_one_outline() {
        let radius_units = mm_to_units(1.0) as f64;
        let circle = |cx: f32| {
            node_ellipse(
                &branch_circle(CIRCLE_RESOLUTION_FINE, radius_units, 0.0),
                Point2::from_mm(cx, 0.0),
                1.0,
                Point2 { x: 0, y: 0 },
                radius_units,
                false,
            )
            .expect("circle must be non-degenerate")
        };
        // Centres 1.0mm apart with radius 1.0mm: the discs overlap.
        let areas = vec![circle(0.0), circle(1.0)];
        let roles = build_roles(&[], &[], &[], &[], &areas, &[], &[], &[], 1.0, &[], 0, 0.4);
        let body = &roles
            .iter()
            .find(|role| role.role == slicer_ir::SupportPlanRole::SupportBody)
            .expect("overlapping circles must produce a body role")
            .regions;
        assert_eq!(
            body.len(),
            1,
            "overlapping node cross-sections must fuse into one outline, got {}              separate regions (each would be walled independently)",
            body.len()
        );
    }

    #[test]
    fn build_roles_normal_density_keeps_fine_circle_vertices() {
        let radius_units = mm_to_units(1.0) as f64;
        let ellipse = node_ellipse(
            &branch_circle(CIRCLE_RESOLUTION_FINE, radius_units, 0.0),
            Point2::from_mm(0.0, 0.0),
            1.0,
            Point2 { x: 0, y: 0 },
            radius_units,
            false,
        )
        .expect("fine circle");
        let roles = build_roles(
            &[],
            &[],
            &[],
            &[],
            &[ellipse],
            &[],
            &[],
            &[],
            1.0,
            &[],
            0,
            0.4,
        );
        let vertex_count = roles[0].regions[0].contour.points.len();
        assert!(
            vertex_count > BRANCH_CIRCLE_SEGMENTS,
            "normal-density contour was truncated/simplified to {vertex_count} vertices"
        );
    }

    #[test]
    fn build_roles_structural_contours_use_circle_resolution() {
        let point = |x: f32, y: f32, width: f32| Point3WithWidth {
            x,
            y,
            z: 0.2,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
            overhang_distance_mm: None,
        };
        let fallback = point(0.0, 0.0, 2.0);
        let regions = structural_body_regions(&[vec![fallback, fallback]], 1.0);
        assert!(regions[0].contour.points.len() <= BRANCH_CIRCLE_SEGMENTS);
    }

    #[test]
    fn structural_regions_exclude_mst_edges_but_keep_node_fallbacks() {
        let point = |x: f32, y: f32| Point3WithWidth {
            x,
            y,
            z: 0.2,
            width: 2.0,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
            overhang_distance_mm: None,
        };
        let fallback = point(2.0, 3.0);
        let regions = structural_body_regions(
            &[
                vec![point(-5.0, 0.0), point(5.0, 10.0)],
                vec![point(5.0, 0.0), point(-5.0, 10.0)],
                vec![fallback, fallback],
            ],
            1.0,
        );
        assert_eq!(regions.len(), 1, "only the per-node fallback may be drawn");
    }

    #[test]
    fn build_roles_simplifies_only_base_under_square_density_at_half_line_width() {
        assert_eq!(role_simplify_tolerance(true, 200, 0.4), None);
        assert_eq!(role_simplify_tolerance(false, 201, 0.4), None);
        assert_eq!(
            role_simplify_tolerance(true, 201, 0.4),
            Some(mm_to_units(0.2) as f64)
        );
    }

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
            support_line_width_mm: DEFAULT_SUPPORT_LINE_WIDTH_MM,
            max_branches_per_layer: DEFAULT_MAX_BRANCHES_PER_LAYER,
            line_width_mm: DEFAULT_LINE_WIDTH_MM,
            tree_support_branch_diameter: 5.0,
            tree_support_branch_diameter_angle: 5.0,
            tree_support_branch_distance: 1.0,
            tree_support_wall_count: 1,
            tree_support_is_slim: false,
            tree_support_style: TreeSupportStyle::Default,
            organic_substitution_requested: false,
            support_raft_layers: 0,
            raft_first_layer_density: 0.4,
            base_raft_layers: 1,
            interface_raft_layers: 0,
            support_interface_top_layers: 2,
            num_top_base_interface_layers: 0,
            support_interface_bottom_layers: -1,
            support_on_build_plate_only: false,
            support_top_z_distance_mm: DEFAULT_TOP_Z_DISTANCE_MM,
            support_layer_height_mm: 0.0,
            independent_support_layer_height: true,
            support_object_xy_distance: DEFAULT_SUPPORT_OBJECT_XY_DISTANCE_MM,
            max_bridge_length_mm: DEFAULT_MAX_BRIDGE_LENGTH_MM,
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

        // Canonical seeds contacts into `contact_nodes[layer_nr - 1]`
        // ("Support must always be 1 layer below overhang"), so the fixture's
        // layer-8 overhang tops its column out on layer 7 even at a zero gap
        // (packet 224 defect F-34).
        let origin_entry = output
            .entries()
            .iter()
            .find(|entry| entry.global_layer_index == 7)
            .unwrap_or_else(|| {
                panic!(
                    "lone fresh contact must emit on layer 7 (one below its layer-8 overhang); got layers {:?} diags {:?}",
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
        // Layer 7 on this 0.2mm stack; see the F-34 note above.
        assert!((segment[0].z - 1.6).abs() < 1e-5);
        assert!((segment[1].z - 1.6).abs() < 1e-5);
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
        let nodes = vec![(0.0_f32, 0.0_f32), (3.0_f32, 4.0_f32)];
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

    // ── F-12 / F-11 canonical merge helpers ──────────────────────────────

    fn square_mm(x0: f32, y0: f32, side: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(x0, y0),
                    Point2::from_mm(x0 + side, y0),
                    Point2::from_mm(x0 + side, y0 + side),
                    Point2::from_mm(x0, y0 + side),
                ],
            },
            holes: Vec::new(),
        }
    }

    /// F-12: canonical `nodes_per_part` bucketing. Group 0 is the
    /// to-buildplate bucket, `parts[i]` is group `i + 1`.
    #[test]
    fn assign_node_group_matches_canonical_nodes_per_part() {
        let parts = vec![square_mm(0.0, 0.0, 10.0), square_mm(50.0, 0.0, 10.0)];
        // to_buildplate always takes group 0, wherever the node sits.
        assert_eq!(assign_node_group(&parts, true, 5.0, 5.0), 0);
        // No parts at all: everything falls into group 0.
        assert_eq!(assign_node_group(&[], false, 5.0, 5.0), 0);
        // Inside part 0 / part 1 → 1 / 2.
        assert_eq!(assign_node_group(&parts, false, 5.0, 5.0), 1);
        assert_eq!(assign_node_group(&parts, false, 55.0, 5.0), 2);
        // Outside both: the closest part wins.
        assert_eq!(assign_node_group(&parts, false, -5.0, 5.0), 1);
        assert_eq!(assign_node_group(&parts, false, 65.0, 5.0), 2);
    }

    /// F-12 is what stops two nodes on opposite sides of a model from ever
    /// becoming MST neighbours: they land in different groups, and canonical
    /// runs one spanning tree per group.
    #[test]
    fn per_part_grouping_separates_opposite_sides_of_the_object() {
        let parts = vec![square_mm(0.0, 0.0, 10.0), square_mm(50.0, 0.0, 10.0)];
        let left = assign_node_group(&parts, false, 2.0, 2.0);
        let right = assign_node_group(&parts, false, 52.0, 2.0);
        assert_ne!(
            left, right,
            "nodes inside different parts must not share a spanning tree"
        );
    }

    /// `is_inside_ex` must treat a point in a hole as OUTSIDE, matching
    /// canonical `ExPolygon::contains`.
    #[test]
    fn is_inside_ex_treats_holes_as_outside() {
        let mut ring = square_mm(0.0, 0.0, 10.0);
        ring.holes.push(Polygon {
            points: vec![
                Point2::from_mm(3.0, 3.0),
                Point2::from_mm(7.0, 3.0),
                Point2::from_mm(7.0, 7.0),
                Point2::from_mm(3.0, 7.0),
            ],
        });
        let polys = vec![ring];
        assert!(is_inside_ex(&polys, 1.0, 1.0), "inside the contour");
        assert!(!is_inside_ex(&polys, 5.0, 5.0), "inside a hole is outside");
        assert!(!is_inside_ex(&polys, 20.0, 20.0), "outside entirely");
    }

    #[test]
    fn move_out_expolys_projects_onto_the_dilated_ring() {
        let mut poly = square_mm(0.0, 0.0, 10.0);
        poly.holes.push(Polygon {
            points: vec![
                Point2::from_mm(4.0, 4.0),
                Point2::from_mm(4.0, 6.0),
                Point2::from_mm(6.0, 6.0),
                Point2::from_mm(6.0, 4.0),
            ],
        });
        let mut point = (3.5, 3.5);

        assert!(move_out_expolys(&[poly], &mut point, 0.2, 100.0));
        assert!(
            (point.0 - 4.2).abs() < 1e-3 && (point.1 - 4.2).abs() < 1e-3,
            "expected the mitered dilated-ring corner, got {point:?}"
        );
    }

    #[test]
    fn move_out_expolys_clamps_to_pt_max_when_budget_is_exceeded() {
        let polys = vec![square_mm(0.0, 0.0, 10.0)];
        let mut point = (0.5, 5.0);

        assert!(move_out_expolys(&polys, &mut point, 0.2, 0.1));
        assert!(
            (point.0 - 0.4).abs() < 1e-3 && (point.1 - 5.0).abs() < 1e-3,
            "expected pt_max clamp instead of the original point, got {point:?}"
        );
    }

    #[test]
    fn move_out_expolys_returns_whether_movement_happened() {
        let polys = vec![square_mm(0.0, 0.0, 10.0)];
        let mut inside = (0.5, 5.0);
        let mut outside = (20.0, 5.0);

        assert!(move_out_expolys(&polys, &mut inside, 0.2, 100.0));
        assert!(!move_out_expolys(&polys, &mut outside, 0.2, 100.0));
        assert_eq!(outside, (20.0, 5.0));
    }

    #[test]
    fn sample_contact_points_erosion_plumbs_miter_limit_3() {
        // Inward erosion turns this V-notch into a convex join whose miter
        // ratio is between Clipper's 2 and 3 limits.
        let polygon = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(40.0, 0.0),
                    Point2::from_mm(40.0, 20.0),
                    Point2::from_mm(21.0, 20.0),
                    Point2::from_mm(20.0, 1.0),
                    Point2::from_mm(19.0, 20.0),
                    Point2::from_mm(0.0, 20.0),
                ],
            },
            holes: Vec::new(),
        };
        let grid: Vec<(f32, f32)> = (0..=13)
            .flat_map(|x| (0..=6).map(move |y| (x as f32 * 3.0, y as f32 * 3.0)))
            .collect();
        let eroded_3 = host::offset_polygons_with_miter_limit(
            std::slice::from_ref(&polygon),
            -2.0,
            OffsetJoinType::Miter,
            3.0,
            0.0,
        );
        let expected = grid
            .iter()
            .copied()
            .filter(|&(x, y)| point_in_any_expoly(&eroded_3, x, y))
            .collect::<Vec<_>>();
        assert!(
            !expected.is_empty(),
            "fixture must expose inner-grid points after miter-limit-3 erosion"
        );

        let samples = sample_contact_points(
            std::slice::from_ref(&polygon),
            Some(&grid),
            0.0,
            2.0,
            0.0,
            false,
        );
        let mut inner_samples = samples.iter().filter(|sample| !sample.is_corner);
        assert!(
            inner_samples.clone().next().is_some()
                && inner_samples.all(|sample| {
                    expected
                        .iter()
                        .any(|&(x, y)| (sample.x - x).abs() < 1e-5 && (sample.y - y).abs() < 1e-5)
                }),
            "sampled inner grid must match explicit miter-limit-3 erosion"
        );
    }

    /// Canonical `get_max_move_dist`: `min(tan_angle * height, width)`, and
    /// `power == 2` returns the SQUARE.
    #[test]
    fn get_max_move_dist_caps_at_the_support_extrusion_width() {
        let mut arena = NodeArena::default();
        let id = arena.create_node(
            Point2::from_mm(0.0, 0.0),
            0,
            0,
            0,
            true,
            None,
            0.2,
            0.2, // height mm
            0.0,
            1.0,
        );
        // tan(45°) = 1 → 1 * 0.2 = 0.2 mm, below the 0.35 mm width cap.
        let d = get_max_move_dist(&arena[id], 1.0, DEFAULT_SUPPORT_LINE_WIDTH_MM, 1);
        assert!((d - 0.2).abs() < 1e-6, "got {d}");
        let d2 = get_max_move_dist(&arena[id], 1.0, DEFAULT_SUPPORT_LINE_WIDTH_MM, 2);
        assert!((d2 - 0.04).abs() < 1e-6, "power 2 must square: got {d2}");
        // A tall node is capped by the extrusion width instead.
        let tall = arena.create_node(
            Point2::from_mm(0.0, 0.0),
            0,
            0,
            0,
            true,
            None,
            5.0,
            5.0,
            0.0,
            1.0,
        );
        let capped = get_max_move_dist(&arena[tall], 1.0, DEFAULT_SUPPORT_LINE_WIDTH_MM, 1);
        assert!(
            (capped - DEFAULT_SUPPORT_LINE_WIDTH_MM).abs() < 1e-6,
            "got {capped}"
        );
    }

    /// MED-1 gap-close: DEV-144's `SupportPlanSkeleton.wall_counts` emit fill
    /// (`plan_for_object`) was only ever exercised by fixtures whose nodes
    /// never carry `need_extra_wall`, so the NONZERO path (a node with
    /// `need_extra_wall >= 1` producing `wall_counts[i] >= 1`) was untested.
    ///
    /// This is a gap-closing pin, not red-first TDD: the fixture below was
    /// chosen by probing until a full-plan run produced nonzero `wall_counts`
    /// (an 8×8 mm plate floating at z=12 mm over 60 layers, whose spread
    /// contact tips converge into a single trunk as they descend — the
    /// convergence produces `parents.len() > 1` merge nodes that
    /// `smooth_nodes` flags with `need_extra_wall`).
    ///
    /// Because the emit fill is inline in `plan_for_object` (there is no
    /// extracted skeleton-builder to call with a hand-built arena), the test
    /// drives the full plan end-to-end and asserts on the emitted
    /// `SupportPlanSkeleton`. Each skeleton must preserve the
    /// `wall_counts.len() == points.len()` parity that both marshal legs
    /// enforce, and the output must exercise the nonzero path while leaving
    /// plain (non-extra-wall) nodes at 0.
    #[test]
    fn wall_counts_emit_nonzero_for_extra_wall_nodes() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 12.0],
            [8.0, 0.0, 12.0],
            [8.0, 8.0, 12.0],
            [0.0, 8.0, 12.0],
        ];
        let triangles = vec![[1, 3, 2], [1, 4, 3]];
        let obj = MeshObjectView {
            object_id: "plate".to_string(),
            vertices,
            triangles,
            paint_layers: vec![],
        };
        let planner = default_planner();
        let lp = default_layer_plan(60, 0.0, 0.2);
        let rs = default_region_segmentation("plate", 60);
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

        let mut nonzero = 0usize;
        let mut zero = 0usize;
        let mut skeleton_entries = 0usize;
        for entry in output.entries() {
            let Some(skeleton) = &entry.skeleton else {
                continue;
            };
            skeleton_entries += 1;
            // Length parity — the invariant the WIT marshal legs assert.
            assert_eq!(
                skeleton.wall_counts.len(),
                skeleton.points.len(),
                "wall_counts/points length parity violated for entry at \
                 layer {} region {}",
                entry.global_layer_index,
                entry.region_id,
            );
            for wc in &skeleton.wall_counts {
                assert!(*wc <= 1, "wall_counts is a bool-derived count but got {wc}");
                if *wc != 0 {
                    nonzero += 1;
                } else {
                    zero += 1;
                }
            }
        }
        assert!(
            skeleton_entries > 0,
            "fixture must produce at least one skeleton entry"
        );
        assert!(
            nonzero > 0,
            "expected at least one wall_counts[i] >= 1 from an extra-wall node; \
             got all zeros"
        );
        assert!(
            zero > 0,
            "plain (non-extra-wall) nodes must stay at 0, but every \
             wall_counts entry was nonzero"
        );
    }
}
