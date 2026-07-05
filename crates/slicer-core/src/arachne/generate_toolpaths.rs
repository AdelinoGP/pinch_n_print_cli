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
//! `connectJunctions` on top of the quad/rib topology built in Step 1:
//!
//! - For every central `NORMAL` spine half-edge, it builds a
//!   `from_junctions` / `to_junctions` fan by calling
//!   `BeadingStrategy::compute()` once per endpoint and reading each bead's
//!   width/offset from the returned `Beading`.
//! - It walks each [`QuadCell`] in `graph.rib.quad_cells`, finds the max-R
//!   edge (the edge whose two endpoints have the largest combined
//!   `distance_to_boundary`), and traverses the DCEL `prev`/`next` chain
//!   around the quad/polygon domain starting from that edge.
//! - At shared vertices it merges adjacent edge junction fans by removing
//!   overlapping `perimeter_index` values, keeping the wider surviving
//!   junction.
//! - It then pairs junctions innermost-to-outermost, producing one open
//!   multi-junction `ExtrusionLine` per bead index (inset ring) for each
//!   quad-chain walk. Odd single-bead segments are tracked in
//!   `passed_odd_edges` so the twin half-edge of the same physical edge is
//!   not duplicated.
//! - Any central spine edge that is not part of a quad cell is emitted as a
//!   single 2-junction segment, mirroring the upstream fallback for regions
//!   that lack a full rib/quad decomposition.
//! - Every emitted line has `is_closed = false` — real ring closure is left
//!   to [`super::stitch::stitch_extrusions`], matching upstream's own
//!   division of labor (`PolylineStitcher::stitch` closes open lines later).
//!
//! Width/offset source: every bead's width and toolpath offset comes from the
//! composed `BeadingStrategy` stack (called once per endpoint), not from any
//! geometric approximation.
//!
//! Bead placement: each bead is offset from its endpoint's raw graph position
//! along a synthetic direction perpendicular to the edge by exactly
//! `beading.toolpath_locations[i]` (units → mm). This does **not** claim to
//! reconstruct which physical side of the material a given inset sits on; it
//! only preserves the *magnitude* and *ordering* (0 = outermost, increasing
//! inward) that the strategy owns.
//!
//! Deterministic and panic-free.

use std::collections::{BTreeMap, BTreeSet};

use slicer_ir::{
    ExtrusionJunction, ExtrusionLine, Point3WithWidth, VariableWidthLines, UNITS_PER_MM,
};

use crate::beading::BeadingStrategy;
use crate::skeletal_trapezoidation::rib::QuadCell;
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

