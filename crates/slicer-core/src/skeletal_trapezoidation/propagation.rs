// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path:
//   src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
//     (`generateTransitionMids` L925-994,
//      `applyTransitions` L1487-1543,
//      `propagateBeadingsUpward` L1800-1826,
//      `propagateBeadingsDownward` L1833-1899)
//   and
//     src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h
//     (`getTransitionThickness`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Bead-count propagation + transition-region marking (T-222, packet 112
//! Step 3 / packet 113b Step 4 of the M2 Arachne port).
//!
//! Faithful port of `generateTransitionMids` (L925), `applyTransitions`
//! (L1487), `propagateBeadingsUpward` (L1800), and `propagateBeadingsDownward`
//! (L1833) from
//! `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp`.
//!
//! The pass order matches upstream:
//! `updateBeadCount` → `filterNoncentralRegions` → `generateTransitionMids` →
//! `generateAllTransitionEnds` → `applyTransitions` → `generateExtraRibs` →
//! `generateSegments` → `propagateBeadingsUpward` →
//! `propagateBeadingsDownward`.
//!
//! Deterministic (index-ordered traversal; `f64` comparisons only ever drive
//! a sort order, never a hash-map key, and fall back to `Ordering::Equal`
//! rather than assuming a `NaN` can't occur) and panic-free.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use super::graph::{STVertex, SkeletalTrapezoidationGraph, TransitionMiddle};
use crate::beading::BeadingStrategy;
use crate::voronoi::NO_INDEX;

/// Snap distance used by `apply_transitions` when deciding whether a
/// transition-end position coincides with an existing vertex. Expressed as a
/// fraction of the edge's length (upstream uses an absolute tolerance; we keep
/// it proportional because the graph's unit scale varies across fixtures).
const SNAP_FRAC: f64 = 1e-6;

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

/// Euclidean length of edge `edge_idx` in the scaled-integer unit space.
fn edge_length(graph: &SkeletalTrapezoidationGraph, edge_idx: usize) -> f64 {
    let Some(edge) = graph.edges.get(edge_idx) else {
        return 0.0;
    };
    let Some(start_v) = graph.vertices.get(edge.start_vertex) else {
        return 0.0;
    };
    let to_idx = resolve_to_vertex(graph, edge_idx);
    let Some(end_v) = graph.vertices.get(to_idx) else {
        return 0.0;
    };
    let dx = end_v.position.x - start_v.position.x;
    let dy = end_v.position.y - start_v.position.y;
    (dx * dx + dy * dy).sqrt()
}

/// Linearly interpolates between two vertex positions by parameter `t`
/// (`t = 0.0` returns `start`, `t = 1.0` returns `end`).
fn interpolate_position(
    a: crate::voronoi::Vertex,
    b: crate::voronoi::Vertex,
    t: f64,
) -> crate::voronoi::Vertex {
    crate::voronoi::Vertex {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
    }
}

/// Computes the interpolated radius at position `pos` along edge `edge_idx`.
fn _radius_at(graph: &SkeletalTrapezoidationGraph, edge_idx: usize, pos: f64) -> f64 {
    let edge = match graph.edges.get(edge_idx) {
        Some(e) => e,
        None => return 0.0,
    };
    let start_r = graph
        .vertices
        .get(edge.start_vertex)
        .map(|v| v.distance_to_boundary)
        .unwrap_or(0.0);
    let to_idx = resolve_to_vertex(graph, edge_idx);
    let end_r = graph
        .vertices
        .get(to_idx)
        .map(|v| v.distance_to_boundary)
        .unwrap_or(start_r);
    start_r + (end_r - start_r) * pos
}

/// Returns all edges whose endpoints' `distance_to_boundary` increase along the
/// edge direction (`start_R < end_R`) and which are central. This is the set
/// upstream calls `upward_quad_mids` in the propagation passes.
///
/// Tie order is index-ascending (deterministic).
fn upward_central_edges(graph: &SkeletalTrapezoidationGraph) -> Vec<usize> {
    let mut order: Vec<usize> = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.central)
        .filter(|(idx, e)| {
            let start_r = graph
                .vertices
                .get(e.start_vertex)
                .map(|v| v.distance_to_boundary)
                .unwrap_or(f64::INFINITY);
            let to_idx = resolve_to_vertex(graph, *idx);
            let end_r = graph
                .vertices
                .get(to_idx)
                .map(|v| v.distance_to_boundary)
                .unwrap_or(f64::NEG_INFINITY);
            start_r < end_r
        })
        .map(|(idx, _)| idx)
        .collect();
    // Sort by descending R so the "upward" walk is from high radius down to
    // low radius in forward order — upstream iterates this list in reverse for
    // upward propagation and forward for downward propagation.
    order.sort_by(|&a, &b| {
        let ra = graph.edges[a].r_max;
        let rb = graph.edges[b].r_max;
        rb.partial_cmp(&ra)
            .unwrap_or(Ordering::Equal)
            .then(a.cmp(&b))
    });
    order
}

