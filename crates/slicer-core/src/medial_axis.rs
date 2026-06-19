// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Geometry/MedialAxis.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Voronoi-diagram medial axis (centerline) for `ExPolygon` inputs.
//!
//! # Overview
//!
//! Implements a segment-site Voronoi diagram via `boostvoronoi` 0.12, filtering
//! interior primary edges and stitching them into [`ThickPolyline`] chains. This
//! is the production VD port (packet 103 / T-041), replacing the M1 long-axis
//! spine approximation.
//!
//! # Feature gating
//!
//! The implementation — including [`MedialAxisError`] and [`medial_axis`] — is
//! compiled only when the `host-algos` feature is enabled (which pulls in
//! `boostvoronoi`). The module itself is always declared in `lib.rs` so the
//! crate compiles cleanly without the feature.
//!
//! # Validation strategy
//!
//! Analytic smoke tests live in the inline `smoke` submodule (enabled with the
//! feature). Rigorous fixture-based golden tests are authored separately
//! (see `tests/fixtures/medial_axis_golden/`).

#[cfg(feature = "host-algos")]
mod impl_ {
    use boostvoronoi::diagram::SourceCategory;
    use boostvoronoi::geometry::{Line as BvLine, Point as BvPoint};
    use boostvoronoi::prelude::{Builder, VoronoiVisualUtils};
    use boostvoronoi::utils::visual_utils::SimpleAffine;

    use slicer_ir::slice_ir::Point2WithWidth;
    use slicer_ir::{point_in_polygon_winding, ExPolygon, Point2, ThickPolyline};

    const POINT_IN_POLY_EPS_MM: f64 = 0.0;

    /// Scaled epsilon for on-boundary checks (in integer coordinate units).
    /// 1 unit = 100 nm; SCALED_EPSILON = 1 unit (100 nm squared distance threshold).
    const SCALED_EPSILON_SQ: f64 = 1.0;

