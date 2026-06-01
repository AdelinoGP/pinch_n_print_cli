//! Assertion helpers for extrusion path fixtures.

use slicer_ir::{ExtrusionPath3D, Polygon};

/// Assert that every point z-coordinate is within `tolerance` of `expected_z_mm`.
///
/// # Panics
/// Panics if any point is outside the tolerance.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
/// use slicer_sdk::test_support::assert_paths::assert_paths_planar;
///
/// let paths = vec![ExtrusionPath3D {
///     points: Vec::new(),
///     role: ExtrusionRole::SparseInfill,
///     speed_factor: 1.0,
/// }];
/// assert_paths_planar(&paths, 0.2, 1e-3);
/// ```
pub fn assert_paths_planar(paths: &[ExtrusionPath3D], expected_z_mm: f32, tolerance: f32) {
    for (path_index, path) in paths.iter().enumerate() {
        for (point_index, point) in path.points.iter().enumerate() {
            let delta = (point.z - expected_z_mm).abs();
            assert!(
                delta <= tolerance,
                "path {path_index} point {point_index} has z={} expected {} +/- {}",
                point.z,
                expected_z_mm,
                tolerance
            );
        }
    }
}

/// Assert that no segment between consecutive points in any path exceeds `max_len_mm`.
///
/// Uses 3D Euclidean distance on (x, y, z) fields of [`Point3WithWidth`].
///
/// # Panics
/// Panics if any segment is longer than `max_len_mm`.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::{ExtrusionPath3D, ExtrusionRole, Point3WithWidth};
/// use slicer_sdk::test_support::assert_paths::assert_max_segment_length;
///
/// let path = ExtrusionPath3D {
///     points: vec![
///         Point3WithWidth { x: 0.0, y: 0.0, z: 0.0, width: 0.4, flow_factor: 1.0, overhang_quartile: None },
///         Point3WithWidth { x: 1.0, y: 0.0, z: 0.0, width: 0.4, flow_factor: 1.0, overhang_quartile: None },
///     ],
///     role: ExtrusionRole::SparseInfill,
///     speed_factor: 1.0,
/// };
/// assert_max_segment_length(&[path], 2.0);
/// ```
pub fn assert_max_segment_length(paths: &[ExtrusionPath3D], max_len_mm: f32) {
    for (path_index, path) in paths.iter().enumerate() {
        for i in 0..path.points.len().saturating_sub(1) {
            let a = &path.points[i];
            let b = &path.points[i + 1];
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let dz = b.z - a.z;
            let len = (dx * dx + dy * dy + dz * dz).sqrt();
            assert!(
                len <= max_len_mm,
                "path {path_index} segment {i}->{} has length {len} exceeding max {max_len_mm}",
                i + 1
            );
        }
    }
}

/// Assert that every point's extrusion width falls within `[min_width, max_width]`.
///
/// # Panics
/// Panics if any point's `width` is outside the range.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::{ExtrusionPath3D, ExtrusionRole, Point3WithWidth};
/// use slicer_sdk::test_support::assert_paths::assert_extrusion_width_range;
///
/// let path = ExtrusionPath3D {
///     points: vec![
///         Point3WithWidth { x: 0.0, y: 0.0, z: 0.0, width: 0.4, flow_factor: 1.0, overhang_quartile: None },
///     ],
///     role: ExtrusionRole::SparseInfill,
///     speed_factor: 1.0,
/// };
/// assert_extrusion_width_range(&[path], 0.3, 0.5);
/// ```
pub fn assert_extrusion_width_range(paths: &[ExtrusionPath3D], min_width: f32, max_width: f32) {
    for (path_index, path) in paths.iter().enumerate() {
        for (point_index, point) in path.points.iter().enumerate() {
            assert!(
                point.width >= min_width && point.width <= max_width,
                "path {path_index} point {point_index} has width {} outside range [{min_width}, {max_width}]",
                point.width
            );
        }
    }
}

