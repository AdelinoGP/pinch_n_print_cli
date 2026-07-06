//! Red tests encoding finding **N3** of the second-pass Arachne parity audit
//! (`target/arachne_parity_audit_20260706_020657.md`, §N3).
//!
//! **Finding N3:** PNP's `apply_transitions`
//! (`crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:646-740`)
//! converts every `TransitionMiddle` directly into a single `insert_node`
//! split at the MID position, with `transition_ratio` hard-set to `0.0`
//! everywhere. Canonical OrcaSlicer instead runs `generateTransitioningRibs`
//! (`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:881-915`):
//! `generateTransitionMids` → `filterTransitionMids` (:1007-1076) →
//! `generateAllTransitionEnds` (:1247-1403) → `applyTransitions`
//! (:1487-1543). Each mid spawns TWO ends — a lower end walking backward and
//! an upper end walking forward, spread over
//! `beading_strategy.getTransitioningLength(lower_bead_count)` around the
//! anchor `getTransitionAnchorPos` — and `applyTransitions` inserts nodes at
//! the END positions with `bead_count = lower` or `lower + 1` per
//! `is_lower_end` (:1525-1526). Ends can recursively travel onto successor
//! edges, assigning every traversed node a FRACTIONAL `transition_ratio`,
//! which `generateSegments` (:1712-1721) then uses to interpolate the
//! node's beading between `bead_count` and `bead_count + 1`.
//!
//! Net effect of the gap: PNP snaps the bead count at one point (abrupt
//! width step at every transition) instead of ramping it over the configured
//! `wall_transition_length`.
//!
//! Host-only: gated behind `host-algos`.

#![cfg(feature = "host-algos")]

use slicer_core::skeletal_trapezoidation::{
    apply_transitions, EdgeType, RibData, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
    TransitionMiddle,
};
use slicer_core::voronoi::{Vertex, NO_INDEX};
use slicer_ir::UNITS_PER_MM;

fn vertex(x_units: f64, r_units: f64, bead_count: Option<u32>) -> STVertex {
    STVertex {
        position: Vertex { x: x_units, y: 0.0 },
        distance_to_boundary: r_units,
        bead_count,
        transition_ratio: 0.0,
    }
}

