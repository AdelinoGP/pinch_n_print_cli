//! Shared helpers for the native scripted SCAD property targets.
//!
//! The adapter capacities below are **derived** from the recipe emitter bound,
//! not chosen as a permissive fallback: eight AST nodes per recipe node plus a
//! fixed headroom of sixteen, two AST levels per recipe level plus eight, and
//! one geometry node per recipe node. `scripted_cases` and `evaluate_recipe`
//! reject any option value below that derivation, so a registry profile that
//! used a smaller number fails loudly instead of silently truncating a recipe.
//!
//! The geometry helpers here are the **test-side** independent observations.
//! They deliberately do not call the engine, the emitter, or the strategy's own
//! reference module: a generated mesh is stimulus, and only an independently
//! derived check can say what it should look like.

// This module is shared by two test targets; each uses a subset of the helpers.
#![allow(dead_code)]

use serde_json::{json, Value};
use slicer_ir::{ExPolygon, IndexedTriangleSet, Point2, Polygon};
use slicer_scad::{ScadEvalOptions, ScadLimits};
use slicer_sdk::test_support::property_testing::scripted::RecipeBounds;

const UNITS_PER_MM: f64 = 10_000.0;

/// Conservative AST node capacity for `bounds`.
///
/// A polygon leaf contributes one AST node per vertex, so the structural node
/// count alone is not a bound. Every AST node needs at least one source byte,
/// which makes `max_source_bytes` a safe upper bound on AST nodes; the
/// structural estimate is the floor for a small source bound.
pub fn derived_ast_nodes(bounds: &RecipeBounds) -> usize {
    (8 * bounds.max_nodes + 16).max(bounds.max_source_bytes)
}

/// Conservative AST depth capacity for `bounds`.
pub fn derived_ast_depth(bounds: &RecipeBounds) -> usize {
    2 * bounds.max_depth + 8
}

/// Conservative evaluated CSG node capacity for `bounds`.
///
/// A polygon leaf expands to one evaluated node per ring plus its Boolean
/// wrappers, so the structural recipe count is not a bound: every evaluated
/// node comes from a statement, and every statement occupies at least one
/// source byte.
pub fn derived_geometry_nodes(bounds: &RecipeBounds) -> usize {
    derived_ast_nodes(bounds)
}

/// Build adapter options whose capacities are derived from `bounds`.
///
/// `max_abs_coordinate_mm` is the domain's canonical-unit envelope converted to
/// millimetres; heights and translations stay inside the same envelope.
pub fn options(
    bounds: &RecipeBounds,
    max_abs_units: i64,
    mesh_vertices: usize,
    mesh_triangles: usize,
    evaluation_steps: u64,
    seed: u64,
) -> ScadEvalOptions {
    options_with_stack(
        bounds,
        max_abs_units,
        mesh_vertices,
        mesh_triangles,
        evaluation_steps,
        ScadLimits::default().worker_stack_bytes,
        seed,
    )
}

/// Build adapter options with an explicit worker stack (registry profiles).
pub fn options_with_stack(
    bounds: &RecipeBounds,
    max_abs_units: i64,
    mesh_vertices: usize,
    mesh_triangles: usize,
    evaluation_steps: u64,
    worker_stack_bytes: usize,
    seed: u64,
) -> ScadEvalOptions {
    ScadEvalOptions {
        parameters: Vec::new(),
        extrude_height_mm: None,
        // exhaustive: pins every derived adapter capacity from the recipe bound
        limits: ScadLimits {
            evaluation_steps,
            max_source_bytes: bounds.max_source_bytes,
            max_ast_nodes: derived_ast_nodes(bounds),
            max_ast_depth: derived_ast_depth(bounds),
            max_geometry_nodes: derived_geometry_nodes(bounds),
            max_facets: u32::from(bounds.max_facets),
            max_vertices: mesh_vertices,
            max_triangles: mesh_triangles,
            // One canonical unit is 1e-4 mm; keep the envelope in millimetres.
            max_abs_coordinate_mm: (max_abs_units as f64) / UNITS_PER_MM,
            worker_stack_bytes,
        },
        random_seed: Some(seed),
    }
}

