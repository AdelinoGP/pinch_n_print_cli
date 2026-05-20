//! Paint-region point query helpers.

use std::collections::HashMap;

use rstar::{RTree, RTreeObject, AABB};

use slicer_ir::{
    ExPolygon, PaintRegionIR, PaintSemantic, PaintValue, Point2, Polygon, SemanticRegion,
};

/// Boundary handling mode for point-in-polygon paint queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryInclusion {
    /// Treat points on polygon boundaries as contained.
    Include,
    /// Treat points on polygon boundaries as excluded unless strictly interior.
    Exclude,
}

/// An entry in the paint region R-Tree spatial index.
#[derive(Clone, Debug)]
pub struct PaintRegionRTreeEntry {
    /// Minimum X coordinate (in 100 nm units, cast to f64).
    pub min_x: f64,
    /// Minimum Y coordinate (in 100 nm units, cast to f64).
    pub min_y: f64,
    /// Maximum X coordinate (in 100 nm units, cast to f64).
    pub max_x: f64,
    /// Maximum Y coordinate (in 100 nm units, cast to f64).
    pub max_y: f64,
    /// Index into the SemanticRegion Vec for this (layer_index, semantic) key.
    pub region_index: usize,
}

impl RTreeObject for PaintRegionRTreeEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners([self.min_x, self.min_y], [self.max_x, self.max_y])
    }
}

/// Spatial index companion for [`PaintRegionIR`], mapping each
/// `(layer_index, semantic)` to an `rstar::RTree` for O(log N) region lookups.
///
/// This index is NOT stored on the IR — it is a companion type built alongside
/// the IR and passed through the annotation pipeline separately. When absent,
/// [`point_in_paint_region`] falls back to linear scan.
#[derive(Clone, Debug)]
pub struct PaintRegionRTreeIndex {
    /// Per-layer, per-semantic R-tree mapping bounding boxes to region indices.
    pub trees: HashMap<u32, HashMap<PaintSemantic, RTree<PaintRegionRTreeEntry>>>,
}

/// Deterministic point-query failures for paint-region lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintRegionQueryError {
    /// Equal-precedence conflicting paint values were encountered.
    DeterministicConflict,
}

/// Queries the paint value for a single point on one layer and semantic.
///
/// When `rtree_index` is `Some`, uses spatial index for O(log N) candidate
/// selection. When `None`, falls back to linear scan over all regions.
pub fn point_in_paint_region(
    paint_regions: &PaintRegionIR,
    layer_index: u32,
    semantic: &PaintSemantic,
    point: Point2,
    boundary_inclusion: BoundaryInclusion,
    rtree_index: Option<&PaintRegionRTreeIndex>,
) -> Result<Option<PaintValue>, PaintRegionQueryError> {
    if let Some(rtree_index) = rtree_index {
        let tree = match rtree_index.trees.get(&layer_index) {
            Some(layer_trees) => match layer_trees.get(semantic) {
                Some(tree) => tree,
                None => return Ok(None),
            },
            None => return Ok(None),
        };

        if tree.size() == 0 {
            return Ok(None);
        }

        let point_f = [point.x as f64, point.y as f64];
        let query_aabb = AABB::from_corners(point_f, point_f);

        let candidate_indices: Vec<usize> = tree
            .locate_in_envelope_intersecting(&query_aabb)
            .map(|entry| entry.region_index)
            .collect();

        if candidate_indices.is_empty() {
            return Ok(None);
        }

        let regions = paint_regions.get(layer_index, semantic);

        let mut winner: Option<(&PaintValue, u64)> = None;

        for &candidate_index in &candidate_indices {
            let region = &regions[candidate_index];
            if !semantic_region_contains_point(region, point, boundary_inclusion) {
                continue;
            }

            match winner {
                None => winner = Some((&region.value, region.paint_order)),
                Some((_, current_order)) if region.paint_order > current_order => {
                    winner = Some((&region.value, region.paint_order));
                }
                Some((current_value, current_order))
                    if region.paint_order == current_order
                        && matches!(semantic, PaintSemantic::Custom(_))
                        && current_value != &region.value =>
                {
                    return Err(PaintRegionQueryError::DeterministicConflict);
                }
                Some(_) => {}
            }
        }

        Ok(winner.map(|(value, _)| value.clone()))
    } else {
        let regions = paint_regions.get(layer_index, semantic);
        point_in_paint_regions(regions, semantic, point, boundary_inclusion)
    }
}

