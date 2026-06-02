//! Mesh segmentation algorithms.
//!
//! Normalize sub-facet paint strokes into whole-triangle assignments.

use std::sync::Arc;

use slicer_ir::{FacetPaintData, IndexedTriangleSet, MeshIR, PaintValue, Point3};

const GEOMETRY_EPSILON: f32 = 1.0e-6;

/// Deterministic reasons a projected paint stroke cannot be normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DegenerateStrokeReason {
    /// The projected stroke has zero area and cannot split a facet.
    ZeroAreaStrokeTriangle,
    /// The projected stroke only grazes an edge, so the split would be ambiguous.
    TangentToFacetEdge,
    /// The projected stroke only touches a triangle vertex, so ownership is ambiguous.
    TouchesFacetVertex,
}

/// Structured mesh-segmentation contract failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshSegmentationError {
    /// A stroke could not be normalized deterministically.
    DegenerateStroke {
        /// Object carrying the invalid paint stroke.
        object_id: String,
        /// Paint layer index within `FacetPaintData.layers`.
        layer_index: usize,
        /// Stroke index within `PaintLayer.strokes`.
        stroke_index: usize,
        /// Stable reason for rejection.
        reason: DegenerateStrokeReason,
    },
}

/// Normalize sub-facet paint strokes into whole-triangle assignments.
pub fn execute_mesh_segmentation(
    mesh_ir: Arc<MeshIR>,
) -> Result<Arc<MeshIR>, MeshSegmentationError> {
    if !mesh_has_subfacet_strokes(&mesh_ir) {
        return Ok(mesh_ir);
    }

    let mut normalized = (*mesh_ir).clone();

    for object in &mut normalized.objects {
        if let Some(paint_data) = object.paint_data.as_mut() {
            normalize_object(&object.id, &mut object.mesh, paint_data)?;
        }
    }

    Ok(Arc::new(normalized))
}

fn mesh_has_subfacet_strokes(mesh_ir: &MeshIR) -> bool {
    mesh_ir.objects.iter().any(|object| {
        object.paint_data.as_ref().is_some_and(|paint_data| {
            paint_data
                .layers
                .iter()
                .any(|layer| !layer.strokes.is_empty())
        })
    })
}

fn normalize_object(
    object_id: &str,
    mesh: &mut IndexedTriangleSet,
    paint_data: &mut FacetPaintData,
) -> Result<(), MeshSegmentationError> {
    for layer_index in 0..paint_data.layers.len() {
        let strokes = std::mem::take(&mut paint_data.layers[layer_index].strokes);

        for (stroke_index, stroke) in strokes.into_iter().enumerate() {
            for stroke_triangle in stroke.triangles {
                let facet_index = locate_target_facet(mesh, &stroke_triangle).ok_or_else(|| {
                    classify_degenerate_stroke(
                        object_id,
                        layer_index,
                        stroke_index,
                        &stroke_triangle,
                    )
                })?;

                let split = clip_triangle_at_stroke(mesh, facet_index, &stroke_triangle).map_err(
                    |reason| MeshSegmentationError::DegenerateStroke {
                        object_id: object_id.to_owned(),
                        layer_index,
                        stroke_index,
                        reason,
                    },
                )?;

                apply_triangle_split(
                    mesh,
                    paint_data,
                    layer_index,
                    facet_index,
                    split,
                    stroke.value.clone(),
                );
            }
        }
    }

    Ok(())
}

fn locate_target_facet(mesh: &IndexedTriangleSet, stroke_triangle: &[Point3; 3]) -> Option<usize> {
    let mut matching_facet = None;

    for facet_index in 0..(mesh.indices.len() / 3) {
        let facet = facet_points(mesh, facet_index)?;
        if stroke_triangle.iter().all(|point| {
            matches!(
                classify_point_in_triangle(*point, &facet),
                PointLocation::Vertex(_) | PointLocation::Edge(_, _) | PointLocation::Interior
            )
        }) {
            if matching_facet.is_some() {
                return None;
            }
            matching_facet = Some(facet_index);
        }
    }

    matching_facet
}

