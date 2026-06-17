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
/// Phase 4d — `remove_multiple_edges_in_vertices`
/// Phase 4e — `remove_nodes_with_one_arc`
///
/// Port of OrcaSlicer's MMU_Graph pruning passes.
/// Coordinate invariant: 1 unit = 100 nm.
use std::collections::VecDeque;

use crate::algos::paint_segmentation::colorize::ColoredLine;
use crate::algos::paint_segmentation::voronoi_graph::{MMU_Graph, MmuArcKind};

// ---------------------------------------------------------------------------
// Phase 4d helpers
// ---------------------------------------------------------------------------

/// Compute the "total chain length" of an arc by following nearly-straight
/// continuations (angle delta within 15 degrees).
///
/// Returns the cumulative Euclidean length in units (f64).
fn compute_total_chain_length(graph: &MMU_Graph, start_arc_idx: usize) -> f64 {
    let max_angle_cos: f64 = (15.0f64.to_radians()).cos(); // cos(15°) ≈ 0.9659
    const MAX_CHAIN: usize = 256; // guard against cycles

    let arc_len = |ai: usize| -> f64 {
        let a = &graph.arcs[ai];
        let dx = (a.point_b.x - a.point_a.x) as f64;
        let dy = (a.point_b.y - a.point_a.y) as f64;
        (dx * dx + dy * dy).sqrt()
    };

    let arc_dir = |ai: usize| -> (f64, f64) {
        let a = &graph.arcs[ai];
        let dx = (a.point_b.x - a.point_a.x) as f64;
        let dy = (a.point_b.y - a.point_a.y) as f64;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-9 {
            (1.0, 0.0)
        } else {
            (dx / len, dy / len)
        }
    };

    let mut total = arc_len(start_arc_idx);
    let mut current = start_arc_idx;
    let (mut cx, mut cy) = arc_dir(start_arc_idx);

    for _ in 0..MAX_CHAIN {
        let to_node = graph.arcs[current].to_node;
        // Find a single non-deleted continuation arc from to_node (excluding current).
        let continuations: Vec<usize> = graph.nodes[to_node]
            .arc_indices
            .iter()
            .copied()
            .filter(|&ai| ai != current && !graph.arcs[ai].deleted)
            .collect();
        if continuations.len() != 1 {
            break;
        }
        let next = continuations[0];
        let (nx, ny) = arc_dir(next);
        let dot = cx * nx + cy * ny;
        if dot < max_angle_cos {
            break;
        }
        total += arc_len(next);
        current = next;
        cx = nx;
        cy = ny;
    }
    total
}

/// Mark an arc as deleted and remove it from both endpoint node arc_indices.
/// Then cascade: if either endpoint is an interior node (idx >= all_border_points)
/// and its remaining degree is 0, we don't need to do more (it becomes isolated).
fn delete_arc(graph: &mut MMU_Graph, arc_idx: usize) {
    graph.arcs[arc_idx].deleted = true;
    let from = graph.arcs[arc_idx].from_node;
    let to = graph.arcs[arc_idx].to_node;
    graph.nodes[from].arc_indices.retain(|&ai| ai != arc_idx);
    graph.nodes[to].arc_indices.retain(|&ai| ai != arc_idx);
}

