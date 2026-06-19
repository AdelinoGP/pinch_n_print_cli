// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/ClipperUtils.cpp / Polygon.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Polygon clipping and offset primitives.

use clipper2_rust::Point64;
use slicer_ir::slice_ir::BoundingBox2;
use slicer_ir::{ExPolygon, Point2, Polygon};

/// A 2D line segment with endpoints in scaled integer coordinates (1 unit = 100 nm).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    /// Start point.
    pub start: Point2,
    /// End point.
    pub end: Point2,
}

/// Boolean clip operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipOperation {
    /// Union of all subject and clip polygons.
    Union,
    /// Intersection between subject and clip polygons.
    Intersection,
    /// Difference: subject minus clip.
    Difference,
    /// Exclusive-or between subject and clip polygons.
    Xor,
}

/// Join style used for polygon offsetting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetJoinType {
    /// Sharp corners.
    Miter,
    /// Rounded corners.
    Round,
    /// Squared corners.
    Square,
}

/// Converts a Polygon to a Path64 (Vec<Point64>) for clipper2-rust.
fn polygon_to_path(poly: &Polygon) -> Vec<Point64> {
    poly.points
        .iter()
        .map(|p| Point64 { x: p.x, y: p.y })
        .collect()
}

/// Converts an ExPolygon to a vector of paths (contour + holes)
/// for clipper2-rust.
fn expolygon_to_paths(exp: &ExPolygon) -> Vec<Vec<Point64>> {
    let mut paths = Vec::new();
    // Contour
    paths.push(polygon_to_path(&exp.contour));
    // Holes
    for hole in &exp.holes {
        paths.push(polygon_to_path(hole));
    }
    paths
}

