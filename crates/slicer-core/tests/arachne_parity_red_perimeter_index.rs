//! Red test encoding finding **N2** (perimeter-index facet) of the
//! second-pass Arachne parity audit
//! (`target/arachne_parity_audit_20260706_020657.md`, §N2).
//!
//! **Finding N2 (perimeter_index facet):** canonical OrcaSlicer sets
//! `ExtrusionJunction::perimeter_index` to the junction's BEAD/INSET index
//! (`junction_idx`) when `generateJunctions` creates it
//! (`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2064-2077`;
//! `utils/ExtrusionJunction.hpp`), and `connectJunctions`'s secondary-fan
//! merge keys on exactly that value (the pop-back rule
//! `from_junctions.back().perimeter_index <= from_prev_junctions.front().perimeter_index`,
//! `SkeletalTrapezoidation.cpp:2302-2314`). PNP zeroes the field at
//! junction-generation time (`generate_toolpaths.rs:299-327`) and later
//! redefines it pipeline-wide as "sequential position within the line's
//! junction Vec" (`pipeline.rs:378-390`), which both breaks any consumer
//! expecting Orca semantics and makes the canonical pop-back merge
//! unimplementable without re-plumbing.
//!
//! This test pins the canonical contract at the `generate_toolpaths` layer:
//! every junction of a line carries `perimeter_index == line.inset_idx`.
//! FAILS on current code (all junctions carry 0, so the inset-1 line
//! violates it).
//!
//! Host-only: gated behind `host-algos`.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::generate_toolpaths::generate_toolpaths;
use slicer_core::beading::{Beading, BeadingStrategy};
use slicer_core::skeletal_trapezoidation::{
    EdgeType, RibData, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::{Vertex, NO_INDEX};
use slicer_ir::UNITS_PER_MM;

/// Equal-split test strategy (same shape as the one in
/// `arachne_parity_red_is_odd_semantics.rs`).
struct FixedBeadingStrategy;

impl BeadingStrategy for FixedBeadingStrategy {
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
        Beading {
            total_thickness: thickness,
            bead_widths: vec![width; bead_count],
            toolpath_locations: (0..bead_count).map(|i| width * (i as f64 + 0.5)).collect(),
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
        bead_count as f64 * 0.4 * UNITS_PER_MM
    }

    fn type_label(&self) -> &'static str {
        "FixedTestStrategy"
    }

    fn get_split_middle_threshold(&self) -> f64 {
        0.99_f64
    }

    fn get_add_middle_threshold(&self) -> f64 {
        0.99_f64
    }
}

/// Minimal single-central-edge domain with `bead_count = 2` at the PEAK
/// (v0, the higher-R endpoint) — the canonical place a Beading is
/// resolved in `generateJunctions` (SkeletalTrapezoidation.cpp:2029,
/// `getBeading(edge->to, ...)`). v1 is the boundary-side endpoint
/// (lower R) and has no in-band beads of its own.
///
/// The peak's `beading_propagation` entry is hand-built with TWO in-band
/// beads (locations 1.0mm and 2.5mm) so the chain walk emits one line
/// per inset_idx in {0, 1} — the assertion below needs to see
/// `inset_idx > 0` lines to validate `perimeter_index == line.inset_idx`
/// at the inner-wall level.
fn two_bead_single_edge_graph() -> SkeletalTrapezoidationGraph {
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 3.0 * UNITS_PER_MM,
        bead_count: Some(2),
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 10.0 * UNITS_PER_MM,
            y: 0.0,
        },
        distance_to_boundary: 1.0 * UNITS_PER_MM,
        bead_count: None,
        transition_ratio: 0.0,
    };

    let edge0 = STHalfEdge {
        start_vertex: 1,
        twin: 1,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let edge1 = STHalfEdge {
        start_vertex: 0,
        twin: 0,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: false,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };

    let mut graph = SkeletalTrapezoidationGraph {
        vertices: vec![v0, v1],
        edges: vec![edge0, edge1],
        centrality_filtered: true,
        rib: RibData::default(),
        ..Default::default()
    };
    // Peak (v0) beading: three in-band beads within the edge's
    // [1.0mm, 3.0mm] band. The canonical mid-to-outer scan in
    // `generate_junctions` starts at `floor((n-1)/2)` and walks toward
    // 0 to find the outermost in-band bead; with n=3 the scan lands on
    // index 1 (R=1.0mm, in-band) and the second pass then walks
    // `(0..=1).rev() = [1, 0]`, emitting TWO in-band junctions. A 2-bead
    // beading would land the scan on index 0 and the second pass would
    // walk `(0..=0).rev() = [0]`, emitting only one junction — not
    // enough to see an `inset_idx > 0` line in the chain walk. Widths
    // are picked so the two emitted beads' widths are visibly different
    // (catches any pre-fix per-bead-recompute bug). Total thickness
    // matches `2 * peak.R` for self-consistency with the
    // `FixedBeadingStrategy.compute` fallback if the side table were
    // ever absent.
    graph.beading_propagation = vec![
        Some(Beading {
            total_thickness: 6.0 * UNITS_PER_MM,
            bead_widths: vec![1.0 * UNITS_PER_MM, 1.5 * UNITS_PER_MM, 1.0 * UNITS_PER_MM],
            toolpath_locations: vec![1.0 * UNITS_PER_MM, 1.5 * UNITS_PER_MM, 2.5 * UNITS_PER_MM],
            left_over: 0.0,
        }),
        None,
    ];
    graph
}

/// Every junction emitted by `generate_toolpaths` must carry its bead/inset
/// index in `perimeter_index`, matching canonical `generateJunctions`
/// (`junction.perimeter_index = junction_idx`,
/// `SkeletalTrapezoidation.cpp:2064-2077`). FAILS on current code.
#[test]
fn n2_junction_perimeter_index_is_bead_index() {
    let graph = two_bead_single_edge_graph();
    let buckets = generate_toolpaths(&graph, &FixedBeadingStrategy);

    let mut saw_nonzero_inset = false;
    for bucket in &buckets {
        for line in bucket {
            if line.inset_idx > 0 {
                saw_nonzero_inset = true;
            }
            for (j_pos, j) in line.junctions.iter().enumerate() {
                assert_eq!(
                    j.perimeter_index, line.inset_idx,
                    "junction {j_pos} of the inset_idx={} line carries perimeter_index={} — \
                     canonical ExtrusionJunction::perimeter_index is the bead/inset index \
                     assigned at generation time (SkeletalTrapezoidation.cpp:2064-2077), and \
                     connectJunctions' overlap pop-back keys on it \
                     (SkeletalTrapezoidation.cpp:2302-2314). PNP zeroes it and later redefines \
                     it as the junction's position within the line (pipeline.rs:378-390) \
                     (finding N2)",
                    line.inset_idx, j.perimeter_index
                );
            }
        }
    }
    assert!(
        saw_nonzero_inset,
        "fixture emitted no inset > 0 line — the fixture is broken, not the assertion"
    );
}
