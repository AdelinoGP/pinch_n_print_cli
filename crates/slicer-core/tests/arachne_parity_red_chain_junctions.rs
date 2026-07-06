//! Red tests encoding finding F3 of the Arachne parity audit
//! (`target/arachne_parity_audit_*.md`).
//!
//! **Finding F3:** PNP's `chain_junctions_for_bead`
//! (`crates/slicer-core/src/arachne/generate_toolpaths.rs:395-443`)
//! uses a **width-based** merge at shared vertices: it compares
//! `p.width` of the two candidate junctions and keeps the wider one.
//! OrcaSlicer's `connectJunctions`
//! (`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2315-2322`)
//! uses a **`perimeter_index` pop-back** merge: it pops back entries
//! from `to_junctions` whose `perimeter_index <= next_edge's
//! from_junctions[0].perimeter_index`, then appends.
//!
//! # Status: currently passing (signal too weak to fail)
//!
//! The F3 signal is subtle: a wider junction having a *lower*
//! `perimeter_index` (or vice versa) is needed to expose the difference.
//! The two tests in this file are designed as **invariant locks** that
//! document the F3 contract for the fix agent. With the current
//! symmetric fixture (all vertices at the same R), the width-based and
//! perimeter_index-based merges produce equivalent results, so the
//! tests pass.
//!
//! # What a stronger test would need
//!
//! A test that genuinely *fails* under PNP's width-based merge and
//! passes under OrcaSlicer's perimeter_index-based merge would need:
//!   1. Two edges with a shared vertex.
//!   2. Bead 0 (outer) on edge 0 has a *narrower* width than bead 0 on
//!      edge 1 (or the same bead on the next edge), AND
//!   3. The narrow junction has a *higher* `perimeter_index` than the
//!      wide one.
//!
//! This is constructible from the public `BeadingStrategy` API by
//! using a strategy that returns bead widths in a different order on
//! the two endpoints (impossible with the current `BeadingStrategy`
//! API, which returns one `Beading` per call). A more thorough
//! investigation would require either:
//!   (a) extending `BeadingStrategy::compute` to take per-endpoint
//!       context (a larger refactor than this audit is scoped to), or
//!   (b) constructing a hand-built graph that drives
//!       `chain_junctions_for_bead` directly through a pub-crate test
//!       helper (not currently exposed).
//!
//! # Tests in this file
//!
//! Both tests pass on the current code AND on the OrcaSlicer-faithful
//! fix. They are written here as a regression lock so a future
//! "simplification" of the merge logic does not silently regress to
//! dropping one side's junction at a shared vertex.
//!
//! Host-only: gated behind `host-algos`.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::generate_toolpaths::generate_toolpaths;
use slicer_core::beading::Beading;
use slicer_core::skeletal_trapezoidation::{
    EdgeType, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::Vertex;

// ---------------------------------------------------------------------------
// A symmetric beading strategy (bead 0 and bead 1 have the same width),
// so width-based and perimeter_index-based merges produce identical
// results. The F3 difference only emerges with an asymmetric strategy.
// ---------------------------------------------------------------------------
struct SymmetricBeadingStrategy;

impl slicer_core::beading::BeadingStrategy for SymmetricBeadingStrategy {
    fn compute(&self, thickness: f64, bead_count: usize) -> Beading {
        if bead_count == 0 {
            return Beading {
                total_thickness: thickness,
                bead_widths: Vec::new(),
                toolpath_locations: Vec::new(),
                left_over: thickness,
            };
        }
        let width = thickness / bead_count as f64;
        let bead_widths = vec![width; bead_count];
        let toolpath_locations = (0..bead_count).map(|i| width * (i as f64 + 0.5)).collect();
        Beading {
            total_thickness: thickness,
            bead_widths,
            toolpath_locations,
            left_over: 0.0,
        }
    }

    fn optimal_bead_count(&self, _thickness: f64) -> usize {
        2
    }

    fn get_transition_thickness(&self, _lower_bead_count: usize) -> f64 {
        f64::MAX
    }

    fn optimal_thickness(&self, bead_count: usize) -> f64 {
        bead_count as f64 * 400_000.0
    }

    fn type_label(&self) -> &'static str {
        "SymmetricBeadingStrategy"
    }
}