    /// Errors that can arise when computing the medial axis.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MedialAxisError {
        /// The contour has fewer than 3 distinct points (or is zero-area).
        DegenerateInput,
        /// At least one coordinate exceeds the i32 range required by boostvoronoi.
        CoordinateOverflow {
            /// The first coordinate component found outside the i32 range, in i64 units (1 unit = 100 nm).
            actual_max: i64,
            /// The i32 ceiling: 2_147_483_647.
            i32_max: i64,
        },
    }

    /// Douglas-Peucker polyline simplification operating on integer-unit `Point2` values.
    ///
    /// Removes near-collinear intermediate points whose perpendicular distance to the
    /// chord is below `epsilon_sq` (squared, in i64 units²).  Closed rings: call with
    /// the ring open (first ≠ last); the function preserves the endpoints and does not
    /// re-close the ring.  Returns at least the two endpoints unless `pts` is shorter.
    ///
    /// Rationale: finely-tessellated arcs (e.g. a 50-segment arc approximation of a
    /// curved boundary) produce a dense near-collinear segment cloud that causes the
    /// Voronoi diagram to generate a spurious junction-web along the arc.  DP at
    /// 0.0115 mm (EPSILON_OFFSET_UNITS = 115 units) collapses such runs while
    /// preserving geometry within tolerance.
    fn douglas_peucker(pts: &[Point2], epsilon_sq: f64) -> Vec<Point2> {
        if pts.len() <= 2 {
            return pts.to_vec();
        }

        // Recursive helper: simplify pts[start..=end], appending to `out`.
        fn dp_recursive(
            pts: &[Point2],
            start: usize,
            end: usize,
            epsilon_sq: f64,
            out: &mut Vec<Point2>,
        ) {
            if end <= start + 1 {
                // Nothing between start and end to consider.
                return;
            }

            let ax = pts[start].x as f64;
            let ay = pts[start].y as f64;
            let bx = pts[end].x as f64;
            let by = pts[end].y as f64;
            let dx = bx - ax;
            let dy = by - ay;
            let len_sq = dx * dx + dy * dy;

            // Find the point with maximum perpendicular distance from segment [start, end].
            let mut max_dist_sq = 0.0_f64;
            let mut max_idx = start + 1;

            for i in (start + 1)..end {
                let px = pts[i].x as f64;
                let py = pts[i].y as f64;

                let dist_sq = if len_sq < 1e-10 {
                    // Degenerate chord: distance to the start point.
                    let ex = px - ax;
                    let ey = py - ay;
                    ex * ex + ey * ey
                } else {
                    // Perpendicular distance squared = |cross|² / len_sq.
                    let cross = (px - ax) * dy - (py - ay) * dx;
                    cross * cross / len_sq
                };

                if dist_sq > max_dist_sq {
                    max_dist_sq = dist_sq;
                    max_idx = i;
                }
            }

            if max_dist_sq > epsilon_sq {
                // Split at max_idx: recurse both halves.
                dp_recursive(pts, start, max_idx, epsilon_sq, out);
                out.push(pts[max_idx]);
                dp_recursive(pts, max_idx, end, epsilon_sq, out);
            }
            // else: all intermediate points are within tolerance; drop them.
        }

        let end = pts.len() - 1;
        let mut result = Vec::with_capacity(pts.len());
        result.push(pts[0]);
        dp_recursive(pts, 0, end, epsilon_sq, &mut result);
        result.push(pts[end]);
        result
    }

    /// DP epsilon in i64 coordinate units (1 unit = 100 nm).
    /// 115 units = 0.0115 mm — collapses near-collinear dense arc tessellation.
    const EPSILON_OFFSET_UNITS: i64 = 115;

    fn to_i32(v: i64) -> Option<i32> {
        i32::try_from(v).ok()
    }

    /// Minimum squared distance from f64 point `(px, py)` (in VD float units)
    /// to a segment defined by two `BvLine<i32>` endpoints, returned as f64 squared.
    fn dist_sq_to_bv_segment(px: f64, py: f64, seg: &BvLine<i32>) -> f64 {
        let ax = seg.start.x as f64;
        let ay = seg.start.y as f64;
        let bx = seg.end.x as f64;
        let by = seg.end.y as f64;
        let dx = bx - ax;
        let dy = by - ay;
        let len_sq = dx * dx + dy * dy;
        if len_sq == 0.0 {
            let ex = px - ax;
            let ey = py - ay;
            return ex * ex + ey * ey;
        }
        let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        let cx = ax + t * dx;
        let cy = ay + t * dy;
        let ex = px - cx;
        let ey = py - cy;
        ex * ex + ey * ey
    }

    /// Deduplicates consecutive (and wrap-around) equal points from a polygon's point list.
    fn distinct_points(pts: &[Point2]) -> Vec<Point2> {
        let mut out: Vec<Point2> = Vec::with_capacity(pts.len());
        for &p in pts {
            if out.last().map_or(true, |last| last != &p) {
                out.push(p);
            }
        }
        // Remove last if it equals first (closed polygon dedup).
        if out.len() >= 2 && out.last() == out.first() {
            out.pop();
        }
        out
    }

    /// Returns true if the VD vertex at (vx, vy) (in raw i64/f64 diagram units)
    /// is interior to the ExPolygon: inside the contour AND outside every hole.
    fn is_interior(input: &ExPolygon, vx: f64, vy: f64) -> bool {
        // VD units are the same as our i64 units (100 nm). Convert to mm for the predicate.
        let px_mm = vx / 10_000.0;
        let py_mm = vy / 10_000.0;
        if !point_in_polygon_winding(input, px_mm, py_mm, POINT_IN_POLY_EPS_MM) {
            return false;
        }
        // Subtract holes.
        for hole in &input.holes {
            let hole_ex = ExPolygon {
                contour: hole.clone(),
                holes: vec![],
            };
            if point_in_polygon_winding(&hole_ex, px_mm, py_mm, POINT_IN_POLY_EPS_MM) {
                return false;
            }
        }
        true
    }

    /// Returns true if the integer-unit point `p` lies on (or very close to) any
    /// contour edge of `expoly` (contour only; holes not checked).
    /// Uses squared-distance threshold `SCALED_EPSILON_SQ`.
    fn point_on_contour(p: Point2, expoly: &ExPolygon) -> bool {
        let pts = &expoly.contour.points;
        let n = pts.len();
        for i in 0..n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            let dsq = crate::geometry::point_to_segment_distance_squared(p, a, b);
            if dsq < SCALED_EPSILON_SQ {
                return true;
            }
        }
        false
    }

    /// Segment–segment intersection (p1,p2) with (p3,p4) in f64 integer units.
    /// Returns the intersection point if it lies strictly inside both segments (u in [0,1]).
    fn seg_seg_intersect(
        p1x: f64,
        p1y: f64,
        p2x: f64,
        p2y: f64,
        p3x: f64,
        p3y: f64,
        p4x: f64,
        p4y: f64,
    ) -> Option<[f64; 2]> {
        let d1x = p2x - p1x;
        let d1y = p2y - p1y;
        let d2x = p4x - p3x;
        let d2y = p4y - p3y;

        let denom = d1x * d2y - d1y * d2x;
        if denom.abs() < 1e-10 {
            return None;
        }
        let t = ((p3x - p1x) * d2y - (p3y - p1y) * d2x) / denom;
        let u = ((p3x - p1x) * d1y - (p3y - p1y) * d1x) / denom;

        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            Some([p1x + t * d1x, p1y + t * d1y])
        } else {
            None
        }
    }

    /// Find the FIRST contour-edge intersection (in vertex order) of the segment
    /// `(ax,ay)→(bx,by)` against the contour of `expoly` (integer units).
    /// Returns the intersection point as `[f64; 2]` (integer units) or `None`.
    fn first_contour_intersection(
        ax: f64,
        ay: f64,
        bx: f64,
        by: f64,
        expoly: &ExPolygon,
    ) -> Option<[f64; 2]> {
        let pts = &expoly.contour.points;
        let n = pts.len();
        for i in 0..n {
            let p3x = pts[i].x as f64;
            let p3y = pts[i].y as f64;
            let p4x = pts[(i + 1) % n].x as f64;
            let p4y = pts[(i + 1) % n].y as f64;
            if let Some(hit) = seg_seg_intersect(ax, ay, bx, by, p3x, p3y, p4x, p4y) {
                return Some(hit);
            }
        }
        None
    }

    /// Merge-collinear-overlapping pre-pass (simple version for medial axis).
    ///
    /// The medial axis input segments come from polygon edges, which in a
    /// well-formed polygon shouldn't overlap, but holes can share collinear
    /// edges with the contour. This pass ensures no two segments overlap on
    /// the same infinite line, preventing the boostvoronoi panic.
    fn merge_collinear_overlapping_simple(segments: Vec<BvLine<i32>>) -> Vec<BvLine<i32>> {
        use std::collections::HashMap;

        fn gcd(a: i64, b: i64) -> i64 {
            let (mut a, mut b) = (a.abs(), b.abs());
            while b != 0 {
                let t = a % b;
                a = b;
                b = t;
            }
            a
        }

        fn canon_dir(dx: i64, dy: i64) -> (i64, i64) {
            let g = gcd(dx, dy).max(1);
            let (mut cdx, mut cdy) = (dx / g, dy / g);
            if cdx < 0 || (cdx == 0 && cdy < 0) {
                cdx = -cdx;
                cdy = -cdy;
            }
            (cdx, cdy)
        }

        // Bucket: canonical line key -> [(t0, t1, p0, p1)]
        let mut buckets: HashMap<((i64, i64), i64), Vec<(i64, i64, (i32, i32), (i32, i32))>> =
            HashMap::new();

        for seg in &segments {
            let (sx, sy) = (seg.start.x as i64, seg.start.y as i64);
            let (ex, ey) = (seg.end.x as i64, seg.end.y as i64);
            let dx = ex - sx;
            let dy = ey - sy;
            let (cdx, cdy) = canon_dir(dx, dy);
            let offset = cdx * sy - cdy * sx;
            let t_s = cdx * sx + cdy * sy;
            let t_e = cdx * ex + cdy * ey;
            let (t0, t1, p0, p1) = if t_s <= t_e {
                (t_s, t_e, (seg.start.x, seg.start.y), (seg.end.x, seg.end.y))
            } else {
                (t_e, t_s, (seg.end.x, seg.end.y), (seg.start.x, seg.start.y))
            };
            buckets
                .entry(((cdx, cdy), offset))
                .or_default()
                .push((t0, t1, p0, p1));
        }

        let mut out: Vec<BvLine<i32>> = Vec::with_capacity(segments.len());

        for ((cdx, cdy), _offset) in buckets.keys().cloned().collect::<Vec<_>>() {
            let mut segs = buckets.remove(&((cdx, cdy), _offset)).unwrap();
            if segs.is_empty() {
                continue;
            }
            // Sort by t0, then t1.
            segs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

            let denom = (cdx * cdx + cdy * cdy).max(1);
            let (anchor_p, anchor_t) = ((segs[0].2 .0 as i64, segs[0].2 .1 as i64), segs[0].0);
            let pos_at = |t: i64| -> (i32, i32) {
                let delta = t - anchor_t;
                let x = anchor_p.0 + (delta * cdx) / denom;
                let y = anchor_p.1 + (delta * cdy) / denom;
                (x as i32, y as i32)
            };

            // Sweep: union overlapping intervals, emit non-overlapping segments.
            let mut cur_t0 = segs[0].0;
            let mut cur_t1 = segs[0].1;

            for &(t0, t1, _, _) in &segs[1..] {
                if t0 <= cur_t1 {
                    // Overlapping or touching: extend.
                    if t1 > cur_t1 {
                        cur_t1 = t1;
                    }
                } else {
                    // Gap: emit current merged segment.
                    let p0 = pos_at(cur_t0);
                    let p1 = pos_at(cur_t1);
                    if p0 != p1 {
                        out.push(BvLine::new(
                            BvPoint { x: p0.0, y: p0.1 },
                            BvPoint { x: p1.0, y: p1.1 },
                        ));
                    }
                    cur_t0 = t0;
                    cur_t1 = t1;
                }
            }
            // Emit the last merged segment.
            let p0 = pos_at(cur_t0);
            let p1 = pos_at(cur_t1);
            if p0 != p1 {
                out.push(BvLine::new(
                    BvPoint { x: p0.0, y: p0.1 },
                    BvPoint { x: p1.0, y: p1.1 },
                ));
            }
        }

        out
    }

    /// Build a flat segment list from a polygon ring (contour or hole).
    fn ring_to_segments(pts: &[Point2]) -> Vec<((i64, i64), (i64, i64))> {
        let n = pts.len();
        (0..n)
            .filter_map(|i| {
                let a = pts[i];
                let b = pts[(i + 1) % n];
                if a == b {
                    None
                } else {
                    Some(((a.x, a.y), (b.x, b.y)))
                }
            })
            .collect()
    }

    /// Compute the width at a VD vertex given its distance to its generating site.
    /// Returns width in diagram units (same as i64 units = 100 nm).
    fn vertex_width(
        vx: f64,
        vy: f64,
        cell_seg: Option<&BvLine<i32>>,
        cell_pt: Option<[f64; 2]>,
    ) -> f64 {
        if let Some(seg) = cell_seg {
            2.0 * dist_sq_to_bv_segment(vx, vy, seg).sqrt()
        } else if let Some([px, py]) = cell_pt {
            let dx = vx - px;
            let dy = vy - py;
            2.0 * (dx * dx + dy * dy).sqrt()
        } else {
            0.0
        }
    }

    /// Retrieve the generating point for a point-type cell (SegmentStart/SegmentEnd/SinglePoint).
    fn retrieve_point_for_cell(
        cell: &boostvoronoi::diagram::Cell,
        bv_segs: &[BvLine<i32>],
    ) -> Option<[f64; 2]> {
        let si = cell.source_index().usize();
        let seg = bv_segs.get(si)?;
        match cell.source_category() {
            SourceCategory::SegmentStart => Some([seg.start.x as f64, seg.start.y as f64]),
            SourceCategory::SegmentEnd => Some([seg.end.x as f64, seg.end.y as f64]),
            SourceCategory::SinglePoint => Some([seg.start.x as f64, seg.start.y as f64]),
            _ => None,
        }
    }

    /// Compute medial axis of `input` via Voronoi diagram.
    ///
    /// - `min_width` / `max_width`: filter thresholds in **millimeters**.
    /// - Returns `Err(MedialAxisError::DegenerateInput)` for < 3 distinct contour points.
    /// - Returns `Ok(vec![])` for valid shapes with no surviving interior edges.
    pub fn medial_axis(
        input: &ExPolygon,
        min_width: f32,
        max_width: f32,
    ) -> Result<Vec<ThickPolyline>, MedialAxisError> {
        // 1. Validate input: need ≥ 3 distinct contour points.
        let contour_pts = distinct_points(&input.contour.points);
        if contour_pts.len() < 3 {
            return Err(MedialAxisError::DegenerateInput);
        }

        // 1b. Coordinate overflow guard: boostvoronoi requires all coordinates to fit
        // in i32.  Explicit bound comparison (no abs(): abs() panics on i64::MIN and
        // would false-reject the valid i32::MIN).  i32::MIN and i32::MAX pass through;
        // anything outside [i32::MIN, i32::MAX] returns a typed error.
        {
            const I32_MIN_AS_I64: i64 = i32::MIN as i64;
            const I32_MAX_AS_I64: i64 = i32::MAX as i64;
            let all_pts = input
                .contour
                .points
                .iter()
                .chain(input.holes.iter().flat_map(|h| h.points.iter()));
            for p in all_pts {
                let x_oob = p.x < I32_MIN_AS_I64 || p.x > I32_MAX_AS_I64;
                let y_oob = p.y < I32_MIN_AS_I64 || p.y > I32_MAX_AS_I64;
                if x_oob || y_oob {
                    // Report the first out-of-range coordinate value (no abs needed).
                    let actual_max = if x_oob { p.x } else { p.y };
                    return Err(MedialAxisError::CoordinateOverflow {
                        actual_max,
                        i32_max: I32_MAX_AS_I64,
                    });
                }
            }
        }

        // 1c. Douglas-Peucker decimation: collapse near-collinear dense tessellation
        // (e.g. finely-tessellated arc segments) so the VD does not produce a
        // junction-web.  Epsilon = 115 units = 0.0115 mm.
        let epsilon_sq = (EPSILON_OFFSET_UNITS as f64).powi(2);

        let contour_pts = {
            let simplified = douglas_peucker(&contour_pts, epsilon_sq);
            // Safety: DP always keeps endpoints; a 3-point ring stays ≥ 3.
            // If somehow DP collapses below 3, fall back to the pre-DP distinct points.
            if simplified.len() >= 3 {
                simplified
            } else {
                contour_pts
            }
        };

        // Collect hole rings after DP.
        let hole_pts_list: Vec<Vec<Point2>> = input
            .holes
            .iter()
            .filter_map(|hole| {
                let dpts = distinct_points(&hole.points);
                if dpts.len() < 3 {
                    return None;
                }
                let simplified = douglas_peucker(&dpts, epsilon_sq);
                if simplified.len() >= 3 {
                    Some(simplified)
                } else {
                    Some(dpts)
                }
            })
            .collect();

        // 2. Build flat segment list: contour first, then holes.
        let mut raw_segments: Vec<((i64, i64), (i64, i64))> = Vec::new();
        raw_segments.extend(ring_to_segments(&contour_pts));
        for hole_pts in &hole_pts_list {
            raw_segments.extend(ring_to_segments(hole_pts));
        }

        // Convert to BvLine<i32>, dropping any segment that overflows i32.
        let mut bv_segs_raw: Vec<BvLine<i32>> = Vec::with_capacity(raw_segments.len());
        for ((sx, sy), (ex, ey)) in &raw_segments {
            let (Some(sx32), Some(sy32), Some(ex32), Some(ey32)) =
                (to_i32(*sx), to_i32(*sy), to_i32(*ex), to_i32(*ey))
            else {
                continue;
            };
            if sx32 == ex32 && sy32 == ey32 {
                continue;
            }
            bv_segs_raw.push(BvLine::new(
                BvPoint { x: sx32, y: sy32 },
                BvPoint { x: ex32, y: ey32 },
            ));
        }

        if bv_segs_raw.is_empty() {
            return Ok(vec![]);
        }

        // 3. Collinear-overlap merge pre-pass.
        let bv_segs = merge_collinear_overlapping_simple(bv_segs_raw);

        if bv_segs.is_empty() {
            return Ok(vec![]);
        }

        // 4. Build the Voronoi diagram.
        // boostvoronoi 0.12.1 emits a bounded VD (no semi-infinite edges for
        // segment sites). All primary edges have two finite endpoints.
        let diagram = match Builder::<i32>::default()
            .with_segments(bv_segs.iter())
            .and_then(|b| b.build())
        {
            Ok(d) => d,
            Err(_) => return Ok(vec![]),
        };

        // 5. Collect surviving interior edges.
        // An edge survives if: primary, both endpoints finite, at least one endpoint interior.
        // For curved edges: discretize via VoronoiVisualUtils::discretize.
        // For linear edges: use the two vertex positions directly.
        //
        // width at each vertex = 2 × distance to its generating site.
        // Width filter (OR-gate): keep iff (w0 >= min_width || w1 >= min_width) && (w0 <= max_width || w1 <= max_width).

        let min_width_units = (min_width as f64) * 10_000.0;
        let max_width_units = (max_width as f64) * 10_000.0;

        // Each surviving edge is stored as (Vec<[f64;2]> points, Vec<f64> widths_units).
        // Points are in diagram f64 units.
        struct SurvivingEdge {
            pts: Vec<[f64; 2]>,
            widths: Vec<f64>,
            v0_id: usize,
            v1_id: usize,
        }

        let mut surviving: Vec<SurvivingEdge> = Vec::new();

        // We need to avoid processing both an edge and its twin.
        // boostvoronoi: for primary edges, one of the pair is primary; iterate all and
        // check is_primary(), skip twin.
        let mut seen_edge_pairs: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();

        for edge in diagram.edges().iter() {
            if !edge.is_primary() {
                continue;
            }

            let twin_idx = match diagram.edge_get_twin(edge.id()) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let twin_edge = match diagram.edge(twin_idx) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let v0_opt = edge.vertex0();
            let v1_opt = twin_edge.vertex0();

            // Skip semi-infinite edges (one endpoint missing).
            // boostvoronoi 0.12.1 with segment sites emits bounded VDs, so these
            // should not appear in practice; drop them rather than fabricating a clip.
            if v0_opt.is_none() || v1_opt.is_none() {
                continue;
            }

            let v0_vi = v0_opt.unwrap();
            let v1_vi = v1_opt.unwrap();

            let v0_id = v0_vi.usize();
            let v1_id = v1_vi.usize();

            // Dedup by canonical pair.
            let pair = if v0_id <= v1_id {
                (v0_id, v1_id)
            } else {
                (v1_id, v0_id)
            };
            if !seen_edge_pairs.insert(pair) {
                continue;
            }

            let v0 = match diagram.vertices().get(v0_id) {
                Some(v) => v,
                None => continue,
            };
            let v1 = match diagram.vertices().get(v1_id) {
                Some(v) => v,
                None => continue,
            };

            let (vx0, vy0) = (v0.x(), v0.y());
            let (vx1, vy1) = (v1.x(), v1.y());

            // Interior filter: at least one endpoint must be interior.
            let in0 = is_interior(input, vx0, vy0);
            let in1 = is_interior(input, vx1, vy1);
            if !in0 && !in1 {
                continue;
            }

            // Resolve cells for width computation.
            let cell0_id = match diagram.edge_get_cell(edge.id()) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let cell0 = match diagram.cell(cell0_id) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Determine site for width computation.
            // Prefer segment cell for accuracy; fall back to point cell.
            let seg_for_width: Option<&BvLine<i32>>;
            let pt_for_width: Option<[f64; 2]>;

            if cell0.contains_segment() {
                let si = cell0.source_index().usize();
                seg_for_width = bv_segs.get(si);
                pt_for_width = None;
            } else {
                seg_for_width = None;
                pt_for_width = retrieve_point_for_cell(cell0, &bv_segs);
            }

            let w0 = vertex_width(vx0, vy0, seg_for_width, pt_for_width);
            let w1 = vertex_width(vx1, vy1, seg_for_width, pt_for_width);

            // OR-gate width filter.
            if !((w0 >= min_width_units || w1 >= min_width_units)
                && (w0 <= max_width_units || w1 <= max_width_units))
            {
                continue;
            }

            // Build the point list for this edge.
            let pts: Vec<[f64; 2]>;
            let widths: Vec<f64>;

            if edge.is_curved() {
                // Curved edge: discretize the parabola.
                // Identify which cell is point and which is segment.
                let cell1_id = match diagram.edge_get_cell(twin_idx) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let cell1 = match diagram.cell(cell1_id) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let (point_focus, segment_directrix): (BvPoint<i32>, BvLine<i32>) =
                    if cell0.contains_point() && cell1.contains_segment() {
                        let pt = retrieve_point_for_cell(cell0, &bv_segs).unwrap_or([vx0, vy0]);
                        let si = cell1.source_index().usize();
                        let seg = bv_segs.get(si).cloned().unwrap_or(BvLine::new(
                            BvPoint {
                                x: vx0 as i32,
                                y: vy0 as i32,
                            },
                            BvPoint {
                                x: vx1 as i32,
                                y: vy1 as i32,
                            },
                        ));
                        (
                            BvPoint {
                                x: pt[0] as i32,
                                y: pt[1] as i32,
                            },
                            seg,
                        )
                    } else if cell1.contains_point() && cell0.contains_segment() {
                        let pt = retrieve_point_for_cell(cell1, &bv_segs).unwrap_or([vx1, vy1]);
                        let si = cell0.source_index().usize();
                        let seg = bv_segs.get(si).cloned().unwrap_or(BvLine::new(
                            BvPoint {
                                x: vx0 as i32,
                                y: vy0 as i32,
                            },
                            BvPoint {
                                x: vx1 as i32,
                                y: vy1 as i32,
                            },
                        ));
                        (
                            BvPoint {
                                x: pt[0] as i32,
                                y: pt[1] as i32,
                            },
                            seg,
                        )
                    } else {
                        // Fallback: treat as linear.
                        let linear_pts = vec![[vx0, vy0], [vx1, vy1]];
                        let linear_widths = vec![w0, w1];
                        pts = linear_pts;
                        widths = linear_widths;
                        surviving.push(SurvivingEdge {
                            pts,
                            widths,
                            v0_id,
                            v1_id,
                        });
                        continue;
                    };

                // Pre-fill with the two f64 endpoints, then let discretize refine.
                let max_dist = 0.1 * 10_000.0; // 0.1 mm in units
                let affine = SimpleAffine::default();
                let mut disc: Vec<[f64; 2]> = vec![[vx0, vy0], [vx1, vy1]];
                VoronoiVisualUtils::discretize::<i32>(
                    &point_focus,
                    &segment_directrix,
                    max_dist,
                    &affine,
                    &mut disc,
                );
                // Compute widths at each discretized point using the segment directrix.
                let disc_widths: Vec<f64> = disc
                    .iter()
                    .map(|&[px, py]| 2.0 * dist_sq_to_bv_segment(px, py, &segment_directrix).sqrt())
                    .collect();
                pts = disc;
                widths = disc_widths;
            } else {
                // Linear edge.
                pts = vec![[vx0, vy0], [vx1, vy1]];
                widths = vec![w0, w1];
            }

            surviving.push(SurvivingEdge {
                pts,
                widths,
                v0_id,
                v1_id,
            });
        }

        if surviving.is_empty() {
            return Ok(vec![]);
        }

        // 6. Stitch edges into ThickPolylines via medial-graph traversal.
        //
        // Strategy (three-pass, topology-aware):
        //   a) Build adjacency: vertex_id -> list of incident kept-edge indices.
        //   b) Classify vertices by TOTAL degree (fixed, never changes):
        //        leaf     = degree 1
        //        through  = degree 2
        //        junction = degree ≥ 3
        //   c) Walk chains using TOTAL degree to decide when to stop:
        //      - Pass A: start at every LEAF; walk through THROUGH-nodes only
        //        (stop immediately if next node is a LEAF or JUNCTION).
        //        This correctly emits each spur as its own short polyline.
        //      - Pass B: start at every JUNCTION for each still-unused incident
        //        edge; walk through THROUGH-nodes until next node is a LEAF or
        //        JUNCTION.  This emits each branch off a junction and the
        //        junction-to-junction spines.
        //      - Pass C: remaining unused edges form pure cycles (every node is
        //        a THROUGH-node, no leaf/junction).  Pick any unused edge, walk
        //        the full cycle back to start vertex, emit as closed polyline.
        //        This is what captures circular / loop-only components such as
        //        the curved_boundary and nested_hole fixtures.
        //      Every kept edge ends up in exactly one emitted polyline.

        // Build adjacency: vertex_id -> list of edge indices (total, not just unused).
        let mut adj: std::collections::HashMap<usize, Vec<usize>> =
            std::collections::HashMap::new();
        for (ei, se) in surviving.iter().enumerate() {
            adj.entry(se.v0_id).or_default().push(ei);
            adj.entry(se.v1_id).or_default().push(ei);
        }

        // Total degree per vertex (fixed; does NOT change as edges are consumed).
        let mut degree: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for se in &surviving {
            *degree.entry(se.v0_id).or_insert(0) += 1;
            *degree.entry(se.v1_id).or_insert(0) += 1;
        }

        let mut used = vec![false; surviving.len()];

        /// Append the points of one kept edge into `chain_pts`, oriented so that
        /// `from_vertex` is the entry end.  If the chain is already non-empty the
        /// shared junction/leaf vertex is already in `chain_pts`, so we skip index 0.
        fn append_edge(
            chain_pts: &mut Vec<Point2WithWidth>,
            se_pts: &[[f64; 2]],
            se_widths: &[f64],
            from_vertex_is_v0: bool,
        ) {
            let skip_first = !chain_pts.is_empty();
            let start_i = if skip_first { 1 } else { 0 };
            if from_vertex_is_v0 {
                for i in start_i..se_pts.len() {
                    let [px, py] = se_pts[i];
                    chain_pts.push(Point2WithWidth {
                        x: (px / 10_000.0) as f32,
                        y: (py / 10_000.0) as f32,
                        width: (se_widths[i] / 10_000.0) as f32,
                    });
                }
            } else {
                // Reversed orientation.
                let n = se_pts.len();
                let end_i = if skip_first { n - 1 } else { n };
                for i in (0..end_i).rev() {
                    let [px, py] = se_pts[i];
                    chain_pts.push(Point2WithWidth {
                        x: (px / 10_000.0) as f32,
                        y: (py / 10_000.0) as f32,
                        width: (se_widths[i] / 10_000.0) as f32,
                    });
                }
            }
        }

        // Walk a chain starting from `start_vertex` along `start_edge`.
        // Stops when the next vertex is a LEAF or JUNCTION (total degree ≠ 2),
        // or when it has no unused outgoing edge.
        // `cycle_start_vertex`: if Some(v), also stop if we reach v (cycle detection).
        // Returns the end vertex id.
        fn walk_chain(
            start_vertex: usize,
            start_edge: usize,
            cycle_start_vertex: Option<usize>,
            surviving: &[SurvivingEdge],
            adj: &std::collections::HashMap<usize, Vec<usize>>,
            degree: &std::collections::HashMap<usize, usize>,
            used: &mut Vec<bool>,
            chain_pts: &mut Vec<Point2WithWidth>,
        ) -> usize {
            let mut cur_vertex = start_vertex;
            let mut cur_edge = start_edge;

            loop {
                used[cur_edge] = true;

                let se = &surviving[cur_edge];
                let forward = se.v0_id == cur_vertex;
                append_edge(chain_pts, &se.pts, &se.widths, forward);

                let next_vertex = if forward { se.v1_id } else { se.v0_id };

                // Stop at cycle start (for Pass C: closed loop).
                if cycle_start_vertex == Some(next_vertex) {
                    return next_vertex;
                }

                // Stop if next_vertex is a LEAF or JUNCTION (not a through-node).
                let next_deg = *degree.get(&next_vertex).unwrap_or(&0);
                if next_deg != 2 {
                    return next_vertex;
                }

                // next_vertex is a through-node: find the one unused continuation edge.
                let next_unused: Option<usize> = adj
                    .get(&next_vertex)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[])
                    .iter()
                    .copied()
                    .find(|&ei| !used[ei]);

                match next_unused {
                    Some(ei) => {
                        cur_vertex = next_vertex;
                        cur_edge = ei;
                    }
                    None => {
                        // Through-node with no unused edges left (already consumed from
                        // the other side) — stop here.
                        return next_vertex;
                    }
                }
            }
        }

        // Each emitted polyline tracks its start and end graph vertex IDs so
        // that the post-stitch trim step can query their degrees.
        struct EmittedPolyline {
            points: Vec<Point2WithWidth>,
            start_vertex: usize,
            end_vertex: usize,
        }

        let mut emitted: Vec<EmittedPolyline> = Vec::new();

        // ── Pass A: chains starting from LEAF vertices (degree == 1). ──────────
        for start_edge in 0..surviving.len() {
            if used[start_edge] {
                continue;
            }
            let se0 = &surviving[start_edge];
            let d0 = *degree.get(&se0.v0_id).unwrap_or(&0);
            let d1 = *degree.get(&se0.v1_id).unwrap_or(&0);

            let start_vertex = if d0 == 1 {
                se0.v0_id
            } else if d1 == 1 {
                se0.v1_id
            } else {
                continue; // no leaf endpoint — defer to Pass B / C
            };

            let mut chain_pts: Vec<Point2WithWidth> = Vec::new();
            let end_vertex = walk_chain(
                start_vertex,
                start_edge,
                None,
                &surviving,
                &adj,
                &degree,
                &mut used,
                &mut chain_pts,
            );

            if chain_pts.len() >= 2 {
                emitted.push(EmittedPolyline {
                    points: chain_pts,
                    start_vertex,
                    end_vertex,
                });
            }
        }

        // ── Pass B: branches from JUNCTION vertices (degree ≥ 3). ─────────────
        for start_edge in 0..surviving.len() {
            if used[start_edge] {
                continue;
            }
            let se0 = &surviving[start_edge];
            let d0 = *degree.get(&se0.v0_id).unwrap_or(&0);
            let d1 = *degree.get(&se0.v1_id).unwrap_or(&0);

            let start_vertex = if d0 >= 3 {
                se0.v0_id
            } else if d1 >= 3 {
                se0.v1_id
            } else {
                continue;
            };

            let mut chain_pts: Vec<Point2WithWidth> = Vec::new();
            let end_vertex = walk_chain(
                start_vertex,
                start_edge,
                None,
                &surviving,
                &adj,
                &degree,
                &mut used,
                &mut chain_pts,
            );

            if chain_pts.len() >= 2 {
                emitted.push(EmittedPolyline {
                    points: chain_pts,
                    start_vertex,
                    end_vertex,
                });
            }
        }

        // ── Pass C: pure cycles (all nodes degree 2, no leaf / junction). ──────
        for start_edge in 0..surviving.len() {
            if used[start_edge] {
                continue;
            }
            let se0 = &surviving[start_edge];
            let start_vertex = se0.v0_id;

            let mut chain_pts: Vec<Point2WithWidth> = Vec::new();
            let end_vertex = walk_chain(
                start_vertex,
                start_edge,
                Some(start_vertex),
                &surviving,
                &adj,
                &degree,
                &mut used,
                &mut chain_pts,
            );

            if chain_pts.len() >= 2 {
                emitted.push(EmittedPolyline {
                    points: chain_pts,
                    start_vertex,
                    end_vertex,
                });
            }
        }

        // 7. Post-stitch processing: extend → remove → reconnect.
        //
        // Faithful port of OrcaSlicer ExPolygon::medial_axis post-processing
        // (ExPolygon.cpp:452-558).
        //
        // An endpoint is OPEN iff its graph vertex has degree == 1 (leaf).
        // Junction ends (degree ≥ 3) and cycle polylines are NOT open.
        //
        // Step 1: compute max_w = global maximum width across ALL emitted points.
        // Step 2: for each polyline, attempt to extend open ends to the contour.
        // Step 3: remove any open-ended polyline whose length < max_w * 2.
        // Step 4: greedy endpoint-equality reconnection of survivors.

        // --- Step 1: global max width (in mm, f32). ---
        let max_w_mm: f32 = emitted
            .iter()
            .flat_map(|pl| pl.points.iter().map(|p| p.width))
            .fold(0.0_f32, f32::max);

        // EP.cpp parameters in integer units.
        let max_width_param_units = (max_width as f64) * 10_000.0;

        // --- Step 2 + 3: per-polyline extend and remove. ---

        // Build working list with open_front / open_back flags and vertex keys.
        struct WorkPolyline {
            points: Vec<Point2WithWidth>,
            open_front: bool,
            open_back: bool,
        }

        let mut work: Vec<WorkPolyline> = emitted
            .into_iter()
            .map(|ep| {
                let d_start = *degree.get(&ep.start_vertex).unwrap_or(&0);
                let d_end = *degree.get(&ep.end_vertex).unwrap_or(&0);
                // An endpoint is open iff degree == 1 (leaf in the medial graph).
                let open_front = d_start == 1;
                let open_back = d_end == 1;
                WorkPolyline {
                    points: ep.points,
                    open_front,
                    open_back,
                }
            })
            .collect();

        // --- Step 2: extend open ends toward the contour boundary. ---
        for pl in work.iter_mut() {
            let n = pl.points.len();

            // Extend front (open_front and not on contour boundary).
            if pl.open_front {
                let front = Point2 {
                    x: (pl.points[0].x as f64 * 10_000.0) as i64,
                    y: (pl.points[0].y as f64 * 10_000.0) as i64,
                };
                if !point_on_contour(front, input) {
                    // p1 = points[0], p2 = points[1] (or midpoint if 2 pts).
                    let (p1x, p1y) = (
                        pl.points[0].x as f64 * 10_000.0,
                        pl.points[0].y as f64 * 10_000.0,
                    );
                    let (p2x, p2y) = if n >= 2 {
                        (
                            pl.points[1].x as f64 * 10_000.0,
                            pl.points[1].y as f64 * 10_000.0,
                        )
                    } else {
                        (p1x, p1y) // degenerate; won't extend
                    };
                    let (p2x, p2y) = if n == 2 {
                        ((p1x + p2x) * 0.5, (p1y + p2y) * 0.5)
                    } else {
                        (p2x, p2y)
                    };
                    // Direction from p2 toward p1 (outward).
                    let dx = p1x - p2x;
                    let dy = p1y - p2y;
                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 1e-10 {
                        let ndx = dx / len;
                        let ndy = dy / len;
                        // ray_start = p1 + dir * max_width_param
                        let rsx = p1x + ndx * max_width_param_units;
                        let rsy = p1y + ndy * max_width_param_units;
                        // Intersect segment (ray_start → p1) with contour.
                        if let Some([hx, hy]) =
                            first_contour_intersection(rsx, rsy, p1x, p1y, input)
                        {
                            pl.points[0].x = (hx / 10_000.0) as f32;
                            pl.points[0].y = (hy / 10_000.0) as f32;
                        }
                    }
                }
            }

            // Extend back (open_back and not on contour boundary).
            if pl.open_back {
                let n = pl.points.len();
                let back = Point2 {
                    x: (pl.points[n - 1].x as f64 * 10_000.0) as i64,
                    y: (pl.points[n - 1].y as f64 * 10_000.0) as i64,
                };
                if !point_on_contour(back, input) {
                    let (p2x, p2y) = (
                        pl.points[n - 1].x as f64 * 10_000.0,
                        pl.points[n - 1].y as f64 * 10_000.0,
                    );
                    let (p1x, p1y) = if n >= 2 {
                        (
                            pl.points[n - 2].x as f64 * 10_000.0,
                            pl.points[n - 2].y as f64 * 10_000.0,
                        )
                    } else {
                        (p2x, p2y)
                    };
                    let (p1x, p1y) = if n == 2 {
                        ((p1x + p2x) * 0.5, (p1y + p2y) * 0.5)
                    } else {
                        (p1x, p1y)
                    };
                    // Direction from p1 toward p2 (outward).
                    let dx = p2x - p1x;
                    let dy = p2y - p1y;
                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 1e-10 {
                        let ndx = dx / len;
                        let ndy = dy / len;
                        // ray_end = p2 + dir * max_width_param
                        let rex = p2x + ndx * max_width_param_units;
                        let rey = p2y + ndy * max_width_param_units;
                        // Intersect segment (p2 → ray_end) with contour.
                        if let Some([hx, hy]) =
                            first_contour_intersection(p2x, p2y, rex, rey, input)
                        {
                            let last = pl.points.len() - 1;
                            pl.points[last].x = (hx / 10_000.0) as f32;
                            pl.points[last].y = (hy / 10_000.0) as f32;
                        }
                    }
                }
            }
        }

        // --- Step 3: remove short open-ended polylines (EP.cpp ExPolygon.cpp:513-518). ---
        // A polyline with at least one OPEN endpoint (a degree-1 leaf end) AND
        // length < 2*max_w is discarded.  Closed loops and polylines whose both
        // ends are junctions/non-open are kept.  `length_mm` and `max_w_mm` are
        // both in millimetres (Point2WithWidth fields are mm), so the threshold
        // comparison is unit-consistent — no mm<->unit (x10000) crossing here.
        work.retain(|pl| {
            if !(pl.open_front || pl.open_back) {
                return true;
            }
            let length_mm: f32 = pl
                .points
                .windows(2)
                .map(|w| {
                    let dx = w[1].x - w[0].x;
                    let dy = w[1].y - w[0].y;
                    (dx * dx + dy * dy).sqrt()
                })
                .sum();
            length_mm >= max_w_mm * 2.0
        });

        // --- Step 4: greedy endpoint-equality reconnection. ---
        // Only runs if any removal happened (work.len() < original count after step 3).
        // Exact match on (x,y) as f32 bit-exact equality (no epsilon, per EP.cpp).
        //
        // Cases (i < j, skip if both ends open on j):
        //   last(i)==last(j) → reverse j, append j[1..] to i; open_back = j.open_front
        //   first(i)==last(j) → reverse i, reverse j (i becomes j reversed; j becomes old i reversed)
        //                        → actually: swap so last(new_i)==last(old_j) then do case above?
        //                        EP.cpp does: reverse i; then first(i)==first(j) path below.
        //                        Simpler: reverse i in place, then last(i)==last(j): reverse j, append.
        //   first(i)==first(j) → reverse i, then append j[1..]; open_front = j.open_back...
        //                         wait, EP.cpp: reverse_i then first(i)==last(j) path
        //                         Let me follow EP.cpp logic directly:
        //                         first(i)==first(j): prepend reverse of j[0..n-1] to i
        //                                              → equivalent to: reverse i; last(i)==first(j); append j[1..]
        //   last(i)==first(j) → append j[1..] to i; open_back = j.open_back
        //
        // After join: erase j, restart j-scan from beginning.

        fn pts_eq(a: &Point2WithWidth, b: &Point2WithWidth) -> bool {
            a.x == b.x && a.y == b.y
        }

        let mut did_reconnect = true;
        while did_reconnect {
            did_reconnect = false;
            let mut i = 0;
            'outer: while i < work.len() {
                let mut j = i + 1;
                while j < work.len() {
                    // Skip j if both its ends are open (EP.cpp: skip if both-open).
                    if work[j].open_front && work[j].open_back {
                        j += 1;
                        continue;
                    }

                    let last_i = work[i].points.len() - 1;
                    let last_j = work[j].points.len() - 1;

                    // Determine which join case applies (index-based, no long-lived borrow).
                    enum JoinCase {
                        LastLast,
                        FirstLast,
                        FirstFirst,
                        LastFirst,
                    }
                    let case = if pts_eq(&work[i].points[last_i], &work[j].points[last_j]) {
                        JoinCase::LastLast
                    } else if pts_eq(&work[i].points[0], &work[j].points[last_j]) {
                        JoinCase::FirstLast
                    } else if pts_eq(&work[i].points[0], &work[j].points[0]) {
                        JoinCase::FirstFirst
                    } else if pts_eq(&work[i].points[last_i], &work[j].points[0]) {
                        JoinCase::LastFirst
                    } else {
                        j += 1;
                        continue;
                    };

                    // Extract j's data before removing it.
                    let j_pts = work[j].points.clone();
                    let j_open_front = work[j].open_front;
                    let j_open_back = work[j].open_back;
                    work.remove(j);

                    match case {
                        JoinCase::LastLast => {
                            // reverse j, append j[1..] to i; open_back = j.open_front (after rev)
                            let mut rev_j = j_pts;
                            rev_j.reverse();
                            work[i].points.extend_from_slice(&rev_j[1..]);
                            work[i].open_back = j_open_front;
                        }
                        JoinCase::FirstLast => {
                            // reverse i; reverse j; append j[1..] to i; open_back = j.open_front
                            work[i].points.reverse();
                            // swap open_front and open_back after reversing
                            let (of, ob) = (work[i].open_back, work[i].open_front);
                            work[i].open_front = of;
                            work[i].open_back = ob;
                            let mut rev_j = j_pts;
                            rev_j.reverse();
                            work[i].points.extend_from_slice(&rev_j[1..]);
                            work[i].open_back = j_open_front;
                        }
                        JoinCase::FirstFirst => {
                            // reverse i; append j[1..] to i; open_back = j.open_back
                            work[i].points.reverse();
                            // swap open_front and open_back after reversing
                            let (of, ob) = (work[i].open_back, work[i].open_front);
                            work[i].open_front = of;
                            work[i].open_back = ob;
                            work[i].points.extend_from_slice(&j_pts[1..]);
                            work[i].open_back = j_open_back;
                        }
                        JoinCase::LastFirst => {
                            // append j[1..] to i; open_back = j.open_back
                            work[i].points.extend_from_slice(&j_pts[1..]);
                            work[i].open_back = j_open_back;
                        }
                    }

                    did_reconnect = true;
                    continue 'outer;
                }
                i += 1;
            }
        }

        // 8. Emit surviving polylines as ThickPolylines (mm).
        let result: Vec<ThickPolyline> = work
            .into_iter()
            .filter(|pl| pl.points.len() >= 2)
            .map(|pl| ThickPolyline { points: pl.points })
            .collect();

        Ok(result)
    }
}

