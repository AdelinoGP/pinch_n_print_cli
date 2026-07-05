// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
// (`generateToolpaths` orchestrator, `generateSegments`, `generateJunctions`,
// `connectJunctions`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Toolpath (variable-width inset) emission from the skeletal trapezoidation
//! graph (packet 113b Step 5 — faithful `connectJunctions`).
//!
//! # Honesty note (no OrcaSlicer oracle; ADAPTATION, not a literal port)
//!
//! This module implements a **faithful Rust adaptation** of OrcaSlicer's
//! `connectJunctions`, generalized to walk over every central-edge domain
//! rather than only edges covered by a
//! [`QuadCell`](crate::skeletal_trapezoidation::rib::QuadCell) (revised for
//! `D-112-MMU-TOPOLOGY`: a `QuadCell` only exists at reflex/sharp polygon
//! corners where a Voronoi vertex sits exactly on the boundary — a convex
//! cell like a square produces zero of them, so gating the walk on
//! `graph.rib.quad_cells` left almost every central spine edge to the naive
//! 2-junction fallback, which downstream `stitch_extrusions` could then
//! bridge across unrelated regions via pure proximity):
//!
//! - For every central `NORMAL` spine half-edge, it builds a
//!   `from_junctions` / `to_junctions` fan by calling
//!   `BeadingStrategy::compute()` once per endpoint and reading each bead's
//!   width/offset from the returned `Beading`.
//! - The outer loop walks every **domain** of central, junction-bearing
//!   half-edges: a domain start is any such edge whose `.prev` does not
//!   continue the domain (missing, non-central, or already visited),
//!   mirroring OrcaSlicer's `unprocessed_quad_starts = edges with
//!   !edge.prev` (`SkeletalTrapezoidation.cpp:2265-2269`). A second pass
//!   force-starts any edge left unvisited by the first (a fully closed
//!   central ring, where every edge's `.prev` is itself a valid domain
//!   edge), so every domain is still walked exactly once.
//! - For each domain start, [`get_domain_max_r_edge`] finds the max-R edge
//!   (the edge whose two endpoints have the largest combined
//!   `distance_to_boundary`) among the edges reachable from it, and
//!   [`walk_domain_chain`] traverses the DCEL `prev`/`next` chain from that
//!   peak — hopping once through `.twin` when a plain `.next`/`.prev` link
//!   would otherwise exit the domain, mirroring OrcaSlicer's
//!   `getNextUnconnected()` continuation
//!   (`SkeletalTrapezoidationGraph.cpp:183-193`).
//! - At shared vertices it merges adjacent edge junction fans by removing
//!   overlapping `perimeter_index` values, keeping the wider surviving
//!   junction.
//! - It then pairs junctions innermost-to-outermost, producing one open
//!   multi-junction `ExtrusionLine` per bead index (inset ring) for each
//!   domain-chain walk. Odd single-bead segments are gated by
//!   `passed_odd_edges` (an actual membership check, not just bookkeeping)
//!   so the twin half-edge of the same physical edge is never duplicated,
//!   matching OrcaSlicer's `quad_start->next->twin` check
//!   (`SkeletalTrapezoidation.cpp:2354-2358`).
//! - Because the two-pass domain walk provably visits every central,
//!   junction-bearing edge, the naive 2-junction fallback is no longer
//!   reachable and has been removed rather than kept as dead code.
//! - Every emitted line has `is_closed = false` — real ring closure is left
//!   to [`super::stitch::stitch_extrusions`], matching upstream's own
//!   division of labor (`PolylineStitcher::stitch` closes open lines later).
//!
//! Width/offset source: every bead's width and toolpath offset comes from the
//! composed `BeadingStrategy` stack (called once per endpoint), not from any
//! geometric approximation.
//!
//! Bead placement: each junction's 2D position is derived by **linear
//! interpolation between the edge's own two Voronoi-vertex endpoints**
//! (`STVertex::position`), parameterized by that endpoint's own
//! `beading.toolpath_locations[i]` measured against both endpoints'
//! `distance_to_boundary`. This mirrors OrcaSlicer's `generateJunctions()`
//! (`SkeletalTrapezoidation.cpp:2013-2079`):
//! `junction = a + (b - a) * (bead_R - start_R) / (end_R - start_R)`, where
//! `a`/`b` are real graph-vertex positions and `start_R`/`end_R` are their
//! `distance_to_boundary` values — there is no perpendicular offset step and
//! no "which physical side" ambiguity, because the interpolation stays on the
//! straight line between two positions that are already correctly placed by
//! the Voronoi diagram itself. The interpolation fraction is clamped to
//! `[0, 1]` (and falls back to the edge's own endpoint when
//! `start_R == end_R`, a constant-radius edge) so a junction can never
//! extrapolate past either endpoint's real position.
//!
//! Deterministic and panic-free.