/// Generates transition-middle annotations for every upward central edge whose
/// bead count increases from `from` to `to`.
///
/// For each step from `start_bead_count` to `end_bead_count - 1`, computes
/// `mid_R = strategy.get_transition_thickness(lower_bead_count) / 2` and the
/// corresponding linear position `mid_pos = edge_size * (mid_R - start_R) /
/// (end_R - start_R)`. The annotation is stored on the edge's
/// [`super::graph::STHalfEdge::transition_mids`] vector.
pub fn generate_transition_mids(
    graph: &mut SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) {
    let n_edges = graph.edges.len();
    for edge_idx in 0..n_edges {
        let edge = match graph.edges.get(edge_idx) {
            Some(e) => e.clone(),
            None => continue,
        };
        if !edge.central {
            continue;
        }
        let start_v = edge.start_vertex;
        let to_v = resolve_to_vertex(graph, edge_idx);
        let Some(start) = graph.vertices.get(start_v).cloned() else {
            continue;
        };
        let Some(end) = graph.vertices.get(to_v).cloned() else {
            continue;
        };
        let start_r = start.distance_to_boundary;
        let end_r = end.distance_to_boundary;
        if start_r >= end_r {
            continue;
        }
        let Some(start_bc) = start.bead_count else {
            continue;
        };
        let Some(end_bc) = end.bead_count else {
            continue;
        };
        if end_bc <= start_bc {
            continue;
        }

        let edge_size = edge_length(graph, edge_idx);
        if edge_size <= 0.0 || !edge_size.is_finite() {
            continue;
        }

        for lower_bc in start_bc..end_bc {
            let mid_r = strategy.get_transition_thickness(lower_bc as usize) / 2.0;
            let t = (mid_r - start_r) / (end_r - start_r);
            let pos = t.clamp(0.0, 1.0);
            graph.edges[edge_idx]
                .transition_mids
                .push(TransitionMiddle {
                    pos,
                    lower_bead_count: lower_bc,
                    mid_r,
                });
        }
    }
}

/// A transition end: position along a half-edge plus the bead count that should
/// be assigned to the inserted/split vertex at this position.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TransitionEnd {
    pos: f64,
    bead_count: u32,
    /// Radius at which this transition end occurs; used to compute the
    /// `transition_ratio` of a newly inserted vertex.
    mid_r: f64,
}

