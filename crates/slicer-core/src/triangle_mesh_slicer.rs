//! Triangle mesh slicer implementation.
//!
//! Converts a 3D triangle mesh into a series of 2D ExPolygon layers at specified Z heights.

use slicer_ir::{ExPolygon, IndexedTriangleSet, Point2, Point3, Polygon};

/// Represents a line segment intersection with a slicing plane.
#[derive(Debug, Clone)]
struct IntersectionLine {
    a: Point2,
    b: Point2,
    a_edge_key: u64,
    b_edge_key: u64,
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

            // Find intersection points
            let mut points = Vec::new();

            // Edge 0: v0 -> v1
            if let Some(point) = intersect_edge(v0, v1, idx0 as i32, idx1 as i32, z_plane) {
                points.push(point);
            }
            // Edge 1: v1 -> v2
            if let Some(point) = intersect_edge(v1, v2, idx1 as i32, idx2 as i32, z_plane) {
                points.push(point);
            }
            // Edge 2: v2 -> v0
            if let Some(point) = intersect_edge(v2, v0, idx2 as i32, idx0 as i32, z_plane) {
                points.push(point);
            }

            // Should have exactly 2 intersection points for a valid slice
            if points.len() == 2 {
                // Order points to ensure consistent winding (external on right)
                let line = IntersectionLine {
                    a: points[1].0,
                    b: points[0].0,
                    a_edge_key: points[1].1,
                    b_edge_key: points[0].1,
                };
                layers_lines[layer_idx].push(line);
            }
        }
    }

    layers_lines
}

/// Compute intersection of an edge with a Z plane
/// Returns Some((point, edge_key)) if intersection exists
/// edge_key is a unique identifier for the mesh edge (min_vertex << 32 | max_vertex)
fn intersect_edge(
    v1: &Point3,
    v2: &Point3,
    id1: i32,
    id2: i32,
    z_plane: f32,
) -> Option<(Point2, u64)> {
    let z1 = v1.z;
    let z2 = v2.z;

    // Create unique edge key (order-independent)
    let edge_key = ((id1.min(id2) as u64) << 32) | (id1.max(id2) as u64);

    // Check if edge crosses the plane
    if (z1 - z_plane).abs() < 1e-6 {
        // Vertex is exactly on plane
        return Some((Point2::from_mm(v1.x, v1.y), edge_key));
    }
    if (z2 - z_plane).abs() < 1e-6 {
        // Vertex is exactly on plane
        return Some((Point2::from_mm(v2.x, v2.y), edge_key));
    }

    // Check if edge straddles the plane
    if (z1 < z_plane && z2 > z_plane) || (z1 > z_plane && z2 < z_plane) {
        // Linear interpolation
        let t = (z_plane - z1) / (z2 - z1);
        let x = v1.x + t * (v2.x - v1.x);
        let y = v1.y + t * (v2.y - v1.y);

        return Some((Point2::from_mm(x, y), edge_key));
    }

    None
}

/// Phase 2: Chain intersection lines into polygons and convert to ExPolygons
fn chain_lines_to_expolygons(lines: Vec<IntersectionLine>) -> Vec<ExPolygon> {
    if lines.is_empty() {
        return Vec::new();
    }

    // Chain lines into polylines using vertex IDs
        let polylines = chain_lines(lines);

    // Convert polylines to Polygons (closed loops)
    let polygons: Vec<Polygon> = polylines
        .into_iter()
        .filter_map(|polyline| {
            if polyline.len() >= 3 {
                Some(Polygon { points: polyline })
            } else {
                None
            }
        })
        .collect();

    // Convert Polygons to ExPolygons using boolean union
    // For simple cases (no holes), this just wraps each polygon
    // For complex cases, union handles nesting
    polygons_to_expolygons(&polygons)
}

