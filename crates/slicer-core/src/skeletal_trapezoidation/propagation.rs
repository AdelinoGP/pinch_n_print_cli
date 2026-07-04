// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
// (`propagateBeadingsUpward`, `propagateBeadingsDownward`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Bead-count propagation + transition-region marking (T-222, packet 112
//! Step 3 of the M2 Arachne port).
//!
//! # Honesty note (no OrcaSlicer oracle; intentional semantic adaptation)
//!
//! Like [`super::centrality`] and [`super::bead_count`], this module does not
//! claim numeric parity with OrcaSlicer. Two corrections from the upstream
//! source, confirmed by direct reading of `SkeletalTrapezoidation.cpp`:
//!
//! 1. **`propagateBeadingsUpward`/`propagateBeadingsDownward` operate on a
//!    "quad" topology this crate's graph does not have.** Upstream fills in
//!    a per-node `Beading` (the concrete bead-width/toolpath-location result
//!    of [`crate::beading`]) by walking `upward_quad_mids` and
//!    `getQuadMaxRedgeTo`'s rib/non-rib edge distinction. This crate's graph
//!    (see [`super::graph`]'s own doc comment) is a direct 1:1 wrap of the
//!    raw `boostvoronoi` diagram, with no quad decomposition and no
//!    `Beading`-per-node concept at the graph-edge level — bead widths are
//!    computed later, from the final integer `bead_count`, not stored here.
//! 2. **Neither upstream function sets `TransitionMiddle`/`TransitionEnd`.**
//!    Those markers are placed by an entirely separate upstream pass
//!    (`generateTransitionMids` → `generateAllTransitionEnds` →
//!    `applyTransitions`), which runs *before* `generateSegments` ever calls
//!    `propagateBeadingsUpward`/`Downward`, and depends on a fractional
//!    `transition_ratio` field this crate's graph does not carry. This
//!    crate has no separate transition-mid pass at all.
//!
//! Rather than porting either upstream mechanism byte-for-byte onto a
//! topology that does not support it, `propagate_beadings_upward`/
//! `propagate_beadings_downward` below implement a **from-first-principles
//! adaptation** true to the *name and intent* of the two upstream functions:
//!
//! - **Propagation**: each function fills [`super::graph::STHalfEdge::bead_count`]
//!   gaps (`None`) on central edges from an already-resolved central
//!   neighbor reachable via the half-edge topology (a central edge sharing a
//!   vertex, excluding the edge's own twin) — mirroring upstream's "only
//!   fills genuinely unbeaded edges, never overrides a real assignment"
//!   contract. `propagate_beadings_upward` walks central edges in ascending
//!   `r_min` order (shallow → deep, i.e. toward higher radius / thicker,
//!   matching upstream's outer→tip walk direction); `propagate_beadings_downward`
//!   walks in descending `r_max` order (the reverse). Since
//!   [`super::bead_count::assign_bead_counts`] already assigns every central
//!   edge a `bead_count`, gap-filling is a no-op in this packet's own
//!   pipeline today (there are no gaps to fill) — it exists for robustness
//!   against a graph built or edited some other way; see
//!   `propagation_fills_gap_from_central_neighbor` in
//!   `crates/slicer-core/tests/propagation.rs` for a hand-built graph that
//!   actually exercises it.
//! - **Transition marking**: after gap-filling, both functions run the same
//!   final step — recomputing [`super::graph::STHalfEdge::is_transition_middle`]/
//!   [`super::graph::STHalfEdge::is_transition_end`] from scratch (idempotent,
//!   not accumulated across calls) over the *final* `bead_count` state. A
//!   central edge is a transition **boundary** when a central neighbor at
//!   exactly one of its two topological "sides" (its `start_vertex`, or its
//!   "to" vertex resolved via the twin convention — see
//!   [`super::graph`]'s doc comment) carries a different `Some(bead_count)`.
//!   An edge whose *both* sides differ from its own count sits inside a
//!   bead-count ramp (`is_transition_middle`); an edge with only one
//!   differing side sits at the ramp's boundary (`is_transition_end`).
//!   Marking depends only on the final `bead_count` values, never on which
//!   direction filled them in, so the two functions' marking output is
//!   identical regardless of call order — a deliberate design choice, not an
//!   oversight.
//!
//! `crates/slicer-core/tests/propagation.rs`'s three required fixtures are
//! **self-captured regression baselines**: they lock in this
//! implementation's own output for regression purposes only. They are not
//! derived from, and must not be described as, OrcaSlicer ground truth.
//!
//! Deterministic (index-ordered traversal; `f64` comparisons only ever drive
//! a sort order, never a hash-map key, and fall back to `Ordering::Equal`
//! rather than assuming a `NaN` can't occur) and panic-free.

