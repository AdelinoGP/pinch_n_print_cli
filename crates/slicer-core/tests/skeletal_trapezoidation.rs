//! Packet 113c tests for the faithful per-cell + interleaved-rib graph
//! construction in [`SkeletalTrapezoidationGraph::from_polygons`].
//!
//! These supersede packet 113b's `quad_rib_topology_square_has_no_ribs` /
//! `quad_rib_topology_is_deterministic`, which encoded the (now-corrected)
//! reflex-corner-only rib assumption. Under the faithful construction a square
//! has *many* ribs, and its outer-wall domain closes into exactly one ring via
//! the `getNextUnconnected` (walk `.next` to a dead end, then take that edge's
//! `.twin`) traversal.
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature, so this whole file is a no-op under default features.

#![cfg(feature = "host-algos")]

use std::collections::BTreeSet;

use slicer_core::skeletal_trapezoidation::{EdgeType, SkeletalTrapezoidationGraph};
use slicer_core::voronoi::NO_INDEX;
use slicer_ir::{ExPolygon, Point2, Polygon};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// A plain 10 mm square (1 unit = 100 nm, so 10 mm = 100_000 units).
fn square_10mm() -> ExPolygon {
    expoly(vec![
        p(0, 0),
        p(100_000, 0),
        p(100_000, 100_000),
        p(0, 100_000),
    ])
}

/// `getNextUnconnected`: walk `.next` until a dead end (an edge with no
/// `.next`), then return that dead-end edge's `.twin` (or `NO_INDEX` if the
/// dead end has no twin — a broken/open domain, exactly the failure the packet
/// fixes).
fn get_next_unconnected(graph: &SkeletalTrapezoidationGraph, start: usize) -> usize {
    let mut cur = start;
    let mut guard = 0usize;
    loop {
        let next = graph.edges[cur].next;
        if next == NO_INDEX {
            return graph.edges[cur].twin;
        }
        cur = next;
        guard += 1;
        if guard > graph.edges.len() + 1 {
            // A `.next` cycle with no dead end — should never happen for a
            // faithfully-built graph; treat as broken.
            return NO_INDEX;
        }
    }
}

/// Decomposes the graph's domain-start seeds (edges with no `.prev`) into
/// closed `getNextUnconnected` rings. Returns the number of rings, or `None`
/// if any walk fails to close (hits a `NO_INDEX` dead end — the pre-fix bug).
fn count_domain_rings(graph: &SkeletalTrapezoidationGraph) -> Option<usize> {
    let seeds: BTreeSet<usize> = (0..graph.edges.len())
        .filter(|&i| graph.edges[i].prev == NO_INDEX)
        .collect();
    if seeds.is_empty() {
        return Some(0);
    }

    let mut unvisited = seeds.clone();
    let mut rings = 0usize;
    let step_budget = graph.edges.len() * 4 + 8;

    while let Some(&start) = unvisited.iter().next() {
        let mut cur = start;
        let mut closed = false;
        for _ in 0..step_budget {
            unvisited.remove(&cur);
            let next = get_next_unconnected(graph, cur);
            if next == NO_INDEX {
                // Open/broken walk: the domain did not close.
                return None;
            }
            if next == start {
                closed = true;
                break;
            }
            cur = next;
        }
        if !closed {
            return None;
        }
        rings += 1;
    }
    Some(rings)
}

/// AC-3: a plain 10 mm square's central-edge domain closes into exactly one
/// ring (every `getNextUnconnected` walk closes, and one walk covers every
/// domain-start seed).
#[test]
fn square_domain_closes_into_one_ring() {
    let graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(&square_10mm()))
        .expect("square should build a skeletal graph");

    assert!(
        !graph.edges.is_empty(),
        "square graph must have edges to walk"
    );

    let seeds: Vec<usize> = (0..graph.edges.len())
        .filter(|&i| graph.edges[i].prev == NO_INDEX)
        .collect();
    assert!(
        !seeds.is_empty(),
        "square graph must have at least one domain-start seed (edge with no .prev)"
    );

    let rings = count_domain_rings(&graph)
        .expect("every getNextUnconnected walk must close (no NO_INDEX dead ends)");

    assert_eq!(
        rings, 1,
        "the square's outer-wall domain must close into exactly one ring, got {rings}"
    );
}

/// AC-N1 (corrects packet 113b's `quad_rib_topology_square_has_no_ribs`): the
/// faithful construction inserts a rib after every transferred edge except each
/// cell's closing edge, so a plain square produces *multiple* rib
/// (`EXTRA_VD`) edges — never zero.
#[test]
fn square_produces_multiple_ribs() {
    let graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(&square_10mm()))
        .expect("square should build a skeletal graph");

    let rib_count = graph
        .edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::EXTRA_VD)
        .count();

    assert!(
        rib_count >= 2,
        "a square must produce multiple rib (EXTRA_VD) edges under the faithful \
         makeRib-after-every-edge construction, got {rib_count}"
    );

    // Ribs come in twinned forth/back pairs, so the count must be even and
    // every rib must have a twin.
    assert_eq!(rib_count % 2, 0, "rib edges must come in twinned pairs");
    for (i, e) in graph.edges.iter().enumerate() {
        if e.edge_type == EdgeType::EXTRA_VD {
            assert_ne!(e.twin, NO_INDEX, "rib edge {i} must have a twin");
            assert_eq!(
                graph.edges[e.twin].edge_type,
                EdgeType::EXTRA_VD,
                "rib edge {i}'s twin must also be a rib"
            );
        }
    }
}

/// AC-N3: two runs on the same input produce identical graphs — same rib edges,
/// chain structure (`next`/`prev`/`twin`/`start_vertex`), and per-edge
/// classification. Construction iterates cells/edges in stable index order with
/// no `HashMap` dependence.
#[test]
fn graph_construction_is_deterministic() {
    let square = square_10mm();

    let g1 = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(&square))
        .expect("square should build a skeletal graph (run 1)");
    let g2 = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(&square))
        .expect("square should build a skeletal graph (run 2)");

    assert_eq!(
        g1.vertices.len(),
        g2.vertices.len(),
        "vertex counts must match across runs"
    );
    assert_eq!(
        g1.edges.len(),
        g2.edges.len(),
        "edge counts must match across runs"
    );

    for (i, (e1, e2)) in g1.edges.iter().zip(g2.edges.iter()).enumerate() {
        assert_eq!(
            e1.start_vertex, e2.start_vertex,
            "start_vertex diverged at edge {i}"
        );
        assert_eq!(e1.twin, e2.twin, "twin diverged at edge {i}");
        assert_eq!(e1.next, e2.next, "next diverged at edge {i}");
        assert_eq!(e1.prev, e2.prev, "prev diverged at edge {i}");
        assert_eq!(e1.edge_type, e2.edge_type, "edge_type diverged at edge {i}");
        assert_eq!(e1.is_curved, e2.is_curved, "is_curved diverged at edge {i}");
    }

    // Whole-graph structural equality (positions, distances, cell assignments).
    assert!(
        g1 == g2,
        "two runs on the same input must produce structurally identical graphs"
    );
}
