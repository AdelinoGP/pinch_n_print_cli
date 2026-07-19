//! Red tests encoding finding **N4** of the second-pass Arachne parity audit
//! (`target/arachne_parity_audit_20260706_020657.md`, §N4).
//!
//! **Finding N4:** PNP sets `ExtrusionLine::is_odd = bead_idx % 2 == 1`
//! (`crates/slicer-core/src/arachne/generate_toolpaths.rs:632`) — i.e. "this
//! is an odd-INDEXED inset". Canonical OrcaSlicer semantics
//! (`OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.hpp:62-70`)
//! are "this is the CENTERLINE bead of a region with an odd bead count — a
//! gap-fill line with no companion on the other side, not a closed loop".
//! Canonically it is computed per segment in `connectJunctions`
//! (`SkeletalTrapezoidation.cpp:2344-2354`): `bead_count % 2 == 1`,
//! `transition_ratio == 0`, innermost junction of the fan, endpoints within
//! 0.005 mm of the quad's peak node.
//!
//! Failure mode: with PNP's definition every 2nd, 4th, ... wall is
//! classified as gap fill. `remove_small_lines`
//! (`crates/slicer-core/src/arachne/remove_small.rs:57`) only removes
//! `is_odd && !is_closed` lines, so short open fragments of REAL inner walls
//! get silently deleted; the stitcher groups by `is_odd`
//! (`stitch.rs:83`), so mislabelled walls can't join their peers; and the
//! flag is forwarded verbatim across the host boundary
//! (`slicer-wasm-host/src/host.rs:1818`).
//!
//! The fixture is a minimal single-central-edge graph with `bead_count = 2`
//! (an EVEN count): canonically NO emitted line may be `is_odd`, because
//! there is no centerline bead.
//!
//! Host-only: gated behind `host-algos`.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::generate_toolpaths::generate_toolpaths;
use slicer_core::arachne::remove_small_lines;
use slicer_core::beading::{Beading, BeadingStrategy};
use slicer_core::skeletal_trapezoidation::{
    EdgeType, RibData, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::{Vertex, NO_INDEX};
use slicer_ir::UNITS_PER_MM;

/// Deterministic test strategy: splits `thickness` into `bead_count`
/// equal-width beads centered in their own slice. Mirrors the shape of the
/// unit-test strategy in `generate_toolpaths.rs`'s own `#[cfg(test)]` module.
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

/// Minimal single-central-edge domain: v0 (R = 3 mm) -> v1 (R = 1 mm,
/// `bead_count = Some(2)`), edge 0 central with a non-central twin (edge 1)
/// so exactly one domain walk emits. The edge is deliberately SHORT (1 mm)
/// so the emitted per-bead lines are short open polylines — eligible for
/// `remove_small_lines` removal iff (mis)labelled `is_odd`.
fn two_bead_single_edge_graph() -> SkeletalTrapezoidationGraph {
    // Minimal single-central-edge domain with a TRANSITION (the peak and
    // boundary carry different `bead_count`s) so Step 1's `generate_junctions`
    // — which resolves the beading at the PEAK and skips no-transition edges
    // per canonical `SkeletalTrapezoidation.cpp:2024-2027` — actually emits
    // junctions for this edge. The peak carries `bead_count = Some(4)` (an
    // even count, larger than 2, so the peak's beading has enough beads that
    // the in-band emission step (`:2064-2077`) actually emits 2 junctions
    // for the 1mm edge — both insets 0 and 1 are needed for AC-3 to have
    // a meaningful "inset-1 line survives `remove_small_lines`" assertion).
    // The canonical `is_odd` rule (`SkeletalTrapezoidation.cpp:2344-2354`)
    // requires `bead_count % 2 == 1` at the PEAK for a centerline gap-fill
    // segment, so an even-count peak guarantees no line is `is_odd` — exactly
    // what AC-2 / AC-3 assert. The boundary side carries `bead_count =
    // Some(1)` (odd) to ensure a real transition (peak != boundary); the
    // transition itself is not the subject of either test, only the peak's
    // `is_odd` truth is.
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 4.0 * UNITS_PER_MM,
        bead_count: Some(4),
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 3.0 * UNITS_PER_MM,
            y: 0.0,
        },
        distance_to_boundary: 1.0 * UNITS_PER_MM,
        bead_count: Some(1),
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
        ..Default::default()
    }
}

/// N4 core: with an EVEN bead count (2), no emitted line is a centerline
/// gap-fill line, so canonically every line must have `is_odd == false`
/// (`ExtrusionLine.hpp:62-70`; `SkeletalTrapezoidation.cpp:2344-2354`
/// requires `bead_count % 2 == 1`). PNP marks the inset-1 line
/// `is_odd = true` because `1 % 2 == 1` (`generate_toolpaths.rs:632`).
/// FAILS on current code.
#[test]
fn n4_even_bead_count_lines_are_never_marked_odd() {
    let graph = two_bead_single_edge_graph();
    let buckets = generate_toolpaths(&graph, &FixedBeadingStrategy);

    let mut saw_any_line = false;
    for bucket in &buckets {
        for line in bucket {
            saw_any_line = true;
            assert!(
                !line.is_odd,
                "line with inset_idx {} is marked is_odd = true, but the region's bead count is \
                 2 (even) — canonical is_odd means \"centerline bead of an ODD bead count\" \
                 (ExtrusionLine.hpp:62-70), never \"odd-indexed inset\" (finding N4)",
                line.inset_idx
            );
        }
    }
    assert!(
        saw_any_line,
        "fixture emitted no lines at all — the fixture is broken, not the assertion"
    );
}

/// N4 consequence: a short open fragment of the SECOND wall (inset 1, even
/// bead count) must survive `remove_small_lines`, whose only eligibility
/// gate is `is_odd && !is_closed` (`remove_small.rs:57`, mirroring
/// `WallToolPaths.cpp:838-856`). Under PNP's inset-parity mislabelling the
/// inset-1 line is treated as gap fill and silently deleted. FAILS on
/// current code.
#[test]
fn n4_even_inner_wall_survives_remove_small_lines() {
    let graph = two_bead_single_edge_graph();
    let buckets = generate_toolpaths(&graph, &FixedBeadingStrategy);
    let lines: Vec<_> = buckets.into_iter().flatten().collect();

    let inset1_before = lines.iter().filter(|l| l.inset_idx == 1).count();
    assert!(
        inset1_before > 0,
        "fixture emitted no inset-1 line — the fixture is broken, not the assertion"
    );

    // Threshold 0.5 * 4.0 = 2.0 mm; the fixture's per-bead lines are < 1 mm
    // long, so anything (mis)labelled odd+open at inset 1 gets removed.
    let survivors = remove_small_lines(lines, 0.5, 4.0, false, false);
    let inset1_after = survivors.iter().filter(|l| l.inset_idx == 1).count();

    assert_eq!(
        inset1_after,
        inset1_before,
        "remove_small_lines deleted {} of {} inset-1 (second wall) line(s): the line was \
         classified is_odd = true by inset parity (generate_toolpaths.rs:632) even though the \
         region's bead count is even — canonical is_odd marks only the odd-count centerline \
         gap-fill bead (finding N4), so real walls must never be eligible for removal",
        inset1_before - inset1_after,
        inset1_before
    );
}