use std::collections::{BTreeMap, BTreeSet};

use slicer_ir::{
    ExtrusionJunction, ExtrusionLine, Point3WithWidth, VariableWidthLines, UNITS_PER_MM,
};

use crate::beading::BeadingStrategy;
use crate::skeletal_trapezoidation::{EdgeType, SkeletalTrapezoidationGraph};
use crate::voronoi::{Vertex, NO_INDEX};

/// XY distance below which a generated 2-junction line's own endpoints are
/// considered coincident (`is_closed = true`). Millimeters, matching
/// `Point3WithWidth`'s coordinate unit.
const CLOSURE_EPS_MM: f32 = 1e-4;

/// Per-edge junction fans produced by `generate_junctions`.
///
/// `from_junctions` live at the half-edge's `start_vertex`; `to_junctions`
/// live at the half-edge's resolved "to" vertex (its twin's start_vertex).
/// Both vectors are ordered outermost bead (`perimeter_index == 0`) to
/// innermost bead (`perimeter_index == n-1`).
type EdgeJunctions = (Vec<ExtrusionJunction>, Vec<ExtrusionJunction>);

/// Resolves a half-edge's "to" vertex index via its twin's `start_vertex`,
/// matching [`crate::skeletal_trapezoidation::graph`]'s own convention.
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

/// Converts a graph vertex position (slicer units, 1 unit = 100 nm) to
/// millimeters.
fn to_mm_xy(v: Vertex) -> (f32, f32) {
    ((v.x / UNITS_PER_MM) as f32, (v.y / UNITS_PER_MM) as f32)
}

/// Squared XY distance between two junction positions.
fn dist_sq_xy(a: Point3WithWidth, b: Point3WithWidth) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// Bounds a [`crate::beading::Beading`] to the number of indices where its
/// `bead_widths`/`toolpath_locations` vectors agree (defensive).
fn usable_len(b: &crate::beading::Beading) -> usize {
    b.bead_widths.len().min(b.toolpath_locations.len())
}

