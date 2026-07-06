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
//! graph (packet 113c Step 4 — faithful `connectJunctions`, replacing packet
//! 113b's central-only-hop `walk_domain_chain` approximation now that packet
//! 113c Step 3 builds the real interleaved-rib graph topology).
//!
//! # Honesty note (no OrcaSlicer oracle; ADAPTATION, not a literal port)
//!
//! `connectJunctions` itself has zero OrcaSlicer unit-test coverage
//! (confirmed by direct search of `OrcaSlicerDocumented/tests/` during this
//! packet's design), so this module remains a **faithful adaptation** of its
//! documented mechanics (`SkeletalTrapezoidation.cpp:2260-2368`), not a
//! byte-for-byte port:
//!
//! - For every central `NORMAL` spine half-edge, [`generate_junctions`]
//!   builds a `from_junctions` / `to_junctions` fan by calling
//!   `BeadingStrategy::compute()` once per endpoint and reading each bead's
//!   width/offset from the returned `Beading` — unchanged from the prior
//!   implementation.
//! - The outer walk seeds `unprocessed_quad_starts` from every edge whose
//!   `.prev` is absent (`STHalfEdge::prev == NO_INDEX`) — per packet 113c
//!   Step 3's graph construction, this is naturally every rib `back_edge`
//!   (`makeRib` never assigns it a `.prev`) plus each cell's first
//!   transferred edge, mirroring OrcaSlicer's
//!   `unprocessed_quad_starts = edges with !edge.prev`
//!   (`SkeletalTrapezoidation.cpp:2265-2269`).
//! - [`find_quad`] walks `.next` from a popped start to its dead end (an
//!   edge whose own `.next` is absent) — the short 2-3 edge run
//!   (`back_rib -> spine -> forth_rib`, or `back_rib -> spine_closing` for a
//!   cell's un-ribbed closing edge) OrcaSlicer calls "the quad."
//! - [`quad_peak_position`] finds the edge within that quad landing on the
//!   node with the largest `distance_to_boundary` — the quad's peak/widest
//!   point — mirroring `getQuadMaxRedgeTo`, and splits the quad into a
//!   start-side arm (`quad[..=peak]`) and an end-side arm
//!   (`quad[peak + 1..]`). In this crate's topology at most one edge per
//!   quad is ever central + `NORMAL` (every rib is `EXTRA_VD` and excluded
//!   from `edge_junctions`), so concatenating the two arms back together
//!   reproduces the quad's own `.next` order; the split exists for fidelity
//!   to the documented algorithm and to correctly bound a quad that (in a
//!   future topology change) contained more than one contributing edge, not
//!   to reorder anything in today's common case.
//! - The quad's dead-end edge's own `.twin` is the next quad's start
//!   (mirroring `getNextUnconnected()`,
//!   `SkeletalTrapezoidationGraph.cpp:183-193`): walking `.next` to a dead
//!   end, then hopping through `.twin`, is exactly what lets this traversal
//!   continue across a junction/branch vertex of any degree — unlike the
//!   prior `walk_domain_chain`, which filtered every hop by domain
//!   membership and broke at every rib once ribs became ubiquitous
//!   (packet 113c Step 3).
//! - Each freshly popped `poly_domain_start` begins a fresh, empty
//!   contributing-edge chain (`full_chain`) — this crate's structural
//!   equivalent of OrcaSlicer's `new_domain_start` flag ("start a new
//!   `ExtrusionLine` only on the first quad of a fresh domain"): there is
//!   exactly one flush point (this domain's own [`emit_chain_lines`] call)
//!   at the end of its walk, so no separate boolean is needed to gate it.
//! - The walk advances quad-by-quad (`quad_start = <dead end>.twin`),
//!   erasing each visited start from `unprocessed_quad_starts`, until it
//!   returns to `poly_domain_start` (ring closed) or `.twin` is absent /
//!   already claimed by another domain (open chain, exhausted). The outer
//!   loop then pops any remaining unprocessed start and repeats, so multiple
//!   disjoint rings/chains are all handled in one [`generate_toolpaths`]
//!   call.
//! - At shared vertices, [`chain_junctions_for_bead`] merges adjacent edges'
//!   junction fans by keeping the wider surviving junction — unchanged from
//!   the prior implementation.
//! - Odd single-bead segments are still gated by `passed_odd_edges` (an
//!   actual membership check, not just bookkeeping) so a physical edge
//!   walked from both directions in the same overall traversal never
//!   double-emits its lone symmetric innermost bead, matching OrcaSlicer's
//!   `quad_start->next->twin` check (`SkeletalTrapezoidation.cpp:2354-2358`).
//! - A domain whose walk wraps back through at least one other edge to its
//!   own starting quad produces a genuinely closed `ExtrusionLine` directly
//!   from this stage. Because the chain's own first (`chain[0]`'s
//!   from-junction) and last (`chain[chain.len() - 1]`'s to-junction) points
//!   are each independently interpolated along their own edge, they land
//!   near — not exactly on — the shared wrap-around vertex, so
//!   [`emit_chain_lines`] merges them (keeping the wider, the same rule used
//!   at every intermediate shared vertex) and writes the merged junction
//!   into both slots, mirroring `stitch_extrusions`'s own `finalize_chain`
//!   convention (`first.xy == last.xy` for `is_closed = true`) rather than
//!   relying on incidental geometric coincidence. A single-edge chain is
//!   never treated as a ring this way (its own two ends are inherently
//!   distinct physical points, not a shared loop-back vertex) even if the
//!   quad-topology walk trivially returns to its start. Unlike the prior
//!   implementation, real ring closure is no longer deferred to
//!   [`super::stitch::stitch_extrusions`] for every case; an exhausted open
//!   chain still leaves `is_closed = false` for that later stage to close
//!   via proximity.
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
            // instead of extrapolating past it. When the edge is constant-
            // radius (`delta_r_mm == 0`), the bead's target radius is the same
            // at both ends, so the from-junction falls back to the edge's own
            // start endpoint (`t = 0`) and the to-junction falls back to the
            // edge's own end endpoint (`t = 1`) — placing the two junctions at
            // their respective chain vertices so a constant-radius edge
            // stitches correctly across shared vertices. (Using `t = 0` for
            // both would collapse both junctions onto the start vertex,
            // dropping the end vertex from the chain and producing a
            // degenerate 0-length segment.)
            let t_from = if delta_r_mm.abs() > f32::EPSILON {
                ((loc_start_mm - r_start_mm) / delta_r_mm).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t_to = if delta_r_mm.abs() > f32::EPSILON {
                ((loc_end_mm - r_start_mm) / delta_r_mm).clamp(0.0, 1.0)
            } else {
                1.0
            };

            // `perimeter_index` is a placeholder here — `bead_idx` duplicates
            // `ExtrusionLine::inset_idx` and doesn't survive `stitch_extrusions`/
            // `simplify_toolpaths` reordering junctions anyway. The real
            // "index within the wall sequence at that vertex" value is
            // assigned once, pipeline-wide, by
            // `arachne::pipeline::assign_perimeter_indices` after every
            // junction-count/order-changing stage has run.
            from_junctions.push(ExtrusionJunction {
                p: Point3WithWidth {
                    x: sx + (ex - sx) * t_from,
                    y: sy + (ey - sy) * t_from,
                    z: 0.0,
                    width: width_start_mm,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                perimeter_index: 0,
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
                perimeter_index: 0,
            });
        }

        edge_junctions.insert(edge_idx, (from_junctions, to_junctions));
    }

    edge_junctions
}

