// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the ModularSlicer architecture.
// -----------------------------------------------------------------------------
/// Phase 4 — post-process painted lines and colorize contours.
///
/// Phase 4a: `post_process_painted_lines` — sort, group, filter/merge painted lines per contour edge.
/// Phase 4b: `colorize_contours` — fill contour edges with ColoredLine records (painted + unpainted gaps).
use crate::algos::paint_segmentation::painted_line::PaintedLine;
use crate::algos::paint_segmentation::triangle_intersect::Line;
use slicer_ir::{PaintValue, Point2};

/// Gap threshold for merging adjacent same-(semantic,value) segments (1 µm = 10 units at 100 nm/unit).
const MERGE_GAP_THRESHOLD: i64 = 10;

/// Minimum segment length to retain (1 unit = 100 nm).
const MIN_SEGMENT_LENGTH: i64 = 1;

/// Gap threshold for emitting an unpainted gap segment in colorize pass (1 unit).
const COLORIZE_GAP_THRESHOLD: i64 = 1;

/// An ordered contour (closed polygon boundary) consisting of chained edges.
#[derive(Debug, Clone)]
pub struct Contour {
    /// Ordered contour edges; consecutive edges share endpoints (end[i] == start[i+1]).
    pub edges: Vec<Line>,
}

impl Contour {
    /// Sum of all edge lengths (Euclidean, in units).
    pub fn total_length(&self) -> i64 {
        self.edges.iter().map(line_length).sum()
    }

