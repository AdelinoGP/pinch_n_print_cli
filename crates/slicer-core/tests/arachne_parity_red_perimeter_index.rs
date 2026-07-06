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
}

/// Minimal single-central-edge domain with `bead_count = 2` at the "to"
/// vertex (same topology as the is_odd fixture: central forward half-edge,
/// non-central twin, so exactly one domain walk emits).
fn two_bead_single_edge_graph() -> SkeletalTrapezoidationGraph {
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 3.0 * UNITS_PER_MM,
        bead_count: None,
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 10.0 * UNITS_PER_MM,
            y: 0.0,
        },
        distance_to_boundary: 1.0 * UNITS_PER_MM,
        bead_count: Some(2),
        transition_ratio: 0.0,
    };

    let edge0 = STHalfEdge {
        start_vertex: 0,
        twin: 1,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let edge1 = STHalfEdge {
        start_vertex: 1,
        twin: 0,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: false,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };

    SkeletalTrapezoidationGraph {
        vertices: vec![v0, v1],
        edges: vec![edge0, edge1],
        centrality_filtered: true,
        rib: RibData::default(),
    }
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