/// Exact signed double area of one ring in squared canonical units.
pub fn signed_area2(ring: &Polygon) -> i128 {
    let mut total: i128 = 0;
    let count = ring.points.len();
    for index in 0..count {
        let a = ring.points[index];
        let b = ring.points[(index + 1) % count];
        total += i128::from(a.x) * i128::from(b.y) - i128::from(a.y) * i128::from(b.x);
    }
    total
}

/// Signed area of a polygon forest in square millimetres.
pub fn forest_area_mm2(polygons: &[ExPolygon]) -> f64 {
    let mut total: i128 = 0;
    for polygon in polygons {
        for ring in std::iter::once(&polygon.contour).chain(polygon.holes.iter()) {
            total += signed_area2(ring);
        }
    }
    total as f64 / (2.0 * UNITS_PER_MM * UNITS_PER_MM)
}

/// Total hole count across a forest.
pub fn hole_count(polygons: &[ExPolygon]) -> usize {
    polygons.iter().map(|polygon| polygon.holes.len()).sum()
}

/// Analytical area of one faceted regular polygon.
pub fn regular_polygon_area_mm2(radius_units: f64, facets: u16) -> f64 {
    let radius = radius_units / UNITS_PER_MM;
    let angle = std::f64::consts::TAU / f64::from(facets);
    0.5 * f64::from(facets) * radius * radius * angle.sin()
}

/// Independent point-in-mesh test by ray casting, using only converted vertices.
///
/// The ray direction is deliberately skew so it does not graze an axis-aligned
/// box edge, which is the only mesh family these witnesses use.
pub fn point_inside_mesh(mesh: &IndexedTriangleSet, point: [f64; 3]) -> bool {
    let direction = [1.0, 0.373_1, 0.114_9];
    let mut crossings = 0_usize;
    for triangle in mesh.indices.chunks_exact(3) {
        let vertices = [
            mesh.vertices[triangle[0] as usize],
            mesh.vertices[triangle[1] as usize],
            mesh.vertices[triangle[2] as usize],
        ];
        let a = [f64::from(vertices[0].x), f64::from(vertices[0].y), f64::from(vertices[0].z)];
        let b = [f64::from(vertices[1].x), f64::from(vertices[1].y), f64::from(vertices[1].z)];
        let c = [f64::from(vertices[2].x), f64::from(vertices[2].y), f64::from(vertices[2].z)];
        let edge1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let edge2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let pvec = cross(direction, edge2);
        let determinant = dot(edge1, pvec);
        if determinant.abs() < 1e-12 {
            continue;
        }
        let inverse = 1.0 / determinant;
        let tvec = [point[0] - a[0], point[1] - a[1], point[2] - a[2]];
        let u = dot(tvec, pvec) * inverse;
        if !(-1e-9..=1.0 + 1e-9).contains(&u) {
            continue;
        }
        let qvec = cross(tvec, edge1);
        let v = dot(direction, qvec) * inverse;
        if v < -1e-9 || u + v > 1.0 + 1e-9 {
            continue;
        }
        let t = dot(edge2, qvec) * inverse;
        if t > 1e-9 {
            crossings += 1;
        }
    }
    crossings % 2 == 1
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Independent axis-aligned bounds of a converted mesh, in millimetres.
pub fn mesh_bounds(mesh: &IndexedTriangleSet) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for (axis, value) in [vertex.x, vertex.y, vertex.z].into_iter().enumerate() {
            min[axis] = min[axis].min(f64::from(value));
            max[axis] = max[axis].max(f64::from(value));
        }
    }
    (min, max)
}

/// An independent point probe in canonical units.
pub fn units_point(point: [i64; 3]) -> [f64; 3] {
    [
        point[0] as f64 / UNITS_PER_MM,
        point[1] as f64 / UNITS_PER_MM,
        point[2] as f64 / UNITS_PER_MM,
    ]
}