/// Finds the "quad" starting at `start`: the edge run obtained by walking
/// `.next` until reaching a dead end (an edge whose own `.next` is absent).
///
/// Mirrors OrcaSlicer's short 2-3 edge quad run
/// (`back_rib -> spine -> forth_rib`, or `back_rib -> spine_closing` for a
/// cell's un-ribbed closing edge — `SkeletalTrapezoidation.cpp:2260-2368`).
/// Always returns at least one edge (`start` itself, when
/// `start.next == NO_INDEX`). Bounded by `graph.edges.len()` steps so a
/// malformed `.next` cycle can never loop forever.
fn find_quad(graph: &SkeletalTrapezoidationGraph, start: usize) -> Vec<usize> {
    let mut quad = vec![start];
    let max_len = graph.edges.len().saturating_add(1);

    loop {
        if quad.len() > max_len {
            break;
        }
        let current = *quad
            .last()
            .expect("quad always has at least one edge (seeded with `start`)");
        let next = graph.edges.get(current).map(|e| e.next).unwrap_or(NO_INDEX);
        if next == NO_INDEX {
            break;
        }
        quad.push(next);
    }

    quad
}

/// Returns the position within `quad` of the edge whose "to" vertex has the
/// largest `distance_to_boundary` — the quad's peak/widest point, mirroring
/// `getQuadMaxRedgeTo`. Ties keep the lowest (first-found) position, for
/// determinism. `quad` is assumed non-empty (guaranteed by [`find_quad`]).
fn quad_peak_position(graph: &SkeletalTrapezoidationGraph, quad: &[usize]) -> usize {
    let mut best_pos = 0;
    let mut best_r = f64::NEG_INFINITY;

    for (pos, &edge_idx) in quad.iter().enumerate() {
        let to_idx = resolve_to_vertex(graph, edge_idx);
        let r = graph
            .vertices
            .get(to_idx)
            .map(|v| v.distance_to_boundary)
            .unwrap_or(0.0);
        if r > best_r {
            best_r = r;
            best_pos = pos;
        }
    }

    best_pos
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
///
/// A domain can span regions of different local depth (e.g. a corner-
/// adjacent edge with a shallow `bead_count` next to a deeper spine edge
/// further from the boundary): rather than clamping the whole chain's emitted
/// bead range to whichever edge supports the *fewest* beads (which would
/// silently drop every inner wall for the entire domain because of one
/// shallow edge), each bead index `b` is emitted only over `chain`'s maximal
/// contiguous sub-runs of edges that all actually carry bead `b`.
///
/// `ring_closed` reports whether the domain-walk that produced `chain`
/// returned to its own starting quad (mirroring `connectJunctions`'s own
/// ring-closure detection, [`generate_toolpaths`]'s doc comment). A "ring"
/// only makes physical sense when the walk wraps back through at least one
/// *other* edge, and only for the sub-run spanning `chain` in its entirety
/// (a partial sub-run, by construction, does not reach back to `chain`'s own
/// start): a single-edge chain's own two ends are inherently distinct
/// physical points (an isolated central edge whose only neighbor is
/// non-central can trivially satisfy the quad-topology closure check without
/// representing a real loop), so a sub-run length of at least 2 is required
/// in addition to `ring_closed` before this function forces closure.
fn emit_chain_lines(
    graph: &SkeletalTrapezoidationGraph,
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
    buckets: &mut BTreeMap<u32, Vec<ExtrusionLine>>,
    passed_odd_edges: &mut BTreeSet<(u32, usize, usize)>,
    chain: &[usize],
    ring_closed: bool,
) {
    if chain.is_empty() {
        return;
    }

    // The overall bead range this chain could ever emit: the *largest* fan
    // length found on any edge, not the smallest -- see this function's doc
    // comment for why a uniform minimum across the whole domain is wrong.
    let mut max_beads: usize = 0;
    for &edge_idx in chain {
        if let Some((from_j, to_j)) = edge_junctions.get(&edge_idx) {
            max_beads = max_beads.max(from_j.len().min(to_j.len()));
        }
    }
    if max_beads == 0 {
        return;
    }

    let has_bead = |edge_idx: usize, bead: usize| {
        edge_junctions
            .get(&edge_idx)
            .map(|(from_j, to_j)| bead < from_j.len().min(to_j.len()))
            .unwrap_or(false)
    };

    for bead in 0..max_beads {
        let bead_idx = bead as u32;

        // Split `chain` into maximal contiguous sub-runs that all carry
        // `bead_idx`.
        let mut sub_runs: Vec<&[usize]> = Vec::new();
        let mut run_start: Option<usize> = None;
        for (i, &edge_idx) in chain.iter().enumerate() {
            if has_bead(edge_idx, bead) {
                if run_start.is_none() {
                    run_start = Some(i);
                }
            } else if let Some(start) = run_start.take() {
                sub_runs.push(&chain[start..i]);
            }
        }
        if let Some(start) = run_start {
            sub_runs.push(&chain[start..]);
        }

        // Only a sub-run spanning `chain` in its entirety can represent the
        // ring-closure the outer domain-walk detected.
        let whole_chain_run = sub_runs.len() == 1 && sub_runs[0].len() == chain.len();

        for sub_run in sub_runs {
            // Odd single-bead dedup: only emit from the lower-indexed
            // half-edge of a twin pair when the segment is the lone odd
            // innermost bead.
            let first_edge = sub_run[0];
            if let Some(edge) = graph.edges.get(first_edge) {
                if edge.twin != NO_INDEX
                    && is_odd_single_bead(graph, edge_junctions, first_edge, bead_idx)
                {
                    let lower = first_edge.min(edge.twin);
                    let higher = first_edge.max(edge.twin);
                    let key = (bead_idx, lower, higher);
                    // Actual membership gate (not just bookkeeping):
                    // whichever direction/chain reaches this odd single-bead
                    // segment first claims it; the other half-edge of the
                    // same physical twin pair is suppressed on any later
                    // attempt. Matches upstream's `quad_start->next->twin`
                    // check (`SkeletalTrapezoidation.cpp:2354-2358`).
                    if passed_odd_edges.contains(&key) {
                        continue;
                    }
                    passed_odd_edges.insert(key);
                }
            }

            let mut junctions = chain_junctions_for_bead(edge_junctions, sub_run, bead);
            if junctions.len() < 2 {
                continue;
            }

            let close_ring = ring_closed && whole_chain_run && sub_run.len() >= 2;
            if close_ring {
                // The sub-run's own first (from-junction of its first edge)
                // and last (to-junction of its last edge) entries are each
                // independently interpolated along their own edge and
                // generally land at different points near the shared
                // wrap-around vertex, not exactly on it -- so, mirroring
                // `stitch_extrusions`'s own `finalize_chain` convention
                // (`first.xy == last.xy` for a genuinely closed loop), merge
                // them (keeping the wider of the two, the same rule
                // `chain_junctions_for_bead` already applies at every
                // intermediate shared vertex) and write the merged junction
                // into both the first and last slots so the two positions
                // coincide exactly, not just approximately.
                let n = junctions.len();
                let first = junctions[0].clone();
                let last = junctions[n - 1].clone();
                let merged = if first.p.width >= last.p.width {
                    first
                } else {
                    last
                };
                junctions[0] = merged.clone();
                junctions[n - 1] = merged;
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
}

/// Emits variable-width toolpath insets from `graph`'s central, bead-counted
/// edges, sourcing every bead's width and toolpath offset from `strategy`.
///
/// This is the packet-113c faithful `connectJunctions` implementation (see
/// this module's doc comment): it precomputes per-edge junction fans, seeds
/// `unprocessed_quad_starts` from every edge with no `.prev`, and walks each
/// domain quad-by-quad ([`find_quad`] plus a `.twin`-hop off the quad's dead
/// end, mirroring `getNextUnconnected`), collecting every central,
/// junction-bearing edge crossed into one ordered chain per domain, then
/// emitting one multi-junction `ExtrusionLine` per bead index for that chain
/// via [`emit_chain_lines`]. A domain whose walk returns to its own start
/// closes its lines (`is_closed = true`, once the chain's own first/last
/// junction positions coincide); an exhausted open chain remains open
/// (`is_closed = false`) for [`super::stitch::stitch_extrusions`] to close
/// later.
///
/// Returns one [`VariableWidthLines`] bucket per distinct `inset_idx`, sorted
/// ascending (`0` = outermost).
pub fn generate_toolpaths(
    graph: &SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) -> Vec<VariableWidthLines> {
    let mut buckets: BTreeMap<u32, Vec<ExtrusionLine>> = BTreeMap::new();
    let mut passed_odd_edges: BTreeSet<(u32, usize, usize)> = BTreeSet::new();

    let edge_junctions = generate_junctions(graph, strategy);

    // Seed `unprocessed_quad_starts` per `connectJunctions`
    // (`SkeletalTrapezoidation.cpp:2265-2269`): every edge whose `.prev` is
    // absent. Packet 113c Step 3's construction guarantees this is every rib
    // `back_edge` (`makeRib` never assigns it a `.prev`) plus each cell's
    // first transferred edge.
    let mut unprocessed_quad_starts: BTreeSet<usize> = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge.prev == NO_INDEX)
        .map(|(idx, _)| idx)
        .collect();

    // Bounds the outer loop so a malformed graph (unexpected orphaned starts
    // that never converge) cannot spin forever; a well-formed graph visits
    // every seeded start exactly once via the inner walk, so this is never
    // reached in practice.
    let max_domains = graph.edges.len().saturating_add(1);
    let mut domains_processed = 0usize;

    while let Some(&poly_domain_start) = unprocessed_quad_starts.iter().next() {
        domains_processed += 1;
        if domains_processed > max_domains {
            break;
        }

        // A fresh, empty chain per popped domain start is this crate's
        // structural equivalent of OrcaSlicer's `new_domain_start` flag: see
        // this module's doc comment for why no separate boolean is needed.
        let mut full_chain: Vec<usize> = Vec::new();
        let mut quad_start = poly_domain_start;
        let mut ring_closed = false;

        loop {
            if !unprocessed_quad_starts.remove(&quad_start) {
                // Already claimed by an earlier domain walk -- stop rather
                // than re-emitting or looping forever (should not happen on
                // well-formed topology; defensive only).
                break;
            }

            let quad = find_quad(graph, quad_start);
            let quad_end = *quad
                .last()
                .expect("find_quad always returns at least one edge");
            let peak_pos = quad_peak_position(graph, &quad);
            // Split at the peak (see this module's doc comment on
            // `quad_peak_position` for why concatenating the two arms back
            // together reproduces `quad`'s own order in this crate's
            // topology) and collect every edge that actually carries
            // junction data (ribs never do).
            let (arm_before, arm_after) = quad.split_at(peak_pos + 1);
            for &edge_idx in arm_before.iter().chain(arm_after.iter()) {
                if edge_junctions.contains_key(&edge_idx) {
                    full_chain.push(edge_idx);
                }
            }

            let next_start = graph
                .edges
                .get(quad_end)
                .map(|e| e.twin)
                .unwrap_or(NO_INDEX);

            if next_start == NO_INDEX {
                // Open chain exhausted.
                break;
            }
            if next_start == poly_domain_start {
                // Ring closed back to this domain's own start.
                ring_closed = true;
                break;
            }
            if !unprocessed_quad_starts.contains(&next_start) {
                // Already visited by (or belongs to) a different domain --
                // stop rather than walking into someone else's territory.
                break;
            }
            quad_start = next_start;
        }

        emit_chain_lines(
            graph,
            &edge_junctions,
            &mut buckets,
            &mut passed_odd_edges,
            &full_chain,
            ring_closed,
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
    /// half-edge (`edge 0`) whose `next` is absent (so `find_quad` returns
    /// the single-edge quad `[0]`) and `prev` is also absent (so `edge 0` is
    /// itself a seeded domain start). `edge 0`'s twin (`edge 1`) is present
    /// (so `resolve_to_vertex` can resolve `v1` as `edge 0`'s "to" vertex,
    /// and the domain walk's `.twin`-hop off `edge 0`'s own dead end lands
    /// back on `edge 0` via `edge 1`'s own dead-end `.twin`, closing this
    /// domain immediately) but marked non-central so it contributes no
    /// junction fan of its own — this fixture is deliberately scoped to
    /// exercise exactly one domain walk emitting exactly once per bead
    /// (the packet-113b regression this test was written to catch: a
    /// pre-fix implementation that walked `next`/`prev` as two independent
    /// chains from a "peak" edge emitted this same single-edge domain's
    /// bead segment twice).
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
            // Deliberately non-central: only edge 0 carries junction data,
            // so this fixture exercises exactly one domain-walk emission.
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

    /// Regression test carried forward from the packet-113b
    /// `process_central_domain` double-emission bug (that function no
    /// longer exists — packet 113c replaced it with the quad-by-quad
    /// `connectJunctions` walk in [`generate_toolpaths`]): before the 113b
    /// fix, a single-edge central domain's `next_chain` and `prev_chain` —
    /// both freshly rooted at the same peak edge, and for a one-edge domain
    /// both trivially equal to `[peak_edge]` — were each passed to a
    /// separate `emit_chain_lines` call, so the domain's lone 2-point bead
    /// segment was emitted twice per bead instead of once. This test still
    /// guards that regression under the new implementation: `edge 0`'s own
    /// quad walk closes immediately (via `edge 1`'s dead-end `.twin` landing
    /// back on `edge 0`), so a single central edge with N beads must
    /// produce exactly N `ExtrusionLine`s (one per bead), not 2N.
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
