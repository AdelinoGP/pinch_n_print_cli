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

/// Reconstructs `ExPolygon` contour/hole/island nesting from a Clipper2
/// `PolyTree64` (boolean-op or offset result). The root's direct children are
/// top-level outer contours; each contour's direct children are its holes;
/// islands nested inside a hole are fresh outer contours one level deeper
/// (recursed). Every ring is re-oriented to the Slic3r convention (contour
/// CCW, hole CW). Mirrors `triangle_mesh_slicer::polygons_to_expolygons`'s
/// tree walk — see that function's doc comment for why PolyTree
/// reconstruction is required instead of treating every result path as an
/// independent solid contour (the latter silently drops holes, which for a
/// Union/Difference/offset result means treating cutouts as solid area).
fn expolygons_from_tree(tree: &clipper2_rust::PolyTree64) -> Vec<ExPolygon> {
    let mut out = Vec::new();
    for &child in tree.nodes[0].children() {
        collect_expolygon_from_tree(tree, child, &mut out);
    }
    out
}

/// Build the `ExPolygon` rooted at contour node `node_idx`, attaching its
/// direct hole children and recursing into islands nested inside those holes.
fn collect_expolygon_from_tree(
    tree: &clipper2_rust::PolyTree64,
    node_idx: usize,
    out: &mut Vec<ExPolygon>,
) {
    let contour = oriented_ring(tree.nodes[node_idx].polygon(), true);

    let mut holes = Vec::new();
    for &hole_idx in tree.nodes[node_idx].children() {
        let hole = oriented_ring(tree.nodes[hole_idx].polygon(), false);
        if hole.len() >= 3 {
            holes.push(Polygon { points: hole });
        }
        // Islands sitting inside this hole are outer contours one level deeper.
        for &inner_idx in tree.nodes[hole_idx].children() {
            collect_expolygon_from_tree(tree, inner_idx, out);
        }
    }

    if contour.len() >= 3 {
        out.push(ExPolygon {
            contour: Polygon { points: contour },
            holes,
        });
    }
}

/// Orients a Clipper ring to the Slic3r convention: `want_ccw` forces CCW
/// (positive signed area) for outer contours, else CW (negative area) for
/// holes.
fn oriented_ring(path: &[Point64], want_ccw: bool) -> Vec<Point2> {
    let mut points: Vec<Point2> = path.iter().map(|p| Point2 { x: p.x, y: p.y }).collect();
    let is_ccw = signed_area_points(&points) > 0.0;
    if is_ccw != want_ccw {
        points.reverse();
    }
    points
}

/// Executes a boolean clip operation on polygon sets.
pub fn clip_polygons(
    subject: &[ExPolygon],
    clip: &[ExPolygon],
    op: ClipOperation,
) -> Vec<ExPolygon> {
    use clipper2_rust::core::FillRule;
    use clipper2_rust::{boolean_op_tree_64, ClipType, PolyTree64};

    // Flatten all polygons (contours + holes) into separate paths — clipper2
    // boolean ops work on flat path lists as input regardless of hierarchy.
    let subject_paths: Vec<Vec<Point64>> = subject.iter().flat_map(expolygon_to_paths).collect();
    let clip_paths: Vec<Vec<Point64>> = clip.iter().flat_map(expolygon_to_paths).collect();

    let clip_type = match op {
        ClipOperation::Union => ClipType::Union,
        ClipOperation::Intersection => ClipType::Intersection,
        ClipOperation::Difference => ClipType::Difference,
        ClipOperation::Xor => ClipType::Xor,
    };

    let mut tree = PolyTree64::new();
    boolean_op_tree_64(
        clip_type,
        FillRule::NonZero,
        &subject_paths,
        &clip_paths,
        &mut tree,
    );

    expolygons_from_tree(&tree)
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
    // miter_limit matches Clipper2's default (2.0); callers needing
    // OrcaSlicer's closing/opening miter limit go through `opening`/
    // `closing_ex`, which pass `ORCA_MORPH_MITER_LIMIT` explicitly.
    inflate_once(polygons, delta_mm, join, 2.0, arc_tolerance_mm)
}

