// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.{hpp,cpp}
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

//! Parent/child graph primitive for Lightning sparse infill.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use slicer_ir::{slice_ir::BoundingBox2, Point2};

use super::distance_field::point_in_polygon;

// Orca ref: Node::straighten and close_enough (TreeNode.cpp); PnP lengths divide by 100.
const CLOSE_ENOUGH_PNP_UNITS: f64 = 0.1;
const WEIGHT_PNP_UNITS: i64 = 10;

/// Shared ownership handle for a [`Node`].
pub type NodeRef = Rc<RefCell<Node>>;

pub(crate) fn to_grid_point(location: Point2, bbox: BoundingBox2, cell_size: i64) -> (i32, i32) {
    let cell_size = cell_size.max(1);
    (
        location.x.saturating_sub(bbox.min.x).div_euclid(cell_size) as i32,
        location.y.saturating_sub(bbox.min.y).div_euclid(cell_size) as i32,
    )
}

/// A location in a Lightning tree and its child branches.
pub struct Node {
    location: Cell<Point2>,
    parent: RefCell<Weak<RefCell<Node>>>,
    children: RefCell<Vec<NodeRef>>,
    is_root: Cell<bool>,
    // `Cell` (not a plain field) because canonical `Node::realign` clears and
    // rewrites this on nodes reached through a `const`-qualified traversal.
    m_last_grounding_location: Cell<Option<Point2>>,
    self_ref: RefCell<Weak<RefCell<Node>>>,
}

impl Node {
    /// Construct a root node at `loc`.
    pub fn new(loc: Point2) -> NodeRef {
        Self::new_with_grounding_location(loc, None)
    }

    /// Construct a root node and retain the location where it was grounded.
    pub fn new_with_grounding_location(
        loc: Point2,
        last_grounding_location: Option<Point2>,
    ) -> NodeRef {
        Rc::new_cyclic(|self_ref| {
            RefCell::new(Self {
                location: Cell::new(loc),
                parent: RefCell::new(Weak::new()),
                children: RefCell::new(Vec::new()),
                is_root: Cell::new(true),
                m_last_grounding_location: Cell::new(last_grounding_location),
                self_ref: RefCell::new(self_ref.clone()),
            })
        })
    }

    /// Return this node's location.
    pub fn location(&self) -> Point2 {
        self.location.get()
    }

    /// Update this node's location.
    pub fn set_location(&self, loc: Point2) {
        self.location.set(loc);
    }

    /// Return whether this node has no parent.
    pub fn is_root(&self) -> bool {
        self.is_root.get()
    }

    /// Return a snapshot of this node's direct children.
    pub fn children(&self) -> Vec<NodeRef> {
        self.children.borrow().clone()
    }

    // 139 DEVIATION: extends 138 surface for per-layer Layer operations; tests co-updated in 139 Step 2.

    /// Return the most recent location at which this root was grounded.
    pub fn get_last_grounding_location(&self) -> Option<Point2> {
        self.m_last_grounding_location.get()
    }

    /// Set the location at which this root was grounded.
    pub fn set_last_grounding_location(&self, location: Option<Point2>) {
        self.m_last_grounding_location.set(location);
    }

    /// Return whether `candidate` is this node or a descendant of this node.
    pub fn has_offspring(&self, candidate: NodeRef) -> bool {
        let self_rc = self
            .self_ref
            .borrow()
            .upgrade()
            .expect("node self reference");
        if Rc::ptr_eq(&self_rc, &candidate) {
            return true;
        }

        self.children()
            .into_iter()
            .any(|child| child.borrow().has_offspring(Rc::clone(&candidate)))
    }

    /// Return the nearest node in this subtree, preserving depth-first ties.
    pub fn closest_node(&self, target: Point2) -> NodeRef {
        let mut closest = self
            .self_ref
            .borrow()
            .upgrade()
            .expect("node self reference");
        let mut closest_distance = squared_distance(self.location(), target);

        for child in self.children() {
            let candidate = child.borrow().closest_node(target);
            let candidate_distance = squared_distance(candidate.borrow().location(), target);
            if candidate_distance < closest_distance {
                closest = candidate;
                closest_distance = candidate_distance;
            }
        }

        closest
    }