/// Splits edge `edge_idx` at position `pos` (fraction of edge length) and
/// inserts a new vertex with the given `bead_count` and `transition_ratio`.
///
/// Returns the index of the new vertex. The original edge (`orig`) keeps its
/// index and its `start_vertex`, now ending at the new split vertex; a new
/// half-edge (`new_edge`) is appended representing the segment from the split
/// point onward to `orig`'s original "to" vertex.
///
/// # DCEL repair (`D-112-MMU-TOPOLOGY` busy-hub fix)
///
/// Splitting a half-edge must rewire *three* link families, not just
/// geometry:
///
/// - `twin` (this crate's `resolve_to_vertex` convention, not a spatial
///   opposite-direction edge): `orig.twin` is repointed to `new_edge_idx` so
///   `resolve_to_vertex(orig)` now resolves to the split vertex. `new_edge`
///   must **keep** its cloned-from-`orig` twin value (`orig`'s *original*
///   twin, captured before mutation) so `resolve_to_vertex(new_edge)` still
///   resolves to `orig`'s original far endpoint — overwriting it to
///   `edge_idx` (as a prior version of this function did) makes every
///   `new_edge` resolve back to `orig`'s *start* vertex instead, which is the
///   root cause this doc comment exists to prevent (see this module's
///   `apply_transitions` doc comment for the full failure chain).
/// - `next`/`prev`: `orig.next` must become `new_edge_idx` and `new_edge.prev`
///   must become `edge_idx` so the two pieces chain correctly; the edge that
///   used to follow `orig` (at `orig`'s pre-mutation `.next`) must have its
///   `.prev` repointed to `new_edge_idx`. Previously neither half of this was
///   done at all — every split's `new_edge` silently kept `orig`'s stale
///   pre-split `next`/`prev` verbatim (via `.clone()`), so
///   `generate_toolpaths::walk_domain_chain`'s DCEL traversal would treat
///   every split-off edge as claiming the *same* next/prev pointers as the
///   original edge, corrupting the domain walk.
///
/// Both repairs generalize correctly across repeated splits along the same
/// physical `edge_idx` (each call further shrinks `orig`'s effective span and
/// correctly re-chains onto whatever the previous split's `new_edge` already
/// was), because `resolve_to_vertex`/`next`/`prev` are always read fresh from
/// the graph's *current* state at the top of each call.
fn insert_node(
    graph: &mut SkeletalTrapezoidationGraph,
    edge_idx: usize,
    pos: f64,
    bead_count: u32,
    transition_ratio: f64,
) -> usize {
    let edge = match graph.edges.get(edge_idx).cloned() {
        Some(e) => e,
        None => return NO_INDEX,
    };
    let start_v = match graph.vertices.get(edge.start_vertex).cloned() {
        Some(v) => v,
        None => return NO_INDEX,
    };
    let to_idx = resolve_to_vertex(graph, edge_idx);
    let end_v = match graph.vertices.get(to_idx).cloned() {
        Some(v) => v,
        None => return NO_INDEX,
    };

    // Interpolate position and radius at the split point.
    let new_pos = interpolate_position(start_v.position, end_v.position, pos.clamp(0.0, 1.0));
    let new_r = start_v.distance_to_boundary
        + (end_v.distance_to_boundary - start_v.distance_to_boundary) * pos.clamp(0.0, 1.0);

    let new_vertex_idx = graph.vertices.len();
    graph.vertices.push(STVertex {
        position: new_pos,
        distance_to_boundary: new_r,
        bead_count: Some(bead_count),
        transition_ratio,
    });

    let new_edge_idx = graph.edges.len();
    let old_next = edge.next; // captured before any mutation, see doc comment.
                              // New edge: from split vertex onward to `orig`'s original "to" vertex.
                              // `new_edge.twin` and `.next` are deliberately left as the values
                              // inherited from `edge.clone()` (see doc comment) — only `start_vertex`,
                              // `prev`, `r_min`/`r_max` need overwriting.
    let mut new_edge = edge.clone();
    new_edge.start_vertex = new_vertex_idx;
    new_edge.prev = edge_idx;
    // r_min/r_max stay on the same edge geometry; recalc from the new bounds.
    let (r_min, r_max) = super::graph::edge_radius_bounds(&graph.vertices, new_vertex_idx, to_idx);
    new_edge.r_min = r_min;
    new_edge.r_max = r_max;
    graph.edges.push(new_edge);

    // Mutate original edge to end at the new split vertex.
    if let Some(orig) = graph.edges.get_mut(edge_idx) {
        let (r_min, r_max) =
            super::graph::edge_radius_bounds(&graph.vertices, orig.start_vertex, new_vertex_idx);
        orig.r_min = r_min;
        orig.r_max = r_max;
        orig.twin = new_edge_idx;
        orig.next = new_edge_idx;
    }
    // Whatever used to follow `orig` in the DCEL chain now follows `new_edge`
    // instead.
    if old_next != NO_INDEX {
        if let Some(follower) = graph.edges.get_mut(old_next) {
            follower.prev = new_edge_idx;
        }
    }

    new_vertex_idx
}

