//! Original Rust adapter from materialised Lightning nodes to evidence records.

use std::collections::HashMap;
use std::rc::Rc;

use slicer_ir::{ExPolygon, Point2};

use crate::distance_field::{closest_point_on_segment, distance_units};
use crate::kernel::{generator::Generator, tree_node::NodeRef};
use crate::model::{
    DomainKind, EvidenceLocation, GraphEdge, GraphLayer, GraphNode, GraphProvenance, GroundKind,
    GroundingWitness, KernelError,
};
use crate::polygon::point_in_expolygon;

/// Caller-owned layer identity and provenance, independent of tree geometry.
pub struct ExtractionContext<'a> {
    pub object_id: &'a str,
    pub local_layer_index: u32,
    pub global_layer_index: u32,
    pub z_mm: f64,
    pub origin_region_index: u32,
    pub provenance: GraphProvenance,
    pub temporary_anchors: &'a [ExPolygon],
}

fn on_segment(point: Point2, endpoints: [Point2; 2]) -> bool {
    distance_units(
        point,
        closest_point_on_segment(point, endpoints[0], endpoints[1]),
    ) <= 0.5
}

fn grounding(
    point: Point2,
    node_id: u32,
    context: &ExtractionContext<'_>,
) -> Result<GroundingWitness, KernelError> {
    let temporary = context.temporary_anchors.iter().any(|polygon| {
        point_in_expolygon(point, polygon)
            || std::iter::once(&polygon.contour)
                .chain(polygon.holes.iter())
                .any(|ring| {
                    let points = &ring.points;
                    (0..points.len())
                        .any(|i| on_segment(point, [points[i], points[(i + 1) % points.len()]]))
                })
    });
    let (kind, domain) = if temporary {
        (GroundKind::TemporaryAnchor, DomainKind::TemporaryAnchor)
    } else {
        (GroundKind::Boundary, DomainKind::Internal)
    };
    let mut containing = context
        .provenance
        .boundary_provenance
        .iter()
        .filter(|boundary| on_segment(point, boundary.endpoints));
    let first = containing.next();
    let boundary = first
        .filter(|boundary| boundary.address.domain == domain)
        .or_else(|| containing.find(|boundary| boundary.address.domain == domain))
        .or(first)
        .ok_or_else(|| KernelError::SourceProvenanceUnavailable {
            context: EvidenceLocation {
                object_id: context.object_id.to_owned(),
                global_layer_index: context.global_layer_index,
                region: None,
                source: None,
                checkpoint: None,
            },
            reason: format!(
                "layer {} node {node_id}: grounding point has no boundary provenance",
                context.global_layer_index,
            ),
        })?;
    Ok(GroundingWitness {
        kind,
        boundary: boundary.address.clone(),
    })
}

fn next_id(len: usize) -> u32 {
    let id = u32::try_from(len + 1).expect("Lightning graph id overflow");
    assert!(
        id < (1 << 24),
        "Lightning graph id exceeds exact evidence range"
    );
    id
}

/// Depth-first extraction in root/child order, preserving each edge occurrence.
/// Shared nodes use their first encountered ancestor for node parent/root identity;
/// each edge retains its own traversal ancestor. Isolated roots are exactly nodes
/// with `parent_id.is_none()` and no outgoing edge.
pub fn extract_roots(
    roots: &[NodeRef],
    sampled_segments: &[[Point2; 2]],
    context: ExtractionContext<'_>,
) -> Result<GraphLayer, KernelError> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut visited = HashMap::new();
    let mut stack: Vec<_> = roots
        .iter()
        .rev()
        .map(|root| (Rc::clone(root), None, None))
        .collect();
    while let Some((node, parent_id, ancestor_id)) = stack.pop() {
        let pointer = Rc::as_ptr(&node) as usize;
        let existing = visited.get(&pointer).copied();
        let node_id = existing.unwrap_or_else(|| next_id(nodes.len()));
        let root_id = ancestor_id.unwrap_or(node_id);
        if let Some(parent_id) = parent_id {
            edges.push(GraphEdge {
                edge_id: next_id(edges.len()),
                parent_id,
                child_id: node_id,
                root_id,
                origin_region_index: context.origin_region_index,
            });
        }
        if existing.is_some() {
            continue;
        }
        visited.insert(pointer, node_id);
        let node = node.borrow();
        let grounding = node
            .get_last_grounding_location()
            .map(|point| grounding(point, node_id, &context))
            .transpose()?;
        nodes.push(GraphNode {
            node_id,
            point: node.location(),
            parent_id,
            root_id,
            origin_region_index: context.origin_region_index,
            grounding,
        });
        for child in node.children().into_iter().rev() {
            stack.push((child, Some(node_id), Some(root_id)));
        }
    }
    Ok(GraphLayer {
        local_layer_index: context.local_layer_index,
        global_layer_index: context.global_layer_index,
        z_mm: context.z_mm,
        provenance: context.provenance,
        nodes,
        edges,
        sampled_segments: sampled_segments.to_vec(),
    })
}