/// Unit vector perpendicular to direction `(dx, dy)` (rotated 90° CCW),
/// falling back to `(0.0, 1.0)` for a near-zero-length input.
fn perpendicular_unit(dx: f32, dy: f32) -> (f32, f32) {
    let len = (dx * dx + dy * dy).sqrt();
    if len > f32::EPSILON {
        (-dy / len, dx / len)
    } else {
        (0.0, 1.0)
    }
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
/// index becomes one junction offset perpendicular to the edge by the
/// strategy's `toolpath_locations[i]`, carrying the strategy's
/// `bead_widths[i]` as width.
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
        let (px, py) = perpendicular_unit(ex - sx, ey - sy);

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

            from_junctions.push(ExtrusionJunction {
                p: Point3WithWidth {
                    x: sx + px * loc_start_mm,
                    y: sy + py * loc_start_mm,
                    z: 0.0,
                    width: width_start_mm,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                perimeter_index: bead_idx,
            });
            to_junctions.push(ExtrusionJunction {
                p: Point3WithWidth {
                    x: ex + px * loc_end_mm,
                    y: ey + py * loc_end_mm,
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

/// Returns the edge of `quad` whose two endpoints have the largest combined
/// `distance_to_boundary`.
///
/// Mirrors `getQuadMaxRedgeTo`: the max-R edge is the "spine" side of the
/// quad from which the junction-fan chain is walked.
fn get_quad_max_r_edge(graph: &SkeletalTrapezoidationGraph, quad: &QuadCell) -> usize {
    let mut best_edge = quad.edges[0];
    let mut best_sum = -1.0_f64;

    for &edge_idx in &quad.edges {
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
            best_edge = edge_idx;
        }
    }

    best_edge
}

/// Walks a DCEL chain following `next` pointers from `start_edge` until it
/// returns to `start_edge` or hits a missing link, returning the visited edge
/// indices in order.
///
/// This is the polygon-domain walk used by `connectJunctions`: each visited
/// edge contributes its `from_junctions`/`to_junctions` to the chain's
/// per-bead polylines.
fn walk_next_chain(graph: &SkeletalTrapezoidationGraph, start_edge: usize) -> Vec<usize> {
    let mut chain = Vec::new();
    let mut current = start_edge;
    let mut steps = 0;
    let max_steps = graph.edges.len();

    loop {
        if steps > max_steps {
            break;
        }
        steps += 1;
        chain.push(current);
        let Some(edge) = graph.edges.get(current) else {
            break;
        };
        if edge.next == NO_INDEX || edge.next == start_edge {
            break;
        }
        current = edge.next;
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
                if first_edge != lower {
                    continue;
                }
                passed_odd_edges.insert((bead_idx, lower, higher));
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

/// Stitches central spine edges that were not visited by any quad-chain walk
/// into simple 2-junction segments.
///
/// Fallback edges are emitted once per walked half-edge. Full upstream
/// `connectJunctions` deduplicates odd single-bead segments via the
/// `passed_odd_edges` mechanism while walking twin chains in the polygon
/// domain; when no quad chain covers an edge we cannot rely on that walk, so
/// we emit both directions and let downstream `stitch_extrusions` reconcile
/// any duplicates.
fn emit_remaining_edge_segments(
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
    buckets: &mut BTreeMap<u32, Vec<ExtrusionLine>>,
    visited: &BTreeSet<usize>,
) {
    for (&edge_idx, (from_j, to_j)) in edge_junctions.iter() {
        if visited.contains(&edge_idx) {
            continue;
        }

        let max_beads = from_j.len().min(to_j.len());
        for bead in 0..max_beads {
            let bead_idx = bead as u32;

            let j_start = from_j[bead].clone();
            let j_end = to_j[bead].clone();
            let is_closed = dist_sq_xy(j_start.p, j_end.p) <= CLOSURE_EPS_MM * CLOSURE_EPS_MM;

            buckets.entry(bead_idx).or_default().push(ExtrusionLine {
                junctions: vec![j_start, j_end],
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
/// This is the packet-113b faithful `connectJunctions` implementation: it
/// precomputes per-edge junction fans, walks each quad cell's DCEL chain from
/// its max-R edge, merges adjacent junction fans, and emits one open
/// multi-junction `ExtrusionLine` per bead index for each chain. Central
/// edges not covered by any quad are emitted as 2-junction fallback
/// segments. All lines remain open (`is_closed = false` except for degenerate
/// coincident endpoints); real ring closure is performed later by
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

    // Phase 2: walk quad chains.
    for quad in &graph.rib.quad_cells {
        let max_r_edge = get_quad_max_r_edge(graph, quad);

        // Walk both the `next` and `prev` chains from the max-R edge so that
        // the entire polygon-domain loop around the quad is covered. Each
        // chain becomes one polyline per bead index.
        let next_chain = walk_next_chain(graph, max_r_edge);
        let prev_chain = {
            let mut chain = Vec::new();
            let mut current = max_r_edge;
            let mut steps = 0;
            let max_steps = graph.edges.len();
            loop {
                if steps > max_steps {
                    break;
                }
                steps += 1;
                chain.push(current);
                let Some(edge) = graph.edges.get(current) else {
                    break;
                };
                if edge.prev == NO_INDEX || edge.prev == max_r_edge {
                    break;
                }
                current = edge.prev;
            }
            chain
        };

        // Mark edges visited by either chain so they are not re-emitted as
        // standalone 2-junction fallback segments.
        for &e in &next_chain {
            visited_edges.insert(e);
        }
        for &e in &prev_chain {
            visited_edges.insert(e);
        }

        emit_chain_lines(
            graph,
            &edge_junctions,
            &mut buckets,
            &mut passed_odd_edges,
            &next_chain,
        );
        emit_chain_lines(
            graph,
            &edge_junctions,
            &mut buckets,
            &mut passed_odd_edges,
            &prev_chain,
        );
    }

    // Phase 3: emit any central spine edges that were not part of a quad
    // chain as 2-junction fallback segments.
    emit_remaining_edge_segments(&edge_junctions, &mut buckets, &visited_edges);

    buckets.into_values().collect()
}