/// For each edge carrying [`super::graph::STHalfEdge::transition_mids`],
/// generates corresponding transition ends (at `pos` on the edge itself, and
/// at `1 - pos` on its twin), sorts each target edge's own ends by position,
/// then inserts new vertices (or snaps to existing endpoints) carrying the
/// correct `bead_count` and `transition_ratio`.
///
/// Snapped existing endpoints receive `transition_ratio = 0.0`. New vertices
/// receive `transition_ratio` derived from the transition step's `mid_r`.
///
/// # Two bugs fixed here (`D-112-MMU-TOPOLOGY` busy-hub root cause)
///
/// 1. **Mirror ends were pushed onto the wrong edge.** The mirrored
///    `1 - pos` transition ends are, by construction and by this function's
///    own original doc comment, meant for the *twin* edge (which walks the
///    opposite direction and so needs the same physical split points
///    expressed in its own position frame) — but the previous implementation
///    appended them to the *same* `edge_idx`'s own end list instead of
///    `edge.twin`'s. For any edge with N of its own transition ends this
///    silently doubled its split count with a second, incorrectly-mirrored
///    set (positions running back toward this edge's own start), which
///    compounds with bug 2 below into a runaway vertex cluster.
/// 2. **Repeated splits along the same `edge_idx` reused a stale position
///    fraction.** [`insert_node`] always operates on `edge_idx`'s *current*
///    span (`edge.start_vertex` to `resolve_to_vertex(edge_idx)`, which
///    shrinks after each split — see that function's doc comment). But
///    `end.pos` values are fractions of the edge's *original*, pre-split
///    length. Passing them straight through for the second, third, ... split
///    on the same `edge_idx` re-interprets an original-frame fraction as a
///    fraction of an already-shrunk sub-edge, landing closer and closer to
///    the shrinking edge's start vertex on every subsequent split — a
///    geometric convergence that produced degenerate clusters of near-
///    identical vertices near sharp polygon corners (confirmed by instrumenting
///    a `cube_4color` MMU triangular cell: 12+ vertices converging to within
///    sub-micron distance of a single corner).
///
///    **Packet 113c Step 6 correction:** the original fix here rescaled
///    ascending-sorted ends onto the "remaining fraction not yet consumed",
///    which implicitly assumed `edge_idx`'s current span shrinks from the
///    *start* side outward (i.e. that repeated splits keep the *far*
///    remaining portion under `edge_idx`). [`insert_node`]'s actual, always-
///    documented behavior is the opposite: `edge_idx` permanently keeps its
///    *original* `start_vertex` and only its *far* endpoint (via `.twin`)
///    moves inward to the newest split vertex on every call — so `edge_idx`'s
///    current span shrinks from the *far/B* side toward `A`, never the other
///    way. Rescaling ascending-sorted ends against that span silently walked
///    every split after the first *back toward `A`* instead of toward `B`
///    (verified by hand-deriving a 2-split example: a `{0.3, 0.7}`-fraction
///    pair landed at absolute fractions `{0.3, 0.171...}` instead of
///    `{0.3, 0.7}` — non-monotonic and wrong), and, independently, produced a
///    different, inconsistent physical position from the *twin* edge's own
///    mirrored splitting of the same physical point. Fixed by sorting each
///    edge's own ends **descending** by `pos` and tracking `far_boundary`
///    (starting at `1.0`, the edge's original far endpoint) that shrinks to
///    each processed end's `pos` — exactly mirroring `edge_idx`'s real
///    far-to-near shrink direction, so `local_pos = end.pos / far_boundary`
///    always lands at the geometrically correct absolute position regardless
///    of how many prior splits already narrowed `edge_idx`'s span.
pub fn apply_transitions(graph: &mut SkeletalTrapezoidationGraph) {
    // Collect transition ends per *target* edge (own ends plus, for edges
    // with a twin, this edge's ends mirrored onto that twin — see doc
    // comment bug 1). Keyed by a `BTreeMap` (not the source edge's own
    // iteration index) precisely so an edge's list combines both its own
    // contributions and any mirrored-in contribution from its twin.
    let mut per_edge_ends: BTreeMap<usize, Vec<TransitionEnd>> = BTreeMap::new();
    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        if edge.transition_mids.is_empty() {
            continue;
        }
        let bucket = per_edge_ends.entry(edge_idx).or_default();
        for tm in &edge.transition_mids {
            bucket.push(TransitionEnd {
                pos: tm.pos,
                bead_count: tm.lower_bead_count,
                mid_r: tm.mid_r,
            });
        }
        // Mirror onto the twin: the twin walks the opposite direction, so
        // the same physical split point sits at `1 - pos` in the twin's own
        // frame. Pushed onto `edge.twin`'s bucket, not this edge's own.
        if edge.twin != NO_INDEX {
            let twin_bucket = per_edge_ends.entry(edge.twin).or_default();
            for tm in &edge.transition_mids {
                twin_bucket.push(TransitionEnd {
                    pos: 1.0 - tm.pos,
                    bead_count: tm.lower_bead_count,
                    mid_r: tm.mid_r,
                });
            }
        }
    }

    for ends in per_edge_ends.values_mut() {
        // Sort *descending* by position — see the "Packet 113c Step 6
        // correction" note in this function's doc comment above for why:
        // `insert_node` always keeps `edge_idx`'s original `start_vertex` and
        // shrinks only its far endpoint, so `edge_idx`'s current span always
        // narrows from the far/B side inward. Processing the farthest end
        // first keeps every subsequent (nearer) end's absolute position
        // inside `edge_idx`'s still-current span.
        ends.sort_by(|a, b| {
            b.pos
                .partial_cmp(&a.pos)
                .unwrap_or(Ordering::Equal)
                .then(a.bead_count.cmp(&b.bead_count))
                .then(a.mid_r.partial_cmp(&b.mid_r).unwrap_or(Ordering::Equal))
        });
        // Deduplicate ends that land at effectively the same position and bead
        // count (common when pos == 0.5 and its mirror coincide).
        ends.dedup_by(|a, b| (a.pos - b.pos).abs() < SNAP_FRAC && a.bead_count == b.bead_count);
    }

    // Apply splits. We process in edge-index order (BTreeMap iteration) so
    // newly inserted edges (appended after every edge this loop started
    // with) do not participate in this pass — their transition_mids are
    // empty.
    for (edge_idx, ends) in per_edge_ends {
        // Tracks the current far boundary of `edge_idx`'s own span, in the
        // *original* (pre-split) position-fraction frame. Starts at `1.0`
        // (the edge's original far/B endpoint) and shrinks to each
        // processed end's `pos` as splits accumulate — mirroring exactly
        // what `insert_node` does to `edge_idx`'s span on every call (keeps
        // `start_vertex`==A, moves the far endpoint inward). See the sort
        // comment above and this function's doc comment for the derivation.
        let mut far_boundary = 1.0_f64;
        for end in ends {
            let local_pos = if far_boundary > f64::EPSILON {
                (end.pos / far_boundary).clamp(0.0, 1.0)
            } else {
                0.0
            };
            far_boundary = end.pos.min(far_boundary);

            let edge = match graph.edges.get(edge_idx) {
                Some(e) => e.clone(),
                None => continue,
            };
            if local_pos < SNAP_FRAC {
                // Snap to the start vertex.
                if let Some(v) = graph.vertices.get_mut(edge.start_vertex) {
                    v.bead_count = Some(end.bead_count);
                    v.transition_ratio = 0.0;
                }
                continue;
            }
            if local_pos > 1.0 - SNAP_FRAC {
                // Snap to the end vertex.
                let to_v = resolve_to_vertex(graph, edge_idx);
                if let Some(v) = graph.vertices.get_mut(to_v) {
                    v.bead_count = Some(end.bead_count);
                    v.transition_ratio = 0.0;
                }
                continue;
            }

            // Transition ratio: fraction from the lower-bead-count transition
            // thickness to the next transition thickness. 0.5 is the neutral
            // midpoint; downstream `generate_toolpaths` will refine this per
            // its own `Beading` computation.
            let ratio = 0.5_f64;
            insert_node(graph, edge_idx, local_pos, end.bead_count, ratio);
        }
    }
}