/// Extract the generated graph and committed samples for one generator layer.
pub fn extract_layer(
    generator: &Generator,
    layer_id: usize,
    context: ExtractionContext<'_>,
) -> Result<GraphLayer, KernelError> {
    extract_roots(
        &generator.get_trees_for_layer(layer_id).tree_roots,
        generator.committed_segments_for_layer(layer_id),
        context,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::tree_node::Node;
    use crate::model::{
        BoundaryAddress, BoundaryProvenance, RegionIdentity, SourceCheckpoint, SourceSurface,
        SourceSurfaceId,
    };

    fn outline() -> Vec<Point2> {
        vec![
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(100.0, 0.0),
            Point2::from_mm(100.0, 100.0),
            Point2::from_mm(0.0, 100.0),
        ]
    }

    fn context(layer: u32) -> ExtractionContext<'static> {
        let points = outline();
        let geometry = ExPolygon {
            contour: slicer_ir::Polygon {
                points: points.clone(),
            },
            holes: Vec::new(),
        };
        // exhaustive: SourceSurface has no Default; every source fixture field is meaningful.
        let source = SourceSurface {
            // exhaustive: SourceSurfaceId has no Default; record the complete layer identity.
            id: SourceSurfaceId {
                identity: RegionIdentity {
                    object_id: "synthetic".to_owned(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                local_layer_index: layer,
                global_layer_index: layer,
                checkpoint: SourceCheckpoint::DuringPlanning,
                surface_token: "outline".to_owned(),
            },
            geometry,
            surface_kind: "internal".to_owned(),
            solid_angle_rad: None,
            bridge_angle_rad: None,
        };
        // exhaustive: ExtractionContext has no Default; supply the full extraction contract.
        ExtractionContext {
            object_id: "synthetic",
            local_layer_index: layer,
            global_layer_index: layer,
            z_mm: 0.2 * (layer + 1) as f64,
            origin_region_index: 7,
            provenance: GraphProvenance {
                source_catalog: vec![source],
                boundary_provenance: (0..points.len())
                    .map(|i| BoundaryProvenance {
                        address: BoundaryAddress {
                            domain: DomainKind::Internal,
                            surface_id: 0,
                            ring_id: 0,
                            segment_id: i as u32,
                        },
                        endpoints: [points[i], points[(i + 1) % points.len()]],
                        contributors: Vec::new(),
                    })
                    .collect(),
                operations: Vec::new(),
            },
            temporary_anchors: &[],
        }
    }

    #[test]
    fn tree_extraction_tdd() {
        let mut generator = Generator::new(
            vec![
                vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(10.0, 0.0),
                    Point2::from_mm(10.0, 10.0),
                    Point2::from_mm(0.0, 10.0),
                ],
                outline(),
            ],
            0.15,
            4_500,
            1,
            2_000,
            45.0,
            5.0,
            5.0,
        );
        generator.generate_trees(&|| {});
        let layer = (0..2)
            .rev()
            .find(|&i| !generator.get_trees_for_layer(i).tree_roots.is_empty())
            .expect("synthetic outlines must generate real trees");
        let graph = extract_layer(&generator, layer, context(layer as u32)).unwrap();
        assert!(!graph.nodes.is_empty());
        assert!(graph.nodes.iter().any(|node| node.parent_id.is_none()));
        assert!(graph.nodes.iter().any(|node| node.parent_id.is_some()));
        let ids: std::collections::HashSet<_> =
            graph.nodes.iter().map(|node| node.node_id).collect();
        assert_eq!(ids.len(), graph.nodes.len());
        assert!(ids.iter().all(|&id| id > 0 && id < (1 << 24)));
        assert_eq!(
            graph.sampled_segments,
            generator.committed_segments_for_layer(layer)
        );
        let grounded: Vec<_> = graph
            .nodes
            .iter()
            .filter_map(|node| node.grounding.as_ref())
            .collect();
        assert!(!grounded.is_empty());
        for witness in grounded {
            assert!((witness.boundary.surface_id as usize) < graph.provenance.source_catalog.len());
            assert!(graph
                .provenance
                .boundary_provenance
                .iter()
                .any(|boundary| boundary.address == witness.boundary));
        }

        let first = Node::new(Point2::from_mm(0.01, 0.01));
        let second = Node::new(Point2::from_mm(0.01, 0.01));
        let child = Node::new(Point2::from_mm(0.02, 0.02));
        first.borrow().add_child_node(Rc::clone(&child));
        first.borrow().add_child_node(Rc::clone(&child));
        let graph = extract_roots(&[first, second], &[], context(0)).unwrap();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[0].point, graph.nodes[2].point);
        assert_ne!(graph.nodes[0].node_id, graph.nodes[2].node_id);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].parent_id, graph.edges[1].parent_id);
        assert_eq!(graph.edges[0].child_id, graph.edges[1].child_id);
        assert_ne!(graph.edges[0].edge_id, graph.edges[1].edge_id);
        assert_eq!(graph.nodes[1].parent_id, Some(graph.nodes[0].node_id));
        assert_eq!(graph.nodes[1].root_id, graph.nodes[0].node_id);
        assert!(graph.nodes[2].parent_id.is_none());
        assert!(!graph
            .edges
            .iter()
            .any(|edge| edge.parent_id == graph.nodes[2].node_id));

        let bad = Node::new_with_grounding_location(
            Point2::from_mm(0.01, 0.01),
            Some(Point2::from_mm(200.0, 200.0)),
        );
        assert!(matches!(extract_roots(&[bad], &[], context(0)),
            Err(KernelError::SourceProvenanceUnavailable { context, reason })
                if context.object_id == "synthetic" && context.global_layer_index == 0
                    && reason.contains("node 1")
        ));
    }
}