use std::cmp::Ordering;

use super::graph::SkeletalTrapezoidationGraph;
use crate::voronoi::NO_INDEX;

/// Resolves a half-edge's "to" vertex index via its twin's `start_vertex`,
/// matching [`super::graph`]'s own convention (see that module's doc comment,
/// and [`super::centrality`]'s identically-named private helper — duplicated
/// here rather than shared, matching this packet's existing per-module
/// convention of small self-contained helpers). Returns [`NO_INDEX`] if
/// unresolvable (missing/out-of-range twin).
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

/// Central half-edges starting at `vertex_idx`, excluding `exclude_a` and
/// `exclude_b` (conventionally an edge's own index and its twin — a
/// half-edge's own twin starts at that edge's "to" vertex, so it can appear
/// in this enumeration when `vertex_idx` is the "to" side; excluding it
/// avoids treating an edge as its own neighbor). Returns edge indices in
/// ascending order (the iteration order of `graph.edges`).
fn central_neighbors_at(
    graph: &SkeletalTrapezoidationGraph,
    vertex_idx: usize,
    exclude_a: usize,
    exclude_b: usize,
) -> Vec<usize> {
    if vertex_idx == NO_INDEX {
        return Vec::new();
    }
    graph
        .edges
        .iter()
        .enumerate()
        .filter(|(idx, e)| {
            *idx != exclude_a && *idx != exclude_b && e.central && e.start_vertex == vertex_idx
        })
        .map(|(idx, _)| idx)
        .collect()
}

/// Index order in which [`propagate_beadings_upward`] (and, reversed, its
/// `_downward` counterpart) visits central edges. `ascending_by_r_min ==
/// true` sorts by `r_min` ascending (ties by index ascending); `false` sorts
/// by `r_max` descending (ties by index ascending). Both orders are total
/// (derived via `partial_cmp` with an `Ordering::Equal` fallback, never a
/// panic) and index-ordered on ties, so the result never depends on `f64`
/// hashing or unstable-sort quirks.
fn central_edge_order(graph: &SkeletalTrapezoidationGraph, ascending_by_r_min: bool) -> Vec<usize> {
    let mut order: Vec<usize> = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.central)
        .map(|(idx, _)| idx)
        .collect();

    order.sort_by(|&a, &b| {
        let key = if ascending_by_r_min {
            graph.edges[a]
                .r_min
                .partial_cmp(&graph.edges[b].r_min)
                .unwrap_or(Ordering::Equal)
        } else {
            graph.edges[b]
                .r_max
                .partial_cmp(&graph.edges[a].r_max)
                .unwrap_or(Ordering::Equal)
        };
        key.then(a.cmp(&b))
    });
    order
}

