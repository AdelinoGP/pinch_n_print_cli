// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

//! Per-layer tree seeding, reconnection, and line conversion for Lightning infill.

use std::collections::HashMap;
use std::rc::Rc;

use slicer_ir::{slice_ir::BoundingBox2, ExPolygon, Point2, Polygon};

use slicer_core::geometry::closest_point_on_segment;
use slicer_core::polygon_ops::clip_polylines;

use super::distance_field::DistanceField;
use super::tree_node::{to_grid_point, Node, NodeRef};
use super::Polygons;

const TREE_CONNECTING_IGNORE_OFFSET: i64 = 1;
// Canonical TreeNode.hpp::locator_cell_size is scaled(4.), i.e. 4 mm.
const TREE_NODE_LOCATOR_CELL_SIZE: i64 = 40_000;

pub(crate) enum GroundingLocation {
    Boundary(Point2),
    Tree(NodeRef),
}

impl GroundingLocation {
    fn point(&self) -> Point2 {
        match self {
            Self::Boundary(point) => *point,
            Self::Tree(node) => node.borrow().location(),
        }
    }
}

/// The Lightning trees generated for one layer.
#[derive(Default)]
pub struct Layer {
    /// Roots of the trees owned by this layer.
    pub tree_roots: Vec<NodeRef>,
    tree_node_locator: HashMap<(i32, i32), Vec<NodeRef>>,
}

impl Layer {
    /// Construct a layer from the roots propagated from the layer above.
    pub fn new(tree_roots: Vec<NodeRef>) -> Self {
        Self {
            tree_roots,
            tree_node_locator: HashMap::new(),
        }
    }

    /// Seed trees for the unsupported cells in this layer.
    pub fn generate_new_trees(
        &mut self,
        current_overhang: &Polygons,
        current_outlines: &Polygons,
        supporting_radius: i64,
        wall_supporting_radius: i64,
        cancel: &dyn Fn(),
    ) {
        let outlines_bbox = bounding_box(current_outlines);
        let mut distance_field = DistanceField::new(
            supporting_radius,
            current_outlines,
            outlines_bbox,
            current_overhang,
        );
        self.rebuild_tree_node_locator(outlines_bbox);
        cancel();

        while let Some(unsupported_location) = distance_field.try_get_next_point() {
            cancel();
            let grounding = self.get_best_grounding_location(
                unsupported_location,
                current_outlines,
                outlines_bbox,
                supporting_radius,
                wall_supporting_radius,
                cancel,
                None,
            );
            let grounding_point = grounding.point();
            self.attach(unsupported_location, grounding, outlines_bbox);
            distance_field.update(grounding_point, unsupported_location);
        }
    }