fn classify_degenerate_stroke(
    object_id: &str,
    layer_index: usize,
    stroke_index: usize,
    stroke_triangle: &[Point3; 3],
) -> MeshSegmentationError {
    let reason = if triangle_area_squared(stroke_triangle) <= GEOMETRY_EPSILON {
        DegenerateStrokeReason::ZeroAreaStrokeTriangle
    } else if has_repeated_point(stroke_triangle) {
        DegenerateStrokeReason::TouchesFacetVertex
    } else {
        DegenerateStrokeReason::TangentToFacetEdge
    };

    MeshSegmentationError::DegenerateStroke {
        object_id: object_id.to_owned(),
        layer_index,
        stroke_index,
        reason,
    }
}

fn clip_triangle_at_stroke(
    mesh: &mut IndexedTriangleSet,
    facet_index: usize,
    stroke_triangle: &[Point3; 3],
) -> Result<TriangleSplit, DegenerateStrokeReason> {
    let parent_indices = facet_vertex_indices(mesh, facet_index)
        .ok_or(DegenerateStrokeReason::TangentToFacetEdge)?;
    let parent_points =
        facet_points(mesh, facet_index).ok_or(DegenerateStrokeReason::TangentToFacetEdge)?;

    if triangle_area_squared(stroke_triangle) <= GEOMETRY_EPSILON {
        return Err(classify_zero_area_stroke(&parent_points, stroke_triangle));
    }

    let mut shared_vertices = Vec::new();
    let mut edge_point = None;
    let mut touches_facet_vertex = false;

    for point in stroke_triangle {
        match classify_point_in_triangle(*point, &parent_points) {
            PointLocation::Vertex(local_index) => {
                touches_facet_vertex = true;
                shared_vertices.push(local_index);
            }
            PointLocation::Edge(a, b) => {
                if point_nearly_eq(*point, parent_points[a])
                    || point_nearly_eq(*point, parent_points[b])
                {
                    return Err(DegenerateStrokeReason::TouchesFacetVertex);
                }

                if edge_point.replace((a, b, *point)).is_some() {
                    return Err(DegenerateStrokeReason::TangentToFacetEdge);
                }
            }
            PointLocation::Interior => {}
            PointLocation::Outside => {
                return Err(DegenerateStrokeReason::TangentToFacetEdge);
            }
        }
    }

    shared_vertices.sort_unstable();
    shared_vertices.dedup();

    if shared_vertices.len() != 2 {
        return Err(if touches_facet_vertex {
            DegenerateStrokeReason::TouchesFacetVertex
        } else {
            DegenerateStrokeReason::TangentToFacetEdge
        });
    }

    let (edge_a, edge_b, edge_point_value) =
        edge_point.ok_or(DegenerateStrokeReason::TangentToFacetEdge)?;
    let remaining_local = (0..3)
        .find(|index| !shared_vertices.contains(index))
        .ok_or(DegenerateStrokeReason::TangentToFacetEdge)?;

    let shared_a = shared_vertices[0];
    let shared_b = shared_vertices[1];
    let edge_matches_shared_a = edge_has_vertices(edge_a, edge_b, shared_a, remaining_local);
    let edge_matches_shared_b = edge_has_vertices(edge_a, edge_b, shared_b, remaining_local);

    if edge_matches_shared_a == edge_matches_shared_b {
        return Err(DegenerateStrokeReason::TangentToFacetEdge);
    }

    let edge_point_index = find_or_append_vertex(mesh, edge_point_value)?;
    let painted = if edge_matches_shared_a {
        [
            parent_indices[shared_a],
            edge_point_index,
            parent_indices[shared_b],
        ]
    } else {
        [
            parent_indices[shared_a],
            parent_indices[shared_b],
            edge_point_index,
        ]
    };
    let unpainted = if edge_matches_shared_a {
        [
            edge_point_index,
            parent_indices[remaining_local],
            parent_indices[shared_b],
        ]
    } else {
        [
            parent_indices[shared_a],
            edge_point_index,
            parent_indices[remaining_local],
        ]
    };

    Ok(TriangleSplit { painted, unpainted })
}