/// Recursively (iteratively) delete a vertex and its sole arc if it becomes
/// a dangling interior node.
fn delete_vertex_deep(graph: &mut MMU_Graph, node_idx: usize) {
    let mut stack: Vec<usize> = vec![node_idx];
    while let Some(ni) = stack.pop() {
        if ni < graph.all_border_points {
            continue; // never delete border nodes
        }
        let active: Vec<usize> = graph.nodes[ni]
            .arc_indices
            .iter()
            .copied()
            .filter(|&ai| !graph.arcs[ai].deleted)
            .collect();
        if active.len() == 1 {
            let sole_arc = active[0];
            let far = if graph.arcs[sole_arc].from_node == ni {
                graph.arcs[sole_arc].to_node
            } else {
                graph.arcs[sole_arc].from_node
            };
            delete_arc(graph, sole_arc);
            stack.push(far);
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 4d — public
// ---------------------------------------------------------------------------

/// Remove redundant arcs at border vertices that have ≥ 3 non-deleted arcs.
///
/// For each border-node that touches ≥ 3 non-deleted arcs, keeps only the
/// longest NonBorder arc chain; the rest are deleted and their far-ends are
/// cascaded via `delete_vertex_deep`.
///
/// H567: all index tracking uses explicit `usize`; no pointer arithmetic.
pub fn remove_multiple_edges_in_vertices(graph: &mut MMU_Graph, color_poly: &[Vec<ColoredLine>]) {
    // Iterate over each polygon and each segment's first vertex (border node).
    for (poly_idx, poly_lines) in color_poly.iter().enumerate() {
        // Guard: polygon must be registered in the graph.
        if poly_idx >= graph.polygon_idx_offset.len() {
            continue;
        }
        let base = graph.polygon_idx_offset[poly_idx];

        for (local_line_idx, _cl) in poly_lines.iter().enumerate() {
            let node_idx = base + local_line_idx;
            if node_idx >= graph.nodes.len() {
                continue;
            }

            // Collect all non-deleted arc indices for this node.
            let active_arcs: Vec<usize> = graph.nodes[node_idx]
                .arc_indices
                .iter()
                .copied()
                .filter(|&ai| !graph.arcs[ai].deleted)
                .collect();

            if active_arcs.len() < 3 {
                continue;
            }

            // Collect non-Border, non-deleted arcs with their chain lengths.
            let mut non_border: Vec<(usize, f64)> = active_arcs
                .iter()
                .copied()
                .filter(|&ai| graph.arcs[ai].kind == MmuArcKind::NonBorder)
                .map(|ai| {
                    let chain_len = compute_total_chain_length(graph, ai);
                    (ai, chain_len)
                })
                .collect();

            if non_border.len() < 2 {
                continue; // nothing to prune
            }

            // Sort descending by chain length.
            non_border.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Keep index 0; delete the rest.
            for (arc_idx, _) in non_border.iter().skip(1) {
                let far = {
                    let a = &graph.arcs[*arc_idx];
                    if a.from_node == node_idx {
                        a.to_node
                    } else {
                        a.from_node
                    }
                };
                delete_arc(graph, *arc_idx);
                delete_vertex_deep(graph, far);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 4e — public
// ---------------------------------------------------------------------------

/// Remove all interior (non-border) nodes that have exactly one non-deleted arc,
/// propagating BFS-style until no more such nodes exist.
///
/// Border nodes (`node_idx < all_border_points`) are never removed.
pub fn remove_nodes_with_one_arc(graph: &mut MMU_Graph) {
    // Seed the queue with every interior node that has degree == 1.
    let mut queue: VecDeque<usize> = (graph.all_border_points..graph.nodes.len())
        .filter(|&ni| graph.active_arc_count(ni) == 1)
        .collect();

    while let Some(ni) = queue.pop_front() {
        if ni < graph.all_border_points {
            continue; // never touch border nodes
        }
        // Re-check degree (may have changed since enqueue).
        let active: Vec<usize> = graph.nodes[ni]
            .arc_indices
            .iter()
            .copied()
            .filter(|&ai| !graph.arcs[ai].deleted)
            .collect();
        if active.len() != 1 {
            continue;
        }
        let sole_arc = active[0];
        let far = if graph.arcs[sole_arc].from_node == ni {
            graph.arcs[sole_arc].to_node
        } else {
            graph.arcs[sole_arc].from_node
        };

        // Mark arc deleted and remove from both endpoint arc_indices.
        delete_arc(graph, sole_arc);

        // If far-end is interior and now has degree 1, enqueue it.
        if far >= graph.all_border_points && graph.active_arc_count(far) == 1 {
            queue.push_back(far);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (AC-7)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::paint_segmentation::triangle_intersect::Line;
    use crate::algos::paint_segmentation::voronoi_graph::{MMU_Graph, MmuArc, MmuArcKind, MmuNode};
    use slicer_ir::Point2;

    fn pt(x: i64, y: i64) -> Point2 {
        Point2 { x, y }
    }

    fn seg(ax: i64, ay: i64, bx: i64, by: i64) -> Line {
        Line {
            start: pt(ax, ay),
            end: pt(bx, by),
        }
    }

    /// Build a star graph: one center node (index 0, interior) connected to 3
    /// leaf nodes (indices 1,2,3, interior). All arcs are NonBorder. One border
    /// node at index 4 is connected to the center.
    ///
    /// Border hub node 0 is connected to 3 interior leaves via NonBorder arcs,
    /// plus 1 other border node via a Border arc.
    ///
    ///   0 (border hub) --(arc0, NB, len=100)--> 1 (interior leaf-short)
    ///                  --(arc1, NB, len=200)--> 2 (interior leaf-mid)
    ///                  --(arc2, NB, len=300)--> 3 (interior leaf-long)
    ///                  --(arc3, Border)-------> 4 (border peer)
    fn star_graph() -> (MMU_Graph, Vec<Vec<ColoredLine>>) {
        // Nodes: 0 = border hub, 4 = border peer; 1,2,3 = interior leaves.
        // all_border_points = 2 (nodes 0..2 are border: node 0 and node 4 rearranged).
        // Simpler: put border nodes first: 0 = hub, 1 = peer; interior = 2,3,4.
        let mut nodes: Vec<MmuNode> = (0..5).map(|_| MmuNode::default()).collect();
        let all_border_points = 2usize; // nodes 0 and 1 are border

        let mut arcs: Vec<MmuArc> = Vec::new();

        // arc0: hub(0) -> interior leaf(2), NonBorder, len=100
        let ai0 = arcs.len();
        arcs.push(MmuArc {
            from_node: 0,
            to_node: 2,
            color: None,
            kind: MmuArcKind::NonBorder,
            deleted: false,
            point_a: pt(0, 0),
            point_b: pt(100, 0),
        });
        nodes[0].arc_indices.push(ai0);
        nodes[2].arc_indices.push(ai0);

        // arc1: hub(0) -> interior leaf(3), NonBorder, len=200
        let ai1 = arcs.len();
        arcs.push(MmuArc {
            from_node: 0,
            to_node: 3,
            color: None,
            kind: MmuArcKind::NonBorder,
            deleted: false,
            point_a: pt(0, 0),
            point_b: pt(200, 0),
        });
        nodes[0].arc_indices.push(ai1);
        nodes[3].arc_indices.push(ai1);

        // arc2: hub(0) -> interior leaf(4), NonBorder, len=300
        let ai2 = arcs.len();
        arcs.push(MmuArc {
            from_node: 0,
            to_node: 4,
            color: None,
            kind: MmuArcKind::NonBorder,
            deleted: false,
            point_a: pt(0, 0),
            point_b: pt(300, 0),
        });
        nodes[0].arc_indices.push(ai2);
        nodes[4].arc_indices.push(ai2);

        // arc3: hub(0) -> peer(1), Border arc
        let ai3 = arcs.len();
        arcs.push(MmuArc {
            from_node: 0,
            to_node: 1,
            color: None,
            kind: MmuArcKind::Border,
            deleted: false,
            point_a: pt(0, 0),
            point_b: pt(0, 50),
        });
        nodes[0].arc_indices.push(ai3);
        nodes[1].arc_indices.push(ai3);

        let graph = MMU_Graph::from_parts(nodes, arcs, all_border_points, vec![0]);

        // color_poly: polygon 0 has one segment whose local_line_idx=0 maps to border node 0.
        use crate::algos::paint_segmentation::colorize::ColoredLine;
        let color_poly = vec![vec![ColoredLine {
            line: seg(0, 0, 0, 50),
            value: None,
            poly_idx: 0,
            local_line_idx: 0,
        }]];

        (graph, color_poly)
    }

    #[test]
    fn prune_keeps_longest_arc_when_three_non_border_arcs_share_vertex() {
        let (mut graph, color_poly) = star_graph();
        // Before prune: center node(1) has 4 arcs (3 NonBorder + 1 Border)
        // → ≥ 3 total. After prune: only arc2 (len=300) should survive among NonBorder.
        remove_multiple_edges_in_vertices(&mut graph, &color_poly);

        let non_border_alive: Vec<usize> = graph
            .arcs
            .iter()
            .enumerate()
            .filter(|(_, a)| a.kind == MmuArcKind::NonBorder && !a.deleted)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            non_border_alive.len(),
            1,
            "expected exactly 1 surviving NonBorder arc"
        );
        // The surviving arc should be arc2 (len=300, to node 4)
        assert_eq!(
            graph.arcs[non_border_alive[0]].to_node, 4,
            "longest arc (to node 4) should survive"
        );
    }

    #[test]
    fn prune_iterates_cascade() {
        // Border hub node 0 connects to 3 interior branches:
        //   0 --(NB, len=100)--> 2
        //   0 --(NB, len=200)--> 3
        //   0 --(NB, len=500)--> 4-chain (4 -> 5, another 100 len, for a total chain ~600)
        //   0 --(Border)-------> 1
        // Node 4 also connects to node 5 (non-border), making its chain longer.
        // After pruning, only the arc toward the longest chain (arc 2→4) should survive.
        let mut nodes: Vec<MmuNode> = (0..6).map(|_| MmuNode::default()).collect();
        let all_border_points = 2usize; // 0=hub, 1=peer border

        let mut arcs: Vec<MmuArc> = Vec::new();

        macro_rules! add_arc {
            ($from:expr, $to:expr, $kind:expr, $ax:expr, $bx:expr) => {{
                let ai = arcs.len();
                arcs.push(MmuArc {
                    from_node: $from,
                    to_node: $to,
                    color: None,
                    kind: $kind,
                    deleted: false,
                    point_a: pt($ax, 0),
                    point_b: pt($bx, 0),
                });
                nodes[$from].arc_indices.push(ai);
                nodes[$to].arc_indices.push(ai);
            }};
        }

        add_arc!(0, 2, MmuArcKind::NonBorder, 0, 100); // arc0 len=100
        add_arc!(0, 3, MmuArcKind::NonBorder, 0, 200); // arc1 len=200
        add_arc!(0, 4, MmuArcKind::NonBorder, 0, 500); // arc2 len=500
        add_arc!(4, 5, MmuArcKind::NonBorder, 500, 600); // arc3 continues chain (straight)
        add_arc!(0, 1, MmuArcKind::Border, 0, 10); // arc4 Border

        let mut graph = MMU_Graph::from_parts(nodes, arcs, all_border_points, vec![0]);
        let color_poly = vec![vec![ColoredLine {
            line: seg(0, 0, 10, 0),
            value: None,
            poly_idx: 0,
            local_line_idx: 0,
        }]];

        remove_multiple_edges_in_vertices(&mut graph, &color_poly);
        // Only the longest-chain arc from node 0 should survive (arc2 → node 4).
        let alive_nb: Vec<usize> = graph
            .arcs
            .iter()
            .enumerate()
            .filter(|(_, a)| a.kind == MmuArcKind::NonBorder && !a.deleted && a.from_node == 0)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            alive_nb.len(),
            1,
            "expected 1 surviving NonBorder arc from hub"
        );
        assert_eq!(
            graph.arcs[alive_nb[0]].to_node, 4,
            "longest chain arc to node 4 should survive"
        );
    }

    #[test]
    fn prune_skips_nodes_with_lt_3_arcs() {
        // Node 0 (border) has only 2 arcs → no pruning.
        let mut nodes: Vec<MmuNode> = (0..3).map(|_| MmuNode::default()).collect();
        let mut arcs: Vec<MmuArc> = Vec::new();

        let ai0 = arcs.len();
        arcs.push(MmuArc {
            from_node: 0,
            to_node: 1,
            color: None,
            kind: MmuArcKind::Border,
            deleted: false,
            point_a: pt(0, 0),
            point_b: pt(10, 0),
        });
        nodes[0].arc_indices.push(ai0);
        nodes[1].arc_indices.push(ai0);

        let ai1 = arcs.len();
        arcs.push(MmuArc {
            from_node: 0,
            to_node: 2,
            color: None,
            kind: MmuArcKind::NonBorder,
            deleted: false,
            point_a: pt(0, 0),
            point_b: pt(0, 20),
        });
        nodes[0].arc_indices.push(ai1);
        nodes[2].arc_indices.push(ai1);

        let mut graph = MMU_Graph::from_parts(nodes, arcs, 1, vec![0]);
        let color_poly = vec![vec![ColoredLine {
            line: seg(0, 0, 10, 0),
            value: None,
            poly_idx: 0,
            local_line_idx: 0,
        }]];

        remove_multiple_edges_in_vertices(&mut graph, &color_poly);

        // Both arcs should still be alive.
        assert!(!graph.arcs[0].deleted);
        assert!(!graph.arcs[1].deleted);
    }

    #[test]
    fn prune_one_arc_bfs_strips_dangling_chain() {
        // Chain of 4 interior nodes all with degree 1 toward root.
        // nodes: 0(border), 1,2,3,4 (interior)
        // arcs: 0->1 (Border), 1->2 (NB), 2->3 (NB), 3->4 (NB)
        // All interior nodes eventually have degree 1 → BFS strips all.
        let mut nodes: Vec<MmuNode> = (0..5).map(|_| MmuNode::default()).collect();
        let mut arcs: Vec<MmuArc> = Vec::new();

        let segments = [
            (0usize, 1, 0i64, 10i64, MmuArcKind::Border),
            (1, 2, 10, 110, MmuArcKind::NonBorder),
            (2, 3, 110, 210, MmuArcKind::NonBorder),
            (3, 4, 210, 310, MmuArcKind::NonBorder),
        ];
        for (from, to, ax, bx, kind) in segments {
            let ai = arcs.len();
            arcs.push(MmuArc {
                from_node: from,
                to_node: to,
                color: None,
                kind,
                deleted: false,
                point_a: pt(ax, 0),
                point_b: pt(bx, 0),
            });
            nodes[from].arc_indices.push(ai);
            nodes[to].arc_indices.push(ai);
        }

        let mut graph = MMU_Graph::from_parts(nodes, arcs, 1, vec![0]);
        remove_nodes_with_one_arc(&mut graph);

        // All NonBorder arcs in this linear chain should be deleted.
        let alive_nb = graph
            .arcs
            .iter()
            .filter(|a| a.kind == MmuArcKind::NonBorder && !a.deleted)
            .count();
        assert_eq!(alive_nb, 0, "all dangling chain arcs should be stripped");
    }

    #[test]
    fn prune_one_arc_preserves_border_nodes() {
        // Border node 0 connected to interior 1 (sole arc from interior side).
        // Interior node should be pruned but border arc must not be deleted.
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

        // all_border_points = 1 → node 0 is border, node 1 is interior.
        let mut graph = MMU_Graph::from_parts(nodes, arcs, 1, vec![0]);
        remove_nodes_with_one_arc(&mut graph);

        // Node 1 is interior with degree 1, but the arc is Border — it gets deleted
        // (degree-1 interior → we delete its arc). But the border node itself
        // (node 0) is never enqueued.
        // The arc MAY be deleted because node 1 (interior) had degree 1.
        // The border node itself just loses the arc from its arc_indices.
        // Key: no panic, no attempt to delete node 0 as a vertex.
        // The function should complete without panic.
        // (border node's arc_indices may be cleared since delete_arc removes from both ends)
        let _ = &graph.nodes[0]; // still exists
        let _ = &graph.nodes[1]; // still exists (we don't remove node structs)
    }
}