    /// Return the distance to an unsupported location, with a valence bonus.
    pub fn get_weighted_distance(&self, unsupported: Point2, supporting_radius: i64) -> i64 {
        // Orca ref: Node::getWeightedDistance (TreeNode.cpp).
        const MIN_VALENCE_FOR_BOOST: usize = 0;
        const MAX_VALENCE_FOR_BOOST: usize = 4;
        const VALENCE_BOOST_MULTIPLIER: i64 = 4;

        let valence = usize::from(!self.is_root()) + self.children.borrow().len();
        let valence_boost = if MIN_VALENCE_FOR_BOOST < valence && valence < MAX_VALENCE_FOR_BOOST {
            VALENCE_BOOST_MULTIPLIER.saturating_mul(supporting_radius)
        } else {
            0
        };
        distance_between(self.location(), unsupported).saturating_sub(valence_boost)
    }

    /// Append this tree's branch polylines in deterministic depth-first order.
    pub fn convert_to_polylines(&self, out: &mut Vec<Vec<Point2>>, line_overlap: i64) {
        // Orca ref: Node::convertToPolylines and removeJunctionOverlap (TreeNode.cpp).
        let mut result = vec![Vec::new()];
        self.convert_to_polylines_recursive(0, &mut result);
        for polyline in &mut result {
            remove_junction_overlap(polyline, line_overlap);
        }
        out.extend(result.into_iter().filter(|polyline| polyline.len() > 1));
    }

    /// Visit this node and all descendants in depth-first order.
    pub fn visit_nodes(&self, mut visitor: impl FnMut(NodeRef)) {
        self.visit_nodes_recursive(&mut visitor);
    }

    fn visit_nodes_recursive(&self, visitor: &mut impl FnMut(NodeRef)) {
        let self_rc = self
            .self_ref
            .borrow()
            .upgrade()
            .expect("node self reference");
        visitor(self_rc);
        for child in self.children() {
            child.borrow().visit_nodes_recursive(visitor);
        }
    }

    fn convert_to_polylines_recursive(&self, long_line_idx: usize, output: &mut Vec<Vec<Point2>>) {
        let children = self.children();
        if children.is_empty() {
            output[long_line_idx].push(self.location());
            return;
        }

        children[0]
            .borrow()
            .convert_to_polylines_recursive(long_line_idx, output);
        output[long_line_idx].push(self.location());

        for child in children.into_iter().skip(1) {
            output.push(Vec::new());
            let child_line_idx = output.len() - 1;
            child
                .borrow()
                .convert_to_polylines_recursive(child_line_idx, output);
            output[child_line_idx].push(self.location());
        }
    }

    /// Construct and attach a child at `child_loc`.
    pub fn add_child(&self, child_loc: Point2) -> NodeRef {
        self.add_child_node(Self::new(child_loc))
    }

    /// Attach an existing node as a child and return it.
    pub fn add_child_node(&self, child: NodeRef) -> NodeRef {
        self.children.borrow_mut().push(Rc::clone(&child));
        *child.borrow().parent.borrow_mut() = self.self_ref.borrow().clone();
        child.borrow().is_root.set(false);
        child
    }

    /// Copy this node and its complete descendant tree.
    pub fn deep_copy(&self) -> NodeRef {
        let copy =
            Self::new_with_grounding_location(self.location(), self.get_last_grounding_location());
        copy.borrow().is_root.set(self.is_root());
        for child in self.children() {
            let child_copy = child.borrow().deep_copy();
            copy.borrow().add_child_node(child_copy);
        }
        copy
    }

    /// Copy, prune, straighten, and realign this tree for the next layer down.
    ///
    /// Port of canonical `Node::propagateToNextLayer` (`TreeNode.cpp`). The
    /// canonical signature appends into a single `next_trees` vector: any
    /// subtree that `realign` disconnects is pushed as an independent root
    /// first, then the main copy is pushed **only if `realign` returned true**
    /// (i.e. this node is still inside `next_outlines`). This port returns that
    /// same vector in that same order instead of taking an out-parameter; an
    /// empty result means the whole tree fell outside the next layer.
    ///
    /// `outline_locator_resolution` is canonical's `outline_locator.resolution()`
    /// — the Lightning locator cell size, not the supporting radius.
    pub fn propagate_to_next_layer(
        &self,
        next_outlines: &[Point2],
        outline_locator_resolution: i64,
        prune_distance: i64,
        smooth_magnitude: i64,
        max_remove_colinear_dist: i64,
    ) -> Vec<NodeRef> {
        let copy = self.deep_copy();
        // Canonical order is prune -> straighten -> realign, and the *only*
        // gate on emitting the copy is realign. Canonical discards
        // `prune`'s returned distance here.
        copy.borrow().prune(prune_distance);
        copy.borrow()
            .straighten(smooth_magnitude, max_remove_colinear_dist);

        let mut next_trees = Vec::new();
        let kept =
            copy.borrow()
                .realign(next_outlines, outline_locator_resolution, &mut next_trees);
        if kept {
            next_trees.push(copy);
        }
        next_trees
    }