/// Chain lines into polylines based on edge connectivity
/// Also merges collinear segments to simplify the polygon
fn chain_lines(lines: Vec<IntersectionLine>) -> Vec<Vec<Point2>> {
    use std::collections::HashMap;

    // Build adjacency map: edge_key -> list of (other_edge_key, line_index)
    let mut adjacency: HashMap<u64, Vec<(u64, usize)>> = HashMap::new();
    for (idx, line) in lines.iter().enumerate() {
        adjacency.entry(line.a_edge_key).or_default().push((line.b_edge_key, idx));
        adjacency.entry(line.b_edge_key).or_default().push((line.a_edge_key, idx));
    }

    let mut used = vec![false; lines.len()];
    let mut polylines = Vec::new();

    for (start_idx, line) in lines.iter().enumerate() {
        if used[start_idx] {
            continue;
        }

        // Start a new polyline from this line
        let mut polyline = vec![line.a, line.b];
        used[start_idx] = true;

        // Extend forward from b_edge_key
        let mut current_edge = line.b_edge_key;
        while let Some(neighbors) = adjacency.get(&current_edge) {
            // Find a neighbor that isn't the current line and hasn't been used
            let next = neighbors.iter().find(|(neighbor_edge, line_idx)| {
                !used[*line_idx] && *neighbor_edge != line.a_edge_key
            });

            if let Some((_next_edge, next_idx)) = next {
                let next_line = &lines[*next_idx];
                // Determine direction: if next_line.a_edge_key matches current_edge, use next_line.b
                // else use next_line.a
                let next_point = if next_line.a_edge_key == current_edge {
                    next_line.b
                } else {
                    next_line.a
                };

                // Check if the new point is collinear with the last two points in polyline
                if let Some(&last) = polyline.last() {
                    if let Some(&second_last) = polyline.get(polyline.len() - 2) {
                        // Check collinearity: cross product of (last - second_last) and (next_point - last) == 0
                        let dx1 = last.x - second_last.x;
                        let dy1 = last.y - second_last.y;
                        let dx2 = next_point.x - last.x;
                        let dy2 = next_point.y - last.y;
                        let cross = dx1 * dy2 - dy1 * dx2;
                        if cross == 0 {
                            // Collinear: replace the last point with the new point
                            *polyline.last_mut().unwrap() = next_point;
                        } else {
                            polyline.push(next_point);
                        }
                    } else {
                        polyline.push(next_point);
                    }
                } else {
                    polyline.push(next_point);
                }

                if next_line.a_edge_key == current_edge {
                    current_edge = next_line.b_edge_key;
                } else {
                    current_edge = next_line.a_edge_key;
                }
                used[*next_idx] = true;
            } else {
                break;
            }
        }

        // Extend backward from a_edge_key
        let mut current_edge = line.a_edge_key;
        while let Some(neighbors) = adjacency.get(&current_edge) {
            let next = neighbors.iter().find(|(neighbor_edge, line_idx)| {
                !used[*line_idx] && *neighbor_edge != line.b_edge_key
            });

            if let Some((_next_edge, next_idx)) = next {
                let next_line = &lines[*next_idx];
                let next_point = if next_line.a_edge_key == current_edge {
                    next_line.b
                } else {
                    next_line.a
                };

                // Check collinearity with the first two points
                if let Some(&first) = polyline.first() {
                    if let Some(&second) = polyline.get(1) {
                        let dx1 = second.x - first.x;
                        let dy1 = second.y - first.y;
                        let dx2 = next_point.x - first.x;
                        let dy2 = next_point.y - first.y;
                        let cross = dx1 * dy2 - dy1 * dx2;
                        if cross == 0 {
                            // Collinear: replace the first point with the new point
                            polyline[0] = next_point;
                        } else {
                            polyline.insert(0, next_point);
                        }
                    } else {
                        polyline.insert(0, next_point);
                    }
                } else {
                    polyline.insert(0, next_point);
                }

                if next_line.a_edge_key == current_edge {
                    current_edge = next_line.b_edge_key;
                } else {
                    current_edge = next_line.a_edge_key;
                }
                used[*next_idx] = true;
            } else {
                break;
            }
        }

        // Check if polyline is closed (first and last point are the same geometrically or close enough)
        if polyline.len() >= 3 {
            // Remove the last point if it's the same as the first (closing point)
            if let Some(first) = polyline.first() {
                if let Some(last) = polyline.last() {
                    if first == last {
                        polyline.pop();
                    }
                }
            }
            polylines.push(polyline);
        }
    }

    polylines
}

/// Convert polygons to ExPolygons using boolean union
fn polygons_to_expolygons(polygons: &[Polygon]) -> Vec<ExPolygon> {
    if polygons.is_empty() {
        return Vec::new();
    }

    use clipper2_rust::{Point64, FillRule};

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
            Point3 { x: 0.0, y: 0.0, z: 0.0 },
            Point3 { x: 1.0, y: 0.0, z: 0.0 },
            Point3 { x: 1.0, y: 1.0, z: 0.0 },
            Point3 { x: 0.0, y: 1.0, z: 0.0 },
            // Top face (z=1)
            Point3 { x: 0.0, y: 0.0, z: 1.0 },
            Point3 { x: 1.0, y: 0.0, z: 1.0 },
            Point3 { x: 1.0, y: 1.0, z: 1.0 },
            Point3 { x: 0.0, y: 1.0, z: 1.0 },
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
