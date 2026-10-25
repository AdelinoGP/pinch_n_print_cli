//! Original Rust orchestration of recorded planning domains and final-fill clips.

use std::collections::{BTreeMap, HashMap, HashSet};

use slicer_core::polygon_ops::clip_polylines;
use slicer_ir::{mm_to_units, Point2};

use crate::kernel::{generator::Generator, generator_from_parameters};
use crate::model::{
    ClippedFill, ClippedLayer, EvidenceLocation, GraphLayer, GraphProvenance, KernelError,
    KernelInput, KernelPlan, LightningParameters, PlanningLayer, RegionIdentity,
};
use crate::polygon::{expolygon_area, point_in_expolygon};
use crate::tree::{extract_layer, ExtractionContext};

const TRANSPORT_ID_LIMIT: usize = 1 << 24;

fn location(
    object_id: &str,
    global_layer_index: u32,
    region: Option<&RegionIdentity>,
) -> EvidenceLocation {
    EvidenceLocation {
        object_id: object_id.to_owned(),
        global_layer_index,
        region: region.cloned(),
        source: None,
        checkpoint: None,
    }
}

fn provenance(layer: &PlanningLayer) -> GraphProvenance {
    GraphProvenance {
        source_catalog: layer.source_catalog.clone(),
        boundary_provenance: layer.boundary_provenance.clone(),
        operations: layer.operations.clone(),
    }
}

fn validate_scalars(
    generator: &Generator,
    parameters: &LightningParameters,
    context: &EvidenceLocation,
) -> Result<(), KernelError> {
    let scalars = [
        (
            "supporting_radius",
            generator.supporting_radius,
            parameters.supporting_radius_mm,
        ),
        (
            "wall_supporting_radius",
            generator.wall_supporting_radius,
            Some(parameters.wall_supporting_radius_mm),
        ),
        (
            "prune_length",
            generator.prune_length,
            Some(parameters.prune_length_mm),
        ),
        (
            "straightening_max_distance",
            generator.straightening_max_distance,
            Some(parameters.straightening_max_distance_mm),
        ),
    ];
    for (field, derived, recorded) in scalars {
        if let Some(recorded) = recorded {
            let recorded = mm_to_units(recorded as f32);
            if derived != recorded {
                return Err(KernelError::RequiredSourceInputMismatch {
                    context: context.clone(),
                    field: field.to_owned(),
                    expected: derived.to_string(),
                    actual: recorded.to_string(),
                });
            }
        }
    }
    Ok(())
}

fn check_id_counts(
    nodes: usize,
    edges: usize,
    object_id: &str,
    global_layer_index: u32,
) -> Result<(), KernelError> {
    if nodes >= TRANSPORT_ID_LIMIT || edges >= TRANSPORT_ID_LIMIT {
        return Err(KernelError::UnrepresentableTransport {
            context: location(object_id, global_layer_index, None),
            reason: "merged Lightning graph ids must be less than 1 << 24".to_owned(),
        });
    }
    Ok(())
}

fn preflight_graph_ids(
    generator: &Generator,
    layer_id: usize,
    target: &GraphLayer,
    object_id: &str,
) -> Result<(), KernelError> {
    // Extraction itself asserts the per-call limit. Count first so even a single
    // oversized group returns the named transport error instead of panicking.
    let mut stack = generator.get_trees_for_layer(layer_id).tree_roots.clone();
    let mut seen = HashSet::new();
    let mut edges = target.edges.len();
    while let Some(node) = stack.pop() {
        if !seen.insert(std::rc::Rc::as_ptr(&node)) {
            continue;
        }
        let children = node.borrow().children();
        edges = edges.saturating_add(children.len());
        check_id_counts(
            target.nodes.len().saturating_add(seen.len()),
            edges,
            object_id,
            target.global_layer_index,
        )?;
        stack.extend(children);
    }
    Ok(())
}

fn merge_graph(
    target: &mut GraphLayer,
    mut graph: GraphLayer,
    object_id: &str,
) -> Result<(), KernelError> {
    check_id_counts(
        target.nodes.len().saturating_add(graph.nodes.len()),
        target.edges.len().saturating_add(graph.edges.len()),
        object_id,
        target.global_layer_index,
    )?;
    let node_ids: HashMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.node_id, (target.nodes.len() + index + 1) as u32))
        .collect();
    for node in &mut graph.nodes {
        node.node_id = node_ids[&node.node_id];
        node.parent_id = node.parent_id.map(|id| node_ids[&id]);
        node.root_id = node_ids[&node.root_id];
    }
    for (index, edge) in graph.edges.iter_mut().enumerate() {
        edge.edge_id = (target.edges.len() + index + 1) as u32;
        edge.parent_id = node_ids[&edge.parent_id];
        edge.child_id = node_ids[&edge.child_id];
        edge.root_id = node_ids[&edge.root_id];
    }
    target.nodes.extend(graph.nodes);
    target.edges.extend(graph.edges);
    target.sampled_segments.extend(graph.sampled_segments);
    Ok(())
}