/// Queries the paint value for a point against a pre-fetched slice of semantic
/// regions (avoiding redundant `PaintRegionIR::get` lookups).
pub fn point_in_paint_regions(
    regions: &[SemanticRegion],
    semantic: &PaintSemantic,
    point: Point2,
    boundary_inclusion: BoundaryInclusion,
) -> Result<Option<PaintValue>, PaintRegionQueryError> {
    let mut winner: Option<(&PaintValue, u64)> = None;

    for region in regions {
        if !semantic_region_contains_point(region, point, boundary_inclusion) {
            continue;
        }

        match winner {
            None => winner = Some((&region.value, region.paint_order)),
            Some((_, current_order)) if region.paint_order > current_order => {
                winner = Some((&region.value, region.paint_order));
            }
            Some((current_value, current_order))
                if region.paint_order == current_order
                    && matches!(semantic, PaintSemantic::Custom(_))
                    && current_value != &region.value =>
            {
                return Err(PaintRegionQueryError::DeterministicConflict);
            }
            Some((_, current_order)) if region.paint_order < current_order => {
                // Regions are sorted descending by paint_order (Step 2 harvest).
                // No remaining region can override the current winner.
                break;
            }
            Some(_) => {}
        }
    }

    Ok(winner.map(|(value, _)| value.clone()))
}

fn semantic_region_contains_point(
    region: &SemanticRegion,
    point: Point2,
    boundary_inclusion: BoundaryInclusion,
) -> bool {
    if let Some(ref aabb) = region.aabb {
        if !aabb.contains_point(point) {
            return false;
        }
    }
    region
        .polygons
        .iter()
        .any(|polygon| ex_polygon_contains_point(polygon, point, boundary_inclusion))
}

/// Test whether a `Point2` is inside an [`ExPolygon`], respecting boundary inclusion rules.
pub fn ex_polygon_contains_point(
    polygon: &ExPolygon,
    point: Point2,
    boundary_inclusion: BoundaryInclusion,
) -> bool {
    match ring_contains_point(&polygon.contour, point) {
        RingContainment::Outside => return false,
        RingContainment::Boundary if matches!(boundary_inclusion, BoundaryInclusion::Exclude) => {
            return false;
        }
        RingContainment::Inside | RingContainment::Boundary => {}
    }

    for hole in &polygon.holes {
        match ring_contains_point(hole, point) {
            RingContainment::Inside => return false,
            RingContainment::Boundary
                if matches!(boundary_inclusion, BoundaryInclusion::Exclude) =>
            {
                return false;
            }
            RingContainment::Outside | RingContainment::Boundary => {}
        }
    }

    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RingContainment {
    Outside,
    Inside,
    Boundary,
}

fn ring_contains_point(ring: &Polygon, point: Point2) -> RingContainment {
    if ring.points.len() < 2 {
        return RingContainment::Outside;
    }

    let mut inside = false;

    for index in 0..ring.points.len() {
        let start = ring.points[index];
        let end = ring.points[(index + 1) % ring.points.len()];

        if point_on_segment(point, start, end) {
            return RingContainment::Boundary;
        }

        if edge_crosses_horizontal_ray(point, start, end) {
            inside = !inside;
        }
    }

    if inside {
        RingContainment::Inside
    } else {
        RingContainment::Outside
    }
}

fn point_on_segment(point: Point2, start: Point2, end: Point2) -> bool {
    let cross = cross_product(start, end, point);
    if cross != 0 {
        return false;
    }

    let min_x = start.x.min(end.x);
    let max_x = start.x.max(end.x);
    let min_y = start.y.min(end.y);
    let max_y = start.y.max(end.y);

    point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
}

fn edge_crosses_horizontal_ray(point: Point2, start: Point2, end: Point2) -> bool {
    let start_above = start.y > point.y;
    let end_above = end.y > point.y;

    if start_above == end_above {
        return false;
    }

    let orient = cross_product(start, end, point);
    if orient == 0 {
        return false;
    }

    if end.y > start.y {
        orient > 0
    } else {
        orient < 0
    }
}

fn cross_product(start: Point2, end: Point2, point: Point2) -> i128 {
    let edge_x = i128::from(end.x - start.x);
    let edge_y = i128::from(end.y - start.y);
    let point_x = i128::from(point.x - start.x);
    let point_y = i128::from(point.y - start.y);

    edge_x * point_y - edge_y * point_x
}