    /// Port of canonical `Layer::getBestGroundingLocation` (`Layer.cpp`).
    ///
    /// Canonical scans the outline first for the nearest boundary point, and
    /// uses that plain distance as the candidate score. The tree search runs
    /// only when the boundary score reaches `wall_supporting_radius`; its
    /// search box is a square of side `2 * boundary_distance` around the
    /// unsupported point. A tree candidate must be *strictly* closer than the
    /// boundary candidate (ties keep the boundary), with a cell-address
    /// tie-break across cells.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn get_best_grounding_location(
        &self,
        unsupported_location: Point2,
        outlines: &Polygons,
        bbox: BoundingBox2,
        supporting_radius: i64,
        wall_supporting_radius: i64,
        cancel: &dyn Fn(),
        exclude: Option<&NodeRef>,
    ) -> GroundingLocation {
        let Some((boundary_location, boundary_distance)) =
            closest_outline_point(unsupported_location, outlines)
        else {
            // Canonical's scan leaves `node_location` at the origin when no ring
            // has three points. Returning that boundary avoids an unbounded grid
            // walk over `i64::MAX`.
            return GroundingLocation::Boundary(Point2::default());
        };
        if boundary_distance < wall_supporting_radius {
            return GroundingLocation::Boundary(boundary_location);
        }

        let radius = boundary_distance.max(0);
        let low = to_grid_point(
            Point2 {
                x: unsupported_location.x.saturating_sub(radius),
                y: unsupported_location.y.saturating_sub(radius),
            },
            bbox,
            TREE_NODE_LOCATOR_CELL_SIZE,
        );
        let high = to_grid_point(
            Point2 {
                x: unsupported_location
                    .x
                    .saturating_add(radius)
                    .saturating_add(TREE_NODE_LOCATOR_CELL_SIZE),
                y: unsupported_location
                    .y
                    .saturating_add(radius)
                    .saturating_add(TREE_NODE_LOCATOR_CELL_SIZE),
            },
            bbox,
            TREE_NODE_LOCATOR_CELL_SIZE,
        );

        let mut best = boundary_distance;
        let mut best_node: Option<NodeRef> = None;
        let mut best_addr = (i32::MIN, i32::MIN);
        for iy in low.1..high.1 {
            cancel();
            for ix in low.0..high.0 {
                let mut local = best;
                let mut local_node: Option<NodeRef> = None;
                if let Some(candidates) = self.tree_node_locator.get(&(ix, iy)) {
                    for candidate in candidates {
                        if exclude.is_some_and(|root| {
                            Rc::ptr_eq(root, candidate)
                                || root.borrow().has_offspring(Rc::clone(candidate))
                        }) {
                            continue;
                        }
                        if polygon_collides_with_line_segment(
                            unsupported_location,
                            candidate.borrow().location(),
                            outlines,
                        ) {
                            continue;
                        }
                        let candidate_distance = candidate
                            .borrow()
                            .get_weighted_distance(unsupported_location, supporting_radius);
                        if candidate_distance < local {
                            local = candidate_distance;
                            local_node = Some(Rc::clone(candidate));
                        }
                    }
                }
                if local < best || (local == best && (iy, ix) < best_addr) {
                    best = local;
                    best_node = local_node;
                    best_addr = (iy, ix);
                }
            }
        }

        best_node.map_or(
            GroundingLocation::Boundary(boundary_location),
            GroundingLocation::Tree,
        )
    }

    /// Reconnect roots to the nearest outline or an existing compatible tree.
    ///
    /// Port of canonical `Layer::reconnectRoots` (`Layer.cpp`). The first
    /// branch is the bounded intersection toward the root's last grounding
    /// location: when the segment from the root to where it was last grounded
    /// crosses the current outline within `resolution * 2`, the crossing point
    /// becomes the new root. Only when that fails does the root search for a
    /// boundary or tree grounding.
    pub fn reconnect_roots(
        &mut self,
        to_be_reconnected: Vec<NodeRef>,
        current_outlines: &Polygons,
        supporting_radius: i64,
        wall_supporting_radius: i64,
    ) {
        let bbox = bounding_box(current_outlines);
        for root in to_be_reconnected {
            let Some(old_root_index) = self
                .tree_roots
                .iter()
                .position(|candidate| Rc::ptr_eq(candidate, &root))
            else {
                continue;
            };

            let root_location = root.borrow().location();
            if let Some(grounding_target) = root.borrow().get_last_grounding_location() {
                if grounding_target != root_location {
                    if let Some(new_root_point) = line_segment_polygons_intersection_point(
                        root_location,
                        grounding_target,
                        current_outlines,
                        TREE_NODE_LOCATOR_CELL_SIZE * 2,
                    ) {
                        let new_root =
                            Node::new_with_grounding_location(new_root_point, Some(new_root_point));
                        root.borrow().add_child_node(Rc::clone(&new_root));
                        new_root.borrow().reroot(None);
                        self.index_tree_node(&new_root, bbox);
                        self.tree_roots[old_root_index] = new_root;
                        continue;
                    }
                }
            }

            let tree_connecting_ignore_width =
                wall_supporting_radius.saturating_sub(TREE_CONNECTING_IGNORE_OFFSET);
            let grounding = self.get_best_grounding_location(
                root_location,
                current_outlines,
                bbox,
                supporting_radius,
                tree_connecting_ignore_width,
                &|| {},
                Some(&root),
            );

            let attach_target = grounding.point();
            let attach_node = root.borrow().closest_node(attach_target);
            attach_node.borrow().reroot(None);

            match grounding {
                GroundingLocation::Boundary(boundary) => {
                    if boundary == root_location {
                        continue;
                    }
                    let new_root = Node::new_with_grounding_location(boundary, Some(boundary));
                    new_root.borrow().add_child_node(attach_node);
                    self.index_tree_node(&new_root, bbox);
                    self.tree_roots[old_root_index] = new_root;
                }
                GroundingLocation::Tree(tree_node) => {
                    tree_node.borrow().add_child_node(attach_node);
                    // Canonical move-back-and-pop: the old root is replaced by
                    // the last root, then the last slot is removed.
                    self.tree_roots[old_root_index] =
                        Rc::clone(self.tree_roots.last().expect("root exists"));
                    self.tree_roots.pop();
                }
            }
        }
    }

    /// Convert all trees into clipped, overlap-reduced polylines.
    ///
    /// Port of canonical `Layer::convertToLines` (`Layer.cpp`). Canonical
    /// passes `to_polygons(expolygon)`, i.e. the contour and every hole as a
    /// flat ring set, to `intersection_pl`; `intersection_pl` treats each ring
    /// as a filled polygon, so a hole does **not** clip here — the final
    /// `intersection_pl` against the ExPolygon (in the fill driver) is what
    /// removes hole interiors.
    pub fn convert_to_lines(
        &self,
        limit_to_outline: &Polygons,
        line_overlap: i64,
    ) -> Vec<Vec<Point2>> {
        if self.tree_roots.is_empty() || limit_to_outline.iter().all(|ring| ring.len() < 3) {
            return Vec::new();
        }

        let mut polylines = Vec::new();
        for tree in &self.tree_roots {
            tree.borrow()
                .convert_to_polylines(&mut polylines, line_overlap);
        }

        let limit: Vec<ExPolygon> = limit_to_outline
            .iter()
            .filter(|ring| ring.len() >= 3)
            .map(|ring| {
                let mut contour = ring.clone();
                if contour.last() != contour.first() {
                    contour.push(contour[0]);
                }
                ExPolygon {
                    contour: Polygon { points: contour },
                    holes: Vec::new(),
                }
            })
            .collect();
        clip_polylines(&polylines, &limit)
    }

    fn attach(
        &mut self,
        unsupported_location: Point2,
        grounding: GroundingLocation,
        bbox: BoundingBox2,
    ) -> NodeRef {
        let new_child = match grounding {
            GroundingLocation::Boundary(boundary_location) => {
                let new_root =
                    Node::new_with_grounding_location(boundary_location, Some(boundary_location));
                let new_child = new_root.borrow().add_child(unsupported_location);
                self.index_tree_node(&new_root, bbox);
                self.tree_roots.push(new_root);
                new_child
            }
            GroundingLocation::Tree(tree_node) => {
                tree_node.borrow().add_child(unsupported_location)
            }
        };
        self.index_tree_node(&new_child, bbox);
        new_child
    }

    fn rebuild_tree_node_locator(&mut self, bbox: BoundingBox2) {
        self.tree_node_locator.clear();
        let roots = self.tree_roots.clone();
        for root in roots {
            root.borrow()
                .visit_nodes(|node| self.index_tree_node(&node, bbox));
        }
    }

    fn index_tree_node(&mut self, node: &NodeRef, bbox: BoundingBox2) {
        self.tree_node_locator
            .entry(to_grid_point(
                node.borrow().location(),
                bbox,
                TREE_NODE_LOCATOR_CELL_SIZE,
            ))
            .or_default()
            .push(Rc::clone(node));
    }
}