/// Plan exclusively from the recorded planning domains, never final-fill inputs.
pub fn plan_lightning(input: &KernelInput) -> Result<KernelPlan, KernelError> {
    let Some(max_global) = input
        .planning_layers
        .iter()
        .map(|layer| layer.global_layer_index)
        .max()
    else {
        return Ok(KernelPlan {
            object_id: input.object_id.clone(),
            layers: Vec::new(),
        });
    };
    let mut merged = BTreeMap::new();
    let mut groups = BTreeMap::<(u32, usize), BTreeMap<u32, Vec<Point2>>>::new();
    for layer in &input.planning_layers {
        let context = location(&input.object_id, layer.global_layer_index, None);
        let graph = GraphLayer {
            local_layer_index: layer.local_layer_index,
            global_layer_index: layer.global_layer_index,
            z_mm: layer.z_mm,
            provenance: provenance(layer),
            nodes: Vec::new(),
            edges: Vec::new(),
            sampled_segments: Vec::new(),
        };
        if merged.insert(layer.global_layer_index, graph).is_some() {
            return Err(KernelError::InvalidInput {
                context,
                reason: format!("duplicate planning layer {}", layer.global_layer_index),
            });
        }
        let mut ordinals = BTreeMap::<u32, usize>::new();
        for (domain_index, domain) in layer.internal_domains.iter().enumerate() {
            let mut identity = None;
            for source in &layer.source_catalog {
                if source.geometry == *domain
                    || domain
                        .contour
                        .points
                        .iter()
                        .all(|point| point_in_expolygon(*point, &source.geometry))
                {
                    match identity {
                        Some(previous) if previous != &source.id.identity => {
                            return Err(KernelError::SourceProvenanceUnavailable {
                                context: context.clone(),
                                reason: format!(
                                    "layer {} domain {domain_index}: multiple source region identities",
                                    layer.global_layer_index,
                                ),
                            });
                        }
                        _ => identity = Some(&source.id.identity),
                    }
                }
            }
            let identity = identity.ok_or_else(|| KernelError::SourceProvenanceUnavailable {
                context: context.clone(),
                reason: format!(
                    "layer {} domain {domain_index}: no source region identity",
                    layer.global_layer_index,
                ),
            })?;
            let origin_region_index = input
                .canonical_region_order
                .iter()
                .position(|candidate| candidate == identity)
                .ok_or_else(|| KernelError::SourceProvenanceUnavailable {
                    context: location(&input.object_id, layer.global_layer_index, Some(identity)),
                    reason: format!(
                        "layer {} domain {domain_index}: source region absent from canonical order",
                        layer.global_layer_index,
                    ),
                })?;
            let origin_region_index = u32::try_from(origin_region_index).map_err(|_| {
                KernelError::UnrepresentableTransport {
                    context: context.clone(),
                    reason: "canonical region index exceeds u32".to_owned(),
                }
            })?;
            let ordinal = ordinals.entry(origin_region_index).or_default();
            groups
                .entry((origin_region_index, *ordinal))
                .or_default()
                .insert(layer.global_layer_index, domain.contour.points.clone());
            *ordinal += 1;
        }
    }

    for ((origin_region_index, _), domains) in groups {
        let mut outlines = vec![Vec::new(); max_global as usize + 1];
        for (global_layer_index, outline) in domains {
            outlines[global_layer_index as usize] = outline;
        }
        let context = location(
            &input.object_id,
            input.planning_layers[0].global_layer_index,
            Some(&input.scalar_source_region),
        );
        let mut generator = generator_from_parameters(outlines, &input.parameters)?;
        validate_scalars(&generator, &input.parameters, &context)?;
        generator.generate_trees(&|| {});
        for layer in &input.planning_layers {
            let target = merged.get_mut(&layer.global_layer_index).unwrap();
            preflight_graph_ids(
                &generator,
                layer.global_layer_index as usize,
                target,
                &input.object_id,
            )?;
            let graph = extract_layer(
                &generator,
                layer.global_layer_index as usize,
                ExtractionContext {
                    object_id: &input.object_id,
                    local_layer_index: layer.local_layer_index,
                    global_layer_index: layer.global_layer_index,
                    z_mm: layer.z_mm,
                    origin_region_index,
                    provenance: provenance(layer),
                    temporary_anchors: &layer.temporary_anchors,
                },
            )?;
            merge_graph(target, graph, &input.object_id)?;
        }
    }
    Ok(KernelPlan {
        object_id: input.object_id.clone(),
        layers: merged.into_values().collect(),
    })
}