/// Single inflate pass with every Clipper2 knob explicit. All offset-shaped
/// wrappers in this module route through here. Uses `ClipperOffset::execute_tree`
/// (rather than the `inflate_paths_64` free function, which only returns a flat
/// path list) so the result's hole/island nesting survives — see
/// [`expolygons_from_tree`].
fn inflate_once(
    polygons: &[ExPolygon],
    delta_mm: f32,
    join: OffsetJoinType,
    miter_limit: f64,
    arc_tolerance_mm: f32,
) -> Vec<ExPolygon> {
    use clipper2_rust::{ClipperOffset, EndType, JoinType, PolyTree64};

    // Convert delta from mm to scaled units (1 unit = 100nm = 10^-4mm)
    let delta_units = (delta_mm as f64) * slicer_ir::UNITS_PER_MM;
    if delta_units == 0.0 {
        // Matches `inflate_paths_64`'s own delta==0.0 short-circuit: an
        // insignificant offset is a no-op, so skip the offset/cleanup pass
        // entirely and hand back the input unchanged (hole structure intact
        // for free, since we never flatten it).
        return polygons.to_vec();
    }

    // Convert polygons to paths
    let paths: Vec<Vec<Point64>> = polygons.iter().flat_map(expolygon_to_paths).collect();
    if paths.is_empty() {
        return Vec::new();
    }

    // Map OffsetJoinType to clipper2_rust JoinType
    let join_type = match join {
        OffsetJoinType::Miter => JoinType::Miter,
        OffsetJoinType::Round => JoinType::Round,
        OffsetJoinType::Square => JoinType::Square,
    };

    let mut clip_offset = ClipperOffset::new(
        miter_limit,
        (arc_tolerance_mm as f64) * slicer_ir::UNITS_PER_MM,
        false,
        false,
    );
    clip_offset.add_paths(&paths, join_type, EndType::Polygon);

    let mut tree = PolyTree64::new();
    clip_offset.execute_tree(delta_units, &mut tree);

    expolygons_from_tree(&tree)
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

/// OrcaSlicer's default miter limit for its closing/opening helpers
/// (`ClipperUtils.hpp` `DefaultMiterLimit = 3.`). The plain [`offset`]
/// wrapper keeps Clipper2's default of 2.0; the morphological helpers use
/// Orca's value for `Miter` joins so parity call sites match the C++.
const ORCA_MORPH_MITER_LIMIT: f64 = 3.0;

/// Arc tolerance (mm) applied to `Round`-join morphological passes — the
/// pre-existing convention in this module. `Miter`/`Square` joins emit no
/// arcs, so they pass 0.
const MORPH_ROUND_ARC_TOLERANCE_MM: f32 = 0.05;

/// One erode/dilate pass of a morphological op, with per-join-type knobs
/// (`Round` keeps the historical 0.05 mm arc tolerance byte-for-byte;
/// `Miter` uses OrcaSlicer's miter limit 3.0).
fn morph_pass(subject: &[ExPolygon], delta_mm: f32, join: OffsetJoinType) -> Vec<ExPolygon> {
    let (miter_limit, arc_tolerance_mm) = match join {
        OffsetJoinType::Round => (2.0, MORPH_ROUND_ARC_TOLERANCE_MM),
        OffsetJoinType::Miter => (ORCA_MORPH_MITER_LIMIT, 0.0),
        OffsetJoinType::Square => (2.0, 0.0),
    };
    inflate_once(subject, delta_mm, join, miter_limit, arc_tolerance_mm)
}

/// Morphological opening: erode then dilate by `distance` (mm).
///
/// Opening removes thin bridges and small protrusions smaller than `distance`.
/// `join` is explicit at every call site: OrcaSlicer's `opening`/`closing`
/// default to `jtMiter` (`ClipperUtils.hpp`), so parity callers pass
/// [`OffsetJoinType::Miter`]; `Round` reproduces this helper's historical
/// hard-coded behaviour bit-for-bit.
pub fn opening(subject: &[ExPolygon], distance: f64, join: OffsetJoinType) -> Vec<ExPolygon> {
    let eroded = morph_pass(subject, -distance as f32, join);
    morph_pass(&eroded, distance as f32, join)
}

/// Morphological closing: dilate then erode by `distance` (mm).
///
/// Closing fills small gaps and holes smaller than `distance`. See
/// [`opening`] for the join-type contract (Miter = OrcaSlicer parity,
/// Round = historical behaviour).
pub fn closing_ex(subject: &[ExPolygon], distance: f64, join: OffsetJoinType) -> Vec<ExPolygon> {
    let dilated = morph_pass(subject, distance as f32, join);
    morph_pass(&dilated, -distance as f32, join)
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
    use clipper2_rust::{inflate_paths_64, ClipperOffset, EndType, JoinType, PolyTree64};

    let join_type = match join {
        OffsetJoinType::Miter => JoinType::Miter,
        OffsetJoinType::Round => JoinType::Round,
        OffsetJoinType::Square => JoinType::Square,
    };

    // Pass 1: delta1 (typically negative / erode). Flat paths only — hole
    // nesting is re-derived once, from pass 2's PolyTree below. Clipper2's
    // offset cleanup pass always emits correctly-wound rings regardless of
    // whether the caller captures a tree, so feeding pass 2 with pass 1's
    // flat output is equivalent to feeding it ExPolygons.
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

    // Pass 2: delta2 (typically positive / dilate). Capture the PolyTree so
    // the two-pass result's hole/island nesting survives — see
    // [`expolygons_from_tree`].
    let delta2_units = delta2_mm * slicer_ir::UNITS_PER_MM;
    let mut clip_offset = ClipperOffset::new(miter_limit, 0.0, false, false);
    clip_offset.add_paths(&intermediate, join_type, EndType::Polygon);
    let mut tree = PolyTree64::new();
    clip_offset.execute_tree(delta2_units, &mut tree);

    expolygons_from_tree(&tree)
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
    signed_area_points(&poly.points)
}

/// Signed area of a point ring using the shoelace formula. Returns a
/// positive value for CCW winding, negative for CW.
fn signed_area_points(pts: &[Point2]) -> f64 {
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

    /// Regression: `clip_polygons`/`union_ex` used to convert every Clipper
    /// result path into an independent solid `ExPolygon` (`holes: Vec::new()`
    /// unconditionally), discarding hole nesting entirely — see this file's
    /// pre-fix history. An outer CCW ring + a strictly-nested CW ring, passed
    /// as two flat "solid" `ExPolygon`s (exactly how a polygon-with-hole is
    /// represented as flat Clipper paths), must union under `FillRule::NonZero`
    /// into ONE `ExPolygon` with ONE hole, not two solid islands.
    #[test]
    fn union_ex_reconstructs_hole_from_opposite_wound_rings() {
        let outer = make_expoly(&[(0, 0), (10, 0), (10, 10), (0, 10)], &[]); // CCW
        let inner = make_expoly(&[(3, 3), (3, 7), (7, 7), (7, 3)], &[]); // CW
        let result = union_ex(&[outer, inner]);
        assert_eq!(
            result.len(),
            1,
            "must merge into a single ExPolygon, not two solid islands"
        );
        assert_eq!(
            result[0].holes.len(),
            1,
            "the CW inner ring must reconstruct as a hole, not a second solid contour"
        );
    }

    /// Regression for the `apply_slice_closing_radius` bug (handoff gap 1):
    /// `offset`'s underlying `inflate_once` used to flatten every result path
    /// into an independent solid `ExPolygon`, so the OrcaSlicer
    /// inflate-then-deflate `slice_closing_radius` round trip silently turned
    /// any hole into a second solid contour. The hole here (0.8mm) is much
    /// larger than 2x the closing radius (0.1mm), so it must survive both
    /// dimensionally (not closed) and hierarchically (still nested as a hole).
    #[test]
    fn offset_erodes_hole_open_correctly_when_shrinking_solid() {
        let annulus = make_expoly(
            &[(0, 0), (20000, 0), (20000, 20000), (0, 20000)],
            &[&[(6000, 6000), (6000, 14000), (14000, 14000), (14000, 6000)]],
        );
        // Erode (shrink solid) by 0.1mm — outer contour shrinks inward, hole grows.
        let result = offset(&[annulus], -0.1, OffsetJoinType::Miter, 0.0);
        assert_eq!(
            result.len(),
            1,
            "erosion must stay a single ExPolygon with its hole, not split (got {result:?})"
        );
        assert_eq!(
            result[0].holes.len(),
            1,
            "hole must survive erosion (grown, not vanished/flattened)"
        );
    }

    #[test]
    fn offset_iterative_erosion_mimics_emit_walls_loop() {
        // Mirrors classic-perimeters emit_walls's iterative inset loop: i=0 by
        // -0.2mm, i=1 by -0.4mm, i>=2 by -0.4mm each, feeding each result into
        // the next. 1 unit = 100nm, so 10_000 units = 1mm: outer 20mm square
        // (0,0)-(200000,200000), 8mm hole (60000,60000)-(140000,140000) — a
        // 6mm gap each side, matching perimeter_parity.rs's real
        // `annulus_frame_mesh` fixture dimensions. Plenty of room for 6
        // iterations (~2.4mm total consumed of a 6mm gap).
        let annulus = make_expoly(
            &[(0, 0), (200000, 0), (200000, 200000), (0, 200000)],
            &[&[
                (60000, 60000),
                (60000, 140000),
                (140000, 140000),
                (140000, 60000),
            ]],
        );
        let mut current = vec![annulus];
        let deltas = [-0.2, -0.4, -0.4, -0.4, -0.4, -0.4];
        for (i, &delta) in deltas.iter().enumerate() {
            let next = offset(&current, delta, OffsetJoinType::Miter, 0.0125);
            assert!(!next.is_empty(), "iteration {i}: offset produced no output");
            assert_eq!(
                next.len(),
                1,
                "iteration {i}: must stay a single ExPolygon, not split into solid islands"
            );
            assert_eq!(
                next[0].holes.len(),
                1,
                "iteration {i}: hole must survive this erosion step"
            );
            current = next;
        }
    }

    #[test]
    fn slice_closing_radius_round_trip_on_real_annulus_dimensions() {
        // Exact production scenario: `apply_slice_closing_radius` (Round join,
        // r=0.049mm, the ResolvedConfig default) applied to the same
        // dimensions as perimeter_parity.rs's `annulus_frame_mesh` fixture
        // (20mm outer, 6-14mm hole).
        let annulus = make_expoly(
            &[(0, 0), (200000, 0), (200000, 200000), (0, 200000)],
            &[&[
                (60000, 60000),
                (60000, 140000),
                (140000, 140000),
                (140000, 60000),
            ]],
        );
        let r = 0.049;
        let inflated = offset(&[annulus], r, OffsetJoinType::Round, 0.0);
        let result = offset(&inflated, -r, OffsetJoinType::Round, 0.0);
        assert_eq!(result.len(), 1, "must stay a single ExPolygon");
        assert_eq!(result[0].holes.len(), 1, "hole must survive the round trip");
    }

    #[test]
    fn offset_round_trip_preserves_hole_nesting() {
        // 2mm outer square (0,0)-(20000,20000), CCW; 0.8mm hole
        // (6000,6000)-(14000,14000), CW.
        let annulus = make_expoly(
            &[(0, 0), (20000, 0), (20000, 20000), (0, 20000)],
            &[&[(6000, 6000), (6000, 14000), (14000, 14000), (14000, 6000)]],
        );
        let inflated = offset(&[annulus], 0.1, OffsetJoinType::Round, 0.0);
        let result = offset(&inflated, -0.1, OffsetJoinType::Round, 0.0);
        assert_eq!(
            result.len(),
            1,
            "must stay a single ExPolygon, not split into two solid islands"
        );
        assert_eq!(
            result[0].holes.len(),
            1,
            "the 0.8mm hole must survive the closing-radius round trip as a hole"
        );
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
        let result = opening(&[sq], 0.1, OffsetJoinType::Miter);
        assert!(!result.is_empty());
    }

    #[test]
    fn closing_ex_dilates_then_erodes() {
        let sq = make_expoly(&[(0, 0), (20000, 0), (20000, 20000), (0, 20000)], &[]);
        let result = closing_ex(&[sq], 0.1, OffsetJoinType::Miter);
        assert!(!result.is_empty());
    }

    /// The `Round` join reproduces the helpers' historical hard-coded
    /// behaviour: on a sharp-cornered square it tessellates corner arcs,
    /// so it must emit strictly more vertices than the `Miter` join, which
    /// adds at most one point per corner.
    #[test]
    fn closing_ex_round_join_is_the_arc_tessellating_legacy_path() {
        let sq = make_expoly(&[(0, 0), (20000, 0), (20000, 20000), (0, 20000)], &[]);
        let round_pts: usize = closing_ex(std::slice::from_ref(&sq), 0.5, OffsetJoinType::Round)
            .iter()
            .map(|e| e.contour.points.len())
            .sum();
        let miter_pts: usize = closing_ex(&[sq], 0.5, OffsetJoinType::Miter)
            .iter()
            .map(|e| e.contour.points.len())
            .sum();
        assert!(
            round_pts > miter_pts,
            "Round closing should tessellate corner arcs (round={round_pts}, miter={miter_pts})"
        );
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