/// Assert that every point in all paths lies inside the given polygon.
///
/// Points are converted from f32 mm coordinates to scaled i64 units for
/// comparison with the polygon's [`Point2`] vertices. Points on the boundary
/// are considered inside (inclusive).
///
/// Uses the ray-casting (even-odd) algorithm with boundary tolerance.
///
/// # Panics
/// Panics if any point is outside the polygon.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::{ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth, Polygon};
/// use slicer_sdk::test_support::assert_paths::assert_paths_inside_polygon;
///
/// let polygon = Polygon {
///     points: vec![
///         Point2::from_mm(0.0, 0.0),
///         Point2::from_mm(10.0, 0.0),
///         Point2::from_mm(10.0, 10.0),
///         Point2::from_mm(0.0, 10.0),
///     ],
/// };
/// let path = ExtrusionPath3D {
///     points: vec![
///         Point3WithWidth { x: 5.0, y: 5.0, z: 0.0, width: 0.4, flow_factor: 1.0, overhang_quartile: None },
///     ],
///     role: ExtrusionRole::SparseInfill,
///     speed_factor: 1.0,
/// };
/// assert_paths_inside_polygon(&[path], &polygon);
/// ```
pub fn assert_paths_inside_polygon(paths: &[ExtrusionPath3D], polygon: &Polygon) {
    use slicer_ir::mm_to_units;

    for (path_index, path) in paths.iter().enumerate() {
        for (point_index, point) in path.points.iter().enumerate() {
            let px = mm_to_units(point.x);
            let py = mm_to_units(point.y);
            assert!(
                point_in_polygon_inclusive(px, py, &polygon.points),
                "path {path_index} point {point_index} ({}, {}) is outside the polygon boundary",
                point.x,
                point.y
            );
        }
    }
}

/// Winding-number point-in-polygon test with boundary inclusion.
///
/// Returns `true` if the point `(px, py)` is inside or on the boundary of
/// the polygon defined by `vertices`.
fn point_in_polygon_inclusive(px: i64, py: i64, vertices: &[slicer_ir::Point2]) -> bool {
    let n = vertices.len();
    if n < 3 {
        return false;
    }

    // First check if point is on any edge (boundary inclusion)
    for i in 0..n {
        let j = (i + 1) % n;
        if point_on_segment(px, py, &vertices[i], &vertices[j]) {
            return true;
        }
    }

    // Winding number algorithm
    let mut winding = 0i32;
    for i in 0..n {
        let j = (i + 1) % n;
        let yi = vertices[i].y;
        let yj = vertices[j].y;

        if yi <= py {
            if yj > py {
                // Upward crossing
                if cross_sign(&vertices[i], &vertices[j], px, py) > 0 {
                    winding += 1;
                }
            }
        } else if yj <= py {
            // Downward crossing
            if cross_sign(&vertices[i], &vertices[j], px, py) < 0 {
                winding -= 1;
            }
        }
    }

    winding != 0
}

/// Cross product sign of (edge × point) for winding number.
/// Positive means point is to the left of the edge from `a` to `b`.
fn cross_sign(a: &slicer_ir::Point2, b: &slicer_ir::Point2, px: i64, py: i64) -> i64 {
    // Use i128 to avoid overflow with i64 coordinates
    let cross = (b.x as i128 - a.x as i128) * (py as i128 - a.y as i128)
        - (px as i128 - a.x as i128) * (b.y as i128 - a.y as i128);
    if cross > 0 {
        1
    } else if cross < 0 {
        -1
    } else {
        0
    }
}

/// Check if point (px, py) lies on segment from a to b.
fn point_on_segment(px: i64, py: i64, a: &slicer_ir::Point2, b: &slicer_ir::Point2) -> bool {
    // Cross product must be zero (collinear)
    let cross = (b.x as i128 - a.x as i128) * (py as i128 - a.y as i128)
        - (px as i128 - a.x as i128) * (b.y as i128 - a.y as i128);
    if cross != 0 {
        return false;
    }
    // Check bounding box
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    px >= min_x && px <= max_x && py >= min_y && py <= max_y
}