/// Computes the exact traversal order [`propagate_beadings_upward`] walks:
/// `upward_central_edges`'s own descending-`r_max` order reversed (yielding
/// ascending order), or — for hand-built test graphs with no strictly-upward
/// central edges at all (every endpoint tied on `distance_to_boundary`) — all
/// central edges sorted ascending by `r_min` (tie-broken by index) and then
/// *also* reversed, exactly mirroring the single `iter.iter().rev())` this
/// function used to apply uniformly to whichever list it picked. Factored out
/// so [`compute_dist_to_bottom_source`] can replay the identical walk (same
/// order ⇒ same accumulated distances) without duplicating this order/
/// fallback logic a third time.
fn upward_propagation_order(graph: &SkeletalTrapezoidationGraph) -> Vec<usize> {
    let order = upward_central_edges(graph);
    if !order.is_empty() {
        let mut ascending = order;
        ascending.reverse();
        return ascending;
    }
    let mut all: Vec<usize> = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.central)
        .map(|(idx, _)| idx)
        .collect();
    all.sort_by(|&a, &b| {
        graph.edges[a]
            .r_min
            .partial_cmp(&graph.edges[b].r_min)
            .unwrap_or(Ordering::Equal)
            .then(a.cmp(&b))
    });
    all.reverse();
    all
}

/// Propagates resolved beadings upward (from lower radius to higher radius)
/// along central edges, copying the `from` node's bead count to an unset `to`
/// node and accumulating distance to the nearest bottom source.
///
/// Mirrors `propagateBeadingsUpward` (L1800-1826). Iterates `upward_quad_mids`
/// in reverse order (see [`upward_propagation_order`]).
pub fn propagate_beadings_upward(graph: &mut SkeletalTrapezoidationGraph) {
    for edge_idx in upward_propagation_order(graph) {
        let edge = match graph.edges.get(edge_idx).cloned() {
            Some(e) => e,
            None => continue,
        };
        let from_v = edge.start_vertex;
        let to_v = resolve_to_vertex(graph, edge_idx);
        if to_v == NO_INDEX || from_v == NO_INDEX {
            continue;
        }
        if graph
            .vertices
            .get(to_v)
            .and_then(|v| v.bead_count)
            .is_some()
        {
            continue;
        }
        let Some(from_bead_count) = graph.vertices.get(from_v).and_then(|v| v.bead_count) else {
            continue;
        };
        let source_transition_ratio = graph.vertices[from_v].transition_ratio;
        if let Some(to_vertex) = graph.vertices.get_mut(to_v) {
            to_vertex.bead_count = Some(from_bead_count);
            // transition_ratio propagates from upstream source.
            to_vertex.transition_ratio = source_transition_ratio;
        }
    }
}

