// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
/// Phase 4f — `extract_colored_segments`
///
/// Walk the pruned MMU graph and emit `ColoredSegment` records.
/// H562: repair chords use `arc_idx: None` (never usize::MAX).
/// H567: all arc tracking uses explicit `usize` indices.
use crate::algos::paint_segmentation::triangle_intersect::Line;
use crate::algos::paint_segmentation::voronoi_graph::{MMU_Graph, MmuArcKind};
use slicer_ir::{PaintValue, Point2};
use std::collections::HashSet;

/// Minimum |signed area| (unit²) for a walk polygon to count as a real region
/// rather than a degenerate sliver. Degenerate repair slivers have area ~0-10;
/// real (even thin) colour strips are ≥ ~1e5, so 1e3 cleanly separates them
/// (mirrors Orca's `poly.is_valid()` which only rejects ~zero-area polygons).
const MIN_WALK_AREA: f64 = 1.0e3;

/// Shoelace signed area over an OPEN point ring (first != last). Positive = CCW.
fn poly_signed_area(pts: &[Point2]) -> f64 {
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut a = 0.0_f64;
    for i in 0..n {
        let p = pts[i];
        let q = pts[(i + 1) % n];
        a += (p.x as f64) * (q.y as f64) - (q.x as f64) * (p.y as f64);
    }
    a * 0.5
}

/// Sign of the orientation of (p, q, r): +1 CCW, -1 CW, 0 collinear.
/// Uses i128 to avoid overflow on i64 coordinates.
fn orient(p: Point2, q: Point2, r: Point2) -> i32 {
    let v = (q.x as i128 - p.x as i128) * (r.y as i128 - p.y as i128)
        - (q.y as i128 - p.y as i128) * (r.x as i128 - p.x as i128);
    v.signum() as i32
}

/// True if open segments ab and cd cross at a single interior point (proper
/// intersection — shared endpoints and collinear overlaps return false).
fn seg_proper_intersect(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let d1 = orient(a, b, c);
    let d2 = orient(a, b, d);
    let d3 = orient(c, d, a);
    let d4 = orient(c, d, b);
    d1 != d2 && d3 != d4 && d1 != 0 && d2 != 0 && d3 != 0 && d4 != 0
}

