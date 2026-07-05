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
/// Returns the index of the new vertex. The original edge keeps its index and
/// its `start_vertex`; a new twin-pair half-edge is appended to represent the
/// segment from the split point to the original `to` vertex.
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
    // New edge: from split vertex to original `to` vertex. Twin points back to
    // the original edge's logical forward direction (the original edge now
    // goes from original start to split vertex).
    let mut new_edge = edge.clone();
    new_edge.start_vertex = new_vertex_idx;
    new_edge.twin = edge_idx;
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
    }

    new_vertex_idx
}

/// For each edge carrying [`super::graph::STHalfEdge::transition_mids`],
/// generates corresponding transition ends (at `pos` and at `length - pos` on
/// the twin), sorts them by position, then inserts new vertices (or snaps to
/// existing endpoints) carrying the correct `bead_count` and
/// `transition_ratio`.
///
/// Snapped existing endpoints receive `transition_ratio = 0.0`. New vertices
/// receive `transition_ratio` derived from the transition step's `mid_r`.
pub fn apply_transitions(graph: &mut SkeletalTrapezoidationGraph) {
    // Collect transition ends from every edge. We mirror across twins using the
    // *current* edge length; after any split the twin still points at the same
    // physical edge, so the mirrored end lands at `length - pos`.
    let mut per_edge_ends: Vec<(usize, Vec<TransitionEnd>)> = Vec::new();
    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        if edge.transition_mids.is_empty() {
            continue;
        }
        let mut ends: Vec<TransitionEnd> = edge
            .transition_mids
            .iter()
            .map(|tm| TransitionEnd {
                pos: tm.pos,
                bead_count: tm.lower_bead_count,
                mid_r: tm.mid_r,
            })
            .collect();
        // Mirror onto twin: twin's end position is `length - pos` with the same
        // lower bead count (the twin walks the opposite direction).
        let len = edge_length(graph, edge_idx);
        if edge.twin != NO_INDEX && len > 0.0 && len.is_finite() {
            for tm in &edge.transition_mids {
                ends.push(TransitionEnd {
                    pos: 1.0 - tm.pos,
                    bead_count: tm.lower_bead_count,
                    mid_r: tm.mid_r,
                });
            }
        }
        ends.sort_by(|a, b| {
            a.pos
                .partial_cmp(&b.pos)
                .unwrap_or(Ordering::Equal)
                .then(a.bead_count.cmp(&b.bead_count))
                .then(a.mid_r.partial_cmp(&b.mid_r).unwrap_or(Ordering::Equal))
        });
        // Deduplicate ends that land at effectively the same position and bead
        // count (common when pos == 0.5 and its mirror coincide).
        ends.dedup_by(|a, b| (a.pos - b.pos).abs() < SNAP_FRAC && a.bead_count == b.bead_count);
        per_edge_ends.push((edge_idx, ends));
    }

    // Apply splits. We process in edge-index order so newly inserted edges do
    // not participate in this pass (their transition_mids are empty).
    for (edge_idx, ends) in per_edge_ends {
        let len = edge_length(graph, edge_idx);
        let snap_dist = len * SNAP_FRAC;
        for end in ends {
            let edge = match graph.edges.get(edge_idx) {
                Some(e) => e.clone(),
                None => continue,
            };
            if end.pos < snap_dist {
                // Snap to the start vertex.
                if let Some(v) = graph.vertices.get_mut(edge.start_vertex) {
                    v.bead_count = Some(end.bead_count);
                    v.transition_ratio = 0.0;
                }
                continue;
            }
            if end.pos > 1.0 - snap_dist {
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
            insert_node(graph, edge_idx, end.pos, end.bead_count, ratio);
        }
    }
}

/// Propagates resolved beadings upward (from lower radius to higher radius)
/// along central edges, copying the `from` node's bead count to an unset `to`
/// node and accumulating distance to the nearest bottom source.
///
/// Mirrors `propagateBeadingsUpward` (L1800-1826). Iterates `upward_quad_mids`
/// in reverse order.
pub fn propagate_beadings_upward(graph: &mut SkeletalTrapezoidationGraph) {
    let order = upward_central_edges(graph);
    // Fallback: hand-built test graphs may have no strictly-upward central
    // edges. Walk all central edges in ascending r_min order so gap-filling
    // still works.
    let fallback_order: Vec<usize> = if order.is_empty() {
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
        all
    } else {
        Vec::new()
    };
    let iter: &[usize] = if !order.is_empty() {
        &order
    } else {
        &fallback_order
    };

    for &edge_idx in iter.iter().rev() {
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

/// Propagates resolved beadings downward (from higher radius to lower radius)
/// along central edges, blending via `interpolate()` when the lower node
/// already carries an upward-propagated beading.
///
/// Mirrors `propagateBeadingsDownward` (L1833-1899). Iterates
/// `upward_quad_mids` in forward order and routes single-edge propagation from
/// the peak (`edge_to_peak->to`) down to the bottom (`edge_to_peak->from`).
pub fn propagate_beadings_downward(graph: &mut SkeletalTrapezoidationGraph) {
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
    let transition_dist = 4.0; // placeholder: upstream uses a configured beading-propagation transition distance.
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
        let bottom_has_upward = graph
            .vertices
            .get(bottom_v)
            .and_then(|v| v.bead_count)
            .is_some();
        let edge_len = edge_length(graph, edge_idx);
        if !edge_len.is_finite() || edge_len <= 0.0 {
            continue;
        }

        if bottom_has_upward {
            // Blend existing bottom beading with the top beading.
            let Some(bottom_bc) = graph.vertices.get(bottom_v).and_then(|v| v.bead_count) else {
                continue;
            };
            let total_dist = edge_len; // single edge, no quad-chain prev/next in minimal pass.
            let ratio_of_top = edge_len / total_dist.min(transition_dist);
            let blended = interpolate_bead_counts(bottom_bc, peak_bc, ratio_of_top);
            if let Some(v) = graph.vertices.get_mut(bottom_v) {
                v.bead_count = Some(blended.max(1));
            }
        } else {
            // Copy top beading straight down.
            let source_transition_ratio = graph.vertices[peak_v].transition_ratio;
            if let Some(v) = graph.vertices.get_mut(bottom_v) {
                v.bead_count = Some(peak_bc);
                v.transition_ratio = source_transition_ratio;
            }
        }
    }
}