fn bounding_box(polygons: &Polygons) -> BoundingBox2 {
    let Some(first) = polygons.iter().flatten().next().copied() else {
        return BoundingBox2::default();
    };
    let mut min = first;
    let mut max = first;
    for point in polygons.iter().flatten().skip(1) {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    BoundingBox2 { min, max }
}

/// Neared outline point and its truncated Euclidean distance.
///
/// Canonical skips rings with two or fewer points, includes the closing edge of
/// every longer ring, and keeps the first candidate on a strict-distance tie.
fn closest_outline_point(target: Point2, polygons: &Polygons) -> Option<(Point2, i64)> {
    let mut best: Option<(Point2, f64)> = None;
    for ring in polygons {
        if ring.len() < 3 {
            continue;
        }
        for (start, end) in ring
            .iter()
            .copied()
            .zip(ring.iter().copied().cycle().skip(1))
            .take(ring.len())
        {
            let candidate = closest_point_on_segment(target, start, end);
            if best.is_none_or(|(_, distance)| candidate.distance_sq < distance) {
                best = Some((candidate.point, candidate.distance_sq));
            }
        }
    }

    best.map(|(point, distance_sq)| (point, distance_sq.sqrt() as i64))
}

/// Whether segment `a`–`b` intersects any edge of the flat outline rings.
///
/// Canonical `polygonCollidesWithLineSegment` (`Layer.cpp`) uses `EdgeGrid` and
/// `Geometry::segments_intersect`, an inclusive predicate: a touch at an
/// endpoint and a collinear overlap both count. Hole rings are ordinary edges
/// in the flat set, so hole edges collide too.
fn polygon_collides_with_line_segment(a: Point2, b: Point2, polygons: &Polygons) -> bool {
    polygons.iter().any(|ring| {
        ring.len() >= 3
            && ring
                .iter()
                .copied()
                .zip(ring.iter().copied().cycle().skip(1))
                .take(ring.len())
                .any(|(start, end)| segments_intersect(a, b, start, end))
    })
}

/// Nearest intersection of segment `a`–`b` with the outline, if the nearest
/// intersection is within `within_max` of `b`.
///
/// Intent-port of `lineSegmentPolygonsIntersection` (`Layer.cpp`): canonical
/// compares the *previous* intersection point against `b` (an uninitialised
/// first read), which is undefined behaviour rather than a stable behaviour.
/// This returns the nearest intersection to `b` and applies the distance gate
/// to it, which is the evident intent. The `EdgeGrid` acceleration is replaced
/// by a brute-force ring scan.
fn line_segment_polygons_intersection_point(
    a: Point2,
    b: Point2,
    polygons: &Polygons,
    within_max: i64,
) -> Option<Point2> {
    if within_max <= 0 {
        return None;
    }
    let mut nearest: Option<(f64, Point2)> = None;
    for ring in polygons {
        if ring.len() < 3 {
            continue;
        }
        for (start, end) in ring
            .iter()
            .copied()
            .zip(ring.iter().copied().cycle().skip(1))
            .take(ring.len())
        {
            let Some(point) = segment_intersection_point(a, b, start, end) else {
                continue;
            };
            let dx = point.x as f64 - b.x as f64;
            let dy = point.y as f64 - b.y as f64;
            let distance = dx.hypot(dy);
            if nearest.is_none_or(|(best, _)| distance < best) {
                nearest = Some((distance, point));
            }
        }
    }
    nearest
        .filter(|(distance, _)| *distance <= within_max as f64)
        .map(|(_, point)| point)
}

fn segments_intersect(a0: Point2, a1: Point2, b0: Point2, b1: Point2) -> bool {
    if segment_intersection_point(a0, a1, b0, b1).is_some() {
        return true;
    }
    // Collinear overlap: `segment_intersection_point` reports no unique point
    // for parallel segments, so test the endpoints' incidence explicitly.
    (orientation(a0, a1, b0) == 0 && point_on_segment(b0, a0, a1))
        || (orientation(a0, a1, b1) == 0 && point_on_segment(b1, a0, a1))
        || (orientation(b0, b1, a0) == 0 && point_on_segment(a0, b0, b1))
        || (orientation(b0, b1, a1) == 0 && point_on_segment(a1, b0, b1))
}

/// Intersection point of two segments, if they cross or touch.
fn segment_intersection_point(a0: Point2, a1: Point2, b0: Point2, b1: Point2) -> Option<Point2> {
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

    Some(Point2 {
        x: (a0.x as f64 + t * rx) as i64,
        y: (a0.y as f64 + t * ry) as i64,
    })
}

fn orientation(a: Point2, b: Point2, c: Point2) -> i128 {
    (i128::from(b.x) - i128::from(a.x)) * (i128::from(c.y) - i128::from(a.y))
        - (i128::from(b.y) - i128::from(a.y)) * (i128::from(c.x) - i128::from(a.x))
}

fn point_on_segment(point: Point2, start: Point2, end: Point2) -> bool {
    orientation(start, end, point) == 0
        && point.x >= start.x.min(end.x)
        && point.x <= start.x.max(end.x)
        && point.y >= start.y.min(end.y)
        && point.y <= start.y.max(end.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_seeds_attach_to_newly_indexed_nodes() {
        let outline: Polygons = vec![vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 100, y: 0 },
            Point2 { x: 100, y: 100 },
            Point2 { x: 0, y: 100 },
        ]];
        let bbox = bounding_box(&outline);
        let mut layer = Layer::default();
        layer.rebuild_tree_node_locator(bbox);
        let first = Point2 { x: 50, y: 50 };
        let grounding =
            layer.get_best_grounding_location(first, &outline, bbox, 6, 1, &|| {}, None);
        assert!(matches!(grounding, GroundingLocation::Boundary(_)));
        let first_node = layer.attach(first, grounding, bbox);
        let second = Point2 { x: 57, y: 50 };
        let grounding =
            layer.get_best_grounding_location(second, &outline, bbox, 6, 1, &|| {}, None);
        let GroundingLocation::Tree(parent) = &grounding else {
            panic!("the second seed must find the first seed's new node");
        };
        assert!(Rc::ptr_eq(parent, &first_node));
        let second_node = layer.attach(second, grounding, bbox);
        assert_eq!(layer.tree_roots.len(), 1);
        assert_eq!(first_node.borrow().children().len(), 1);
        assert!(Rc::ptr_eq(&first_node.borrow().children()[0], &second_node));
        let cell = to_grid_point(first, bbox, TREE_NODE_LOCATOR_CELL_SIZE);
        assert_eq!(cell, (0, 0)); // Both seeds and the root share the canonical 4 mm cell.
        assert_eq!(layer.tree_node_locator[&cell].len(), 3);
    }
}