/// True if the closed polygon over `pts` (open ring, implicit last→first edge)
/// is simple — no two non-adjacent edges properly intersect.
fn is_simple_closed(pts: &[Point2]) -> bool {
    let n = pts.len();
    if n < 4 {
        return true;
    }
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        for j in (i + 1)..n {
            // Skip edges adjacent to edge i (they share a vertex).
            if (i + 1) % n == j || (j + 1) % n == i {
                continue;
            }
            let c = pts[j];
            let d = pts[(j + 1) % n];
            if seg_proper_intersect(a, b, c, d) {
                return false;
            }
        }
    }
    true
}

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
/// `arrived_via_arc`. Implements Orca's leftmost-angle convention
/// (MultiMaterialSegmentation.cpp:401-447):
///   - Excludes BORDER arcs whose colour differs from `seed_color` (NON_BORDER always allowed).
///   - If arrived via a BORDER arc of a different colour, returns None immediately.
///   - Selects the candidate with the smallest angle relative to the reverse-travel
///     direction of the incoming arc (leftmost turn), using `dir_from_node` to orient
///     each candidate leaving the current node.
fn get_next_arc(
    graph: &MMU_Graph,
    used_border: &[bool],
    used_nb: &HashSet<(usize, usize)>,
    arrived_via_arc: usize,
    node_idx: usize,
    seed_color: Option<PaintValue>,
) -> Option<usize> {
    let arc_in = &graph.arcs[arrived_via_arc];

    // Orca parity (MultiMaterialSegmentation.cpp:401-405): if we arrived along a
    // BORDER arc whose colour differs from the walk seed colour, the walk cannot
    // continue here. NON_BORDER arcs have no colour and never trigger this gate.
    if arc_in.kind == MmuArcKind::Border && arc_in.color != seed_color {
        return None;
    }

    // `process_line_vec_n` (Orca): reverse of the travel direction — points from
    // node_idx back toward the node we came from. Use `node_idx` to determine
    // which stored endpoint corresponds to our current position vs the previous node.
    let (node_pt, prev_pt) = if arc_in.from_node == node_idx {
        (arc_in.point_a, arc_in.point_b)
    } else {
        (arc_in.point_b, arc_in.point_a)
    };
    let mut prx = (prev_pt.x - node_pt.x) as f64;
    let mut pry = (prev_pt.y - node_pt.y) as f64;
    let plen = (prx * prx + pry * pry).sqrt();
    if plen < 1e-9 {
        prx = 1.0;
        pry = 0.0;
    } else {
        prx /= plen;
        pry /= plen;
    }

    // Candidates: non-deleted, unused arcs incident on node_idx, not the arc we
    // arrived on, excluding BORDER arcs of a different colour than the seed.
    // NON_BORDER (Voronoi bisector) arcs are never excluded by the colour filter.
    let candidates: Vec<usize> = graph.nodes[node_idx]
        .arc_indices
        .iter()
        .copied()
        .filter(|&ai| {
            let a = &graph.arcs[ai];
            let avail = if a.kind == MmuArcKind::Border {
                !used_border[ai]
            } else {
                // NonBorder usable if this direction (leaving node_idx) is free.
                !used_nb.contains(&(ai, node_idx))
            };
            ai != arrived_via_arc
                && !a.deleted
                && avail
                && !(a.kind == MmuArcKind::Border && a.color != seed_color)
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Leftmost-arc selection by Orca's angle convention (MultiMaterialSegmentation.cpp:430-447):
    // compute angle in [0, 2π) between the reverse-travel vector and each candidate's
    // leaving-node direction (via `dir_from_node`). Pick the SMALLEST angle (leftmost).
    let angle_of = |ai: usize| -> f64 {
        // Orientation: leaving node_idx — use dir_from_node, NOT raw point_a→point_b.
        let (dx, dy) = dir_from_node(&graph.arcs[ai], node_idx);
        let dot = (dx * prx + dy * pry).clamp(-1.0, 1.0);
        let mut ang = dot.acos();
        // cross2(neighbour_vec, incoming_rev) < 0 → reflex angle → 2π − angle.
        if dx * pry - dy * prx < 0.0 {
            ang = 2.0 * std::f64::consts::PI - ang;
        }
        ang
    };
    let chosen = candidates.iter().copied().min_by(|&a, &b| {
        angle_of(a)
            .partial_cmp(&angle_of(b))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    if std::env::var("PNP_PAINTSEG_CANDDBG").is_ok() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 40 {
            let descr: Vec<String> = candidates
                .iter()
                .map(|&ai| {
                    let a = &graph.arcs[ai];
                    let far = if a.from_node == node_idx {
                        a.to_node
                    } else {
                        a.from_node
                    };
                    format!(
                        "{}{}->{}@{:.2}",
                        if a.kind == MmuArcKind::Border {
                            "B"
                        } else {
                            "N"
                        },
                        node_idx,
                        far,
                        angle_of(ai)
                    )
                })
                .collect();
            eprintln!(
                "CANDDBG at node {} (seed={:?}) cands=[{}] chose={:?}",
                node_idx,
                seed_color,
                descr.join(" "),
                chosen.map(|ai| {
                    let a = &graph.arcs[ai];
                    if a.from_node == node_idx {
                        a.to_node
                    } else {
                        a.from_node
                    }
                })
            );
        }
    }
    chosen
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
    // Border arcs are single-use (winding direction). NonBorder (bisector) arcs are
    // traversable ONCE PER DIRECTION — matching OrcaSlicer's bidirectional NonBorder
    // arcs — keyed by (arc_idx, entry_node). This lets two adjacent colour regions
    // each use a shared bisector (once in each direction) rather than the first walk
    // starving the second (the ~50%-coverage failure mode).
    let mut used_border: Vec<bool> = vec![false; graph.arcs.len()];
    let mut used_nb: HashSet<(usize, usize)> = HashSet::new();
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
                    && !used_border[ai]
                    && graph.arcs[ai].kind == MmuArcKind::Border
            })
            .collect();

        for seed_arc_idx in seed_arcs {
            if used_border[seed_arc_idx] {
                continue;
            }

            // Capture the walk's seed colour once. Passed unchanged to every
            // get_next_arc call so colour filtering always uses the walk's
            // originating colour, not the colour of the arc currently being
            // traversed (which may be None for NON_BORDER arcs).
            let seed_color = graph.arcs[seed_arc_idx].color.clone();

            // Determine orientation: walk forward from start_node.
            let mut current_node = start_node;
            let mut current_arc = seed_arc_idx;
            let mut walk_segments: Vec<ColoredSegment> = Vec::new();
            // Parallel to walk_segments: (arc_idx, entry_node) per traversal, so the
            // repair can free the exact directed usage when it pops an arc.
            let mut walk_traversals: Vec<(usize, usize)> = Vec::new();
            let mut steps: usize = 0;
            const MAX_STEPS: usize = 65536; // guard

            loop {
                let is_border_arc = graph.arcs[current_arc].kind == MmuArcKind::Border;
                let already_used = if is_border_arc {
                    used_border[current_arc]
                } else {
                    used_nb.contains(&(current_arc, current_node))
                };
                if already_used || steps >= MAX_STEPS {
                    break;
                }
                if is_border_arc {
                    used_border[current_arc] = true;
                } else {
                    used_nb.insert((current_arc, current_node));
                }
                walk_traversals.push((current_arc, current_node));
                steps += 1;

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

                // Find continuation. Always pass seed_color so the colour filter
                // is stable across NON_BORDER arc traversals within a walk.
                match get_next_arc(
                    graph,
                    &used_border,
                    &used_nb,
                    current_arc,
                    next_node,
                    seed_color.clone(),
                ) {
                    Some(next_arc) => {
                        current_node = next_node;
                        current_arc = next_arc;
                    }
                    None => {
                        // Dead end: stop accumulating. The walk is closed implicitly
                        // by the polygon builder; validity (CCW + simple) and the
                        // pop-and-retry repair are handled below (Orca 547-562).
                        break;
                    }
                }
            }

            // ---- Walk-closure repair (OrcaSlicer MultiMaterialSegmentation.cpp:547-562) ----
            // Validate the polygon of the accumulated walk. If it is not a simple,
            // positive-area ring, pop the last arc (returning it to the pool so other
            // colours' walks can use it) and retry with the shorter prefix. This both
            // discards self-intersecting/degenerate tails AND frees over-consumed
            // bisector arcs — without it, the first walks grab shared NonBorder arcs
            // and starve later colours (the ~50%-coverage failure mode).
            let pre_repair_len = walk_segments.len();
            let pre_repair_trav: Vec<(usize, usize)> =
                if std::env::var("PNP_PAINTSEG_FAILWALK").is_ok() {
                    walk_traversals.clone()
                } else {
                    Vec::new()
                };
            let mut valid = false;
            let rdbg = std::env::var("PNP_PAINTSEG_REPAIRDBG").is_ok() && seed_color.is_some();
            while walk_segments.len() >= 3 {
                let pts: Vec<slicer_ir::Point2> =
                    walk_segments.iter().map(|s| s.line.start).collect();
                let area = poly_signed_area(&pts);
                let simple = is_simple_closed(&pts);
                if rdbg {
                    use std::sync::atomic::{AtomicUsize, Ordering};
                    static N: AtomicUsize = AtomicUsize::new(0);
                    if N.fetch_add(1, Ordering::Relaxed) < 30 {
                        eprintln!(
                            "REPAIRDBG seed={:?} len={} area={:.0} simple={}",
                            seed_color,
                            walk_segments.len(),
                            area,
                            simple
                        );
                    }
                }
                if area.abs() >= MIN_WALK_AREA && simple {
                    valid = true;
                    break;
                }
                // Pop the last arc and free its directed usage for reuse by other walks.
                walk_segments.pop();
                if let Some((ai, entry)) = walk_traversals.pop() {
                    if graph.arcs[ai].kind == MmuArcKind::Border {
                        used_border[ai] = false;
                    } else {
                        used_nb.remove(&(ai, entry));
                    }
                }
            }
            // The seed border arc must never be re-seeded, even on discard.
            used_border[seed_arc_idx] = true;
            if std::env::var("PNP_PAINTSEG_WALKDETAIL").is_ok() && walk_idx < 16 {
                let area = {
                    let pts: Vec<slicer_ir::Point2> =
                        walk_segments.iter().map(|s| s.line.start).collect();
                    poly_signed_area(&pts)
                };
                eprintln!(
                    "WALKDETAIL seed_color={:?} pre_len={} post_len={} area={:.0} valid={}",
                    seed_color,
                    pre_repair_len,
                    walk_segments.len(),
                    area,
                    valid
                );
            }
            // Dump the geometry of a FAILING painted walk (pre-repair) to locate the
            // self-intersection. Gated; prints only the first few per process.
            if !valid && std::env::var("PNP_PAINTSEG_FAILWALK").is_ok() {
                use std::sync::atomic::{AtomicUsize, Ordering};
                static N: AtomicUsize = AtomicUsize::new(0);
                if seed_color.is_some()
                    && pre_repair_len >= 4
                    && N.fetch_add(1, Ordering::Relaxed) < 3
                {
                    // Re-walk was consumed; reconstruct from result is not possible, so
                    // print the kinds/nodes of the traversal we recorded.
                    let trav: Vec<String> = pre_repair_trav
                        .iter()
                        .map(|&(ai, en)| {
                            let a = &graph.arcs[ai];
                            let far = if a.from_node == en {
                                a.to_node
                            } else {
                                a.from_node
                            };
                            format!(
                                "{}:{}->{}{}",
                                if a.kind == MmuArcKind::Border {
                                    "B"
                                } else {
                                    "N"
                                },
                                en,
                                far,
                                if a.from_node < graph.all_border_points
                                    || a.to_node < graph.all_border_points
                                {
                                    "*"
                                } else {
                                    ""
                                }
                            )
                        })
                        .collect();
                    eprintln!(
                        "FAILWALK seed={:?} prelen={} trav=[{}]",
                        seed_color,
                        pre_repair_len,
                        trav.join(" ")
                    );
                }
            }
            if valid {
                result.append(&mut walk_segments);
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

    /// Square with a centre node (index 4) and four spokes (centre↔corner) so each
    /// border arc can close into a triangle (border arc + two spokes). NonBorder
    /// spokes are registered at BOTH endpoints (matching `from_colored_lines`) and
    /// are traversable once per direction. This is the smallest graph that exercises
    /// real per-arc closure + the colour filter the way the live decomposition does.
    fn square_with_center_graph(colors: [Option<PaintValue>; 4]) -> MMU_Graph {
        let corners = [(0i64, 0i64), (100, 0), (100, 100), (0, 100)];
        let center = (50i64, 50i64);
        let mut nodes: Vec<MmuNode> = (0..5).map(|_| MmuNode::default()).collect();
        let mut arcs: Vec<MmuArc> = Vec::new();
        // 4 border arcs (corner ring), registered at from_node only (winding dir).
        for i in 0..4 {
            let (ax, ay) = corners[i];
            let (bx, by) = corners[(i + 1) % 4];
            let ai = arcs.len();
            arcs.push(MmuArc {
                from_node: i,
                to_node: (i + 1) % 4,
                color: colors[i].clone(),
                kind: MmuArcKind::Border,
                deleted: false,
                point_a: pt(ax, ay),
                point_b: pt(bx, by),
            });
            nodes[i].arc_indices.push(ai);
        }
        // 4 spokes centre↔corner, NonBorder, registered at both nodes.
        for i in 0..4 {
            let (cx, cy) = center;
            let (bx, by) = corners[i];
            let ai = arcs.len();
            arcs.push(MmuArc {
                from_node: 4,
                to_node: i,
                color: None,
                kind: MmuArcKind::NonBorder,
                deleted: false,
                point_a: pt(cx, cy),
                point_b: pt(bx, by),
            });
            nodes[4].arc_indices.push(ai);
            nodes[i].arc_indices.push(ai);
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
        // Square + centre, border colours alternating around the ring. Each border
        // arc closes into its own triangle (border arc + two None spokes). Orca's
        // colour filter forbids a walk from continuing across a BORDER arc of a
        // different colour, so NO walk may carry two distinct paint colours: this is
        // the faithful "separates at colour change" contract (a single mixed-colour
        // walk would be the colour-flood bug the cell-shortcut was introduced to dodge).
        use slicer_ir::PaintValue;
        use std::collections::{BTreeMap, BTreeSet};
        let graph = square_with_center_graph([
            Some(PaintValue::ToolIndex(0)),
            Some(PaintValue::ToolIndex(1)),
            Some(PaintValue::ToolIndex(0)),
            Some(PaintValue::ToolIndex(1)),
        ]);
        let segs = extract_colored_segments(&graph, 2);

        // Group real-arc segments per walk; each walk must carry at most ONE non-None
        // (paint) colour — the colour filter never lets a walk cross a colour change.
        let mut per_walk: BTreeMap<usize, BTreeSet<Option<PaintValue>>> = BTreeMap::new();
        for s in segs.iter().filter(|s| s.arc_idx.is_some()) {
            per_walk.entry(s.poly_idx).or_default().insert(s.color.clone());
        }
        assert!(!per_walk.is_empty(), "expected at least one closed walk");
        for (pidx, colset) in &per_walk {
            let painted: BTreeSet<_> = colset.iter().filter(|c| c.is_some()).collect();
            assert!(
                painted.len() <= 1,
                "walk {pidx} crossed a colour boundary (mixed colours {colset:?})"
            );
        }

        // Both paint colours must survive as seed colours across the separated walks.
        let seed_colors: BTreeSet<Option<PaintValue>> = per_walk
            .values()
            .filter_map(|cs| cs.iter().find(|c| c.is_some()).cloned())
            .collect();
        assert!(
            seed_colors.contains(&Some(PaintValue::ToolIndex(0)))
                && seed_colors.contains(&Some(PaintValue::ToolIndex(1))),
            "both colours must appear as separated walks; got {seed_colors:?}"
        );
    }

    #[test]
    fn extract_discards_non_closing_walk() {
        // A single border arc between two nodes cannot close into a polygon. The
        // faithful repair (Orca pop-retry, MultiMaterialSegmentation.cpp:547-562)
        // POPS the degenerate tail and discards the walk entirely — it never emits a
        // synthetic chord (the old arc_idx:None sentinel path no longer exists).
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

        assert!(
            segs.is_empty(),
            "a non-closing single-arc walk must be discarded; got {} segment(s)",
            segs.len()
        );
        // H562/H567: no segment may ever carry the usize::MAX sentinel.
        assert!(
            segs.iter().all(|s| s.arc_idx != Some(usize::MAX)),
            "must never use usize::MAX as an arc index"
        );
    }

    #[test]
    fn extract_poly_idx_increments_per_walk() {
        // Square + centre, all border arcs unpainted: each of the four border arcs
        // closes into its own triangle, so the walk index increments once per emitted
        // walk and the poly_idx values are distinct and contiguous from 0.
        let graph = square_with_center_graph([None, None, None, None]);
        let segs = extract_colored_segments(&graph, 1);

        let poly_idxs: std::collections::BTreeSet<usize> =
            segs.iter().map(|s| s.poly_idx).collect();
        assert!(
            poly_idxs.len() >= 2,
            "expected multiple distinct walks, got {poly_idxs:?}"
        );
        // Contiguous from 0: one increment per kept walk, no gaps.
        let max = *poly_idxs.iter().max().unwrap();
        assert_eq!(
            poly_idxs.len(),
            max + 1,
            "poly_idx must be contiguous from 0; got {poly_idxs:?}"
        );
    }
}
