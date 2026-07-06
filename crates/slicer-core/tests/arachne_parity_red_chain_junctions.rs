//! Red tests encoding finding F3 of the Arachne parity audit
//! (`target/arachne_parity_audit_*.md`).
//!
//! **Finding F3 (as stated by the audit):** PNP's `chain_junctions_for_bead`
//! (`crates/slicer-core/src/arachne/generate_toolpaths.rs:395-443`)
//! uses a **width-based** merge at shared vertices: it compares
//! `p.width` of the two candidate junctions and keeps the wider one.
//! OrcaSlicer's `connectJunctions`
//! (`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2315-2322`)
//! uses a **`perimeter_index` pop-back** merge.
//!
//! # Status: F3 (the merge heuristic) is NOT a finding
//!
//! Re-investigation (post-F1/F2/F5/F6 fixes) concluded F3's stated
//! severity rationale ("missing beads") is wrong: PNP's
//! `chain_junctions_for_bead` runs **per bead index**
//! (`generate_toolpaths.rs:588`), so at a shared vertex it merges
//! `edge_i.to_junctions[b]` with `edge_{i+1}.from_junctions[b]` — the
//! **same bead `b`** — and keeps exactly one. No bead goes missing;
//! the audit was reasoning about OrcaSlicer's full-list `connectJunctions`
//! (where pop_back removes a perimeter_index from the combined list),
//! which is a different architecture. The position difference between
//! the two merges is sub-vertex interpolation noise that the audit's
//! own red tests could not distinguish (see the original "What a
//! stronger test would need" note below, kept for history). F3 is
//! documented as not-a-finding, matching F4 and F7.
//!
//! # The actual defect this file now guards: the `t_to` constant-radius fallback
//!
//! While investigating F3, a separate real defect was found in
//! `generate_junctions` (`generate_toolpaths.rs:282-291`): for a
//! constant-radius central edge (`delta_r_mm == 0`), the `to_junctions`
//! fallback was `t = 0` (start vertex), placing both `from_junctions[b]`
//! and `to_junctions[b]` at the **start** vertex — collapsing the edge's
//! two junctions onto one point and dropping the end vertex from the
//! chain. This fires on real topology (102 constant-radius central
//! edges in `cube_4color.3mf`), producing visibly wrong wall geometry
//! (~0.17mm Y-shift on the outer walls). The fix: `t_to` falls back to
//! `1.0` (end vertex) for constant-radius edges, so `from_junctions`
//! lands at the start and `to_junctions` lands at the end.
//!
//! OrcaSlicer avoids this case entirely (`SkeletalTrapezoidation.cpp:2024-2025`
//! skips edges where `end_R >= start_R`, which includes constant-radius),
//! but PNP processes all central edges and must emit both endpoints'
//! junctions to chain across shared vertices.
//!
//! # Original F3 "What a stronger test would need" (kept for history)
//!
//! A test that genuinely *fails* under PNP's width-based merge and
//! passes under OrcaSlicer's perimeter_index-based merge would need:
//!   1. Two edges with a shared vertex.
//!   2. Bead 0 (outer) on edge 0 has a *narrower* width than bead 0 on
//!      edge 1 (or the same bead on the next edge), AND
//!   3. The narrow junction has a *higher* `perimeter_index` than the
//!      wide one.
//!
//! This is not constructible from the public `BeadingStrategy` API
//! without a per-endpoint context extension, and (per the re-
//! investigation) would not demonstrate a real defect even if built.
//!
//! # Tests in this file
//!
//! The first two tests are the original F3 invariant locks (count and
//! width-positivity). The third test is the strengthened constant-radius
//! position assertion that genuinely fails under the old `t_to = 0`
//! fallback and passes under the `t_to = 1` fix.
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

// ---------------------------------------------------------------------------
// Strengthened constant-radius position assertion (guards the real defect
// found during F3 re-investigation: the `t_to = 0` fallback for constant-
// radius edges). The `make_f3_target_graph` fixture is a 3-vertex constant-
// radius chain (v0=(0,0), v1=(50,0), v2=(100,0), all R=10mm). For a
// constant-radius edge, the to-junction must land at the edge's END vertex,
// not its start vertex. Under the old `t_to = 0` fallback both junctions
// landed at the start, collapsing the chain. This test asserts the actual
// x-coordinates of the emitted junctions match the vertices v0/v1/v2 in
// order — it fails under `t_to = 0` and passes under `t_to = 1`.
// ---------------------------------------------------------------------------

#[test]
fn constant_radius_chain_to_junction_lands_at_end_vertex_not_start() {
    let graph = make_f3_target_graph();
    let strategy = SymmetricBeadingStrategy;
    let output = generate_toolpaths(&graph, &strategy);

    assert!(!output.is_empty(), "expected at least one inset bucket");
    for bucket in &output {
        for line in bucket {
            assert_eq!(
                line.junctions.len(),
                3,
                "constant-radius 2-edge chain must produce exactly 3 \
                 junctions (one per vertex), got {}",
                line.junctions.len()
            );
            // v0 = (0, 0) mm, v1 = (5, 0) mm, v2 = (10, 0) mm. The chain
            // is e0 (v0->v1) then e1 (v1->v2). Under the `t_to = 1` fix:
            //   junction[0] = e0.from = v0 = (0, 0)
            //   junction[1] = merge(e0.to=v1, e1.from=v1) = v1 = (5, 0)
            //   junction[2] = e1.to = v2 = (10, 0)
            // Under the old `t_to = 0` fallback:
            //   junction[0] = v0 = (0, 0)
            //   junction[1] = merge(e0.to=v0 [WRONG], e1.from=v1) — the
            //     width tiebreak (`>=`, line 427) keeps e0.to = v0, so
            //     junction[1] = v0 = (0, 0) [WRONG, not v1]
            //   junction[2] = e1.to = v0 [WRONG, not v2]
            // So the x-coordinates under the bug are (0, 0, 0) instead of
            // (0, 5, 10). Assert the correct values.
            let xs: Vec<f32> = line.junctions.iter().map(|j| j.p.x).collect();
            assert!(
                (xs[0] - 0.0).abs() < 1e-3
                    && (xs[1] - 5.0).abs() < 1e-3
                    && (xs[2] - 10.0).abs() < 1e-3,
                "constant-radius chain junction x-coordinates must be \
                 (0, 5, 10) mm (v0, v1, v2). Got {xs:?}. If the middle \
                 and end are 0 instead of 5/10, the `t_to` constant-\
                 radius fallback in `generate_junctions` regressed to \
                 `t = 0` (start vertex), collapsing both junctions onto \
                 the start and dropping the end vertex from the chain."
            );
        }
    }
}
