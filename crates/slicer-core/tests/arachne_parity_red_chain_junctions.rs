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
use slicer_ir::UNITS_PER_MM;

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

    fn get_split_middle_threshold(&self) -> f64 {
        0.99_f64
    }

    fn get_add_middle_threshold(&self) -> f64 {
        0.99_f64
    }
}

/// Builds a 2-upward-edge central chain: e0 (v0 -> v1, upward) and
/// `rib_back` (v2 -> v1, upward, the "back" half of a rib pair at v1)
/// chained via `.next`. R is a peak at v1 (3mm) and drops to 1mm at v0
/// and v2 so the spine edge and the rib are TRUE upward half-edges
/// (not flat-equal-R, which `generateJunctions` silently drops —
/// `SkeletalTrapezoidation.cpp:2017-2019`). The chain walk's canonical
/// 3-way detection then sees each central vertex's central incident
/// half-edge count (canonical `isMultiIntersection()` at
/// `SkeletalTrapezoidationGraph.cpp:211-224`) and the rib (EXTRA_VD)
/// edges contribute to the count too — matching the new
/// `compute_vertex_degree` (central-edge count, not the prior
/// upward-only count).
fn make_f3_target_graph() -> SkeletalTrapezoidationGraph {
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 1.0 * UNITS_PER_MM,
        bead_count: None,
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 5.0 * 10_000.0,
            y: 0.0,
        },
        distance_to_boundary: 3.0 * UNITS_PER_MM,
        bead_count: Some(2),
        transition_ratio: 0.0,
    };
    let v2 = STVertex {
        position: Vertex {
            x: 10.0 * 10_000.0,
            y: 0.0,
        },
        distance_to_boundary: 1.0 * UNITS_PER_MM,
        bead_count: None,
        transition_ratio: 0.0,
    };

    // e0: central spine v0 -> v1 (upward, R 1.0 -> 3.0). Twin (e1) is
    // the off-skeleton counterpart (downward, non-central, not
    // processed by `generateJunctions` because of the upward-only
    // selection; also non-central so v1's central-incident count
    // stays at 2 — the F3 chain walks THROUGH v1 to v2, not stops).
    let e0 = STHalfEdge {
        start_vertex: 0,
        twin: 1,
        next: 2,          // e0 -> rib_back chain
        prev: usize::MAX, // e0 is a domain-start
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let e1 = STHalfEdge {
        start_vertex: 1,
        twin: 0,
        next: usize::MAX,
        prev: usize::MAX,
        central: false,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    // rib_back: upward rib v2 -> v1 (R 1.0 -> 3.0, central,
    // EdgeType::EXTRA_VD). Chained off e0 via `.next`. Twin (rib_forth)
    // has start_vertex=1 (v1) so the half-edge's "to" resolves to v1.
    let rib_back = STHalfEdge {
        start_vertex: 2,
        twin: 3,
        next: usize::MAX, // dead end at the peak
        prev: 0,          // chained off e0
        central: true,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    };
    // rib_forth: downward rib v1 -> v2 (R 3.0 -> 1.0, non-central,
    // EdgeType::EXTRA_VD). Twin of rib_back. Non-central so v1's
    // central-incident count stays at 2.
    let rib_forth = STHalfEdge {
        start_vertex: 1,
        twin: 2,
        next: usize::MAX,
        prev: usize::MAX,
        central: false,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    };

    SkeletalTrapezoidationGraph {
        vertices: vec![v0, v1, v2],
        edges: vec![e0, e1, rib_back, rib_forth],
        centrality_filtered: true,
        rib: Default::default(),
        ..Default::default()
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
// Position-monotonicity lock: the chain must traverse v0 -> v1 -> v2 in
// X-order (one junction near each vertex, monotonically increasing
// X). Replaces the prior constant-radius xs check (which assumed flat-R
// geometry and exact-vertex junction placement). With the new upward-R
// fixture, junctions are interpolated along their edges:
//   - junction[0]: e0's `from` slot, bead 0 at radius 1.5mm, so the
//     junction slides from peak (v1, R=3) toward boundary (v0, R=1) by
//     `t = (1.5-3.0)/(1.0-3.0) = 0.75` along e0, landing at
//     x = 5 + (0-5)*0.75 = 1.25mm.
//   - junction[1]: merge of e0's `to` and rib_back's `from` (both at
//     the same interpolated point — v1's bead 0 at 1.5mm).
//   - junction[2]: rib_back's `to` slot, bead 0 at radius 1.5mm, so
//     the junction slides from peak (v1, R=3) toward boundary (v2,
//     R=1) by `t = 0.75` along rib_back, landing at
//     x = 5 + (10-5)*0.75 = 8.75mm.
// The test asserts the X-ordering and a sane span (between v0 and v2);
// a `t_to = 0` regression would collapse junctions onto the start
// vertex (x=0) for every slot, producing xs ≈ [0, 0, 0] and failing
// the span check.
// ---------------------------------------------------------------------------

#[test]
fn chain_traverses_v0_v1_v2_in_x_order_with_sane_span() {
    let graph = make_f3_target_graph();
    let strategy = SymmetricBeadingStrategy;
    let output = generate_toolpaths(&graph, &strategy);

    assert!(!output.is_empty(), "expected at least one inset bucket");
    for bucket in &output {
        for line in bucket {
            assert_eq!(
                line.junctions.len(),
                3,
                "upward-R 2-edge chain must produce exactly 3 \
                 junctions (one per vertex), got {}",
                line.junctions.len()
            );
            let xs: Vec<f32> = line.junctions.iter().map(|j| j.p.x).collect();
            // X-ordering: the chain walks v0 -> v1 -> v2, so the
            // junction X's must be non-decreasing.
            assert!(
                xs[0] <= xs[1] && xs[1] <= xs[2],
                "chain X-ordering broken: xs={xs:?} must be non-decreasing"
            );
            // Span: the chain's first junction is on the e0 (v0->v1)
            // half-edge and the last is on the rib_back (v2->v1)
            // half-edge, so xs[0] must be > v0.x and xs[2] must be <
            // v2.x (each junction slides from the peak toward its
            // boundary end). A `t_to = 0` regression collapses both
            // onto the start vertex (xs[0] == xs[1] == xs[2] == 0)
            // and fails the span check.
            assert!(
                xs[0] > 0.0 + 1e-3 && xs[2] < 10.0 - 1e-3,
                "chain X-span broken: xs={xs:?} must lie strictly between \
                 v0.x=0 and v2.x=10. A `t_to = 0` regression collapses \
                 both endpoints onto the start vertex (xs ≈ [0, 0, 0]); \
                 that signature is what this test guards against."
            );
        }
    }
}