/// Build a registry-shaped property context for a direct `PropertyRun` test.
///
/// This mirrors the exact schema `cargo xtask test --properties` writes, so the
/// foundation's real validation and completion path is what the test drives.
pub fn context(
    artifact_dir: &std::path::Path,
    property_id: &str,
    test_name: &str,
    cases: u32,
    seed: u64,
) -> Value {
    json!({
        "schema_version": 1,
        "run_token": "scripted-contract-run",
        "property_id": property_id,
        "tier": "routine",
        "seed": seed,
        "package": "slicer-sdk",
        "target_kind": "test",
        "target_name": "property_scripted_contracts",
        "test_name": test_name,
        "features": ["slicer-sdk/property_testing_scad"],
        "profile": {
            "cases": cases,
            "max_local_rejects": 16,
            "max_global_rejects": 64,
            "max_flat_map_regens": 0,
            "max_default_size_range": 32,
            "max_shrink_iters": 64,
            "max_shrink_time_ms": 0,
            "rng_algorithm": "chacha",
            "isolation": "in_process",
            "timeout_ms": 0,
            "bounds": { "unused": 1 }
        },
        "artifact_dir": artifact_dir.to_string_lossy(),
        "proptest_version": "1.11.0",
        "generator_version": 1,
        "source_revision": "scripted-contract-revision",
        "dirty_paths": [],
        "source_state_digest": "scripted-contract-digest",
        "cargo_target_dir": Value::Null,
        "completion_eligible": true
    })
}

/// Reflex-turn witness: any ring corner whose turn is clockwise for a CCW ring.
pub fn has_reflex_turn(ring: &Polygon) -> bool {
    let count = ring.points.len();
    if count < 3 {
        return false;
    }
    for index in 0..count {
        let a = ring.points[(index + count - 1) % count];
        let b = ring.points[index];
        let c = ring.points[(index + 1) % count];
        let cross = i128::from(b.x - a.x) * i128::from(c.y - b.y)
            - i128::from(b.y - a.y) * i128::from(c.x - b.x);
        if cross < 0 {
            return true;
        }
    }
    false
}

/// A proven-empty visibility kernel for the opposing-wall family.
///
/// For a CCW ring, the outward normal of edge `a -> b` is `(dy, -dx)`. Two walls
/// facing each other (`+x` at `x = left`, `-x` at `x = right > left`) whose
/// vertical spans overlap impose `x >= left` and `x <= right` on every kernel
/// point, so no point can see the whole ring. This is a proof for that family,
/// not a sampling argument.
pub fn empty_kernel_wall_pair(ring: &Polygon) -> bool {
    let count = ring.points.len();
    let mut left_walls: Vec<(i128, i128, i128)> = Vec::new();
    let mut right_walls: Vec<(i128, i128, i128)> = Vec::new();
    for index in 0..count {
        let a = ring.points[index];
        let b = ring.points[(index + 1) % count];
        let dx = i128::from(b.x - a.x);
        let dy = i128::from(b.y - a.y);
        if dx == 0 {
            // Vertical wall: outward normal is (+dy, 0) or (-dy, 0).
            let x = i128::from(a.x);
            let low = i128::from(a.y.min(b.y));
            let high = i128::from(a.y.max(b.y));
            if dy > 0 {
                left_walls.push((x, low, high));
            } else {
                right_walls.push((x, low, high));
            }
        }
    }
    left_walls.iter().any(|left| {
        right_walls.iter().any(|right| {
            right.0 > left.0 && left.1 < right.2 && right.1 < left.2
        })
    })
}

/// Exact Euclidean distance between two closed segments, in canonical units.
pub fn segment_distance(a: [Point2; 2], b: [Point2; 2]) -> f64 {
    fn point_segment(p: Point2, a: Point2, b: Point2) -> f64 {
        let (px, py) = (p.x as f64, p.y as f64);
        let (ax, ay) = (a.x as f64, a.y as f64);
        let (bx, by) = (b.x as f64, b.y as f64);
        let (dx, dy) = (bx - ax, by - ay);
        let length_sq = dx * dx + dy * dy;
        let t = if length_sq == 0.0 {
            0.0
        } else {
            (((px - ax) * dx + (py - ay) * dy) / length_sq).clamp(0.0, 1.0)
        };
        let (cx, cy) = (ax + t * dx, ay + t * dy);
        ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
    }
    if segments_intersect(a, b) {
        return 0.0;
    }
    point_segment(a[0], b[0], b[1])
        .min(point_segment(a[1], b[0], b[1]))
        .min(point_segment(b[0], a[0], a[1]))
        .min(point_segment(b[1], a[0], a[1]))
}