fn apply_triangle_split(
    mesh: &mut IndexedTriangleSet,
    paint_data: &mut FacetPaintData,
    target_layer_index: usize,
    facet_index: usize,
    split: TriangleSplit,
    paint_value: PaintValue,
) {
    let index_offset = facet_index * 3;
    mesh.indices.splice(
        index_offset..(index_offset + 3),
        [
            split.painted[0],
            split.painted[1],
            split.painted[2],
            split.unpainted[0],
            split.unpainted[1],
            split.unpainted[2],
        ],
    );

    for (layer_index, layer) in paint_data.layers.iter_mut().enumerate() {
        let original_value = layer.facet_values[facet_index].clone();
        let replacement = if layer_index == target_layer_index {
            [Some(paint_value.clone()), original_value]
        } else {
            [original_value.clone(), original_value]
        };
        layer
            .facet_values
            .splice(facet_index..(facet_index + 1), replacement);
    }
}

fn facet_vertex_indices(mesh: &IndexedTriangleSet, facet_index: usize) -> Option<[u32; 3]> {
    let base = facet_index.checked_mul(3)?;
    Some([
        *mesh.indices.get(base)?,
        *mesh.indices.get(base + 1)?,
        *mesh.indices.get(base + 2)?,
    ])
}

fn facet_points(mesh: &IndexedTriangleSet, facet_index: usize) -> Option<[Point3; 3]> {
    let indices = facet_vertex_indices(mesh, facet_index)?;
    Some([
        *mesh.vertices.get(indices[0] as usize)?,
        *mesh.vertices.get(indices[1] as usize)?,
        *mesh.vertices.get(indices[2] as usize)?,
    ])
}

fn find_or_append_vertex(
    mesh: &mut IndexedTriangleSet,
    point: Point3,
) -> Result<u32, DegenerateStrokeReason> {
    if let Some((index, _)) = mesh
        .vertices
        .iter()
        .enumerate()
        .find(|(_, existing)| point_nearly_eq(**existing, point))
    {
        return u32::try_from(index).map_err(|_| DegenerateStrokeReason::TangentToFacetEdge);
    }

    let next_index = u32::try_from(mesh.vertices.len())
        .map_err(|_| DegenerateStrokeReason::TangentToFacetEdge)?;
    mesh.vertices.push(point);
    Ok(next_index)
}

fn has_repeated_point(triangle: &[Point3; 3]) -> bool {
    point_nearly_eq(triangle[0], triangle[1])
        || point_nearly_eq(triangle[0], triangle[2])
        || point_nearly_eq(triangle[1], triangle[2])
}

fn classify_zero_area_stroke(
    parent_triangle: &[Point3; 3],
    stroke_triangle: &[Point3; 3],
) -> DegenerateStrokeReason {
    if has_repeated_point(stroke_triangle) {
        return DegenerateStrokeReason::ZeroAreaStrokeTriangle;
    }

    let mut matched_edge = None;

    for point in stroke_triangle {
        match classify_point_in_triangle(*point, parent_triangle) {
            PointLocation::Vertex(local_index) => {
                let incident_edge = stroke_triangle
                    .iter()
                    .filter(|candidate| !point_nearly_eq(**candidate, *point))
                    .find_map(|candidate| {
                        match classify_point_in_triangle(*candidate, parent_triangle) {
                            PointLocation::Edge(a, b) if a == local_index || b == local_index => {
                                Some((a, b))
                            }
                            _ => None,
                        }
                    });

                if let Some(edge) = incident_edge {
                    if matched_edge.get_or_insert(edge) != &edge {
                        return DegenerateStrokeReason::TangentToFacetEdge;
                    }
                    continue;
                }

                return DegenerateStrokeReason::TouchesFacetVertex;
            }
            PointLocation::Edge(a, b) => {
                if matched_edge.get_or_insert((a, b)) != &(a, b) {
                    return DegenerateStrokeReason::ZeroAreaStrokeTriangle;
                }
            }
            PointLocation::Interior | PointLocation::Outside => {
                return DegenerateStrokeReason::ZeroAreaStrokeTriangle;
            }
        }
    }

    DegenerateStrokeReason::TangentToFacetEdge
}

fn edge_has_vertices(edge_a: usize, edge_b: usize, expected_a: usize, expected_b: usize) -> bool {
    (edge_a == expected_a && edge_b == expected_b) || (edge_a == expected_b && edge_b == expected_a)
}