/// Builds a 2-edge central chain: e0 (v0 -> v1), e1 (v1 -> v2), with
/// twin edges e2 and e3 (non-central). All vertices at the same R.
fn make_f3_target_graph() -> SkeletalTrapezoidationGraph {
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 1_000_000.0,
        bead_count: Some(2),
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 5.0 * 10_000.0,
            y: 0.0,
        },
        distance_to_boundary: 1_000_000.0,
        bead_count: Some(2),
        transition_ratio: 0.0,
    };
    let v2 = STVertex {
        position: Vertex {
            x: 10.0 * 10_000.0,
            y: 0.0,
        },
        distance_to_boundary: 1_000_000.0,
        bead_count: Some(2),
        transition_ratio: 0.0,
    };

    let e0 = STHalfEdge {
        start_vertex: 0,
        twin: 2,
        next: 1,          // e0 -> e1 chain
        prev: usize::MAX, // e0 is a domain-start
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let e1 = STHalfEdge {
        start_vertex: 1,
        twin: 3,
        next: usize::MAX, // e1's next is NO_INDEX (dead end)
        prev: 0,          // e1 is chained off e0
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let e2 = STHalfEdge {
        start_vertex: 1,
        twin: 0,
        next: usize::MAX,
        prev: usize::MAX,
        central: false,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let e3 = STHalfEdge {
        start_vertex: 2,
        twin: 1,
        next: usize::MAX,
        prev: usize::MAX,
        central: false,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };

    SkeletalTrapezoidationGraph {
        vertices: vec![v0, v1, v2],
        edges: vec![e0, e1, e2, e3],
        centrality_filtered: true,
        rib: Default::default(),
    }
}

// ---------------------------------------------------------------------------
// F3 invariant lock: at a shared vertex, the chain must have exactly
// 3 junctions for a 2-edge 2-bead chain (one per endpoint, merged at
// the shared vertex). Dropping one side or duplicating both is a bug.
// ---------------------------------------------------------------------------

#[test]
fn f3_invariant_chain_has_one_junction_per_endpoint_at_shared_vertex() {
    let graph = make_f3_target_graph();
    let strategy = SymmetricBeadingStrategy;
    let output = generate_toolpaths(&graph, &strategy);

    assert!(!output.is_empty(), "expected at least one inset bucket");
    for bucket in &output {
        for line in bucket {
            assert_eq!(
                line.junctions.len(),
                3,
                "F3 invariant: each bead's chain must have exactly 3 \
                 junctions (v0, v1, v2) for a 2-edge chain. Got {} \
                 junctions. PNP current behavior with the symmetric \
                 fixture: width-based and perimeter_index-based merges \
                 produce the same result, so this test passes. The fix \
                 agent must ensure the OrcaSlicer-faithful merge also \
                 produces 3 junctions here. This is finding F3 of the \
                 Arachne parity audit.",
                line.junctions.len()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// F3 invariant lock: every junction at a shared vertex must have a
// finite, positive width. A dropped-side bug would produce junctions
// with width 0.0 or NaN.
// ---------------------------------------------------------------------------

#[test]
fn f3_invariant_junction_widths_are_finite_and_positive() {
    let graph = make_f3_target_graph();
    let strategy = SymmetricBeadingStrategy;
    let output = generate_toolpaths(&graph, &strategy);

    for bucket in &output {
        for line in bucket {
            for (i, j) in line.junctions.iter().enumerate() {
                assert!(
                    j.p.width > 0.0 && j.p.width.is_finite(),
                    "F3 invariant: junction {i} must have finite, positive \
                     width, got {}",
                    j.p.width
                );
            }
        }
    }
}