    /// Re-anchor this subtree against the outlines of the layer below.
    ///
    /// Port of canonical `Node::realign` (`TreeNode.cpp`). Returns `true` when
    /// this node lies inside `outlines` and therefore stays attached to its
    /// parent. Descendants that survive but get disconnected are re-emitted
    /// into `rerooted_parts` as independent roots.
    ///
    /// Note the canonical asymmetry, reproduced here: when this node is inside
    /// and had to drop a crossing child it *clears* its own grounding location,
    /// whereas when this node is outside each surviving child inherits the
    /// **dead parent's** position as its grounding location.
    ///
    /// `outlines` is one flat ring in the Pinch 'n Print model (a layer outline
    /// arrives as a single `contour.points` list). If a caller ever passes a
    /// concatenation of several rings, the even-odd test in `point_in_polygon`
    /// will also walk the spurious wrap-around edge from the last point back to
    /// the first; the outline representation is deliberately left alone here
    /// rather than restructured.
    fn realign(
        &self,
        outlines: &[Point2],
        outline_locator_resolution: i64,
        rerooted_parts: &mut Vec<NodeRef>,
    ) -> bool {
        if outlines.is_empty() {
            return false;
        }

        // Canonical passes `outline_locator.resolution() * 2` as the maximum
        // distance from the parent at which a boundary crossing still counts.
        let crossing_max_dist = outline_locator_resolution.saturating_mul(2);
        let my_location = self.location();

        if point_in_polygon(my_location, outlines) {
            let mut reground_me = false;
            let mut retained: Vec<NodeRef> = Vec::new();
            for child in self.children() {
                let mut connect_branch =
                    child
                        .borrow()
                        .realign(outlines, outline_locator_resolution, rerooted_parts);
                if connect_branch {
                    let child_location = child.borrow().location();
                    if line_segment_polygons_intersection(
                        child_location,
                        my_location,
                        outlines,
                        crossing_max_dist,
                    ) {
                        {
                            let child_node = child.borrow();
                            child_node.set_last_grounding_location(None);
                            *child_node.parent.borrow_mut() = Weak::new();
                            child_node.is_root.set(true);
                        }
                        rerooted_parts.push(Rc::clone(&child));
                        reground_me = true;
                        connect_branch = false;
                    }
                }
                if connect_branch {
                    retained.push(child);
                }
            }
            *self.children.borrow_mut() = retained;
            if reground_me {
                self.set_last_grounding_location(None);
            }
            return true;
        }

        // Outside the next outline: this node dies, but any descendant that is
        // still inside is lifted out as an independent root.
        for child in self.children() {
            if child
                .borrow()
                .realign(outlines, outline_locator_resolution, rerooted_parts)
            {
                {
                    let child_node = child.borrow();
                    child_node.set_last_grounding_location(Some(my_location));
                    *child_node.parent.borrow_mut() = Weak::new();
                    child_node.is_root.set(true);
                }
                rerooted_parts.push(child);
            }
        }
        self.children.borrow_mut().clear();
        false
    }

    /// Reverse parent-child edges up to the root and optionally attach this
    /// node to `new_parent` during the recursive unwind.
    pub fn reroot(&self, new_parent: Option<NodeRef>) {
        let self_rc = self
            .self_ref
            .borrow()
            .upgrade()
            .expect("node self reference");
        if !self.is_root() {
            if let Some(old_parent) = self.parent.borrow().upgrade() {
                old_parent.borrow().reroot(Some(Rc::clone(&self_rc)));
                self.children.borrow_mut().push(old_parent);
            }
        }

        if let Some(new_parent) = new_parent {
            self.children
                .borrow_mut()
                .retain(|child| !Rc::ptr_eq(child, &new_parent));
            new_parent
                .borrow()
                .children
                .borrow_mut()
                .push(Rc::clone(&self_rc));
            self.is_root.set(false);
            *self.parent.borrow_mut() = Rc::downgrade(&new_parent);
        } else {
            self.is_root.set(true);
            *self.parent.borrow_mut() = Weak::new();
        }
    }

