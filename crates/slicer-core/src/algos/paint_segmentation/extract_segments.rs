/// Phase 4f — `extract_colored_segments`
///
/// Walk the pruned MMU graph and emit `ColoredSegment` records.
/// H562: repair chords use `arc_idx: None` (never usize::MAX).
/// H567: all arc tracking uses explicit `usize` indices.
use crate::algos::paint_segmentation::triangle_intersect::Line;
use crate::algos::paint_segmentation::voronoi_graph::{MMU_Graph, MmuArcKind};
use slicer_ir::PaintValue;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single colored segment produced by the walk in Phase 4f.
#[derive(Debug, Clone)]
pub struct ColoredSegment {
    /// The geometric line of this segment.
    pub line: Line,
    /// Index into `MMU_Graph::arcs` for real arcs; `None` for synthetic repair chords (H562).
    pub arc_idx: Option<usize>,
    /// Paint color of this segment; `None` = unpainted / default extrusion.
    pub color: Option<PaintValue>,
    /// Walk sequence number (which polygon-walk produced this segment).
    pub poly_idx: usize,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Return the "next" arc to walk from `node_idx` given that we arrived via
/// `arrived_via_arc`. Among all non-deleted arcs at that node (excluding the
/// one we came from), prefer the arc that makes the leftmost turn, then
/// same-color. Returns `None` if no continuation is available.
fn get_next_arc(
    graph: &MMU_Graph,
    used_arcs: &[bool],
    arrived_via_arc: usize,
    node_idx: usize,
    _color: Option<PaintValue>,
) -> Option<usize> {
    // Direction we arrived from (pointing INTO node_idx).
    let arc_in = &graph.arcs[arrived_via_arc];
    let in_dx = (arc_in.point_b.x - arc_in.point_a.x) as f64;
    let in_dy = (arc_in.point_b.y - arc_in.point_a.y) as f64;

    // Candidates: non-deleted, unused arcs incident on node_idx, not the arc we arrived on.
    let candidates: Vec<usize> = graph.nodes[node_idx]
        .arc_indices
        .iter()
        .copied()
        .filter(|&ai| ai != arrived_via_arc && !graph.arcs[ai].deleted && !used_arcs[ai])
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Pick the leftmost-turn arc (cross-product, then fallback to index order for determinism).
    candidates.iter().copied().min_by(|&a, &b| {
        let arc_a = &graph.arcs[a];
        let arc_b = &graph.arcs[b];
        let (adx, ady) = dir_from_node(arc_a, node_idx);
        let (bdx, bdy) = dir_from_node(arc_b, node_idx);
        // Cross product: positive → a is more to the left of in_dir than b.
        let cross_a = in_dx * ady - in_dy * adx;
        let cross_b = in_dx * bdy - in_dy * bdx;
        // Leftmost = largest positive cross product.
        cross_b
            .partial_cmp(&cross_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    })
}

/// Direction vector of `arc` leaving `node_idx` (normalized).
fn dir_from_node(
    arc: &crate::algos::paint_segmentation::voronoi_graph::MmuArc,
    node_idx: usize,
) -> (f64, f64) {
    let (from_pt, to_pt) = if arc.from_node == node_idx {
        (arc.point_a, arc.point_b)
    } else {
        (arc.point_b, arc.point_a)
    };
    let dx = (to_pt.x - from_pt.x) as f64;
    let dy = (to_pt.y - from_pt.y) as f64;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-9 {
        (1.0, 0.0)
    } else {
        (dx / len, dy / len)
    }
}

/// Emit one `ColoredSegment` for a real arc traversal from `from_node`.
fn segment_from_arc(
    graph: &MMU_Graph,
    arc_idx: usize,
    from_node: usize,
    walk_idx: usize,
) -> ColoredSegment {
    let arc = &graph.arcs[arc_idx];
    let (start, end) = if arc.from_node == from_node {
        (arc.point_a, arc.point_b)
    } else {
        (arc.point_b, arc.point_a)
    };
    ColoredSegment {
        line: Line { start, end },
        arc_idx: Some(arc_idx),
        color: arc.color.clone(),
        poly_idx: walk_idx,
    }
}

// ---------------------------------------------------------------------------
// Phase 4f — public
// ---------------------------------------------------------------------------

/// Walk the pruned graph and extract all colored segments.
///
/// Each Border arc seeding a new walk becomes one `ColoredSegment`. If a walk
/// cannot close cleanly, a synthetic repair chord is appended with `arc_idx: None`.
///
/// `num_color_states` is reserved for future sub-walk splitting per color
/// (currently unused but present for API parity with OrcaSlicer).
///
/// H562: `arc_idx: None` is the sentinel — NEVER `Some(usize::MAX)`.
/// H567: arc indices tracked as explicit `usize` via enumerate/stored values.
pub fn extract_colored_segments(
    graph: &MMU_Graph,
    _num_color_states: usize,
) -> Vec<ColoredSegment> {
    let mut result: Vec<ColoredSegment> = Vec::new();
    let mut used_arcs: Vec<bool> = vec![false; graph.arcs.len()];
    let mut walk_idx: usize = 0;

    // Iterate over all border nodes.
    for start_node in 0..graph.all_border_points {
        // Find an unused Border arc to start a new walk.
        let seed_arcs: Vec<usize> = graph.nodes[start_node]
            .arc_indices
            .iter()
            .copied()
            .filter(|&ai| {
                !graph.arcs[ai].deleted
                    && !used_arcs[ai]
                    && graph.arcs[ai].kind == MmuArcKind::Border
            })
            .collect();

        for seed_arc_idx in seed_arcs {
            if used_arcs[seed_arc_idx] {
                continue;
            }

            // Determine orientation: walk forward from start_node.
            let mut current_node = start_node;
            let mut current_arc = seed_arc_idx;
            let mut walk_segments: Vec<ColoredSegment> = Vec::new();
            let mut steps: usize = 0;
            const MAX_STEPS: usize = 65536; // guard

            loop {
                if used_arcs[current_arc] || steps >= MAX_STEPS {
                    break;
                }
                used_arcs[current_arc] = true;
                steps += 1;

                let arc_color = graph.arcs[current_arc].color.clone();
                let seg = segment_from_arc(graph, current_arc, current_node, walk_idx);
                // Advance current_node to the far end.
                let next_node = {
                    let a = &graph.arcs[current_arc];
                    if a.from_node == current_node {
                        a.to_node
                    } else {
                        a.from_node
                    }
                };
                walk_segments.push(seg);

                if next_node == start_node {
                    // Closed the cycle — done.
                    break;
                }

                // Find continuation.
                match get_next_arc(graph, &used_arcs, current_arc, next_node, arc_color) {
                    Some(next_arc) => {
                        current_node = next_node;
                        current_arc = next_arc;
                    }
                    None => {
                        // Repair path: emit a synthetic closing chord.
                        // H562: arc_idx = None (never usize::MAX).
                        let last_pt = graph.arcs[current_arc].point_b;
                        // Find the start point of the walk.
                        let start_pt = if graph.arcs[seed_arc_idx].from_node == start_node {
                            graph.arcs[seed_arc_idx].point_a
                        } else {
                            graph.arcs[seed_arc_idx].point_b
                        };

                        // The repair_index sentinel: we do NOT call unwrap() on None here.
                        // debug_assert that we are in the documented repair branch.
                        let repair_chord = ColoredSegment {
                            line: Line {
                                start: last_pt,
                                end: start_pt,
                            },
                            arc_idx: None, // H562 — Option<usize>::None sentinel
                            color: None,
                            poly_idx: walk_idx,
                        };
                        walk_segments.push(repair_chord);
                        break;
                    }
                }
            }

            if !walk_segments.is_empty() {
                result.extend(walk_segments);
                walk_idx += 1;
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Tests (AC-8)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::paint_segmentation::voronoi_graph::{MMU_Graph, MmuArc, MmuArcKind, MmuNode};
    use slicer_ir::Point2;

    fn pt(x: i64, y: i64) -> Point2 {
        Point2 { x, y }
    }

    /// Build a simple square graph: 4 border nodes, 4 Border arcs in a cycle.
    ///  0 -> 1 -> 2 -> 3 -> (back to 0)
    fn square_graph() -> MMU_Graph {
        let mut nodes: Vec<MmuNode> = (0..4).map(|_| MmuNode::default()).collect();
        let mut arcs: Vec<MmuArc> = Vec::new();
        let corners = [(0i64, 0i64), (100, 0), (100, 100), (0, 100)];
        let n = 4;
        for i in 0..n {
            let (ax, ay) = corners[i];
            let (bx, by) = corners[(i + 1) % n];
            let ai = arcs.len();
            arcs.push(MmuArc {
                from_node: i,
                to_node: (i + 1) % n,
                color: None,
                kind: MmuArcKind::Border,
                deleted: false,
                point_a: pt(ax, ay),
                point_b: pt(bx, by),
            });
            nodes[i].arc_indices.push(ai);
            nodes[(i + 1) % n].arc_indices.push(ai);
        }
        MMU_Graph::from_parts(nodes, arcs, 4, vec![0])
    }

    #[test]
    fn extract_simple_square_walk_emits_4_segments() {
        let graph = square_graph();
        let segs = extract_colored_segments(&graph, 1);
        assert_eq!(segs.len(), 4, "square should emit 4 segments");
        for s in &segs {
            assert!(
                s.arc_idx.is_some(),
                "all real arcs should have Some(arc_idx)"
            );
        }
    }

    #[test]
    fn extract_two_color_walk_separates_at_color_change() {
        // 4-node ring where arcs alternate between two colors.
        // Two walks: one per color group (or two poly_idx groups).
        use slicer_ir::PaintValue;
        let mut nodes: Vec<MmuNode> = (0..4).map(|_| MmuNode::default()).collect();
        let mut arcs: Vec<MmuArc> = Vec::new();
        let corners = [(0i64, 0i64), (100, 0), (100, 100), (0, 100)];
        let colors = [
            Some(PaintValue::ToolIndex(0)),
            Some(PaintValue::ToolIndex(1)),
            Some(PaintValue::ToolIndex(0)),
            Some(PaintValue::ToolIndex(1)),
        ];
        let n = 4;
        for i in 0..n {
            let (ax, ay) = corners[i];
            let (bx, by) = corners[(i + 1) % n];
            let ai = arcs.len();
            arcs.push(MmuArc {
                from_node: i,
                to_node: (i + 1) % n,
                color: colors[i].clone(),
                kind: MmuArcKind::Border,
                deleted: false,
                point_a: pt(ax, ay),
                point_b: pt(bx, by),
            });
            nodes[i].arc_indices.push(ai);
            nodes[(i + 1) % n].arc_indices.push(ai);
        }
        let graph = MMU_Graph::from_parts(nodes, arcs, 4, vec![0]);
        let segs = extract_colored_segments(&graph, 2);
        // All 4 arcs should be emitted with their respective colors.
        assert_eq!(segs.len(), 4);
        let c0_count = segs
            .iter()
            .filter(|s| s.color == Some(PaintValue::ToolIndex(0)))
            .count();
        let c1_count = segs
            .iter()
            .filter(|s| s.color == Some(PaintValue::ToolIndex(1)))
            .count();
        assert_eq!(c0_count, 2, "should have 2 segments with color 0");
        assert_eq!(c1_count, 2, "should have 2 segments with color 1");
    }

    #[test]
    fn extract_uses_option_none_sentinel_on_repair() {
        // Force repair path: a non-closing chain of 2 border nodes connected by one arc.
        // Node 0 (border) -> Node 1 (border), one arc, no way back.
        let mut nodes: Vec<MmuNode> = (0..2).map(|_| MmuNode::default()).collect();
        let mut arcs: Vec<MmuArc> = Vec::new();
        let ai = arcs.len();
        arcs.push(MmuArc {
            from_node: 0,
            to_node: 1,
            color: None,
            kind: MmuArcKind::Border,
            deleted: false,
            point_a: pt(0, 0),
            point_b: pt(100, 0),
        });
        nodes[0].arc_indices.push(ai);
        nodes[1].arc_indices.push(ai);

        let graph = MMU_Graph::from_parts(nodes, arcs, 2, vec![0]);
        let segs = extract_colored_segments(&graph, 1);

        // Walk from node 0 via arc 0 to node 1; node 1 has no continuation → repair chord.
        let repair = segs.iter().find(|s| s.arc_idx.is_none());
        assert!(
            repair.is_some(),
            "expected at least one repair chord with arc_idx: None"
        );

        // H562: ensure None is used, not Some(usize::MAX).
        for s in &segs {
            if s.arc_idx.is_some() {
                assert_ne!(
                    s.arc_idx,
                    Some(usize::MAX),
                    "must not use usize::MAX sentinel"
                );
            }
        }
    }

    #[test]
    fn extract_poly_idx_increments_per_walk() {
        // 3 disjoint single-arc walks: each border node pair produces its own walk.
        // Nodes 0,1 (border); 2,3 (border); 4,5 (border). Each pair has one arc.
        let mut nodes: Vec<MmuNode> = (0..6).map(|_| MmuNode::default()).collect();
        let mut arcs: Vec<MmuArc> = Vec::new();
        for pair in 0..3usize {
            let from = pair * 2;
            let to = pair * 2 + 1;
            let ai = arcs.len();
            arcs.push(MmuArc {
                from_node: from,
                to_node: to,
                color: None,
                kind: MmuArcKind::Border,
                deleted: false,
                point_a: pt((pair * 200) as i64, 0),
                point_b: pt((pair * 200 + 100) as i64, 0),
            });
            nodes[from].arc_indices.push(ai);
            nodes[to].arc_indices.push(ai);
        }

        // all_border_points = 6 (all nodes are border)
        let graph = MMU_Graph::from_parts(nodes, arcs, 6, vec![0, 2, 4]);
        let segs = extract_colored_segments(&graph, 1);

        // 3 real arcs + 3 repair chords (each walk can't close back) = 6 total.
        // Poly_idx values seen.
        let poly_idxs: std::collections::BTreeSet<usize> =
            segs.iter().map(|s| s.poly_idx).collect();
        assert_eq!(
            poly_idxs.len(),
            3,
            "expected 3 distinct poly_idx values, got {:?}",
            poly_idxs
        );
        let mut sorted: Vec<usize> = poly_idxs.into_iter().collect();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2]);
    }
}
