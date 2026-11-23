//! Independent reference predicates for property-test domains.

use std::collections::{BTreeSet, VecDeque};

use slicer_ir::{ExPolygon, IndexedTriangleSet, LayerCollectionIR, Point2, Polygon};

use super::domains::{BoundaryRule, DomainError, GeometryCase, GeometryDomain, SmallGraph};

mod exact;
mod mesh;

/// The exact relation between two closed line segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentRelation {
    /// The segments have no point in common.
    Disjoint,
    /// Both segments cross at an interior point.
    ProperCrossing,
    /// The segments meet at exactly one endpoint of at least one segment.
    EndpointTouch,
    /// Collinear segments share a positive-length interval.
    CollinearOverlap,
}

/// A point's location relative to a polygon region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointLocation {
    /// The point is not in the region or on any ring.
    Outside,
    /// The point lies on any outer or hole ring.
    Boundary,
    /// The point is strictly inside a component and outside its holes.
    Inside,
}

/// Sign of `(b - a) × (c - a)` in canonical integer units.
///
/// All widened arithmetic is checked; extreme coordinates can overflow.
pub fn orientation(a: Point2, b: Point2, c: Point2) -> Result<i8, DomainError> {
    exact::checked_orientation(a, b, c)
}

/// Signed double area in squared canonical units, using checked arithmetic.
///
/// Empty and short rings are summed as given; use [`validate_ring`] for validity.
pub fn signed_area2(ring: &Polygon) -> Result<i128, DomainError> {
    exact::checked_signed_area2(ring)
}

/// Classify two closed segments, including point segments and collinear overlap.
pub fn segment_relation(a: [Point2; 2], b: [Point2; 2]) -> Result<SegmentRelation, DomainError> {
    Ok(exact::exact_segment_relation(a, b))
}

/// Check distinct vertices, nonzero area, and simple cyclic edge relationships.
///
/// A closing vertex must not be repeated; adjacent edges share only their common
/// endpoint. Checked area overflow is returned rather than wrapped.
pub fn validate_ring(ring: &Polygon) -> Result<(), DomainError> {
    exact::validate_ring(ring)
}

/// Classify a point against all outer and hole boundaries before their interiors.
pub fn point_in_region(
    point: Point2,
    polygons: &[ExPolygon],
) -> Result<PointLocation, DomainError> {
    Ok(exact::point_in_region(point, polygons))
}

/// Test the entire closed segment, partitioned at every exact boundary meeting.
///
/// Strict containment excludes every boundary point; inclusive containment
/// permits boundary points but never an outside interval.
pub fn segment_in_region(
    segment: [Point2; 2],
    polygons: &[ExPolygon],
    boundary: BoundaryRule,
) -> Result<bool, DomainError> {
    Ok(exact::segment_in_region(segment, polygons, boundary))
}

/// Validate bounded geometry, disjoint ring nesting, winding, and class witnesses.
///
/// These are small-input test predicates, not production repair logic. `Thin`
/// and `NearDegenerate` use structural edge-clearance witnesses, not general
/// proofs of thinness. Boundary rules govern queries, not allowed ring contact.
/// Coordinates are limited to 1,000,000 absolute canonical units, and count
/// bounds must fit in `u32`; excessive bounds are rejected, never narrowed.
pub fn validate_geometry(case: &GeometryCase, domain: &GeometryDomain) -> Result<(), DomainError> {
    exact::validate_geometry(case, domain)
}

/// Validate finite vertices, triangle structure, indices and nonzero area.
///
/// In closed mode, every undirected edge must occur exactly twice, in opposite
/// directions. This check does not certify absence of 3D self-intersections.
pub fn validate_indexed_mesh(
    mesh: &IndexedTriangleSet,
    require_closed: bool,
) -> Result<(), DomainError> {
    self::mesh::validate_indexed_mesh(mesh, require_closed)
}

/// Sum oriented tetrahedral volumes in mm³ after structural validation.
///
/// A consistently outward-oriented closed surface has positive signed volume.
/// Nonfinite arithmetic intermediates produce [`DomainError::ArithmeticOverflow`].
/// With today's f32 vertices and at-most-64-bit slice lengths, each coordinate
/// is below `2^128`, each tetrahedron dot product below `2^387`, and the total
/// below `2^451`, all within f64 range. The checks remain explicit if the IR
/// representation changes; non-finite input vertices are invalid input instead.
pub fn mesh_signed_volume_mm3(mesh: &IndexedTriangleSet) -> Result<f64, DomainError> {
    self::mesh::mesh_signed_volume_mm3(mesh)
}

/// Enumerate node pairs connected by nonempty directed paths.
///
/// A self-pair means a cycle. Duplicate nodes and unknown endpoints are errors.
/// This is a breadth-first path model, not a topological sorting algorithm.
pub fn graph_reachability(graph: &SmallGraph) -> Result<BTreeSet<(u16, u16)>, DomainError> {
    let nodes: BTreeSet<_> = graph.node_ids.iter().copied().collect();
    if nodes.len() != graph.node_ids.len()
        || graph
            .edges
            .iter()
            .any(|(a, b)| !nodes.contains(a) || !nodes.contains(b))
    {
        return Err(DomainError::InvalidInput(
            "graph has duplicate nodes or unknown endpoints".into(),
        ));
    }
    let mut pairs = BTreeSet::new();
    for &source in &nodes {
        let mut queue = VecDeque::new();
        for &(_, target) in graph.edges.iter().filter(|(start, _)| *start == source) {
            queue.push_back(target);
        }
        let mut seen = BTreeSet::new();
        while let Some(node) = queue.pop_front() {
            if seen.insert(node) {
                pairs.insert((source, node));
                queue.extend(
                    graph
                        .edges
                        .iter()
                        .filter_map(|&(start, end)| (start == node).then_some(end)),
                );
            }
        }
    }
    Ok(pairs)
}

/// Return cycle nodes and all nodes reachable from a cycle by a nonempty path.
pub fn graph_cycle_remainder(graph: &SmallGraph) -> Result<BTreeSet<u16>, DomainError> {
    let paths = graph_reachability(graph)?;
    let cycle_nodes: BTreeSet<_> = paths
        .iter()
        .filter_map(|&(a, b)| (a == b).then_some(a))
        .collect();
    Ok(paths
        .iter()
        .filter_map(|&(a, b)| cycle_nodes.contains(&a).then_some(b))
        .collect())
}

/// Return the first travel anchor absent from the ordered entity IDs.
#[must_use]
pub fn first_missing_anchor(layer: &LayerCollectionIR) -> Option<u64> {
    let ids: BTreeSet<_> = layer
        .ordered_entities
        .iter()
        .map(|entity| entity.entity_id)
        .collect();
    layer
        .travel_moves
        .iter()
        .map(|travel| travel.entity_id)
        .find(|id| !ids.contains(id))
}

/// Test whether an order is a complete bijection over `0..len`.
#[must_use]
pub fn is_permutation(order: &[usize], len: usize) -> bool {
    order.len() == len
        && order.iter().all(|&index| index < len)
        && order.iter().copied().collect::<BTreeSet<_>>().len() == len
}

/// Test contiguity and authored order; group disjointness is a separate concern.
#[must_use]
pub fn locked_groups_are_contiguous(order: &[usize], groups: &[Vec<usize>]) -> bool {
    groups
        .iter()
        .all(|group| !group.is_empty() && order.windows(group.len()).any(|window| window == group))
}