    /// Prune leaf paths by `distance` and return the greatest consumed length.
    pub fn prune(&self, distance: i64) -> i64 {
        if distance <= 0 {
            return 0;
        }

        let mut max_distance_pruned = 0;
        let mut child_index = 0;
        while child_index < self.children.borrow().len() {
            let child = Rc::clone(&self.children.borrow()[child_index]);
            let distance_pruned_child = child.borrow().prune(distance);

            if distance_pruned_child >= distance {
                max_distance_pruned = max_distance_pruned.max(distance_pruned_child);
                child_index += 1;
                continue;
            }

            let parent_location = self.location();
            let child_location = child.borrow().location();
            let edge_length = distance_between(parent_location, child_location);
            if distance_pruned_child + edge_length <= distance {
                max_distance_pruned =
                    max_distance_pruned.max(distance_pruned_child.saturating_add(edge_length));
                self.children.borrow_mut().remove(child_index);
            } else {
                let remaining = distance - distance_pruned_child;
                let dx = (parent_location.x - child_location.x) as f64;
                let dy = (parent_location.y - child_location.y) as f64;
                let length = dx.hypot(dy);
                child.borrow().set_location(Point2 {
                    x: (child_location.x as f64 + dx / length * remaining as f64) as i64,
                    y: (child_location.y as f64 + dy / length * remaining as f64) as i64,
                });
                max_distance_pruned = max_distance_pruned.max(distance);
                child_index += 1;
            }
        }

        max_distance_pruned
    }

    /// Move nodes toward straight paths without exceeding `magnitude` per move.
    pub fn straighten(&self, magnitude: i64, max_remove_colinear_dist: i64) {
        let max_remove_colinear_dist2 = i128::from(max_remove_colinear_dist)
            .saturating_mul(i128::from(max_remove_colinear_dist));
        self.straighten_recursive(magnitude, self.location(), 0, max_remove_colinear_dist2);
    }

    fn straighten_recursive(
        &self,
        magnitude: i64,
        junction_above: Point2,
        accumulated_dist: i64,
        max_remove_colinear_dist2: i128,
    ) -> RectilinearJunction {
        // Orca ref: Node::straighten and junction_magnitude_factor (TreeNode.cpp).
        const JUNCTION_MAGNITUDE_NUMERATOR: i64 = 3;
        const JUNCTION_MAGNITUDE_DENOMINATOR: i64 = 4;

        let children = self.children();
        if children.len() == 1 {
            let child = &children[0];
            let child_dist = distance_between(self.location(), child.borrow().location());
            let junction_below = child.borrow().straighten_recursive(
                magnitude,
                junction_above,
                accumulated_dist.saturating_add(child_dist),
                max_remove_colinear_dist2,
            );
            let total_dist_to_junction_below = junction_below.total_recti_dist;
            let a = junction_above;
            let b = junction_below.junction_loc;
            if a != b {
                let denominator = total_dist_to_junction_below.max(1);
                let destination = interpolate(a, b, accumulated_dist, denominator);
                self.set_location(move_toward(self.location(), destination, magnitude));
            }

            if let Some(child) = self.children().first().cloned() {
                if let Some(parent) = self.parent.borrow().upgrade() {
                    let parent_location = parent.borrow().location();
                    let child_location = child.borrow().location();
                    if squared_distance(child_location, parent_location) < max_remove_colinear_dist2
                        && distance_to_line_squared(
                            self.location(),
                            parent_location,
                            child_location,
                        ) <= CLOSE_ENOUGH_PNP_UNITS * CLOSE_ENOUGH_PNP_UNITS
                    {
                        *child.borrow().parent.borrow_mut() = Rc::downgrade(&parent);
                        let self_rc = self.self_ref.borrow().upgrade();
                        if let Some(self_rc) = self_rc {
                            let parent_node = parent.borrow();
                            let mut siblings = parent_node.children.borrow_mut();
                            if let Some(sibling) = siblings
                                .iter_mut()
                                .find(|sibling| Rc::ptr_eq(sibling, &self_rc))
                            {
                                *sibling = child;
                            }
                        }
                    }
                }
            }

            junction_below
        } else {
            let mut junction_moving_dir =
                normalized_scaled_difference(self.location(), junction_above, WEIGHT_PNP_UNITS);
            let mut prevent_junction_moving = false;
            for child in children {
                let child_dist = distance_between(self.location(), child.borrow().location());
                let below = child.borrow().straighten_recursive(
                    magnitude,
                    self.location(),
                    child_dist,
                    max_remove_colinear_dist2,
                );
                let child_direction = normalized_scaled_difference(
                    self.location(),
                    below.junction_loc,
                    WEIGHT_PNP_UNITS,
                );
                junction_moving_dir.x = junction_moving_dir.x.saturating_add(child_direction.x);
                junction_moving_dir.y = junction_moving_dir.y.saturating_add(child_direction.y);
                if below.total_recti_dist < magnitude {
                    prevent_junction_moving = true;
                }
            }

            let junction_magnitude = magnitude.saturating_mul(JUNCTION_MAGNITUDE_NUMERATOR)
                / JUNCTION_MAGNITUDE_DENOMINATOR;
            if junction_moving_dir != Point2::default()
                && !self.children.borrow().is_empty()
                && !self.is_root()
                && !prevent_junction_moving
            {
                let direction_length = distance_between(Point2::default(), junction_moving_dir);
                if direction_length > junction_magnitude && direction_length > 0 {
                    junction_moving_dir.x =
                        junction_moving_dir.x * junction_magnitude / direction_length;
                    junction_moving_dir.y =
                        junction_moving_dir.y * junction_magnitude / direction_length;
                }
                let location = self.location();
                self.set_location(Point2 {
                    x: location.x.saturating_add(junction_moving_dir.x),
                    y: location.y.saturating_add(junction_moving_dir.y),
                });
            }
            RectilinearJunction {
                total_recti_dist: accumulated_dist,
                junction_loc: self.location(),
            }
        }
    }
}

