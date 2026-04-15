//! Triangle mesh slicer implementation.
//!
//! Converts a 3D triangle mesh into a series of 2D ExPolygon layers at specified Z heights.

use slicer_ir::{ExPolygon, IndexedTriangleSet, Point2, Point3, Polygon};

use std::collections::HashMap;

/// Represents a line segment intersection with a slicing plane.
#[derive(Debug, Clone)]
struct IntersectionLine {
    a: Point2,
    b: Point2,
    a_topology: EndpointTopology,
    b_topology: EndpointTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EndpointTopology {
    Vertex(i32),
    Edge(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntersectionPoint {
    point: Point2,
    topology: EndpointTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VertexPlaneRelation {
    Below,
    On,
    Above,
}

/// Converts a 3D triangle mesh into 2D ExPolygon layers at specified Z heights.
///
/// # Arguments
/// * `mesh` - The input triangle mesh (vertices and indices)
/// * `zs` - List of Z heights to slice at (in millimeters)
///
/// # Returns
/// A vector of layers, where each layer is a vector of ExPolygons
pub fn slice_mesh_ex(mesh: &IndexedTriangleSet, zs: &[f32]) -> Vec<Vec<ExPolygon>> {
    if zs.is_empty() || mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return vec![vec![]; zs.len()];
    }

    // Phase 1: Generate intersection lines for each layer
    let layers_lines = slice_make_lines(mesh, zs);

    // Phase 2: Chain lines into polygons and convert to ExPolygons
    layers_lines
        .into_iter()
        .map(|lines| chain_lines_to_expolygons(lines))
        .collect()
}

/// Phase 1: Generate intersection lines for each layer
fn slice_make_lines(mesh: &IndexedTriangleSet, zs: &[f32]) -> Vec<Vec<IntersectionLine>> {
    let mut layers_lines: Vec<Vec<IntersectionLine>> = vec![Vec::new(); zs.len()];

    // Iterate over all triangles
    for chunk in mesh.indices.chunks(3) {
        if chunk.len() < 3 {
            continue;
        }

        let idx0 = chunk[0] as usize;
        let idx1 = chunk[1] as usize;
        let idx2 = chunk[2] as usize;

        // Get triangle vertices
        let v0 = &mesh.vertices[idx0];
        let v1 = &mesh.vertices[idx1];
        let v2 = &mesh.vertices[idx2];

        // Find min and max Z of the triangle
        let min_z = v0.z.min(v1.z).min(v2.z);
        let max_z = v0.z.max(v1.z).max(v2.z);

        // Skip triangles that don't intersect any slicing plane
        if max_z < zs[0] || min_z > zs[zs.len() - 1] {
            continue;
        }

        // Find which layers this triangle intersects
        let min_layer_idx = match zs.binary_search_by(|z| z.partial_cmp(&min_z).unwrap()) {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        let max_layer_idx = match zs.binary_search_by(|z| z.partial_cmp(&max_z).unwrap()) {
            Ok(idx) => idx,
            Err(idx) => idx.min(zs.len() - 1),
        };

        // For each layer this triangle intersects, compute intersection line
        for layer_idx in min_layer_idx..=max_layer_idx {
            let z_plane = zs[layer_idx];

            // Skip horizontal triangles (all vertices on same Z plane)
            if (v0.z - v1.z).abs() < 1e-6
                && (v1.z - v2.z).abs() < 1e-6
                && (v2.z - v0.z).abs() < 1e-6
            {
                continue;
            }

            // Check if triangle straddles the plane
            if v0.z < z_plane && v1.z < z_plane && v2.z < z_plane {
                continue;
            }
            if v0.z > z_plane && v1.z > z_plane && v2.z > z_plane {
                continue;
            }

            let points = triangle_intersections(
                [v0, v1, v2],
                [idx0 as i32, idx1 as i32, idx2 as i32],
                z_plane,
            );

            // Should have exactly 2 intersection points for a valid slice
            if points.len() == 2 {
                // Order points to ensure consistent winding (external on right)
                let line = IntersectionLine {
                    a: points[1].point,
                    b: points[0].point,
                    a_topology: points[1].topology,
                    b_topology: points[0].topology,
                };
                layers_lines[layer_idx].push(line);
            }
        }
    }

    layers_lines
}

fn triangle_intersections(
    vertices: [&Point3; 3],
    vertex_ids: [i32; 3],
    z_plane: f32,
) -> Vec<IntersectionPoint> {
    let relations = vertices.map(|vertex| classify_vertex(vertex.z, z_plane));
    let on_plane = relations
        .iter()
        .filter(|relation| **relation == VertexPlaneRelation::On)
        .count();
    let above_plane = relations
        .iter()
        .filter(|relation| **relation == VertexPlaneRelation::Above)
        .count();
    let below_plane = relations
        .iter()
        .filter(|relation| **relation == VertexPlaneRelation::Below)
        .count();

    if on_plane == 3 || on_plane == 2 || above_plane == 0 || below_plane == 0 {
        return Vec::new();
    }

    let mut intersections = Vec::new();
    for (start, end) in [(0usize, 1usize), (1, 2), (2, 0)] {
        if let Some(point) = intersect_edge(
            vertices[start],
            vertices[end],
            vertex_ids[start],
            vertex_ids[end],
            relations[start],
            relations[end],
            z_plane,
        ) {
            push_unique_intersection(&mut intersections, point);
        }
    }

    intersections
}

fn classify_vertex(z: f32, z_plane: f32) -> VertexPlaneRelation {
    const EPSILON: f32 = 1e-6;

    if (z - z_plane).abs() < EPSILON {
        VertexPlaneRelation::On
    } else if z < z_plane {
        VertexPlaneRelation::Below
    } else {
        VertexPlaneRelation::Above
    }
}

fn push_unique_intersection(
    intersections: &mut Vec<IntersectionPoint>,
    candidate: IntersectionPoint,
) {
    if intersections.iter().any(|existing| {
        existing.topology == candidate.topology || existing.point == candidate.point
    }) {
        return;
    }

    intersections.push(candidate);
}

fn intersect_edge(
    v1: &Point3,
    v2: &Point3,
    id1: i32,
    id2: i32,
    relation1: VertexPlaneRelation,
    relation2: VertexPlaneRelation,
    z_plane: f32,
) -> Option<IntersectionPoint> {
    match (relation1, relation2) {
        (VertexPlaneRelation::On, VertexPlaneRelation::Above)
        | (VertexPlaneRelation::On, VertexPlaneRelation::Below) => Some(IntersectionPoint {
            point: Point2::from_mm(v1.x, v1.y),
            topology: EndpointTopology::Vertex(id1),
        }),
        (VertexPlaneRelation::Above, VertexPlaneRelation::On)
        | (VertexPlaneRelation::Below, VertexPlaneRelation::On) => Some(IntersectionPoint {
            point: Point2::from_mm(v2.x, v2.y),
            topology: EndpointTopology::Vertex(id2),
        }),
        (VertexPlaneRelation::Above, VertexPlaneRelation::Below)
        | (VertexPlaneRelation::Below, VertexPlaneRelation::Above) => {
            // Canonicalize interpolation by vertex ID. Two triangles sharing
            // an edge traverse it in opposite winding orders, which makes the
            // naive `v1 + t*(v2-v1)` formula produce slightly different
            // results after f32 rounding on each triangle. That desyncs the
            // downstream chain walker (points on the same physical edge no
            // longer compare equal across adjacent triangles). Always
            // interpolate from the lower-id endpoint to the higher-id
            // endpoint so both neighbors produce bitwise-identical points.
            let (a, b) = if id1 < id2 { (v1, v2) } else { (v2, v1) };
            let t = (z_plane - a.z) / (b.z - a.z);
            let x = a.x + t * (b.x - a.x);
            let y = a.y + t * (b.y - a.y);

            Some(IntersectionPoint {
                point: Point2::from_mm(x, y),
                topology: EndpointTopology::Edge(edge_key(id1, id2)),
            })
        }
        _ => None,
    }
}

fn edge_key(id1: i32, id2: i32) -> u64 {
    ((id1.min(id2) as u64) << 32) | (id1.max(id2) as u64)
}

/// Phase 2: Chain intersection lines into polygons and convert to ExPolygons
fn chain_lines_to_expolygons(lines: Vec<IntersectionLine>) -> Vec<ExPolygon> {
    if lines.is_empty() {
        return Vec::new();
    }

    let polygons = chain_lines(lines);

    // Convert Polygons to ExPolygons using boolean union
    // For simple cases (no holes), this just wraps each polygon
    // For complex cases, union handles nesting
    polygons_to_expolygons(&polygons)
}

/// Chain intersection lines into closed polygons using undirected
/// point connectivity.
///
/// Each physical intersection point lies on a unique mesh edge (or at a
/// mesh vertex on the slicing plane). For a 2-manifold mesh, a clean
/// slice produces a closed loop per island, where each point is shared
/// by exactly two lines. The per-line `a`/`b` ordering depends on
/// triangle winding and edge-iteration order, and is not consistent
/// across adjacent triangles — a directed `a → b` walk fragments chains
/// and leaves shared edges orphaned. Treating the connection as
/// undirected recovers the full loop regardless of the emitted
/// orientation on each side of each shared edge.
///
/// The canonical edge-interpolation in `intersect_edge` gives bitwise-
/// identical endpoint coordinates on both triangles of a shared edge,
/// so point equality is reliable here.
fn chain_lines(lines: Vec<IntersectionLine>) -> Vec<Polygon> {
    if lines.is_empty() {
        return Vec::new();
    }

    // Undirected endpoint index: each point maps to the list of line
    // indices touching it at either end.
    let mut by_point: HashMap<Point2, Vec<usize>> = HashMap::new();
    for (idx, line) in lines.iter().enumerate() {
        by_point.entry(line.a).or_default().push(idx);
        by_point.entry(line.b).or_default().push(idx);
    }

    let mut used = vec![false; lines.len()];
    let mut polygons = Vec::new();

    for start_idx in 0..lines.len() {
        if used[start_idx] {
            continue;
        }

        used[start_idx] = true;
        let start_line = &lines[start_idx];
        let mut loop_points: Vec<Point2> = vec![start_line.a, start_line.b];
        let mut current_point = start_line.b;
        let loop_closed = loop {
            let Some(&next_idx) = by_point
                .get(&current_point)
                .and_then(|candidates| candidates.iter().find(|&&i| !used[i]))
            else {
                break false;
            };
            used[next_idx] = true;
            let next_line = &lines[next_idx];
            // Step to the endpoint of `next_line` that is NOT the point
            // we arrived at; that's the walker's new frontier.
            let next_point = if next_line.a == current_point {
                next_line.b
            } else {
                next_line.a
            };
            if next_point == start_line.a {
                // Closed the loop; don't re-push the start point.
                break true;
            }
            loop_points.push(next_point);
            current_point = next_point;
        };

        if loop_closed && loop_points.len() >= 3 {
            polygons.push(Polygon {
                points: simplify_polygon_points(loop_points),
            });
        }
    }

    polygons
}

fn find_unused_line(candidates: Option<&Vec<usize>>, used: &[bool]) -> Option<usize> {
    candidates?
        .iter()
        .copied()
        .find(|candidate_idx| !used[*candidate_idx])
}

fn simplify_polygon_points(mut points: Vec<Point2>) -> Vec<Point2> {
    loop {
        if points.len() < 3 {
            return points;
        }

        let mut changed = false;
        let len = points.len();
        for idx in 0..len {
            let prev = points[(idx + len - 1) % len];
            let current = points[idx];
            let next = points[(idx + 1) % len];

            if is_collinear(prev, current, next) {
                points.remove(idx);
                changed = true;
                break;
            }
        }

        if !changed {
            return points;
        }
    }
}

fn is_collinear(a: Point2, b: Point2, c: Point2) -> bool {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let bcx = c.x - b.x;
    let bcy = c.y - b.y;
    abx * bcy - aby * bcx == 0
}

/// Convert polygons to ExPolygons using boolean union
fn polygons_to_expolygons(polygons: &[Polygon]) -> Vec<ExPolygon> {
    if polygons.is_empty() {
        return Vec::new();
    }

    use clipper2_rust::{FillRule, Point64};

    // Convert polygons to clipper paths
    let paths: Vec<Vec<Point64>> = polygons
        .iter()
        .map(|poly| {
            poly.points
                .iter()
                .map(|p| Point64 { x: p.x, y: p.y })
                .collect()
        })
        .collect();

    // Perform union to resolve nesting and holes
    // If we have multiple polygons, union them to get proper hierarchy
    let result_paths = if paths.len() > 1 {
        // Union all paths together
        let mut result = vec![paths[0].clone()];
        for path in paths.iter().skip(1) {
            // Wrap the single path in a Vec to match function signature
            let clips: Vec<Vec<Point64>> = vec![path.clone()];
            result = clipper2_rust::union_64(&result, &clips, FillRule::NonZero);
        }
        result
    } else {
        paths
    };

    // Convert result paths back to ExPolygons
    // For simplicity, treat each path as a separate ExPolygon with no holes
    // In a full implementation, we'd need to detect which paths are holes
    result_paths
        .into_iter()
        .map(|path| {
            let points: Vec<Point2> = path
                .into_iter()
                .map(|p| Point2 { x: p.x, y: p.y })
                .collect();
            ExPolygon {
                contour: Polygon { points },
                holes: Vec::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_mesh() {
        let mesh = IndexedTriangleSet {
            vertices: vec![],
            indices: vec![],
        };
        let zs = vec![0.0, 0.5, 1.0];
        let result = slice_mesh_ex(&mesh, &zs);
        assert_eq!(result.len(), 3);
        assert!(result[0].is_empty());
        assert!(result[1].is_empty());
        assert!(result[2].is_empty());
    }

    #[test]
    fn test_cube_sliced_at_half_height() {
        // Create a unit cube from (0,0,0) to (1,1,1)
        let vertices = vec![
            // Bottom face (z=0)
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 1.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            // Top face (z=1)
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 1.0,
            },
        ];

        // 12 triangles (2 per face)
        let indices = vec![
            // Bottom face (z=0) - 2 triangles
            0, 2, 1, // triangle 1
            0, 3, 2, // triangle 2
            // Top face (z=1) - 2 triangles
            4, 5, 6, // triangle 3
            4, 6, 7, // triangle 4
            // Side faces
            // Front face (y=0)
            0, 1, 5, // triangle 5
            0, 5, 4, // triangle 6
            // Right face (x=1)
            1, 2, 6, // triangle 7
            1, 6, 5, // triangle 8
            // Back face (y=1)
            2, 3, 7, // triangle 9
            2, 7, 6, // triangle 10
            // Left face (x=0)
            3, 0, 4, // triangle 11
            3, 4, 7, // triangle 12
        ];

        let mesh = IndexedTriangleSet { vertices, indices };
        let zs = vec![0.5];
        let result = slice_mesh_ex(&mesh, &zs);

        // Should produce one layer
        assert_eq!(result.len(), 1);

        // Should contain one polygon (the square cross-section)
        let layer = &result[0];
        assert_eq!(layer.len(), 1);

        // Check the polygon has 4 points (square)
        let expolygon = &layer[0];
        assert_eq!(expolygon.contour.points.len(), 4);
        assert!(expolygon.holes.is_empty());

        // Check points are at correct locations (scaled integers)
        let expected_points = vec![
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(1.0, 0.0),
            Point2::from_mm(1.0, 1.0),
            Point2::from_mm(0.0, 1.0),
        ];

        // Check if the contour points match
        for point in &expolygon.contour.points {
            let is_valid = expected_points.iter().any(|p| p == point);
            assert!(is_valid, "Unexpected point: {:?}", point);
        }
    }
}