/// A single central twin-pair edge along +x: v0 at the origin
/// (R = 1 mm, bead_count 1) to v1 at 10 mm (R = 3 mm, bead_count 2), with a
/// `TransitionMiddle` at pos 0.5 (`lower_bead_count = 1`,
/// `mid_r = 2 mm`) on the forward half-edge.
fn single_edge_with_mid() -> SkeletalTrapezoidationGraph {
    let v0 = vertex(0.0, 1.0 * UNITS_PER_MM, Some(1));
    let v1 = vertex(10.0 * UNITS_PER_MM, 3.0 * UNITS_PER_MM, Some(2));

    let mut edge0 = STHalfEdge {
        start_vertex: 0,
        twin: 1,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    edge0.transition_mids.push(TransitionMiddle {
        pos: 0.5,
        lower_bead_count: 1,
        mid_r: 2.0 * UNITS_PER_MM,
    });
    let edge1 = STHalfEdge {
        start_vertex: 1,
        twin: 0,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: true,
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

/// N3 core — a transition must NOT collapse to a single split exactly at the
/// mid position.
///
/// Canonically the mid spawns a lower end at `mid − anchor·L` and an upper
/// end at `mid + (1 − anchor)·L` (`SkeletalTrapezoidation.cpp:1257-1263`,
/// `L = getTransitioningLength(1)`, strictly positive), so `applyTransitions`
/// never inserts a spine node exactly at the mid carrying the LOWER bead
/// count — insertions land at the two end stations (bead counts 1 and 2,
/// :1525-1526), or snap to existing vertices when an end runs off the edge.
///
/// PNP inserts exactly one new spine vertex, exactly at the mid position,
/// with `bead_count = lower` (propagation.rs:690-739). This test asserts
/// that specific divergent signature is absent. FAILS on current code.
#[test]
fn n3_apply_transitions_creates_lower_and_upper_end_splits() {
    let mut graph = single_edge_with_mid();
    let pre_vertex_count = graph.vertices.len();
    let mid_x_units = 5.0 * UNITS_PER_MM; // pos 0.5 of the 10mm edge

    apply_transitions(&mut graph);

    // New spine vertices = appended vertices that are not boundary/rib-foot
    // sentinels (distance_to_boundary > 0).
    let new_spine: Vec<&STVertex> = graph.vertices[pre_vertex_count..]
        .iter()
        .filter(|v| v.distance_to_boundary > 0.0)
        .collect();

    let tol_units = 0.01 * UNITS_PER_MM;
    let single_split_at_mid_with_lower_count = new_spine.len() == 1
        && (new_spine[0].position.x - mid_x_units).abs() <= tol_units
        && new_spine[0].bead_count == Some(1);

    assert!(
        !single_split_at_mid_with_lower_count,
        "apply_transitions produced exactly one new spine vertex, exactly at the transition MID \
         (x = {:.0} units) and carrying only the lower bead count Some(1) — the canonical \
         algorithm converts the mid into a LOWER and an UPPER transition end straddling the mid \
         by the configured transition length (SkeletalTrapezoidation.cpp:1247-1403), and inserts \
         nodes at the END positions with bead counts {{1, 2}} \
         (SkeletalTrapezoidation.cpp:1525-1526). Splitting once at the mid produces an abrupt \
         width step instead of a ramp (finding N3)",
        mid_x_units
    );
}

/// N3, ratio propagation — a transition end that spills past a graph vertex
/// must leave that vertex with a FRACTIONAL `transition_ratio`.
///
/// Fixture: a two-edge central chain where edge A is only 0.2 mm long with
/// the transition mid at its middle — so the upper transition end
/// necessarily travels past the shared vertex v1 onto edge B for any
/// plausible transition length (the registered `wall_transition_length`
/// default is 0.4 mm). Canonically `generateTransitionEnd`'s recursion
/// (`SkeletalTrapezoidation.cpp:1331-1371`) assigns v1 an interpolated
/// `transition_ratio` strictly between 0 and 1 (and `bead_count = lower`),
/// which is what makes `generateSegments` (:1712-1721) blend the beading
/// there. PNP never writes a fractional ratio anywhere
/// (propagation.rs:714/723 and insert_node:331 always write `0.0`).
/// FAILS on current code.
#[test]
fn n3_transition_spilling_past_vertex_sets_fractional_ratio() {
    // Edge A: v0 (x=0, R=1.00mm, bc 1) -> v1 (x=0.2mm, R=1.10mm, bc 2).
    // Edge B: v1 -> v2 (x=5.2mm, R=3.60mm, bc 2). Radius slope 0.5 on both.
    let v0 = vertex(0.0, 1.0 * UNITS_PER_MM, Some(1));
    let v1 = vertex(0.2 * UNITS_PER_MM, 1.1 * UNITS_PER_MM, Some(2));
    let v2 = vertex(5.2 * UNITS_PER_MM, 3.6 * UNITS_PER_MM, Some(2));

    // Forward chain a (v0->v1) then b (v1->v2); reverse twins a' and b'.
    let mut edge_a = STHalfEdge {
        start_vertex: 0,
        twin: 2,
        next: 1,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    edge_a.transition_mids.push(TransitionMiddle {
        pos: 0.5,
        lower_bead_count: 1,
        mid_r: 1.05 * UNITS_PER_MM,
    });
    let edge_b = STHalfEdge {
        start_vertex: 1,
        twin: 3,
        next: NO_INDEX,
        prev: 0,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let edge_a_rev = STHalfEdge {
        start_vertex: 1,
        twin: 0,
        next: NO_INDEX,
        prev: 3,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let edge_b_rev = STHalfEdge {
        start_vertex: 2,
        twin: 1,
        next: 2,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };

    let mut graph = SkeletalTrapezoidationGraph {
        vertices: vec![v0, v1, v2],
        edges: vec![edge_a, edge_b, edge_a_rev, edge_b_rev],
        centrality_filtered: true,
        rib: RibData::default(),
    };

    apply_transitions(&mut graph);

    let ratio = graph.vertices[1].transition_ratio;
    assert!(
        ratio > 0.0 && ratio < 1.0,
        "shared vertex v1 has transition_ratio = {ratio} after apply_transitions; the transition \
         mid sits 0.1 mm before v1 while the configured transition length (default 0.4 mm) \
         extends past it, so the canonical upper-end walk must traverse v1 and assign it a \
         fractional transition_ratio (SkeletalTrapezoidation.cpp:1331-1371) — the value \
         generateSegments uses to interpolate the beading between 1 and 2 beads \
         (SkeletalTrapezoidation.cpp:1712-1721). PNP never writes a fractional ratio \
         (finding N3), so widths step abruptly instead of ramping"
    );
}