/// Fills [`super::graph::STHalfEdge::bead_count`] gaps (`None`) on central
/// edges visited in `order`, from an already-resolved central neighbor (per
/// [`central_neighbors_at`], checked at both the edge's `start_vertex` and
/// its "to" vertex) — see this module's doc comment. Never overwrites an
/// existing `Some(_)`. Ties among multiple resolvable neighbors break on
/// lowest edge index (deterministic, no float-keyed map involved).
fn fill_gaps(graph: &mut SkeletalTrapezoidationGraph, order: &[usize]) {
    for &idx in order {
        if graph.edges[idx].bead_count.is_some() {
            continue;
        }
        let (start_v, twin) = {
            let e = &graph.edges[idx];
            (e.start_vertex, e.twin)
        };
        let to_v = resolve_to_vertex(graph, idx);

        let mut candidates = central_neighbors_at(graph, start_v, idx, twin);
        candidates.extend(central_neighbors_at(graph, to_v, idx, twin));
        candidates.retain(|&c| graph.edges[c].bead_count.is_some());
        candidates.sort_unstable();

        if let Some(&source) = candidates.first() {
            graph.edges[idx].bead_count = graph.edges[source].bead_count;
        }
    }
}

/// Recomputes [`super::graph::STHalfEdge::is_transition_middle`]/
/// `is_transition_end` for every edge from scratch (idempotent — not
/// accumulated across repeated calls), from the graph's current
/// `bead_count` state. See this module's doc comment for the
/// boundary-vs-interior rule. Non-central edges, and central edges with no
/// `bead_count` assigned, are always left/reset to `false` on both flags.
fn mark_transitions(graph: &mut SkeletalTrapezoidationGraph) {
    let edge_count = graph.edges.len();
    let mut end_flags = vec![false; edge_count];
    let mut middle_flags = vec![false; edge_count];

    for idx in 0..edge_count {
        let edge = &graph.edges[idx];
        if !edge.central {
            continue;
        }
        let Some(bc) = edge.bead_count else {
            continue;
        };
        let start_v = edge.start_vertex;
        let twin = edge.twin;
        let to_v = resolve_to_vertex(graph, idx);

        let start_side = central_neighbors_at(graph, start_v, idx, twin);
        let to_side = central_neighbors_at(graph, to_v, idx, twin);

        let differs = |side: &[usize]| {
            side.iter()
                .any(|&n| matches!(graph.edges[n].bead_count, Some(nb) if nb != bc))
        };
        let start_differs = differs(&start_side);
        let to_differs = differs(&to_side);

        if start_differs && to_differs {
            middle_flags[idx] = true;
        } else if start_differs || to_differs {
            end_flags[idx] = true;
        }
    }

    for (idx, edge) in graph.edges.iter_mut().enumerate() {
        edge.is_transition_end = end_flags[idx];
        edge.is_transition_middle = middle_flags[idx];
    }
}

/// Propagates [`super::graph::STHalfEdge::bead_count`] "upward" (toward
/// higher radius / thicker central edges) and marks bead-count transition
/// boundaries.
///
/// Walks central edges in ascending-`r_min` order, filling any `bead_count
/// == None` gap from an already-visited central neighbor (never overwriting
/// a real assignment), then recomputes `is_transition_middle`/
/// `is_transition_end` for every edge from the final `bead_count` state.
/// Deterministic and panic-free. See this module's doc comment for why this
/// is a from-first-principles adaptation of OrcaSlicer's
/// `propagateBeadingsUpward`, not a literal port.
pub fn propagate_beadings_upward(graph: &mut SkeletalTrapezoidationGraph) {
    let order = central_edge_order(graph, true);
    fill_gaps(graph, &order);
    mark_transitions(graph);
}

/// Propagates [`super::graph::STHalfEdge::bead_count`] "downward" (the
/// reverse direction of [`propagate_beadings_upward`]: toward lower radius /
/// thinner central edges) and marks bead-count transition boundaries.
///
/// Walks central edges in descending-`r_max` order, filling any
/// `bead_count == None` gap from an already-visited central neighbor (never
/// overwriting a real assignment), then recomputes `is_transition_middle`/
/// `is_transition_end` for every edge from the final `bead_count` state —
/// identical final marking logic to [`propagate_beadings_upward`] (marking
/// depends only on the final `bead_count` values, not on traversal
/// direction; see this module's doc comment). Deterministic and panic-free.
pub fn propagate_beadings_downward(graph: &mut SkeletalTrapezoidationGraph) {
    let order = central_edge_order(graph, false);
    fill_gaps(graph, &order);
    mark_transitions(graph);
}