    /// Number of edges in this contour.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// A contour edge segment that has been assigned a paint value (or left unpainted).
#[derive(Debug, Clone, PartialEq)]
pub struct ColoredLine {
    /// The 2D line segment (sub-segment of a contour edge).
    pub line: Line,
    /// Paint value; `None` means unpainted / default extrusion.
    pub value: Option<PaintValue>,
    /// Index of the parent contour in the contour slice.
    pub poly_idx: usize,
    /// Index of the edge within the contour.
    pub local_line_idx: usize,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Euclidean length of a line (integer approximation, rounds to nearest unit).
fn line_length(l: &Line) -> i64 {
    let dx = l.end.x - l.start.x;
    let dy = l.end.y - l.start.y;
    ((dx * dx + dy * dy) as f64).sqrt().round() as i64
}

/// Squared Euclidean distance between two points.
fn sq_dist(a: Point2, b: Point2) -> i64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

// ---------------------------------------------------------------------------
// Phase 4a
// ---------------------------------------------------------------------------

/// Post-process painted lines: sort by (contour_idx, line_idx, projection distance, length),
/// group by (contour_idx, line_idx), filter/merge adjacent same-value segments, trim to edge
/// bounds, discard short segments.
///
/// Returns `Vec<Vec<PaintedLine>>` indexed by contour index.
pub fn post_process_painted_lines(
    contours: &[Contour],
    mut painted_lines: Vec<PaintedLine>,
) -> Vec<Vec<PaintedLine>> {
    if painted_lines.is_empty() || contours.is_empty() {
        return vec![Vec::new(); contours.len()];
    }

    // Sort by (contour_idx, line_idx, sq_dist from edge start, segment length).
    painted_lines.sort_by(|a, b| {
        a.contour_idx
            .cmp(&b.contour_idx)
            .then(a.line_idx.cmp(&b.line_idx))
            .then_with(|| {
                let edge_a = &contours[a.contour_idx].edges[a.line_idx];
                let edge_b = &contours[b.contour_idx].edges[b.line_idx];
                let da = sq_dist(a.projected_line.start, edge_a.start);
                let db = sq_dist(b.projected_line.start, edge_b.start);
                da.cmp(&db)
            })
            .then_with(|| {
                let la = line_length(&a.projected_line);
                let lb = line_length(&b.projected_line);
                la.cmp(&lb)
            })
    });

    let mut result: Vec<Vec<PaintedLine>> = vec![Vec::new(); contours.len()];

    // Group by (contour_idx, line_idx) via linear scan.
    let mut i = 0;
    while i < painted_lines.len() {
        let ci = painted_lines[i].contour_idx;
        let li = painted_lines[i].line_idx;
        let group_start = i;

        while i < painted_lines.len()
            && painted_lines[i].contour_idx == ci
            && painted_lines[i].line_idx == li
        {
            i += 1;
        }

        let group = &painted_lines[group_start..i];

        if ci < contours.len() && li < contours[ci].edges.len() {
            let contour_edge = &contours[ci].edges[li];
            let filtered = filter_painted_lines(contour_edge, group);
            result[ci].extend(filtered);
        }
    }

    result
}

/// Filter a group of painted lines that all project onto the same contour edge:
/// - Normalize each `projected_line` so its `start` is the endpoint closer to
///   `contour_edge.start` (matches edge walk direction).
/// - Sort by parametric position along the edge.
/// - Merge adjacent same-(semantic,value) segments with gap ≤ MERGE_GAP_THRESHOLD.
/// - Trim first segment to start at edge start; trim last to end at edge end.
/// - Discard segments shorter than MIN_SEGMENT_LENGTH.
///
/// **Direction normalization is required**: `triangle_z_intersection` produces lines
/// whose direction depends on the source triangle's vertex order, NOT the contour
/// edge walk direction.  Without normalization, the trim step would clobber
/// reverse-direction PaintedLines to zero length and discard them — causing the
/// edge to lose its paint color even though the source triangle is correctly
/// painted.  Symptom: a uniformly-painted vertical face produces no colored
/// segment for its contour edge → adjacent Voronoi cell stays `None` (BASE).
fn filter_painted_lines(contour_edge: &Line, group: &[PaintedLine]) -> Vec<PaintedLine> {
    if group.is_empty() {
        return Vec::new();
    }

    // Step 1: normalize projected_line direction so start is closer to edge.start.
    let mut normalized: Vec<PaintedLine> = Vec::with_capacity(group.len());
    for pl in group {
        let d_start = sq_dist(pl.projected_line.start, contour_edge.start);
        let d_end = sq_dist(pl.projected_line.end, contour_edge.start);
        let mut copy = pl.clone();
        if d_end < d_start {
            std::mem::swap(&mut copy.projected_line.start, &mut copy.projected_line.end);
            std::mem::swap(&mut copy.line.start, &mut copy.line.end);
        }
        normalized.push(copy);
    }

    // Step 2: re-sort by sq_dist(projected_line.start, edge.start) — the post_process
    // pre-sort used the un-normalized start, which is no longer correct.
    normalized.sort_by(|a, b| {
        let da = sq_dist(a.projected_line.start, contour_edge.start);
        let db = sq_dist(b.projected_line.start, contour_edge.start);
        da.cmp(&db)
    });

    // Step 3: merge adjacent same-(semantic,value) lines.
    let mut merged: Vec<PaintedLine> = Vec::new();
    for pl in normalized {
        if let Some(last) = merged.last_mut() {
            let gap = sq_dist(last.projected_line.end, pl.projected_line.start);
            let same_paint = last.semantic == pl.semantic && last.value == pl.value;
            if same_paint && gap <= MERGE_GAP_THRESHOLD * MERGE_GAP_THRESHOLD {
                last.projected_line.end = pl.projected_line.end;
                last.line.end = pl.line.end;
                continue;
            }
        }
        merged.push(pl);
    }

    // Step 4: trim first → edge.start, last → edge.end.
    if let Some(first) = merged.first_mut() {
        first.projected_line.start = contour_edge.start;
        first.line.start = contour_edge.start;
    }
    if let Some(last) = merged.last_mut() {
        last.projected_line.end = contour_edge.end;
        last.line.end = contour_edge.end;
    }

    // Step 5: discard short segments.
    merged.retain(|pl| line_length(&pl.projected_line) >= MIN_SEGMENT_LENGTH);

    merged
}

// ---------------------------------------------------------------------------
// Phase 4b
// ---------------------------------------------------------------------------

/// Colorize contours: fill each contour edge with ColoredLine records covering painted
/// segments and unpainted gaps.
///
/// Returns `Vec<Vec<ColoredLine>>` indexed by contour_idx, with no gaps in coverage.
pub fn colorize_contours(
    contours: &[Contour],
    filtered_painted: &[Vec<PaintedLine>],
) -> Vec<Vec<ColoredLine>> {
    debug_assert_eq!(
        contours.len(),
        filtered_painted.len(),
        "contours.len() must equal filtered_painted.len()"
    );

    let mut output: Vec<Vec<ColoredLine>> = Vec::with_capacity(contours.len());

    for (contour_idx, (contour, painted_group)) in
        contours.iter().zip(filtered_painted.iter()).enumerate()
    {
        let mut colored: Vec<ColoredLine> = Vec::new();

        if painted_group.is_empty() {
            // Emit one ColoredLine per edge with value=None.
            for (edge_idx, edge) in contour.edges.iter().enumerate() {
                colored.push(ColoredLine {
                    line: *edge,
                    value: None,
                    poly_idx: contour_idx,
                    local_line_idx: edge_idx,
                });
            }
        } else {
            // Walk a cursor along the contour accumulating painted + gap segments.
            // cursor is the cumulative distance from contour start already covered.
            let mut edge_cursor: usize = 0; // current edge index
            let mut pos_in_edge: i64 = 0; // how far along the current edge we've covered

            for pl in painted_group.iter() {
                // Determine painted segment's position in contour coordinates.
                // For simplicity, we identify the edge by line_idx and use the edge directly.
                let target_edge_idx = pl.line_idx;

                // Emit unpainted segments for any completely skipped edges.
                while edge_cursor < target_edge_idx && edge_cursor < contour.edges.len() {
                    let edge = &contour.edges[edge_cursor];
                    let edge_len = line_length(edge);
                    if edge_len - pos_in_edge > COLORIZE_GAP_THRESHOLD {
                        // Emit remainder of this edge as unpainted.
                        let gap_start = interpolate_along_line(edge, pos_in_edge, edge_len);
                        colored.push(ColoredLine {
                            line: Line {
                                start: gap_start,
                                end: edge.end,
                            },
                            value: None,
                            poly_idx: contour_idx,
                            local_line_idx: edge_cursor,
                        });
                    }
                    edge_cursor += 1;
                    pos_in_edge = 0;
                }

                if edge_cursor >= contour.edges.len() {
                    break;
                }

                let edge = &contour.edges[edge_cursor];
                let edge_len = line_length(edge);

                // Compute where the painted segment starts/ends within this edge.
                let paint_start_dist = line_length(&Line {
                    start: edge.start,
                    end: pl.projected_line.start,
                });
                let paint_end_dist = line_length(&Line {
                    start: edge.start,
                    end: pl.projected_line.end,
                });

                // Emit unpainted prefix gap if needed.
                if paint_start_dist - pos_in_edge > COLORIZE_GAP_THRESHOLD {
                    let gap_start = interpolate_along_line(edge, pos_in_edge, edge_len);
                    let gap_end = pl.projected_line.start;
                    colored.push(ColoredLine {
                        line: Line {
                            start: gap_start,
                            end: gap_end,
                        },
                        value: None,
                        poly_idx: contour_idx,
                        local_line_idx: edge_cursor,
                    });
                }

                // Emit the painted segment.
                colored.push(ColoredLine {
                    line: pl.projected_line,
                    value: Some(pl.value.clone()),
                    poly_idx: contour_idx,
                    local_line_idx: edge_cursor,
                });

                pos_in_edge = paint_end_dist.max(0).min(edge_len);
            }

            // Emit trailing unpainted segments for any remaining edges.
            while edge_cursor < contour.edges.len() {
                let edge = &contour.edges[edge_cursor];
                let edge_len = line_length(edge);
                if edge_len - pos_in_edge > COLORIZE_GAP_THRESHOLD {
                    let gap_start = interpolate_along_line(edge, pos_in_edge, edge_len);
                    colored.push(ColoredLine {
                        line: Line {
                            start: gap_start,
                            end: edge.end,
                        },
                        value: None,
                        poly_idx: contour_idx,
                        local_line_idx: edge_cursor,
                    });
                }
                edge_cursor += 1;
                pos_in_edge = 0;
            }
        }

        output.push(colored);
    }

    output
}

/// Linearly interpolate a point `dist` units from the start of a line.
/// If the line has zero length, returns the start point.
fn interpolate_along_line(line: &Line, dist: i64, total_len: i64) -> Point2 {
    if total_len == 0 || dist == 0 {
        return line.start;
    }
    if dist >= total_len {
        return line.end;
    }
    let t = dist as f64 / total_len as f64;
    Point2 {
        x: line.start.x + ((line.end.x - line.start.x) as f64 * t).round() as i64,
        y: line.start.y + ((line.end.y - line.start.y) as f64 * t).round() as i64,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{PaintSemantic, PaintValue, Point2};

    fn pt(x: i64, y: i64) -> Point2 {
        Point2 { x, y }
    }

    fn make_line(sx: i64, sy: i64, ex: i64, ey: i64) -> Line {
        Line {
            start: pt(sx, sy),
            end: pt(ex, ey),
        }
    }

    fn make_painted(
        line: Line,
        projected: Line,
        contour_idx: usize,
        line_idx: usize,
        value: PaintValue,
    ) -> PaintedLine {
        use crate::algos::paint_segmentation::painted_line::PaintedLine;
        PaintedLine {
            line,
            semantic: PaintSemantic::Material,
            value,
            cell_indices: Vec::new(),
            contour_idx,
            line_idx,
            projected_line: projected,
        }
    }

    fn make_contour(edges: Vec<Line>) -> Contour {
        Contour { edges }
    }

    // --- Test 1: empty painted_lines → empty per-contour vecs ---
    #[test]
    fn post_process_empty_input_returns_empty_vecs() {
        let contour = make_contour(vec![make_line(0, 0, 100, 0)]);
        let result = post_process_painted_lines(&[contour], Vec::new());
        assert_eq!(result.len(), 1);
        assert!(result[0].is_empty());
    }

    // --- Test 2: sort by (contour_idx, line_idx) ---
    #[test]
    fn post_process_sorts_by_contour_and_line_idx() {
        let edge0 = make_line(0, 0, 1000, 0);
        let edge1 = make_line(1000, 0, 2000, 0);
        let contour = make_contour(vec![edge0, edge1]);

        // Feed in reverse order: line_idx=1 before line_idx=0
        let pl1 = make_painted(
            make_line(1000, 0, 2000, 0),
            make_line(1000, 0, 2000, 0),
            0,
            1,
            PaintValue::ToolIndex(2),
        );
        let pl0 = make_painted(
            make_line(0, 0, 1000, 0),
            make_line(0, 0, 1000, 0),
            0,
            0,
            PaintValue::ToolIndex(1),
        );

        let result = post_process_painted_lines(&[contour], vec![pl1, pl0]);
        // result[0] should contain both; the first entry's line_idx corresponds to edge 0
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[0][0].line_idx, 0);
        assert_eq!(result[0][1].line_idx, 1);
    }

    // --- Test 3: merge adjacent same-(semantic,value) segments with gap ≤ 10 units ---
    #[test]
    fn post_process_merges_adjacent_same_value_segments() {
        // Two segments on the same edge with a gap of exactly 5 units (≤ 10).
        let edge = make_line(0, 0, 2000, 0);
        let contour = make_contour(vec![edge]);

        let seg_a = make_painted(
            make_line(0, 0, 990, 0),
            make_line(0, 0, 990, 0),
            0,
            0,
            PaintValue::ToolIndex(1),
        );
        // gap = 995 - 990 = 5 units
        let seg_b = make_painted(
            make_line(995, 0, 2000, 0),
            make_line(995, 0, 2000, 0),
            0,
            0,
            PaintValue::ToolIndex(1),
        );

        let result = post_process_painted_lines(&[contour], vec![seg_a, seg_b]);
        // Should be merged into one segment (trimmed to edge bounds → 0..2000).
        assert_eq!(
            result[0].len(),
            1,
            "adjacent same-value segments must merge"
        );
    }

    // --- Test 4: discard segments shorter than 1 unit ---
    #[test]
    fn post_process_discards_short_segments() {
        // Feed two identical-value segments on the SAME edge where both are present,
        // but also feed a zero-length segment on a SECOND edge (isolated, not first/last
        // in its group) — after trim of the second-edge group, the zero-length stays short.
        // Simpler approach: two edges; edge 1 has a zero-length projected segment
        // that is NOT the first-or-last in the group (create another group member to absorb trim),
        // but this gets complex.
        //
        // Spec says trim happens to first/last, then discard.  The easiest direct test:
        // a group of TWO segments on the same edge where the *middle* segment (after merge)
        // has length 0.  Since merge+trim only touches first/last, a segment that merges
        // to zero length due to being a duplicate endpoint should be discarded.
        //
        // Practical approach: edge of length 1000; two same-value segments:
        //   seg_a: (0,0)→(500,0)    (trimmed start → edge.start=0,0; no change)
        //   seg_b: (500,0)→(500,0)  (zero-length, trimmed end → edge.end=1000,0 → non-trivial)
        // Because seg_b is the last, its end is trimmed to edge.end=(1000,0), making it non-zero.
        //
        // To actually hit the discard path, use a DIFFERENT-value segment so it is NOT merged,
        // and place it on a SECOND edge where the adjacent same-value segment absorbs the trim:
        // edge 0 runs 0→1000; edge 1 runs 1000→2000.
        // On edge 1: one segment of exactly 0 length that is neither first-nor-last (impossible
        // with a single-element group after merge).
        //
        // The correct minimal test: multiple segments on edge 0 where the middle one is short
        // (not first, not last, not merged because different values).
        // Feed: seg_a(value=1): 0→600, seg_mid(value=2): 601→601 (0-len), seg_c(value=1): 602→1000.
        // seg_mid is not first-or-last, is not merged (different value). After discard pass,
        // seg_mid (length 0) is discarded.
        let edge = make_line(0, 0, 1000, 0);
        let contour = make_contour(vec![edge]);

        let seg_a = make_painted(
            make_line(0, 0, 600, 0),
            make_line(0, 0, 600, 0),
            0,
            0,
            PaintValue::ToolIndex(1),
        );
        // Zero-length segment with a DIFFERENT value so it won't merge with neighbours.
        let seg_mid = make_painted(
            make_line(601, 0, 601, 0),
            make_line(601, 0, 601, 0),
            0,
            0,
            PaintValue::ToolIndex(99),
        );
        let seg_c = make_painted(
            make_line(602, 0, 1000, 0),
            make_line(602, 0, 1000, 0),
            0,
            0,
            PaintValue::ToolIndex(1),
        );

        let result = post_process_painted_lines(&[contour], vec![seg_a, seg_mid, seg_c]);
        // seg_mid (value=99, length=0) must be discarded; only 2 segments should remain.
        let has_mid = result[0]
            .iter()
            .any(|pl| pl.value == PaintValue::ToolIndex(99));
        assert!(!has_mid, "zero-length segment (value=99) must be discarded");
        // Remaining segments must be non-trivially short.
        for pl in &result[0] {
            assert!(
                line_length(&pl.projected_line) >= MIN_SEGMENT_LENGTH,
                "all retained segments must be >= MIN_SEGMENT_LENGTH"
            );
        }
    }

    // --- Test 5: colorize_contours on contour with NO painted lines → N ColoredLines all None ---
    #[test]
    fn colorize_no_painted_lines_emits_all_none() {
        let edges = vec![
            make_line(0, 0, 100, 0),
            make_line(100, 0, 100, 100),
            make_line(100, 100, 0, 100),
        ];
        let n = edges.len();
        let contour = make_contour(edges);
        let filtered: Vec<Vec<PaintedLine>> = vec![Vec::new()];

        let result = colorize_contours(&[contour], &filtered);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), n, "one ColoredLine per edge");
        for (i, cl) in result[0].iter().enumerate() {
            assert_eq!(cl.value, None, "edge {} must be None", i);
            assert_eq!(cl.poly_idx, 0);
            assert_eq!(cl.local_line_idx, i);
        }
    }