/// Executes a boolean clip operation on polygon sets.
pub fn clip_polygons(
    subject: &[ExPolygon],
    clip: &[ExPolygon],
    op: ClipOperation,
) -> Vec<ExPolygon> {
    use clipper2_rust::core::FillRule;
    use clipper2_rust::{difference_64, intersect_64, union_64, xor_64};

    // Flatten all polygons (contours + holes) into separate paths
    // This is necessary because clipper2 operations work on flat path lists
    // Note: This approach loses hole-contour relationships
    let subject_paths: Vec<Vec<Point64>> = subject.iter().flat_map(expolygon_to_paths).collect();
    let clip_paths: Vec<Vec<Point64>> = clip.iter().flat_map(expolygon_to_paths).collect();

    let result_paths = match op {
        ClipOperation::Union => union_64(&subject_paths, &clip_paths, FillRule::NonZero),
        ClipOperation::Intersection => intersect_64(&subject_paths, &clip_paths, FillRule::NonZero),
        ClipOperation::Difference => difference_64(&subject_paths, &clip_paths, FillRule::NonZero),
        ClipOperation::Xor => xor_64(&subject_paths, &clip_paths, FillRule::NonZero),
    };

    // Convert result paths back to ExPolygon
    // Note: This simple conversion treats every path as a separate ExPolygon with no holes.
    // A full implementation would use PolyTree to reconstruct hierarchy.
    result_paths
        .into_iter()
        .map(|path| {
            // Reconstruct Polygon from Point64 vector
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

/// Computes the union of polygon sets.
pub fn union(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    clip_polygons(subject, clip, ClipOperation::Union)
}

/// Computes the intersection of polygon sets.
pub fn intersection(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    clip_polygons(subject, clip, ClipOperation::Intersection)
}

/// Computes the difference of polygon sets.
pub fn difference(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    clip_polygons(subject, clip, ClipOperation::Difference)
}

/// Computes the exclusive-or of polygon sets.
pub fn xor(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    clip_polygons(subject, clip, ClipOperation::Xor)
}

/// Returned by [`validate_polygon_simplicity`] when a polygon fails the simplicity
/// check. `contour_indices` lists the indices of contours (outer = 0; holes = 1..)
/// that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolygonSimplicityError {
    /// Indices of the contours that failed the simplicity check.
    /// Index 0 is the outer contour; indices 1.. are holes in order.
    pub contour_indices: Vec<usize>,
}

/// Verify that every contour of `poly` is simple (no self-intersections, no
/// duplicate-vertex degeneracies that would break clipper2 set-ops). Wraps the
/// clipper2-rust simplicity primitive: a polygon-set passed through clipper2's
/// union with itself produces the same area iff every contour is simple. We use
/// the more direct route — re-running the input through clipper2's union and
/// comparing contour count + signed area per contour.
///
/// Returns `Ok(())` for simple polygons; `Err(PolygonSimplicityError { contour_indices })`
/// listing the failing contour indices when invalid.
pub fn validate_polygon_simplicity(poly: &ExPolygon) -> Result<(), PolygonSimplicityError> {
    use clipper2_rust::core::FillRule;
    use clipper2_rust::{area, union_64};

    // Epsilon for area comparison: 1.0 in workspace units² (1 unit = 100 nm).
    // Self-intersecting contours (e.g. bowties) lose significant area after
    // union cleans them, so 1.0 unit² is a conservative but safe threshold.
    const AREA_EPSILON: f64 = 1.0;

    // Build a flat iterator of (index, path) for outer contour + holes.
    let outer_path: Vec<Point64> = polygon_to_path(&poly.contour);
    let all_contours: Vec<(usize, Vec<Point64>)> = std::iter::once((0usize, outer_path))
        .chain(
            poly.holes
                .iter()
                .enumerate()
                .map(|(i, h)| (i + 1, polygon_to_path(h))),
        )
        .collect();

    let mut failing: Vec<usize> = Vec::new();

    for (idx, path) in &all_contours {
        let original_area = area(path).abs();

        // Self-union a single contour; a simple ring stays as one ring with
        // the same area. A bowtie or self-crossing ring splits into 2+ rings
        // or the area changes significantly.
        let subject: Vec<Vec<Point64>> = vec![path.clone()];
        let clip: Vec<Vec<Point64>> = Vec::new();
        let result = union_64(&subject, &clip, FillRule::NonZero);

        let changed = if result.len() != 1 {
            true
        } else {
            let result_area = area(&result[0]).abs();
            (result_area - original_area).abs() > AREA_EPSILON
        };

        if changed {
            failing.push(*idx);
        }
    }

    if failing.is_empty() {
        Ok(())
    } else {
        Err(PolygonSimplicityError {
            contour_indices: failing,
        })
    }
}

/// Offsets polygons by `delta_mm` millimeters.
pub fn offset(
    polygons: &[ExPolygon],
    delta_mm: f32,
    join: OffsetJoinType,
    arc_tolerance_mm: f32,
) -> Vec<ExPolygon> {
    use clipper2_rust::inflate_paths_64;
    use clipper2_rust::{EndType, JoinType};

    // Convert polygons to paths
    let paths: Vec<Vec<Point64>> = polygons.iter().flat_map(expolygon_to_paths).collect();

    // Convert delta from mm to scaled units (1 unit = 100nm = 10^-4mm)
    let delta_units = (delta_mm as f64) * slicer_ir::UNITS_PER_MM;

    // Map OffsetJoinType to clipper2_rust JoinType
    let join_type = match join {
        OffsetJoinType::Miter => JoinType::Miter,
        OffsetJoinType::Round => JoinType::Round,
        OffsetJoinType::Square => JoinType::Square,
    };

    // Execute offset operation
    // inflate_paths_64 signature: inflate_paths_64(&paths, delta, join_type, end_type, miter_limit, arc_tolerance)
    // We use EndType::Polygon for closed polygon offsetting
    // miter_limit and arc_tolerance can be defaults (2.0 and 0.0)
    let result_paths = inflate_paths_64(
        &paths,
        delta_units,
        join_type,
        EndType::Polygon,
        2.0,
        (arc_tolerance_mm as f64) * slicer_ir::UNITS_PER_MM,
    );

    // Convert result paths back to ExPolygon
    // Note: Same limitation as clip_polygons - treats every path as separate ExPolygon with no holes
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

/// Union of all subject ExPolygons with each other (no separate clip set).
///
/// Wraps the existing [`union`] by using an empty clip set so that only
/// subject-vs-subject overlaps are merged.
pub fn union_ex(subject: &[ExPolygon]) -> Vec<ExPolygon> {
    union(subject, &[])
}

/// Pairwise intersection of two ExPolygon sets.
///
/// Wraps the existing [`intersection`].
pub fn intersection_ex(a: &[ExPolygon], b: &[ExPolygon]) -> Vec<ExPolygon> {
    intersection(a, b)
}

/// Difference of two ExPolygon sets.
///
/// Wraps the existing [`difference`].
pub fn difference_ex(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    difference(subject, clip)
}

/// Morphological opening: erode then dilate by `distance` (mm).
///
/// Opening removes thin bridges and small protrusions smaller than `distance`.
pub fn opening(subject: &[ExPolygon], distance: f64) -> Vec<ExPolygon> {
    let eroded = offset(subject, -distance as f32, OffsetJoinType::Round, 0.05);
    offset(&eroded, distance as f32, OffsetJoinType::Round, 0.05)
}

/// Morphological closing: dilate then erode by `distance` (mm).
///
/// Closing fills small gaps and holes smaller than `distance`.
pub fn closing_ex(subject: &[ExPolygon], distance: f64) -> Vec<ExPolygon> {
    let dilated = offset(subject, distance as f32, OffsetJoinType::Round, 0.05);
    offset(&dilated, -distance as f32, OffsetJoinType::Round, 0.05)
}

/// Two-pass offset: apply `delta1_mm` first (negative = erode), then `delta2_mm` (positive = dilate).
///
/// Mirrors OrcaSlicer `ClipperUtils::offset2_ex(input, delta1, delta2, JoinType, miterLimit)`.
/// Argument order (negative-first, positive-second) is a contract: callers must pass deltas
/// in that semantic order.
///
/// `miter_limit` is forwarded to the underlying Clipper2 inflate call.  The existing `offset`
/// wrapper uses a hard-coded miter limit of 2.0; here we accept an explicit value and pass it
/// through by delegating to the low-level `inflate_paths_64` directly for both passes so that
/// the caller-supplied miter limit is respected.
pub fn offset2_ex(
    polys: &[ExPolygon],
    delta1_mm: f64,
    delta2_mm: f64,
    join: OffsetJoinType,
    miter_limit: f64,
) -> Vec<ExPolygon> {
    use clipper2_rust::inflate_paths_64;
    use clipper2_rust::{EndType, JoinType};

    let join_type = match join {
        OffsetJoinType::Miter => JoinType::Miter,
        OffsetJoinType::Round => JoinType::Round,
        OffsetJoinType::Square => JoinType::Square,
    };

    // Pass 1: delta1 (typically negative / erode)
    let paths1: Vec<Vec<Point64>> = polys.iter().flat_map(expolygon_to_paths).collect();
    let delta1_units = delta1_mm * slicer_ir::UNITS_PER_MM;
    let intermediate = inflate_paths_64(
        &paths1,
        delta1_units,
        join_type,
        EndType::Polygon,
        miter_limit,
        0.0,
    );

    if intermediate.is_empty() {
        return Vec::new();
    }

    // Pass 2: delta2 (typically positive / dilate)
    let delta2_units = delta2_mm * slicer_ir::UNITS_PER_MM;
    let result_paths = inflate_paths_64(
        &intermediate,
        delta2_units,
        join_type,
        EndType::Polygon,
        miter_limit,
        0.0,
    );

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

/// Morphological open using configurable join type and miter limit.
///
/// `opening_ex(polys, d, join, miter)` = `offset2_ex(polys, -d, +d, join, miter)`.
/// Removes thin protrusions and bridges smaller than `delta_mm`.
pub fn opening_ex(
    polys: &[ExPolygon],
    delta_mm: f64,
    join: OffsetJoinType,
    miter_limit: f64,
) -> Vec<ExPolygon> {
    offset2_ex(polys, -delta_mm, delta_mm, join, miter_limit)
}

/// Keeps only the single [`ExPolygon`] with the greatest contour area (CCW shoelace area).
///
/// On ties (equal area within float equality), the lower-indexed polygon is kept.
/// If `polys` is empty the call is a no-op.
pub fn keep_largest_contour_only(polys: &mut Vec<ExPolygon>) {
    if polys.is_empty() {
        return;
    }
    let best = polys
        .iter()
        .enumerate()
        .max_by(|(i, a), (j, b)| {
            let area_a = signed_area(&a.contour).abs();
            let area_b = signed_area(&b.contour).abs();
            // On float tie, lower index wins (so compare reversed for max_by)
            area_a
                .partial_cmp(&area_b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(j.cmp(i)) // higher j loses → lower index is kept
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    let winner = polys.swap_remove(best);
    polys.clear();
    polys.push(winner);
}

/// Signed area of a polygon ring (contour) using the shoelace formula.
/// Returns a positive value for CCW winding, negative for CW.
fn signed_area(poly: &Polygon) -> f64 {
    let pts = &poly.points;
    if pts.len() < 3 {
        return 0.0;
    }
    let mut area = 0i64;
    for i in 0..pts.len() {
        let j = (i + 1) % pts.len();
        area += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
    }
    area as f64 * 0.5
}

/// Filter out polygons whose contour area is below `min_area`, and remove
/// holes whose area is below `min_hole_area`.  Operates in-place on `polys`.
///
/// Areas are in workspace-unit² (1 unit = 100 nm, so 1 unit² = 10⁻⁸ mm²).
pub fn remove_small_and_small_holes(polys: &mut Vec<ExPolygon>, min_area: f64, min_hole_area: f64) {
    polys.retain(|exp| signed_area(&exp.contour).abs() >= min_area);
    for exp in polys.iter_mut() {
        exp.holes
            .retain(|hole| signed_area(hole).abs() >= min_hole_area);
    }
}

/// Simplify all contours (outer + holes) of each ExPolygon using the
/// Ramer-Douglas-Peucker algorithm with the given `tolerance` (in workspace
/// units).
pub fn expolygons_simplify(polygons: &[ExPolygon], tolerance: f64) -> Vec<ExPolygon> {
    polygons
        .iter()
        .map(|exp| ExPolygon {
            contour: simplify_polygon(&exp.contour, tolerance),
            holes: exp
                .holes
                .iter()
                .map(|h| simplify_polygon(h, tolerance))
                .collect(),
        })
        .collect()
}

/// Ramer-Douglas-Peucker simplification of a polygon ring.
fn simplify_polygon(poly: &Polygon, tolerance: f64) -> Polygon {
    let pts = &poly.points;
    if pts.len() <= 2 {
        return poly.clone();
    }
    let mut keep = vec![false; pts.len()];
    rdp(pts, 0, pts.len() - 1, tolerance, &mut keep);
    // Always keep first and last (which are the same for a closed ring,
    // but we keep all marked points).
    keep[0] = true;
    keep[pts.len() - 1] = true;
    Polygon {
        points: pts
            .iter()
            .zip(keep.iter())
            .filter_map(|(p, &k)| if k { Some(*p) } else { None })
            .collect(),
    }
}

/// Recursive Ramer-Douglas-Peucker helper.
fn rdp(points: &[Point2], start: usize, end: usize, epsilon: f64, keep: &mut [bool]) {
    if end <= start + 1 {
        return;
    }
    let (mut max_dist, mut max_idx) = (0.0f64, start + 1);
    let sx = points[start].x as f64;
    let sy = points[start].y as f64;
    let ex = points[end].x as f64;
    let ey = points[end].y as f64;
    let dx = ex - sx;
    let dy = ey - sy;
    let line_len_sq = dx * dx + dy * dy;

    for i in (start + 1)..end {
        let px = points[i].x as f64;
        let py = points[i].y as f64;
        let dist = if line_len_sq < 1.0 {
            ((px - sx) * (px - sx) + (py - sy) * (py - sy)).sqrt()
        } else {
            ((dy * px - dx * py + ex * sy - ey * sx).abs()) / line_len_sq.sqrt()
        };
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    if max_dist > epsilon {
        keep[max_idx] = true;
        rdp(points, start, max_idx, epsilon, keep);
        rdp(points, max_idx, end, epsilon, keep);
    }
}

/// Remove consecutive duplicate (identical) points from `points` in-place.
pub fn remove_duplicates(points: &mut Vec<Point2>) {
    points.dedup();
}

/// Cohen-Sutherland outcodes for line clipping.
const INSIDE: u8 = 0;
const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const BOTTOM: u8 = 4;
const TOP: u8 = 8;

fn outcode(p: Point2, bbox: &BoundingBox2) -> u8 {
    let mut code = INSIDE;
    if p.x < bbox.min.x {
        code |= LEFT;
    } else if p.x > bbox.max.x {
        code |= RIGHT;
    }
    if p.y < bbox.min.y {
        code |= BOTTOM;
    } else if p.y > bbox.max.y {
        code |= TOP;
    }
    code
}

/// Clip a 2D line segment to a bounding box using the Cohen-Sutherland algorithm.
///
/// Returns `Some(Line)` with the clipped endpoints, or `None` if the segment
/// lies entirely outside the bbox.
pub fn clip_line_with_bbox(line: &Line, bbox: &BoundingBox2) -> Option<Line> {
    let mut x0 = line.start.x as f64;
    let mut y0 = line.start.y as f64;
    let mut x1 = line.end.x as f64;
    let mut y1 = line.end.y as f64;

    let mut code0 = outcode(line.start, bbox);
    let mut code1 = outcode(line.end, bbox);

    loop {
        if code0 == 0 && code1 == 0 {
            return Some(Line {
                start: Point2 {
                    x: x0 as i64,
                    y: y0 as i64,
                },
                end: Point2 {
                    x: x1 as i64,
                    y: y1 as i64,
                },
            });
        }
        if code0 & code1 != 0 {
            return None;
        }

        let (x, y, new_code) = if code0 != 0 {
            clip_endpoint(x0, y0, x1, y1, code0, bbox)
        } else {
            clip_endpoint(x1, y1, x0, y0, code1, bbox)
        };

        if code0 != 0 {
            x0 = x;
            y0 = y;
            code0 = new_code;
        } else {
            x1 = x;
            y1 = y;
            code1 = new_code;
        }
    }
}

fn clip_endpoint(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    code: u8,
    bbox: &BoundingBox2,
) -> (f64, f64, u8) {
    let xmin = bbox.min.x as f64;
    let xmax = bbox.max.x as f64;
    let ymin = bbox.min.y as f64;
    let ymax = bbox.max.y as f64;

    let (mut x, mut y) = (0.0, 0.0);
    if code & TOP != 0 {
        x = x0 + (x1 - x0) * (ymax - y0) / (y1 - y0);
        y = ymax;
    } else if code & BOTTOM != 0 {
        x = x0 + (x1 - x0) * (ymin - y0) / (y1 - y0);
        y = ymin;
    } else if code & RIGHT != 0 {
        y = y0 + (y1 - y0) * (xmax - x0) / (x1 - x0);
        x = xmax;
    } else if code & LEFT != 0 {
        y = y0 + (y1 - y0) * (xmin - x0) / (x1 - x0);
        x = xmin;
    }
    (
        x,
        y,
        outcode(
            Point2 {
                x: x as i64,
                y: y as i64,
            },
            bbox,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ExPolygon, Point2, Polygon};

    fn make_polygon(pts: &[(i64, i64)]) -> Polygon {
        Polygon {
            points: pts.iter().map(|&(x, y)| Point2 { x, y }).collect(),
        }
    }

    fn make_expoly(contour: &[(i64, i64)], holes: &[&[(i64, i64)]]) -> ExPolygon {
        ExPolygon {
            contour: make_polygon(contour),
            holes: holes.iter().map(|h| make_polygon(h)).collect(),
        }
    }

    fn square_10() -> ExPolygon {
        make_expoly(&[(0, 0), (10, 0), (10, 10), (0, 10)], &[])
    }

    fn square_10_offset(dx: i64, dy: i64) -> ExPolygon {
        make_expoly(
            &[(dx, dy), (10 + dx, dy), (10 + dx, 10 + dy), (dx, 10 + dy)],
            &[],
        )
    }

    #[test]
    fn clip_operation_variants_are_distinct() {
        assert_ne!(ClipOperation::Union, ClipOperation::Difference);
    }

    #[test]
    fn validate_polygon_simplicity_accepts_simple_square() {
        let square = square_10();
        assert!(validate_polygon_simplicity(&square).is_ok());
    }

    #[test]
    fn validate_polygon_simplicity_rejects_bowtie() {
        let bowtie = make_expoly(&[(0, 0), (10, 10), (10, 0), (0, 10)], &[]);
        let result = validate_polygon_simplicity(&bowtie);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contour_indices.contains(&0),
            "outer contour (index 0) must be flagged"
        );
    }

    #[test]
    fn union_ex_merges_overlapping_squares() {
        let a = square_10();
        let b = square_10_offset(5, 0);
        let result = union_ex(&[a, b]);
        assert!(!result.is_empty());
    }

    #[test]
    fn intersection_ex_returns_overlap() {
        let a = square_10();
        let b = square_10_offset(5, 0);
        let result = intersection_ex(&[a], &[b]);
        assert!(!result.is_empty());
    }

    #[test]
    fn difference_ex_removes_overlap() {
        let a = square_10();
        let b = square_10_offset(5, 0);
        let result = difference_ex(&[a], &[b]);
        assert!(!result.is_empty());
    }

    #[test]
    fn opening_erodes_then_dilates() {
        // 20000 units = 2mm per side; erode/dilate by 0.1mm (1000 units)
        let sq = make_expoly(&[(0, 0), (20000, 0), (20000, 20000), (0, 20000)], &[]);
        let result = opening(&[sq], 0.1);
        assert!(!result.is_empty());
    }

    #[test]
    fn closing_ex_dilates_then_erodes() {
        let sq = make_expoly(&[(0, 0), (20000, 0), (20000, 20000), (0, 20000)], &[]);
        let result = closing_ex(&[sq], 0.1);
        assert!(!result.is_empty());
    }

    #[test]
    fn remove_small_removes_tiny_polygon() {
        let big = square_10();
        let tiny = make_expoly(&[(0, 0), (1, 0), (1, 1), (0, 1)], &[]);
        let mut polys = vec![big, tiny];
        // min_area = 50; tiny has area 1, big has area 100
        remove_small_and_small_holes(&mut polys, 50.0, 0.0);
        assert_eq!(polys.len(), 1);
    }

    #[test]
    fn remove_small_removes_small_holes() {
        let outer = make_polygon(&[(0, 0), (100, 0), (100, 100), (0, 100)]);
        let hole = make_polygon(&[(10, 10), (11, 10), (11, 11), (10, 11)]); // area = 1
        let mut polys = vec![ExPolygon {
            contour: outer,
            holes: vec![hole],
        }];
        remove_small_and_small_holes(&mut polys, 0.0, 50.0);
        assert!(polys[0].holes.is_empty(), "small hole should be removed");
    }

    #[test]
    fn expolygons_simplify_preserves_square() {
        let sq = square_10();
        let result = expolygons_simplify(&[sq], 0.1);
        assert_eq!(result.len(), 1);
        // A square has 4 corners; RDP should keep all of them at tolerance 0.1
        assert!(result[0].contour.points.len() >= 4);
    }

    #[test]
    fn remove_duplicates_collapses_runs() {
        let mut pts = vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 0, y: 0 },
            Point2 { x: 1, y: 1 },
            Point2 { x: 1, y: 1 },
            Point2 { x: 1, y: 1 },
            Point2 { x: 2, y: 2 },
        ];
        remove_duplicates(&mut pts);
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], Point2 { x: 0, y: 0 });
        assert_eq!(pts[1], Point2 { x: 1, y: 1 });
        assert_eq!(pts[2], Point2 { x: 2, y: 2 });
    }

    #[test]
    fn clip_line_with_bbox_fully_inside() {
        let bbox = BoundingBox2 {
            min: Point2 { x: 0, y: 0 },
            max: Point2 { x: 100, y: 100 },
        };
        let line = Line {
            start: Point2 { x: 10, y: 10 },
            end: Point2 { x: 50, y: 50 },
        };
        let result = clip_line_with_bbox(&line, &bbox);
        assert!(result.is_some());
        let clipped = result.unwrap();
        assert_eq!(clipped.start, line.start);
        assert_eq!(clipped.end, line.end);
    }

    #[test]
    fn clip_line_with_bbox_fully_outside() {
        let bbox = BoundingBox2 {
            min: Point2 { x: 0, y: 0 },
            max: Point2 { x: 100, y: 100 },
        };
        let line = Line {
            start: Point2 { x: 200, y: 200 },
            end: Point2 { x: 300, y: 300 },
        };
        assert!(clip_line_with_bbox(&line, &bbox).is_none());
    }

    #[test]
    fn clip_line_with_bbox_partial_clip() {
        let bbox = BoundingBox2 {
            min: Point2 { x: 0, y: 0 },
            max: Point2 { x: 100, y: 100 },
        };
        let line = Line {
            start: Point2 { x: -50, y: 50 },
            end: Point2 { x: 150, y: 50 },
        };
        let result = clip_line_with_bbox(&line, &bbox);
        assert!(result.is_some());
        let clipped = result.unwrap();
        assert_eq!(clipped.start.x, 0);
        assert_eq!(clipped.end.x, 100);
    }
}