fn segments_intersect(a: [Point2; 2], b: [Point2; 2]) -> bool {
    fn orient(p: Point2, q: Point2, r: Point2) -> i128 {
        i128::from(q.x - p.x) * i128::from(r.y - p.y) - i128::from(q.y - p.y) * i128::from(r.x - p.x)
    }
    let d1 = orient(a[0], a[1], b[0]);
    let d2 = orient(a[0], a[1], b[1]);
    let d3 = orient(b[0], b[1], a[0]);
    let d4 = orient(b[0], b[1], a[1]);
    d1 * d2 < 0 && d3 * d4 < 0
}

/// Minimum distance between non-adjacent edges of one ring, in canonical units.
pub fn nonadjacent_edge_clearance(ring: &Polygon) -> f64 {
    let count = ring.points.len();
    let edges: Vec<[Point2; 2]> = (0..count)
        .map(|index| [ring.points[index], ring.points[(index + 1) % count]])
        .collect();
    let mut best = f64::INFINITY;
    for i in 0..edges.len() {
        for j in (i + 1)..edges.len() {
            if j == i + 1 || (i == 0 && j == edges.len() - 1) {
                continue;
            }
            best = best.min(segment_distance(edges[i], edges[j]));
        }
    }
    best
}

/// A vertex that is within two canonical units of the line through its
/// neighbours, but not exactly on it: the almost-collinear family.
pub fn has_almost_collinear_vertex(ring: &Polygon) -> bool {
    let count = ring.points.len();
    if count < 3 {
        return false;
    }
    for index in 0..count {
        let a = ring.points[(index + count - 1) % count];
        let b = ring.points[index];
        let c = ring.points[(index + 1) % count];
        let cross = i128::from(b.x - a.x) * i128::from(c.y - a.y)
            - i128::from(b.y - a.y) * i128::from(c.x - a.x);
        if cross == 0 {
            continue;
        }
        let base = ((i128::from(c.x - a.x)).pow(2) + (i128::from(c.y - a.y)).pow(2)) as f64;
        let base = base.sqrt();
        if base == 0.0 {
            continue;
        }
        let distance = (cross.unsigned_abs() as f64) / base;
        if distance <= 2.0 {
            return true;
        }
    }
    false
}

/// Ring bounding box in canonical units.
pub fn ring_bbox(ring: &Polygon) -> (i64, i64, i64, i64) {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for point in &ring.points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    (min_x, min_y, max_x, max_y)
}

/// Reject emitted programs that would not be self-contained.
pub fn assert_self_contained(text: &str) {
    for forbidden in [
        "include", "import", "use<", "surface(", "text(", "rands(", "for (", "function ",
    ] {
        assert!(
            !text.contains(forbidden),
            "emitted source contains the forbidden construct {forbidden:?}:\n{text}"
        );
    }
}

/// Whether a converted mesh contains an interior point with no material one
/// unit below it: an independent measurement of an unsupported span.
///
/// Sampling is aligned to the converted mesh's own vertex coordinates (and their
/// midpoints), so a thin plate between two coarse grid lines cannot be missed.
pub fn has_unsupported_span(mesh: &IndexedTriangleSet) -> bool {
    let (min, max) = mesh_bounds(mesh);
    let axis_samples = |axis: usize| {
        let mut values: Vec<f64> = mesh
            .vertices
            .iter()
            .map(|vertex| f64::from([vertex.x, vertex.y, vertex.z][axis]))
            .collect();
        values.sort_by(|left, right| left.partial_cmp(right).expect("finite vertices"));
        values.dedup_by(|left, right| (*left - *right).abs() < 1e-9);
        let mut samples = Vec::new();
        for pair in values.windows(2) {
            samples.push(pair[0] + (pair[1] - pair[0]) * 0.5);
        }
        samples.extend(values.iter().map(|value| value + 1e-6));
        samples.extend(values.iter().map(|value| value - 1e-6));
        samples.retain(|value| *value > min[axis] && *value < max[axis]);
        samples
    };
    let xs = axis_samples(0);
    let ys = axis_samples(1);
    let zs = axis_samples(2);
    let below = 1e-4_f64;
    for x in &xs {
        for y in &ys {
            for z in &zs {
                if *z - below <= min[2] + 1e-9 {
                    continue;
                }
                if point_inside_mesh(mesh, [*x, *y, *z])
                    && !point_inside_mesh(mesh, [*x, *y, *z - below])
                {
                    return true;
                }
            }
        }
    }
    false
}