fn classify_point_in_triangle(point: Point3, triangle: &[Point3; 3]) -> PointLocation {
    let a = triangle[0];
    let b = triangle[1];
    let c = triangle[2];
    let v0 = sub(b, a);
    let v1 = sub(c, a);
    let v2 = sub(point, a);
    let normal = cross(v0, v1);

    if dot(normal, v2).abs() > GEOMETRY_EPSILON {
        return PointLocation::Outside;
    }

    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denom = d00 * d11 - d01 * d01;

    if denom.abs() <= GEOMETRY_EPSILON {
        return PointLocation::Outside;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    if u < -GEOMETRY_EPSILON || v < -GEOMETRY_EPSILON || w < -GEOMETRY_EPSILON {
        return PointLocation::Outside;
    }

    if nearly_zero(v) && nearly_zero(w) {
        return PointLocation::Vertex(0);
    }
    if nearly_zero(u) && nearly_zero(w) {
        return PointLocation::Vertex(1);
    }
    if nearly_zero(u) && nearly_zero(v) {
        return PointLocation::Vertex(2);
    }

    if nearly_zero(w) {
        return PointLocation::Edge(0, 1);
    }
    if nearly_zero(u) {
        return PointLocation::Edge(1, 2);
    }
    if nearly_zero(v) {
        return PointLocation::Edge(0, 2);
    }

    PointLocation::Interior
}

fn triangle_area_squared(triangle: &[Point3; 3]) -> f32 {
    let ab = sub(triangle[1], triangle[0]);
    let ac = sub(triangle[2], triangle[0]);
    let area_vector = cross(ab, ac);
    dot(area_vector, area_vector)
}

fn point_nearly_eq(left: Point3, right: Point3) -> bool {
    nearly_zero(left.x - right.x) && nearly_zero(left.y - right.y) && nearly_zero(left.z - right.z)
}

fn nearly_zero(value: f32) -> bool {
    value.abs() <= GEOMETRY_EPSILON
}

fn sub(left: Point3, right: Point3) -> [f32; 3] {
    [left.x - right.x, left.y - right.y, left.z - right.z]
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TriangleSplit {
    painted: [u32; 3],
    unpainted: [u32; 3],
}

enum PointLocation {
    Vertex(usize),
    Edge(usize, usize),
    Interior,
    Outside,
}

#[cfg(test)]
mod tests {
    use super::{clip_triangle_at_stroke, DegenerateStrokeReason};
    use slicer_ir::{IndexedTriangleSet, Point3};

    #[test]
    fn clip_triangle_at_stroke_rejects_zero_area_triangles() {
        let mut mesh = single_triangle_mesh();
        let stroke = [
            point3(0.5, 0.5, 0.0),
            point3(0.5, 0.5, 0.0),
            point3(0.5, 0.5, 0.0),
        ];

        assert_eq!(
            clip_triangle_at_stroke(&mut mesh, 0, &stroke),
            Err(DegenerateStrokeReason::ZeroAreaStrokeTriangle)
        );
    }

    #[test]
    fn clip_triangle_at_stroke_rejects_edge_tangent_strokes() {
        let mut mesh = single_triangle_mesh();
        let stroke = [
            point3(0.0, 0.0, 0.0),
            point3(1.0, 0.0, 0.0),
            point3(2.0, 0.0, 0.0),
        ];

        assert_eq!(
            clip_triangle_at_stroke(&mut mesh, 0, &stroke),
            Err(DegenerateStrokeReason::TangentToFacetEdge)
        );
    }

    #[test]
    fn clip_triangle_at_stroke_rejects_vertex_touching_strokes() {
        let mut mesh = single_triangle_mesh();
        let stroke = [
            point3(0.0, 0.0, 0.0),
            point3(0.5, 0.25, 0.0),
            point3(0.75, 0.25, 0.0),
        ];

        assert_eq!(
            clip_triangle_at_stroke(&mut mesh, 0, &stroke),
            Err(DegenerateStrokeReason::TouchesFacetVertex)
        );
    }

    fn single_triangle_mesh() -> IndexedTriangleSet {
        IndexedTriangleSet {
            vertices: vec![
                point3(0.0, 0.0, 0.0),
                point3(2.0, 0.0, 0.0),
                point3(1.0, 1.0, 0.0),
            ],
            indices: vec![0, 1, 2],
        }
    }

    fn point3(x: f32, y: f32, z: f32) -> Point3 {
        Point3 { x, y, z }
    }
}
