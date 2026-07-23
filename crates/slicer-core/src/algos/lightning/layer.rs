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

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use slicer_ir::{slice_ir::BoundingBox2, ExPolygon, Point2, Polygon};

use crate::geometry::closest_point_on_segment;
use crate::polygon_ops::clip_polylines;

use super::distance_field::DistanceField;
use super::tree_node::{to_grid_point, Node, NodeRef};

const TREE_CONNECTING_IGNORE_OFFSET: i64 = 1;
const TREE_NODE_LOCATOR_CELL_SIZE: i64 = 4;

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
        current_overhang: &[Point2],
        current_outlines: &[Point2],
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
            let Some(grounding) = self.get_best_grounding_location(
                unsupported_location,
                current_outlines,
                outlines_bbox,
                supporting_radius,
                wall_supporting_radius,
                cancel,
            ) else {
                distance_field.update(unsupported_location, unsupported_location);
                continue;
            };
            let grounding_point = grounding.point();
            self.attach(unsupported_location, grounding);
            distance_field.update(grounding_point, unsupported_location);
        }
    }

    // Orca ref: Layer.cpp::getBestGroundingLocation.
    pub(crate) fn get_best_grounding_location(
        &self,
        unsupported_location: Point2,
        outline: &[Point2],
        bbox: BoundingBox2,
        supporting_radius: i64,
        wall_supporting_radius: i64,
        cancel: &dyn Fn(),
    ) -> Option<GroundingLocation> {
        let mut outline_locator = BTreeMap::<(i32, i32), Vec<Point2>>::new();
        for point in outline {
            outline_locator
                .entry(to_grid_point(*point, bbox, TREE_NODE_LOCATOR_CELL_SIZE))
                .or_default()
                .push(*point);
        }
        if let Some((closest, _)) = closest_outline_point(unsupported_location, outline) {
            outline_locator
                .entry(to_grid_point(closest, bbox, TREE_NODE_LOCATOR_CELL_SIZE))
                .or_default()
                .push(closest);
        }

        let tree_connecting_ignore_width =
            wall_supporting_radius.saturating_sub(TREE_CONNECTING_IGNORE_OFFSET);
        let mut best_boundary: Option<(i64, GroundingLocation)> = None;
        let mut excluded_boundary_found = false;
        for candidates in outline_locator.values() {
            cancel();
            for candidate in candidates {
                let candidate_distance = distance_between(unsupported_location, *candidate);
                if candidate_distance <= tree_connecting_ignore_width {
                    excluded_boundary_found = true;
                    continue;
                }
                if best_boundary
                    .as_ref()
                    .is_none_or(|(distance, _)| candidate_distance < *distance)
                {
                    best_boundary =
                        Some((candidate_distance, GroundingLocation::Boundary(*candidate)));
                }
            }
        }

        let mut best = (!excluded_boundary_found)
            .then_some(best_boundary)
            .flatten();
        let mut tree_cells: Vec<_> = self.tree_node_locator.keys().copied().collect();
        tree_cells.sort_unstable();
        for cell in tree_cells {
            cancel();
            let Some(candidates) = self.tree_node_locator.get(&cell) else {
                continue;
            };
            for candidate in candidates {
                let location = candidate.borrow().location();
                let Some((_, wall_distance)) = closest_outline_point(location, outline) else {
                    continue;
                };
                if wall_distance <= tree_connecting_ignore_width {
                    continue;
                }
                let candidate_distance = candidate
                    .borrow()
                    .get_weighted_distance(unsupported_location, supporting_radius);
                if best
                    .as_ref()
                    .is_none_or(|(distance, _)| candidate_distance <= *distance)
                {
                    best = Some((
                        candidate_distance,
                        GroundingLocation::Tree(Rc::clone(candidate)),
                    ));
                }
            }
        }

        best.map(|(_, grounding)| grounding)
    }

    /// Reconnect roots to the nearest outline or an existing compatible tree.
    pub fn reconnect_roots(
        &mut self,
        to_be_reconnected: Vec<NodeRef>,
        current_outlines: &[Point2],
        supporting_radius: i64,
        wall_supporting_radius: i64,
    ) {
        for root in to_be_reconnected {
            let Some(old_root_index) = self
                .tree_roots
                .iter()
                .position(|candidate| Rc::ptr_eq(candidate, &root))
            else {
                continue;
            };

            let root_location = root.borrow().location();
            let grounding_target = root
                .borrow()
                .get_last_grounding_location()
                .unwrap_or(root_location);
            let Some((boundary_location, _)) =
                closest_outline_point(grounding_target, current_outlines)
            else {
                continue;
            };
            let boundary_distance = distance_between(root_location, boundary_location);
            if boundary_location == root_location {
                continue;
            }

            let tree_connecting_ignore_width =
                wall_supporting_radius.saturating_sub(TREE_CONNECTING_IGNORE_OFFSET);
            let grounding = if boundary_distance >= tree_connecting_ignore_width {
                self.closest_compatible_tree(
                    &root,
                    root_location,
                    supporting_radius,
                    boundary_distance,
                )
                .map(GroundingLocation::Tree)
                .unwrap_or(GroundingLocation::Boundary(boundary_location))
            } else {
                GroundingLocation::Boundary(boundary_location)
            };

            let attach_target = grounding.point();
            let attach_node = root.borrow().closest_node(attach_target);
            attach_node.borrow().reroot(None);

            match grounding {
                GroundingLocation::Boundary(boundary) => {
                    let new_root = Node::new_with_grounding_location(boundary, Some(boundary));
                    new_root.borrow().add_child_node(attach_node);
                    self.tree_roots[old_root_index] = new_root;
                }
                GroundingLocation::Tree(tree_node) => {
                    tree_node.borrow().add_child_node(attach_node);
                    self.tree_roots.remove(old_root_index);
                }
            }
        }
    }

    /// Convert all trees into clipped, overlap-reduced polylines.
    pub fn convert_to_lines(
        &self,
        limit_to_outline: &[Point2],
        line_overlap: i64,
    ) -> Vec<Vec<Point2>> {
        if self.tree_roots.is_empty() || limit_to_outline.len() < 3 {
            return Vec::new();
        }

        let mut polylines = Vec::new();
        for tree in &self.tree_roots {
            tree.borrow()
                .convert_to_polylines(&mut polylines, line_overlap);
        }

        let mut contour = limit_to_outline.to_vec();
        if contour.last() != contour.first() {
            contour.push(contour[0]);
        }
        let limit = ExPolygon {
            contour: Polygon { points: contour },
            holes: Vec::new(),
        };
        clip_polylines(&polylines, &[limit])
    }

    fn attach(&mut self, unsupported_location: Point2, grounding: GroundingLocation) -> NodeRef {
        match grounding {
            GroundingLocation::Boundary(boundary_location) => {
                let new_root =
                    Node::new_with_grounding_location(boundary_location, Some(boundary_location));
                let new_child = new_root.borrow().add_child(unsupported_location);
                self.tree_roots.push(new_root);
                new_child
            }
            GroundingLocation::Tree(tree_node) => {
                tree_node.borrow().add_child(unsupported_location)
            }
        }
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

    fn closest_compatible_tree(
        &self,
        excluded_root: &NodeRef,
        target: Point2,
        supporting_radius: i64,
        boundary_distance: i64,
    ) -> Option<NodeRef> {
        let mut best: Option<NodeRef> = None;
        let mut best_distance = boundary_distance;

        for candidate_root in &self.tree_roots {
            if excluded_root
                .borrow()
                .has_offspring(Rc::clone(candidate_root))
            {
                continue;
            }

            candidate_root.borrow().visit_nodes(|candidate| {
                if excluded_root.borrow().has_offspring(Rc::clone(&candidate)) {
                    return;
                }
                let candidate_distance = candidate
                    .borrow()
                    .get_weighted_distance(target, supporting_radius);
                if candidate_distance < best_distance {
                    best_distance = candidate_distance;
                    best = Some(candidate);
                }
            });
        }

        best
    }
}

fn bounding_box(points: &[Point2]) -> BoundingBox2 {
    let Some(first) = points.first().copied() else {
        return BoundingBox2::default();
    };
    let mut min = first;
    let mut max = first;
    for point in points.iter().skip(1) {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    BoundingBox2 { min, max }
}

fn closest_outline_point(target: Point2, outline: &[Point2]) -> Option<(Point2, i64)> {
    if outline.is_empty() {
        return None;
    }
    if outline.len() == 1 {
        return Some((outline[0], distance_between(target, outline[0])));
    }

    let mut best = None;
    for (start, end) in outline
        .iter()
        .copied()
        .zip(outline.iter().copied().cycle().skip(1))
        .take(outline.len())
    {
        let candidate = closest_point_on_segment(target, start, end);
        if best
            .as_ref()
            .is_none_or(|(_, distance)| candidate.distance_sq < *distance)
        {
            best = Some((candidate.point, candidate.distance_sq));
        }
    }

    best.map(|(point, distance_sq)| (point, distance_sq.sqrt() as i64))
}

fn distance_between(first: Point2, second: Point2) -> i64 {
    let dx = (first.x - second.x) as f64;
    let dy = (first.y - second.y) as f64;
    dx.hypot(dy) as i64
}