/// Interpolates a bead count using the upstream `interpolate()` weighting.
/// Weight is `ratio_of_top = dist_to_bottom_source /
/// min(total_dist, beading_propagation_transition_dist)`. Returns a
/// floating-point blend; callers round as appropriate.
fn interpolate_bead_counts(bottom_bc: u32, top_bc: u32, ratio_of_top: f64) -> u32 {
    let t = ratio_of_top.clamp(0.0, 1.0);
    let blended = bottom_bc as f64 * (1.0 - t) + top_bc as f64 * t;
    blended.round() as u32
}

/// Structural set of vertices that receive a *primary* (non-propagated) bead
/// count: exactly the "to" vertices of central edges, matching
/// `bead_count.rs::assign_bead_counts`'s own primary-pass gate ("for each
/// central edge, assign the bead count at the edge's `to` vertex") — computed
/// fresh from the graph's *current* edge topology, not from the (already
/// fully-propagated, hence uninformative) `bead_count` field.
///
/// This also correctly captures every vertex `apply_transitions::insert_node`
/// creates: splitting a central edge repoints that edge's own "to" (via
/// `.twin`) to the new split vertex, and `insert_node` always sets
/// `bead_count: Some(_)` directly on it — so by construction a split vertex
/// is *also* the "to" vertex of a (now-shrunk) central edge in the
/// post-`apply_transitions` graph this function is always called against,
/// and is correctly treated as primary/real, not propagated.
///
/// Vertices that are never any central edge's "to" (rare — only possible for
/// a vertex with no incoming central edge at all) are the genuine gaps
/// [`propagate_beadings_upward`] exists to fill; used by
/// [`compute_dist_to_bottom_source`] to know when accumulated distance is
/// implicitly zero (a real source) versus needs summing along the chain.
fn primary_source_vertices(graph: &SkeletalTrapezoidationGraph) -> BTreeSet<usize> {
    let mut set = BTreeSet::new();
    for edge_idx in 0..graph.edges.len() {
        if !graph.edges[edge_idx].central {
            continue;
        }
        let to_v = resolve_to_vertex(graph, edge_idx);
        if to_v != NO_INDEX {
            set.insert(to_v);
        }
    }
    set
}

/// Recomputes, from the graph's current structure, the accumulated
/// "distance to bottom source" (`dist_to_bottom_source` in the pre-digested
/// OrcaSlicer notes) that [`propagate_beadings_upward`] built while filling
/// gaps — i.e. how far a genuinely gap-filled vertex's copied bead count has
/// travelled from the nearest real/primary source below it.
///
/// # Why this is recomputed rather than passed through directly
///
/// [`propagate_beadings_downward`]'s signature is frozen (every existing test
/// call site invokes it with no extra arguments), so it cannot *receive* the
/// map [`propagate_beadings_upward`] would have produced. This function
/// closes that gap by replaying the identical walk
/// ([`upward_propagation_order`]) using [`primary_source_vertices`] as the
/// "already had a real bead count" gate — structurally equivalent to
/// `propagate_beadings_upward`'s own `bead_count.is_some()` gate at the time
/// it actually ran, because in this crate's simplified (quad-less, rib-less)
/// topology every central edge's "to" vertex already gets a primary
/// assignment from `assign_bead_counts`/`apply_transitions` (see
/// [`primary_source_vertices`]'s doc comment) — "propagated, non-primary" is
/// the rare gap case, not the rule, so this recomputation is exact for this
/// crate's graph shape, not merely an approximation.
///
/// Vertices absent from the returned map have an implicit distance of `0.0`
/// (either a genuine primary source, or simply never touched by upward
/// propagation) — matching upstream's zero-initialized `BeadingPropagation`
/// for a freshly-created real beading.
fn compute_dist_to_bottom_source(graph: &SkeletalTrapezoidationGraph) -> BTreeMap<usize, f64> {
    let primary = primary_source_vertices(graph);
    let mut dist: BTreeMap<usize, f64> = BTreeMap::new();
    for edge_idx in upward_propagation_order(graph) {
        let edge = match graph.edges.get(edge_idx) {
            Some(e) => e,
            None => continue,
        };
        let from_v = edge.start_vertex;
        let to_v = resolve_to_vertex(graph, edge_idx);
        if to_v == NO_INDEX || from_v == NO_INDEX || primary.contains(&to_v) {
            continue;
        }
        let edge_len = edge_length(graph, edge_idx);
        let edge_len = if edge_len.is_finite() && edge_len > 0.0 {
            edge_len
        } else {
            0.0
        };
        let from_dist = if primary.contains(&from_v) {
            0.0
        } else {
            dist.get(&from_v).copied().unwrap_or(0.0)
        };
        // `or_insert`: the first edge (in traversal order) that reaches `to_v`
        // wins, matching `propagate_beadings_upward`'s own "skip if already
        // has a bead count" gate — once filled, later edges targeting the
        // same `to_v` are no-ops there too.
        dist.entry(to_v).or_insert(from_dist + edge_len);
    }
    dist
}