struct RectilinearJunction {
    total_recti_dist: i64,
    junction_loc: Point2,
}

fn distance_between(first: Point2, second: Point2) -> i64 {
    let dx = (first.x - second.x) as f64;
    let dy = (first.y - second.y) as f64;
    dx.hypot(dy) as i64
}

fn squared_distance(first: Point2, second: Point2) -> i128 {
    let dx = i128::from(first.x) - i128::from(second.x);
    let dy = i128::from(first.y) - i128::from(second.y);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

fn remove_junction_overlap(polyline: &mut Vec<Point2>, line_overlap: i64) {
    if line_overlap <= 0 || polyline.len() <= 1 {
        return;
    }

    let mut to_be_reduced = line_overlap as f64;
    let mut a = *polyline.last().expect("polyline has a point");
    let mut point_index = polyline.len() - 2;
    loop {
        let b = polyline[point_index];
        let dx = (b.x - a.x) as f64;
        let dy = (b.y - a.y) as f64;
        let segment_length = dx.hypot(dy);
        if segment_length >= to_be_reduced {
            let ratio = to_be_reduced / segment_length;
            let last_index = polyline.len() - 1;
            polyline[last_index] = Point2 {
                x: (a.x as f64 + dx * ratio) as i64,
                y: (a.y as f64 + dy * ratio) as i64,
            };
            return;
        }

        to_be_reduced -= segment_length;
        polyline.pop();
        if polyline.len() <= 1 {
            return;
        }
        a = b;
        point_index -= 1;
    }
}

fn interpolate(start: Point2, end: Point2, numerator: i64, denominator: i64) -> Point2 {
    let numerator = i128::from(numerator);
    let denominator = i128::from(denominator);
    let x = i128::from(start.x).saturating_add(
        (i128::from(end.x) - i128::from(start.x)).saturating_mul(numerator) / denominator,
    );
    let y = i128::from(start.y).saturating_add(
        (i128::from(end.y) - i128::from(start.y)).saturating_mul(numerator) / denominator,
    );
    Point2 {
        x: x as i64,
        y: y as i64,
    }
}

fn move_toward(current: Point2, destination: Point2, magnitude: i64) -> Point2 {
    let dx = i128::from(destination.x) - i128::from(current.x);
    let dy = i128::from(destination.y) - i128::from(current.y);
    let distance2 = dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy));
    let magnitude2 = i128::from(magnitude).saturating_mul(i128::from(magnitude));
    if distance2 <= magnitude2 || magnitude <= 0 {
        return if distance2 <= magnitude2 {
            destination
        } else {
            current
        };
    }

    let distance = (distance2 as f64).sqrt();
    Point2 {
        x: (current.x as f64 + dx as f64 / distance * magnitude as f64) as i64,
        y: (current.y as f64 + dy as f64 / distance * magnitude as f64) as i64,
    }
}

