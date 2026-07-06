//! Red test encoding finding F7 of the Arachne parity audit
//! (`target/arachne_parity_audit_*.md`).
//!
//! **Finding F7 (already FIXED in packet 113c Step 8b):** PNP's
//! `propagation.rs::propagate_beadings_downward_with_transition_dist`
//! was fixed in packet 113c Step 8b for the production path
//! (line 750-754 uses `default_beading_propagation_transition_dist()`
//! as a real fallback, and `total_dist` is now the cumulative chain
//! distance). The no-argument `propagate_beadings_downward`
//! (line 866-870) also delegates to the same corrected code.
//!
//! The audit's F7 section in `target/arachne_parity_audit_*.md` was
//! written before the fix verification ran, and is therefore now
//! stale: the documented BLOCKED bug from packet 112 Step 3 is no
//! longer reproducible on the current code (this test passes).
//!
//! # Status
//!
//! PASSES (regression lock). The bug is fixed. This test guards
//! against a future regression that re-introduces the placeholder
//! `transition_dist = 4.0` or the "total_dist = edge_len" simplification.
//!
//! The original canonical red test is
//! `arachne_invariants::junction_count_delta_bound_at_domain_chain_stitches`,
//! which now also passes (verified 2026-07-05).
//!
//! Host-only: gated behind `host-algos`.

#![cfg(feature = "host-algos")]

use std::collections::BTreeSet;