fn invalid_dispatch(context: &EvidenceLocation, token: &str, reason: &str) -> KernelError {
    KernelError::InvalidInput {
        context: context.clone(),
        reason: format!("dispatch token {token:?}: {reason}"),
    }
}

/// Clip each final dispatch independently, preserving edge occurrences and direction.
pub fn clip_lightning(
    plan: &KernelPlan,
    input: &KernelInput,
) -> Result<Vec<ClippedLayer>, KernelError> {
    let mut tokens = HashSet::new();
    for layer in &input.layers {
        for region in &layer.regions {
            for fill in &region.final_fills {
                let context = location(
                    &input.object_id,
                    layer.global_layer_index,
                    Some(&fill.caller_identity),
                );
                if fill.dispatch_token.trim().is_empty() {
                    return Err(invalid_dispatch(
                        &context,
                        &fill.dispatch_token,
                        "missing token",
                    ));
                }
                if !tokens.insert(fill.dispatch_token.as_str()) {
                    return Err(invalid_dispatch(
                        &context,
                        &fill.dispatch_token,
                        "duplicate token",
                    ));
                }
            }
        }
    }

    let mut clipped = Vec::new();
    for layer in &input.layers {
        let mut fills = Vec::new();
        for region in &layer.regions {
            for fill in &region.final_fills {
                let context = location(
                    &input.object_id,
                    layer.global_layer_index,
                    Some(&fill.caller_identity),
                );
                let invalid = |reason| invalid_dispatch(&context, &fill.dispatch_token, reason);
                let mut origins = input
                    .canonical_region_order
                    .iter()
                    .enumerate()
                    .filter(|(_, identity)| *identity == &fill.caller_identity);
                let (origin, _) = origins
                    .next()
                    .ok_or_else(|| invalid("missing caller region in canonical order"))?;
                if origins.next().is_some() {
                    return Err(invalid("ambiguous caller region in canonical order"));
                }
                let mut layers = plan
                    .layers
                    .iter()
                    .filter(|graph| graph.global_layer_index == layer.global_layer_index);
                let graph = layers.next().ok_or_else(|| invalid("missing plan layer"))?;
                if layers.next().is_some() {
                    return Err(invalid("ambiguous plan layer"));
                }
                if !graph
                    .nodes
                    .iter()
                    .any(|node| node.origin_region_index as usize == origin)
                {
                    return Err(invalid("missing plan nodes for caller region"));
                }
                let mut nodes = HashMap::new();
                for node in &graph.nodes {
                    if nodes.insert(node.node_id, node).is_some() {
                        return Err(invalid("ambiguous plan node id"));
                    }
                }
                let mut subjects = Vec::new();
                for edge in &graph.edges {
                    if edge.origin_region_index as usize != origin {
                        continue;
                    }
                    let parent = nodes
                        .get(&edge.parent_id)
                        .ok_or_else(|| invalid("missing edge parent"))?;
                    let child = nodes
                        .get(&edge.child_id)
                        .ok_or_else(|| invalid("missing edge child"))?;
                    if parent.origin_region_index as usize != origin
                        || child.origin_region_index as usize != origin
                    {
                        return Err(invalid("ambiguous edge region"));
                    }
                    subjects.push(vec![parent.point, child.point]);
                }
                let segments = if fill.geometry.contour.points.len() < 3
                    || expolygon_area(&fill.geometry) <= 0.0
                {
                    Vec::new()
                } else {
                    clip_polylines(&subjects, std::slice::from_ref(&fill.geometry))
                        .into_iter()
                        .flat_map(|polyline| {
                            polyline
                                .windows(2)
                                .map(|pair| [pair[0], pair[1]])
                                .collect::<Vec<_>>()
                        })
                        .collect()
                };
                fills.push(ClippedFill {
                    dispatch_token: fill.dispatch_token.clone(),
                    caller_identity: fill.caller_identity.clone(),
                    contributors: fill.contributors.clone(),
                    segments,
                });
            }
        }
        if !fills.is_empty() {
            clipped.push(ClippedLayer {
                object_id: input.object_id.clone(),
                local_layer_index: layer.local_layer_index,
                global_layer_index: layer.global_layer_index,
                z_mm: layer.z_mm,
                fills,
            });
        }
    }
    Ok(clipped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        BoundaryAddress, BoundaryContribution, BoundaryProvenance, ContributionRelation,
        DomainKind, FillPolicyInput, FinalFillInput, GeometryOpKind, GeometryOperation, GraphEdge,
        GraphNode, LayerInput, RegionInput, SourceCheckpoint, SourceSegmentRef, SourceSurface,
        SourceSurfaceId,
    };
    use slicer_ir::{ExPolygon, Polygon};

    fn identity(region_id: u64) -> RegionIdentity {
        RegionIdentity {
            object_id: "planner-object".to_owned(),
            region_id,
            variant_chain: Vec::new(),
        }
    }

    fn rectangle(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: x0, y: y0 },
                    Point2 { x: x1, y: y0 },
                    Point2 { x: x1, y: y1 },
                    Point2 { x: x0, y: y1 },
                ],
            },
            holes: Vec::new(),
        }
    }

    fn parameters() -> LightningParameters {
        LightningParameters {
            nozzle_diameter_mm: 0.4,
            extrusion_width_mm: 0.44999999999999996,
            density_fraction: 0.15,
            multiline: 1,
            layer_height_mm: 0.2,
            overhang_angle_deg: 45.0,
            prune_angle_deg: 5.0,
            straightening_angle_deg: 5.0,
            supporting_radius_mm: Some(3.0),
            wall_supporting_radius_mm: 0.19999999999999998,
            prune_length_mm: 0.017497,
            straightening_max_distance_mm: 0.017497,
        } // exhaustive: canonical Lightning constructor and recorded derived scalars.
    }

    fn policy() -> FillPolicyInput {
        FillPolicyInput {
            infill_direction_deg: 0.0,
            sparse_rotate_template: String::new(),
            solid_infill_direction_deg: 0.0,
            solid_rotate_template: String::new(),
            top_layer_direction_deg: 0.0,
            bottom_layer_direction_deg: 0.0,
            align_to_model: false,
            model_alignment_offset_rad: 0.0,
            resolved_sparse_angle_rad: 0.0,
            resolved_solid_angle_rad: 0.0,
            resolved_top_angle_rad: None,
            resolved_bottom_angle_rad: None,
            fixed_angle: false,
            is_bridge: false,
            bridge_angle_rad: None,
        } // exhaustive: final dispatch policy transport is not a planner input.
    }

    fn planning_layer(domains: Vec<ExPolygon>, global: u32, local: u32) -> PlanningLayer {
        let mut catalog = Vec::new();
        let mut boundaries = Vec::new();
        for (domain_index, domain) in domains.iter().enumerate() {
            let source = SourceSurfaceId {
                identity: identity(7),
                local_layer_index: local,
                global_layer_index: global,
                checkpoint: SourceCheckpoint::DuringPlanning,
                surface_token: format!("domain-{domain_index}"),
            }; // exhaustive: source identity resolves each synthetic planning domain.
            catalog.push(SourceSurface {
                id: source.clone(),
                geometry: domain.clone(),
                surface_kind: "internal".to_owned(),
                solid_angle_rad: None,
                bridge_angle_rad: None,
            }); // exhaustive: source catalog geometry and optional angles are preserved.
            for segment in 0..domain.contour.points.len() {
                boundaries.push(BoundaryProvenance {
                    address: BoundaryAddress {
                        domain: DomainKind::Internal,
                        surface_id: domain_index as u32,
                        ring_id: 0,
                        segment_id: segment as u32,
                    },
                    endpoints: [
                        domain.contour.points[segment],
                        domain.contour.points[(segment + 1) % domain.contour.points.len()],
                    ],
                    contributors: vec![BoundaryContribution {
                        source: SourceSegmentRef {
                            surface: source.clone(),
                            ring_id: Some(0),
                            segment_id: Some(segment as u32),
                        },
                        relation: ContributionRelation::Generated { operation_id: 1 },
                    }],
                });
            }
        }
        let operation = GeometryOperation {
            operation_id: 1,
            kind: GeometryOpKind::Union,
            checkpoint: SourceCheckpoint::DuringPlanning,
            inputs: catalog
                .iter()
                .map(|source| SourceSegmentRef {
                    surface: source.id.clone(),
                    ring_id: None,
                    segment_id: None,
                })
                .collect(),
            parent_operations: Vec::new(),
            operands: Vec::new(),
            offset_units: None,
            join: None,
            result_geometry: domains.clone(),
            outputs: boundaries.iter().map(|edge| edge.address.clone()).collect(),
        }; // exhaustive: nonempty operation provenance detects dropped planner evidence.
        PlanningLayer {
            object_id: "planner-object".to_owned(),
            local_layer_index: local,
            global_layer_index: global,
            z_mm: (global + 1) as f64 * 0.2,
            internal_domains: domains,
            void_domains: Vec::new(),
            temporary_anchors: Vec::new(),
            source_regions: vec![identity(7)],
            source_catalog: catalog,
            boundary_provenance: boundaries,
            operations: vec![operation],
        } // exhaustive: synthetic planning layer owns all boundary and source evidence.
    }

    fn input() -> KernelInput {
        KernelInput {
            object_id: "planner-object".to_owned(),
            layers: Vec::new(),
            planning_layers: vec![planning_layer(
                vec![rectangle(0, 0, 200_000, 200_000)],
                1,
                0,
            )],
            canonical_region_order: vec![identity(0), identity(7)],
            scalar_source_region: identity(7),
            parameters: parameters(),
        } // exhaustive: independent planning transport leaves B-only layers unpopulated.
    }

    fn set_fill(input: &mut KernelInput, geometry: ExPolygon, token: &str) {
        let fill = FinalFillInput {
            dispatch_token: token.to_owned(),
            caller_identity: identity(7),
            geometry,
            contributors: vec![input.planning_layers[0].source_catalog[0].id.clone()],
            policy: policy(),
        }; // exhaustive: final geometry and contributor identity exercise clipping transport.
        let region = RegionInput {
            identity: identity(7),
            outlines: Vec::new(),
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            internal_solid_fill: Vec::new(),
            internal_bridge_areas: Vec::new(),
            internal_bridge_angles_deg: Vec::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            effective_parameters: parameters(),
            shell_surfaces: Vec::new(),
            classified_surfaces: Vec::new(),
            fill_policy: policy(),
            final_fill_expolygons: Vec::new(),
            final_source_catalog: Vec::new(),
            final_surface_sources: Vec::new(),
            final_fills: vec![fill],
        }; // exhaustive: B-only region fields must not affect the planning graph.
        input.layers = vec![LayerInput {
            local_layer_index: 19,
            global_layer_index: 1,
            z_mm: 0.4,
            effective_layer_height_mm: 0.2,
            regions: vec![region],
        }]; // exhaustive: clipping output carries the caller's layer metadata unchanged.
    }

    fn subjects(plan: &KernelPlan) -> Vec<[Point2; 2]> {
        let graph = &plan.layers[0];
        let nodes: HashMap<_, _> = graph
            .nodes
            .iter()
            .map(|node| (node.node_id, node.point))
            .collect();
        graph
            .edges
            .iter()
            .map(|edge| [nodes[&edge.parent_id], nodes[&edge.child_id]])
            .collect()
    }

    fn length(segment: &[Point2; 2]) -> f64 {
        let dx = (segment[1].x - segment[0].x) as f64;
        let dy = (segment[1].y - segment[0].y) as f64;
        dx.hypot(dy)
    }

    fn oriented_subsegment(segment: &[Point2; 2], subject: &[Point2; 2]) -> bool {
        let dx = (subject[1].x - subject[0].x) as f64;
        let dy = (subject[1].y - subject[0].y) as f64;
        let tolerance = 2.0 * dx.hypot(dy);
        let within = segment.iter().all(|point| {
            let px = (point.x - subject[0].x) as f64;
            let py = (point.y - subject[0].y) as f64;
            let projection = px * dx + py * dy;
            (px * dy - py * dx).abs() <= tolerance
                && projection >= -tolerance
                && projection <= dx * dx + dy * dy + tolerance
        });
        within
            && (segment[1].x - segment[0].x) as f64 * dx + (segment[1].y - segment[0].y) as f64 * dy
                > 0.0
    }

    fn crosses_open_rectangle(segment: &[Point2; 2], bounds: [i64; 4]) -> bool {
        let mut enter = 0.0_f64;
        let mut exit = 1.0_f64;
        for (start, end, min, max) in [
            (segment[0].x, segment[1].x, bounds[0], bounds[2]),
            (segment[0].y, segment[1].y, bounds[1], bounds[3]),
        ] {
            let delta = (end - start) as f64;
            if delta == 0.0 {
                if start <= min || start >= max {
                    return false;
                }
            } else {
                let a = (min - start) as f64 / delta;
                let b = (max - start) as f64 / delta;
                enter = enter.max(a.min(b));
                exit = exit.min(a.max(b));
            }
        }
        enter < exit
    }

    fn assert_in_rectangle(segments: &[[Point2; 2]], bounds: [i64; 4]) {
        // clip_polylines includes boundary edges by inflating the clip one IR unit.
        for point in segments.iter().flatten() {
            assert!(point.x >= bounds[0] - 1 && point.x <= bounds[2] + 1);
            assert!(point.y >= bounds[1] - 1 && point.y <= bounds[3] + 1);
        }
    }

    #[test]
    fn clip_lightning_semantics_tdd() {
        let mut input = input();
        let plan = plan_lightning(&input).unwrap();
        let subjects = subjects(&plan);
        assert!(
            !subjects.is_empty(),
            "global layer 1 must actually generate trees"
        );
        let subject = subjects
            .iter()
            .filter(|segment| {
                let midpoint = Point2 {
                    x: (segment[0].x + segment[1].x) / 2,
                    y: (segment[0].y + segment[1].y) / 2,
                };
                midpoint.x > 2_000
                    && midpoint.x < 198_000
                    && midpoint.y > 2_000
                    && midpoint.y < 198_000
            })
            .max_by(|a, b| length(a).total_cmp(&length(b)))
            .expect("generated graph needs an interior edge for the hole test");
        assert!(length(subject) > 2_000.0);
        let cx = (subject[0].x + subject[1].x) / 2;
        let cy = (subject[0].y + subject[1].y) / 2;
        let hole_bounds = [cx - 200, cy - 200, cx + 200, cy + 200];
        let mut geometry = rectangle(0, 0, 200_000, 200_000);
        let mut hole = rectangle(
            hole_bounds[0],
            hole_bounds[1],
            hole_bounds[2],
            hole_bounds[3],
        );
        hole.contour.points.reverse();
        geometry.holes.push(hole.contour);
        set_fill(&mut input, geometry, "hole-fill");
        let output = clip_lightning(&plan, &input).unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].object_id, input.object_id);
        assert_eq!(output[0].local_layer_index, 19);
        assert_eq!(output[0].global_layer_index, 1);
        assert_eq!(output[0].z_mm, 0.4);
        let clipped = &output[0].fills[0];
        assert_eq!(clipped.dispatch_token, "hole-fill");
        assert_eq!(clipped.caller_identity, identity(7));
        assert_eq!(
            clipped.contributors,
            input.layers[0].regions[0].final_fills[0].contributors
        );
        assert!(!clipped.segments.is_empty());
        assert_in_rectangle(&clipped.segments, [0, 0, 200_000, 200_000]);
        // The helper's one-unit boundary inclusion may enter the hole by one unit.
        let hole_interior = [cx - 198, cy - 198, cx + 198, cy + 198];
        assert!(crosses_open_rectangle(subject, hole_interior));
        for segment in &clipped.segments {
            assert_eq!(segment.len(), 2);
            assert!(!crosses_open_rectangle(segment, hole_interior));
            assert!(subjects
                .iter()
                .any(|edge| oriented_subsegment(segment, edge)));
        }
        let original_length: f64 = subjects.iter().map(length).sum();
        let clipped_length: f64 = clipped.segments.iter().map(length).sum();
        assert!(
            clipped_length < original_length - 100.0,
            "clipping to the planning outline would leave the hole crossing intact"
        );

        let px = (3 * subject[0].x + subject[1].x) / 4;
        let py = (3 * subject[0].y + subject[1].y) / 4;
        let partial_bounds = [px - 200, py - 200, px + 200, py + 200];
        set_fill(
            &mut input,
            rectangle(px - 200, py - 200, px + 200, py + 200),
            "partial",
        );
        let partial = clip_lightning(&plan, &input).unwrap();
        let segments = &partial[0].fills[0].segments;
        assert!(!segments.is_empty());
        assert!(segments.len() <= clipped.segments.len());
        assert_in_rectangle(segments, partial_bounds);
        assert!(segments.iter().map(length).sum::<f64>() < original_length);
        assert_ne!(
            segments, &subjects,
            "final geometry, not the planning domain, selects segments"
        );

        let mut empty = rectangle(0, 0, 1, 1);
        empty.contour.points.clear();
        set_fill(&mut input, empty, "empty");
        assert!(clip_lightning(&plan, &input).unwrap()[0].fills[0]
            .segments
            .is_empty());
        set_fill(&mut input, rectangle(0, 0, 0, 100), "zero-area");
        assert!(clip_lightning(&plan, &input).unwrap()[0].fills[0]
            .segments
            .is_empty());

        let duplicate = input.layers[0].clone();
        input.layers.push(duplicate);
        assert!(matches!(clip_lightning(&plan, &input),
            Err(KernelError::InvalidInput { reason, .. }) if reason.contains("zero-area")));
        input.layers.pop();
        input.layers[0].regions[0].final_fills[0].caller_identity = identity(99);
        assert!(matches!(clip_lightning(&plan, &input),
            Err(KernelError::InvalidInput { reason, .. }) if reason.contains("zero-area")));

        for token in ["", " \t\n"] {
            set_fill(&mut input, rectangle(0, 0, 200_000, 200_000), token);
            assert!(matches!(clip_lightning(&plan, &input),
                Err(KernelError::InvalidInput { reason, .. })
                    if reason.contains(&format!("dispatch token {token:?}"))
                        && reason.contains("missing token")));
        }
        set_fill(&mut input, rectangle(0, 0, 200_000, 200_000), "collision");
        let duplicate = input.layers[0].regions[0].final_fills[0].clone();
        input.layers[0].regions[0].final_fills.push(duplicate);
        assert!(matches!(clip_lightning(&plan, &input),
            Err(KernelError::InvalidInput { reason, .. })
                if reason.contains("dispatch token \"collision\"")
                    && reason.contains("duplicate token")));

        assert_clip_occurrences_and_ambiguous_dispatch();
    }

    #[test]
    fn plan_ignores_b_only_layers() {
        let mut input = input();
        let without_layers = plan_lightning(&input).unwrap();
        assert!(!without_layers.layers[0].edges.is_empty());
        set_fill(
            &mut input,
            rectangle(10_000, 10_000, 50_000, 50_000),
            "b-only",
        );
        let with_layers = plan_lightning(&input).unwrap();
        assert_eq!(without_layers, with_layers);
        input.layers.clear();
        assert_eq!(with_layers, plan_lightning(&input).unwrap());
        input.planning_layers.clear();
        assert_eq!(
            plan_lightning(&input).unwrap(),
            KernelPlan {
                object_id: input.object_id.clone(),
                layers: Vec::new(),
            }
        );
    }

    #[test]
    fn plan_groups_and_merges() {
        let mut input = input();
        let domains = vec![
            rectangle(0, 0, 200_000, 200_000),
            rectangle(300_000, 0, 500_000, 200_000),
        ];
        input.planning_layers = vec![
            planning_layer(domains.clone(), 3, 2),
            planning_layer(domains, 1, 0),
        ];
        // Multiple catalog occurrences in the same region remain resolvable.
        let mut same_region = input.planning_layers[0].source_catalog[0].clone();
        same_region.id.surface_token = "another-same-region-occurrence".to_owned();
        input.planning_layers[0].source_catalog.push(same_region);
        let plan = plan_lightning(&input).unwrap();
        assert_eq!(
            plan.layers
                .iter()
                .map(|layer| layer.global_layer_index)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
        for graph in &plan.layers {
            let source = input
                .planning_layers
                .iter()
                .find(|layer| layer.global_layer_index == graph.global_layer_index)
                .unwrap();
            assert_eq!(graph.local_layer_index, source.local_layer_index);
            assert_eq!(graph.z_mm, source.z_mm);
            assert_eq!(graph.provenance.source_catalog, source.source_catalog);
            assert_eq!(
                graph.provenance.boundary_provenance,
                source.boundary_provenance
            );
            assert_eq!(graph.provenance.operations, source.operations);
            assert!(!graph.provenance.operations.is_empty());
            assert!(!graph.edges.is_empty());
            assert!(!graph.sampled_segments.is_empty());
            assert!(graph.nodes.iter().any(|node| node.point.x < 200_001));
            assert!(graph.nodes.iter().any(|node| node.point.x > 299_999));
            let node_ids: HashSet<_> = graph.nodes.iter().map(|node| node.node_id).collect();
            let edge_ids: HashSet<_> = graph.edges.iter().map(|edge| edge.edge_id).collect();
            assert_eq!(node_ids.len(), graph.nodes.len());
            assert_eq!(edge_ids.len(), graph.edges.len());
            for node in &graph.nodes {
                assert!(node.node_id > 0 && node.node_id < 1 << 24);
                assert_eq!(node.origin_region_index, 1);
                assert!(node_ids.contains(&node.root_id));
                let root = graph
                    .nodes
                    .iter()
                    .find(|root| root.node_id == node.root_id)
                    .unwrap();
                assert_eq!(root.point.x < 250_000, node.point.x < 250_000);
                if let Some(parent) = node.parent_id {
                    assert!(node_ids.contains(&parent));
                }
            }
            for edge in &graph.edges {
                assert!(edge.edge_id > 0 && edge.edge_id < 1 << 24);
                assert_eq!(edge.origin_region_index, 1);
                assert!(node_ids.contains(&edge.parent_id));
                assert!(node_ids.contains(&edge.child_id));
                assert!(node_ids.contains(&edge.root_id));
                let parent = graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == edge.parent_id)
                    .unwrap();
                let child = graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == edge.child_id)
                    .unwrap();
                assert_eq!(parent.point.x < 250_000, child.point.x < 250_000);
            }
        }
        assert_eq!(
            plan,
            plan_lightning(&input).unwrap(),
            "group traversal must be deterministic"
        );
    }

    #[test]
    fn graph_id_count_limit_is_exclusive() {
        assert!(
            check_id_counts(TRANSPORT_ID_LIMIT - 1, TRANSPORT_ID_LIMIT - 1, "object", 3).is_ok()
        );
        for (nodes, edges) in [(TRANSPORT_ID_LIMIT, 0), (0, TRANSPORT_ID_LIMIT)] {
            assert!(matches!(check_id_counts(nodes, edges, "object", 3),
                Err(KernelError::UnrepresentableTransport { context, .. })
                    if context.object_id == "object" && context.global_layer_index == 3));
        }
    }

    #[test]
    fn plan_rejects_unresolved_domains_and_scalar_disagreements() {
        let input = input();
        let mut missing = input.clone();
        missing.planning_layers[0].source_catalog.clear();
        assert!(matches!(plan_lightning(&missing),
            Err(KernelError::SourceProvenanceUnavailable { context, reason })
                if context.global_layer_index == 1 && reason.contains("layer 1 domain 0")));
        let mut ambiguous = input.clone();
        let mut source = ambiguous.planning_layers[0].source_catalog[0].clone();
        source.id.identity = identity(0);
        ambiguous.planning_layers[0].source_catalog.push(source);
        assert!(matches!(plan_lightning(&ambiguous),
            Err(KernelError::SourceProvenanceUnavailable { reason, .. })
                if reason.contains("layer 1 domain 0")));
        let mut absent = input.clone();
        absent.canonical_region_order.clear();
        assert!(matches!(
            plan_lightning(&absent),
            Err(KernelError::SourceProvenanceUnavailable { .. })
        ));
        for field in [
            "supporting_radius",
            "wall_supporting_radius",
            "prune_length",
            "straightening_max_distance",
        ] {
            let mut mismatch = input.clone();
            match field {
                "supporting_radius" => mismatch.parameters.supporting_radius_mm = Some(4.0),
                "wall_supporting_radius" => mismatch.parameters.wall_supporting_radius_mm = 1.0,
                "prune_length" => mismatch.parameters.prune_length_mm = 1.0,
                _ => mismatch.parameters.straightening_max_distance_mm = 1.0,
            }
            assert!(matches!(plan_lightning(&mismatch),
                Err(KernelError::RequiredSourceInputMismatch { field: actual, expected, actual: recorded, .. })
                    if actual == field && expected != recorded));
        }
        let mut mismatch = input;
        mismatch.parameters.supporting_radius_mm = None;
        assert!(matches!(
            plan_lightning(&mismatch),
            Err(KernelError::InvalidInput { .. })
        ));
    }

    fn assert_clip_occurrences_and_ambiguous_dispatch() {
        let mut input = input();
        let mut plan = plan_lightning(&input).unwrap();
        plan.layers[0].nodes = [
            (
                1,
                Point2 {
                    x: 50_000,
                    y: 100_000,
                },
            ),
            (
                2,
                Point2 {
                    x: 150_000,
                    y: 100_000,
                },
            ),
            (
                3,
                Point2 {
                    x: 100_000,
                    y: 150_000,
                },
            ),
        ]
        .into_iter()
        .map(|(node_id, point)| GraphNode {
            node_id,
            point,
            parent_id: None,
            root_id: node_id,
            origin_region_index: 1,
            grounding: None,
        })
        .collect(); // exhaustive: known directed edges and an isolated root falsify welding.
        plan.layers[0].edges = [(1, 1, 2), (2, 1, 2), (3, 2, 1)]
            .into_iter()
            .map(|(edge_id, parent_id, child_id)| GraphEdge {
                edge_id,
                parent_id,
                child_id,
                root_id: parent_id,
                origin_region_index: 1,
            })
            .collect(); // exhaustive: repeated and reversed occurrences retain exact order.
        set_fill(&mut input, rectangle(0, 0, 200_000, 200_000), "occurrences");
        let forward = [
            Point2 {
                x: 50_000,
                y: 100_000,
            },
            Point2 {
                x: 150_000,
                y: 100_000,
            },
        ];
        assert_eq!(
            clip_lightning(&plan, &input).unwrap()[0].fills[0].segments,
            vec![forward, forward, [forward[1], forward[0]]]
        );

        let mut ambiguous = input.clone();
        ambiguous.canonical_region_order.push(identity(7));
        assert!(matches!(clip_lightning(&plan, &ambiguous),
            Err(KernelError::InvalidInput { reason, .. })
                if reason.contains("occurrences") && reason.contains("ambiguous")));
        let mut ambiguous_plan = plan.clone();
        ambiguous_plan.layers.push(plan.layers[0].clone());
        assert!(matches!(clip_lightning(&ambiguous_plan, &input),
            Err(KernelError::InvalidInput { reason, .. }) if reason.contains("occurrences")));
        plan.layers[0].nodes.clear();
        assert!(matches!(clip_lightning(&plan, &input),
            Err(KernelError::InvalidInput { reason, .. }) if reason.contains("occurrences")));
        plan.layers.clear();
        assert!(matches!(clip_lightning(&plan, &input),
            Err(KernelError::InvalidInput { reason, .. }) if reason.contains("occurrences")));
    }
}