    // --- Test 6: 4-edge contour, ONE painted line covering edge[1] entirely ---
    #[test]
    fn colorize_single_painted_line_on_edge1() {
        let edges = vec![
            make_line(0, 0, 100, 0),   // edge 0
            make_line(100, 0, 200, 0), // edge 1
            make_line(200, 0, 300, 0), // edge 2
            make_line(300, 0, 400, 0), // edge 3
        ];
        let contour = make_contour(edges.clone());

        let painted = make_painted(edges[1], edges[1], 0, 1, PaintValue::ToolIndex(7));
        let filtered: Vec<Vec<PaintedLine>> = vec![vec![painted]];

        let result = colorize_contours(&[contour], &filtered);
        assert_eq!(result.len(), 1);

        // Find the painted entry.
        let painted_entries: Vec<&ColoredLine> =
            result[0].iter().filter(|cl| cl.value.is_some()).collect();
        assert_eq!(painted_entries.len(), 1);
        assert_eq!(painted_entries[0].value, Some(PaintValue::ToolIndex(7)));
        assert_eq!(painted_entries[0].local_line_idx, 1);
        assert_eq!(painted_entries[0].poly_idx, 0);

        // All remaining entries must be None.
        for cl in result[0].iter().filter(|cl| cl.value.is_none()) {
            assert_ne!(cl.local_line_idx, 1, "edge 1 should be painted, not None");
        }
    }