/// Assert that no two path segments from different paths intersect.
///
/// Tests all segment pairs across different paths for intersection using
/// 2D projection (x, y coordinates only).
///
/// # Panics
/// Panics if any two segments from different paths intersect.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::{ExtrusionPath3D, ExtrusionRole, Point3WithWidth};
/// use slicer_sdk::test_support::assert_paths::assert_no_path_intersections;
///
/// let path_a = ExtrusionPath3D {
///     points: vec![
///         Point3WithWidth { x: 0.0, y: 0.0, z: 0.0, width: 0.4, flow_factor: 1.0, overhang_quartile: None },
///         Point3WithWidth { x: 10.0, y: 0.0, z: 0.0, width: 0.4, flow_factor: 1.0, overhang_quartile: None },
///     ],
///     role: ExtrusionRole::SparseInfill,
///     speed_factor: 1.0,
/// };
/// let path_b = ExtrusionPath3D {
///     points: vec![
///         Point3WithWidth { x: 0.0, y: 1.0, z: 0.0, width: 0.4, flow_factor: 1.0, overhang_quartile: None },
///         Point3WithWidth { x: 10.0, y: 1.0, z: 0.0, width: 0.4, flow_factor: 1.0, overhang_quartile: None },
///     ],
///     role: ExtrusionRole::SparseInfill,
///     speed_factor: 1.0,
/// };
/// assert_no_path_intersections(&[path_a, path_b]);
/// ```
pub fn assert_no_path_intersections(paths: &[ExtrusionPath3D]) {
    // Collect all segments with path index
    let mut segments: Vec<(usize, [f32; 4])> = Vec::new();
    for (path_index, path) in paths.iter().enumerate() {
        for i in 0..path.points.len().saturating_sub(1) {
            let a = &path.points[i];
            let b = &path.points[i + 1];
            segments.push((path_index, [a.x, a.y, b.x, b.y]));
        }
    }

    // Check all pairs from different paths
    for i in 0..segments.len() {
        for j in (i + 1)..segments.len() {
            if segments[i].0 == segments[j].0 {
                continue; // same path, skip
            }
            let [ax1, ay1, ax2, ay2] = segments[i].1;
            let [bx1, by1, bx2, by2] = segments[j].1;
            if segments_intersect_proper(ax1, ay1, ax2, ay2, bx1, by1, bx2, by2) {
                panic!(
                    "paths {} and {} intersect: segment ({ax1},{ay1})->({ax2},{ay2}) \
                     crosses ({bx1},{by1})->({bx2},{by2})",
                    segments[i].0, segments[j].0
                );
            }
        }
    }
}

/// Test if two segments properly intersect (cross each other, not just share endpoints).
#[allow(clippy::too_many_arguments)]
fn segments_intersect_proper(
    ax1: f32,
    ay1: f32,
    ax2: f32,
    ay2: f32,
    bx1: f32,
    by1: f32,
    bx2: f32,
    by2: f32,
) -> bool {
    let d1 = cross_2d(bx1, by1, bx2, by2, ax1, ay1);
    let d2 = cross_2d(bx1, by1, bx2, by2, ax2, ay2);
    let d3 = cross_2d(ax1, ay1, ax2, ay2, bx1, by1);
    let d4 = cross_2d(ax1, ay1, ax2, ay2, bx2, by2);

    // Proper intersection: endpoints of each segment on opposite sides of the other
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }

    // Collinear overlap cases — check if an endpoint lies on the other segment
    let eps = 1e-10_f32;
    if d1.abs() < eps && on_segment_f32(bx1, by1, bx2, by2, ax1, ay1) {
        return true;
    }
    if d2.abs() < eps && on_segment_f32(bx1, by1, bx2, by2, ax2, ay2) {
        return true;
    }
    if d3.abs() < eps && on_segment_f32(ax1, ay1, ax2, ay2, bx1, by1) {
        return true;
    }
    if d4.abs() < eps && on_segment_f32(ax1, ay1, ax2, ay2, bx2, by2) {
        return true;
    }

    false
}

/// 2D cross product: sign of (B-A) × (P-A).
fn cross_2d(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

/// Check if point P is strictly within the bounding box of segment A-B (for collinear case).
/// Returns true only for interior points, not endpoints, to avoid false positives at
/// shared endpoints.
fn on_segment_f32(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> bool {
    let min_x = ax.min(bx);
    let max_x = ax.max(bx);
    let min_y = ay.min(by);
    let max_y = ay.max(by);
    // Strictly interior — exclude exact endpoint matches
    let is_endpoint = (px == ax && py == ay) || (px == bx && py == by);
    !is_endpoint && px >= min_x && px <= max_x && py >= min_y && py <= max_y
}
