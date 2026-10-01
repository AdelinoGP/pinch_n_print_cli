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
//! The fixture is a minimal canonical quad ring whose spine nodes carry
//! `bead_count = 4` (an EVEN count): canonically NO emitted line may be
//! `is_odd`, because there is no centerline bead.
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

/// Minimal canonical quad ring around a short strip (0.2 mm long, 0.6 mm
/// wide): one flat central spine A -> B at R = 0.3 mm with `bead_count = 4`
/// (EVEN) at both ends, and four quads, each starting and ending on the
/// outline, mirroring the chains `from_polygons` builds:
///
/// - bottom: `b0 -> A` (rib up), `A -> B` (spine), `B -> b1` (rib down);
/// - right end: `b1 -> B`, `B -> b2`;
/// - top: `b2 -> B`, `B -> A` (spine twin), `A -> b3`;
/// - left end: `b3 -> A`, `A -> b0`.
///
/// Each quad's last edge is the twin of the next quad's first edge, so one
/// `connectJunctions` domain walk closes the ring. The four rising ribs
/// carry junction fans for beads 0 and 1 (the in-band half of the 4-bead
/// beading), so every emitted line is a short (< 2 mm) loop fragment of a
/// real wall — eligible for `remove_small_lines` iff (mis)labelled `is_odd`.
fn even_bead_strip_ring_graph() -> SkeletalTrapezoidationGraph {
    let mm = UNITS_PER_MM;
    let (len, half_width) = (0.2 * mm, 0.3 * mm);
    let node = |x: f64, y: f64, r: f64, bead_count: Option<u32>| STVertex {
        position: Vertex { x, y },
        distance_to_boundary: r,
        bead_count,
        transition_ratio: 0.0,
    };
    let vertices = vec![
        node(0.0, half_width, half_width, Some(4)), // 0: A
        node(len, half_width, half_width, Some(4)), // 1: B
        node(0.0, 0.0, 0.0, None),                  // 2: b0
        node(len, 0.0, 0.0, None),                  // 3: b1
        node(len, 2.0 * half_width, 0.0, None),     // 4: b2
        node(0.0, 2.0 * half_width, 0.0, None),     // 5: b3
    ];
    let half_edge =
        |start_vertex: usize, twin: usize, prev: usize, next: usize, spine: bool| STHalfEdge {
            start_vertex,
            twin,
            prev,
            next,
            central: spine,
            edge_type: if spine {
                EdgeType::NORMAL
            } else {
                EdgeType::EXTRA_VD
            },
            ..STHalfEdge::default()
        };
    let none = NO_INDEX;
    let edges = vec![
        half_edge(2, 9, none, 1, false), // 0: b0 -> A
        half_edge(0, 5, 0, 2, true),     // 1: A -> B
        half_edge(1, 3, 1, none, false), // 2: B -> b1
        half_edge(3, 2, none, 4, false), // 3: b1 -> B
        half_edge(1, 6, 3, none, false), // 4: B -> b2
        half_edge(1, 1, 6, 7, true),     // 5: B -> A
        half_edge(4, 4, none, 5, false), // 6: b2 -> B
        half_edge(0, 8, 5, none, false), // 7: A -> b3
        half_edge(5, 7, none, 9, false), // 8: b3 -> A
        half_edge(0, 0, 8, none, false), // 9: A -> b0
    ];

    SkeletalTrapezoidationGraph {
        vertices,
        edges,
        centrality_filtered: true,
        rib: RibData::default(),
        ..Default::default()
    }
}

/// N4 core: with an EVEN bead count (4), no emitted line is a centerline
/// gap-fill line, so canonically every line must have `is_odd == false`
/// (`ExtrusionLine.hpp:62-70`; `SkeletalTrapezoidation.cpp:2344-2354`
/// requires `bead_count % 2 == 1`). PNP marks the inset-1 line
/// `is_odd = true` because `1 % 2 == 1` (`generate_toolpaths.rs:632`).
/// FAILS on current code.
#[test]
fn n4_even_bead_count_lines_are_never_marked_odd() {
    let graph = even_bead_strip_ring_graph();
    let buckets = generate_toolpaths(&graph, &FixedBeadingStrategy);

    let mut saw_any_line = false;
    for bucket in &buckets {
        for line in bucket {
            saw_any_line = true;
            assert!(
                !line.is_odd,
                "line with inset_idx {} is marked is_odd = true, but the region's bead count is \
                 4 (even) — canonical is_odd means \"centerline bead of an ODD bead count\" \
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
    let graph = even_bead_strip_ring_graph();
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