fn normalized_scaled_difference(start: Point2, end: Point2, magnitude: i64) -> Point2 {
    let dx = (end.x - start.x) as f64;
    let dy = (end.y - start.y) as f64;
    let length = dx.hypot(dy);
    if length == 0.0 {
        Point2::default()
    } else {
        Point2 {
            x: (dx / length * magnitude as f64) as i64,
            y: (dy / length * magnitude as f64) as i64,
        }
    }
}

/// Whether segment `a`–`b` crosses `outlines` within `within_max_dist` of `b`.
///
/// Intent-port of canonical `lineSegmentPolygonsIntersection` (`TreeNode.cpp`).
///
/// **Deliberate divergence — the canonical implementation is buggy and the bug
/// is NOT reproduced here.** Canonical's `EdgeGrid` visitor computes a fresh
/// intersection point `ip`, then measures the candidate distance from the
/// *previously stored* `intersection_pt` member rather than from `ip`. That
/// member is uninitialised until the first candidate is accepted, so for the
/// common single-intersection case the accept/reject decision reads
/// indeterminate memory — undefined behaviour, not a stable behaviour worth
/// matching. This port implements the evident intent: find the intersection of
/// `a`–`b` with the outline that is nearest to `b`, and report whether that
/// nearest intersection is closer to `b` than `within_max_dist`.
///
/// The canonical `EdgeGrid` acceleration structure is also skipped; a
/// brute-force scan over the outline's segments is equivalent and the outlines
/// handled here are small.
fn line_segment_polygons_intersection(
    a: Point2,
    b: Point2,
    outlines: &[Point2],
    within_max_dist: i64,
) -> bool {
    if outlines.len() < 2 || within_max_dist <= 0 {
        return false;
    }

    let mut nearest2 = within_max_dist as f64 * within_max_dist as f64;
    let mut found = false;

    for (start, end) in outlines
        .iter()
        .copied()
        .zip(outlines.iter().copied().cycle().skip(1))
        .take(outlines.len())
    {
        let Some((ix, iy)) = segment_segment_intersection(a, b, start, end) else {
            continue;
        };
        let dx = ix - b.x as f64;
        let dy = iy - b.y as f64;
        let dist2 = dx * dx + dy * dy;
        if dist2 < nearest2 {
            nearest2 = dist2;
            found = true;
        }
    }

    found
}

/// Intersection point of segments `a0`–`a1` and `b0`–`b1`, if they cross.
///
/// Mirrors canonical `Geometry::segment_segment_intersection`: parallel and
/// collinear pairs report no intersection.
fn segment_segment_intersection(
    a0: Point2,
    a1: Point2,
    b0: Point2,
    b1: Point2,
) -> Option<(f64, f64)> {
    let rx = (a1.x - a0.x) as f64;
    let ry = (a1.y - a0.y) as f64;
    let sx = (b1.x - b0.x) as f64;
    let sy = (b1.y - b0.y) as f64;
    let denominator = rx * sy - ry * sx;
    if denominator == 0.0 {
        return None;
    }

    let qpx = (b0.x - a0.x) as f64;
    let qpy = (b0.y - a0.y) as f64;
    let t = (qpx * sy - qpy * sx) / denominator;
    let u = (qpx * ry - qpy * rx) / denominator;
    if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
        return None;
    }

    Some((a0.x as f64 + t * rx, a0.y as f64 + t * ry))
}

fn distance_to_line_squared(point: Point2, start: Point2, end: Point2) -> f64 {
    let line_x = (end.x - start.x) as f64;
    let line_y = (end.y - start.y) as f64;
    let point_x = (point.x - start.x) as f64;
    let point_y = (point.y - start.y) as f64;
    let line_length2 = line_x * line_x + line_y * line_y;
    if line_length2 == 0.0 {
        point_x * point_x + point_y * point_y
    } else {
        let cross = point_x * line_y - point_y * line_x;
        cross * cross / line_length2
    }
}
