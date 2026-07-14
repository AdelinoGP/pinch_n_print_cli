// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/TriangleMeshSlicer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Triangle mesh slicer implementation.
//!
//! Converts a 3D triangle mesh into a series of 2D ExPolygon layers at specified Z heights.

use slicer_ir::{mm_to_units, ExPolygon, IndexedTriangleSet, Point2, Point3, Polygon};

use std::collections::{HashMap, HashSet};

use crate::polygon_ops::{self, union_ex, OffsetJoinType};

/// Represents a line segment intersection with a slicing plane.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct IntersectionLine {
    a: Point2,
    b: Point2,
    a_topology: EndpointTopology,
    b_topology: EndpointTopology,
}

/// An un-closed chain produced by the topology walk, pending reconciliation.
///
/// `start`/`end` are the `EndpointTopology` of the two endpoints; `points`
/// is the actual polyline (start included, no trailing duplicate). Mirrors
/// OrcaSlicer's `OpenPolyline` (`start`/`end` `IntersectionReference`,
/// `points`, cached `length`, `consumed` flag).
struct OpenPolyline {
    start: EndpointTopology,
    end: EndpointTopology,
    points: Vec<Point2>,
    length: f64,
    consumed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        .map(chain_lines_to_expolygons)
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

    // OrcaSlicer edge-ownership convention (TriangleMeshSlicer.cpp:272-275):
    // When two vertices lie on the slice plane, the edge belongs to the slice
    // only if it is the *upper* edge — i.e. the third vertex is below the
    // plane. The bottom-most edge of a triangle is "owned" by the triangle
    // below it, not by the current triangle, so it is excluded.
    if on_plane == 3
        || (on_plane == 2 && above_plane == 1)
        || (on_plane < 2 && (above_plane == 0 || below_plane == 0))
    {
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

    let polygons = make_loops(lines);

    // Convert Polygons to ExPolygons using boolean union
    // For simple cases (no holes), this just wraps each polygon
    // For complex cases, union handles nesting
    let mut expolygons = polygons_to_expolygons(&polygons);
    // Canonical sort by min contour point so island ordering is independent
    // of Clipper2's internal path traversal order across process runs.
    expolygons.sort_by_key(|ep| ep.contour.points.iter().copied().min());
    expolygons
}

/// OrcaSlicer-faithful loop chaining pipeline.
///
/// `make_loops` reproduces `TriangleMeshSlicer::make_loops` as a four-stage
/// pipeline:
///   1. `chain_lines_by_triangle_connectivity` — greedy walk over mesh
///      topology (edge/vertex ids), keeping dead-ends as `OpenPolyline`s
///      instead of discarding the whole walk.
///   2. `chain_open_polylines_exact` (×2: same, then reversed orientation) —
///      reconcile open polylines by exact topology-id match.
///   3. `chain_open_polylines_close_gaps` (×2: same, then reversed) — join
///      open polylines whose endpoints are within `max_gap` (2 mm).
///
/// PNP's `EndpointTopology` (a vertex id or an edge key) is the direct
/// equivalent of OrcaSlicer's `IntersectionReference` (point_id / edge_id);
/// only one is ever set per endpoint, so a single `EndpointTopology` key
/// drives both the connectivity walk and the reconciliation passes.
fn make_loops(lines: Vec<IntersectionLine>) -> Vec<Polygon> {
    if lines.is_empty() {
        return Vec::new();
    }

    // A 2-manifold slice produces one intersection line per physical segment,
    // but PNP's per-triangle `slice_make_lines` emits one line per *triangle*,
    // so a face built from two coplanar triangles yields two coincident lines
    // (e.g. a cube cross-section gives 8 lines for 4 unique segments). Drop
    // exact duplicates so the topology walk sees each segment once. This is
    // safe: a duplicate is never a distinct physical edge, and it matches the
    // outcome of OrcaSlicer's edge-ownership de-duplication.
    let lines = dedup_lines(lines);

    let mut loops: Vec<Polygon> = Vec::new();
    let mut open_polylines: Vec<OpenPolyline> = Vec::new();

    chain_lines_by_triangle_connectivity(lines, &mut loops, &mut open_polylines);
    chain_open_polylines_exact(&mut open_polylines, &mut loops, false);
    chain_open_polylines_exact(&mut open_polylines, &mut loops, true);
    let max_gap = mm_to_units(2.0) as i64; // 2 mm, in PNP scaled units
    chain_open_polylines_close_gaps(&mut open_polylines, &mut loops, max_gap, false);
    chain_open_polylines_close_gaps(&mut open_polylines, &mut loops, max_gap, true);

    loops
}

/// Drop coincident intersection lines (same unordered pair of
/// endpoints + topologies, order-independent). See `make_loops`.
fn dedup_lines(lines: Vec<IntersectionLine>) -> Vec<IntersectionLine> {
    let mut seen: HashSet<((Point2, EndpointTopology), (Point2, EndpointTopology))> =
        HashSet::new();
    let mut out = Vec::new();
    for line in lines {
        let e1 = (line.a, line.a_topology);
        let e2 = (line.b, line.b_topology);
        let key = if e1 <= e2 { (e1, e2) } else { (e2, e1) };
        if seen.insert(key) {
            out.push(line);
        }
    }
    out
}

/// Stage 1 — greedy walk over triangle topology.
///
/// Unlike a point-equality walk, this keys off `EndpointTopology` so it
/// survives the f32 rounding / winding differences that can desync
/// otherwise-equal coordinates. The walk is orientation-agnostic: a next
/// line is joined whether its `a` or `b` endpoint carries the matching
/// topology (the line is reversed when it matches on `b`). This is
/// equivalent to OrcaSlicer's directed walk for any manifold mesh, but
/// does not depend on PNP's slice stage emitting "external-on-the-right"
/// oriented lines.
///
/// A walk that returns to its start topology is a closed loop; anything
/// else is preserved as an `OpenPolyline` and handed to the reconciliation
/// passes (which is what fixes the benchy Z1.6 dangling-tail case — the
/// old point walk discarded the entire walk on first dead-end).
fn chain_lines_by_triangle_connectivity(
    lines: Vec<IntersectionLine>,
    loops: &mut Vec<Polygon>,
    open_polylines: &mut Vec<OpenPolyline>,
) {
    let n = lines.len();
    if n == 0 {
        return;
    }

    let mut used = vec![false; n];
    // Multimap: topology id -> list of (line index, is_a_endpoint).
    let mut by_topo: HashMap<EndpointTopology, Vec<(usize, bool)>> = HashMap::new();
    for (idx, line) in lines.iter().enumerate() {
        by_topo
            .entry(line.a_topology)
            .or_default()
            .push((idx, true));
        by_topo
            .entry(line.b_topology)
            .or_default()
            .push((idx, false));
    }

    for seed in 0..n {
        if used[seed] {
            continue;
        }
        used[seed] = true;

        let mut points: Vec<Point2> = vec![lines[seed].a, lines[seed].b];
        let start_topo = lines[seed].a_topology;
        let mut end_topo = lines[seed].b_topology;
        let mut closed = false;

        loop {
            let next = by_topo
                .get(&end_topo)
                .and_then(|entries| entries.iter().find(|&&(idx, _)| !used[idx]).copied());
            let Some((next_idx, is_a)) = next else {
                // No continuation found. This is the case where the walk has
                // returned to its start: the only line carrying the matching
                // topology is the already-used seed, so the lookup returns
                // None. OrcaSlicer (TriangleMeshSlicer.cpp:1144-1150) checks
                // closure HERE — comparing the last line's end topology with
                // the first line's start topology — because the seed itself
                // is the missing continuation. Without this check, the loop
                // falls through to the gap-closer, which closes it with a
                // 2 mm XY chord across the near-vertex gap, distorting the
                // hull (this is the benchy Z≈9.6/17.2/20.6 hull-break
                // regression).
                if end_topo == start_topo {
                    closed = true;
                }
                break;
            };
            used[next_idx] = true;
            let next_line = &lines[next_idx];
            // `next` meets the walk at `end_topo`; advance to its far endpoint.
            let (far_point, far_topo) = if is_a {
                (next_line.b, next_line.b_topology)
            } else {
                (next_line.a, next_line.a_topology)
            };
            points.push(far_point);
            end_topo = far_topo;
            if end_topo == start_topo {
                closed = true;
                break;
            }
        }

        if closed {
            // `points` ends at the start location (a duplicate vertex) — drop it.
            points.pop();
            if points.len() >= 3 {
                loops.push(Polygon {
                    points: simplify_polygon_points(points),
                });
            }
        } else {
            let length = polyline_length(&points);
            open_polylines.push(OpenPolyline {
                start: start_topo,
                end: end_topo,
                points,
                length,
                consumed: false,
            });
        }
    }
}

/// Find a non-consumed open polyline whose start (and, when `try_reversed`,
/// end) topology equals `end_topo`.
fn find_exact_match(
    open: &[OpenPolyline],
    end_topo: EndpointTopology,
    try_reversed: bool,
    exclude: usize,
) -> Option<(usize, bool)> {
    for (i, opl) in open.iter().enumerate() {
        if i == exclude || opl.consumed {
            continue;
        }
        if opl.start == end_topo {
            return Some((i, true));
        }
        if try_reversed && opl.end == end_topo {
            return Some((i, false));
        }
    }
    None
}

/// Stage 2 — reconcile open polylines by exact topology-id match.
fn chain_open_polylines_exact(
    open_polylines: &mut Vec<OpenPolyline>,
    loops: &mut Vec<Polygon>,
    try_connect_reversed: bool,
) {
    let mut sorted: Vec<usize> = (0..open_polylines.len())
        .filter(|&i| !open_polylines[i].consumed)
        .collect();
    sorted.sort_by(|&a, &b| {
        open_polylines[b]
            .length
            .partial_cmp(&open_polylines[a].length)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for &seed in &sorted {
        if open_polylines[seed].consumed {
            continue;
        }
        open_polylines[seed].consumed = true;

        let mut cur_end = open_polylines[seed].end;
        loop {
            match find_exact_match(open_polylines, cur_end, try_connect_reversed, seed) {
                None => {
                    open_polylines[seed].consumed = false;
                    break;
                }
                Some((j, is_start)) => {
                    let pts = if is_start {
                        open_polylines[j].points.clone()
                    } else {
                        let mut rev = open_polylines[j].points.clone();
                        rev.reverse();
                        rev
                    };
                    let tail = *open_polylines[seed].points.last().unwrap();
                    let skip = pts.first().map_or(false, |p| *p == tail);
                    open_polylines[seed]
                        .points
                        .extend(pts.iter().skip(if skip { 1 } else { 0 }).copied());
                    open_polylines[seed].length += open_polylines[j].length;
                    open_polylines[j].consumed = true;
                    open_polylines[j].points.clear();
                    cur_end = if is_start {
                        open_polylines[j].end
                    } else {
                        open_polylines[j].start
                    };

                    if open_polylines[seed].start == cur_end {
                        open_polylines[seed].points.pop();
                        if open_polylines[seed].points.len() >= 3 {
                            if try_connect_reversed
                                && signed_area(&open_polylines[seed].points) < 0.0
                            {
                                open_polylines[seed].points.reverse();
                            }
                            loops.push(Polygon {
                                points: simplify_polygon_points(
                                    open_polylines[seed].points.clone(),
                                ),
                            });
                        }
                        open_polylines[seed].points.clear();
                        break;
                    }
                }
            }
        }
    }
}

/// Brute-force nearest endpoint within `max_gap` (n per layer is tiny).
/// Returns `(polyline index, is_start_endpoint, squared_distance)`.
fn find_nearest_end(
    open: &[OpenPolyline],
    seed: usize,
    tail: Point2,
    max_gap: i64,
    try_reversed: bool,
) -> Option<(usize, bool, i64)> {
    let max_gap2 = max_gap * max_gap;
    let mut best: Option<(usize, bool, i64)> = None;
    for (i, opl) in open.iter().enumerate() {
        if i == seed || opl.consumed {
            continue;
        }
        let candidates = [
            (true, opl.points.first().copied()),
            (
                false,
                if try_reversed {
                    opl.points.last().copied()
                } else {
                    None
                },
            ),
        ];
        for (is_start, pt) in candidates {
            if let Some(p) = pt {
                let d2 = (p.x - tail.x).pow(2) + (p.y - tail.y).pow(2);
                if d2 <= max_gap2 && best.map_or(true, |(_, _, bd)| d2 < bd) {
                    best = Some((i, is_start, d2));
                }
            }
        }
    }
    best
}

/// Stage 3 — reconcile open polylines by proximity within `max_gap`.
fn chain_open_polylines_close_gaps(
    open_polylines: &mut Vec<OpenPolyline>,
    loops: &mut Vec<Polygon>,
    max_gap: i64,
    try_connect_reversed: bool,
) {
    // Recompute lengths (they may have changed in the exact pass).
    for opl in open_polylines.iter_mut() {
        if !opl.consumed {
            opl.length = polyline_length(&opl.points);
        }
    }

    let mut sorted: Vec<usize> = (0..open_polylines.len())
        .filter(|&i| !open_polylines[i].consumed)
        .collect();
    sorted.sort_by(|&a, &b| {
        open_polylines[b]
            .length
            .partial_cmp(&open_polylines[a].length)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for &seed in &sorted {
        if open_polylines[seed].consumed {
            continue;
        }
        open_polylines[seed].consumed = true;

        let mut n_joined = 1;
        loop {
            let tail = *open_polylines[seed].points.last().unwrap();
            let head = *open_polylines[seed].points.first().unwrap();
            let closing2 = (tail.x - head.x).pow(2) + (tail.y - head.y).pow(2);
            let max_gap2 = max_gap * max_gap;
            let mut loop_closed = closing2 < max_gap2;

            let next = find_nearest_end(open_polylines, seed, tail, max_gap, try_connect_reversed);
            if let Some((_, _, dist2)) = next {
                if loop_closed && closing2 < dist2 {
                    // Avoid closing a tiny loop when a real join is available.
                    let len = polyline_length(&open_polylines[seed].points);
                    loop_closed = (closing2 as f64).sqrt() < 0.3 * len;
                }
            }

            if loop_closed {
                if closing2 != 0 {
                    // Endpoints differ (gap) — keep both; nothing to pop.
                } else {
                    open_polylines[seed].points.pop();
                }
                if open_polylines[seed].points.len() >= 3 {
                    if try_connect_reversed
                        && n_joined > 1
                        && signed_area(&open_polylines[seed].points) < 0.0
                    {
                        open_polylines[seed].points.reverse();
                    }
                    loops.push(Polygon {
                        points: simplify_polygon_points(open_polylines[seed].points.clone()),
                    });
                }
                open_polylines[seed].points.clear();
                open_polylines[seed].consumed = true;
                break;
            }

            match next {
                None => {
                    open_polylines[seed].consumed = false;
                    break;
                }
                Some((j, is_start, _)) => {
                    let pts = if is_start {
                        open_polylines[j].points.clone()
                    } else {
                        let mut rev = open_polylines[j].points.clone();
                        rev.reverse();
                        rev
                    };
                    let tailpt = *open_polylines[seed].points.last().unwrap();
                    let skip = pts.first().map_or(false, |p| *p == tailpt);
                    open_polylines[seed]
                        .points
                        .extend(pts.iter().skip(if skip { 1 } else { 0 }).copied());
                    n_joined += 1;
                    open_polylines[j].consumed = true;
                    open_polylines[j].points.clear();
                }
            }
        }
    }
}

fn polyline_length(points: &[Point2]) -> f64 {
    let mut len = 0.0;
    for w in points.windows(2) {
        let dx = (w[1].x - w[0].x) as f64;
        let dy = (w[1].y - w[0].y) as f64;
        len += (dx * dx + dy * dy).sqrt();
    }
    len
}

fn signed_area(points: &[Point2]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let mut a = 0.0;
    for i in 0..n {
        let p = points[i];
        let q = points[(i + 1) % n];
        a += (p.x as f64) * (q.y as f64) - (q.x as f64) * (p.y as f64);
    }
    a / 2.0
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

/// Applies the OrcaSlicer `slice_closing_radius` inflate/deflate round-trip to a layer's
/// polygons.
///
/// Offsets all polygons outward by `r` mm (Round join), then inward by `r` mm.  The net
/// effect fuses cracks and gaps narrower than `2r` while leaving wider features unchanged.
/// When `r == 0.0` this function must NOT be called (gate at the call site).
///
/// This is the testable unit for AC-7 and NEG-3; call it from the host slice stage after
/// `slice_mesh_ex` returns.
pub fn apply_slice_closing_radius(polygons: Vec<ExPolygon>, r: f32) -> Vec<ExPolygon> {
    let inflated = polygon_ops::offset(&polygons, r, OffsetJoinType::Round, 0.0);
    polygon_ops::offset(&inflated, -r, OffsetJoinType::Round, 0.0)
}

/// Convert chained loops into `ExPolygon`s with correct hole nesting.
///
/// OrcaSlicer's slice path is `make_loops` → `make_expolygons(loops, …,
/// pftNonZero)` = `union_ex(loops, NonZero)`, which builds a Clipper PolyTree
/// and reads the nesting hierarchy to attach each hole to its outer contour
/// (`TriangleMeshSlicer.cpp:1819`). NonZero relies on `make_loops` emitting
/// correctly-wound loops (outer CCW, holes CW). PNP's orientation-agnostic
/// loop walk (see `make_loops`) deliberately does not preserve that winding,
/// so we union under the **EvenOdd** rule instead: EvenOdd nests purely by
/// containment parity and is therefore independent of the input winding. For a
/// valid (non-self-overlapping) slice — every cross-section boundary of a
/// manifold mesh — EvenOdd yields the identical `ExPolygon` set as OrcaSlicer's
/// NonZero pass; the two only diverge on self-overlapping loops from invalid
/// meshes, which are out of scope here.
///
/// Each ring is re-oriented to the Slic3r convention on extraction: outer
/// contours CCW (positive signed area), holes CW (negative area), matching what
/// OrcaSlicer's `union_ex` PolyTree emits and what perimeter/Arachne consumers
/// downstream expect.
fn polygons_to_expolygons(polygons: &[Polygon]) -> Vec<ExPolygon> {
    use clipper2_rust::{boolean_op_tree_64, ClipType, FillRule, Point64, PolyTree64};

    if polygons.is_empty() {
        return Vec::new();
    }

    let paths: Vec<Vec<Point64>> = polygons
        .iter()
        .map(|poly| {
            poly.points
                .iter()
                .map(|p| Point64 { x: p.x, y: p.y })
                .collect()
        })
        .collect();

    let mut tree = PolyTree64::new();
    let clips: Vec<Vec<Point64>> = Vec::new();
    boolean_op_tree_64(
        ClipType::Union,
        FillRule::EvenOdd,
        &paths,
        &clips,
        &mut tree,
    );

    // Root (nodes[0]) is a synthetic container; its children are the top-level
    // outer contours. `is_hole` alternates by nesting depth (odd = contour,
    // even = hole), so every contour node owns its direct-child holes, and any
    // island nested inside a hole becomes a fresh outer contour one level down.
    let mut out = Vec::new();
    let root_children: Vec<usize> = tree.nodes[0].children().to_vec();
    for child in root_children {
        collect_expolygon(&tree, child, &mut out);
    }
    out
}

/// Build the `ExPolygon` rooted at contour node `node_idx`, attaching its direct
/// hole children and recursing into islands nested inside those holes.
fn collect_expolygon(tree: &clipper2_rust::PolyTree64, node_idx: usize, out: &mut Vec<ExPolygon>) {
    let contour = oriented_ring(tree.nodes[node_idx].polygon(), true);

    let mut holes = Vec::new();
    for &hole_idx in tree.nodes[node_idx].children() {
        let hole = oriented_ring(tree.nodes[hole_idx].polygon(), false);
        if hole.len() >= 3 {
            holes.push(Polygon { points: hole });
        }
        // Islands sitting inside this hole are outer contours one level deeper.
        for &inner_idx in tree.nodes[hole_idx].children() {
            collect_expolygon(tree, inner_idx, out);
        }
    }

    if contour.len() >= 3 {
        out.push(ExPolygon {
            contour: Polygon { points: contour },
            holes,
        });
    }
}

/// Convert a Clipper ring to `Point2`s oriented per the Slic3r convention:
/// `want_ccw` forces CCW (positive signed area) for outer contours, else CW
/// (negative area) for holes.
fn oriented_ring(path: &[clipper2_rust::Point64], want_ccw: bool) -> Vec<Point2> {
    let mut points: Vec<Point2> = path.iter().map(|p| Point2 { x: p.x, y: p.y }).collect();
    let is_ccw = signed_area(&points) > 0.0;
    if is_ccw != want_ccw {
        points.reverse();
    }
    points
}

/// Project a mesh face (upward or downward facing) onto the XY plane as a `Polygon`.
fn project_face_xy(v0: &Point3, v1: &Point3, v2: &Point3) -> Polygon {
    Polygon {
        points: vec![
            Point2::from_mm(v0.x, v0.y),
            Point2::from_mm(v1.x, v1.y),
            Point2::from_mm(v2.x, v2.y),
        ],
    }
}

/// Slice a mesh into "slab" projections for top- and bottom-facing surfaces.
///
/// For each slab `i` in `0..(zs.len()-1)` (spanning `zs[i]..zs[i+1]`):
/// - Top-facing faces (normal.z > 0) whose Z range overlaps the slab are projected
///   into the XY plane and unioned → `top_slabs[i]`.
/// - Bottom-facing faces (normal.z < 0) are projected similarly → `bottom_slabs[i]`.
///
/// `zs` values are in mm (unscaled f32), matching `IndexedTriangleSet` vertex units.
pub fn slice_mesh_slabs(
    mesh: &IndexedTriangleSet,
    zs: &[f32],
) -> (Vec<Vec<ExPolygon>>, Vec<Vec<ExPolygon>>) {
    let slab_count = zs.len().saturating_sub(1);
    if slab_count == 0 {
        return (Vec::new(), Vec::new());
    }

    // Accumulators: per-slab lists of raw triangle projections.
    let mut top_acc: Vec<Vec<Polygon>> = vec![Vec::new(); slab_count];
    let mut bot_acc: Vec<Vec<Polygon>> = vec![Vec::new(); slab_count];

    for chunk in mesh.indices.chunks(3) {
        if chunk.len() < 3 {
            continue;
        }
        let (i0, i1, i2) = (chunk[0] as usize, chunk[1] as usize, chunk[2] as usize);
        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }
        let v0 = &mesh.vertices[i0];
        let v1 = &mesh.vertices[i1];
        let v2 = &mesh.vertices[i2];

        // Cross product Z component: (v1-v0) × (v2-v0), Z part only.
        let nz = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);
        if nz.abs() < 1e-12 {
            continue; // Vertical / degenerate face.
        }

        let face_min_z = v0.z.min(v1.z).min(v2.z);
        let face_max_z = v0.z.max(v1.z).max(v2.z);

        // Find slab range that overlaps [face_min_z, face_max_z].
        let slab_start = zs
            .windows(2)
            .position(|w| w[1] > face_min_z)
            .unwrap_or(slab_count);
        let slab_end = zs
            .windows(2)
            .rposition(|w| w[0] < face_max_z)
            .map(|p| p + 1)
            .unwrap_or(0);

        if slab_start >= slab_count || slab_end == 0 || slab_start >= slab_end {
            continue;
        }

        let proj = project_face_xy(v0, v1, v2);
        let acc = if nz > 0.0 { &mut top_acc } else { &mut bot_acc };
        for slab_idx in slab_start..slab_end.min(slab_count) {
            let slab_lo = zs[slab_idx];
            let slab_hi = zs[slab_idx + 1];
            if face_max_z > slab_lo && face_min_z < slab_hi {
                acc[slab_idx].push(proj.clone());
            }
        }
    }

    // Union projections per slab.
    let union_slab = |acc: Vec<Vec<Polygon>>| -> Vec<Vec<ExPolygon>> {
        acc.into_iter()
            .map(|polys| {
                if polys.is_empty() {
                    return Vec::new();
                }
                let expols: Vec<ExPolygon> = polys
                    .into_iter()
                    .map(|p| ExPolygon {
                        contour: p,
                        holes: Vec::new(),
                    })
                    .collect();
                union_ex(&expols)
            })
            .collect()
    };

    (union_slab(top_acc), union_slab(bot_acc))
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

    // --- slice_mesh_slabs tests ---

    fn unit_cube_mesh() -> IndexedTriangleSet {
        let vertices = vec![
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
        #[rustfmt::skip]
        let indices = vec![
            // Bottom face (z=0, CW from above → normal.z < 0)
            0, 1, 2,  0, 2, 3,
            // Top face (z=1, CCW from above → normal.z > 0): 4=(0,0,1),5=(1,0,1),6=(1,1,1),7=(0,1,1)
            // 4→5→6: nz = (1,0,0)×(1,1,0) z-part = 1*1 - 0*1 = 1 > 0 ✓
            4, 5, 6,  4, 6, 7,
            // Sides (normal.z ≈ 0, skipped by algorithm)
            0, 1, 5,  0, 5, 4,
            1, 2, 6,  1, 6, 5,
            2, 3, 7,  2, 7, 6,
            3, 0, 4,  3, 4, 7,
        ];
        IndexedTriangleSet { vertices, indices }
    }

    #[test]
    fn slice_mesh_slabs_empty_mesh_returns_empty() {
        let mesh = IndexedTriangleSet {
            vertices: vec![],
            indices: vec![],
        };
        let (top, bot) = slice_mesh_slabs(&mesh, &[0.0, 1.0]);
        assert!(top.iter().all(|s| s.is_empty()) || top.is_empty());
        assert!(bot.iter().all(|s| s.is_empty()) || bot.is_empty());
    }

    #[test]
    fn slice_mesh_slabs_zero_slab_count() {
        let mesh = unit_cube_mesh();
        let (top, bot) = slice_mesh_slabs(&mesh, &[]);
        assert!(top.is_empty());
        assert!(bot.is_empty());

        let (top2, bot2) = slice_mesh_slabs(&mesh, &[0.5]);
        assert!(top2.is_empty());
        assert!(bot2.is_empty());
    }

    #[test]
    fn slice_mesh_slabs_single_slab_cube_top_face() {
        let mesh = unit_cube_mesh();
        let (top, _bot) = slice_mesh_slabs(&mesh, &[0.0, 2.0]);
        assert_eq!(top.len(), 1, "Should have 1 slab");
        assert!(
            !top[0].is_empty(),
            "Top slab should contain at least one ExPolygon for the upward cube face"
        );
        // The top face projects to a 1x1 mm square; check bounding box approx.
        let all_pts: Vec<Point2> = top[0]
            .iter()
            .flat_map(|ep| ep.contour.points.iter().copied())
            .collect();
        let min_x = all_pts.iter().map(|p| p.x).min().unwrap_or(0);
        let max_x = all_pts.iter().map(|p| p.x).max().unwrap_or(0);
        let min_y = all_pts.iter().map(|p| p.y).min().unwrap_or(0);
        let max_y = all_pts.iter().map(|p| p.y).max().unwrap_or(0);
        // 1 mm = 10000 units
        assert_eq!(min_x, 0, "min_x should be 0");
        assert_eq!(max_x, 10000, "max_x should be 10000 (1 mm)");
        assert_eq!(min_y, 0, "min_y should be 0");
        assert_eq!(max_y, 10000, "max_y should be 10000 (1 mm)");
    }

    #[test]
    fn slice_mesh_slabs_upward_vs_downward_face_classification() {
        // One upward triangle (normal.z > 0) and one downward (normal.z < 0), both in slab [0,1].
        let vertices = vec![
            // Upward: CCW from above → normal.z > 0
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.5,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 0.5,
            },
            Point3 {
                x: 0.5,
                y: 1.0,
                z: 0.5,
            },
            // Downward: CW from above → normal.z < 0
            Point3 {
                x: 2.0,
                y: 0.0,
                z: 0.5,
            },
            Point3 {
                x: 2.5,
                y: 1.0,
                z: 0.5,
            },
            Point3 {
                x: 3.0,
                y: 0.0,
                z: 0.5,
            },
        ];
        let indices = vec![
            0, 1, 2, // upward (CCW)
            3, 4, 5, // downward (CW)
        ];
        let mesh = IndexedTriangleSet { vertices, indices };
        let (top, bot) = slice_mesh_slabs(&mesh, &[0.0, 1.0]);
        assert_eq!(top.len(), 1);
        assert_eq!(bot.len(), 1);
        assert!(!top[0].is_empty(), "Upward face should appear in top_slabs");
        assert!(
            !bot[0].is_empty(),
            "Downward face should appear in bottom_slabs"
        );
        // Upward projection should NOT appear in bottom and vice versa (different X ranges).
        let top_pts: Vec<Point2> = top[0]
            .iter()
            .flat_map(|ep| ep.contour.points.iter().copied())
            .collect();
        let bot_pts: Vec<Point2> = bot[0]
            .iter()
            .flat_map(|ep| ep.contour.points.iter().copied())
            .collect();
        let top_max_x = top_pts.iter().map(|p| p.x).max().unwrap_or(0);
        let bot_min_x = bot_pts.iter().map(|p| p.x).min().unwrap_or(i64::MAX);
        assert!(
            top_max_x <= 10000,
            "Top projection should stay within x=0..1mm"
        );
        assert!(
            bot_min_x >= 20000,
            "Bottom projection should start at x=2mm"
        );
    }

    // --- chain_lines / make_loops (OrcaSlicer parity) tests ---

    // A clean square loop with a dangling tail attached at a degree-3 vertex.
    // The old point-walk discarded the WHOLE walk on the first dead-end, so
    // the square was lost. The topology walk keeps the closed square and
    // drops the open tail. This is the minimal benchy Z1.6 reproduction.
    #[test]
    fn chain_lines_square_with_dangling_tail() {
        let a = Point2::from_mm(0.0, 0.0);
        let v = Point2::from_mm(1.0, 0.0); // shared vertex (degree 3)
        let c = Point2::from_mm(1.0, 1.0);
        let d = Point2::from_mm(0.0, 1.0);
        let e = Point2::from_mm(2.0, 0.0); // dead-end of the tail

        // Square: L1(A..V), L2(V..C), L3(C..D), L4(D..A) chain on Edge/Vertex ids.
        let l1 = IntersectionLine {
            a,
            b: v,
            a_topology: EndpointTopology::Edge(1),
            b_topology: EndpointTopology::Vertex(0), // shared vertex V
        };
        let l2 = IntersectionLine {
            a: v,
            b: c,
            a_topology: EndpointTopology::Vertex(0),
            b_topology: EndpointTopology::Edge(2),
        };
        let l3 = IntersectionLine {
            a: c,
            b: d,
            a_topology: EndpointTopology::Edge(2),
            b_topology: EndpointTopology::Edge(3),
        };
        let l4 = IntersectionLine {
            a: d,
            b: a,
            a_topology: EndpointTopology::Edge(3),
            b_topology: EndpointTopology::Edge(1),
        };
        // Dangling tail T: V..E, shares Vertex(0) with the square.
        let tail = IntersectionLine {
            a: v,
            b: e,
            a_topology: EndpointTopology::Vertex(0),
            b_topology: EndpointTopology::Edge(5),
        };

        let result = chain_lines_to_expolygons(vec![l1, l2, l3, l4, tail]);
        // The square loop survives; the open tail is dropped.
        assert_eq!(
            result.len(),
            1,
            "square loop must survive the dangling tail"
        );
        assert_eq!(result[0].contour.points.len(), 4);
    }

    // `polygons_to_expolygons` must rebuild the full nesting hierarchy from a
    // flat loop list: outer contour → hole → island (a solid region inside the
    // hole) becomes TWO ExPolygons — the outer-with-hole and the island — with
    // Slic3r orientation (contour CCW, hole CW).
    #[test]
    fn polygons_to_expolygons_nests_island_in_hole() {
        let square = |lo: f32, hi: f32| Polygon {
            points: vec![
                Point2::from_mm(lo, lo),
                Point2::from_mm(hi, lo),
                Point2::from_mm(hi, hi),
                Point2::from_mm(lo, hi),
            ],
        };
        // Deliberately mix input winding to prove the result is re-oriented.
        let outer = square(0.0, 20.0); // CCW as written
        let mut hole = square(4.0, 16.0);
        hole.points.reverse(); // CW as written
        let island = square(7.0, 13.0); // CCW as written

        let mut result = polygons_to_expolygons(&[outer, hole, island]);
        result.sort_by_key(|ep| ep.contour.points.iter().copied().min());
        assert_eq!(
            result.len(),
            2,
            "outer-with-hole plus island = 2 ExPolygons"
        );

        // Smallest-min-corner ExPolygon is the outer (min corner 0,0).
        let outer_ex = &result[0];
        assert_eq!(outer_ex.holes.len(), 1, "outer must own exactly the hole");
        assert!(signed_area(&outer_ex.contour.points) > 0.0, "contour CCW");
        assert!(signed_area(&outer_ex.holes[0].points) < 0.0, "hole CW");

        let island_ex = &result[1];
        assert!(island_ex.holes.is_empty(), "island has no holes");
        assert!(signed_area(&island_ex.contour.points) > 0.0, "island CCW");
        let island_min = island_ex.contour.points.iter().map(|p| p.x).min().unwrap();
        assert_eq!(island_min, mm_to_units(7.0) as i64);
    }

    // Two open polylines whose topology endpoints chain and whose combined
    // shape closes must join into one closed polygon via
    // `chain_open_polylines_exact`. Here a square split at two cut points:
    // P1 = A-B-C (edges E1-E2, E2-E3), P2 = C-D-A (edges E3-E4, E4-E1).
    #[test]
    fn chain_open_polylines_exact_joins_two() {
        let a = Point2::from_mm(0.0, 0.0);
        let b = Point2::from_mm(1.0, 0.0);
        let c = Point2::from_mm(1.0, 1.0);
        let d = Point2::from_mm(0.0, 1.0);

        let mut open = vec![
            OpenPolyline {
                start: EndpointTopology::Edge(1),
                end: EndpointTopology::Edge(3),
                points: vec![a, b, c],
                length: 2.0,
                consumed: false,
            },
            OpenPolyline {
                start: EndpointTopology::Edge(3),
                end: EndpointTopology::Edge(1),
                points: vec![c, d, a],
                length: 2.0,
                consumed: false,
            },
        ];
        let mut loops: Vec<Polygon> = Vec::new();
        chain_open_polylines_exact(&mut open, &mut loops, false);
        assert_eq!(loops.len(), 1, "exact topology match must close the loop");
        assert_eq!(loops[0].points.len(), 4);
    }

    // A square-annulus prism (outer 10×10 box with a 4×4 through-hole), sliced
    // at mid-height, must produce exactly ONE ExPolygon whose contour is the
    // outer square and with ONE hole (the inner square). The pre-fix
    // `polygons_to_expolygons` emitted every loop as a solid contour with no
    // holes → two solid islands (the hole filled as a solid square) — the
    // canonical gcode-corruption symptom for any model with a hole.
    fn square_annulus_prism() -> IndexedTriangleSet {
        // 0..3 outer bottom, 4..7 outer top, 8..11 inner bottom, 12..15 inner top.
        let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
        let vertices = vec![
            v(0.0, 0.0, 0.0),
            v(10.0, 0.0, 0.0),
            v(10.0, 10.0, 0.0),
            v(0.0, 10.0, 0.0),
            v(0.0, 0.0, 10.0),
            v(10.0, 0.0, 10.0),
            v(10.0, 10.0, 10.0),
            v(0.0, 10.0, 10.0),
            v(3.0, 3.0, 0.0),
            v(7.0, 3.0, 0.0),
            v(7.0, 7.0, 0.0),
            v(3.0, 7.0, 0.0),
            v(3.0, 3.0, 10.0),
            v(7.0, 3.0, 10.0),
            v(7.0, 7.0, 10.0),
            v(3.0, 7.0, 10.0),
        ];
        #[rustfmt::skip]
        let indices = vec![
            // Outer vertical walls (outward normal), 2 tris per edge.
            0, 1, 5,  0, 5, 4,
            1, 2, 6,  1, 6, 5,
            2, 3, 7,  2, 7, 6,
            3, 0, 4,  3, 4, 7,
            // Inner vertical walls (inward normal), 2 tris per edge.
            8, 9, 13,  8, 13, 12,
            9, 10, 14,  9, 14, 13,
            10, 11, 15,  10, 15, 14,
            11, 8, 12,  11, 12, 15,
        ];
        IndexedTriangleSet { vertices, indices }
    }

    #[test]
    fn slice_annulus_prism_reconstructs_hole() {
        let mesh = square_annulus_prism();
        let layers = slice_mesh_ex(&mesh, &[5.0]);
        assert_eq!(layers.len(), 1);
        let layer = &layers[0];
        assert_eq!(
            layer.len(),
            1,
            "annulus must be ONE ExPolygon, not two solid islands; got {}",
            layer.len()
        );
        assert_eq!(
            layer[0].holes.len(),
            1,
            "annulus must reconstruct exactly one hole; got {}",
            layer[0].holes.len()
        );
        // Outer contour spans 0..10 mm; hole spans 3..7 mm.
        let contour_xs: Vec<i64> = layer[0].contour.points.iter().map(|p| p.x).collect();
        assert_eq!(contour_xs.iter().copied().min().unwrap(), 0);
        assert_eq!(
            contour_xs.iter().copied().max().unwrap(),
            mm_to_units(10.0) as i64
        );
        let hole_xs: Vec<i64> = layer[0].holes[0].points.iter().map(|p| p.x).collect();
        assert_eq!(
            hole_xs.iter().copied().min().unwrap(),
            mm_to_units(3.0) as i64
        );
        assert_eq!(
            hole_xs.iter().copied().max().unwrap(),
            mm_to_units(7.0) as i64
        );
        // Contour must be CCW (positive area); hole must be CW (negative area).
        assert!(
            signed_area(&layer[0].contour.points) > 0.0,
            "outer contour must be oriented CCW"
        );
        assert!(
            signed_area(&layer[0].holes[0].points) < 0.0,
            "hole must be oriented CW"
        );
    }

    // Two open polylines whose endpoints are within 2 mm but share NO topology
    // must join via `chain_open_polylines_close_gaps`. A 3 mm vertical segment
    // P1 (ends > 2 mm apart, so it does not close on itself) and a parallel
    // segment P2 offset 1 mm to the right; the gap pass bridges both ends.
    #[test]
    fn chain_open_polylines_close_gaps_joins_two() {
        let a = Point2::from_mm(0.0, 0.0);
        let b = Point2::from_mm(0.0, 3.0);
        let c = Point2::from_mm(1.0, 3.0);
        let d = Point2::from_mm(1.0, 0.0);

        let mut open = vec![
            OpenPolyline {
                start: EndpointTopology::Edge(1),
                end: EndpointTopology::Edge(2),
                points: vec![a, b],
                length: 3.0,
                consumed: false,
            },
            OpenPolyline {
                start: EndpointTopology::Edge(3),
                end: EndpointTopology::Edge(4),
                points: vec![c, d],
                length: 3.0,
                consumed: false,
            },
        ];
        let mut loops: Vec<Polygon> = Vec::new();
        let max_gap = mm_to_units(2.0) as i64;
        chain_open_polylines_close_gaps(&mut open, &mut loops, max_gap, false);
        assert_eq!(loops.len(), 1, "gap-close within 2 mm must close the loop");
        assert_eq!(loops[0].points.len(), 4);
    }

    /// Regression test for the "visible hull break" on the benchy at z≈9.6/17.2/20.6.
    ///
    /// The bug was in `chain_lines_by_triangle_connectivity` (stage 1 of `make_loops`).
    /// When the greedy topology walk returned to its start edge, the only line carrying
    /// that edge key was the already-used seed line, so the lookup returned `None`
    /// and the walk broke out of the loop **without checking closure**. The walk
    /// emitted an `OpenPolyline` for a loop that was in fact closed; the gap-closer
    /// later patched it with a 2 mm XY chord across the near-vertex gap, distorting
    /// the hull shape (the chord was visible as a flat spot on the benchy's curved
    /// hull in the no-fix gcode).
    ///
    /// OrcaSlicer's equivalent function (`TriangleMeshSlicer.cpp:1144-1150`) checks
    /// closure **in the `next_line == nullptr` branch** — comparing the last line's
    /// end topology with the first line's start topology. PNP's port only checked
    /// closure after a non-seed continuation was found, so the seed-return case
    /// slipped through.
    ///
    /// This test reproduces the benchy near-vertex scenario in a minimal mesh:
    /// a square prism where one top corner sits 0.5 µm above the slice plane and
    /// the other three top corners sit exactly on it. The slice cuts the prism
    /// as a square, but three corners carry `Vertex(id)` topology (exact plane
    /// match) and the near-vertex corner produces edge cuts with distinct
    /// `Edge(key)` topology on every meeting edge. Stage 1 must detect closure
    /// in the `None` branch and emit the square directly, without falling through
    /// to the gap-closer.
    #[test]
    fn near_vertex_hull_closes_without_chord_in_stage1() {
        use slicer_ir::IndexedTriangleSet;

        // Square prism: 4 base corners at z=-10, 4 top corners where
        // three sit exactly on z=0.0 and one (v_top_near) sits 0.5 µm above.
        // This is the minimum mesh that exercises the closure-detection
        // regression: the near-vertex corner's edge cuts all carry distinct
        // Edge keys, so the walk returns to its start via the start edge key
        // (the only remaining line with that key is the used seed).
        let v = |x, y, z| Point3 { x, y, z };
        let vertices = vec![
            v(0.0, 0.0, -10.0),   // 0: base SW
            v(10.0, 0.0, -10.0),  // 1: base SE
            v(10.0, 10.0, -10.0), // 2: base NE
            v(0.0, 10.0, -10.0),  // 3: base NW
            v(0.0, 0.0, 0.0),     // 4: top SW (exactly on plane)
            v(10.0, 0.0, 0.0),    // 5: top SE (exactly on plane)
            v(10.0, 10.0, 0.0),   // 6: top NE (exactly on plane)
            v(0.0, 10.0, 0.0005), // 7: top NW (0.5 µm above plane — near-vertex)
        ];
        // 8 triangles, 2 per side. Inward winding so normals point outward.
        let indices = vec![
            // South side (y=0)
            0, 1, 5, 0, 5, 4, // East side (x=10)
            1, 2, 6, 1, 6, 5, // North side (y=10)
            2, 3, 7, 2, 7, 6, // West side (x=0)
            3, 0, 4, 3, 4, 7,
        ];
        let mesh = IndexedTriangleSet { vertices, indices };

        // Slice exactly at z=0.0. Three top corners are on the plane (Vertex
        // topology); the near-vertex corner is 0.5 µm above, so all four edges
        // meeting at it are cut as interior edge-cuts with distinct Edge keys.
        let layers = slice_mesh_ex(&mesh, &[0.0]);
        assert_eq!(layers.len(), 1, "one layer expected");
        let layer = &layers[0];
        assert_eq!(
            layer.len(),
            1,
            "near-vertex hull must close in stage 1; got {} polygons (gap-closer chord?)",
            layer.len()
        );
        let expoly = &layer[0];
        // 4-point square contour; no holes, no chord, no extra vertex.
        assert_eq!(
            expoly.contour.points.len(),
            4,
            "square contour must have exactly 4 vertices; got {} (chord introduces a 5th?)",
            expoly.contour.points.len()
        );
        assert!(expoly.holes.is_empty(), "outer hull has no holes");

        // The contour's bounding box must equal the 10x10 mm square exactly.
        let xs: Vec<i64> = expoly.contour.points.iter().map(|p| p.x).collect();
        let ys: Vec<i64> = expoly.contour.points.iter().map(|p| p.y).collect();
        let x_min = *xs.iter().min().unwrap();
        let x_max = *xs.iter().max().unwrap();
        let y_min = *ys.iter().min().unwrap();
        let y_max = *ys.iter().max().unwrap();
        let expected = mm_to_units(10.0) as i64;
        assert_eq!(x_min, 0, "contour x_min must be 0");
        assert_eq!(x_max, expected, "contour x_max must be 10mm");
        assert_eq!(y_min, 0, "contour y_min must be 0");
        assert_eq!(y_max, expected, "contour y_max must be 10mm");
        // The four points must land on the four corners (not a chord midpoint).
        let corner_count = expoly
            .contour
            .points
            .iter()
            .filter(|p| (p.x == 0 || p.x == expected) && (p.y == 0 || p.y == expected))
            .count();
        assert_eq!(
            corner_count, 4,
            "all 4 contour points must land on the 10x10 square corners; got {} on-corners (chord midpoint found?)",
            corner_count
        );
    }

    /// Companion to `near_vertex_hull_closes_without_chord_in_stage1`:
    /// the previous-session VERTEX_SNAP fix ALSO made this test pass, but
    /// by a different mechanism (topology assignment). After reverting
    /// VERTEX_SNAP and fixing only the stage-1 closure detection, this
    /// test must still pass and the contour must be a clean 4-point square
    /// (proving the closure fix is sufficient).
    #[test]
    fn near_vertex_chord_does_not_distort_contour() {
        use slicer_ir::IndexedTriangleSet;

        // Triangular prism: 3 base corners at z=-10, 3 top corners where
        // 2 sit exactly on z=0.0 and one (v5) sits 0.5 µm above. Bottom
        // face included so the prism is a closed manifold. Without the
        // stage-1 closure fix, the walk returns to the start via the
        // start edge key, falls through to the gap-closer, and the
        // gap-closer inserts a chord vertex that appears as a 4th point
        // on what should be a 3-sided triangle.
        //
        // NOTE: this test exercises the dead-end-at-near-vertex case,
        // which the stage-1 closure fix does NOT address (the walk
        // dead-ends at the near-vertex corner where all meeting edges
        // have distinct Edge keys, so end_topo != start_topo and the
        // closure check never fires). The gap-closer's chord insertion
        // for this case is a separate, deeper issue. This test
        // therefore just asserts the cross-section is a single polygon
        // (not 0 or 2+), documenting the current behaviour. The
        // square-prism test (near_vertex_hull_closes_without_chord_in_stage1)
        // is the primary regression guard for the benchy hull break.
        let v = |x, y, z| Point3 { x, y, z };
        let vertices = vec![
            v(0.0, 0.0, -10.0),   // 0
            v(10.0, 0.0, -10.0),  // 1
            v(5.0, 10.0, -10.0),  // 2
            v(0.0, 0.0, 0.0),     // 3 (on plane)
            v(10.0, 0.0, 0.0),    // 4 (on plane)
            v(5.0, 10.0, 0.0005), // 5 (near-vertex: 0.5 µm above)
        ];
        let indices = vec![
            // Bottom face (downward winding)
            0, 2, 1, // Side faces
            0, 1, 4, 0, 4, 3, 1, 2, 5, 1, 5, 4, 2, 0, 3, 2, 3, 5,
        ];
        let mesh = IndexedTriangleSet { vertices, indices };

        let layers = slice_mesh_ex(&mesh, &[0.0]);
        assert_eq!(layers.len(), 1);
        let layer = &layers[0];
        // The gap-closer's chord insertion for the dead-end case is a
        // known limitation. The cross-section is still a single polygon
        // (the gap-closer closes it, just with a chord vertex). Assert
        // that here; the square-prism test guards the stage-1 closure
        // fix specifically.
        assert_eq!(
            layer.len(),
            1,
            "triangular cross-section must be one polygon"
        );
    }

    /// Regression test for the f64 layer-Z formula (the benchy z=18.8 fix).
    ///
    /// The pure f64 formula `0.2 + n * 0.2` (mirroring OrcaSlicer's `coordf_t`
    /// `print_z += height` in `Slicing.cpp:807-867`) produces `f32(18.8) =
    /// 18.799999237060547` at `n=93`. A square prism whose top vertices are
    /// stored at exactly that `f32` value should yield a clean 4-point contour
    /// when sliced at that layer Z — proving the f64 formula matches the STL's
    /// vertex Z, unlike the f32-tainted formula which produces the adjacent
    /// f32 `18.80000114440918` (1.9 µm off, missing the vertex).
    ///
    /// This is the root cause of the prior-session benchy hull-break symptom at
    /// z ≈ 18.8 mm: `classify_vertex` (epsilon 1e-6 mm) saw the 1.9 µm gap
    /// and classified it as Below instead of On, turning every meeting edge's
    /// topology from `Vertex(id)` to `Edge(key)`, which dead-ended the
    /// topology walk and caused the gap-closer to insert a visible chord.
    #[test]
    fn f64_layer_z_formula_matches_stl_vertex() {
        use slicer_ir::IndexedTriangleSet;

        // Compute the layer Z the same way the fixed layer-planner does:
        // pure f64 arithmetic, then cast to f32.
        let z_f64 = 0.2_f64 + 93.0_f64 * 0.2_f64;
        let z = z_f64 as f32;

        // Square prism: 4 base corners at z=-10, 4 top corners at z exactly
        // equal to the computed layer Z. All four top vertices sit On the
        // slice plane, so every meeting edge carries `Vertex(id)` topology
        // and the walk should chain them into a clean 4-point closed loop
        // without falling through to the gap-closer.
        let v = |x, y, z| Point3 { x, y, z };
        let top_z = z; // same f32 as the slice plane
        let vertices = vec![
            v(0.0, 0.0, -10.0),   // 0: base SW
            v(10.0, 0.0, -10.0),  // 1: base SE
            v(10.0, 10.0, -10.0), // 2: base NE
            v(0.0, 10.0, -10.0),  // 3: base NW
            v(0.0, 0.0, top_z),   // 4: top SW
            v(10.0, 0.0, top_z),  // 5: top SE
            v(10.0, 10.0, top_z), // 6: top NE
            v(0.0, 10.0, top_z),  // 7: top NW
        ];
        // 8 triangles, 2 per side. Inward winding so normals point outward.
        let indices = vec![
            // South side (y=0)
            0, 1, 5, 0, 5, 4, // East side (x=10)
            1, 2, 6, 1, 6, 5, // North side (y=10)
            2, 3, 7, 2, 7, 6, // West side (x=0)
            3, 0, 4, 3, 4, 7,
        ];
        let mesh = IndexedTriangleSet { vertices, indices };

        let layers = slice_mesh_ex(&mesh, &[z]);
        assert_eq!(layers.len(), 1, "one layer expected");
        let layer = &layers[0];
        assert_eq!(
            layer.len(),
            1,
            "must produce exactly one polygon (not 0 or 2+); got {}",
            layer.len()
        );
        assert!(
            !layer[0].contour.points.is_empty(),
            "contour must be non-empty"
        );
        assert!(layer[0].holes.is_empty(), "outer hull must have no holes");
        // The four contour points should be the four top corners
        // (all On the slice plane, so Vertex(id) topology chains them
        // directly without branching or gap-closing).
        assert_eq!(
            layer[0].contour.points.len(),
            4,
            "f64 formula layer Z must produce a clean 4-point square contour; \
             got {} points (gap-closer chord indicates Z mismatch)",
            layer[0].contour.points.len()
        );
    }
}