// Public re-exports (feature-gated).
#[cfg(feature = "host-algos")]
pub use impl_::MedialAxisError;

#[cfg(feature = "host-algos")]
pub use impl_::medial_axis;

// Inline smoke tests — compiled only with host-algos feature.
#[cfg(all(test, feature = "host-algos"))]
mod smoke {
    use super::{medial_axis, MedialAxisError};
    use slicer_ir::{mm_to_units, ExPolygon, Point2, Polygon};

    fn rect_mm(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(x0, y0),
                    Point2::from_mm(x1, y0),
                    Point2::from_mm(x1, y1),
                    Point2::from_mm(x0, y1),
                ],
            },
            holes: vec![],
        }
    }

    /// (a) A 1×10 mm rectangle should yield ≥1 ThickPolyline.
    /// Central points (not near ends) should have width ≈ 1.0 mm ± 0.1 mm.
    #[test]
    fn smoke_rectangle_yields_polylines_with_correct_width() {
        let rect = rect_mm(0.0, 0.0, 10.0, 1.0);
        let result = medial_axis(&rect, 0.0, f32::MAX).expect("should not be degenerate");
        assert!(
            !result.is_empty(),
            "expected ≥1 ThickPolyline for 1×10 mm rect"
        );

        // Find the longest polyline (should be the central spine).
        let longest = result.iter().max_by_key(|pl| pl.points.len()).unwrap();

        // The spine should have ≥2 points, all at y ≈ 0.5 mm, width ≈ 1.0 mm.
        assert!(
            longest.points.len() >= 2,
            "longest polyline should have ≥2 points"
        );

        // All points on the spine should have y ≈ 0.5 mm and width ≈ 1.0 mm.
        for p in &longest.points {
            if p.width > 0.1 {
                // Skip corner spur endpoints (width≈0).
                assert!(
                    (p.y - 0.5).abs() < 0.1,
                    "spine point y={:.4} not ≈ 0.5 mm",
                    p.y
                );
                assert!(
                    (p.width - 1.0).abs() < 0.1,
                    "spine point width={:.4} not ≈ 1.0 mm",
                    p.width
                );
            }
        }
    }

    /// (b) A degenerate contour (2 distinct points) must return Err(DegenerateInput).
    #[test]
    fn smoke_degenerate_two_points_returns_error() {
        let degen = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 {
                        x: mm_to_units(5.0),
                        y: 0,
                    },
                ],
            },
            holes: vec![],
        };
        let result = medial_axis(&degen, 0.0, f32::MAX);
        assert_eq!(
            result,
            Err(MedialAxisError::DegenerateInput),
            "2-point degenerate contour must return DegenerateInput"
        );
    }
}