use slicer_core::arachne::preprocess_input_outline;
use slicer_core::arachne::{run_arachne_pipeline, ArachneParams, PreprocessParams};
use slicer_core::skeletal_trapezoidation::{
    apply_transitions, assign_bead_counts, filter_central, generate_transition_mids,
    propagate_beadings_downward, propagate_beadings_upward, CentralityParams, EdgeType,
    SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::NO_INDEX;
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

#[allow(dead_code)]
fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// Resolves a half-edge's "to" vertex via its twin's `start_vertex`.
fn resolve_to_vertex(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> usize {
    let Some(edge) = graph.edges.get(edge_idx) else {
        return NO_INDEX;
    };
    if edge.twin == NO_INDEX {
        return NO_INDEX;
    }
    graph
        .edges
        .get(edge.twin)
        .map(|twin_edge| twin_edge.start_vertex)
        .unwrap_or(NO_INDEX)
}

/// Walks `.next` from `start` until a dead end, returning the visited
/// edge indices. Mirrors `arachne_invariants::find_quad`.
fn find_quad(graph: &SkeletalTrapezoidationGraph, start: usize) -> Vec<usize> {
    let max_len = graph.edges.len() + 1;
    let mut quad = vec![start];
    loop {
        assert!(
            quad.len() <= max_len,
            "quad walk from edge {start} exceeded {max_len} edges without reaching a dead end"
        );
        let current = *quad.last().expect("quad always has at least one edge");
        let next = graph.edges[current].next;
        if next == NO_INDEX {
            break;
        }
        quad.push(next);
    }
    quad
}

/// Builds the domain chains via the same algorithm as `generate_toolpaths`.
fn build_domain_chains(graph: &SkeletalTrapezoidationGraph) -> Vec<Vec<usize>> {
    let mut unprocessed: BTreeSet<usize> = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.prev == NO_INDEX)
        .map(|(idx, _)| idx)
        .collect();

    let mut chains = Vec::new();
    while let Some(&domain_start) = unprocessed.iter().next() {
        let mut chain = Vec::new();
        let mut quad_start = domain_start;
        loop {
            if !unprocessed.remove(&quad_start) {
                break;
            }
            let quad = find_quad(graph, quad_start);
            let quad_end = *quad.last().expect("find_quad returns >= 1 edge");
            for &edge_idx in &quad {
                let edge = &graph.edges[edge_idx];
                if edge.central && edge.edge_type == EdgeType::NORMAL {
                    chain.push(edge_idx);
                }
            }
            let next_start = graph
                .edges
                .get(quad_end)
                .map(|e| e.twin)
                .unwrap_or(NO_INDEX);
            if next_start == NO_INDEX || next_start == domain_start {
                break;
            }
            if !unprocessed.contains(&next_start) {
                break;
            }
            quad_start = next_start;
        }
        chains.push(chain);
    }
    chains
}

fn square_10mm() -> ExPolygon {
    let side = (10.0 * UNITS_PER_MM) as i64;
    expoly(vec![p(0, 0), p(side, 0), p(side, side), p(0, side)])
}

/// F7 red test: the bead-count delta at every domain-chain stitch must
/// be bounded by 1 (`|from.bc - to.bc| <= 1`). Fails under PNP current
/// because the `transition_dist = 4.0` placeholder in
/// `propagate_beadings_downward` corrupts the corner bead count.
///
/// This is a re-formulation of the canonical
/// `arachne_invariants::junction_count_delta_bound_at_domain_chain_stitches`
/// test, scoped to the F7 finding only and named to make the link
/// to the audit explicit.
#[test]
fn f7_invariant_bead_count_delta_bounded_at_domain_chain_stitches() {
    let square = square_10mm();
    let lines = run_arachne_pipeline(
        std::slice::from_ref(&square),
        &ArachneParams::default(),
        false,
    )
    .expect("square should produce Ok(lines)");

    // Corroborate that the pipeline actually produced lines.
    assert!(
        !lines.is_empty(),
        "expected non-empty toolpath output before checking graph structure"
    );

    // Build the same graph the pipeline would have built, then walk
    // domain chains and check bead-count deltas.
    let cleaned =
        preprocess_input_outline(std::slice::from_ref(&square), &PreprocessParams::default());
    let mut graph =
        SkeletalTrapezoidationGraph::from_polygons(&cleaned).expect("square should build a graph");

    let centrality_params = CentralityParams::new(0.01 * UNITS_PER_MM, 0.0);
    filter_central(&mut graph, &centrality_params, std::f64::consts::PI);

    use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
    let strategy = BeadingStrategyFactory::create_stack(&BeadingFactoryParams::default());
    assign_bead_counts(&mut graph, strategy.as_ref())
        .expect("centrality was run, so assign_bead_counts must succeed");
    generate_transition_mids(&mut graph, strategy.as_ref());
    apply_transitions(&mut graph);
    propagate_beadings_upward(&mut graph);
    // Use the no-argument `propagate_beadings_downward` (which is the
    // frozen entry point every existing test calls) -- this is the
    // F7 bug path: it delegates to the placeholder
    // `default_beading_propagation_transition_dist()` fallback, which
    // for `BeadingFactoryParams::default()` is 0.4mm (real value) but
    // for the per-edge `total_dist` math inside the function, the
    // placeholder of 4.0 units (= 0.0004mm) is what corrupts the
    // ratio_of_top calculation. (See the F7 doc comment for the full
    // derivation.)
    propagate_beadings_downward(&mut graph);

    let chains = build_domain_chains(&graph);
    let mut checked_pairs = 0usize;
    for chain in &chains {
        for pair in chain.windows(2) {
            let (edge_a, edge_b) = (pair[0], pair[1]);
            let to_a = resolve_to_vertex(&graph, edge_a);
            let to_b = resolve_to_vertex(&graph, edge_b);
            let bc_a = graph.vertices.get(to_a).and_then(|v| v.bead_count);
            let bc_b = graph.vertices.get(to_b).and_then(|v| v.bead_count);
            if let (Some(bc_a), Some(bc_b)) = (bc_a, bc_b) {
                checked_pairs += 1;
                let delta = (bc_a as i64 - bc_b as i64).abs();
                assert!(
                    delta <= 1,
                    "F7 invariant: junction-count delta at a domain-chain \
                     stitch between edge {edge_a} (to-vertex bc {bc_a}) and \
                     edge {edge_b} (to-vertex bc {bc_b}): delta {delta} > 1. \
                     This is the documented BLOCKED bug from packet 112 Step 3 \
                     / packet 113b Step 4: `propagate_beadings_downward`'s \
                     `transition_dist = 4.0` placeholder (4 units = 0.0004mm) \
                     causes ratio_of_top to clamp to 1.0, overwriting the \
                     bottom vertex's bead count from 0 to 9 on a 10mm square. \
                     See the Arachne parity audit, finding F7."
                );
            }
        }
    }
    assert!(
        checked_pairs > 0,
        "expected at least one adjacent central-edge pair to check in the \
         10mm square's domain chains. Got 0 (no central edges or no chain \
         has >= 2 central edges)."
    );
}