/// Real (non-placeholder) default beading-propagation transition distance,
/// sourced from this crate's own registered
/// [`crate::beading::factory::BeadingFactoryParams::default`]'s
/// `default_transition_length` (4000 units = 0.4mm at `UNITS_PER_MM`,
/// matching the `wall_transition_length` config key's registered default).
///
/// Used by [`propagate_beadings_downward`] (the frozen, no-argument entry
/// point every existing caller/test invokes) as a real fallback in place of
/// the previous placeholder `4.0` (0.0004mm). Callers with access to the
/// actual configured value — [`crate::arachne::pipeline::run_arachne_pipeline`]
/// — call [`propagate_beadings_downward_with_transition_dist`] directly with
/// that value instead, so production runs get real config fidelity even
/// though this default exists for the frozen entry point.
fn default_beading_propagation_transition_dist() -> f64 {
    crate::beading::factory::BeadingFactoryParams::default().default_transition_length
}

/// Propagates resolved beadings downward (from higher radius to lower
/// radius) along central edges, blending via `interpolate()` when the lower
/// node already carries a beading, using `transition_dist` as the
/// beading-propagation transition distance (upstream's
/// `beading_propagation_transition_dist`, in this crate's units).
///
/// Mirrors `propagateBeadingsDownward` (L1833-1899). Iterates
/// `upward_quad_mids` in forward order (descending R) and routes single-edge
/// propagation from the peak (`edge_to_peak->to`) down to the bottom
/// (`edge_to_peak->from`).
///
/// # Packet 113c Step 8b fix
///
/// The previous implementation used a placeholder `transition_dist = 4.0`
/// (0.0004mm) and computed `total_dist` as only the current edge's own
/// length — ignoring any distance already accumulated along the chain. Both
/// combined to make `ratio_of_top` clamp to (effectively) 1.0 for virtually
/// every real edge via a *symmetric* `clamp(0.0, 1.0)`, turning the intended
/// bead-count blend into an unconditional top-overwrite: e.g. a 10mm
/// square's four corners, correctly assigned `bead_count = 0` by the primary
/// pass, were silently overwritten to `max_bead_count` here — corrupting
/// bead counts at domain-chain stitch points.
///
/// This port now:
/// - computes `total_dist` as the real cumulative chain distance
///   (`top_dist_from_source + edge_len + bottom_dist_to_source`, this
///   crate's names for upstream's `top_beading.dist_from_top_source` /
///   `bottom_beading.dist_to_bottom_source`);
/// - floors `ratio_of_top` at `0.0` only (no ceiling clamp) and instead
///   branches explicitly on `ratio_of_top >= 1.0` for the full-overwrite
///   case, matching upstream's asymmetric gate;
/// - extends the `dist_from_top_source` bookkeeping on every full-copy
///   (both the "no existing beading" branch and the "ratio >= 1.0" branch)
///   so further-down edges in the same chain see the correct cumulative
///   distance, while the merge/blend branch deliberately does *not* extend
///   it — matching upstream's fresh `BeadingPropagation(merged_beading)` (a
///   blended node becomes its own new zero-distance reference point); and
/// - gates the ratio math on the bottom node already carrying *some* beading
///   at all (`bottom_has_beading`), matching upstream's `!hasBeading()`
///   branch — this gate already existed correctly in the prior
///   implementation and is preserved, not part of the bug.
///
/// `dist_to_bottom_source` (accumulated *upward* by
/// [`propagate_beadings_upward`]) is recomputed here via
/// [`compute_dist_to_bottom_source`] rather than received directly from that
/// earlier call — see that function's doc comment for why this is exact,
/// not an approximation, and why it must be recomputed rather than passed
/// through (this function's own signature is frozen).
pub fn propagate_beadings_downward_with_transition_dist(
    graph: &mut SkeletalTrapezoidationGraph,
    transition_dist: f64,
) {
    let transition_dist = if transition_dist.is_finite() && transition_dist > 0.0 {
        transition_dist
    } else {
        default_beading_propagation_transition_dist()
    };

    let dist_to_bottom_source = compute_dist_to_bottom_source(graph);

    let order = upward_central_edges(graph);
    // Fallback for hand-built test graphs with no strictly-upward edges.
    let fallback_order: Vec<usize> = if order.is_empty() {
        let mut all: Vec<usize> = graph
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.central)
            .map(|(idx, _)| idx)
            .collect();
        all.sort_by(|&a, &b| {
            graph.edges[b]
                .r_max
                .partial_cmp(&graph.edges[a].r_max)
                .unwrap_or(Ordering::Equal)
                .then(a.cmp(&b))
        });
        all
    } else {
        Vec::new()
    };
    let iter: &[usize] = if !order.is_empty() {
        &order
    } else {
        &fallback_order
    };

    // Accumulated distance from the nearest real top-side source, keyed by
    // vertex — built fresh during this single forward (descending-R) walk;
    // see the "Packet 113c Step 8b fix" doc comment above for why the merge
    // branch deliberately never inserts into this map.
    let mut dist_from_top_source: BTreeMap<usize, f64> = BTreeMap::new();

    for &edge_idx in iter {
        let edge = match graph.edges.get(edge_idx).cloned() {
            Some(e) => e,
            None => continue,
        };
        // For a single central edge, the peak is the `to` vertex (higher R),
        // the bottom is the `from` vertex (lower R).
        let peak_v = resolve_to_vertex(graph, edge_idx);
        let bottom_v = edge.start_vertex;
        if peak_v == NO_INDEX || bottom_v == NO_INDEX {
            continue;
        }
        let Some(peak_bc) = graph.vertices.get(peak_v).and_then(|v| v.bead_count) else {
            continue;
        };
        let edge_len = edge_length(graph, edge_idx);
        if !edge_len.is_finite() || edge_len <= 0.0 {
            continue;
        }

        let top_dist_from_source = dist_from_top_source.get(&peak_v).copied().unwrap_or(0.0);
        let source_transition_ratio = graph.vertices[peak_v].transition_ratio;
        let bottom_has_beading = graph
            .vertices
            .get(bottom_v)
            .and_then(|v| v.bead_count)
            .is_some();

        if !bottom_has_beading {
            // Fresh downward propagation (upstream's `!hasBeading()` branch):
            // no ratio math, straight copy, and extend the bookkeeping so
            // further-down edges see the correct cumulative distance.
            if let Some(v) = graph.vertices.get_mut(bottom_v) {
                v.bead_count = Some(peak_bc);
                v.transition_ratio = source_transition_ratio;
            }
            dist_from_top_source.insert(bottom_v, top_dist_from_source + edge_len);
            continue;
        }

        let Some(bottom_bc) = graph.vertices.get(bottom_v).and_then(|v| v.bead_count) else {
            continue;
        };
        let bottom_dist_to_source = dist_to_bottom_source.get(&bottom_v).copied().unwrap_or(0.0);
        let total_dist = top_dist_from_source + edge_len + bottom_dist_to_source;
        let denom = total_dist.min(transition_dist).max(f64::EPSILON);
        // Floor at 0 only -- NOT a symmetric clamp(0,1); the explicit
        // `>= 1.0` branch below handles the ceiling (see doc comment).
        let ratio_of_top = (bottom_dist_to_source / denom).max(0.0);

        if ratio_of_top >= 1.0 {
            // Full overwrite -- still extends dist_from_top_source (upstream:
            // `bottom_beading = top_beading; bottom_beading.dist_from_top_source += length;`).
            if let Some(v) = graph.vertices.get_mut(bottom_v) {
                v.bead_count = Some(peak_bc);
                v.transition_ratio = source_transition_ratio;
            }
            dist_from_top_source.insert(bottom_v, top_dist_from_source + edge_len);
        } else {
            let blended = interpolate_bead_counts(bottom_bc, peak_bc, ratio_of_top);
            if let Some(v) = graph.vertices.get_mut(bottom_v) {
                v.bead_count = Some(blended);
            }
            // Merge branch deliberately does not record `dist_from_top_source`
            // for `bottom_v` -- see doc comment (upstream: a merged beading is
            // a fresh `BeadingPropagation`, i.e. distance resets to 0 here).
        }
    }
}

/// Frozen no-argument entry point every existing caller/test invokes.
/// Delegates to [`propagate_beadings_downward_with_transition_dist`] with
/// [`default_beading_propagation_transition_dist`] — see both functions' doc
/// comments for the packet 113c Step 8b fix and why the transition distance
/// can't be threaded through this particular signature.
pub fn propagate_beadings_downward(graph: &mut SkeletalTrapezoidationGraph) {
    propagate_beadings_downward_with_transition_dist(
        graph,
        default_beading_propagation_transition_dist(),
    );
}