/// Builds the per-edge `from_junctions` / `to_junctions` fans for every
/// central spine half-edge.
///
/// Mirrors `generateJunctions`: each endpoint gets its own `Beading` from
/// `strategy.compute(2.0 * distance_to_boundary, bead_count)`, and each bead
/// index becomes one junction positioned by linearly interpolating between
/// the edge's two endpoint positions (`start_vertex`/`end_vertex`), using
/// that endpoint's own `toolpath_locations[i]` as the target radius and both
/// endpoints' `distance_to_boundary` as the interpolation's radius bounds —
/// see this module's doc comment ("Bead placement") for the exact formula.
/// Carries the strategy's `bead_widths[i]` as width, unchanged from before.
fn generate_junctions(
    graph: &SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) -> BTreeMap<usize, EdgeJunctions> {
    let mut edge_junctions: BTreeMap<usize, EdgeJunctions> = BTreeMap::new();

    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        if !edge.central {
            continue;
        }
        if edge.edge_type == EdgeType::EXTRA_VD {
            continue;
        }
        let to_idx = resolve_to_vertex(graph, edge_idx);
        let Some(end_vertex) = graph.vertices.get(to_idx) else {
            continue;
        };
        let Some(bead_count) = end_vertex.bead_count else {
            continue;
        };
        if bead_count == 0 {
            continue;
        }

        let Some(start_vertex) = graph.vertices.get(edge.start_vertex) else {
            continue;
        };

        let beading_start =
            strategy.compute(2.0 * start_vertex.distance_to_boundary, bead_count as usize);
        let beading_end =
            strategy.compute(2.0 * end_vertex.distance_to_boundary, bead_count as usize);

        let start_len = usable_len(&beading_start);
        let end_len = usable_len(&beading_end);
        let effective_count = (bead_count as usize).min(start_len.max(end_len));
        if effective_count == 0 {
            continue;
        }

        let (sx, sy) = to_mm_xy(start_vertex.position);
        let (ex, ey) = to_mm_xy(end_vertex.position);
        let r_start_mm = (start_vertex.distance_to_boundary / UNITS_PER_MM) as f32;
        let r_end_mm = (end_vertex.distance_to_boundary / UNITS_PER_MM) as f32;
        let delta_r_mm = r_end_mm - r_start_mm;

        let mut from_junctions = Vec::with_capacity(effective_count);
        let mut to_junctions = Vec::with_capacity(effective_count);

        for bead in 0..effective_count {
            let bead_idx = bead as u32;

            let (width_start_units, loc_start_units) = if bead < start_len {
                (
                    beading_start.bead_widths[bead],
                    beading_start.toolpath_locations[bead],
                )
            } else {
                (
                    beading_end.bead_widths[bead],
                    beading_end.toolpath_locations[bead],
                )
            };
            let (width_end_units, loc_end_units) = if bead < end_len {
                (
                    beading_end.bead_widths[bead],
                    beading_end.toolpath_locations[bead],
                )
            } else {
                (
                    beading_start.bead_widths[bead],
                    beading_start.toolpath_locations[bead],
                )
            };

            let width_start_mm = (width_start_units / UNITS_PER_MM) as f32;
            let width_end_mm = (width_end_units / UNITS_PER_MM) as f32;
            let loc_start_mm = (loc_start_units / UNITS_PER_MM) as f32;
            let loc_end_mm = (loc_end_units / UNITS_PER_MM) as f32;

            // Faithful `generateJunctions` interpolation
            // (`SkeletalTrapezoidation.cpp:2013-2079`): position each junction
            // by walking along the real edge segment `start_pos -> end_pos`,
            // parameterized by where this bead's own target radius
            // (`loc_*_mm`, that endpoint's `toolpath_locations[i]`) falls
            // between the two endpoints' `distance_to_boundary` values.
            // Clamped to `[0, 1]` so a bead whose target radius falls outside
            // this edge's local radius range lands on the nearer endpoint
            // instead of extrapolating past it; falls back to `t = 0` (the
            // start endpoint) when the edge is constant-radius
            // (`delta_r_mm == 0`), matching this doc's "falls back to the
            // edge's own endpoint" guarantee.
            let t_from = if delta_r_mm.abs() > f32::EPSILON {
                ((loc_start_mm - r_start_mm) / delta_r_mm).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t_to = if delta_r_mm.abs() > f32::EPSILON {
                ((loc_end_mm - r_start_mm) / delta_r_mm).clamp(0.0, 1.0)
            } else {
                0.0
            };

            from_junctions.push(ExtrusionJunction {
                p: Point3WithWidth {
                    x: sx + (ex - sx) * t_from,
                    y: sy + (ey - sy) * t_from,
                    z: 0.0,
                    width: width_start_mm,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                perimeter_index: bead_idx,
            });
            to_junctions.push(ExtrusionJunction {
                p: Point3WithWidth {
                    x: sx + (ex - sx) * t_to,
                    y: sy + (ey - sy) * t_to,
                    z: 0.0,
                    width: width_end_mm,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                perimeter_index: bead_idx,
            });
        }

        edge_junctions.insert(edge_idx, (from_junctions, to_junctions));
    }

    edge_junctions
}

/// Returns the edge among `candidates` whose two endpoints have the largest
/// combined `distance_to_boundary`, or `None` if `candidates` is empty.
///
/// Generalizes upstream's `getQuadMaxRedgeTo` (previously scoped to a single
/// [`QuadCell`](crate::skeletal_trapezoidation::rib::QuadCell)'s four edges)
/// to an arbitrary candidate list: the max-R edge is the "widest" point of
/// whatever central-edge domain is being walked, from which the junction-fan
/// chain radiates outward toward the domain's narrower ends.
fn get_domain_max_r_edge(
    graph: &SkeletalTrapezoidationGraph,
    candidates: &[usize],
) -> Option<usize> {
    let mut best_edge = None;
    let mut best_sum = -1.0_f64;

    for &edge_idx in candidates {
        let Some(edge) = graph.edges.get(edge_idx) else {
            continue;
        };
        let to_idx = resolve_to_vertex(graph, edge_idx);
        let from_d = graph
            .vertices
            .get(edge.start_vertex)
            .map(|v| v.distance_to_boundary)
            .unwrap_or(0.0);
        let to_d = graph
            .vertices
            .get(to_idx)
            .map(|v| v.distance_to_boundary)
            .unwrap_or(0.0);
        let sum = from_d + to_d;
        if sum > best_sum {
            best_sum = sum;
            best_edge = Some(edge_idx);
        }
    }

    best_edge
}

/// Direction of a [`walk_domain_chain`] traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalkDir {
    /// Follow `STHalfEdge::next`.
    Next,
    /// Follow `STHalfEdge::prev`.
    Prev,
}

/// Reads the raw `next`/`prev` link off `edge_idx` for the given direction,
/// or [`NO_INDEX`] if `edge_idx` is out of range.
fn domain_link(graph: &SkeletalTrapezoidationGraph, edge_idx: usize, dir: WalkDir) -> usize {
    graph
        .edges
        .get(edge_idx)
        .map(|e| match dir {
            WalkDir::Next => e.next,
            WalkDir::Prev => e.prev,
        })
        .unwrap_or(NO_INDEX)
}

/// Walks a DCEL chain in direction `dir` starting from `start_edge`, staying
/// within the connected central-edge domain (edges present in
/// `edge_junctions`) that no *other* domain walk has already claimed
/// (`visited_edges`).
///
/// Following the plain `next`/`prev` link is preferred; when that link is
/// missing or exits the domain (lands on a non-central/no-junction edge, an
/// already-claimed edge — e.g. a rib or the source boundary segment), this
/// hops once through that edge's `twin` to try to resume on the far side,
/// mirroring OrcaSlicer's `getNextUnconnected()` continuation
/// (`SkeletalTrapezoidationGraph.cpp:183-193`). Stops on a true dead end, a
/// failed twin-hop, or ring closure back to `start_edge`. Returns the
/// visited edge indices in walk order (`start_edge` first).
///
/// `visited_edges` matters here, not just at the outer domain-start level:
/// this crate's simplified transition-insertion pass can leave a *direct*
/// (non-hopped) `next`/`prev` link from an edge with no independent domain
/// membership straight onto an edge another domain has already fully
/// consumed (observed on the tapered-wedge fixture — edge `18`'s plain
/// `next` is edge `13`, with no twin-hop involved, even though `13` is
/// itself a separate, already-processed domain start). Without this check, a
/// later domain could silently re-emit an earlier domain's data by chain-
/// walking straight onto it, while its own distinct junction fan is dropped.
fn walk_domain_chain(
    graph: &SkeletalTrapezoidationGraph,
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
    visited_edges: &BTreeSet<usize>,
    start_edge: usize,
    dir: WalkDir,
) -> Vec<usize> {
    let mut chain = Vec::new();
    let mut seen: BTreeSet<usize> = BTreeSet::new();
    let mut current = start_edge;
    let max_steps = graph.edges.len();

    let is_available = |edge_idx: usize, seen: &BTreeSet<usize>| {
        edge_junctions.contains_key(&edge_idx)
            && !visited_edges.contains(&edge_idx)
            && !seen.contains(&edge_idx)
    };

    loop {
        if !seen.insert(current) || chain.len() > max_steps {
            break;
        }
        chain.push(current);

        let plain = domain_link(graph, current, dir);
        let advance = if plain != NO_INDEX && is_available(plain, &seen) {
            Some(plain)
        } else if plain != NO_INDEX {
            graph
                .edges
                .get(plain)
                .map(|e| e.twin)
                .filter(|&t| t != NO_INDEX && is_available(t, &seen))
        } else {
            None
        };

        match advance {
            Some(next) => current = next,
            None => break,
        }
    }

    chain
}

/// Collects, for each bead index along an edge chain, the sequence of
/// junctions that form one open polyline.
///
/// The chain is a list of edge indices `[e0, e1, ..., eN]` walked in order.
/// For each edge we take the appropriate endpoint junction for bead `b`:
/// - The first edge contributes its `from_junctions[b]` at the chain start.
/// - Each subsequent shared vertex contributes the wider of the current edge's
///   `to_junctions[b]` and the next edge's `from_junctions[b]`; this is the
///   faithful "pop overlapping perimeter_index values" merge adapted to our
///   immutable per-edge fan storage.
/// - The last edge contributes its `to_junctions[b]` at the chain end.
fn chain_junctions_for_bead(
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
    chain: &[usize],
    bead: usize,
) -> Vec<ExtrusionJunction> {
    let mut junctions = Vec::with_capacity(chain.len() + 1);

    for (i, &edge_idx) in chain.iter().enumerate() {
        let Some((from_j, to_j)) = edge_junctions.get(&edge_idx) else {
            continue;
        };
        if i == 0 {
            if let Some(j) = from_j.get(bead) {
                junctions.push(j.clone());
            }
        }
        if i == chain.len() - 1 {
            if let Some(j) = to_j.get(bead) {
                junctions.push(j.clone());
            }
        } else {
            // Intermediate shared vertex: merge this edge's to-junction with
            // the next edge's from-junction for this bead, keeping the wider
            // width. This matches upstream's "pop overlapping perimeter_index"
            // merge at shared vertices.
            if let Some(next_edge) = chain.get(i + 1) {
                let this_to = to_j.get(bead);
                let next_from = edge_junctions
                    .get(next_edge)
                    .and_then(|(next_from_j, _)| next_from_j.get(bead));
                let chosen = match (this_to, next_from) {
                    (Some(a), Some(b)) => {
                        if a.p.width >= b.p.width {
                            a.clone()
                        } else {
                            b.clone()
                        }
                    }
                    (Some(a), None) => a.clone(),
                    (None, Some(b)) => b.clone(),
                    (None, None) => continue,
                };
                junctions.push(chosen);
            }
        }
    }

    junctions
}

/// Returns true if the segment defined by `(edge_idx, twin_idx, bead_idx)` is
/// an odd single-bead segment that should suppress its twin.
///
/// Upstream tracks these in `passed_odd_edges` to avoid emitting the same
/// single-bead odd inset from both half-edges of a physical edge.
fn is_odd_single_bead(
    graph: &SkeletalTrapezoidationGraph,
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
    edge_idx: usize,
    bead_idx: u32,
) -> bool {
    let Some(edge) = graph.edges.get(edge_idx) else {
        return false;
    };
    if edge.twin == NO_INDEX {
        return false;
    }
    let Some(end_vertex) = graph.vertices.get(resolve_to_vertex(graph, edge_idx)) else {
        return false;
    };
    let Some(bead_count) = end_vertex.bead_count else {
        return false;
    };
    if bead_count == 0 || bead_count % 2 == 0 {
        return false;
    }
    if bead_idx != bead_count - 1 {
        return false;
    }
    if end_vertex.transition_ratio != 0.0 {
        return false;
    }
    // Must actually have junctions at this index on this edge.
    edge_junctions
        .get(&edge_idx)
        .map(|(from_j, to_j)| (bead_idx as usize) < from_j.len().min(to_j.len()))
        .unwrap_or(false)
}

/// Emits one multi-junction `ExtrusionLine` per bead index for the given edge
/// chain, using `passed_odd_edges` to suppress twin duplication for odd
/// single-bead segments.
fn emit_chain_lines(
    graph: &SkeletalTrapezoidationGraph,
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
    buckets: &mut BTreeMap<u32, Vec<ExtrusionLine>>,
    passed_odd_edges: &mut BTreeSet<(u32, usize, usize)>,
    chain: &[usize],
) {
    if chain.is_empty() {
        return;
    }

    // Determine the bead range we can emit from this chain: the minimum fan
    // length across all edges (after merge) so every emitted polyline is
    // complete.
    let mut max_beads: usize = usize::MAX;
    for &edge_idx in chain {
        if let Some((from_j, to_j)) = edge_junctions.get(&edge_idx) {
            max_beads = max_beads.min(from_j.len().min(to_j.len()));
        }
    }
    if max_beads == usize::MAX || max_beads == 0 {
        return;
    }

    for bead in 0..max_beads {
        let bead_idx = bead as u32;

        // Odd single-bead dedup: only emit from the lower-indexed half-edge
        // of a twin pair when the segment is the lone odd innermost bead.
        let first_edge = chain[0];
        let first_edge_obj = graph.edges.get(first_edge);
        if let Some(edge) = first_edge_obj {
            if edge.twin != NO_INDEX
                && is_odd_single_bead(graph, edge_junctions, first_edge, bead_idx)
            {
                let lower = first_edge.min(edge.twin);
                let higher = first_edge.max(edge.twin);
                let key = (bead_idx, lower, higher);
                // Actual membership gate (not just bookkeeping): whichever
                // direction/chain reaches this odd single-bead segment first
                // claims it; the other half-edge of the same physical twin
                // pair is suppressed on any later attempt. Matches upstream's
                // `quad_start->next->twin` check
                // (`SkeletalTrapezoidation.cpp:2354-2358`).
                if passed_odd_edges.contains(&key) {
                    continue;
                }
                passed_odd_edges.insert(key);
            }
        }

        let junctions = chain_junctions_for_bead(edge_junctions, chain, bead);
        if junctions.len() < 2 {
            continue;
        }

        let start = junctions[0].p;
        let end = junctions[junctions.len() - 1].p;
        let is_closed = dist_sq_xy(start, end) <= CLOSURE_EPS_MM * CLOSURE_EPS_MM;

        buckets.entry(bead_idx).or_default().push(ExtrusionLine {
            junctions,
            inset_idx: bead_idx,
            is_odd: bead_idx % 2 == 1,
            is_closed,
        });
    }
}

/// Returns `true` if `edge_idx` is a central, junction-bearing edge —
/// i.e. a member of the domain that [`walk_domain_chain`] can traverse.
fn is_domain_edge(edge_junctions: &BTreeMap<usize, EdgeJunctions>, edge_idx: usize) -> bool {
    edge_junctions.contains_key(&edge_idx)
}

/// Returns `true` if `edge_idx`'s `.prev` does not continue the domain —
/// i.e. `edge_idx` is a valid starting point for a domain walk.
///
/// Mirrors OrcaSlicer's `unprocessed_quad_starts = edges with !edge.prev`
/// (`SkeletalTrapezoidation.cpp:2265-2269`): a domain start is an edge whose
/// previous half-edge is missing, non-central, or already claimed by an
/// earlier walk (the last case lets a second pass force a start inside any
/// fully closed central ring, where every edge's `.prev` is itself a valid,
/// not-yet-visited domain edge).
fn is_domain_start(
    graph: &SkeletalTrapezoidationGraph,
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
    visited: &BTreeSet<usize>,
    edge_idx: usize,
) -> bool {
    let prev = domain_link(graph, edge_idx, WalkDir::Prev);
    prev == NO_INDEX || !is_domain_edge(edge_junctions, prev) || visited.contains(&prev)
}

/// Processes one central-edge domain starting at `start_edge`: finds the
/// max-R "peak" edge among everything reachable from `start_edge` (probed in
/// both directions), then walks the `next` and `prev` chains from that peak,
/// emitting one open multi-junction `ExtrusionLine` per bead index for each
/// chain and marking every visited edge so it is never walked twice.
fn process_central_domain(
    graph: &SkeletalTrapezoidationGraph,
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
    buckets: &mut BTreeMap<u32, Vec<ExtrusionLine>>,
    passed_odd_edges: &mut BTreeSet<(u32, usize, usize)>,
    visited_edges: &mut BTreeSet<usize>,
    start_edge: usize,
) {
    let mut probe = walk_domain_chain(
        graph,
        edge_junctions,
        &*visited_edges,
        start_edge,
        WalkDir::Next,
    );
    probe.extend(walk_domain_chain(
        graph,
        edge_junctions,
        &*visited_edges,
        start_edge,
        WalkDir::Prev,
    ));
    // If the peak search (which itself skips already-visited edges) still
    // lands on something visited — e.g. `start_edge` was the only probe
    // candidate and got claimed by a concurrent-looking domain in between —
    // fall back to `start_edge` so this domain's own junction data is never
    // silently dropped.
    let peak_edge = get_domain_max_r_edge(graph, &probe)
        .filter(|e| !visited_edges.contains(e))
        .unwrap_or(start_edge);

    let next_chain = walk_domain_chain(
        graph,
        edge_junctions,
        &*visited_edges,
        peak_edge,
        WalkDir::Next,
    );
    let prev_chain = walk_domain_chain(
        graph,
        edge_junctions,
        &*visited_edges,
        peak_edge,
        WalkDir::Prev,
    );

    for &e in &next_chain {
        visited_edges.insert(e);
    }
    for &e in &prev_chain {
        visited_edges.insert(e);
    }
    // The domain start itself is always reachable from the peak by
    // construction, but insert it defensively so a peak search that (for any
    // reason) fails to rediscover `start_edge` still guarantees forward
    // progress for the outer loop.
    visited_edges.insert(start_edge);

    // `next_chain` and `prev_chain` both start at `peak_edge` by construction
    // (each is a fresh `walk_domain_chain` call rooted there). Emitting them
    // as two independent calls to `emit_chain_lines` — as this function used
    // to do — fragments what should be ONE continuous per-bead polyline
    // spanning the whole domain into two disjoint lines that both include
    // `peak_edge`: every single-edge domain (the common case for a plain,
    // non-reflex quad's lone central spine edge, where neither direction can
    // extend past the peak) then emits the *same* two-junction segment
    // twice, and any multi-edge domain gets an artificial split at its
    // widest point instead of one line from end to end. This is the
    // regression this packet's fix targets (many spurious sub-1mm
    // `ExtrusionLine`s in `cube_4color_arachne_fragments_walls_by_color`).
    //
    // Fix: splice `prev_chain` (reversed, dropping its shared leading
    // `peak_edge`) in front of `next_chain` (which already starts with
    // `peak_edge`) to form one domain-order edge chain, and emit it once.
    // This mirrors OrcaSlicer's `connectJunctions`, which concatenates
    // segments onto a single running path per bead rather than starting a
    // fresh `ExtrusionLine` at every quad-max-r edge
    // (`SkeletalTrapezoidation.cpp:2210-2234`).
    let mut full_chain: Vec<usize> = prev_chain.iter().skip(1).rev().copied().collect();
    full_chain.extend(next_chain.iter().copied());

    emit_chain_lines(
        graph,
        edge_junctions,
        buckets,
        passed_odd_edges,
        &full_chain,
    );
}

/// Emits variable-width toolpath insets from `graph`'s central, bead-counted
/// edges, sourcing every bead's width and toolpath offset from `strategy`.
///
/// This is the packet-113b faithful `connectJunctions` implementation: it
/// precomputes per-edge junction fans, then walks every central-edge domain
/// (a maximal run of connected central, junction-bearing half-edges) from
/// its max-R peak, merging adjacent junction fans and emitting one open
/// multi-junction `ExtrusionLine` per bead index for each chain. All lines
/// remain open (`is_closed = false` except for degenerate coincident
/// endpoints); real ring closure is performed later by
/// [`super::stitch::stitch_extrusions`].
///
/// Returns one [`VariableWidthLines`] bucket per distinct `inset_idx`, sorted
/// ascending (`0` = outermost).
pub fn generate_toolpaths(
    graph: &SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) -> Vec<VariableWidthLines> {
    let mut buckets: BTreeMap<u32, Vec<ExtrusionLine>> = BTreeMap::new();
    let mut passed_odd_edges: BTreeSet<(u32, usize, usize)> = BTreeSet::new();
    let mut visited_edges: BTreeSet<usize> = BTreeSet::new();

    let edge_junctions = generate_junctions(graph, strategy);
    let domain_edge_ids: Vec<usize> = edge_junctions.keys().copied().collect();

    // Pass 1: natural domain starts (open chains — edges whose `.prev` does
    // not continue the domain).
    for &edge_idx in &domain_edge_ids {
        if visited_edges.contains(&edge_idx) {
            continue;
        }
        if !is_domain_start(graph, &edge_junctions, &visited_edges, edge_idx) {
            continue;
        }
        process_central_domain(
            graph,
            &edge_junctions,
            &mut buckets,
            &mut passed_odd_edges,
            &mut visited_edges,
            edge_idx,
        );
    }

    // Pass 2: any edge still unvisited belongs to a fully closed central
    // ring (every edge's `.prev` is itself a valid, not-yet-visited domain
    // edge, so pass 1 never recognized a start there) — force a start at the
    // lowest-indexed remaining edge of each such ring. Combined with pass 1,
    // this guarantees every central, junction-bearing edge is visited
    // exactly once, so no post-hoc 2-junction fallback pass is reachable.
    for &edge_idx in &domain_edge_ids {
        if visited_edges.contains(&edge_idx) {
            continue;
        }
        process_central_domain(
            graph,
            &edge_junctions,
            &mut buckets,
            &mut passed_odd_edges,
            &mut visited_edges,
            edge_idx,
        );
    }

    buckets.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beading::Beading;
    use crate::skeletal_trapezoidation::{STHalfEdge, STVertex};

    /// Deterministic bead-width/location generator for
    /// `single_edge_domain_emits_each_bead_line_exactly_once`: splits
    /// `thickness` into `bead_count` equal-width beads centered within their
    /// own slice, so bead positions vary predictably with each endpoint's own
    /// local thickness (`2 * distance_to_boundary`) without depending on any
    /// production `BeadingStrategy`'s internals.
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
            "FixedTestStrategy"
        }
    }

    /// Builds the smallest possible single central-edge domain: two vertices
    /// (`v0` at the origin, `distance_to_boundary` = 30mm; `v1` at (100mm,
    /// 0), `distance_to_boundary` = 10mm) joined by exactly one central
    /// half-edge (`edge 0`) whose `next`/`prev` are both absent, so
    /// `process_central_domain` walks it as a one-edge domain in both
    /// directions from its own peak (itself) — the exact shape of upstream's
    /// "lone central spine edge of a plain, non-reflex quad" case called out
    /// in `process_central_domain`'s doc comment. `edge 0`'s twin (`edge 1`)
    /// is present (so `resolve_to_vertex` can resolve `v1` as `edge 0`'s "to"
    /// vertex) but marked non-central so it contributes no junction fan of
    /// its own and does not form a second domain — this fixture is
    /// deliberately scoped to exercise exactly one `process_central_domain`
    /// call.
    fn single_edge_domain_graph() -> SkeletalTrapezoidationGraph {
        let v0 = STVertex {
            position: Vertex { x: 0.0, y: 0.0 },
            distance_to_boundary: 300_000.0, // 30mm
            bead_count: None,
            transition_ratio: 0.0,
        };
        let v1 = STVertex {
            position: Vertex {
                x: 1_000_000.0, // 100mm
                y: 0.0,
            },
            distance_to_boundary: 100_000.0, // 10mm
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
            // Deliberately non-central: only edge 0 forms a domain, so this
            // fixture exercises exactly one process_central_domain call.
            central: false,
            edge_type: EdgeType::NORMAL,
            ..STHalfEdge::default()
        };

        SkeletalTrapezoidationGraph {
            vertices: vec![v0, v1],
            edges: vec![edge0, edge1],
            centrality_filtered: true,
            rib: Default::default(),
        }
    }

    /// Regression test for the packet-113b `process_central_domain` fix
    /// (see this module's doc comment and the inline comment above the
    /// `full_chain` splice in `process_central_domain`): before the fix, a
    /// single-edge central domain's `next_chain` and `prev_chain` — both
    /// freshly rooted at the same peak edge, and for a one-edge domain both
    /// trivially equal to `[peak_edge]` — were each passed to a separate
    /// `emit_chain_lines` call, so the domain's lone 2-point bead segment was
    /// emitted twice per bead instead of once. The fix splices the reversed
    /// `prev_chain` (minus its shared leading peak edge, which is empty here
    /// since `peak_edge` has no `prev`) onto `next_chain` and calls
    /// `emit_chain_lines` exactly once, so a single central edge with N beads
    /// must produce exactly N `ExtrusionLine`s (one per bead), not 2N.
    #[test]
    fn single_edge_domain_emits_each_bead_line_exactly_once() {
        let graph = single_edge_domain_graph();
        let strategy = FixedBeadingStrategy;

        let output = generate_toolpaths(&graph, &strategy);

        // bead_count = 2 on the domain edge's "to" vertex means two inset
        // buckets (one per bead index).
        assert_eq!(
            output.len(),
            2,
            "expected exactly 2 inset buckets (one per bead index), got {}",
            output.len()
        );

        let mut total_lines = 0usize;
        for (bucket_pos, bucket) in output.iter().enumerate() {
            let inset_idx = bucket.first().map(|l| l.inset_idx);
            assert_eq!(
                bucket.len(),
                1,
                "inset bucket at outer-Vec position {bucket_pos} (inset_idx {inset_idx:?}): \
                 expected exactly 1 ExtrusionLine for this single-edge domain, got {} -- a count \
                 of 2 (or any even multiple) indicates emit_chain_lines was called more than \
                 once for the same domain walk (the double-emission bug this test guards \
                 against)",
                bucket.len()
            );
            total_lines += bucket.len();

            let line = &bucket[0];
            assert_eq!(
                line.junctions.len(),
                2,
                "inset {}: expected exactly 2 junctions (one per endpoint) for a single-edge \
                 domain, got {}",
                line.inset_idx,
                line.junctions.len()
            );

            // The line's own two junctions must not be an exact duplicate of
            // each other: a degenerate 0-length "line" would also slip past
            // a bare count check.
            let start = line.junctions[0].p;
            let end = line.junctions[1].p;
            assert!(
                dist_sq_xy(start, end) > CLOSURE_EPS_MM * CLOSURE_EPS_MM,
                "inset {}: expected two distinct junction positions, got the same point twice \
                 ({start:?})",
                line.inset_idx
            );
        }

        // Total ExtrusionLine count across every bucket must be exactly 2
        // (one line per bead), not 4 -- which the pre-fix double-
        // `emit_chain_lines`-call bug would have produced for this two-bead
        // single-edge domain.
        assert_eq!(
            total_lines, 2,
            "expected exactly 2 total ExtrusionLines across all buckets (one per bead), got {} \
             -- the pre-fix code emitted the domain's next_chain and prev_chain as two separate \
             emit_chain_lines calls, double-emitting every bead's line",
            total_lines
        );
    }
}