    // --- Test 7: painted line covering half of edge[1] → prefix + painted + suffix ---
    #[test]
    fn colorize_half_edge_painted_emits_prefix_and_suffix() {
        // edge[1] runs from x=1000 to x=2000 (length 1000 units).
        let edges = vec![
            make_line(0, 0, 1000, 0),    // edge 0
            make_line(1000, 0, 2000, 0), // edge 1 — 1000 units long
            make_line(2000, 0, 3000, 0), // edge 2
        ];
        let contour = make_contour(edges.clone());

        // Painted segment covers x=1400..1600 on edge[1] (middle 200 units).
        let pl = make_painted(
            make_line(1400, 0, 1600, 0),
            make_line(1400, 0, 1600, 0),
            0,
            1,
            PaintValue::ToolIndex(3),
        );
        let filtered: Vec<Vec<PaintedLine>> = vec![vec![pl]];

        let result = colorize_contours(&[contour], &filtered);
        assert_eq!(result.len(), 1);

        // There should be at least one Some(value) entry for edge[1].
        let painted_on_edge1: Vec<&ColoredLine> = result[0]
            .iter()
            .filter(|cl| cl.local_line_idx == 1 && cl.value.is_some())
            .collect();
        assert!(
            !painted_on_edge1.is_empty(),
            "edge[1] must have a painted segment"
        );
        assert_eq!(painted_on_edge1[0].value, Some(PaintValue::ToolIndex(3)));

        // There should also be unpainted segments on edge[1].
        let unpainted_on_edge1: Vec<&ColoredLine> = result[0]
            .iter()
            .filter(|cl| cl.local_line_idx == 1 && cl.value.is_none())
            .collect();
        assert!(
            !unpainted_on_edge1.is_empty(),
            "edge[1] must have unpainted prefix and/or suffix"
        );
    }

    // --- Test 8: debug_assert fires on length mismatch ---
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn colorize_panics_on_length_mismatch() {
        let contour = make_contour(vec![make_line(0, 0, 100, 0)]);
        // filtered_painted has 2 entries but contours has 1 — should panic in debug.
        let _ = colorize_contours(&[contour], &[Vec::new(), Vec::new()]);
    }
}
